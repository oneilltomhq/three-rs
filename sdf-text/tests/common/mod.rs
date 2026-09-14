#![allow(dead_code)] // each test binary uses a different subset

//! Shared helpers for the golden-data tests.

use std::path::PathBuf;

pub fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

pub fn golden(name: &str) -> serde_json::Value {
    let path = golden_dir().join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

pub fn roboto() -> sdf_text::VectorFont {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/Roboto-Regular.ttf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    sdf_text::VectorFont::parse(bytes, "tests/assets/Roboto-Regular.ttf").unwrap()
}

/// Minimal base64 decoder — the goldens carry raw little-endian buffers as
/// base64 and pulling a crate in for eight lines is not worth it.
pub fn b64(s: &str) -> Vec<u8> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rev = [0xFFu8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        rev[c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let v = rev[c as usize];
        assert_ne!(v, 0xFF, "bad base64 byte {c}");
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

pub fn as_f32_vec(bytes: &[u8]) -> Vec<f32> {
    assert_eq!(bytes.len() % 4, 0);
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

// ── The lib3 test's own shader-math helpers ────────────────────────────────
//
// These live in `test/sdf-pipeline.test.mjs`, not in the library, so they stay
// test code here too. Note the `edgeWidth` is a hardcoded 0.1, unrelated to the
// material's real `edgeWidth` — layers 4 and 6 are checking the *shape* of the
// smoothstep, not the shipped constant.

/// GLSL/WGSL `smoothstep`.
pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `shaderAlpha(sdfValue)` with the test's fixed `edge = 0.5`, `edgeWidth = 0.1`.
pub fn shader_alpha_fixed_width(sdf_value: f64) -> f64 {
    let edge = 0.5;
    let edge_width = 0.1;
    smoothstep(edge - edge_width, edge + edge_width, sdf_value)
}

/// Deserialises one golden layout case's `params` object into a
/// [`sdf_text::LayoutParams`]. Absent keys take the `LAYOUT_DEFAULTS` value,
/// exactly as the JS destructuring does.
pub fn params_from_json(v: &serde_json::Value) -> sdf_text::LayoutParams {
    use sdf_text::{Anchor, LayoutParams, LineHeight, TextAlign};
    let mut p = LayoutParams::default();
    let o = v.as_object().expect("params must be an object");
    for (key, val) in o {
        match key.as_str() {
            "text" => p.text = val.as_str().unwrap().to_string(),
            "fontSize" => p.font_size = val.as_f64().unwrap(),
            "fontFamily" => p.font_family = val.as_str().unwrap().to_string(),
            "fontWeight" => p.font_weight = val.as_str().unwrap().to_string(),
            "fontStyle" => p.font_style = val.as_str().unwrap().to_string(),
            "letterSpacing" => p.letter_spacing = val.as_f64().unwrap(),
            "lineHeight" => {
                p.line_height = match val {
                    serde_json::Value::Number(n) => LineHeight::Factor(n.as_f64().unwrap()),
                    serde_json::Value::String(s) => LineHeight::parse(s),
                    _ => LineHeight::Normal,
                }
            }
            "anchorX" | "anchorY" => {
                let a = match val {
                    serde_json::Value::Number(n) => Anchor::Offset(n.as_f64().unwrap()),
                    serde_json::Value::String(s) => Anchor::named(s),
                    other => panic!("anchor must be a number or a string, got {other}"),
                };
                if key == "anchorX" {
                    p.anchor_x = a;
                } else {
                    p.anchor_y = a;
                }
            }
            "textAlign" => {
                p.text_align = match val.as_str().unwrap() {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    _ => TextAlign::Left,
                }
            }
            "maxWidth" => p.max_width = val.as_f64().unwrap_or(f64::INFINITY),
            other => panic!("unknown layout parameter {other:?} in the golden"),
        }
    }
    p
}
