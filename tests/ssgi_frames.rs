//! `ssgi()`, over frames. three lists `webgpu_postprocessing_ssgi` in its
//! e2e exception list, so there is no reference screenshot and no rung; the
//! SSGI shader is gated against three's dump in
//! `tests/nodes_display_wgsl.rs`, and what its two targets hold is checked
//! here.
//!
//! An unlit red wall standing on an unlit white floor, seen from in front
//! and to the side, with a scene pass writing colour, depth and packed
//! normals into the SSGI node and one of its targets as the pipeline's
//! output. So:
//!
//! * where the quad discards (no geometry: depth 1) both targets keep their
//!   clear: white in the AO, which three clears to `0xffffff`, and black in
//!   the GI, because `WebGPUBackend.beginRender()` clears every attachment
//!   after the first to opaque black;
//! * the floor out in the open is barely occluded in the AO (about 240 on
//!   the Iris Xe), and the floor at the wall's foot is darker (about 210):
//!   the wall is a horizon above the floor's hemisphere;
//! * the same floor at the wall's foot gathers the wall's red as GI — red
//!   well above green — and more of it than the open floor does (about 208
//!   against 0);
//! * nothing is NaN (a NaN reads back as 0, so a black AO pixel where the
//!   floor is would show it);
//! * with temporal filtering (three's default) the slices turn per frame,
//!   so two frames differ; without it they are the same;
//! * `giIntensity = 0` is a uniform, not a rebuild: the next frame's GI is
//!   black wherever there is geometry.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::display::{ssgi, SsgiNode};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    float, normal_view, output_property, pack_normal_to_rgb, texture_uv, uv, vec4_join,
};
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// Every canvas pixel's RGB.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<[u8; 3]> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| [p[0], p[1], p[2]]).collect()
}

fn index((x, y): (u32, u32)) -> usize {
    (y * SIZE + x) as usize
}

/// The pixels of the `(2r+1)²` block around `(x, y)`.
fn block((x, y): (u32, u32), r: u32) -> impl Iterator<Item = (u32, u32)> {
    (y - r..=y + r).flat_map(move |yy| (x - r..=x + r).map(move |xx| (xx, yy)))
}

/// The mean of channel `c` over the 5×5 block around a pixel: the page's
/// two slices of eight steps are noisy pixel to pixel.
fn mean(rgb: &[[u8; 3]], at: (u32, u32), c: usize) -> f64 {
    block(at, 2)
        .map(|p| f64::from(rgb[index(p)][c]))
        .sum::<f64>()
        / 25.0
}

fn unlit(hex: u32) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(hex);
    material
}

/// The canvas pixel a world point lands on.
fn pixel(camera: &PerspectiveCamera, x: f64, y: f64, z: f64) -> (u32, u32) {
    let mut p = Vector3::new(x, y, z);
    p.project(camera);
    let px = ((p.x + 1.0) * 0.5 * f64::from(SIZE)).floor() as u32;
    let py = ((1.0 - p.y) * 0.5 * f64::from(SIZE)).floor() as u32;
    assert!(
        (2..SIZE - 2).contains(&px) && (2..SIZE - 2).contains(&py),
        "({x}, {y}, {z}) is on screen at ({px}, {py})"
    );
    (px, py)
}

#[test]
fn ssgi_occludes_and_bounces_the_wall() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let scene = Scene::new();
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), unlit(0xffffff));
    floor
        .borrow_mut()
        .set_rotation(-std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    scene.add(&floor);
    // Facing +x, its foot along x = -1.
    let wall = Mesh::new(Rc::new(plane_geometry(6.0, 3.0, 1, 1)), unlit(0xff0000));
    {
        let mut object = wall.borrow_mut();
        object.set_rotation(0.0, std::f64::consts::FRAC_PI_2, 0.0);
        object.position.set(-1.0, 1.5, 0.0);
    }
    scene.add(&wall);

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(2.0, 2.0, 3.0);
    camera.look_at(&Vector3::new(-0.5, 0.2, 0.0));
    camera.update_matrix_world();
    let foot = pixel(&camera, -0.6, 0.0, 0.6);
    let open = pixel(&camera, 1.2, 0.0, 1.0);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass = pass(scene, camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_pass.set_mrt(scene_mrt);
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");

    let gi_pass: SsgiNode = ssgi(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("normal"),
        camera,
    );
    // The page's.
    gi_pass.slice_count.set(vec![2.0]);
    gi_pass.step_count.set(vec![8.0]);
    gi_pass.set_use_temporal_filtering(false);

    let mut beauty_view = RenderPipeline::new();
    beauty_view.output_node = Some(scene_pass.node());
    let mut ao_view = RenderPipeline::new();
    ao_view.output_node = Some(vec4_join(vec![
        texture_uv(&gi_pass.ao_texture(), uv()).xyz(),
        float(1.0),
    ]));
    let mut gi_view = RenderPipeline::new();
    gi_view.output_node = Some(vec4_join(vec![gi_pass.gi_node(), float(1.0)]));

    // The probes are where they are meant to be.
    let beauty = frame(&mut beauty_view, &mut renderer);
    let sky = (SIZE - 3, 2);
    assert_eq!(beauty[index(sky)], [0, 0, 0], "the top right corner is sky");
    for probe in [foot, open] {
        assert!(
            block(probe, 2).all(|p| beauty[index(p)] == [255, 255, 255]),
            "{probe:?} is on the floor"
        );
    }

    // AO.
    let ao = frame(&mut ao_view, &mut renderer);
    assert_eq!(ao[index(sky)][0], 255, "the sky keeps the AO's white clear");
    assert!(
        ao.iter().all(|p| p[0] > 0),
        "no AO pixel is black: the AO is never NaN"
    );
    let foot_ao = mean(&ao, foot, 0);
    let open_ao = mean(&ao, open, 0);
    println!("AO: foot {foot_ao:.1} at {foot:?}, open {open_ao:.1} at {open:?}");
    assert!(
        open_ao > 220.0,
        "the open floor is unoccluded ({open_ao:.1})"
    );
    assert!(
        foot_ao < open_ao - 15.0,
        "the wall's foot is occluded ({foot_ao:.1} against {open_ao:.1})"
    );

    // GI.
    let gi = frame(&mut gi_view, &mut renderer);
    // `WebGPUBackend.beginRender()` clears every attachment after the first
    // to opaque black, whatever the clear colour.
    assert_eq!(
        gi[index(sky)],
        [0, 0, 0],
        "the sky keeps the GI's black clear"
    );
    let foot_red = mean(&gi, foot, 0);
    let foot_green = mean(&gi, foot, 1);
    let open_red = mean(&gi, open, 0);
    println!("GI: foot red {foot_red:.1} green {foot_green:.1}, open red {open_red:.1}");
    assert!(
        foot_red > foot_green + 40.0,
        "the floor at the wall's foot gathers the wall's red ({foot_red:.1} against green {foot_green:.1})"
    );
    assert!(
        foot_red > open_red + 20.0,
        "more of it than the open floor ({foot_red:.1} against {open_red:.1})"
    );

    // Temporal filtering off: the same noise every frame.
    let again = frame(&mut ao_view, &mut renderer);
    assert!(
        frame(&mut ao_view, &mut renderer) == again,
        "without temporal filtering two frames are the same"
    );
    // On: the slices turn per frame.
    gi_pass.set_use_temporal_filtering(true);
    let first = frame(&mut ao_view, &mut renderer);
    let second = frame(&mut ao_view, &mut renderer);
    let changed = first.iter().zip(&second).filter(|(a, b)| a != b).count();
    println!("temporal: {changed} pixels changed between two frames");
    assert!(changed > 0, "with temporal filtering the frames differ");
    assert!(
        second.iter().all(|p| p[0] > 0),
        "no NaN under temporal filtering"
    );

    // `giIntensity = 0`: a uniform write, seen on the next frame.
    gi_pass.gi_intensity.set(vec![0.0]);
    let dark = frame(&mut gi_view, &mut renderer);
    assert!(
        mean(&dark, foot, 0) < 1.0 && mean(&dark, open, 0) < 1.0,
        "no GI at zero intensity"
    );
}
