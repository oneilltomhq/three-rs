//! Numeric oracle for `CatmullRomCurve3` and `Color.setHSL`, against values
//! dumped from three.js' own `webgpu_lines_fat` example.
//!
//! `scouts/webgpu_lines_fat/spline_oracle.json` holds, as `f32`, the 64
//! `hilbert3D` control points, the 768 spline samples three.js computes from
//! them (`spline.getPoint( i / divisions )`) and the 768 HSL colours the
//! example assigns along the line. The port has to reproduce both exactly once
//! narrowed to `f32`: the arithmetic is the same `f64` arithmetic in the same
//! order, so anything looser would mean a real divergence.
//!
//! The oracle lives in a sibling worktree, so a checkout without it skips.

use std::path::PathBuf;

use serde_json::Value;
use three_rs::extras::{CatmullRomCurve3, Curve};
use three_rs::math::{Color, ColorSpace, Vector3};

/// The oracle path, relative to this crate, or `None` when it is not checked out.
fn oracle_path() -> Option<PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../scouts/scouts/webgpu_lines_fat/spline_oracle.json");
    path.exists().then_some(path)
}

fn f32s(value: &Value, key: &str) -> Vec<f32> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("three-rs: spline_oracle.json: `{key}` is not an array"))
        .iter()
        .map(|v| {
            v.as_f64().unwrap_or_else(|| {
                panic!("three-rs: spline_oracle.json: `{key}` holds a non-number")
            }) as f32
        })
        .collect()
}

/// `(value as f32)` must be the very same bit pattern the oracle holds.
#[track_caller]
fn bits_eq(actual: f64, expected: f32, what: &str) {
    let narrowed = actual as f32;
    assert!(
        narrowed.to_bits() == expected.to_bits(),
        "{what}: {narrowed} (bits {:#010x}) != {expected} (bits {:#010x}); f64 was {actual}",
        narrowed.to_bits(),
        expected.to_bits()
    );
}

#[test]
fn spline_oracle_positions_and_colors() {
    let Some(path) = oracle_path() else {
        eprintln!(
            "skipping: ../scouts/scouts/webgpu_lines_fat/spline_oracle.json is not present \
             (the `scouts` worktree is not checked out)"
        );
        return;
    };

    let json: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    let hilbert_point_count = json["hilbertPointCount"].as_u64().unwrap() as usize;
    let divisions = json["divisions"].as_u64().unwrap() as usize;

    let hilbert = f32s(&json, "hilbert");
    let positions = f32s(&json, "positions");
    let colors = f32s(&json, "colors");

    assert_eq!(hilbert.len(), hilbert_point_count * 3, "hilbert length");
    assert_eq!(positions.len(), divisions * 3, "positions length");
    assert_eq!(colors.len(), divisions * 3, "colors length");

    // `new THREE.CatmullRomCurve3( points )`: not closed, centripetal, tension 0.5.
    let spline = CatmullRomCurve3::new(
        hilbert
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| Vector3::new(p[0] as f64, p[1] as f64, p[2] as f64))
            .collect(),
    );

    let mut color = Color::default();

    for i in 0..divisions {
        let t = i as f64 / divisions as f64;

        let point = spline.get_point(t);
        bits_eq(point.x, positions[i * 3], &format!("positions[{i}].x"));
        bits_eq(point.y, positions[i * 3 + 1], &format!("positions[{i}].y"));
        bits_eq(point.z, positions[i * 3 + 2], &format!("positions[{i}].z"));

        color.set_hsl(t, 1.0, 0.5, ColorSpace::SRGB);
        bits_eq(color.r, colors[i * 3], &format!("colors[{i}].r"));
        bits_eq(color.g, colors[i * 3 + 1], &format!("colors[{i}].g"));
        bits_eq(color.b, colors[i * 3 + 2], &format!("colors[{i}].b"));
    }
}
