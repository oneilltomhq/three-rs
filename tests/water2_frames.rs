//! [`Water2Mesh`] over frames. three lists `webgpu_water` in its e2e
//! exception list, so the page has no rung; the water's shaders are gated
//! against three's dumps in `tests/nodes_water_wgsl.rs`, and what the frames
//! do is checked here.
//!
//! A red box under a white water plane, a blue background, and a camera
//! straight above. Both normal maps are flat (`(0.5, 0.5, 1)`, the normal
//! `+y`), so the distortion offset is 0 and the Fresnel term at the centre is
//! the 0.02 reflectivity: the water shows what is under it.
//!
//! * The first frame's delta time is 0, so `updateFlow()` only sets the half
//!   cycle: `flowConfig = (0, 0.075, 0.075)`.
//! * The mirror's target is added to the water during the first render, not
//!   at construction, as upstream adds it inside `setup()`.
//! * The water refracts what was drawn before it: the box at the centre and
//!   the background at a corner, nothing NaN.
//! * `flowConfig.x` advances by `flowSpeed * delta` once per render, `y` stays
//!   half a cycle ahead modulo the cycle, and both reset at a full cycle.
//! * The `color` uniform reaches the frame.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::objects::{Water2Mesh, Water2MeshOptions};
use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::objects::Background;
use three_rs::textures::Texture;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene,
};

const SIZE: u32 = 32;

/// A 4×4 normal map pointing straight out of the surface.
fn flat_normal_map() -> Texture {
    Texture::new(4, 4, Some([128u8, 128, 255, 255].repeat(16)))
}

fn frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: &mut PerspectiveCamera,
    now_ms: f64,
) -> Vec<u8> {
    three_rs::testing::pin_time(Some(now_ms));
    renderer.render(scene, camera);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIZE + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn assert_flow(water: &Water2Mesh, expected: [f64; 3], when: &str) {
    let actual = water.flow_config.get();
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, e)| (a - e).abs() < 1e-9),
        "{when}: flowConfig {actual:?}, expected {expected:?}"
    );
}

#[test]
fn water2_flows_and_refracts_over_frames() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::new(0.0, 0.0, 1.0)));

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::new(1.0, 0.0, 0.0);
    let block = Mesh::new(Rc::new(box_geometry(2.0, 1.0, 2.0, 1, 1, 1)), material);
    block.borrow_mut().position.y = -1.0;
    scene.add(&block);

    let water = Water2Mesh::new(
        Rc::new(plane_geometry(10.0, 10.0, 1, 1)),
        Water2MeshOptions::new(flat_normal_map(), flat_normal_map()),
    );
    water.mesh.borrow_mut().set_rotation(-PI / 2.0, 0.0, 0.0);
    scene.add(&water.mesh);

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
    {
        let mut node = camera.node.borrow_mut();
        node.position.set(0.0, 5.0, 0.0);
        node.set_rotation(-PI / 2.0, 0.0, 0.0);
    }

    assert!(
        water.mesh.children().is_empty(),
        "the mirror's target waits for the first render"
    );
    assert_flow(&water, [0.0, 0.0, 0.0], "before the first frame");

    // Frame 1: delta 0.
    let first = frame(&mut renderer, &mut scene, &mut camera, 0.0);
    assert_eq!(
        water.mesh.children().len(),
        1,
        "the first render adds the mirror's target to the water"
    );
    assert_flow(&water, [0.0, 0.075, 0.075], "frame 1");

    let centre = pixel(&first, SIZE / 2, SIZE / 2);
    assert!(
        centre[0] > 200 && centre[1] < 40 && centre[2] < 40,
        "the water at the centre shows the box under it: {centre:?}"
    );
    let corner = pixel(&first, 1, 1);
    assert!(
        corner[2] > 200 && corner[0] < 40 && corner[1] < 40,
        "the water at a corner shows the background: {corner:?}"
    );

    // Frame 2, a second later: `x += 0.03 * 1`, `y = x + 0.075`.
    frame(&mut renderer, &mut scene, &mut camera, 1000.0);
    assert_flow(&water, [0.03, 0.105, 0.075], "frame 2");

    // Frame 3 at the same time: delta 0, so nothing moves — the update runs
    // per render, with that render's delta.
    let third = frame(&mut renderer, &mut scene, &mut camera, 1000.0);
    assert_flow(&water, [0.03, 0.105, 0.075], "frame 3");
    assert_eq!(water.mesh.children().len(), 1, "the target is added once");
    let centre = pixel(&third, SIZE / 2, SIZE / 2);
    assert!(
        centre[0] > 200 && centre[1] < 40 && centre[2] < 40,
        "frame 3's centre is still the box: {centre:?}"
    );

    // Two more seconds: x = 0.09, and y = 0.165 wraps to 0.015.
    frame(&mut renderer, &mut scene, &mut camera, 2000.0);
    assert_flow(&water, [0.06, 0.135, 0.075], "frame 4");
    frame(&mut renderer, &mut scene, &mut camera, 3000.0);
    assert_flow(&water, [0.09, 0.165 - 0.15, 0.075], "frame 5");
    frame(&mut renderer, &mut scene, &mut camera, 4000.0);
    assert_flow(&water, [0.12, 0.195 - 0.15, 0.075], "frame 6");
    // x reaches the cycle: both reset.
    frame(&mut renderer, &mut scene, &mut camera, 5000.0);
    assert_flow(&water, [0.0, 0.075, 0.075], "frame 7");

    // `waterNode.color.value.set( … )`: a green water over a red box is dark.
    water.color.set(vec![0.0, 1.0, 0.0]);
    let tinted = frame(&mut renderer, &mut scene, &mut camera, 5000.0);
    let centre = pixel(&tinted, SIZE / 2, SIZE / 2);
    assert!(
        centre[0] < 10 && centre[1] < 60 && centre[2] < 10,
        "green water over a red box: {centre:?}"
    );
}
