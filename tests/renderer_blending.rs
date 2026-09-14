//! Blend state, drawn: that `material.blending` / `transparent` reach the
//! pipeline and that a transparent draw lands on top of the opaque ones.
//!
//! The table itself is unit-tested against `WebGPUPipelineUtils` in
//! `src/materials/blending.rs`; this is the end-to-end half, which no rung
//! exercises — every rung-1–9 material is opaque `NormalBlending`, so its
//! pipeline carries no blend state at all.
//!
//! One `#[test]`: each render builds its own device, and cargo runs test
//! functions inside a binary concurrently.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::materials::Blending;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene,
    Vector3,
};

const WIDTH: f64 = 200.0;
const HEIGHT: f64 = 200.0;

/// An opaque red quad at z = 0, and a green one at z = 0.01 — nearer the camera —
/// covering its right half, with `blending` / `transparent` / `opacity` set by
/// the caller.
fn render(overlay: impl FnOnce(&mut MeshBasicNodeMaterial)) -> Vec<u8> {
    let mut camera = PerspectiveCamera::new(60.0, WIDTH / HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let geometry = Rc::new(plane_geometry(4.0, 4.0, 1, 1));

    let mut base = MeshBasicNodeMaterial::new();
    base.color = Color::new(1.0, 0.0, 0.0);
    let red = Mesh::new(geometry.clone(), base);
    scene.add(&red);

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::new(0.0, 1.0, 0.0);
    overlay(&mut material);
    let green = Mesh::new(geometry, material);
    green.borrow_mut().position.set(2.0, 0.0, 0.01);
    scene.add(&green);

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH, HEIGHT);
    renderer.render(&mut scene, &mut camera);

    let (width, height, pixels) = renderer.read_canvas_pixels();
    assert_eq!((width, height), (WIDTH as u32, HEIGHT as u32));
    pixels
}

/// The pixel at the centre of the overlap, as `( r, g, b )`.
fn overlap(pixels: &[u8]) -> (u8, u8, u8) {
    // The red quad spans x ∈ [-2, 2] world, the green one [0, 4]; the overlap's
    // centre is x = 1, y = 0, which at this camera is right of the middle.
    let x = (WIDTH as usize) * 5 / 8;
    let y = (HEIGHT as usize) / 2;
    let i = (y * WIDTH as usize + x) * 4;
    (pixels[i], pixels[i + 1], pixels[i + 2])
}

#[test]
fn blending_reaches_the_pipeline() {
    // Opaque `NormalBlending`: no blend state, so the nearer green quad simply
    // replaces the red one. This is every rung's case.
    let opaque = render(|_| {});
    let (r, g, b) = overlap(&opaque);
    println!("opaque overlap: {r} {g} {b}");
    assert!(g > 200 && r < 40 && b < 40, "opaque overlay: {r} {g} {b}");

    // `transparent: true` with half opacity and the default `NormalBlending`:
    // `( SrcAlpha, OneMinusSrcAlpha )`, so the overlap is half green over half
    // red. The gate in `_getBlending()` opens because `transparent` is true —
    // `DiffuseColor.w = 1.0` is no longer emitted either, so the opacity
    // actually reaches the blender.
    let normal = render(|material| {
        material.transparent = true;
        material.opacity = 0.5;
    });
    let (r, g, b) = overlap(&normal);
    println!("normal-blended overlap: {r} {g} {b}");
    assert!(
        r > 60 && g > 60 && b < 40,
        "a half-opacity NormalBlending overlay should mix red and green, got {r} {g} {b}"
    );

    // `AdditiveBlending` at full opacity: `( SrcAlpha, One )`, so green is added
    // to the red already in the target and the overlap is yellow. This is the
    // galaxy example's blend mode.
    let additive = render(|material| {
        material.transparent = true;
        material.blending = Blending::Additive;
    });
    let (r, g, b) = overlap(&additive);
    println!("additive overlap: {r} {g} {b}");
    assert!(
        r > 200 && g > 200 && b < 40,
        "an additive overlay over red should be yellow, got {r} {g} {b}"
    );
}
