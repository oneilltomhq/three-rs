//! `ao()`, over frames. three lists `webgpu_postprocessing_ao` in its e2e
//! exception list, so there is no reference screenshot and no rung; the GTAO
//! shader is gated against three's dump in `tests/nodes_display_wgsl.rs`,
//! and what the frames do is checked here.
//!
//! A white box standing on a white floor, seen from above and in front, with
//! a normal pre-pass feeding the AO node and the AO texture itself as the
//! pipeline's output, so the canvas is the occlusion. So:
//!
//! * where the quad discards (no geometry: depth 1) the target keeps its
//!   white clear, so the sky is 255;
//! * the floor out in the open is unoccluded, near 255;
//! * the floor just in front of the box, and the box's foot, are darker: the
//!   box's side is a horizon above the floor's hemisphere;
//! * nothing is NaN (a NaN reads back as 0 in `rgba8unorm`, so a black
//!   pixel where the floor is would show it);
//! * `resolutionScale = 0.5` renders the AO at half size and reads the
//!   centre depth with a `textureGather`, and still puts the contact dark;
//! * `samples = 8` rebuilds the material with three steps per slice
//!   before the next frame, and the frame still comes out;
//! * with `builtinAOContext`, the beauty of a Standard material under an
//!   ambient-only environment is darker at the contact than the same
//!   beauty without it, and the same out in the open.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::{box_geometry, plane_geometry};

use three_rs::nodes::display::ao;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{normal_view, pack_normal_to_rgb, screen_uv};
use three_rs::nodes::velocity::velocity;
use three_rs::{
    pass, AmbientLight, Color, Mesh, MeshBasicNodeMaterial, MeshStandardNodeMaterial,
    PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// The red channel of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> (u32, u32, Vec<u8>) {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    (width, height, pixels.chunks(4).map(|p| p[0]).collect())
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

/// A unit box on a floor, lit by nothing but an ambient light; the camera
/// looks down at the box's foot.
fn box_on_floor(
    material: MeshBasicNodeMaterial,
) -> (Rc<RefCell<Scene>>, Rc<RefCell<PerspectiveCamera>>) {
    let scene = Scene::new();
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), material.clone());
    floor
        .borrow_mut()
        .set_rotation(-std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    scene.add(&floor);
    let cube = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    cube.borrow_mut().position.y = 0.5;
    scene.add(&cube);
    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 3.0));

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(0.0, 1.6, 2.6);
    camera.look_at(&Vector3::new(0.0, 0.3, 0.0));
    (Rc::new(RefCell::new(scene)), Rc::new(RefCell::new(camera)))
}

/// Where the box's front foot meets the floor, in canvas pixels, and a patch
/// of open floor at the bottom of the frame, found from the AO frame itself:
/// the darkest row of the lower half is the contact.
fn contact_and_open(red: &[u8]) -> ((u32, u32), (u32, u32)) {
    let x = SIZE / 2;
    let contact_y = (SIZE / 2..SIZE - 4)
        .min_by_key(|&y| mean(red, x, y, 2) as u32)
        .unwrap();
    ((x, contact_y), (x, SIZE - 4))
}

#[test]
fn gtao_darkens_the_contact() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let mut white = MeshBasicNodeMaterial::new();
    white.color = Color::from_hex(0xffffff);
    let (scene, camera) = box_on_floor(white);

    let mut pre_pass = pass(scene.clone(), camera.clone());
    pre_pass.set_transparent(false);
    pre_pass.set_mrt(mrt(vec![
        ("output", pack_normal_to_rgb(normal_view())),
        ("velocity", velocity()),
    ]));
    let _ = pre_pass.texture_node("output");
    let _ = pre_pass.texture_node("depth");
    let _ = pre_pass.texture_node("velocity");

    let ao_node = ao(
        &pre_pass.depth_texture(),
        &pre_pass.texture(),
        camera.clone(),
    );
    ao_node.radius.set(vec![0.5]);

    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(ao_node.node());

    let (width, height, full) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (SIZE, SIZE));

    assert_eq!(at(&full, 2, 2), 255, "the sky keeps the white clear");
    assert!(
        full.iter().all(|&r| r > 0),
        "no pixel is black: the AO is never NaN"
    );
    let (contact, open) = contact_and_open(&full);
    let contact_ao = mean(&full, contact.0, contact.1, 1);
    let open_ao = mean(&full, open.0, open.1, 1);
    println!(
        "full resolution: contact {contact_ao:.1} at row {}, open {open_ao:.1}",
        contact.1
    );
    assert!(
        open_ao > 230.0,
        "the open floor is unoccluded ({open_ao:.1})"
    );
    assert!(
        contact_ao < open_ao - 40.0,
        "the box's foot is occluded ({contact_ao:.1} against {open_ao:.1})"
    );

    // Half resolution: the gather path.
    ao_node.set_resolution_scale(0.5);
    let half = frame(&mut pipeline, &mut renderer).2;
    assert!(half.iter().all(|&r| r > 0), "no NaN at half resolution");
    let contact_half = mean(&half, contact.0, contact.1, 1);
    let open_half = mean(&half, open.0, open.1, 1);
    println!("half resolution: contact {contact_half:.1}, open {open_half:.1}");
    assert!(
        open_half > 230.0,
        "the open floor is unoccluded ({open_half:.1})"
    );
    assert!(
        contact_half < open_half - 40.0,
        "the box's foot is occluded at half resolution ({contact_half:.1} against {open_half:.1})"
    );

    // Fewer samples rebuild the material; the frame after it is still AO.
    ao_node.set_resolution_scale(1.0);
    ao_node.set_samples(8);
    ao_node.set_use_temporal_filtering(true);
    let few = frame(&mut pipeline, &mut renderer).2;
    assert!(few.iter().all(|&r| r > 0), "no NaN with 8 samples");
    let contact_few = mean(&few, contact.0, contact.1, 1);
    let open_few = mean(&few, open.0, open.1, 1);
    println!("8 samples, temporal: contact {contact_few:.1}, open {open_few:.1}");
    assert!(
        contact_few < open_few - 40.0,
        "still occluded with 8 samples"
    );

    // `builtinAOContext`: the beauty, with and without.
    let (scene, camera) = box_on_floor(MeshStandardNodeMaterial::standard(
        Color::from_hex(0xffffff),
        1.0,
        0.0,
    ));
    let mut pre_pass = pass(scene.clone(), camera.clone());
    pre_pass.set_transparent(false);
    pre_pass.set_mrt(mrt(vec![
        ("output", pack_normal_to_rgb(normal_view())),
        ("velocity", velocity()),
    ]));
    let _ = pre_pass.texture_node("output");
    let _ = pre_pass.texture_node("depth");
    let _ = pre_pass.texture_node("velocity");
    let ao_node = ao(
        &pre_pass.depth_texture(),
        &pre_pass.texture(),
        camera.clone(),
    );
    ao_node.radius.set(vec![0.5]);

    let plain_pass = pass(scene.clone(), camera.clone());
    let mut plain = RenderPipeline::new();
    plain.output_node = Some(plain_pass.texture_node("output"));
    let without = frame(&mut plain, &mut renderer).2;

    let occluded_pass = pass(scene.clone(), camera.clone());
    occluded_pass.set_context_ao(ao_node.sample(screen_uv()).x());
    let mut occluded = RenderPipeline::new();
    occluded.output_node = Some(occluded_pass.texture_node("output"));
    let with = frame(&mut occluded, &mut renderer).2;

    let open_without = mean(&without, open.0, open.1, 1);
    let open_with = mean(&with, open.0, open.1, 1);
    let contact_without = mean(&without, contact.0, contact.1, 1);
    let contact_with = mean(&with, contact.0, contact.1, 1);
    println!(
        "beauty: open {open_without:.1} -> {open_with:.1}, contact {contact_without:.1} -> {contact_with:.1}"
    );
    assert!(
        (open_with - open_without).abs() < 6.0,
        "the open floor is unchanged by the AO context ({open_without:.1} -> {open_with:.1})"
    );
    assert!(
        contact_with < contact_without - 20.0,
        "the AO context darkens the contact ({contact_without:.1} -> {contact_with:.1})"
    );
}
