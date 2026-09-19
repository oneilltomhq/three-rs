//! Half-float textures and render targets, and the render target's viewport —
//! the renderer half of the first sitting of `webgpu_pmrem_cubemap`.
//!
//! PMREM needs three things this covers, and each of them fails silently: an
//! `rgba16float` render target it can read back, half-float source textures
//! (the HDR cube, and the 2-D `DataTexture` the equirect cousin uses) uploaded
//! with the right stride, and a viewport so that 21 passes can tile two shared
//! 768×1024 atlases. A four-byte stride on an eight-byte texel shears a face
//! diagonally; a bottom-left viewport convention puts every mip in the wrong
//! tile. Both look like plausible environments.
//!
//! The values are exact, not approximate: every number used here is
//! representable in binary16, the target is `rgba16float`, the samplers are at
//! texel centres, and `from_half_float` is exact — so a texel that has been
//! through the upload, the sample and the readback must come back as the half
//! that went in.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use three_rs::extras::to_half_float;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::tsl::{cube_texture, texture_uv, uv, vec3, vec4};
use three_rs::renderer::{RenderTarget, RenderTargetOptions};
use three_rs::textures::{CubeTexture, Image, MinFilter};
use three_rs::{QuadMesh, Renderer, RendererParameters, Texture, TextureFilter, TextureType};

const SIZE: u32 = 8;

/// A depth-less `rgba16float` target — `RenderTarget( w, h, { type:
/// HalfFloatType } )`, which is what `PMREMGenerator._allocateTargets()` makes
/// (`depthBuffer: false`, `format: RGBAFormat`, `type: HalfFloatType`).
fn half_float_target(width: u32, height: u32) -> RenderTarget {
    RenderTarget::new_with_options(
        width,
        height,
        RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: false,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )
    .expect("HalfFloatType is a colour type")
}

/// The `( r, g, b, a )` at `( x, y )` of a readback, `y` down from the top.
fn texel(pixels: &[f32], width: u32, x: u32, y: u32) -> [f32; 4] {
    let i = ((y * width + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// A quad whose fragment is exactly `node`, rendered into `target`.
fn render_node(renderer: &mut Renderer, target: &RenderTarget, node: three_rs::nodes::NodeRef) {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(node);
    material.depth_test = false;
    material.depth_write = false;

    renderer.set_render_target(Some(target.clone()));
    renderer.render_quad(&QuadMesh::new(material));
    renderer.set_render_target(None);
}

/// Four halves a texel, in the order the faces of a 2×2 cube are written.
fn halves(values: [f64; 4]) -> [u16; 4] {
    [
        to_half_float(values[0]),
        to_half_float(values[1]),
        to_half_float(values[2]),
        to_half_float(values[3]),
    ]
}

#[test]
fn half_float_targets_textures_and_the_viewport() {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();

    a_formatted_target_reads_back_as_halves(&mut renderer);
    the_viewport_clips_the_draw_to_the_top_right(&mut renderer);
    a_half_float_texture_survives_the_upload(&mut renderer);
    a_half_float_cube_keeps_its_faces_and_rows(&mut renderer);
}

/// The target really is `rgba16float`, and a value no 8-bit target could hold
/// comes back intact. All four are exactly representable in binary16 — at 1024
/// the ulp is 1, and `0.0009765625` is `2^-10` — and they are far outside
/// `[0, 1]`, so an `rgba8unorm` target would clamp them to 1 and 0 and an
/// `rgba8unorm-srgb` one would put a transfer function on them as well.
fn a_formatted_target_reads_back_as_halves(renderer: &mut Renderer) {
    let target = half_float_target(SIZE, SIZE);
    render_node(renderer, &target, vec4(1024.0, -3.25, 0.0009765625, 1.0));

    let (width, height, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    assert_eq!(pixels.len() as u32, SIZE * SIZE * 4);

    for y in 0..SIZE {
        for x in 0..SIZE {
            assert_eq!(
                texel(&pixels, SIZE, x, y),
                [1024.0, -3.25, 0.0009765625, 1.0],
                "({x}, {y})"
            );
        }
    }

    // And the RGBA8 readback refuses it rather than handing back garbage.
    assert!(renderer.read_target_pixels(&target).is_err());
}

/// `renderTarget.viewport` restricts the draw to one tile of the target, in
/// wgpu's **top-left** convention — the same one three.js' WebGPU backend uses,
/// since `WebGPUBackend.updateViewport()` passes `renderTarget.viewport`
/// straight to `setViewport` with no flip.
///
/// This is the assertion that separates the two conventions: the viewport is
/// the top-right quadrant, so the lit texels must be at small `y`. With GL's
/// bottom-left origin the same numbers would light the *bottom*-right quadrant,
/// and every PMREM mip would land one tile-row off — a plausible, wrong,
/// silently blurred environment.
fn the_viewport_clips_the_draw_to_the_top_right(renderer: &mut Renderer) {
    let half = SIZE / 2;
    let target = half_float_target(SIZE, SIZE);
    target.set_viewport(half as f64, 0.0, half as f64, half as f64);

    render_node(renderer, &target, vec4(0.5, 0.25, 0.125, 1.0));

    let (_, _, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();

    for y in 0..SIZE {
        for x in 0..SIZE {
            let inside = x >= half && y < half;
            let expected = if inside {
                [0.5, 0.25, 0.125, 1.0]
            } else {
                // The clear covers the whole attachment — `LoadOp::Clear` is
                // not clipped by the viewport, which is what makes the untouched
                // three quadrants readable at all.
                [0.0, 0.0, 0.0, 0.0]
            };
            assert_eq!(texel(&pixels, SIZE, x, y), expected, "({x}, {y})");
        }
    }

    // `RenderTarget.setSize()` puts the viewport back to the whole target, as
    // three.js does, so a resized target does not keep a stale tile.
    target.set_size(SIZE, SIZE * 2);
    assert_eq!(target.viewport().z, SIZE as f64);
    assert_eq!(target.viewport().w, (SIZE * 2) as f64);
}

/// A 2×2 `rgba16float` `DataTexture` uploaded, sampled at each texel centre and
/// read back. The stride is eight bytes a texel; at four the rows would shear.
fn a_half_float_texture_survives_the_upload(renderer: &mut Renderer) {
    // Row v = 0 first: `flipY` is false on a `DataTexture`, so the first row of
    // the array is the first row of the image.
    let mut data = Vec::new();
    for value in [[1024.0, 0.0, 0.0, 1.0], [0.0, 2048.0, 0.0, 1.0]] {
        data.extend_from_slice(&halves(value));
    }
    for value in [[0.0, 0.0, 4096.0, 1.0], [8.0, 16.0, 32.0, 1.0]] {
        data.extend_from_slice(&halves(value));
    }

    let map = Texture::data_rgba16float(2, 2, &data);
    assert_eq!(map.format(), wgpu::TextureFormat::Rgba16Float);
    // Nearest, so each quadrant of the quad is exactly one texel with no
    // interpolation across the seam.
    map.set_min_filter(MinFilter::Nearest);
    map.set_mag_filter(TextureFilter::Nearest);

    let target = half_float_target(SIZE, SIZE);
    render_node(renderer, &target, texture_uv(&map, uv()));

    let (_, _, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();

    // `uv()` on the full-screen triangle has v = 0 at the *top* of the
    // framebuffer, so the first row of the array is the top half of the target.
    let quarter = SIZE / 4;
    assert_eq!(
        texel(&pixels, SIZE, quarter, quarter),
        [1024.0, 0.0, 0.0, 1.0],
        "texel ( 0, 0 )"
    );
    assert_eq!(
        texel(&pixels, SIZE, SIZE - quarter, quarter),
        [0.0, 2048.0, 0.0, 1.0],
        "texel ( 1, 0 )"
    );
    assert_eq!(
        texel(&pixels, SIZE, quarter, SIZE - quarter),
        [0.0, 0.0, 4096.0, 1.0],
        "texel ( 0, 1 )"
    );
    assert_eq!(
        texel(&pixels, SIZE, SIZE - quarter, SIZE - quarter),
        [8.0, 16.0, 32.0, 1.0],
        "texel ( 1, 1 )"
    );
}

/// The six faces of a half-float `CubeTexture` reach their own layers, right
/// way up.
///
/// Each face is 2×2 and every texel carries its own face index and its own
/// `( column, row )`, so one sample per face at the `( 0, 0 )` texel's centre
/// pins the layer order *and* the row order. `HDRCubeTextureLoader` builds
/// exactly this shape from the six pisa `.hdr` files; the values here are
/// synthetic so the test needs no asset.
fn a_half_float_cube_keeps_its_faces_and_rows(renderer: &mut Renderer) {
    let images = (0..6)
        .map(|face| {
            let mut data = Vec::new();
            for row in 0..2 {
                for column in 0..2 {
                    data.extend_from_slice(&halves([
                        (face + 1) as f64,
                        column as f64 + 1.0,
                        row as f64 + 1.0,
                        1.0,
                    ]));
                }
            }
            Image::rgba16float(2, 2, &data)
        })
        .collect();

    let cube = CubeTexture::new(images);
    cube.set_texture_type(TextureType::HalfFloat).unwrap();
    cube.set_generate_mipmaps(false);
    cube.set_filters(MinFilter::Linear, TextureFilter::Linear);

    // The direction that lands on the centre of texel ( 0, 0 ) of each face,
    // in the GPU's cube space, from the cube-map projection:
    // `u = ( sc / |ma| + 1 ) / 2`, `v = ( tc / |ma| + 1 ) / 2`, so u = v = 0.25
    // means sc = tc = -|ma| / 2.
    let directions = [
        (1.0, 0.5, 0.5),   // +X: sc = -z, tc = -y
        (-1.0, 0.5, -0.5), // -X: sc =  z, tc = -y
        (-0.5, 1.0, -0.5), // +Y: sc =  x, tc =  z
        (-0.5, -1.0, 0.5), // -Y: sc =  x, tc = -z
        (-0.5, 0.5, 1.0),  // +Z: sc =  x, tc = -y
        (0.5, 0.5, -1.0),  // -Z: sc = -x, tc = -y
    ];

    for (face, (x, y, z)) in directions.into_iter().enumerate() {
        let target = half_float_target(SIZE, SIZE);
        // `cube_texture()` is three.js' node, which negates x on the way in
        // (`WGSLNodeBuilder.generateTextureSample`'s `vec3( -uv.x, uv.yz )`) —
        // three's cube convention is mirrored against the GPU's. Undo it here
        // so the directions above stay in the space the layer order is
        // defined in.
        render_node(renderer, &target, cube_texture(&cube, vec3(-x, y, z)));

        let (_, _, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();
        let sampled = texel(&pixels, SIZE, SIZE / 2, SIZE / 2);

        assert_eq!(
            sampled[0],
            (face + 1) as f32,
            "direction ({x}, {y}, {z}) sampled layer {} instead of {face}",
            sampled[0] - 1.0
        );
        // Column 0, row 0 — a vertical flip of the face would read row 1 here.
        assert_eq!([sampled[1], sampled[2]], [1.0, 1.0], "face {face} texel");
    }
}
