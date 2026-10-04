//! `denoise()`, over a frame. No r187 example page uses `DenoiseNode`, so
//! there is no reference screenshot and no rung; the shader is gated against
//! the dump of `tools/dump-pages/denoise.html` in
//! `tests/nodes_display_wgsl.rs`, and what it does to a frame is checked
//! here.
//!
//! A plane facing the camera, painted with a pixel-scale checkerboard of two
//! greys (the "noise"), and a white plane nearer the camera covering the
//! right half of the frame. The scene pass's colour, depth and view normal
//! feed the denoiser, whose output is the canvas. So:
//!
//! * on the far plane, away from the white one, the checkerboard comes out
//!   much flatter: its pixel-to-pixel variance drops well below the raw
//!   frame's, and its mean stays where it was;
//! * the depth edge stays sharp: with a tight `depthPhi` a tap on the other
//!   plane gets no weight, so the white plane stays white and the
//!   checkerboard next to it is not pulled towards white;
//! * the same holds with no normal input (`getNormalFromDepth`).
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::display::{denoise, SampleFn};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    float, floor, mod_float, normal_view, output_property, screen_coordinate, texture_uv, vec3_join,
};
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

const SIZE: u32 = 64;

/// The red channel of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| p[0]).collect()
}

/// The pixels of columns `x0..x1`, rows 4 to `SIZE - 4`.
fn region(red: &[u8], x0: u32, x1: u32) -> Vec<f64> {
    (4..SIZE - 4)
        .flat_map(|y| (x0..x1).map(move |x| red[(y * SIZE + x) as usize] as f64))
        .collect()
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn variance(v: &[f64]) -> f64 {
    let m = mean(v);
    v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64
}

#[test]
fn denoise_flattens_noise_and_keeps_edges() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let scene = Scene::new();
    // A checkerboard of 0.35 and 0.65 grey, one pixel per square.
    let sc = screen_coordinate();
    let parity = mod_float(floor(sc.x()).add(floor(sc.y())), 2.0);
    let grey = float(0.35).add(parity.mul(0.3));
    let mut checker = MeshBasicNodeMaterial::new();
    checker.color_node = Some(vec3_join(vec![grey.clone(), grey.clone(), grey]));
    let back = Mesh::new(Rc::new(plane_geometry(10.0, 10.0, 1, 1)), checker);
    scene.add(&back);
    let mut white = MeshBasicNodeMaterial::new();
    white.color = Color::from_hex(0xffffff);
    let front = Mesh::new(Rc::new(plane_geometry(2.0, 4.0, 1, 1)), white);
    front.borrow_mut().position.x = 1.0;
    front.borrow_mut().position.z = 0.8;
    scene.add(&front);

    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 2.0;

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene, camera.clone());
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("normal", normal_view()),
    ]));
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");

    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(scene_pass.texture_node("output"));
    let raw = frame(&mut pipeline, &mut renderer);

    // The white plane covers the right half; find its first column.
    let mid = SIZE / 2;
    let edge = (0..SIZE)
        .find(|&x| raw[(mid * SIZE + x) as usize] >= 254)
        .expect("the white plane is in frame");
    assert!(
        (SIZE / 2 - 3..=SIZE / 2 + 3).contains(&edge),
        "the white plane starts near the middle (column {edge})"
    );
    let raw_noise = region(&raw, 4, edge - 8);
    let raw_var = variance(&raw_noise);
    assert!(
        raw_var > 300.0,
        "the raw checkerboard is noisy ({raw_var:.1})"
    );

    let normal_tex = scene_pass.texture_named("normal");
    let normal: SampleFn = Rc::new(move |coord| texture_uv(&normal_tex, coord));
    for (label, normal) in [("normals", Some(normal)), ("from depth", None)] {
        let node = denoise(
            &scene_pass.texture(),
            &scene_pass.depth_texture(),
            normal,
            &camera,
        );
        node.depth_phi.set(vec![0.1]);
        pipeline.output_node = Some(node.node());
        let out = frame(&mut pipeline, &mut renderer);

        let noise = region(&out, 4, edge - 8);
        let (var, m) = (variance(&noise), mean(&noise));
        println!(
            "{label}: variance {raw_var:.1} -> {var:.1}, mean {:.1} -> {m:.1}",
            mean(&raw_noise)
        );
        assert!(
            var < raw_var / 4.0,
            "{label}: the checkerboard comes out flatter ({raw_var:.1} -> {var:.1})"
        );
        assert!(
            (m - mean(&raw_noise)).abs() < 12.0,
            "{label}: and keeps its mean ({:.1} -> {m:.1})",
            mean(&raw_noise)
        );

        let white_side = region(&out, edge + 2, SIZE - 2);
        let darkest = white_side.iter().cloned().fold(255.0, f64::min);
        assert!(
            darkest > 245.0,
            "{label}: the white plane stays white next to the edge ({darkest})"
        );
        let near_edge = mean(&region(&out, edge - 3, edge - 1));
        assert!(
            near_edge < m + 12.0,
            "{label}: the checkerboard next to the edge is not pulled to white \
             ({near_edge:.1} against {m:.1})"
        );
    }
}
