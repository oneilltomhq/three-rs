//! `ssao()`, over frames. three lists `webgpu_postprocessing_ao` (whose SSAO
//! mode uses `SSAONode`) in its e2e exception list, so there is no reference
//! screenshot and no rung; the AO and blur shaders are gated against the
//! dump of `tools/dump-pages/ssao.html` in `tests/nodes_display_wgsl.rs`,
//! and what the frames do is checked here.
//!
//! A white box standing on a white floor, seen from above and in front, with
//! a normal pre-pass feeding the SSAO node and the AO texture itself as the
//! pipeline's output, so the canvas is the occlusion. So:
//!
//! * where the AO quad discards (no geometry: depth 1) the target keeps its
//!   white clear, so the sky is 255;
//! * the floor out in the open is nearly unoccluded, and the crease where
//!   the box's foot meets the floor is clearly darker;
//! * nothing is NaN (a NaN reads back as 0 in `rgba8unorm`);
//! * the raw AO (`blurEnabled = false`) carries the per-pixel
//!   interleaved-gradient-noise rotation of the Vogel disk, and the
//!   depth-aware blur takes most of it out: the mean squared difference
//!   between neighbouring floor pixels drops, and the crease stays dark.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::nodes::display::{ssao, SampleFn};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{normal_view, pack_normal_to_rgb, texture_uv, unpack_rgb_to_normal};
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// The red channel of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| p[0]).collect()
}

fn at(red: &[u8], x: u32, y: u32) -> u8 {
    red[(y * SIZE + x) as usize]
}

/// The mean of a `(2r+1)²` block around `(x, y)`.
fn mean(red: &[u8], x: u32, y: u32, r: u32) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for yy in y - r..=y + r {
        for xx in x - r..=x + r {
            sum += at(red, xx, yy) as f64;
            n += 1.0;
        }
    }
    sum / n
}

/// The mean squared difference between horizontally and vertically
/// neighbouring pixels over the lower half of the frame (all floor and box,
/// no sky).
fn roughness(red: &[u8]) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for y in SIZE / 2..SIZE - 1 {
        for x in 1..SIZE - 1 {
            let p = at(red, x, y) as f64;
            for q in [at(red, x + 1, y), at(red, x, y + 1)] {
                sum += (p - q as f64).powi(2);
                n += 1.0;
            }
        }
    }
    sum / n
}

/// Where the box's front foot meets the floor, found from the AO frame
/// itself (the darkest row of the lower half), and a patch of open floor at
/// the bottom of the frame.
fn contact_and_open(red: &[u8]) -> ((u32, u32), (u32, u32)) {
    let x = SIZE / 2;
    let contact_y = (SIZE / 2..SIZE - 4)
        .min_by_key(|&y| mean(red, x, y, 2) as u32)
        .unwrap();
    ((x, contact_y), (x, SIZE - 4))
}

#[test]
fn ssao_darkens_the_crease_and_blurs() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let scene = Scene::new();
    let mut white = MeshBasicNodeMaterial::new();
    white.color = Color::from_hex(0xffffff);
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), white.clone());
    floor
        .borrow_mut()
        .set_rotation(-std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    scene.add(&floor);
    let cube = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), white);
    cube.borrow_mut().position.y = 0.5;
    scene.add(&cube);
    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(0.0, 1.6, 2.6);
    camera.look_at(&Vector3::new(0.0, 0.3, 0.0));
    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut pre_pass = pass(scene, camera.clone());
    pre_pass.set_transparent(false);
    pre_pass.set_mrt(mrt(vec![("output", pack_normal_to_rgb(normal_view()))]));
    let _ = pre_pass.texture_node("output");
    let _ = pre_pass.texture_node("depth");
    let packed = pre_pass.texture();
    let normal: SampleFn = Rc::new(move |coord| unpack_rgb_to_normal(texture_uv(&packed, coord)));

    let ao_node = ssao(&pre_pass.depth_texture(), normal, &camera);
    ao_node.set_resolution_scale(1.0);
    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(ao_node.node());

    ao_node.set_blur_enabled(false);
    let raw = frame(&mut pipeline, &mut renderer);
    ao_node.set_blur_enabled(true);
    let blurred = frame(&mut pipeline, &mut renderer);

    for (label, red) in [("raw", &raw), ("blurred", &blurred)] {
        assert_eq!(at(red, 2, 2), 255, "{label}: the sky keeps the white clear");
        assert!(
            red.iter().all(|&r| r > 0),
            "{label}: no pixel is black: the AO is never NaN"
        );
    }

    let (contact, open) = contact_and_open(&blurred);
    for (label, red) in [("raw", &raw), ("blurred", &blurred)] {
        let contact_ao = mean(red, contact.0, contact.1, 1);
        let open_ao = mean(red, open.0, open.1, 1);
        println!(
            "{label}: crease {contact_ao:.1} at row {}, open {open_ao:.1}",
            contact.1
        );
        assert!(
            open_ao > 220.0,
            "{label}: the open floor is nearly unoccluded ({open_ao:.1})"
        );
        assert!(
            contact_ao < open_ao - 40.0,
            "{label}: the crease is occluded ({contact_ao:.1} against {open_ao:.1})"
        );
    }

    let (rough_raw, rough_blurred) = (roughness(&raw), roughness(&blurred));
    println!("roughness: raw {rough_raw:.1}, blurred {rough_blurred:.1}");
    assert!(
        rough_blurred < rough_raw * 0.6,
        "the blur smooths the per-pixel noise ({rough_raw:.1} -> {rough_blurred:.1})"
    );
}
