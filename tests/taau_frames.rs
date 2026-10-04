//! `taau()`, over frames. three's own reference for its one page,
//! `webgpu_upscaling_taau`, is one it misses on this machine, so the page's
//! rung is ignored; the seed and resolve shaders are gated against three's
//! dump in `tests/nodes_display_wgsl.rs`, and what the frames do is checked
//! here.
//!
//! A white box, turned so its silhouette has slanted edges, in front of a
//! black background, rendered by a pass at half the canvas's resolution
//! with `mrt( { output, velocity } )`, and upsampled by TAAU to the canvas.
//! A second, plain pass of the same scene at the same scale, sampled
//! bilinearly at the canvas's UVs, is the "plain resolve" the frames are
//! compared with: it is exactly what TAAU's seed writes into the history.
//!
//! * The output is the canvas's size, twice the input's.
//! * The first frame is unjittered, as three's is, and resolves against the
//!   seeded history, so it is the plain resolve: equal to it away from the
//!   silhouette, and off it only on the silhouette, where the background's
//!   pixels — disoccluded against the still-empty previous depth — take the
//!   9-tap reconstruction instead of the bilinear seed.
//! * Over the jittered frames after it a static scene converges: late
//!   frames hardly differ from the next, and the inside of the box and the
//!   background stay put.
//! * When the box moves, the pixels it left go back to the background on the
//!   frame it leaves them: the background behind is disoccluded, so no ghost
//!   is left behind.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::box_geometry;
use three_rs::nodes::display::taau;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{output_property, texture_uv, uv};
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

/// Whether pixel `i` has, within `radius`, both a pixel `reference` draws as
/// the box and one it draws as the background.
fn near_silhouette(i: usize, reference: &[u8], radius: i32) -> bool {
    let size = SIZE as i32;
    let (x, y) = (i as i32 % size, i as i32 / size);
    let at =
        |x: i32, y: i32| reference[(y.clamp(0, size - 1) * size + x.clamp(0, size - 1)) as usize];
    let near: Vec<u8> = (-radius..=radius)
        .flat_map(|dy| (-radius..=radius).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| at(x + dx, y + dy))
        .collect();
    near.iter().any(|&n| n > 247) && near.iter().any(|&n| n < 8)
}

fn abs_diff(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(&a, &b)| a.abs_diff(b)).collect()
}

#[test]
fn taau_upsamples_over_frames() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let scene = Scene::new();
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    let mesh = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    mesh.borrow_mut().set_rotation(0.0, 0.0, 0.3);
    mesh.borrow_mut().position.x = -0.6;
    scene.add(&mesh);

    let camera = PerspectiveCamera::new(70.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 2.5;

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    // The plain resolve: the same scene at the same scale, sampled
    // bilinearly at the canvas's UVs, as the seed samples it.
    let plain_pass = pass(scene.clone(), camera.clone());
    plain_pass.set_resolution_scale(0.5);
    let mut plain = RenderPipeline::new();
    plain.output_node = Some(texture_uv(&plain_pass.texture(), uv()));

    let scene_pass = pass(scene, camera.clone());
    scene_pass.set_resolution_scale(0.5);
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("velocity", velocity()),
    ]));
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");

    let mut pipeline = RenderPipeline::new();
    let taau_node = taau(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
    );
    taau_node.attach(&mut pipeline);
    pipeline.output_node = Some(taau_node.node());

    let (_, _, reference) = frame(&mut plain, &mut renderer);
    let (width, height, first) = frame(&mut pipeline, &mut renderer);

    // The output is the canvas's size; the input is half of it.
    assert_eq!((width, height), (SIZE, SIZE));
    assert_eq!(scene_pass.texture().size(), (SIZE / 2, SIZE / 2));
    assert_eq!(taau_node.texture().size(), (SIZE, SIZE));

    // The first frame is the plain resolve, off it by more than a step only
    // within the 3×3 input neighbourhood of the silhouette (three output
    // pixels), and only at a handful of pixels.
    let diff = abs_diff(&first, &reference);
    let off: Vec<usize> = (0..diff.len()).filter(|&i| diff[i] > 1).collect();
    eprintln!(
        "first frame against the plain resolve: {} pixels off by more than a step, at most {}",
        off.len(),
        diff.iter().max().unwrap()
    );
    for &i in &off {
        assert!(
            near_silhouette(i, &reference, 3),
            "pixel ({}, {}) = {} differs from the plain resolve ({}) off the silhouette",
            i as u32 % SIZE,
            i as u32 / SIZE,
            first[i],
            reference[i]
        );
    }
    assert!(
        off.len() <= 16,
        "{} pixels differ from the plain resolve",
        off.len()
    );
    let centre = (SIZE / 2 * SIZE + SIZE / 2 - SIZE / 4) as usize;
    assert!(first[centre] > 247, "the box covers the left half's centre");
    assert!(first[0] < 8, "the corner is background");

    // A static scene converges: after more than one Halton cycle a frame
    // hardly differs from the next.
    let mut last = first.clone();
    let mut previous = first.clone();
    for _ in 0..47 {
        previous = last;
        last = frame(&mut pipeline, &mut renderer).2;
    }
    let step = abs_diff(&previous, &last);
    let max_step = *step.iter().max().unwrap();
    let total_step: u32 = step.iter().map(|&d| d as u32).sum();
    eprintln!("frame 47 against 48: at most {max_step}, {total_step} in total");
    assert!(
        max_step <= 8,
        "a converged frame moves a pixel by {max_step}"
    );
    assert!(
        total_step <= SIZE * SIZE / 4,
        "a converged frame moves {total_step} steps in total"
    );
    assert!(last[centre] > 247, "the inside of the box stays white");
    assert!(last[0] < 8, "the background stays black");
    for (i, &r) in last.iter().enumerate() {
        if r > 8 && r < 247 {
            assert!(
                near_silhouette(i, &reference, 3),
                "pixel ({}, {}) = {r} is blended but not on the silhouette",
                i as u32 % SIZE,
                i as u32 / SIZE
            );
        }
    }

    // Move the box to the right half, 0.4 a frame. What it leaves is
    // disoccluded background, so the trail is black on every frame.
    let converged = last;
    let mut moved = Vec::new();
    for _ in 0..3 {
        mesh.borrow_mut().position.x += 0.4;
        moved = frame(&mut pipeline, &mut renderer).2;
    }
    let (_, _, moved_reference) = frame(&mut plain, &mut renderer);
    let trail: Vec<usize> = (0..moved.len())
        .filter(|&i| {
            converged[i] > 247 && moved_reference[i] < 8 && !near_silhouette(i, &moved_reference, 3)
        })
        .collect();
    let ghost = trail.iter().map(|&i| moved[i]).max().unwrap_or(0);
    eprintln!("trail: {} pixels, brightest {ghost}", trail.len());
    assert!(trail.len() >= 100, "the box left {} pixels", trail.len());
    assert!(ghost < 8, "the box left a ghost of {ghost}");
    let moved_centre = centre + (SIZE / 2) as usize;
    assert!(moved[moved_centre] > 247, "the box is at its new place");
}
