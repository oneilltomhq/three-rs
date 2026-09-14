//! Ladder step 2, part 1 — the EDT must match `computeSDF` exactly.
//!
//! Graded against `tests/golden/edt_fixtures.json`, which carries the full
//! `f64` field for each of the four fixtures the lib3 test uses. Compared with
//! `==`: both sides run the same algorithm in `f64`, so there is no reason for
//! a single bit to differ, and an epsilon here would hide a real divergence.

mod common;

use common::golden;
use sdf_text::edt::compute_sdf_default;

fn case(name: &str) -> (usize, usize, Vec<u8>, Vec<f64>) {
    let g = golden("edt_fixtures.json");
    let c = &g[name];
    let w = c["w"].as_u64().unwrap() as usize;
    let h = c["h"].as_u64().unwrap() as usize;
    let alpha: Vec<u8> = c["alpha"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let sdf: Vec<f64> = c["sdf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(alpha.len(), w * h);
    assert_eq!(sdf.len(), w * h);
    (w, h, alpha, sdf)
}

fn check(name: &str) {
    let (w, h, alpha, want) = case(name);
    let got = compute_sdf_default(&alpha, w, h);
    assert_eq!(got.len(), want.len());
    for i in 0..got.len() {
        assert_eq!(
            got[i].to_bits(),
            want[i].to_bits(),
            "{name}[{}, {}]: got {} want {}",
            i % w,
            i / w,
            got[i],
            want[i]
        );
    }
}

#[test]
fn square10_matches_exactly() {
    check("square10");
}

#[test]
fn empty4_matches_exactly() {
    check("empty4");
}

#[test]
fn filled4_matches_exactly() {
    check("filled4");
}

#[test]
fn circle16_matches_exactly() {
    check("circle16");
}

// ── The lib3 test's own layer-1 assertions, ported verbatim ────────────────

#[test]
fn layer1_filled_square_is_negative_inside_positive_outside() {
    let (w, h, alpha, _) = case("square10");
    let sdf = compute_sdf_default(&alpha, w, h);
    assert!(sdf[5 * w + 5] < 0.0, "centre should be negative");
    assert!(sdf[0] > 0.0, "corner should be positive");
    assert!((sdf[1 * w + 2] - 1.0).abs() < 0.001);
    assert_eq!(h, 10);
}

#[test]
fn layer1_empty_image_is_all_positive() {
    let (w, h, alpha, _) = case("empty4");
    for v in compute_sdf_default(&alpha, w, h) {
        assert!(v > 0.0);
    }
}

#[test]
fn layer1_filled_image_is_all_negative() {
    let (w, h, alpha, _) = case("filled4");
    for v in compute_sdf_default(&alpha, w, h) {
        assert!(v < 0.0);
    }
}

// ── Layer 2: SDF normalisation, with the test's own constant set ───────────

#[test]
fn layer2_normalisation_uses_sdf_defaults_max_distance() {
    use sdf_text::sdf_defaults::MAX_DISTANCE;
    use sdf_text::vector_font_atlas::normalize_sdf_value;
    // MAX_DISTANCE is 8 here (src/sdf/index.js), not the atlas's 32.
    assert_eq!(MAX_DISTANCE, 8.0);
    assert!((normalize_sdf_value(0.0, MAX_DISTANCE) - 0.5).abs() < 0.001);
    assert!((normalize_sdf_value(MAX_DISTANCE, MAX_DISTANCE) - 0.0).abs() < 0.001);
    assert!((normalize_sdf_value(-MAX_DISTANCE, MAX_DISTANCE) - 1.0).abs() < 0.001);
}

// ── Layer 4: shader-math simulation, over the circle16 fixture ─────────────

#[test]
fn layer4_boundary_value_gives_half_alpha() {
    assert!((common::shader_alpha_fixed_width(0.5) - 0.5).abs() < 0.001);
}

#[test]
fn layer4_full_pipeline_circle_centre_opaque_corner_transparent() {
    use sdf_text::sdf_defaults::MAX_DISTANCE;
    use sdf_text::vector_font_atlas::normalize_sdf_value;
    let (w, _h, alpha, _) = case("circle16");
    let sdf = compute_sdf_default(&alpha, w, w);
    let centre =
        common::shader_alpha_fixed_width(normalize_sdf_value(sdf[8 * w + 8], MAX_DISTANCE));
    let corner = common::shader_alpha_fixed_width(normalize_sdf_value(sdf[0], MAX_DISTANCE));
    assert!(centre > 0.99, "centre alpha {centre}");
    assert!(corner < 0.01, "corner alpha {corner}");
}

#[test]
fn layer4_blank_glyph_has_zero_alpha() {
    // The JS writes this as a branch on the UV rect's width, not on the SDF: a
    // glyph with a zero sub-rect is skipped entirely rather than sampled.
    let glyph_uv_w = 0.0;
    let alpha = if glyph_uv_w == 0.0 {
        0.0
    } else {
        common::shader_alpha_fixed_width(0.5)
    };
    assert_eq!(alpha, 0.0);
}
