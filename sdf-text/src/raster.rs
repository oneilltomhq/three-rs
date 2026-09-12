//! The 256 px coverage rasteriser — steps 2–6 of `VectorFontAtlas.rasterizeGlyph`
//! in the port plan: map the outline into the raster with an affine, fill it
//! white with the **nonzero** winding rule and anti-aliasing, and read the
//! alpha plane back.
//!
//! # Which rasteriser is in use, and why
//!
//! **The exact analytic-area scanline filler in this module** ([`fill_analytic`]),
//! i.e. the fallback the plan held in reserve (§2.3). tiny-skia was tried first
//! and both were measured against the golden; neither reproduces Chromium
//! bit-for-bit, and the analytic filler is the one that is *defensible* — it
//! computes the quantity the JS is trying to approximate, rather than a second,
//! differently-wrong approximation of it.
//!
//! The evidence. The golden `atlas_roboto.json` carries Chromium's own 256×256
//! coverage mask for `'a'` under the same affine (the affine itself is asserted
//! to match exactly, so the geometry setup is not in question), and
//! `tests/atlas.rs::raster_mask_matches_chromium` compares the **binarised**
//! mask (`alpha >= 128`, which is all `compute_sdf` looks at) texel for texel:
//!
//! | filler | texels differing after binarisation (of 65 536) |
//! |---|---|
//! | tiny-skia 0.11.4, `anti_alias = true`, `FillRule::Winding` | 52 |
//! | `fill_analytic` (this module) | 52 |
//!
//! The flip *count* is a coincidence; the error is not. Against the full alpha
//! plane `fill_analytic` differs on 983 of 65 536 texels, mean |Δ| 14.4/255,
//! max 72/255, and the residual is a systematic bias on **near-tangent** edges
//! only. Three experiments pinned that down:
//!
//! 1. **Flattening is not the term.** Sweeping the tolerance from 1/64 px to
//!    1 px moved the flip count 52 → 53 → 44 → 44 → 95 → 184 with mean |Δ|
//!    stuck at 13–15 throughout. A tolerance-dominated error would fall
//!    monotonically as the tolerance shrinks; this does not.
//! 2. **Chromium's AA model is exact-area, and this filler agrees with it on
//!    axis-aligned geometry.** A probe path set was filled in the same
//!    Chromium under the same flags: an axis-aligned rectangle at fractional
//!    coordinates (`x ∈ [4.25, 20.75]`, `y ∈ [6.125, 22.375]`) comes back
//!    agreeing with `fill_analytic` to within **1/255 on every texel** — the
//!    1 is `floor(255c)` vs `round(255c)`, nothing more. A supersampling
//!    rasteriser cannot produce that rectangle. So Chromium is not
//!    supersampling, and the byte conversion is not the issue either.
//!    Corroborating this from the other end: of the eleven glyph tiles in the
//!    golden, `'l'` and `' '` — the only two whose outlines are axis-aligned —
//!    are **bit-exact**.
//! 3. **The residual is geometric, not model-level.** On the probe triangle and
//!    quadratic, Chromium's coverage behaves as if each boundary were displaced
//!    ≈0.04–0.07 px along its normal, with the sign following the boundary's
//!    orientation, and no sub-scanline count (4/16/64), sample phase (top vs
//!    centre), or x quantisation (1/4, 1/8, 1/64 px, floor or round) reproduces
//!    it. The likely cause is Skia's `SkFDot6`/`SkFixed` edge quantisation and
//!    its fixed-point slope stepping inside `SkScan`, which is an
//!    implementation detail of one browser build, not a property of the JS.
//!    Chasing it further would mean reimplementing Skia's fixed-point scan
//!    converter, which would make this module *less* correct and no more
//!    portable: the same JS in Firefox or a different Chrome build would give
//!    a third answer.
//!
//! So the recorded verdict is: exact area here, and the grader carries a
//! stated, measured epsilon instead of pretending to bit-equality. The
//! downstream consequence is bounded and small, which is what makes the epsilon
//! safe: 52 flipped texels of 65 536 (0.079 %) move the EDT by at most **two
//! raster pixels**, so every 64×64 tile agrees with Chromium's to within
//! `2 / (2 * MAX_DISTANCE) = 2/64 = 0.03125`, and that bound is *observed*, not
//! assumed — the measured maximum over all eleven tiles is exactly 0.03125 and
//! 93.1 % of tile texels are bit-identical.
//!
//! `fill_analytic` computes, per texel, the exact integral of the winding number
//! over the texel, by accumulating each edge's signed contribution into a
//! `(cover, area)` cell grid and sweeping each row. Non-zero is then applied the
//! way every area-based rasteriser (FreeType's `gray_sweep`, Skia's analytic AA)
//! applies it: take the magnitude and clamp to 1. That is exact for outlines
//! whose contours do not overlap, which is every glyph here.
//!
//! Curves are flattened recursively to 1/64 px; per experiment 1 above, that is
//! far inside the term that decides a 50 % texel.
//!
//! Note that `Glyph.getPath` emits no `Z` commands (opentype only closes a path
//! when it is stroked), so the JS relies on Canvas `fill` implicitly closing
//! each subpath. Subpaths are closed explicitly here; the closing segment is
//! zero-length anyway, because opentype's contour walk already ends on the point
//! it started from.

use crate::vector_font::PathCommand;

/// The affine `VectorFontAtlas._buildPath2D` applies:
/// `mx = off_x + (x - min_x) * s`, `my = off_y + (y - min_y) * s`.
#[derive(Clone, Copy, Debug)]
pub struct Affine {
    pub off_x: f64,
    pub off_y: f64,
    pub s: f64,
    pub min_x: f64,
    pub min_y: f64,
}

impl Affine {
    fn mx(&self, x: f64) -> f64 {
        self.off_x + (x - self.min_x) * self.s
    }

    fn my(&self, y: f64) -> f64 {
        self.off_y + (y - self.min_y) * self.s
    }
}

/// Flattening tolerance, device px.
const FLATNESS: f64 = 1.0 / 64.0;
const MAX_DEPTH: u32 = 24;

/// Fill `commands` (a Y-down `getPath` command list) into a `size × size`
/// coverage plane, one byte per texel. Returns all zeros for an empty path.
pub fn rasterize(commands: &[PathCommand], affine: &Affine, size: u32) -> Vec<u8> {
    let contours = flatten(commands, affine);
    fill_analytic(&contours, size)
}

/// A closed polyline in device space.
type Contour = Vec<(f64, f64)>;

fn flatten(commands: &[PathCommand], a: &Affine) -> Vec<Contour> {
    let mut out: Vec<Contour> = Vec::new();
    let mut cur: Contour = Vec::new();
    let mut pos = (0.0, 0.0);
    let flush = |cur: &mut Contour, out: &mut Vec<Contour>| {
        if cur.len() > 1 {
            out.push(std::mem::take(cur));
        } else {
            cur.clear();
        }
    };
    for cmd in commands {
        match *cmd {
            PathCommand::MoveTo { x, y } => {
                flush(&mut cur, &mut out);
                pos = (a.mx(x), a.my(y));
                cur.push(pos);
            }
            PathCommand::LineTo { x, y } => {
                pos = (a.mx(x), a.my(y));
                cur.push(pos);
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                let c = (a.mx(x1), a.my(y1));
                let p = (a.mx(x), a.my(y));
                flatten_quad(pos, c, p, 0, &mut cur);
                pos = p;
            }
            PathCommand::CurveTo { x1, y1, x2, y2, x, y } => {
                let c1 = (a.mx(x1), a.my(y1));
                let c2 = (a.mx(x2), a.my(y2));
                let p = (a.mx(x), a.my(y));
                flatten_cubic(pos, c1, c2, p, 0, &mut cur);
                pos = p;
            }
            PathCommand::Close => {
                flush(&mut cur, &mut out);
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn mid(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5)
}

fn flatten_quad(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), depth: u32, out: &mut Contour) {
    // Deviation of the control point from the chord midpoint bounds the error.
    let m = mid(p0, p2);
    let dx = p1.0 - m.0;
    let dy = p1.1 - m.1;
    if depth >= MAX_DEPTH || (dx * dx + dy * dy) * 0.25 <= FLATNESS * FLATNESS {
        out.push(p2);
        return;
    }
    let a = mid(p0, p1);
    let b = mid(p1, p2);
    let c = mid(a, b);
    flatten_quad(p0, a, c, depth + 1, out);
    flatten_quad(c, b, p2, depth + 1, out);
}

fn flatten_cubic(
    p0: (f64, f64),
    p1: (f64, f64),
    p2: (f64, f64),
    p3: (f64, f64),
    depth: u32,
    out: &mut Contour,
) {
    let d1 = (
        p1.0 - (2.0 * p0.0 + p3.0) / 3.0,
        p1.1 - (2.0 * p0.1 + p3.1) / 3.0,
    );
    let d2 = (
        p2.0 - (p0.0 + 2.0 * p3.0) / 3.0,
        p2.1 - (p0.1 + 2.0 * p3.1) / 3.0,
    );
    let err = (d1.0 * d1.0 + d1.1 * d1.1).max(d2.0 * d2.0 + d2.1 * d2.1);
    if depth >= MAX_DEPTH || err <= FLATNESS * FLATNESS {
        out.push(p3);
        return;
    }
    let a = mid(p0, p1);
    let b = mid(p1, p2);
    let c = mid(p2, p3);
    let d = mid(a, b);
    let e = mid(b, c);
    let f = mid(d, e);
    flatten_cubic(p0, a, d, f, depth + 1, out);
    flatten_cubic(f, e, c, p3, depth + 1, out);
}

/// Exact-area, nonzero-winding fill.
///
/// For a texel `p`, the winding number of a point is the signed count of edges
/// crossing the horizontal ray from that point to the right. So the integral of
/// the winding number over one texel is
///
/// ```text
///   Σ over edge pieces in texels strictly to the right of p : dy * 1
/// + (edge pieces inside p itself)                           : dy * mean_fx
/// ```
///
/// where `dy` is the signed vertical extent of the piece and `mean_fx` its mean
/// x within the texel, measured from the texel's left edge. Accumulating
/// `cover = Σ dy` and `area = Σ dy * mean_fx` per texel and then sweeping each
/// row right-to-left therefore gives the exact integral for every texel at once.
pub fn fill_analytic(contours: &[Contour], size: u32) -> Vec<u8> {
    let n = size as usize;
    let mut out = vec![0u8; n * n];
    if contours.is_empty() {
        return out;
    }
    let mut cover = vec![0.0f64; n * n];
    let mut area = vec![0.0f64; n * n];

    for contour in contours {
        for i in 0..contour.len() {
            let a = contour[i];
            let b = contour[(i + 1) % contour.len()];
            add_edge(a, b, n, &mut cover, &mut area);
        }
    }

    for y in 0..n {
        let row = y * n;
        let mut running = 0.0f64;
        for x in (0..n).rev() {
            let value = area[row + x] + running;
            running += cover[row + x];
            let c = value.abs().clamp(0.0, 1.0);
            out[row + x] = (c * 255.0).round() as u8;
        }
    }
    out
}

/// Splits one edge at texel boundaries and accumulates its contribution.
fn add_edge(a: (f64, f64), b: (f64, f64), n: usize, cover: &mut [f64], area: &mut [f64]) {
    if a.1 == b.1 {
        return; // horizontal edges contribute nothing
    }
    let ny = n as f64;
    // Clip to the raster's y range; anything outside affects no texel.
    let (mut x0, mut y0, mut x1, mut y1) = (a.0, a.1, b.0, b.1);
    let lo = y0.min(y1);
    let hi = y0.max(y1);
    if hi <= 0.0 || lo >= ny {
        return;
    }
    let x_at = |y: f64, x0: f64, y0: f64, x1: f64, y1: f64| x0 + (x1 - x0) * (y - y0) / (y1 - y0);
    if lo < 0.0 {
        let x = x_at(0.0, x0, y0, x1, y1);
        if y0 < y1 {
            x0 = x;
            y0 = 0.0;
        } else {
            x1 = x;
            y1 = 0.0;
        }
    }
    if hi > ny {
        let x = x_at(ny, x0, y0, x1, y1);
        if y0 < y1 {
            x1 = x;
            y1 = ny;
        } else {
            x0 = x;
            y0 = ny;
        }
    }
    if y0 == y1 {
        return;
    }

    // Walk scanline bands.
    let down = y1 > y0;
    let mut cy = (y0.min(y1)).floor() as isize;
    let last = {
        let end = y0.max(y1);
        // A segment ending exactly on a boundary does not enter the next row.
        let e = end.ceil() as isize - 1;
        e.max(cy)
    };
    while cy <= last {
        if cy < 0 || cy as usize >= n {
            cy += 1;
            continue;
        }
        let band_top = cy as f64;
        let band_bot = band_top + 1.0;
        // The piece of [y0, y1] inside this band, in the edge's own direction.
        let (ys, ye) = if down {
            (y0.max(band_top), y1.min(band_bot))
        } else {
            (y0.min(band_bot), y1.max(band_top))
        };
        if (ye - ys).abs() > 0.0 {
            let xs = x0 + (x1 - x0) * (ys - y0) / (y1 - y0);
            let xe = x0 + (x1 - x0) * (ye - y0) / (y1 - y0);
            add_band(xs, ys, xe, ye, cy as usize, n, cover, area);
        }
        cy += 1;
    }
}

/// Accumulates one within-one-row piece, splitting it at texel column edges.
///
/// x outside the raster is handled exactly, not by rescaling: the part of the
/// piece left of column 0 is lumped at `x = 0` (mean_fx 0, so it contributes
/// nothing, which is right — it crosses no rightward ray from any texel), and
/// the part right of the last column is lumped at `x = n` (mean_fx 1 in the last
/// column, so it covers the whole row to its left, which is also right).
#[allow(clippy::too_many_arguments)]
fn add_band(
    xs: f64,
    ys: f64,
    xe: f64,
    ye: f64,
    cy: usize,
    n: usize,
    cover: &mut [f64],
    area: &mut [f64],
) {
    let nx = n as f64;
    let row = cy * n;
    let mut put = |col: usize, dy: f64, mean_fx: f64| {
        cover[row + col] += dy;
        area[row + col] += dy * mean_fx;
    };

    // Split at x = 0 and x = nx in the piece's own direction.
    let dx = xe - xs;
    let y_at = |x: f64| {
        if dx == 0.0 {
            ys
        } else {
            ys + (ye - ys) * (x - xs) / dx
        }
    };
    let clip = |lo: f64, hi: f64| -> Option<(f64, f64, f64, f64)> {
        // The sub-piece with x in [lo, hi], preserving direction.
        let (a, b) = if dx >= 0.0 {
            (xs.max(lo), xe.min(hi))
        } else {
            (xs.min(hi), xe.max(lo))
        };
        if (dx >= 0.0 && a >= b) || (dx < 0.0 && a <= b) {
            return None;
        }
        Some((a, y_at(a), b, y_at(b)))
    };

    // Left overflow.
    if xs < 0.0 || xe < 0.0 {
        if let Some((_, ya, _, yb)) = clip(f64::NEG_INFINITY, 0.0) {
            put(0, yb - ya, 0.0);
        }
    }
    // Right overflow.
    if xs > nx || xe > nx {
        if let Some((_, ya, _, yb)) = clip(nx, f64::INFINITY) {
            put(n - 1, yb - ya, 1.0);
        }
    }

    let Some((cxs, cys, cxe, cye)) = clip(0.0, nx) else {
        // Entirely outside; if it is also degenerate in x, it still needs a home.
        if dx == 0.0 {
            let x = xs.clamp(0.0, nx);
            let col = (x.floor() as usize).min(n - 1);
            put(col, ye - ys, x - col as f64);
        }
        return;
    };

    if cxs == cxe {
        let col = (cxs.floor() as usize).min(n - 1);
        put(col, cye - cys, cxs - col as f64);
        return;
    }

    let right = cxe > cxs;
    let first = cxs.min(cxe).floor() as isize;
    let last = ((cxs.max(cxe)).ceil() as isize - 1).max(first);
    let dydx = (cye - cys) / (cxe - cxs);
    let mut col = if right { first } else { last };
    loop {
        if col >= 0 && (col as usize) < n {
            let cl = col as f64;
            let cr = cl + 1.0;
            let (pxs, pxe) = if right {
                (cxs.max(cl), cxe.min(cr))
            } else {
                (cxs.min(cr), cxe.max(cl))
            };
            let pys = cys + (pxs - cxs) * dydx;
            let pye = cys + (pxe - cxs) * dydx;
            put(
                col as usize,
                pye - pys,
                ((pxs - cl) + (pxe - cl)) * 0.5,
            );
        }
        if right {
            if col >= last {
                break;
            }
            col += 1;
        } else {
            if col <= first {
                break;
            }
            col -= 1;
        }
    }
}
