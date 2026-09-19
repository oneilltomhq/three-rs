//! Numeric gates for the PMREM generator, against oracles taken from three.js
//! itself.
//!
//! Every table below was printed by running three's own code (r186, the vendor
//! tree) rather than by transcribing its arithmetic:
//!
//! * the GGX ladder and the atlas rectangles came from driving a real
//!   `PMREMGenerator` — `_setSize( 256 )`, `_allocateTarget( false )`,
//!   `_init( … )` — and calling `_applyGGXFilter( target, i - 1, i )` for
//!   `i` in `1 .. _lodMeshes.length` with `_setViewport` and
//!   `renderer.render` replaced by recorders, so the numbers are the ones
//!   three would have written into its uniforms;
//! * the cubeUV sizes came from `_generateCubeUVSize` in `PMREMNode.js`.
//!
//! The shaders are gated separately, by diffing `examples/dump_wgsl.rs`'s
//! output against the scout's capture of three's WGSL.

use three_rs::nodes::pmrem_node::generate_cube_uv_size;
use three_rs::renderer::pmrem::{create_planes, ggx_step, tile_rect, LOD_MIN};

/// `_setSize( 256 )`: `lodMax = floor( log2( 256 ) )`, `cubeSize = 2 ^ lodMax`.
const LOD_MAX: usize = 8;
const CUBE_SIZE: usize = 256;
/// `_lodMeshes.length` — `lodMax - LOD_MIN + 1 + EXTRA_LODS`.
const LOD_COUNT: usize = 11;

/// `_createPlanes( 8 ).sizeLods`.
#[test]
fn plane_sizes_match_three() {
    let sizes: Vec<usize> = create_planes(LOD_MAX).iter().map(|m| m.size).collect();
    assert_eq!(sizes, vec![256, 128, 64, 32, 16, 16, 16, 16, 16, 16, 16]);
    assert_eq!(sizes.len(), LOD_COUNT);
}

/// `_allocateTarget`: `3 * max( cubeSize, 16 * 7 ) x 4 * cubeSize`.
#[test]
fn atlas_size_matches_three() {
    assert_eq!(3 * CUBE_SIZE.max(16 * 7), 768);
    assert_eq!(4 * CUBE_SIZE, 1024);
}

/// `lodIn`, `lodOut`, `ggxUniforms.roughness.value` and
/// `ggxUniforms.mipInt.value` for the filter pass, `mipInt` for the copy-back,
/// then the viewport `x`, `y` and the LOD's plane size.
///
/// `mipInt` goes negative for the extra LODs, which is why the port does the
/// subtraction in floating point: `lodIn` runs up to 9 against a `lodMax` of 8.
struct Step {
    lod_in: usize,
    lod_out: usize,
    roughness: f64,
    mip_in: f64,
    mip_out: f64,
    x: usize,
    y: usize,
    size: usize,
}

const GGX_LADDER: [Step; 10] = [
    Step {
        lod_in: 0,
        lod_out: 1,
        roughness: 0.0125,
        mip_in: 8.0,
        mip_out: 7.0,
        x: 0,
        y: 512,
        size: 128,
    },
    Step {
        lod_in: 1,
        lod_out: 2,
        roughness: 0.04330127018922194,
        mip_in: 7.0,
        mip_out: 6.0,
        x: 0,
        y: 768,
        size: 64,
    },
    Step {
        lod_in: 2,
        lod_out: 3,
        roughness: 0.0838525491562421,
        mip_in: 6.0,
        mip_out: 5.0,
        x: 0,
        y: 896,
        size: 32,
    },
    Step {
        lod_in: 3,
        lod_out: 4,
        roughness: 0.13228756555322957,
        mip_in: 5.0,
        mip_out: 4.0,
        x: 0,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 4,
        lod_out: 5,
        roughness: 0.18749999999999994,
        mip_in: 4.0,
        mip_out: 3.0,
        x: 48,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 5,
        lod_out: 6,
        roughness: 0.24874685927665496,
        mip_in: 3.0,
        mip_out: 2.0,
        x: 96,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 6,
        lod_out: 7,
        roughness: 0.315485736603099,
        mip_in: 2.0,
        mip_out: 1.0,
        x: 144,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 7,
        lod_out: 8,
        roughness: 0.3872983346207419,
        mip_in: 1.0,
        mip_out: 0.0,
        x: 192,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 8,
        lod_out: 9,
        roughness: 0.4638493828819867,
        mip_in: 0.0,
        mip_out: -1.0,
        x: 240,
        y: 960,
        size: 16,
    },
    Step {
        lod_in: 9,
        lod_out: 10,
        roughness: 0.5448623679425841,
        mip_in: -1.0,
        mip_out: -2.0,
        x: 288,
        y: 960,
        size: 16,
    },
];

#[test]
fn ggx_ladder_matches_three() {
    for step in &GGX_LADDER {
        let (lod_in, lod_out) = (step.lod_in, step.lod_out);
        let (got_roughness, got_mip) = ggx_step(LOD_MAX, LOD_COUNT, lod_in, lod_out);
        // Bit-exact: the same sequence of f64 operations on the same inputs.
        assert_eq!(
            got_roughness, step.roughness,
            "adjustedRoughness for {lod_in} -> {lod_out}"
        );
        assert_eq!(
            got_mip, step.mip_in,
            "mipInt for the {lod_in} -> {lod_out} filter"
        );
        assert_eq!(
            LOD_MAX as f64 - lod_out as f64,
            step.mip_out,
            "mipInt for the {lod_in} -> {lod_out} copy-back"
        );
    }
}

#[test]
fn atlas_rectangles_match_three() {
    let sizes: Vec<usize> = create_planes(LOD_MAX).iter().map(|m| m.size).collect();
    for step in &GGX_LADDER {
        let (lod_out, x, y, size) = (step.lod_out, step.x, step.y, step.size);
        assert_eq!(sizes[lod_out], size, "sizeLods[{lod_out}]");
        let (got_x, got_y, got_size) = tile_rect(LOD_MAX, CUBE_SIZE, size, lod_out);
        assert_eq!((got_x, got_y, got_size), (x, y, size), "tile {lod_out}");
        // Both passes of a step write the same 3x2-tile rectangle, the first
        // into the ping-pong target and the second back into the atlas.
        assert_eq!((3 * got_size, 2 * got_size), (3 * size, 2 * size));
        assert!(
            x + 3 * size <= 768 && y + 2 * size <= 1024,
            "tile {lod_out} fits"
        );
    }
    // Mip 0 is the full-width band at the top: `_textureToCubeUV` writes it
    // directly rather than through `_applyGGXFilter`.
    assert_eq!(tile_rect(LOD_MAX, CUBE_SIZE, CUBE_SIZE, 0), (0, 0, 256));
    // `LOD_MIN` is what puts the extra LODs in their own columns: lod 4 is the
    // last one in column 0.
    assert_eq!(LOD_MIN, 4);
}

/// `_generateCubeUVSize( imageHeight )` for the atlas heights a 1024², 512²,
/// 256², 128² and 64² PMREM has. The `7 * 16` floor is why the last three all
/// share a `texelWidth`.
#[test]
fn cube_uv_size_matches_three() {
    for &(height, texel_width, texel_height, max_mip) in &[
        (1024u32, 0.0013020833333333333, 0.0009765625, 8.0),
        (512, 0.0026041666666666665, 0.001953125, 7.0),
        (256, 0.002976190476190476, 0.00390625, 6.0),
        (128, 0.002976190476190476, 0.0078125, 5.0),
        (64, 0.002976190476190476, 0.015625, 4.0),
    ] {
        assert_eq!(
            generate_cube_uv_size(height),
            (texel_width, texel_height, max_mip),
            "_generateCubeUVSize( {height} )"
        );
    }
}
