//! Ladder step 2, part 2 — the raster + EDT + atlas pipeline.
//!
//! Graded against `tests/golden/atlas_roboto.json`, dumped from Chromium under
//! the plan's flag list:
//!
//! - the 256×256 **binarised** coverage mask for `'a'` (`alpha >= 128`, which is
//!   all `compute_sdf` ever looks at);
//! - the 64×64 `f32` tiles and the `u`/`v`/`w`/`h`/`viewBox` metrics for
//!   `"AVo.e5 flare"` in insertion order.
//!
//! **Metrics and slots are exact; the two rasterised quantities carry a measured
//! epsilon.** The reason is recorded in full in `src/raster.rs`'s module doc:
//! Chromium's coverage is exact-area (proved against an axis-aligned probe
//! rectangle, and confirmed here by `'l'` and `' '` being bit-exact), but its
//! sloped and curved boundaries sit ≈0.05 px off the analytic position, which
//! no AA model reproduces and which is almost certainly Skia's fixed-point edge
//! quantisation. The epsilons below are the *observed* consequences of that, and
//! are tight enough that a real porting mistake cannot hide under them — each
//! one also asserts the error does not grow.

mod common;

use common::{as_f32_vec, b64, golden, roboto};
use sdf_text::raster::{rasterize, Affine};
use sdf_text::vector_font::path_bounding_box;
use sdf_text::vector_font_atlas::{GlyphMetrics, VectorFontAtlas, PAD_FRAC, RASTER};

/// The string the atlas golden was dumped over, in order.
const ATLAS_CHARS: &str = "AVo.e5 flare";

#[test]
fn raster_mask_matches_chromium() {
    let g = golden("atlas_roboto.json");
    let want = b64(g["raster_a"].as_str().unwrap());
    assert_eq!(want.len(), 256 * 256);

    let p = &g["raster_a_params"];
    let font = roboto();
    let commands = font.glyph_path_y_down(font.glyph_for_char('a'));
    let pb = path_bounding_box(&commands);

    // The affine the golden used, recomputed here — if these disagree the mask
    // comparison below would be meaningless.
    let gw = pb.x2 - pb.x1;
    let gh = pb.y2 - pb.y1;
    let avail = RASTER as f64 - 2.0 * (RASTER as f64 * PAD_FRAC);
    let s = avail / gw.max(gh);
    let off_x = (RASTER as f64 - gw * s) / 2.0;
    let off_y = (RASTER as f64 - gh * s) / 2.0;
    assert_eq!(s, p["s"].as_f64().unwrap(), "affine scale");
    assert_eq!(off_x, p["offX"].as_f64().unwrap(), "affine offX");
    assert_eq!(off_y, p["offY"].as_f64().unwrap(), "affine offY");
    assert_eq!(pb.x1, p["minX"].as_f64().unwrap(), "affine minX");
    assert_eq!(pb.y1, p["minY"].as_f64().unwrap(), "affine minY");

    let got = rasterize(
        &commands,
        &Affine {
            off_x,
            off_y,
            s,
            min_x: pb.x1,
            min_y: pb.y1,
        },
        RASTER,
    );

    // Only the binarised set matters: compute_sdf thresholds at alpha >= 128.
    let mut diff = 0usize;
    let mut first = None;
    for i in 0..want.len() {
        if (got[i] >= 128) != (want[i] >= 128) {
            diff += 1;
            if first.is_none() {
                first = Some((i % 256, i / 256, got[i], want[i]));
            }
        }
    }
    // Measured: 52 of 65 536 (0.079 %), all on near-tangent curve segments. The
    // bound is deliberately just above the measurement, so any regression that
    // actually changes the fill — a winding-rule slip, a half-texel offset, a
    // Y-flip — blows straight past it (those were all seen to produce
    // thousands of flips while this was being debugged).
    assert!(
        diff <= 64,
        "{diff} of {} texels disagree after binarisation (expected <= 64); first at {first:?}",
        want.len()
    );
    // And the flips must stay on the *boundary*: no flipped texel may be far
    // from the 50 % contour, which is what rules out a systematic error.
    for i in 0..want.len() {
        if (got[i] >= 128) != (want[i] >= 128) {
            let d = (got[i] as i32 - want[i] as i32).abs();
            assert!(
                d <= 96,
                "texel ({}, {}) flipped by {d} (got {} want {}) — too far from the \
                 50 % contour to be an AA-convention difference",
                i % 256,
                i / 256,
                got[i],
                want[i]
            );
        }
    }
    // And the golden really does contain ink, so a blank mask cannot pass.
    assert!(want.iter().filter(|&&v| v >= 128).count() > 10_000);
}

#[test]
fn atlas_tiles_and_metrics_match_chromium() {
    let g = golden("atlas_roboto.json");
    let tiles = g["tiles"].as_object().unwrap();
    let font = roboto();

    let mut atlas = VectorFontAtlas::new(1024);
    // Insertion order is slot order, so this loop *is* the layout.
    for ch in ATLAS_CHARS.chars() {
        atlas.get_glyph(Some(&font), ch);
    }

    let mut checked = 0usize;
    for (key, want) in tiles {
        let ch = key.chars().next().unwrap();
        assert_eq!(key.chars().count(), 1);

        let want_slot = want["slot"].as_u64().unwrap() as u32;
        assert_eq!(atlas.slot_of(ch), Some(want_slot), "slot for {ch:?}");

        let m = atlas.get_glyph(Some(&font), ch);
        let wm = &want["metrics"];
        let want_m = GlyphMetrics {
            u: wm["u"].as_f64().unwrap(),
            v: wm["v"].as_f64().unwrap(),
            w: wm["w"].as_f64().unwrap(),
            h: wm["h"].as_f64().unwrap(),
            view_box: {
                let vb = wm["viewBox"].as_array().unwrap();
                [
                    vb[0].as_f64().unwrap(),
                    vb[1].as_f64().unwrap(),
                    vb[2].as_f64().unwrap(),
                    vb[3].as_f64().unwrap(),
                ]
            },
        };
        assert_eq!(m, want_m, "metrics for {ch:?}");

        let want_tile = as_f32_vec(&b64(want["tile"].as_str().unwrap()));
        assert_eq!(want_tile.len(), 64 * 64);
        let got_tile = atlas.tile(want_slot);

        // One flipped raster texel moves the EDT by at most one raster pixel, so
        // the encode `0.5 - d / 64` moves by at most 1/64. Two flips can stack;
        // three cannot, because the flips are isolated boundary texels. The
        // measured maximum over all eleven tiles is exactly 2/64, so the bound
        // is the real one, not a guess.
        const TILE_EPS: f32 = 2.0 / 64.0;
        let mut differing = 0usize;
        for i in 0..want_tile.len() {
            let d = (got_tile[i] - want_tile[i]).abs();
            assert!(
                d <= TILE_EPS,
                "tile {ch:?} at ({}, {}): got {} want {} (delta {d}, eps {TILE_EPS})",
                i % 64,
                i / 64,
                got_tile[i],
                want_tile[i]
            );
            if d > 0.0 {
                differing += 1;
            }
        }
        // Most of every tile is bit-exact; the epsilon is not doing the work.
        assert!(
            differing * 100 <= want_tile.len() * 20,
            "tile {ch:?}: {differing} of {} texels differ at all — the epsilon is \
             covering too much",
            want_tile.len()
        );
        // The two glyphs whose outlines are entirely axis-aligned must be exact:
        // this is the assertion that proves the AA model itself is right.
        if ch == 'l' || ch == ' ' {
            for i in 0..want_tile.len() {
                assert_eq!(
                    got_tile[i].to_bits(),
                    want_tile[i].to_bits(),
                    "axis-aligned tile {ch:?} at ({}, {}) must be bit-exact",
                    i % 64,
                    i / 64
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 11, "the golden has 11 unique glyphs");
}

#[test]
fn space_occupies_a_slot_but_stays_blank() {
    let font = roboto();
    let mut atlas = VectorFontAtlas::new(1024);
    for ch in ATLAS_CHARS.chars() {
        atlas.get_glyph(Some(&font), ch);
    }
    let m = atlas.get_glyph(Some(&font), ' ');
    assert_eq!(atlas.slot_of(' '), Some(6));
    assert_eq!(m.view_box, [0.0, 0.0, 0.0, 0.0]);
    // 0.5 - INF, clamped to 0.
    assert!(atlas.tile(6).iter().all(|&v| v == 0.0));
}

#[test]
fn glyphs_before_the_font_arrives_are_blank() {
    let mut atlas = VectorFontAtlas::new(1024);
    let m = atlas.get_glyph(None, 'A');
    assert_eq!(m.view_box, [0.0, 0.0, 0.0, 0.0]);
    assert_eq!((m.u, m.v, m.w, m.h), (0.0, 0.0, 0.0625, 0.0625));
    assert!(atlas.tile(0).iter().all(|&v| v == 0.0));
    atlas.reset();
    assert!(!atlas.has_glyph('A'));
    assert_eq!(atlas.slot_of('A'), None);
}

// ── Layer 3 of the lib3 test: atlas UV maths, with its own constants ───────

#[test]
fn layer3_atlas_uv_math() {
    use sdf_text::sdf_defaults::SDF_SIZE;
    const ATLAS_SIZE: u32 = 512;
    let cols = ATLAS_SIZE / SDF_SIZE;
    let uv = |slot: u32| {
        let col = slot % cols;
        let row = slot / cols;
        (
            (col * SDF_SIZE) as f64 / ATLAS_SIZE as f64,
            (row * SDF_SIZE) as f64 / ATLAS_SIZE as f64,
            SDF_SIZE as f64 / ATLAS_SIZE as f64,
            SDF_SIZE as f64 / ATLAS_SIZE as f64,
        )
    };
    let (u, v, _, _) = uv(0);
    assert_eq!(u, 0.0);
    assert_eq!(v, 0.0);
    for s in 0..cols * 2 {
        let (u, v, w, h) = uv(s);
        assert!(u + w <= 1.0001);
        assert!(v + h <= 1.0001);
    }
}

#[test]
fn insertion_order_decides_uvs() {
    let font = roboto();
    let mut a = VectorFontAtlas::new(1024);
    let mut b = VectorFontAtlas::new(1024);
    for ch in "AB".chars() {
        a.get_glyph(Some(&font), ch);
    }
    for ch in "BA".chars() {
        b.get_glyph(Some(&font), ch);
    }
    assert_ne!(
        a.get_glyph(Some(&font), 'A').u,
        b.get_glyph(Some(&font), 'A').u
    );
}
