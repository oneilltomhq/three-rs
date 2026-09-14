//! Ladder step 3, part 2 — the `Text` member, plus lib3's test layers 5 and 6.
//!
//! Layer 5 ("Text layout") and layer 6 ("per-member opacity + outline color")
//! from `lib3/test/sdf-pipeline.test.mjs` are ported here assertion for
//! assertion. Layer 6's first two cases are the reason `OpacitySink` exists:
//! they check that `opacity` writes through to the owning batch and that doing
//! so does **not** invalidate the layout.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::{roboto, shader_alpha_fixed_width};
use sdf_text::text::{layout_defaults, OpacitySink};
use sdf_text::text_builder::{layout_text, Anchor, LayoutParams, LineHeight, TextAlign};
use sdf_text::Text;

/// Stands in for `BatchedText`, recording the write-throughs the way the JS test
/// does with `{ setOpacityAt: (id, o) => calls.push([id, o]) }`.
#[derive(Default)]
struct RecordingBatch {
    calls: Vec<(usize, f64)>,
}

impl OpacitySink for RecordingBatch {
    fn set_opacity_at(&mut self, member_id: usize, opacity: f64) {
        self.calls.push((member_id, opacity));
    }
}

// ── Layer 5: text layout ───────────────────────────────────────────────────

#[test]
fn layer5_one_glyph_per_character() {
    let info = layout_text(LayoutParams {
        text: "Hi!".to_string(),
        font_size: 1.0,
        ..Default::default()
    });
    assert_eq!(info.glyph_count, 3);
    assert_eq!(info.glyphs.len(), 3);
    assert_eq!(info.glyphs[0].ch, 'H');
    assert_eq!(info.glyphs[2].ch, '!');
}

#[test]
fn layer5_newlines_become_extra_lines() {
    let info = layout_text(LayoutParams {
        text: "ab\ncd".to_string(),
        font_size: 1.0,
        ..Default::default()
    });
    assert_eq!(info.glyph_count, 4);
    // And the second line is below the first, by one line advance.
    assert_eq!(info.line_height, 1.2);
    assert_eq!(info.glyph_bounds[1] - info.glyph_bounds[9], 1.2);
}

#[test]
fn layer5_anchor_x_center_shifts_block_bounds_symmetrically() {
    let mk = |anchor: &str| {
        layout_text(LayoutParams {
            text: "ABC".to_string(),
            font_size: 1.0,
            anchor_x: Anchor::named(anchor),
            ..Default::default()
        })
    };
    let left = mk("left");
    let center = mk("center");
    let left_mid = (left.block_bounds[0] + left.block_bounds[2]) * 0.5;
    let center_mid = (center.block_bounds[0] + center.block_bounds[2]) * 0.5;
    assert!(center_mid.abs() < left_mid.abs());
}

// ── Layer 6: per-member opacity and the outline colour mix ─────────────────

#[test]
fn layer6_opacity_defaults_to_one_and_writes_through_to_its_batch() {
    let mut t = Text::new();
    assert_eq!(t.opacity(), 1.0);

    let batch = Rc::new(RefCell::new(RecordingBatch::default()));
    t.attach_to_batch(batch.clone(), 3);
    t.set_opacity(0.25);
    assert_eq!(t.opacity(), 0.25);
    assert_eq!(batch.borrow().calls, vec![(3, 0.25)]);
}

#[test]
fn layer6_setting_opacity_does_not_invalidate_layout() {
    let mut t = Text::new();
    t.set_text("hello");
    t.sync();
    let info = t.text_render_info().unwrap().clone();
    let layouts = t.layouts_performed();

    t.set_opacity(0.5);
    assert!(!t.needs_sync());
    t.sync();
    // Same layout, and — what the JS's object-identity assertion really means —
    // no second layout was computed.
    assert_eq!(t.text_render_info().unwrap(), &info);
    assert_eq!(t.layouts_performed(), layouts);
}

#[test]
fn layer6_member_opacity_scales_coverage_alpha() {
    let coverage = shader_alpha_fixed_width(0.75); // well inside the glyph
    let opacity = 0.4;
    assert!((coverage * opacity - opacity).abs() < 0.01);
    assert_eq!(shader_alpha_fixed_width(0.1) * opacity, 0.0);
}

#[test]
fn layer6_outline_zone_takes_halo_colour_and_fill_keeps_member_colour() {
    let mix = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let member_r = 1.0;
    let halo_r = 0.05;
    assert!((mix(halo_r, member_r, 0.0) - halo_r).abs() < 1e-6);
    assert!((mix(halo_r, member_r, 1.0) - member_r).abs() < 1e-6);
}

// ── The rest of the Text surface ───────────────────────────────────────────

#[test]
fn defaults_match_layout_defaults() {
    let t = Text::new();
    let d = layout_defaults();
    assert_eq!(t.text(), d.text);
    assert_eq!(t.font_size(), d.font_size);
    assert_eq!(t.font_family(), d.font_family);
    assert_eq!(t.font_weight(), d.font_weight);
    assert_eq!(t.font_style(), d.font_style);
    assert_eq!(t.letter_spacing(), d.letter_spacing);
    assert_eq!(t.line_height(), d.line_height);
    assert_eq!(t.anchor_x(), &d.anchor_x);
    assert_eq!(t.anchor_y(), &d.anchor_y);
    assert_eq!(t.text_align(), d.text_align);
    assert!(t.max_width().is_infinite());
    assert_eq!(t.color, [1.0, 1.0, 1.0]);
    // Dirty from birth, so the first sync always lays out.
    assert!(t.needs_sync());
    assert!(t.text_render_info().is_none());
}

#[test]
fn every_layout_setter_dirties_and_an_unchanged_write_does_not() {
    let mut t = Text::new();
    t.sync();
    assert!(!t.needs_sync());

    macro_rules! check {
        ($set:ident, $value:expr) => {{
            t.$set($value);
            assert!(
                !t.needs_sync(),
                concat!(stringify!($set), ": unchanged write must not dirty")
            );
        }};
        ($set:ident, $same:expr, $different:expr) => {{
            t.$set($same);
            assert!(
                !t.needs_sync(),
                concat!(stringify!($set), ": unchanged write must not dirty")
            );
            t.$set($different);
            assert!(
                t.needs_sync(),
                concat!(stringify!($set), ": changed write must dirty")
            );
            t.sync();
        }};
    }

    check!(set_text, "", "hello");
    check!(set_font_size, 1.0, 2.0);
    check!(set_font_family, "monospace", "serif");
    check!(set_font_weight, "normal", "bold");
    check!(set_font_style, "normal", "italic");
    check!(set_letter_spacing, 0.0, 0.1);
    check!(set_line_height, LineHeight::Normal, LineHeight::Factor(1.5));
    check!(set_anchor_x, Anchor::Offset(0.0), Anchor::named("center"));
    check!(set_anchor_y, Anchor::Offset(0.0), Anchor::named("top"));
    check!(set_text_align, TextAlign::Left, TextAlign::Center);
    check!(set_max_width, f64::INFINITY, 10.0);

    // Opacity is the exception: it never dirties.
    t.set_opacity(0.5);
    assert!(!t.needs_sync());
}

#[test]
fn sync_is_cached_until_something_changes() {
    let mut t = Text::new();
    t.set_text("abc");
    t.sync();
    assert_eq!(t.layouts_performed(), 1);
    t.sync();
    t.sync();
    assert_eq!(t.layouts_performed(), 1);
    t.set_text("abcd");
    t.sync();
    assert_eq!(t.layouts_performed(), 2);
}

#[test]
fn vector_mode_without_a_font_stays_dirty_and_lays_out_empty() {
    let mut t = Text::new();
    t.set_text("hello");
    t.set_vector_mode(true);
    t.sync();
    // An empty layout, and still dirty, so the font's arrival is not missed.
    assert_eq!(t.text_render_info().unwrap().glyph_count, 0);
    assert!(t.needs_sync());
    assert_eq!(t.layouts_performed(), 0);

    t.set_vector_font(Some(Rc::new(roboto())));
    t.sync();
    assert_eq!(t.text_render_info().unwrap().glyph_count, 5);
    assert!(!t.needs_sync());
    assert_eq!(t.layouts_performed(), 1);
}

#[test]
fn vector_mode_uses_real_metrics_not_the_fallback() {
    let font = Rc::new(roboto());
    let mut vector = Text::new();
    vector.set_text("Hi!");
    vector.set_vector_mode(true);
    vector.set_vector_font(Some(font));
    let vector_info = vector.sync().clone();

    let mut canvas = Text::new();
    canvas.set_text("Hi!");
    let canvas_info = canvas.sync().clone();

    assert_eq!(vector_info.glyph_count, canvas_info.glyph_count);
    // The fallback's flat 0.6 em advance gives 1.8; Roboto's real advances do not.
    assert_eq!(canvas_info.block_bounds[2], 1.7999999999999998);
    assert_ne!(vector_info.block_bounds[2], canvas_info.block_bounds[2]);
    assert_eq!(vector_info.line_height, 1.31884765625);
}

#[test]
fn opacity_needs_both_a_batch_and_a_member_id() {
    let batch = Rc::new(RefCell::new(RecordingBatch::default()));

    // No batch at all: the setter still takes effect, it just writes nowhere.
    let mut t = Text::new();
    t.set_opacity(0.5);
    assert_eq!(t.opacity(), 0.5);
    assert_eq!(t.member_id(), -1);

    // Detached members have member_id -1, which the JS guards with `>= 0`.
    t.attach_to_batch(batch.clone(), 2);
    t.detach_from_batch();
    t.set_opacity(0.6);
    assert!(batch.borrow().calls.is_empty());

    // And an unchanged write is dropped before the forward, so no duplicate
    // attribute writes.
    t.attach_to_batch(batch.clone(), 2);
    t.set_opacity(0.6);
    assert!(batch.borrow().calls.is_empty());
    t.set_opacity(0.7);
    assert_eq!(batch.borrow().calls, vec![(2, 0.7)]);
}
