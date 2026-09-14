//! Ladder step 3 — layout, graded against `layout_vector.json` and
//! `layout_canvas_fallback.json`.
//!
//! Both goldens are compared **exactly**: `glyph_bounds` bit-for-bit as `f32`
//! (`to_bits`, so a `-0.0` would be caught), and `block_bounds`, `line_height`,
//! `ascender` and `descender` bit-for-bit as `f64`. There is no epsilon in this
//! file and there should not be: every arithmetic step is the same sequence of
//! IEEE-754 operations on the same inputs, the font metrics feeding it are
//! already proved exact by `tests/vector_font.rs`, and an epsilon here would
//! hide precisely the quirks (the `f32`-only truncation, the letter-spacing
//! asymmetry, the anchor fall-through) that the port exists to preserve.

mod common;

use common::{golden, params_from_json, roboto};
use sdf_text::text_builder::{
    empty_render_info, layout_text, layout_text_vector, Anchor, LayoutParams, LineHeight,
    TextAlign, TextRenderInfo,
};

/// Compares one `TextRenderInfo` against one golden case.
fn check_case(case: &serde_json::Value, got: &TextRenderInfo, label: &str) {
    assert_eq!(
        got.glyph_count,
        case["glyphCount"].as_u64().unwrap() as usize,
        "{label}: glyph count"
    );

    let want_chars: Vec<char> = case["chars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let s = v.as_str().unwrap();
            assert_eq!(s.chars().count(), 1);
            s.chars().next().unwrap()
        })
        .collect();
    let got_chars: Vec<char> = got.glyphs.iter().map(|g| g.ch).collect();
    assert_eq!(got_chars, want_chars, "{label}: characters");

    let want_bounds: Vec<f32> = case["glyphBounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(
        got.glyph_bounds.len(),
        want_bounds.len(),
        "{label}: glyphBounds length"
    );
    for i in 0..want_bounds.len() {
        let (g, w) = (got.glyph_bounds[i], want_bounds[i]);
        assert_eq!(
            g.to_bits(),
            w.to_bits(),
            "{label}: glyphBounds[{i}] (glyph {}, {}): got {g} want {w}",
            i / 4,
            ["minX", "minY", "maxX", "maxY"][i % 4]
        );
    }

    let want_block: Vec<f64> = case["blockBounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    for i in 0..4 {
        assert_eq!(
            got.block_bounds[i].to_bits(),
            want_block[i].to_bits(),
            "{label}: blockBounds[{i}]: got {} want {}",
            got.block_bounds[i],
            want_block[i]
        );
        // The JS assigns the same four numbers to both.
        assert_eq!(got.visible_bounds[i], got.block_bounds[i]);
    }

    for (name, g, w) in [
        (
            "lineHeight",
            got.line_height,
            case["lineHeight"].as_f64().unwrap(),
        ),
        ("ascender", got.ascender, case["ascender"].as_f64().unwrap()),
        (
            "descender",
            got.descender,
            case["descender"].as_f64().unwrap(),
        ),
    ] {
        assert_eq!(
            g.to_bits(),
            w.to_bits(),
            "{label}: {name}: got {g} want {w}"
        );
    }
}

#[test]
fn layout_text_vector_matches_the_golden() {
    let font = roboto();
    let g = golden("layout_vector.json");
    let cases = g.as_array().unwrap();
    assert_eq!(cases.len(), 6, "the golden has 6 cases");

    for (i, case) in cases.iter().enumerate() {
        let params = params_from_json(&case["params"]);
        let label = format!("vector case {i} ({:?})", params.text);
        let got = layout_text_vector(params, Some(&font));
        check_case(case, &got, &label);
    }
}

#[test]
fn layout_text_canvas_fallback_matches_the_golden() {
    let g = golden("layout_canvas_fallback.json");
    let cases = g.as_array().unwrap();
    assert_eq!(cases.len(), 6, "the golden has 6 cases");

    for (i, case) in cases.iter().enumerate() {
        let params = params_from_json(&case["params"]);
        let label = format!("canvas-fallback case {i} ({:?})", params.text);
        let got = layout_text(params);
        check_case(case, &got, &label);
    }
}

// ── The quirks, pinned individually ────────────────────────────────────────
//
// The golden cases above would catch a regression in any of these, but not
// legibly: a single wrong `glyphBounds[37]` does not say which rule broke. These
// name the rules.

#[test]
fn unknown_anchor_keywords_fall_through_to_zero() {
    let font = roboto();
    let base = LayoutParams {
        text: "start-anchored".to_string(),
        ..Default::default()
    };

    let left = layout_text_vector(
        LayoutParams {
            anchor_x: Anchor::named("left"),
            ..base.clone()
        },
        Some(&font),
    );
    // 'start' and 'end' are what d33 passes, and neither is a branch of
    // resolveAnchor — so both are left-anchored.
    for keyword in ["start", "end", "middle", "", "LEFT"] {
        let got = layout_text_vector(
            LayoutParams {
                anchor_x: Anchor::named(keyword),
                ..base.clone()
            },
            Some(&font),
        );
        assert_eq!(
            got.block_bounds, left.block_bounds,
            "anchorX {keyword:?} should behave as 'left'"
        );
    }
    // And a numeric anchor is a plain negated offset, not a keyword lookup.
    let offset = layout_text_vector(
        LayoutParams {
            anchor_x: Anchor::Offset(0.25),
            ..base.clone()
        },
        Some(&font),
    );
    assert_eq!(offset.block_bounds[0], left.block_bounds[0] - 0.25);

    // anchorY has its own keyword set: 'left' means nothing on that axis.
    let y_nonsense = layout_text_vector(
        LayoutParams {
            anchor_y: Anchor::named("left"),
            ..base.clone()
        },
        Some(&font),
    );
    let y_bottom = layout_text_vector(
        LayoutParams {
            anchor_y: Anchor::named("bottom"),
            ..base.clone()
        },
        Some(&font),
    );
    assert_eq!(y_nonsense.block_bounds, y_bottom.block_bounds);
}

#[test]
fn letter_spacing_is_measured_between_but_advanced_after() {
    let font = roboto();
    let spacing = 0.1;
    let params = LayoutParams {
        text: "AB".to_string(),
        letter_spacing: spacing,
        max_width: 100.0,
        text_align: TextAlign::Right,
        ..Default::default()
    };
    let got = layout_text_vector(params.clone(), Some(&font));

    // The measure pass counts one gap (between A and B); the pen loop adds one
    // after A and one after B. Right-aligning from a 100-wide box therefore
    // leaves the block's right edge one whole letter_spacing short of 100 …
    let right_edge_of_ink = got.glyph_bounds[6] as f64; // B's maxX
    assert!(right_edge_of_ink < 100.0);

    // … and the asymmetry is exactly one spacing: with spacing 0 the measured
    // width and the pen travel agree, so the right edge lands where the ink
    // ends. Compare the align offsets directly.
    let zero = layout_text_vector(
        LayoutParams {
            letter_spacing: 0.0,
            ..params.clone()
        },
        Some(&font),
    );
    // Unanchored pen positions differ by the spacing on A's advance only, so the
    // measured line width differs by exactly `spacing`. Read that back from the
    // pre-anchor glyph boxes, which carry the align offset.
    let with = got.glyphs[0].bounds[0];
    let without = zero.glyphs[0].bounds[0];
    assert!(
        ((with - without) - (-spacing)).abs() < 1e-12,
        "the centred/right-aligned offset should move by exactly one spacing \
         (got {}, want {})",
        with - without,
        -spacing
    );
}

#[test]
fn lines_march_downward_in_y() {
    let font = roboto();
    let got = layout_text_vector(
        LayoutParams {
            text: "one\ntwo".to_string(),
            anchor_y: Anchor::named("bottom"),
            ..Default::default()
        },
        Some(&font),
    );
    // With anchorY 'bottom' the offset is just -blockMinY, so the *second* line
    // sits at the bottom: the first line's glyphs are the higher ones.
    let first_line_min_y = got.glyph_bounds[1];
    let last_line_min_y = got.glyph_bounds[(got.glyph_count - 1) * 4 + 1];
    assert!(
        first_line_min_y > last_line_min_y,
        "line 1 ({first_line_min_y}) should be above line 2 ({last_line_min_y})"
    );
    // And the gap is the line advance.
    let advance = got.line_height;
    assert!(((first_line_min_y - last_line_min_y) as f64 - advance).abs() < 1e-6);
}

#[test]
fn kerning_moves_the_current_glyph_not_the_previous_advance() {
    let font = roboto();
    // 'AV' kerns; the pair is non-zero in Roboto's GPOS.
    let kern = font.kerning('A', 'V');
    assert!(kern < 0.0, "expected a negative A/V kern, got {kern}");

    let got = layout_text_vector(
        LayoutParams {
            text: "AV".to_string(),
            ..Default::default()
        },
        Some(&font),
    );
    let scale = 1.0 / font.units_per_em as f64;
    let a_adv = font.advance_width('A') * scale;
    let v_box = font.bounding_box('V').unwrap();

    // V's minX = A's advance + kern + V's bbox.x1. If the kern were folded into
    // A's advance instead, A's own box would move; it must not.
    let a_box = font.bounding_box('A').unwrap();
    assert_eq!(got.glyphs[0].bounds[0], a_box.x1 * scale, "A must not move");
    let want_v_min_x = a_adv + kern * scale + v_box.x1 * scale;
    assert!((got.glyphs[1].bounds[0] - want_v_min_x).abs() < 1e-15);
}

#[test]
fn f32_truncation_happens_only_in_glyph_bounds() {
    // The golden's first canvas case carries 1.7999999999999998 in blockBounds
    // and 1.7999999523162842 — the f32 of the same number — in glyphBounds.
    let got = layout_text(LayoutParams {
        text: "Hi!".to_string(),
        ..Default::default()
    });
    assert_eq!(got.block_bounds[2], 1.7999999999999998);
    assert_eq!(got.glyph_bounds[10], 1.7999999999999998f64 as f32);
    assert_ne!(got.block_bounds[2], got.glyph_bounds[10] as f64);
}

#[test]
fn centring_the_fallback_layout_produces_nan_x() {
    // line_width is forced to 0 and max_width defaults to Infinity, so the align
    // offset is +Infinity, every pen position is +Infinity, blockWidth is
    // Infinity - Infinity = NaN, and offsetX is -Infinity — so every x comes out
    // NaN while the y column stays finite. Confirmed against the JS, which
    // serialises the same layout as [null, 0, null, 1, …].
    let got = layout_text(LayoutParams {
        text: "ABC".to_string(),
        text_align: TextAlign::Center,
        ..Default::default()
    });
    for (i, v) in got.glyph_bounds.iter().enumerate() {
        if i % 2 == 0 {
            assert!(v.is_nan(), "glyphBounds[{i}] should be NaN, got {v}");
        } else {
            assert!(v.is_finite(), "glyphBounds[{i}] should be finite, got {v}");
        }
    }
    assert!(got.block_bounds[0].is_nan() && got.block_bounds[2].is_nan());
    assert_eq!([got.block_bounds[1], got.block_bounds[3]], [0.0, 1.0]);
}

#[test]
fn the_fallback_measure_is_64x_smaller_than_the_fallback_advance() {
    // A real quirk, not a transcription slip: `measureRunWidth(null, …)` returns
    // `str.length * 0.6 * scale` — it drops the `MEASURE_FONT_PX` factor that the
    // per-character `charWidth` keeps (`MEASURE_FONT_PX * 0.6 * scale`). So the
    // width used for line breaking is 1/64 of the width the pen actually
    // travels, and `max_width` is effectively 64x looser in this branch. At
    // font_size 1 an 11-character string measures 0.103 while occupying 6.6, so
    // a max_width of 2 does not wrap it. Verified against the JS, which also
    // returns one line of 11 glyphs here.
    let got = layout_text(LayoutParams {
        text: "aaa bbb ccc".to_string(),
        max_width: 2.0,
        ..Default::default()
    });
    assert_eq!(got.glyph_count, 11);
    assert_eq!(got.block_bounds, [0.0, 0.0, 6.599999999999999, 1.0]);
}

#[test]
fn space_takes_the_no_ink_branch_in_the_vector_path() {
    let font = roboto();
    let got = layout_text_vector(
        LayoutParams {
            text: " ".to_string(),
            ..Default::default()
        },
        Some(&font),
    );
    let scale = 1.0 / font.units_per_em as f64;
    assert_eq!(got.glyph_count, 1, "a space still gets a glyph entry");
    // Box is advance-wide and descender..ascender tall, not the (absent) ink box.
    let b = got.glyphs[0].bounds;
    assert_eq!(b[2] - b[0], font.advance_width(' ') * scale);
    assert_eq!(b[3] - b[1], (font.ascender - font.descender) * scale);
}

#[test]
fn no_font_gives_an_empty_layout() {
    let params = LayoutParams {
        text: "ignored".to_string(),
        ..Default::default()
    };
    let got = layout_text_vector(params.clone(), None);
    assert_eq!(got, empty_render_info(params));
    assert_eq!(got.glyph_count, 0);
    assert_eq!(got.block_bounds, [0.0; 4]);
    assert_eq!(got.line_height, 0.0);
}

#[test]
fn empty_text_zeroes_the_block_instead_of_leaving_infinities() {
    let font = roboto();
    let got = layout_text_vector(LayoutParams::default(), Some(&font));
    assert_eq!(got.glyph_count, 0);
    assert_eq!(got.block_bounds, [0.0; 4]);
    // But lineHeight/ascender/descender are still the font's, unlike
    // empty_render_info's zeros.
    assert!(got.line_height > 0.0);
}

#[test]
fn line_height_parses_like_parse_float() {
    assert_eq!(LineHeight::parse("normal"), LineHeight::Normal);
    assert_eq!(LineHeight::parse("1.5"), LineHeight::Factor(1.5));
    assert_eq!(LineHeight::parse("1.5em"), LineHeight::Factor(1.5));
    assert_eq!(LineHeight::parse("  2 "), LineHeight::Factor(2.0));
    assert_eq!(LineHeight::parse("-0.5"), LineHeight::Factor(-0.5));
    assert_eq!(LineHeight::parse("1e2"), LineHeight::Factor(100.0));
    assert_eq!(LineHeight::parse("1e"), LineHeight::Factor(1.0));
    assert_eq!(LineHeight::parse("abc"), LineHeight::Normal);
    assert_eq!(LineHeight::parse(""), LineHeight::Normal);
    assert_eq!(LineHeight::parse("Infinity"), LineHeight::Normal);

    // The vector path's "normal" is the font's own natural height, not 1.2 em.
    let font = roboto();
    let natural = layout_text_vector(
        LayoutParams {
            text: "x".to_string(),
            ..Default::default()
        },
        Some(&font),
    );
    assert_ne!(natural.line_height, 1.2);
    let fixed = layout_text_vector(
        LayoutParams {
            text: "x".to_string(),
            line_height: LineHeight::Factor(0.9),
            font_size: 0.37,
            ..Default::default()
        },
        Some(&font),
    );
    assert_eq!(fixed.line_height, 0.9 * 0.37);
}

#[test]
fn line_breaking_only_runs_for_a_finite_positive_max_width() {
    let font = roboto();
    let mk = |max_width: f64| {
        layout_text_vector(
            LayoutParams {
                text: "aaa bbb ccc".to_string(),
                max_width,
                ..Default::default()
            },
            Some(&font),
        )
    };
    let unbroken = mk(f64::INFINITY);
    assert_eq!(mk(0.0).block_bounds, unbroken.block_bounds);
    assert_eq!(mk(-1.0).block_bounds, unbroken.block_bounds);
    // A tight width does break, and the spaces are kept on the line they end.
    // (block_bounds[1] is always 0 here — the default anchor is a zero offset,
    // which subtracts blockMinY — so the block *height* is what shows it.)
    let broken = mk(2.0);
    let height = |i: &TextRenderInfo| i.block_bounds[3] - i.block_bounds[1];
    assert!(
        height(&broken) > height(&unbroken) + unbroken.line_height,
        "should have wrapped to 3 lines: broken {} vs unbroken {}",
        height(&broken),
        height(&unbroken)
    );
    assert_eq!(
        broken.glyph_count, unbroken.glyph_count,
        "wrapping keeps every character, spaces included"
    );
}
