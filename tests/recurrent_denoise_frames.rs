//! `recurrentDenoise()`, over frames. Its quad shaders are gated against
//! three's dump in `tests/nodes_display_wgsl.rs`; what the frames do is
//! checked here.
//!
//! A unit box seen face on, in front of an empty background, unlit grey
//! whose beauty is modulated per pixel by a hash of the pixel and a
//! per-frame seed: ±10 % white-ish noise on a flat region, with depth and
//! normals from the same pass. The denoiser (`'diffuse'`, three's defaults,
//! `accumulate`) reads the beauty as `raw`. Its input is a history texture
//! the test fills from the denoiser's own output after every frame, with the
//! alpha advanced from `1 / n` to `1 / ( n + 1 )` — what
//! `TemporalReprojectNode` does between the page's frames, here with a still
//! camera so the reprojection is the identity. The history starts as alpha
//! 1, so the first frame's input says "one frame accumulated".
//!
//! * the first frame's denoised face has well under the input's variance,
//!   and keeps its mean within a few levels;
//! * the silhouette between the face and the background stays where it is
//!   (to within two pixels): the background is discarded, and a tap there
//!   is far from the face's plane, so it weighs nothing;
//! * over the frames after it, each with fresh noise, the face's variance
//!   keeps dropping, and nothing is NaN;
//! * a resize restarts the target at the new size, without a panic.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::box_geometry;
use three_rs::nodes::display::{recurrent_denoise, RecurrentDenoiseOptions, SampleFn};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    float, hash, normal_view, output_property, pack_normal_to_rgb, screen_size, screen_uv,
    texture_uv, uniform_settable, vec3, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::Texture;
use three_rs::{
    pass, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
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

/// The pixels within `r` of the centre: the middle of the face.
fn flat_region(r: u32) -> impl Iterator<Item = usize> {
    let c = SIZE / 2;
    (c - r..c + r).flat_map(move |y| (c - r..c + r).map(move |x| (y * SIZE + x) as usize))
}

/// Mean and variance of the red channel over the flat region.
fn stats(red: &[u8]) -> (f64, f64) {
    let values: Vec<f64> = flat_region(10).map(|i| f64::from(red[i])).collect();
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, variance)
}

/// Every pixel whose coverage (face or background) differs between `frame`
/// and `reference` is within two pixels of `reference`'s silhouette.
fn assert_silhouette_kept(frame: &[u8], reference: &[u8]) {
    let size = SIZE as i32;
    let covered = |red: &[u8], x: i32, y: i32| {
        red[(y.clamp(0, size - 1) * size + x.clamp(0, size - 1)) as usize] > 16
    };
    let mut face = 0;
    for y in 0..size {
        for x in 0..size {
            face += usize::from(covered(reference, x, y));
            if covered(frame, x, y) != covered(reference, x, y) {
                let near: Vec<bool> = (-2..=2)
                    .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| covered(reference, x + dx, y + dy))
                    .collect();
                assert!(
                    near.contains(&true) && near.contains(&false),
                    "pixel ({x}, {y}) changed coverage away from the silhouette"
                );
            }
        }
    }
    assert!(
        face > 600,
        "the face covers a good part of the frame ({face} px)"
    );
}

/// The history texture's next contents: the denoiser's output with its
/// frame weight advanced by one frame (`1 / ( 1 / a + 1 )`), resampled to
/// the history's size.
fn advance_history(output: (u32, u32, Vec<f32>), history: &Texture) {
    let (width, height, texels) = output;
    let (hw, hh) = history.size();
    let mut data = Vec::with_capacity((hw * hh * 4) as usize);
    for y in 0..hh {
        for x in 0..hw {
            let (sx, sy) = (x * width / hw, y * height / hh);
            let t = &texels[((sy * width + sx) * 4) as usize..][..4];
            let a = if t[3] > 0.0 {
                1.0 / (1.0 / t[3] + 1.0)
            } else {
                1.0
            };
            data.extend_from_slice(&[t[0], t[1], t[2], a]);
        }
    }
    history.set_data(data.iter().flat_map(|v| v.to_ne_bytes()).collect());
}

#[test]
fn recurrent_denoise_filters_over_frames() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    // Grey, times 0.9..1.1 by a hash of the pixel and the frame's seed.
    let (seed, seed_value) = uniform_settable(Type::F32, vec![0.0]);
    let pixel = screen_uv().mul(screen_size()).floor();
    let noise = hash(
        pixel
            .x()
            .add(pixel.y().mul(1024.0))
            .add(seed.mul(1_048_576.0)),
    );
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(vec3(0.5, 0.5, 0.5).mul(float(0.9).add(noise.mul(0.2))));
    let scene = Scene::new();
    let mesh = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    // Turned off the view axis: a normal exactly along it degenerates the
    // diffuse kernel's tangent (`normalize( cross( up, n ) )` of a zero
    // vector), in three as here.
    mesh.borrow_mut().set_rotation(0.3, 0.4, 0.0);
    scene.add(&mesh);

    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 2.0;
    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass = pass(scene, camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_pass.set_mrt(scene_mrt);
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");

    let history = Texture::data_rgba32float(
        SIZE,
        SIZE,
        &[0.0, 0.0, 0.0, 1.0].repeat((SIZE * SIZE) as usize),
    );
    let normal: SampleFn = {
        let normal = scene_pass.texture_named("normal");
        Rc::new(move |coord| texture_uv(&normal, coord))
    };
    let denoise = recurrent_denoise(
        &history,
        camera,
        RecurrentDenoiseOptions {
            depth: Some(scene_pass.depth_texture()),
            normal: Some(normal),
            raw: Some(scene_pass.texture()),
            ..Default::default()
        },
    );

    let mut beauty_view = RenderPipeline::new();
    beauty_view.output_node = Some(scene_pass.node());
    let mut denoise_view = RenderPipeline::new();
    // The colour only: the alpha is the frame weight for the next frame.
    denoise_view.output_node = Some(vec4_join(vec![denoise.node().xyz(), float(1.0)]));

    // The first frame.
    let beauty = frame(&mut beauty_view, &mut renderer);
    let (beauty_mean, beauty_variance) = stats(&beauty);
    assert!(beauty[0] < 8, "the corner is background");
    assert!(
        beauty_variance > 10.0,
        "the input is noisy (variance {beauty_variance:.1})"
    );
    let first = frame(&mut denoise_view, &mut renderer);
    let (first_mean, first_variance) = stats(&first);
    println!(
        "input mean {beauty_mean:.2} variance {beauty_variance:.2}; \
         first frame mean {first_mean:.2} variance {first_variance:.2}"
    );
    assert!(
        first_variance < beauty_variance * 0.25,
        "one denoise frame cuts the variance ({first_variance:.2} against {beauty_variance:.2})"
    );
    assert!(
        (first_mean - beauty_mean).abs() < 3.0,
        "the denoised face keeps its mean ({first_mean:.2} against {beauty_mean:.2})"
    );
    assert_silhouette_kept(&first, &beauty);

    // Fresh noise every frame, accumulated through the history.
    let mut variances = vec![first_variance];
    let mut last = first;
    for n in 1..=12 {
        let output = renderer
            .read_target_pixels_rgba16f(denoise.render_target())
            .unwrap();
        assert!(
            output.2.iter().all(|v| v.is_finite()),
            "frame {n}: no denoised texel is NaN or infinite"
        );
        advance_history(output, &history);
        seed_value.set(vec![f64::from(n)]);
        last = frame(&mut denoise_view, &mut renderer);
        if n % 4 == 0 {
            variances.push(stats(&last).1);
        }
    }
    println!("variance at frames 1, 5, 9, 13: {variances:.2?}");
    for pair in variances.windows(2) {
        assert!(
            pair[1] < pair[0],
            "the variance keeps dropping as frames accumulate ({variances:.2?})"
        );
    }
    let (last_mean, _) = stats(&last);
    assert!(
        (last_mean - beauty_mean).abs() < 3.0,
        "the accumulated face keeps its mean ({last_mean:.2} against {beauty_mean:.2})"
    );
    assert_silhouette_kept(&last, &beauty);

    // A resize restarts the target at the new size.
    renderer.set_size(f64::from(SIZE / 2), f64::from(SIZE / 2));
    denoise_view.render(&mut renderer);
    assert_eq!(denoise.render_target().size(), (SIZE / 2, SIZE / 2));
    let output = renderer
        .read_target_pixels_rgba16f(denoise.render_target())
        .unwrap();
    assert!(
        output.2.iter().all(|v| v.is_finite()),
        "after the resize no denoised texel is NaN or infinite"
    );
    advance_history(output, &history);
    denoise_view.render(&mut renderer);
}
