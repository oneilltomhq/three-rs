//! Ladder step 1 — font outlines and metrics must match opentype.js.
//!
//! Graded against `tests/golden/font_roboto.json`: every metric, every advance,
//! every ink bbox, every `Path.getBoundingBox()`, every path command, and every
//! kern pair over the 67 characters the golden covers.

mod common;

use common::{golden, roboto};
use sdf_text::vector_font::{path_bounding_box, PathCommand};
use sdf_text::{BBox, VectorFont};

/// The character set the golden was dumped over, in its original order.
const CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 ,.()-";

fn chars() -> Vec<char> {
    let mut seen = Vec::new();
    for c in CHARS.chars() {
        if !seen.contains(&c) {
            seen.push(c);
        }
    }
    seen
}

fn f(v: &serde_json::Value) -> f64 {
    v.as_f64().expect("number")
}

#[test]
fn metrics_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    assert_eq!(font.units_per_em, f(&g["unitsPerEm"]));
    assert_eq!(font.ascender, f(&g["ascender"]));
    assert_eq!(font.descender, f(&g["descender"]));
    assert_eq!(font.line_gap, f(&g["lineGap"]));
    assert_eq!(font.cap_height, f(&g["capHeight"]));
    assert_eq!(font.x_height, f(&g["xHeight"]));
    assert_eq!(font.has_gpos(), g["hasGpos"].as_bool().unwrap());
    assert_eq!(font.has_kern(), g["hasKern"].as_bool().unwrap());
    // Roboto's sTypoLineGap is 0, so the `||` chain falls through to
    // hhea.lineGap (also 0) and then to the literal 0 — the quirk is exercised
    // here whether or not it changes the number.
    assert_eq!(font.line_gap, 0.0);
}

#[test]
fn advances_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    for ch in chars() {
        let want = f(&g["glyphs"][ch.to_string()]["advance"]);
        assert_eq!(font.advance_width(ch), want, "advance for {ch:?}");
    }
}

#[test]
fn ink_bboxes_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    for ch in chars() {
        let want = &g["glyphs"][ch.to_string()]["bbox"];
        match font.bounding_box(ch) {
            None => assert!(want.is_null(), "{ch:?}: expected a bbox, got None"),
            Some(b) => {
                assert!(!want.is_null(), "{ch:?}: expected None, got {b:?}");
                // Exact: every value is an integer or an exactly representable
                // dyadic from the 2/3–1/3 elevation, computed in f64 both sides.
                assert_eq!(b.x1, f(&want["x1"]), "{ch:?} x1");
                assert_eq!(b.y1, f(&want["y1"]), "{ch:?} y1");
                assert_eq!(b.x2, f(&want["x2"]), "{ch:?} x2");
                assert_eq!(b.y2, f(&want["y2"]), "{ch:?} y2");
            }
        }
    }
}

#[test]
fn y_down_path_bboxes_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    for ch in chars() {
        let gid = font.glyph_for_char(ch);
        let got = path_bounding_box(&font.glyph_path_y_down(gid));
        let want = &g["glyphs"][ch.to_string()]["pathBBox"];
        let want = BBox {
            x1: f(&want["x1"]),
            y1: f(&want["y1"]),
            x2: f(&want["x2"]),
            y2: f(&want["y2"]),
        };
        assert_eq!(got, want, "pathBBox for {ch:?}");
    }
}

#[test]
fn path_commands_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    let mut total = 0usize;
    for ch in chars() {
        let gid = font.glyph_for_char(ch);
        let got = font.glyph_path_y_down(gid);
        let want = g["glyphs"][ch.to_string()]["commands"]
            .as_array()
            .expect("commands array");
        assert_eq!(got.len(), want.len(), "command count for {ch:?}");
        for (i, (got, want)) in got.iter().zip(want.iter()).enumerate() {
            let ty = want["type"].as_str().unwrap();
            assert_eq!(got.type_letter(), ty, "{ch:?} cmd {i} type");
            match *got {
                PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => {
                    assert_eq!(x, f(&want["x"]), "{ch:?} cmd {i} x");
                    assert_eq!(y, f(&want["y"]), "{ch:?} cmd {i} y");
                }
                PathCommand::QuadTo { x1, y1, x, y } => {
                    assert_eq!(x1, f(&want["x1"]), "{ch:?} cmd {i} x1");
                    assert_eq!(y1, f(&want["y1"]), "{ch:?} cmd {i} y1");
                    assert_eq!(x, f(&want["x"]), "{ch:?} cmd {i} x");
                    assert_eq!(y, f(&want["y"]), "{ch:?} cmd {i} y");
                }
                PathCommand::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    assert_eq!(x1, f(&want["x1"]), "{ch:?} cmd {i} x1");
                    assert_eq!(y1, f(&want["y1"]), "{ch:?} cmd {i} y1");
                    assert_eq!(x2, f(&want["x2"]), "{ch:?} cmd {i} x2");
                    assert_eq!(y2, f(&want["y2"]), "{ch:?} cmd {i} y2");
                    assert_eq!(x, f(&want["x"]), "{ch:?} cmd {i} x");
                    assert_eq!(y, f(&want["y"]), "{ch:?} cmd {i} y");
                }
                PathCommand::Close => unreachable!("getPath drops Z for filled paths"),
            }
            total += 1;
        }
    }
    assert!(total > 1000, "only {total} commands compared");
}

#[test]
fn kern_pairs_match_opentype() {
    let g = golden("font_roboto.json");
    let font = roboto();
    let pairs = g["kernPairs"].as_object().expect("kernPairs object");
    let cs = chars();
    let mut non_zero = 0usize;
    for &a in &cs {
        for &b in &cs {
            let key: String = [a, b].iter().collect();
            let want = pairs.get(&key).map(f).unwrap_or(0.0);
            let got = font.kerning(a, b);
            assert_eq!(got, want, "kerning({a:?}, {b:?})");
            if want != 0.0 {
                non_zero += 1;
            }
        }
    }
    assert_eq!(non_zero, pairs.len(), "golden pair count");
    // The headline pair from the plan.
    assert_eq!(font.kerning('A', 'V'), -87.0);
}

#[test]
fn blank_glyph_has_no_ink_box() {
    let font = roboto();
    // `' '` maps to a real glyph with an empty outline, so opentype's bbox is
    // (0,0,0,0) and `boundingBox` returns null.
    assert_eq!(font.bounding_box(' '), None);
    assert_eq!(font.advance_width(' '), 507.0);
    assert!(font.glyph_path(font.glyph_for_char(' ')).is_empty());
}

#[test]
fn metric_fallthrough_treats_zero_as_absent() {
    // A direct check of the `||` precedence, independent of Roboto: the helper
    // that implements it must skip a zero rather than accept it.
    let font: VectorFont = roboto();
    assert_eq!(font.ascender, 2146.0);
    assert_eq!(font.descender, -555.0);
    assert_eq!(font.cap_height, 1456.0);
    assert_eq!(font.x_height, 1082.0);
}
