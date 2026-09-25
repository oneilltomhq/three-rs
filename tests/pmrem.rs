//! Numeric gates for the PMREM generator, against oracles taken from three.js
//! itself.
//!
//! Every table below was printed by node against the vendor tree at 5f610f5
//! (after 2f80402 replaced the cubeUV atlas with a mipmapped cube):
//! `PMREMGenerator.lodToRoughness` is the class's own static,
//! `floorPowerOfTwo` is `MathUtils`', and the `lodBias` and integration
//! `sourceLod` columns evaluate `_applyPMREM()`'s two expressions for each
//! level. `roughnessToMip` is `PMREMUtils.js`' after b745e6c dropped the
//! base-level texel correction.
//!
//! The shaders are gated separately, by diffing `examples/dump_wgsl.rs`'s
//! output against three's WGSL.

use three_rs::nodes::pmrem_utils::roughness_to_mip_value;
use three_rs::renderer::pmrem::{
    cube_size_for, lod_bias, lod_to_roughness, max_lod_for, uses_integration, INTEGRATION_SIZE,
};

/// `_setSize( n )` — `max( 256, floorPowerOfTwo( n ) )`.
#[test]
fn cube_size_matches_three() {
    for &(requested, size) in &[
        (1u32, 256u32),
        (100, 256),
        (255, 256),
        (256, 256),
        (257, 256),
        (511, 256),
        (512, 512),
        (1000, 512),
        (1024, 1024),
        (2048, 2048),
    ] {
        assert_eq!(cube_size_for(requested), size, "_setSize( {requested} )");
    }
}

/// One level of `_applyPMREM()`: its roughness, whether the integration
/// material fills it, and the uniform that material reads — `lodBias` for
/// GGX, `sourceLod` for the integration.
struct Level {
    roughness: f64,
    integration: bool,
    bias_or_source_lod: f64,
}

const fn level(roughness: f64, integration: bool, bias_or_source_lod: f64) -> Level {
    Level {
        roughness,
        integration,
        bias_or_source_lod,
    }
}

const LADDER_256: [Level; 6] = [
    level(0.0, false, 0.0),
    level(0.10557280900008414, false, 12.279860827031644),
    level(0.2254033307585166, false, 10.091319771730024),
    level(0.3675444679663241, true, 4.0),
    level(0.5527864045000421, true, 4.0),
    level(1.0, true, 4.0),
];

const LADDER_512: [Level; 7] = [
    level(0.0, false, 0.0),
    level(0.0871290708247231, false, 13.833885314256298),
    level(0.18350341907227397, false, 11.684723552334452),
    level(0.2928932188134524, false, 10.335587856687802),
    level(0.42264973081037416, true, 5.0),
    level(0.5917517095361371, true, 5.0),
    level(1.0, true, 5.0),
];

const LADDER_1024: [Level; 8] = [
    level(0.0, false, 0.0),
    level(0.07417990022744847, false, 15.298136975098014),
    level(0.15484574527148343, false, 13.174673955480015),
    level(0.2440710539815456, false, 11.861735027312122),
    level(0.3453463292920228, false, 10.860249661666892),
    level(0.4654775161751512, true, 6.0),
    level(0.6220355269907727, true, 6.0),
    level(1.0, true, 6.0),
];

fn check_ladder(size: u32, ladder: &[Level]) {
    let max_lod = max_lod_for(size);
    // `maxLod = log2( size ) - LOD_MIN`, and the target has `maxLod + 1` levels.
    assert_eq!(max_lod as usize + 1, ladder.len(), "maxLod for {size}");
    for (lod, want) in ladder.iter().enumerate() {
        let lod = lod as u32;
        let roughness = lod_to_roughness(lod, max_lod);
        // Bit-exact: the same sequence of f64 operations on the same inputs.
        assert_eq!(roughness, want.roughness, "roughness, {size} lod {lod}");
        assert_eq!(
            uses_integration(lod, max_lod),
            want.integration,
            "material, {size} lod {lod}"
        );
        let got = if want.integration {
            (size as f64 / INTEGRATION_SIZE as f64).log2()
        } else {
            lod_bias(size, roughness)
        };
        assert_eq!(got, want.bias_or_source_lod, "uniform, {size} lod {lod}");
    }
}

#[test]
fn level_ladder_matches_three() {
    check_ladder(256, &LADDER_256);
    check_ladder(512, &LADDER_512);
    check_ladder(1024, &LADDER_1024);
}

/// `roughnessToMip( r, 5 )` — `maxLod * r * ( 2 - r )` on a clamped `r`.
/// It inverts `lodToRoughness`, so each level's roughness reads back its own
/// level.
#[test]
fn roughness_to_mip_matches_three() {
    for &(roughness, mip) in &[
        (-1.0, 0.0),
        (0.0, 0.0),
        (0.045, 0.43987499999999996),
        (0.25, 2.1875),
        (0.5, 3.75),
        (0.75, 4.6875),
        (1.0, 5.0),
        (2.0, 5.0),
    ] {
        assert_eq!(roughness_to_mip_value(roughness, 5.0), mip, "roughnessToMip( {roughness} )");
    }
    for lod in 0..=5 {
        let back = roughness_to_mip_value(lod_to_roughness(lod, 5), 5.0);
        assert!((back - lod as f64).abs() < 1e-12, "lod {lod} reads back {back}");
    }
}
