//! `ssr()` with `stochastic: true`, over frames. The page that uses it,
//! `webgpu_postprocessing_ssr_denoise`, is not ported yet, so there is no
//! rung; the SSR shader is gated against three's dump in
//! `tests/nodes_display_wgsl.rs`, and what it renders is checked here.
//!
//! A white unlit box standing on a black floor, in front of a black
//! background, with every pixel a mirror (`metalness 1`, `roughness 0`) and
//! no environment, so the only light SSR can find is the box's:
//!
//! * the floor shows the box where its mirror image lands, and nothing
//!   anywhere else: the box's own faces reflect the black floor or miss, and a
//!   miss with no environment is black;
//! * `intensity = 0` zeroes the colour;
//! * each frame advances the noise index, so two consecutive frames differ,
//!   but they are two samples of one reflection, so their means agree;
//! * nothing is NaN, and alpha carries the hit's ray length, which is
//!   positive.
//!
//! The SSR target is read back as half floats, before any tone mapping or
//! colour-space conversion.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::nodes::display::{ssr, SampleFn, SsrNode, SsrOptions};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{float, normal_view, output_property, pack_normal_to_rgb, texture_uv};
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// A pixel's colour reads as lit above this (the box is 1.0).
const LIT: f32 = 0.05;

fn unlit(hex: u32) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(hex);
    material
}

/// One frame of the SSR target: RGBA per pixel, top row first.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer, node: &SsrNode) -> Vec<[f32; 4]> {
    pipeline.render(renderer);
    let (width, height, texels) = renderer
        .read_target_pixels_rgba16f(&node.render_target())
        .unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    texels.chunks(4).map(|t| [t[0], t[1], t[2], t[3]]).collect()
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
/// inside): the largest distance past any edge's line, exact inside a convex
/// polygon and a lower bound outside it.
fn outside(hull: &[(f64, f64)], i: usize) -> f64 {
    let (px, py) = (
        (i as u32 % SIZE) as f64 + 0.5,
        (i as u32 / SIZE) as f64 + 0.5,
    );
    (0..hull.len())
        .map(|k| {
            let (a, b) = (hull[k], hull[(k + 1) % hull.len()]);
            let (ex, ey) = (b.0 - a.0, b.1 - a.1);
            // Anticlockwise: the outward normal of edge (ex, ey) is (ey, -ex).
            ((px - a.0) * ey - (py - a.1) * ex) / ex.hypot(ey)
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

fn brightness(t: &[f32; 4]) -> f32 {
    t[0].max(t[1]).max(t[2])
}

#[test]
fn ssr_stochastic_reflects_the_box_in_the_floor() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let scene = Scene::new();
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), unlit(0x000000));
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

    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(0.0, 1.2, 3.0);
    camera.look_at(&Vector3::new(0.0, 0.0, 0.0));
    camera.update_matrix_world();
    let mirrored = outline(&camera, -1.0, 0.0);
    let real = outline(&camera, 0.0, 1.0);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass = pass(scene, camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_pass.set_mrt(scene_mrt);
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");

    let normal_texture = scene_pass.texture_named("normal");
    let normal: SampleFn =
        Rc::new(move |coord| texture_uv(&normal_texture, coord).mul(2.0).sub(1.0));
    let ssr_node = ssr(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        normal,
        SsrOptions::new(float(1.0), Some(float(0.0))).with_stochastic(true),
        camera,
    );
    assert!(ssr_node.stochastic());
    ssr_node.max_distance().set(vec![5.0]);

    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(ssr_node.node());

    let first = frame(&mut pipeline, &mut renderer, &ssr_node);
    assert!(
        first.iter().flatten().all(|v| v.is_finite()),
        "no SSR texel is NaN or infinite"
    );

    // Light only where the mirrored box lands.
    let lit: Vec<usize> = (0..first.len())
        .filter(|&i| brightness(&first[i]) > LIT)
        .collect();
    for &i in &lit {
        assert!(
            outside(&mirrored, i) < 1.5,
            "pixel ({}, {}) = {:?} is lit outside the box's mirror image",
            i as u32 % SIZE,
            i as u32 / SIZE,
            first[i]
        );
    }
    // The floor inside the mirror image, clear of the box itself and of the
    // outline's edges, is lit, at about the box's brightness.
    let image: Vec<usize> = (0..first.len())
        .filter(|&i| outside(&mirrored, i) < -2.0 && outside(&real, i) > 2.0)
        .collect();
    assert!(
        image.len() > 40,
        "the mirror image covers some floor ({} pixels)",
        image.len()
    );
    let image_lit = image
        .iter()
        .filter(|&&i| brightness(&first[i]) > 0.5)
        .count();
    assert!(
        image_lit * 10 >= image.len() * 9,
        "the mirror image is lit ({image_lit} of {} pixels)",
        image.len()
    );
    // A hit carries its ray length in alpha.
    for &i in &lit {
        assert!(
            first[i][3] > 0.0 && first[i][3] < 5.0,
            "pixel ({}, {}) is a hit with ray length {}",
            i as u32 % SIZE,
            i as u32 / SIZE,
            first[i][3]
        );
    }

    // The noise index advances: the next frame is a different sample of the
    // same reflection.
    let second = frame(&mut pipeline, &mut renderer, &ssr_node);
    assert!(second.iter().flatten().all(|v| v.is_finite()));
    let changed = (0..first.len())
        .filter(|&i| (0..4).any(|c| first[i][c] != second[i][c]))
        .count();
    assert!(changed > 0, "two consecutive frames are identical");
    let mean = |f: &[[f32; 4]]| {
        image
            .iter()
            .map(|&i| f64::from(brightness(&f[i])))
            .sum::<f64>()
            / image.len() as f64
    };
    let (m1, m2) = (mean(&first), mean(&second));
    assert!(
        (m1 - m2).abs() * 255.0 < 3.0,
        "the two frames' means agree within a few levels ({m1} vs {m2})"
    );

    // `intensity = 0` zeroes the colour.
    ssr_node.intensity().set(vec![0.0]);
    let dark = frame(&mut pipeline, &mut renderer, &ssr_node);
    assert!(
        dark.iter()
            .all(|t| t[0] == 0.0 && t[1] == 0.0 && t[2] == 0.0),
        "intensity 0 leaves no colour"
    );
}
