//! `retroPass()`, drawn. `webgpu_postprocessing_retro`'s e2e rung is ignored
//! (three fails its own reference on this machine), and the composition
//! quads are gated against three's dumps in `tests/nodes_display_wgsl.rs`;
//! what the pass itself does to a frame is checked here.
//!
//! A one-unit plane straight in front of a 90° camera covers the middle half
//! of a 64×64 canvas. Its classic (no node slots) material maps a 64×64
//! texture whose red channel is a three-texel checkerboard and whose blue
//! channel is `v`. Behind it, a node background whose colour is
//! `normalWorld.z`, built in a deferred `Fn` as `ps1Background` is. So:
//!
//! * the pass renders at a quarter of the canvas with nearest filtering, so
//!   the composed frame is made of uniform 4×4 blocks, where a plain pass of
//!   the same scene is not (the background is a smooth gradient);
//! * `filterTextures = false` samples level 0, so the checker stays pure
//!   black and white; `true` samples the mip the footprint asks for, which
//!   averages the checker to grey;
//! * the background is drawn back-facing, so `normalWorld` is flipped and the
//!   sky straight ahead is bright, not clamped to black;
//! * `affineDistortion` 1 interpolates the uv without perspective, which on a
//!   tilted plane moves `v`, and 0 draws exactly what it drew before;
//! * the vertices snap to the quarter-size grid, so moving the plane by less
//!   than half a texel changes nothing at all, where a plain pass moves its
//!   edge;
//! * a small node-material plane with a `colorNode`, top right, is drawn as
//!   itself: three's property copy puts its `colorNode` back over
//!   `materialColor`;
//! * `barrelMask( barrelUV( -0.1 ) )` over the pass blacks out the middle of
//!   each edge, which a pincushion pulls from outside the frame, and keeps the
//!   corners.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::display::{barrel_mask, barrel_uv, retro_pass, RetroPassOptions};
use three_rs::nodes::tsl::{
    call, float, inline_fn, normal_world, texture_uv, uniform_settable, uv, vec3, vec3_join,
    vec4_join,
};
use three_rs::nodes::Type;
use three_rs::objects::Background;
use three_rs::textures::{MinFilter, TextureFilter};
use three_rs::{
    pass, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, Texture,
};

const SIZE: u32 = 64;

/// Every canvas pixel, RGBA.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<[u8; 4]> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| [p[0], p[1], p[2], p[3]]).collect()
}

fn at(pixels: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    pixels[(y * SIZE + x) as usize]
}

/// The 4×4 blocks of the canvas that hold more than one colour.
fn mixed_blocks(pixels: &[[u8; 4]]) -> usize {
    let mut mixed = 0;
    for by in 0..SIZE / 4 {
        for bx in 0..SIZE / 4 {
            let first = at(pixels, bx * 4, by * 4);
            if (0..16).any(|i| at(pixels, bx * 4 + i % 4, by * 4 + i / 4) != first) {
                mixed += 1;
            }
        }
    }
    mixed
}

/// The pixels the plane covers when it faces the camera: the middle 32×32.
fn plane_pixels(pixels: &[[u8; 4]]) -> Vec<[u8; 4]> {
    (16..48)
        .flat_map(|y| (16..48).map(move |x| (x, y)))
        .map(|(x, y)| at(pixels, x, y))
        .collect()
}

#[test]
fn retro_pass_frames() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    // Red: a checker of three-texel squares, which the eight-texel steps
    // between the pass's pixel centres do not alias to one colour. Blue: `v`, rising down the image rows.
    let mut data = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64u32 {
        for x in 0..64u32 {
            let checker = if (x / 3 + y / 3) % 2 == 0 { 255 } else { 0 };
            data.extend_from_slice(&[checker, 0, (y * 4) as u8, 255]);
        }
    }
    let map = Texture::new(64, 64, Some(data));
    map.set_mag_filter(TextureFilter::Nearest);
    map.set_min_filter(MinFilter::NearestMipmapNearest);

    let mut scene = Scene::new();
    // `normalWorld.z` in a deferred `Fn()`, run inside the background's own
    // (back-sided) material.
    scene.background = Some(Background::Node(call(
        &inline_fn(0, Type::Vec3, |_| {
            let z = normal_world().z();
            vec3_join(vec![z.clone(), z.clone(), z])
        }),
        Vec::new(),
    )));
    let mut material = MeshBasicNodeMaterial::new();
    material.map = Some(map);
    let mesh = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    scene.add(&mesh);

    // A node material: `colorNode` set, so three's for-in copies it (and the
    // null `vertexNode` and `contextNode`) over the retro material's. At
    // NDC 0.6..0.85 on both axes it covers texel columns 13 and 14 of 16 and
    // rows 1 and 2 from the top; as a classic material it would be white.
    let mut green = MeshBasicNodeMaterial::new();
    green.color_node = Some(vec3(0.0, 1.0, 0.0));
    let corner = Mesh::new(Rc::new(plane_geometry(0.25, 0.25, 1, 1)), green);
    corner.borrow_mut().position.x = 0.725;
    corner.borrow_mut().position.y = 0.725;
    scene.add(&corner);

    let camera = PerspectiveCamera::new(90.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 1.0;

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let (affine, affine_value) = uniform_settable(Type::F32, vec![0.0]);
    let retro = retro_pass(
        scene.clone(),
        camera.clone(),
        RetroPassOptions::default().affine_distortion(affine),
    );
    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(retro.node());

    // A quarter of the canvas, nearest filtered: uniform 4×4 blocks.
    let first = frame(&mut pipeline, &mut renderer);
    assert_eq!(retro.pass().texture().size(), (SIZE / 4, SIZE / 4));
    assert_eq!(mixed_blocks(&first), 0, "every 4×4 block is one texel");

    // The same scene through a plain pass is not blocky.
    let plain = pass(scene.clone(), camera.clone());
    let mut plain_pipeline = RenderPipeline::new();
    plain_pipeline.output_node = Some(texture_uv(&plain.texture(), uv()));
    let full = frame(&mut plain_pipeline, &mut renderer);
    let full_mixed = mixed_blocks(&full);
    assert!(
        full_mixed > 16,
        "a full-resolution pass has blocks of more than one colour ({full_mixed})"
    );

    // The background is back-facing: straight ahead its flipped normal
    // points at the camera, so `normalWorld.z` is positive there.
    for (x, y) in [(32, 1), (1, 32), (62, 32), (32, 62)] {
        let [r, ..] = at(&first, x, y);
        assert!(r > 150, "the sky at ({x}, {y}) is bright ({r})");
    }

    // `filterTextures = false`: level 0, so the checker is pure.
    let plane = plane_pixels(&first);
    let pure = plane.iter().filter(|p| p[0] < 16 || p[0] > 239).count();
    assert_eq!(
        pure,
        plane.len(),
        "level 0 keeps the checker black and white"
    );
    assert!(plane.iter().any(|p| p[0] > 239) && plane.iter().any(|p| p[0] < 16));

    // The node material keeps its own `colorNode`: green, not the white
    // `materialColor` a classic source gets.
    for (x, y) in [(53, 5), (58, 10)] {
        let [r, g, b, _] = at(&first, x, y);
        assert!(
            r < 4 && g > 251 && b < 4,
            "the node material at ({x}, {y}) is its own green ({r}, {g}, {b})"
        );
    }

    // Moved right by 0.03 world units, which at one unit from a 90° camera is
    // 0.03 in NDC: 0.24 of a quarter-size texel, 0.96 of a canvas pixel. The
    // snapped vertices round back to the same texel corners, so the frame is
    // unchanged, the checker texels sampled included. Without the snap the
    // edge would cover the same texels (a texel centre is half a texel from
    // either corner) but each texel would sample the map about two texels
    // further left. The full-resolution plain pass moves its left edge off
    // column 16.
    mesh.borrow_mut().position.x = 0.03;
    let shifted = frame(&mut pipeline, &mut renderer);
    let changed = shifted.iter().zip(&first).filter(|(a, b)| a != b).count();
    assert_eq!(changed, 0, "a sub-texel move snaps back to the same frame");
    let full_moved = frame(&mut plain_pipeline, &mut renderer);
    assert_ne!(
        (16..48).map(|y| at(&full_moved, 16, y)).collect::<Vec<_>>(),
        (16..48).map(|y| at(&full, 16, y)).collect::<Vec<_>>(),
        "a plain pass moves the plane's left edge"
    );
    mesh.borrow_mut().position.x = 0.0;

    // `filterTextures = true`: the footprint is eight texels a pixel, whose
    // mip is the checker's average.
    retro.set_filter_textures(true);
    let filtered = frame(&mut pipeline, &mut renderer);
    assert_eq!(mixed_blocks(&filtered), 0);
    let grey = plane_pixels(&filtered)
        .iter()
        .filter(|p| p[0] > 150 && p[0] < 225)
        .count();
    assert_eq!(grey, 32 * 32, "mipmapped, the checker averages to grey");
    retro.set_filter_textures(false);
    assert_eq!(frame(&mut pipeline, &mut renderer), first);

    // Tilted away at the top, the plane is foreshortened, and affine
    // interpolation moves `v`.
    mesh.borrow_mut().set_rotation(-0.8, 0.0, 0.0);
    let perspective = frame(&mut pipeline, &mut renderer);
    affine_value.set(vec![1.0]);
    let affine = frame(&mut pipeline, &mut renderer);
    let moved = perspective
        .iter()
        .zip(&affine)
        .filter(|(a, b)| a[2].abs_diff(b[2]) > 8)
        .count();
    assert!(
        moved >= 64,
        "affine mapping moves the uv on a tilted plane ({moved} pixels)"
    );
    assert_eq!(mixed_blocks(&affine), 0);
    affine_value.set(vec![0.0]);
    assert_eq!(frame(&mut pipeline, &mut renderer), perspective);
    mesh.borrow_mut().set_rotation(0.0, 0.0, 0.0);

    // `barrelMask( barrelUV( curvature ) )` over the pass. A negative
    // curvature is a pincushion: the corners stay put and the middle of each
    // edge reads from outside the unit square, which the mask blacks out.
    let mask = barrel_mask(barrel_uv(float(-0.1), uv()));
    let mut masked_pipeline = RenderPipeline::new();
    masked_pipeline.output_node = Some(vec4_join(vec![retro.node().rgb().mul(mask), float(1.0)]));
    let masked = frame(&mut masked_pipeline, &mut renderer);
    for (x, y) in [(0, 32), (63, 32), (32, 0), (32, 63)] {
        assert_eq!(
            at(&masked, x, y)[..3],
            [0, 0, 0],
            "the edge's middle at ({x}, {y}) is outside the barrel"
        );
    }
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63), (32, 32)] {
        assert_eq!(
            at(&masked, x, y)[..3],
            at(&first, x, y)[..3],
            "({x}, {y}) is inside the barrel"
        );
    }
}
