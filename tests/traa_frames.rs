//! `traa()`, over frames. three lists `webgpu_postprocessing_traa` in its
//! e2e exception list, so there is no reference screenshot and no rung; the
//! resolve shader is gated against three's dump in
//! `tests/nodes_display_wgsl.rs`, and what the frames do is checked here.
//!
//! A white box, turned so its silhouette has slanted edges, in front of a
//! black background, with no MSAA. Each frame the camera is jittered by a
//! sub-pixel Halton offset and the frame is blended into the history, so:
//!
//! * the first frame is the jittered beauty, copied into the history as is,
//!   so almost every pixel is either the box or the background. Not every:
//!   variance clipping pulls even an identical history into the mean and
//!   spread of its 3×3 neighbourhood, and at a one-pixel tip of the
//!   silhouette that box excludes the tip's own colour, so the tip is
//!   blended on the first frame too, as it is in three;
//! * over the frames after it, pixels on the silhouette take values in
//!   between, which is the anti-aliasing;
//! * pixels well inside the box and well outside it do not change, so
//!   nothing smears and nothing goes NaN;
//! * a resize restarts the history at the new size, and the frame after it
//!   is again nearly only the box or the background.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::box_geometry;
use three_rs::nodes::display::traa;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::output_property;
use three_rs::nodes::velocity::velocity;
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

const SIZE: u32 = 48;

/// The red channel of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> (u32, u32, Vec<u8>) {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    (width, height, pixels.chunks(4).map(|p| p[0]).collect())
}

/// Pixels that are neither the background nor the box.
fn in_between(red: &[u8]) -> usize {
    red.iter().filter(|&&r| r > 8 && r < 247).count()
}

/// Every in-between pixel of `frame` sits on the silhouette: within two
/// pixels of one that `reference` drew as the box and one it drew as the
/// background.
fn assert_on_silhouette(frame: &[u8], reference: &[u8], size: u32) {
    let size = size as i32;
    let at =
        |x: i32, y: i32| reference[(y.clamp(0, size - 1) * size + x.clamp(0, size - 1)) as usize];
    for (i, &r) in frame.iter().enumerate() {
        if r > 8 && r < 247 {
            let (x, y) = (i as i32 % size, i as i32 / size);
            let near: Vec<u8> = (-2..=2)
                .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| at(x + dx, y + dy))
                .collect();
            assert!(
                near.iter().any(|&n| n > 247) && near.iter().any(|&n| n < 8),
                "pixel ({x}, {y}) = {r} is blended but not on the silhouette"
            );
        }
    }
}

#[test]
fn traa_anti_aliases_over_frames() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let scene = Scene::new();
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    let mesh = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    mesh.borrow_mut().set_rotation(0.0, 0.0, 0.3);
    scene.add(&mesh);

    let camera = PerspectiveCamera::new(70.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 2.5;

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene, camera.clone());
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]));
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");

    let mut pipeline = RenderPipeline::new();
    let traa_node = traa(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
    );
    traa_node.attach(&mut pipeline);
    // A second node in the same pipeline does not jitter the camera again.
    assert!(!pipeline.claim_view_offset());
    pipeline.output_node = Some(traa_node.node());

    let (width, height, first) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (SIZE, SIZE));
    let first_blended = in_between(&first);
    assert!(
        first_blended <= 4,
        "the first frame is the jittered beauty, nearly unblended ({first_blended} pixels in between)"
    );
    assert_on_silhouette(&first, &first, SIZE);
    let centre = (SIZE / 2 * SIZE + SIZE / 2) as usize;
    assert!(first[centre] > 247, "the box covers the centre");
    assert!(first[0] < 8, "the corner is background");

    // The camera's view offset is cleared after every frame.
    assert!(
        camera
            .borrow()
            .view
            .as_ref()
            .is_some_and(|view| !view.enabled),
        "the jitter was set, and is cleared"
    );

    let mut last = first.clone();
    for _ in 0..15 {
        last = frame(&mut pipeline, &mut renderer).2;
    }
    let edge = in_between(&last);
    assert!(
        edge >= first_blended + 8,
        "after sixteen jittered frames the silhouette is blended ({edge} pixels in between)"
    );
    assert!(last[centre] > 247, "the inside of the box stays white");
    assert!(last[0] < 8, "the background stays black");
    assert_on_silhouette(&last, &first, SIZE);

    // A resize restarts the history at the new size.
    renderer.set_size(SIZE as f64 / 2.0, SIZE as f64 / 2.0);
    let (width, height, restarted) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (SIZE / 2, SIZE / 2));
    let restarted_blended = in_between(&restarted);
    assert!(
        restarted_blended <= 4,
        "the frame after a resize starts the history again ({restarted_blended} pixels in between)"
    );
    assert_on_silhouette(&restarted, &restarted, SIZE / 2);
}
