//! `fsr1()`, over frames. Its page, `webgpu_upscaling_fsr1`, is not graded
//! (three fails its own reference for it on this machine); the EASU and RCAS
//! shaders are gated against three's dump in `tests/nodes_display_wgsl.rs`,
//! and what the two passes do to a frame is checked here.
//!
//! The scene is a hard vertical edge: a grey 0.25 background, and a plane in
//! colour 0.75 covering the right half of the view exactly, so the edge falls
//! on a texel boundary of the half-resolution scene pass. The pass renders at
//! `setResolutionScale( 0.5 )`, as the page's does, and the frame is read
//! straight to the canvas (`outputColorTransform = false`). So:
//!
//! * both FSR targets, and the canvas, are at canvas size while the scene
//!   pass is at half of it;
//! * away from the edge the colour does not move: EASU's weights are
//!   normalised and RCAS's lobe vanishes where the cross is flat, so a
//!   constant comes through as itself (pixels whose footprints straddle the
//!   edge, and the two-pixel border, whose unclamped `textureLoad`s may read
//!   off the texture, are left out);
//! * the edge is steeper than the bilinear upscale of the same pass texture:
//!   bilinear spreads a texel step over two output pixels at a quarter and
//!   three quarters, so its steepest step across a row is half the contrast,
//!   and FSR's edge-directed Lanczos lobe plus RCAS beat that;
//! * EASU alone (sharpness 30, where RCAS's lobe is ~1e-9) stays inside the
//!   edge's range, being clamped to its nearest four texels, and is steeper
//!   than bilinear but less steep than with RCAS, which may undershoot;
//! * after a resize both targets follow the drawing buffer;
//! * the free `fsr1()` of a plain colour node, which draws it through its own
//!   `convertToTexture()`, gives back that colour.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::math::Color;
use three_rs::nodes::display::{fsr1, Fsr1Node};
use three_rs::nodes::tsl::{float, texture_uv, uv, vec3, vec4};
use three_rs::objects::Background;
use three_rs::{
    pass, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

/// The red channel of every canvas pixel, rows top to bottom.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer, size: u32) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (size, size));
    pixels.chunks(4).map(|p| p[0]).collect()
}

/// The pixels off the two-pixel border, as `( x, y, value )`.
fn interior(red: &[u8], size: u32) -> impl Iterator<Item = (u32, u32, u8)> + '_ {
    red.iter().enumerate().filter_map(move |(i, &r)| {
        let (x, y) = (i as u32 % size, i as u32 / size);
        (x > 1 && y > 1 && x < size - 2 && y < size - 2).then_some((x, y, r))
    })
}

/// The steepest step between neighbours along each interior row, summed
/// over the rows.
fn steepness(red: &[u8], size: u32) -> u32 {
    (2..size - 2)
        .map(|y| {
            let row = &red[(y * size) as usize..((y + 1) * size) as usize];
            row.windows(2)
                .map(|w| u32::from(w[0].abs_diff(w[1])))
                .max()
                .unwrap()
        })
        .sum()
}

#[test]
fn fsr1_upscales_a_hard_edge() {
    const SIZE: u32 = 32;
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    // A 90° camera one unit away sees x in -1..1; the plane covers 0..2.
    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::new(0.25, 0.25, 0.25)));
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(vec3(0.75, 0.75, 0.75));
    let plane = Mesh::new(Rc::new(plane_geometry(2.0, 4.0, 1, 1)), material);
    plane.borrow_mut().position.x = 1.0;
    scene.add(&plane);
    let camera = PerspectiveCamera::new(90.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 1.0;
    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass = pass(scene, camera);
    scene_pass.set_resolution_scale(0.5);
    let mut pipeline = RenderPipeline::new();
    pipeline.output_color_transform = false;

    // Bilinear: the half-size pass texture sampled at the canvas's uv.
    pipeline.output_node = Some(texture_uv(&scene_pass.texture(), uv()));
    let bilinear = frame(&mut pipeline, &mut renderer, SIZE);
    assert_eq!(scene_pass.texture().size(), (SIZE / 2, SIZE / 2));
    let (dark, light) = (
        bilinear[(16 * SIZE + 4) as usize],
        bilinear[(16 * SIZE + 27) as usize],
    );
    assert!(
        dark.abs_diff(64) <= 1 && light.abs_diff(191) <= 1,
        "the edge runs {dark} to {light}"
    );
    let bilinear_ramp: Vec<u8> = (13..19)
        .map(|x| bilinear[(16 * SIZE + x) as usize])
        .collect();
    assert!(
        bilinear_ramp[2] > dark + 20 && bilinear_ramp[3] + 20 < light,
        "bilinear spreads the step over columns 15 and 16: {bilinear_ramp:?}"
    );

    let fsr = Fsr1Node::new(
        &scene_pass.texture(),
        float(Fsr1Node::DEFAULT_SHARPNESS),
        false,
    );
    pipeline.output_node = Some(fsr.node());
    let upscaled = frame(&mut pipeline, &mut renderer, SIZE);
    assert_eq!(fsr.easu_texture().size(), (SIZE, SIZE));
    assert_eq!(fsr.texture().size(), (SIZE, SIZE));

    // Columns whose EASU taps (two texels either side) and RCAS cross are
    // all on one side of the edge, at texel 7 | 8 of the pass.
    for (x, y, r) in interior(&upscaled, SIZE) {
        let want = if x <= 10 {
            dark
        } else if x >= 21 {
            light
        } else {
            continue;
        };
        assert!(r.abs_diff(want) <= 1, "flat ({x}, {y}) is {r}, want {want}");
    }
    let (sharp, soft) = (steepness(&upscaled, SIZE), steepness(&bilinear, SIZE));
    let ramp: Vec<u8> = (13..19)
        .map(|x| upscaled[(16 * SIZE + x) as usize])
        .collect();
    assert!(
        sharp > soft + soft / 10,
        "FSR's steepest steps sum to {sharp}, bilinear's to {soft}; \
         row 16 runs {ramp:?} against {bilinear_ramp:?}"
    );

    // EASU alone: at sharpness 30 RCAS's lobe is `exp2( -30 )`, so the
    // output is the EASU target. EASU clamps each pixel to its nearest four
    // texels, so nothing leaves the edge's range (RCAS may undershoot it),
    // and it is already steeper than bilinear, but less steep than with RCAS.
    let easu_only = Fsr1Node::new(&scene_pass.texture(), float(30.0), false);
    pipeline.output_node = Some(easu_only.node());
    let easu = frame(&mut pipeline, &mut renderer, SIZE);
    for (x, y, r) in interior(&easu, SIZE) {
        assert!(
            r + 1 >= dark && r <= light + 1,
            "EASU ({x}, {y}) is {r}, outside {dark}..{light}"
        );
    }
    let easu_steep = steepness(&easu, SIZE);
    let easu_ramp: Vec<u8> = (13..19).map(|x| easu[(16 * SIZE + x) as usize]).collect();
    assert!(
        easu_steep > soft && easu_steep < sharp,
        "EASU's steepest steps sum to {easu_steep}, between bilinear's {soft} and \
         FSR's {sharp}; row 16 runs {easu_ramp:?}"
    );
    pipeline.output_node = Some(fsr.node());

    // A resize: both targets follow the drawing buffer.
    let size = 48;
    renderer.set_size(f64::from(size), f64::from(size));
    let resized = frame(&mut pipeline, &mut renderer, size);
    assert_eq!(fsr.easu_texture().size(), (size, size));
    assert_eq!(fsr.texture().size(), (size, size));
    assert_eq!(scene_pass.texture().size(), (size / 2, size / 2));
    let (left, right) = (
        resized[(24 * size + 4) as usize],
        resized[(24 * size + 43) as usize],
    );
    assert!(
        left.abs_diff(dark) <= 1 && right.abs_diff(light) <= 1,
        "at 48 px the edge runs {left} to {right}"
    );

    // The free `fsr1()`: its own `convertToTexture()` of a plain colour.
    let flat = fsr1(vec4(0.5, 0.5, 0.5, 1.0), 0.0, false);
    pipeline.output_node = Some(flat.node());
    let grey = frame(&mut pipeline, &mut renderer, size);
    for (x, y, r) in interior(&grey, size) {
        assert!(r.abs_diff(128) <= 1, "fsr1( 0.5 ) at ({x}, {y}) is {r}");
    }
}
