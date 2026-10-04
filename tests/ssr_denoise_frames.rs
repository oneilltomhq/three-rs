//! `webgpu_postprocessing_ssr_denoise`'s whole chain, over frames: the
//! stochastic `ssr()` into `temporalReproject()` (`'specular'`) into
//! `recurrentDenoise()` (`'specular'`, `accumulate`, ray-length alpha), the
//! denoised texture fed back as SSR's history and as the reprojection's,
//! the denoised reflection added to the beauty, and `sharpen( traa( … ) )`
//! over it, with TRAA owning the view offset. The page's shaders are gated
//! against three's dump in `tests/nodes_display_wgsl.rs`; the page itself is
//! ungraded (`docs/webgpu_postprocessing_ssr_denoise-progress.md`), so what
//! the chain renders is checked here.
//!
//! A white unlit box standing on a rough metal floor (`metalness 1`,
//! `roughness 0.25`, the page's patched lighting so it has no environment
//! specular), against a black background, under an ambient light of no
//! intensity and no environment, so the only light anywhere on the floor is
//! the box's
//! reflection, and every stochastic SSR sample is noisy:
//!
//! * the first frame has no NaN or infinity in the SSR or denoise targets
//!   (the denoiser reads the reprojection, so a NaN there shows in its
//!   output);
//! * the floor inside the box's mirror image is lit on the canvas, and the
//!   floor well away from it is dark;
//! * over the next frames the reflection stays where it is (the lit
//!   pixels' centroid moves by under a pixel, and the image's mean by under
//!   a dozen levels), while the denoised floor's frame-to-frame noise, the mean
//!   per-pixel variance over a window of frames, falls;
//! * a resize restarts the chain's targets at the new size, and the next
//!   frames are finite.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::nodes::display::{
    convert_to_texture, recurrent_denoise, sharpen, ssr, temporal_reproject, traa,
    DenoiseAlphaSource, DenoiseMode, RecurrentDenoiseOptions, SampleFn, SsrOptions,
    TemporalReprojectMode, TemporalReprojectOptions,
};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    diffuse_color, float, material_metalness_value, material_roughness_value, normal_view,
    output_property, pack_normal_to_rgb, texture_uv, to_var, vec2_join, vec4_join,
};
use three_rs::nodes::velocity::velocity;
use three_rs::textures::TextureType;
use three_rs::{
    pass, AmbientLight, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline,
    Renderer, RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// Frames per window when measuring the per-pixel noise.
const WINDOW: usize = 4;

fn unlit(hex: u32) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(hex);
    material
}

/// The canvas position (in pixels, y down) a world point projects to.
fn project(camera: &PerspectiveCamera, [x, y, z]: [f64; 3]) -> (f64, f64) {
    let mut p = Vector3::new(x, y, z);
    p.project(camera);
    (
        (p.x + 1.0) * 0.5 * f64::from(SIZE),
        (1.0 - p.y) * 0.5 * f64::from(SIZE),
    )
}

/// The screen outline of the axis-aligned box `[-0.5, 0.5] × [y0, y1] ×
/// [-0.5, 0.5]`: the convex hull of its projected corners, anticlockwise in
/// pixel coordinates.
fn outline(camera: &PerspectiveCamera, y0: f64, y1: f64) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = [-0.5, 0.5]
        .iter()
        .flat_map(|&x| [y0, y1].map(move |y| (x, y)))
        .flat_map(|(x, y)| [-0.5, 0.5].map(move |z| [x, y, z]))
        .map(|corner| project(camera, corner))
        .collect();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Andrew's monotone chain.
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f64, f64)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &(f64, f64)>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// How far the centre of pixel `i` is outside `hull`, in pixels (negative
/// inside): the largest distance past any edge's line.
fn outside(hull: &[(f64, f64)], i: usize) -> f64 {
    let (px, py) = (
        (i as u32 % SIZE) as f64 + 0.5,
        (i as u32 / SIZE) as f64 + 0.5,
    );
    (0..hull.len())
        .map(|k| {
            let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            ((px - a.0) * ey - (py - a.1) * ex) / ex.hypot(ey)
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

/// What one frame leaves behind: the canvas's brightest channel per pixel,
/// and the denoised reflection's.
struct Frame {
    canvas: Vec<f64>,
    denoised: Vec<f64>,
}

fn brightest(chunk: &[f32]) -> f64 {
    f64::from(chunk[0].max(chunk[1]).max(chunk[2]))
}

fn assert_finite(renderer: &mut Renderer, target: &three_rs::renderer::RenderTarget, what: &str) {
    let (_, _, texels) = renderer.read_target_pixels_rgba16f(target).unwrap();
    assert!(
        texels.iter().all(|v| v.is_finite()),
        "no {what} texel is NaN or infinite"
    );
}

/// Mean over `pixels` of each pixel's variance across `frames`.
fn per_pixel_variance(frames: &[Frame], pixels: &[usize]) -> f64 {
    let n = frames.len() as f64;
    pixels
        .iter()
        .map(|&i| {
            let mean = frames.iter().map(|f| f.denoised[i]).sum::<f64>() / n;
            frames
                .iter()
                .map(|f| (f.denoised[i] - mean).powi(2))
                .sum::<f64>()
                / n
        })
        .sum::<f64>()
        / pixels.len() as f64
}

/// The centroid of the canvas pixels in `region` brighter than `level`, and
/// how many there are.
fn lit_centroid(canvas: &[f64], region: &[usize], level: f64) -> ((f64, f64), usize) {
    let lit: Vec<usize> = region
        .iter()
        .copied()
        .filter(|&i| canvas[i] > level)
        .collect();
    let n = lit.len().max(1) as f64;
    let x = lit.iter().map(|&i| f64::from(i as u32 % SIZE)).sum::<f64>() / n;
    let y = lit.iter().map(|&i| f64::from(i as u32 / SIZE)).sum::<f64>() / n;
    ((x, y), lit.len())
}

#[test]
fn ssr_denoise_chain_converges_in_place() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let scene = Scene::new();
    let mut floor_material = MeshBasicNodeMaterial::standard(Color::from_hex(0xffffff), 0.25, 1.0);
    floor_material.environment_specular = false;
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), floor_material);
    floor
        .borrow_mut()
        .set_rotation(-std::f64::consts::FRAC_PI_2, 0.0, 0.0);
    scene.add(&floor);
    let cube = Mesh::new(
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        unlit(0xffffff),
    );
    cube.borrow_mut().position.y = 0.5;
    scene.add(&cube);
    // A light of no intensity: without any light `NodeMaterial.setupLighting()`
    // leaves the floor's outgoing light at its diffuse colour, unlit white.
    // With one, the lighting model runs, and a metal under an ambient light
    // with no environment is black.
    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 0.0));

    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(0.0, 1.2, 3.0);
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));
    camera.update_matrix_world();
    let mirrored = outline(&camera, -1.0, 0.0);
    let real = outline(&camera, 0.0, 1.0);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    // The page's scene pass: its MRT, and its texture nodes in its order.
    let scene_pass = pass(scene, camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("diffuseColor", || {
        vec4_join(vec![diffuse_color().rgb(), material_metalness_value()])
    });
    scene_mrt.set_deferred("normal", || {
        vec4_join(vec![
            pack_normal_to_rgb(normal_view()).rgb(),
            material_roughness_value(),
        ])
    });
    scene_mrt.set("velocity", velocity());
    scene_pass.set_mrt(scene_mrt);
    let scene_pass_color = scene_pass.texture_node("output");
    let scene_pass_normal = scene_pass.texture_node("normal");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");
    let scene_pass_diffuse = scene_pass.texture_node("diffuseColor");
    let normal_texture = scene_pass.texture_named("normal");
    normal_texture.set_texture_type(TextureType::UnsignedByte);
    let diffuse_texture = scene_pass.texture_named("diffuseColor");
    diffuse_texture.set_texture_type(TextureType::UnsignedByte);
    let velocity_texture = scene_pass.texture_named("velocity");
    let depth_texture = scene_pass.depth_texture();

    let scene_normal: SampleFn = {
        let normal_texture = normal_texture.clone();
        Rc::new(move |coord| texture_uv(&normal_texture, coord).rgb().mul(2.0).sub(1.0))
    };
    let scene_metal_rough: SampleFn = {
        let (diffuse_texture, normal_texture) = (diffuse_texture.clone(), normal_texture.clone());
        Rc::new(move |coord: three_rs::nodes::NodeRef| {
            vec2_join(vec![
                texture_uv(&diffuse_texture, coord.clone()).w(),
                texture_uv(&normal_texture, coord).w(),
            ])
        })
    };
    let scene_diffuse: SampleFn = {
        let diffuse_texture = diffuse_texture.clone();
        Rc::new(move |coord| texture_uv(&diffuse_texture, coord))
    };
    let scene_normal_packed: SampleFn = {
        let normal_texture = normal_texture.clone();
        Rc::new(move |coord| texture_uv(&normal_texture, coord))
    };

    let ssr_node = ssr(
        &scene_pass.texture(),
        &depth_texture,
        scene_normal,
        SsrOptions::new(scene_pass_diffuse.w(), Some(scene_pass_normal.w()))
            .with_stochastic(true)
            .with_diffuse(scene_diffuse)
            .with_binary_refine(true),
        camera.clone(),
    );
    ssr_node.max_distance().set(vec![5.0]);
    let temporal_reproject_node = temporal_reproject(
        &ssr_node.render_target().texture(),
        &depth_texture,
        &normal_texture,
        &velocity_texture,
        camera.clone(),
        TemporalReprojectOptions {
            mode: TemporalReprojectMode::Specular,
            accumulate: false,
            ..TemporalReprojectOptions::default()
        },
    );
    let denoise_node = recurrent_denoise(
        &temporal_reproject_node.texture(),
        camera.clone(),
        RecurrentDenoiseOptions {
            depth: Some(depth_texture.clone()),
            normal: Some(scene_normal_packed),
            raw: Some(ssr_node.render_target().texture()),
            metal_roughness: Some(scene_metal_rough),
            mode: DenoiseMode::Specular,
            accumulate: true,
            ..RecurrentDenoiseOptions::default()
        },
    );
    denoise_node.set_alpha_source(DenoiseAlphaSource::RayLength);
    ssr_node.set_history(&denoise_node.texture(), &velocity_texture);
    temporal_reproject_node.set_history_texture(Some(&denoise_node.texture()));

    let blend = vec4_join(vec![
        denoise_node.node().rgb(),
        to_var(None, ssr_node.node().w().greater_than(0.0)),
    ]);
    let combined = vec4_join(vec![scene_pass_color.rgb().add(blend.rgb()), float(1.0)]);
    let beauty = convert_to_texture(combined);
    let traa_node = traa(
        &beauty.texture(),
        &depth_texture,
        &velocity_texture,
        camera.clone(),
    );
    let mut pipeline = RenderPipeline::new();
    traa_node.attach(&mut pipeline);
    temporal_reproject_node.attach(&mut pipeline);
    let sharpen_node = sharpen(traa_node.node(), 0.0, false);
    pipeline.output_node = Some(sharpen_node.node());
    pipeline.output_color_transform = false;

    let mut render = |renderer: &mut Renderer| -> Frame {
        pipeline.render(renderer);
        let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
        let canvas = pixels
            .chunks(4)
            .map(|p| f64::from(p[0].max(p[1]).max(p[2])) / 255.0)
            .collect();
        let (dw, dh, texels) = renderer
            .read_target_pixels_rgba16f(denoise_node.render_target())
            .unwrap();
        assert_eq!((width, height), (dw, dh));
        assert!(
            texels.iter().all(|v| v.is_finite()),
            "no denoised texel is NaN or infinite"
        );
        let denoised = texels.chunks(4).map(brightest).collect();
        Frame { canvas, denoised }
    };

    // The first frame.
    let first = render(&mut renderer);
    assert_finite(&mut renderer, &ssr_node.render_target(), "SSR");

    // The floor inside the mirror image, clear of the box and of the
    // outline's edges; and the floor well away from it, below the box's
    // image in the frame's lower corners.
    let image: Vec<usize> = (0..first.canvas.len())
        .filter(|&i| outside(&mirrored, i) < -2.0 && outside(&real, i) > 2.0)
        .collect();
    assert!(
        image.len() > 40,
        "the mirror image covers some floor ({} pixels)",
        image.len()
    );
    let far: Vec<usize> = (0..first.canvas.len())
        .filter(|&i| i as u32 / SIZE > SIZE * 3 / 4)
        .filter(|&i| outside(&mirrored, i) > 8.0)
        .collect();
    assert!(!far.is_empty(), "some floor is far from the mirror image");
    let mean = |values: &[f64], pixels: &[usize]| {
        pixels.iter().map(|&i| values[i]).sum::<f64>() / pixels.len() as f64
    };
    let image_mean = mean(&first.canvas, &image);
    let far_mean = mean(&first.canvas, &far);
    println!("first frame: image mean {image_mean:.3}, far floor mean {far_mean:.3}");
    assert!(
        image_mean > 0.3,
        "the floor shows the box's reflection (mean {image_mean:.3})"
    );
    assert!(
        far_mean < 0.05,
        "the floor away from the reflection is dark (mean {far_mean:.3})"
    );
    let (first_centroid, first_lit) = lit_centroid(&first.canvas, &image, 0.5 * image_mean);

    // The frames after it.
    let mut frames = vec![first];
    for _ in 1..(4 * WINDOW) {
        frames.push(render(&mut renderer));
    }
    let variances: Vec<f64> = frames
        .chunks(WINDOW)
        .map(|window| per_pixel_variance(window, &image))
        .collect();
    println!("per-pixel variance over windows of {WINDOW} frames: {variances:.6?}");
    assert!(
        variances[variances.len() - 1] < variances[0] * 0.5,
        "the floor's per-pixel noise falls ({variances:.6?})"
    );
    for (n, frame) in frames.iter().enumerate().skip(1) {
        let (centroid, lit) = lit_centroid(&frame.canvas, &image, 0.5 * image_mean);
        let moved = (centroid.0 - first_centroid.0).hypot(centroid.1 - first_centroid.1);
        assert!(
            moved < 1.0,
            "frame {n}: the reflection stays put (its centroid moved {moved:.2} px)"
        );
        assert!(
            lit * 10 >= first_lit * 9,
            "frame {n}: the reflection keeps its extent ({lit} of {first_lit} pixels lit)"
        );
        let frame_mean = mean(&frame.canvas, &image);
        assert!(
            (frame_mean - image_mean).abs() * 255.0 < 12.0,
            "frame {n}: the reflection keeps its brightness ({frame_mean:.3} against {image_mean:.3})"
        );
    }

    // A resize restarts the chain at the new size.
    renderer.set_size(f64::from(SIZE / 2), f64::from(SIZE / 2));
    for _ in 0..2 {
        pipeline.render(&mut renderer);
        assert_eq!(denoise_node.render_target().size(), (SIZE / 2, SIZE / 2));
        assert_eq!(ssr_node.render_target().size(), (SIZE / 2, SIZE / 2));
        assert_finite(
            &mut renderer,
            denoise_node.render_target(),
            "resized denoise",
        );
        assert_finite(&mut renderer, &ssr_node.render_target(), "resized SSR");
    }
}
