//! `temporal_reproject()`, over frames. Three has no page that isolates the
//! node (it is a stage of `webgpu_postprocessing_ssr_denoise`, which is not a
//! rung yet), so the seed and resolve shaders are gated against three's dump
//! in `tests/nodes_display_wgsl.rs` and what the frames do is checked here.
//!
//! The scene is `traa_frames`'s: a white box, turned so its silhouette has
//! slanted edges, in front of a black background, with no MSAA, and an MRT
//! of output, packed view normal and velocity. The node does not jitter the
//! camera and the scene does not move, so every texel of the box reprojects
//! onto itself and every history tap whose previous depth and normal agree is
//! trusted. The pipeline shows the resolve's red channel in red and its
//! alpha — `1 / frameCount`, the weight a downstream accumulating pass gives
//! the current frame — in green, without the output colour transform, so:
//!
//! * the first frame has no usable previous depth, so every tap is rejected
//!   and the history falls back to the beauty: the box or the background,
//!   with `1 / 2` in the box's alpha. Not every pixel: as in `traa_frames`,
//!   variance clipping pulls a one-pixel tip of the silhouette into its
//!   neighbourhood's box;
//! * with `accumulate`, each frame after it trusts the history it copied, so
//!   the box's alpha falls as `1 / (frames + 1)` while its colour, the inside
//!   and the outside stay as they were and nothing goes NaN;
//! * a resize restarts the history: the alpha is back to `1 / 2`;
//! * without `accumulate`, given an all-black history texture, the first
//!   frame (and the one after a resize) reads the freshly seeded internal
//!   history instead, so it is the beauty; from the second frame on, where
//!   the black history is accepted — the box — the output is the black
//!   history pulled part of the way towards the beauty by variance clipping,
//!   and the background stays black.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::box_geometry;
use three_rs::nodes::display::{
    temporal_reproject, TemporalReprojectNode, TemporalReprojectOptions,
};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{float, normal_view, output_property, pack_normal_to_rgb, vec4_join};
use three_rs::nodes::velocity::velocity;
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline,
    RenderTarget, Renderer, RendererParameters, Scene,
};

const SIZE: u32 = 48;

/// Each canvas pixel's red (the resolved colour) and green (its alpha).
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> (u32, u32, Vec<[u8; 2]>) {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    (
        width,
        height,
        pixels.chunks(4).map(|p| [p[0], p[1]]).collect(),
    )
}

fn red(pixels: &[[u8; 2]]) -> Vec<u8> {
    pixels.iter().map(|p| p[0]).collect()
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

/// `value` as an 8-bit channel.
fn unorm(value: f64) -> u8 {
    (value * 255.0).round() as u8
}

fn assert_near(actual: u8, expected: u8, what: &str) {
    assert!(
        actual.abs_diff(expected) <= 2,
        "{what}: {actual}, expected {expected}"
    );
}

/// The pipeline that shows `node`'s red and alpha.
fn view(node: &TemporalReprojectNode) -> RenderPipeline {
    let mut pipeline = RenderPipeline::new();
    node.attach(&mut pipeline);
    let resolved = node.node();
    pipeline.output_node = Some(vec4_join(vec![
        resolved.x(),
        resolved.w(),
        float(0.0),
        float(1.0),
    ]));
    pipeline.output_color_transform = false;
    pipeline
}

fn reproject(
    scene_pass: &PassNode,
    camera: &Rc<RefCell<PerspectiveCamera>>,
    accumulate: bool,
) -> TemporalReprojectNode {
    temporal_reproject(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("normal"),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
        TemporalReprojectOptions {
            accumulate,
            ..TemporalReprojectOptions::default()
        },
    )
}

#[test]
fn temporal_reproject_reprojects_over_frames() {
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
    let mut scene_mrt = mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]);
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_pass.set_mrt(scene_mrt);
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");
    let _ = scene_pass.texture_node("velocity");

    let centre = (SIZE / 2 * SIZE + SIZE / 2) as usize;

    // `accumulate: true`: the node's own history.
    let accumulating = reproject(&scene_pass, &camera, true);
    let mut pipeline = view(&accumulating);
    // The node claimed the view offset, without jittering.
    assert!(!pipeline.claim_view_offset());

    let (width, height, first) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (SIZE, SIZE));
    let beauty = red(&first);
    let first_blended = in_between(&beauty);
    assert!(
        first_blended <= 4,
        "the first frame is the beauty, nearly unblended ({first_blended} pixels in between)"
    );
    assert_on_silhouette(&beauty, &beauty, SIZE);
    assert!(first[centre][0] > 247, "the box covers the centre");
    assert!(first[0][0] < 8, "the corner is background");
    assert_near(
        first[centre][1],
        unorm(0.5),
        "the first frame's alpha in the box",
    );
    assert!(
        camera
            .borrow()
            .view
            .as_ref()
            .is_none_or(|view| !view.enabled),
        "the node does not jitter the camera"
    );

    let mut last = first.clone();
    for frames in 2..=8 {
        last = frame(&mut pipeline, &mut renderer).2;
        assert_near(
            last[centre][1],
            unorm(1.0 / (frames as f64 + 1.0)),
            "the box's alpha as the history ages",
        );
    }
    let last_red = red(&last);
    assert!(last[centre][0] > 247, "the inside of the box stays white");
    assert!(last[0][0] < 8, "the background stays black");
    assert!(
        in_between(&last_red) <= first_blended + 4,
        "a still scene's history keeps its colour"
    );
    assert_on_silhouette(&last_red, &beauty, SIZE);
    // A NaN would land as 0 (or garbage) somewhere the beauty was not.
    let changed = last_red
        .iter()
        .zip(&beauty)
        .filter(|(&now, &then)| now.abs_diff(then) > 8)
        .count();
    assert!(
        changed <= first_blended,
        "eight frames on, the colour is the first frame's ({changed} pixels moved)"
    );
    assert!(
        last.iter()
            .zip(&beauty)
            .filter(|(_, &b)| b > 247)
            .all(|(pixel, _)| pixel[1].abs_diff(unorm(1.0 / 9.0)) <= 2),
        "every box pixel's history is eight frames old"
    );

    // A resize restarts the history at the new size.
    renderer.set_size(SIZE as f64 / 2.0, SIZE as f64 / 2.0);
    let (width, height, restarted) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (SIZE / 2, SIZE / 2));
    let small_centre = (SIZE / 4 * SIZE / 2 + SIZE / 4) as usize;
    let restarted_red = red(&restarted);
    assert!(
        in_between(&restarted_red) <= 4,
        "the frame after a resize is the beauty again"
    );
    assert!(restarted[small_centre][0] > 247);
    assert_near(
        restarted[small_centre][1],
        unorm(0.5),
        "the alpha after a resize, the history restarted",
    );
    accumulating.dispose();

    // `accumulate: false` with an all-black history of the full size.
    renderer.set_size(SIZE as f64, SIZE as f64);
    let black = RenderTarget::new(SIZE, SIZE);
    renderer.init_render_target(&black);
    let external = reproject(&scene_pass, &camera, false);
    external.set_history_texture(Some(&black.texture()));
    let mut pipeline = view(&external);

    let seeded = frame(&mut pipeline, &mut renderer).2;
    let seeded_red = red(&seeded);
    assert!(
        in_between(&seeded_red) <= 4,
        "the first frame reads the seeded internal history, not the black one"
    );
    assert!(seeded[centre][0] > 247);

    let blended = frame(&mut pipeline, &mut renderer).2;
    for (i, (&b, &pixel)) in beauty.iter().zip(&blended).enumerate() {
        if b < 8 {
            assert!(pixel[0] < 8, "pixel {i}: the background stays black");
        }
    }
    // Inside the box the neighbourhood is all white, so its variance box is
    // the one point white compressed by `luminance × flickerSuppression × 10
    // + 1` = 11. The black history clips onto it and is decompressed with the
    // history's own scale, 1, giving 1 / 11; `clampIntensity × max( motion ×
    // 10, 0.25 ) × ( 1 + ( 1 − stretch ) + ( 1 − trust ) )` = 0.5 of the way
    // there is taken. The alpha is `1 / maxFrames`: the black history's alpha
    // of 0 is an infinitely old one.
    assert_near(
        blended[centre][0],
        unorm(0.5 / 11.0),
        "the box: the black history, half clipped towards the beauty",
    );
    assert_near(blended[centre][1], unorm(1.0 / 32.0), "the box's alpha");
    let box_pixels = beauty.iter().filter(|&&b| b > 247).count();
    let accepted = blended
        .iter()
        .zip(&beauty)
        .filter(|(pixel, &b)| b > 247 && pixel[0] < 247)
        .count();
    assert!(
        accepted * 10 >= box_pixels * 9,
        "the black history is accepted across the box ({accepted} of {box_pixels} pixels)"
    );

    // After a resize the stale external history is swapped for the seeded
    // internal one for a frame.
    renderer.set_size(SIZE as f64 / 2.0, SIZE as f64 / 2.0);
    let swapped = red(&frame(&mut pipeline, &mut renderer).2);
    assert!(
        in_between(&swapped) <= 4,
        "the frame after a resize is the beauty"
    );
    assert!(swapped[small_centre] > 247);
}
