//! Parsed font with real typographic metrics and kerning — a port of lib3's
//! `src/sdf-text/VectorFont.js`, which itself is a thin layer over opentype.js.
//!
//! The grader is `tests/golden/font_roboto.json`, dumped from opentype.js
//! 2.0.0, so this module's job is not "be a correct font library" but "be
//! opentype.js". Three places where that means *not* using the obvious
//! `ttf-parser` call:
//!
//! 1. **Outline commands** (`glyph_path`). opentype.js walks each TrueType
//!    contour starting from its *last* point and then iterates every point of
//!    the contour, so it emits one segment per point — including a final
//!    segment back to the point it moved to, and a degenerate `L` at every
//!    on-curve point that terminates a `Q`. `ttf-parser`'s `OutlineBuilder`
//!    emits the clean form. We reproduce opentype's form by parsing `glyf`
//!    ourselves, because the command list is graded verbatim.
//! 2. **Ink bbox** (`bounding_box`). opentype's `Path.getBoundingBox` solves
//!    the exact curve extrema (quads elevated to cubics with the 2/3–1/3 rule);
//!    `ttf-parser`'s `glyph_bounding_box` returns the stored `glyf` header box.
//!    They disagree, and the difference moves every glyph quad.
//! 3. **Kerning** (`kerning`). opentype.js's `getKerningValue` uses GPOS only —
//!    and never falls back to `kern` — whenever the GPOS table exists, and
//!    returns the *first* coverage hit over the type-2 lookups of the `kern`
//!    feature in the default script's default langsys. PairPos format 2 returns
//!    on the first covered subtable even when the class pair is zero.
//!
//! Metric precedence uses JS `||`, so a **zero** value falls through
//! (`sTypoLineGap == 0` → `hhea.lineGap` → `0`). That is deliberate.

use std::collections::HashMap;

use owned_ttf_parser::{AsFaceRef, Face, GlyphId, OwnedFace, Tag};

use crate::error::Error;

/// A path command in the opentype.js `Path.commands` vocabulary.
///
/// `Close` exists because `Glyph.path` (the Y-up path the bbox is measured on)
/// contains `Z`; `Glyph.getPath(0, 0, unitsPerEm)` drops it (it only emits `Z`
/// when the path is stroked), so the rasteriser never sees one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    /// Quadratic: one control point.
    QuadTo {
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    /// Cubic: two control points. TrueType never produces these; CFF does.
    CurveTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    Close,
}

impl PathCommand {
    /// The opentype.js `cmd.type` letter, for golden comparison.
    pub fn type_letter(&self) -> &'static str {
        match self {
            PathCommand::MoveTo { .. } => "M",
            PathCommand::LineTo { .. } => "L",
            PathCommand::QuadTo { .. } => "Q",
            PathCommand::CurveTo { .. } => "C",
            PathCommand::Close => "Z",
        }
    }
}

/// An axis-aligned box, matching opentype's `BoundingBox` field names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// One raw `glyf` point.
#[derive(Clone, Copy, Debug)]
struct GlyfPoint {
    x: f64,
    y: f64,
    on_curve: bool,
    last_of_contour: bool,
}

pub struct VectorFont {
    face: OwnedFace,
    /// Source URL/label, for cache keying. Mirrors `VectorFont.src`.
    pub src: String,
    pub units_per_em: f64,
    pub ascender: f64,
    pub descender: f64,
    pub line_gap: f64,
    pub cap_height: f64,
    pub x_height: f64,
    /// GPOS type-2 lookup subtable offsets reachable from the default script's
    /// `kern` feature, in lookup-then-subtable order. Absolute offsets into the
    /// GPOS table bytes. `None` when there is no GPOS table at all — the only
    /// case in which opentype.js consults `kern`.
    kern_subtables: Option<Vec<usize>>,
}

impl VectorFont {
    /// Port of `new VectorFont(opentype.parse(buffer), src)`.
    ///
    /// `VectorFont.load` in the JS also accepts a URL and `fetch`es it; there
    /// is no fetch here, so callers read the bytes themselves (see the skip
    /// register in the crate README).
    pub fn parse(data: Vec<u8>, src: impl Into<String>) -> Result<Self, Error> {
        let src = src.into();
        let face = OwnedFace::from_vec(data, 0).map_err(|source| Error::Parse {
            src: src.clone(),
            source,
        })?;

        let (upem, asc, desc, gap, cap, xh) = {
            let f = face.as_face_ref();
            let tables = f.tables();
            // JS: (os2 && os2.sTypoAscender) || (hhea && hhea.ascender) || font.ascender
            // `||` so a zero falls through. opentype's own `font.ascender` is
            // the hhea value, so the third arm repeats the second.
            let os2 = tables.os2;
            let hhea_asc = tables.hhea.ascender as f64;
            let hhea_desc = tables.hhea.descender as f64;
            let hhea_gap = tables.hhea.line_gap as f64;
            let or0 = |a: f64, b: f64, c: f64| {
                if a != 0.0 {
                    a
                } else if b != 0.0 {
                    b
                } else {
                    c
                }
            };
            let asc = or0(
                os2.map(|t| t.typographic_ascender() as f64).unwrap_or(0.0),
                hhea_asc,
                hhea_asc,
            );
            let desc = or0(
                os2.map(|t| t.typographic_descender() as f64).unwrap_or(0.0),
                hhea_desc,
                hhea_desc,
            );
            let gap = or0(
                os2.map(|t| t.typographic_line_gap() as f64).unwrap_or(0.0),
                hhea_gap,
                0.0,
            );
            // capHeight / xHeight: `(os2 && os2.sCapHeight) || 0`.
            let cap = os2
                .and_then(|t| t.capital_height())
                .map(|v| v as f64)
                .unwrap_or(0.0);
            let xh = os2
                .and_then(|t| t.x_height())
                .map(|v| v as f64)
                .unwrap_or(0.0);
            (f.units_per_em() as f64, asc, desc, gap, cap, xh)
        };

        let kern_subtables = {
            let f = face.as_face_ref();
            f.raw_face()
                .table(Tag::from_bytes(b"GPOS"))
                .map(collect_kern_subtables)
        };

        Ok(VectorFont {
            face,
            src,
            units_per_em: upem,
            ascender: asc,
            descender: desc,
            line_gap: gap,
            cap_height: cap,
            x_height: xh,
            kern_subtables,
        })
    }

    fn face(&self) -> &Face<'_> {
        self.face.as_face_ref()
    }

    pub fn has_gpos(&self) -> bool {
        self.kern_subtables.is_some()
    }

    pub fn has_kern(&self) -> bool {
        self.face()
            .raw_face()
            .table(Tag::from_bytes(b"kern"))
            .is_some()
    }

    /// Port of `glyphForChar`. opentype.js's `charToGlyph` falls back to glyph 0
    /// (`.notdef`) for an unmapped character rather than returning null, so this
    /// never fails for a parsed font.
    pub fn glyph_for_char(&self, ch: char) -> GlyphId {
        self.face().glyph_index(ch).unwrap_or(GlyphId(0))
    }

    /// Horizontal advance in font units.
    pub fn advance_width(&self, ch: char) -> f64 {
        let gid = self.glyph_for_char(ch);
        self.face().glyph_hor_advance(gid).unwrap_or(0) as f64
    }

    /// Ink bounding box in font units, **Y-up**; `None` for blank glyphs.
    ///
    /// JS: `if (!(b.x2 > b.x1 && b.y2 > b.y1)) return null;` — so the all-zero
    /// box opentype hands back for an empty outline becomes `None`.
    pub fn bounding_box(&self, ch: char) -> Option<BBox> {
        let gid = self.glyph_for_char(ch);
        let b = path_bounding_box(&self.glyph_path(gid));
        if b.x2 > b.x1 && b.y2 > b.y1 {
            Some(b)
        } else {
            None
        }
    }

    /// `glyph.path` — the Y-up command list, `Z`-terminated per contour.
    pub fn glyph_path(&self, gid: GlyphId) -> Vec<PathCommand> {
        let mut points = Vec::new();
        self.collect_points(gid, &mut points, &mut Vec::new());
        contour_path(&points)
    }

    /// `glyph.getPath(0, 0, unitsPerEm)` — the same outline with Y negated and
    /// the `Z` commands dropped (opentype only emits `Z` for stroked paths).
    /// `fontSize == unitsPerEm` makes the scale exactly 1, so coordinates stay
    /// at their font-unit values (halves appear from implied midpoints).
    pub fn glyph_path_y_down(&self, gid: GlyphId) -> Vec<PathCommand> {
        self.glyph_path(gid)
            .into_iter()
            .filter_map(|c| match c {
                PathCommand::MoveTo { x, y } => Some(PathCommand::MoveTo { x, y: -y }),
                PathCommand::LineTo { x, y } => Some(PathCommand::LineTo { x, y: -y }),
                PathCommand::QuadTo { x1, y1, x, y } => Some(PathCommand::QuadTo {
                    x1,
                    y1: -y1,
                    x,
                    y: -y,
                }),
                PathCommand::CurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => Some(PathCommand::CurveTo {
                    x1,
                    y1: -y1,
                    x2,
                    y2: -y2,
                    x,
                    y: -y,
                }),
                PathCommand::Close => None,
            })
            .collect()
    }

    /// Kerning adjustment in font units. See the module doc for the semantics.
    pub fn kerning(&self, left: char, right: char) -> f64 {
        let l = self.glyph_for_char(left);
        let r = self.glyph_for_char(right);
        self.kerning_by_gid(l, r)
    }

    pub fn kerning_by_gid(&self, left: GlyphId, right: GlyphId) -> f64 {
        let Some(subtables) = self.kern_subtables.as_ref() else {
            // GPOS absent: opentype.js reads `kerningPairs` (the legacy `kern`
            // table). Not implemented — see the crate README's skip register.
            // Every font in this port has GPOS.
            return 0.0;
        };
        let face = self.face();
        let Some(gpos) = face.raw_face().table(Tag::from_bytes(b"GPOS")) else {
            return 0.0;
        };
        for &off in subtables {
            let Some(sub) = gpos.get(off..) else { continue };
            match pair_pos_lookup(sub, left, right) {
                PairPosResult::NotCovered => continue,
                PairPosResult::NoPair => continue,
                PairPosResult::Value(v) => return v as f64,
            }
        }
        0.0
    }

    // ── glyf point collection (opentype.js `parseGlyph` + `buildPath`) ──────

    fn collect_points(&self, gid: GlyphId, out: &mut Vec<GlyfPoint>, resolving: &mut Vec<u16>) {
        let face = self.face();
        let Some(glyf) = face.raw_face().table(Tag::from_bytes(b"glyf")) else {
            return;
        };
        // `FaceTables` does not expose `loca`, so read it raw. `indexToLocFormat`
        // lives at head+50.
        let Some(loca) = face.raw_face().table(Tag::from_bytes(b"loca")) else {
            return;
        };
        let Some(head) = face.raw_face().table(Tag::from_bytes(b"head")) else {
            return;
        };
        if head.len() < 52 {
            return;
        }
        let long = read_i16(head, 50) != 0;
        // An empty loca range means an empty glyph; opentype never calls
        // parseGlyph for it, so it has no points at all.
        let Some(range) = loca_range(loca, gid.0, long) else {
            return;
        };
        let Some(data) = glyf.get(range) else { return };
        if data.len() < 10 {
            return;
        }
        let ncont = read_i16(data, 0);
        if ncont > 0 {
            parse_simple_glyph(data, ncont as usize, out);
        } else if ncont < 0 {
            resolving.push(gid.0);
            for comp in parse_composite(data) {
                if resolving.contains(&comp.glyph_index) {
                    continue;
                }
                let mut sub = Vec::new();
                self.collect_points(GlyphId(comp.glyph_index), &mut sub, resolving);
                // `matchedPoints` (ARGS_ARE_XY_VALUES clear) is not supported;
                // see the crate README's skip register.
                for p in sub.iter_mut() {
                    let (x, y) = (p.x, p.y);
                    p.x = comp.x_scale * x + comp.scale10 * y + comp.dx;
                    p.y = comp.scale01 * x + comp.y_scale * y + comp.dy;
                }
                out.extend_from_slice(&sub);
            }
            resolving.pop();
        }
    }
}

/// `loca` lookup: the half-open byte range of a glyph inside `glyf`, or `None`
/// when the entry is empty (a blank glyph) or out of range.
fn loca_range(loca: &[u8], gid: u16, long: bool) -> Option<std::ops::Range<usize>> {
    let (start, end) = if long {
        let at = gid as usize * 4;
        if at + 8 > loca.len() {
            return None;
        }
        (
            u32::from_be_bytes([loca[at], loca[at + 1], loca[at + 2], loca[at + 3]]) as usize,
            u32::from_be_bytes([loca[at + 4], loca[at + 5], loca[at + 6], loca[at + 7]]) as usize,
        )
    } else {
        let at = gid as usize * 2;
        if at + 4 > loca.len() {
            return None;
        }
        (
            read_u16(loca, at) as usize * 2,
            read_u16(loca, at + 2) as usize * 2,
        )
    };
    if start >= end {
        None
    } else {
        Some(start..end)
    }
}

// ── opentype.js's contour walk (`getPath(points)`) ──────────────────────────

fn contour_path(points: &[GlyfPoint]) -> Vec<PathCommand> {
    let mut out = Vec::new();
    let mut contour: Vec<GlyfPoint> = Vec::new();
    for p in points {
        contour.push(*p);
        if !p.last_of_contour {
            continue;
        }
        emit_contour(&contour, &mut out);
        contour.clear();
    }
    out
}

fn emit_contour(contour: &[GlyfPoint], out: &mut Vec<PathCommand>) {
    if contour.is_empty() {
        return;
    }
    let mut curr = contour[contour.len() - 1];
    let mut next = contour[0];
    if curr.on_curve {
        out.push(PathCommand::MoveTo {
            x: curr.x,
            y: curr.y,
        });
    } else if next.on_curve {
        out.push(PathCommand::MoveTo {
            x: next.x,
            y: next.y,
        });
    } else {
        out.push(PathCommand::MoveTo {
            x: (curr.x + next.x) * 0.5,
            y: (curr.y + next.y) * 0.5,
        });
    }
    for i in 0..contour.len() {
        curr = next;
        next = contour[(i + 1) % contour.len()];
        if curr.on_curve {
            out.push(PathCommand::LineTo {
                x: curr.x,
                y: curr.y,
            });
        } else {
            let (nx, ny) = if next.on_curve {
                (next.x, next.y)
            } else {
                ((curr.x + next.x) * 0.5, (curr.y + next.y) * 0.5)
            };
            out.push(PathCommand::QuadTo {
                x1: curr.x,
                y1: curr.y,
                x: nx,
                y: ny,
            });
        }
    }
    out.push(PathCommand::Close);
}

// ── opentype's exact-extrema bounding box (`Path.getBoundingBox`) ───────────

/// Exact-extrema bbox over a Y-up or Y-down command list, in `f64`.
/// Empty path → `(0, 0, 0, 0)`, matching `if (box.isEmpty()) box.addPoint(0,0)`.
pub fn path_bounding_box(commands: &[PathCommand]) -> BBox {
    let mut b = Builder::new();
    let (mut start_x, mut start_y) = (0.0, 0.0);
    let (mut prev_x, mut prev_y) = (0.0, 0.0);
    for cmd in commands {
        match *cmd {
            PathCommand::MoveTo { x, y } => {
                b.add_point(Some(x), Some(y));
                start_x = x;
                start_y = y;
                prev_x = x;
                prev_y = y;
            }
            PathCommand::LineTo { x, y } => {
                b.add_point(Some(x), Some(y));
                prev_x = x;
                prev_y = y;
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                b.add_quad(prev_x, prev_y, x1, y1, x, y);
                prev_x = x;
                prev_y = y;
            }
            PathCommand::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                b.add_bezier(prev_x, prev_y, x1, y1, x2, y2, x, y);
                prev_x = x;
                prev_y = y;
            }
            PathCommand::Close => {
                prev_x = start_x;
                prev_y = start_y;
            }
        }
    }
    if b.empty {
        b.add_point(Some(0.0), Some(0.0));
    }
    BBox {
        x1: b.x1,
        y1: b.y1,
        x2: b.x2,
        y2: b.y2,
    }
}

struct Builder {
    empty: bool,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl Builder {
    fn new() -> Self {
        Builder {
            empty: true,
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
        }
    }

    /// `BoundingBox.addPoint`, where a `null` argument skips that axis. The JS
    /// initialises both axes from NaN and seeds them on the first numeric value
    /// *of that axis*, but every path starts with an `M` that seeds both, so a
    /// single `empty` flag is equivalent.
    fn add_point(&mut self, x: Option<f64>, y: Option<f64>) {
        if self.empty {
            if let Some(x) = x {
                self.x1 = x;
                self.x2 = x;
            }
            if let Some(y) = y {
                self.y1 = y;
                self.y2 = y;
            }
            if x.is_some() && y.is_some() {
                self.empty = false;
            }
            return;
        }
        if let Some(x) = x {
            if x < self.x1 {
                self.x1 = x;
            }
            if x > self.x2 {
                self.x2 = x;
            }
        }
        if let Some(y) = y {
            if y < self.y1 {
                self.y1 = y;
            }
            if y > self.y2 {
                self.y2 = y;
            }
        }
    }

    /// `BoundingBox.addQuad` — elevate to a cubic with the 2/3–1/3 rule.
    fn add_quad(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, x: f64, y: f64) {
        let cp1x = x0 + 2.0 / 3.0 * (x1 - x0);
        let cp1y = y0 + 2.0 / 3.0 * (y1 - y0);
        let cp2x = cp1x + 1.0 / 3.0 * (x - x0);
        let cp2y = cp1y + 1.0 / 3.0 * (y - y0);
        self.add_bezier(x0, y0, cp1x, cp1y, cp2x, cp2y, x, y);
    }

    /// `BoundingBox.addBezier` — solve the derivative quadratic per axis and add
    /// the curve point at every root strictly inside (0, 1).
    #[allow(clippy::too_many_arguments)]
    fn add_bezier(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        let p0 = [x0, y0];
        let p1 = [x1, y1];
        let p2 = [x2, y2];
        let p3 = [x, y];
        self.add_point(Some(x0), Some(y0));
        self.add_point(Some(x), Some(y));
        for i in 0..2 {
            let b = 6.0 * p0[i] - 12.0 * p1[i] + 6.0 * p2[i];
            let a = -3.0 * p0[i] + 9.0 * p1[i] - 9.0 * p2[i] + 3.0 * p3[i];
            let c = 3.0 * p1[i] - 3.0 * p0[i];
            let mut add = |v: f64, axis: usize| {
                if axis == 0 {
                    self.add_point(Some(v), None)
                } else {
                    self.add_point(None, Some(v))
                }
            };
            if a == 0.0 {
                if b == 0.0 {
                    continue;
                }
                let t = -c / b;
                if 0.0 < t && t < 1.0 {
                    add(derive(p0[i], p1[i], p2[i], p3[i], t), i);
                }
                continue;
            }
            let b2ac = b * b - 4.0 * c * a;
            if b2ac < 0.0 {
                continue;
            }
            let t1 = (-b + b2ac.sqrt()) / (2.0 * a);
            if 0.0 < t1 && t1 < 1.0 {
                add(derive(p0[i], p1[i], p2[i], p3[i], t1), i);
            }
            let t2 = (-b - b2ac.sqrt()) / (2.0 * a);
            if 0.0 < t2 && t2 < 1.0 {
                add(derive(p0[i], p1[i], p2[i], p3[i], t2), i);
            }
        }
    }
}

fn derive(v0: f64, v1: f64, v2: f64, v3: f64, t: f64) -> f64 {
    (1.0 - t).powi(3) * v0
        + 3.0 * (1.0 - t).powi(2) * t * v1
        + 3.0 * (1.0 - t) * t.powi(2) * v2
        + t.powi(3) * v3
}

// ── `glyf` parsing ─────────────────────────────────────────────────────────

fn read_i16(d: &[u8], at: usize) -> i16 {
    i16::from_be_bytes([d[at], d[at + 1]])
}

fn read_u16(d: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([d[at], d[at + 1]])
}

fn parse_simple_glyph(data: &[u8], ncont: usize, out: &mut Vec<GlyfPoint>) {
    let mut p = 10;
    let mut end_pts = Vec::with_capacity(ncont);
    for _ in 0..ncont {
        if p + 2 > data.len() {
            return;
        }
        end_pts.push(read_u16(data, p) as usize);
        p += 2;
    }
    if p + 2 > data.len() {
        return;
    }
    let instr_len = read_u16(data, p) as usize;
    p += 2 + instr_len;
    let n = match end_pts.last() {
        Some(&last) => last + 1,
        None => return,
    };

    let mut flags: Vec<u8> = Vec::with_capacity(n);
    while flags.len() < n {
        if p >= data.len() {
            return;
        }
        let flag = data[p];
        p += 1;
        flags.push(flag);
        if flag & 8 != 0 {
            if p >= data.len() {
                return;
            }
            let repeat = data[p];
            p += 1;
            for _ in 0..repeat {
                flags.push(flag);
            }
        }
    }
    flags.truncate(n);

    let base = out.len();
    for (i, flag) in flags.iter().enumerate().take(n) {
        out.push(GlyfPoint {
            x: 0.0,
            y: 0.0,
            on_curve: flag & 1 != 0,
            last_of_contour: end_pts.contains(&i),
        });
    }

    // X then Y, each a delta chain. `parseGlyphCoordinate` with
    // (short, same) = (0x02, 0x10) then (0x04, 0x20).
    let coord = |p: &mut usize, flag: u8, prev: f64, short: u8, same: u8| -> f64 {
        if flag & short != 0 {
            if *p >= data.len() {
                return prev;
            }
            let v = data[*p] as f64;
            *p += 1;
            prev + if flag & same == 0 { -v } else { v }
        } else if flag & same != 0 {
            prev
        } else {
            if *p + 2 > data.len() {
                return prev;
            }
            let v = read_i16(data, *p) as f64;
            *p += 2;
            prev + v
        }
    };

    let mut px = 0.0;
    for i in 0..n {
        px = coord(&mut p, flags[i], px, 0x02, 0x10);
        out[base + i].x = px;
    }
    let mut py = 0.0;
    for i in 0..n {
        py = coord(&mut p, flags[i], py, 0x04, 0x20);
        out[base + i].y = py;
    }
}

struct Component {
    glyph_index: u16,
    x_scale: f64,
    scale01: f64,
    scale10: f64,
    y_scale: f64,
    dx: f64,
    dy: f64,
}

fn parse_composite(data: &[u8]) -> Vec<Component> {
    let mut out = Vec::new();
    let mut p = 10;
    loop {
        if p + 4 > data.len() {
            break;
        }
        let flags = read_u16(data, p);
        let glyph_index = read_u16(data, p + 2);
        p += 4;
        let mut c = Component {
            glyph_index,
            x_scale: 1.0,
            scale01: 0.0,
            scale10: 0.0,
            y_scale: 1.0,
            dx: 0.0,
            dy: 0.0,
        };
        if flags & 1 != 0 {
            if flags & 2 != 0 {
                c.dx = read_i16(data, p) as f64;
                c.dy = read_i16(data, p + 2) as f64;
            }
            p += 4;
        } else {
            if flags & 2 != 0 {
                c.dx = data[p] as i8 as f64;
                c.dy = data[p + 1] as i8 as f64;
            }
            p += 2;
        }
        let f2dot14 = |at: usize| read_i16(data, at) as f64 / 16384.0;
        if flags & 8 != 0 {
            c.x_scale = f2dot14(p);
            c.y_scale = c.x_scale;
            p += 2;
        } else if flags & 64 != 0 {
            c.x_scale = f2dot14(p);
            c.y_scale = f2dot14(p + 2);
            p += 4;
        } else if flags & 128 != 0 {
            c.x_scale = f2dot14(p);
            c.scale01 = f2dot14(p + 2);
            c.scale10 = f2dot14(p + 4);
            c.y_scale = f2dot14(p + 6);
            p += 8;
        }
        out.push(c);
        if flags & 32 == 0 {
            break;
        }
    }
    out
}

// ── GPOS `kern` walk (opentype.js `Position.getKerningValue`) ───────────────

/// Collects the subtable offsets of every **type-2** lookup listed by the
/// `kern` feature of the default script's default langsys, in the order
/// opentype.js walks them.
///
/// Default script name (`getDefaultScriptName`): `DFLT` if present, else `latn`,
/// else none — in which case there is no feature table and the list is empty.
fn collect_kern_subtables(gpos: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if gpos.len() < 10 {
        return out;
    }
    let script_list_off = read_u16(gpos, 4) as usize;
    let feature_list_off = read_u16(gpos, 6) as usize;
    let lookup_list_off = read_u16(gpos, 8) as usize;

    // Script list → the default script's Script table.
    let Some(sl) = gpos.get(script_list_off..) else {
        return out;
    };
    if sl.len() < 2 {
        return out;
    }
    let script_count = read_u16(sl, 0) as usize;
    let mut tags = Vec::with_capacity(script_count);
    for i in 0..script_count {
        let at = 2 + i * 6;
        if at + 6 > sl.len() {
            return out;
        }
        tags.push((
            [sl[at], sl[at + 1], sl[at + 2], sl[at + 3]],
            read_u16(sl, at + 4) as usize,
        ));
    }
    let pick = tags
        .iter()
        .find(|(t, _)| t == b"DFLT")
        .or_else(|| tags.iter().find(|(t, _)| t == b"latn"));
    let Some(&(_, script_off)) = pick else {
        return out;
    };

    // Script table → defaultLangSys (opentype uses it whenever `language` is
    // unset, which `getKerningTables` leaves so).
    let Some(st) = sl.get(script_off..) else {
        return out;
    };
    if st.len() < 2 {
        return out;
    }
    let default_lang_sys_off = read_u16(st, 0) as usize;
    if default_lang_sys_off == 0 {
        return out;
    }
    let Some(ls) = st.get(default_lang_sys_off..) else {
        return out;
    };
    if ls.len() < 6 {
        return out;
    }
    let feature_index_count = read_u16(ls, 4) as usize;
    let mut feature_indices = Vec::with_capacity(feature_index_count);
    for i in 0..feature_index_count {
        let at = 6 + i * 2;
        if at + 2 > ls.len() {
            return out;
        }
        feature_indices.push(read_u16(ls, at) as usize);
    }

    // Feature list → the *first* listed feature tagged `kern` (opentype's
    // `getFeatureTable` returns on the first match).
    let Some(fl) = gpos.get(feature_list_off..) else {
        return out;
    };
    if fl.len() < 2 {
        return out;
    }
    let feature_count = read_u16(fl, 0) as usize;
    let mut feature_off = None;
    for idx in feature_indices {
        if idx >= feature_count {
            continue;
        }
        let at = 2 + idx * 6;
        if at + 6 > fl.len() {
            continue;
        }
        if &fl[at..at + 4] == b"kern" {
            feature_off = Some(read_u16(fl, at + 4) as usize);
            break;
        }
    }
    let Some(feature_off) = feature_off else {
        return out;
    };
    let Some(ft) = fl.get(feature_off..) else {
        return out;
    };
    if ft.len() < 4 {
        return out;
    }
    let lookup_count = read_u16(ft, 2) as usize;
    let mut lookup_indices = Vec::with_capacity(lookup_count);
    for i in 0..lookup_count {
        let at = 4 + i * 2;
        if at + 2 > ft.len() {
            return out;
        }
        lookup_indices.push(read_u16(ft, at) as usize);
    }

    // Lookup list → keep lookupType == 2 (PairPos) only, as
    // `getLookupTables(script, language, 'kern', 2)` does. A type-9 extension
    // lookup wrapping a PairPos is therefore *skipped*, exactly as in the JS.
    let Some(ll) = gpos.get(lookup_list_off..) else {
        return out;
    };
    if ll.len() < 2 {
        return out;
    }
    let total = read_u16(ll, 0) as usize;
    for idx in lookup_indices {
        if idx >= total {
            continue;
        }
        let at = 2 + idx * 2;
        if at + 2 > ll.len() {
            continue;
        }
        let lookup_off = read_u16(ll, at) as usize;
        let abs = lookup_list_off + lookup_off;
        let Some(lt) = gpos.get(abs..) else { continue };
        if lt.len() < 6 {
            continue;
        }
        if read_u16(lt, 0) != 2 {
            continue;
        }
        let sub_count = read_u16(lt, 4) as usize;
        for j in 0..sub_count {
            let sat = 6 + j * 2;
            if sat + 2 > lt.len() {
                break;
            }
            out.push(abs + read_u16(lt, sat) as usize);
        }
    }
    out
}

enum PairPosResult {
    /// The left glyph is not in the subtable's coverage — try the next subtable.
    NotCovered,
    /// Covered but format 1 found no matching second glyph — `break` in the JS,
    /// which also means "try the next subtable".
    NoPair,
    Value(i16),
}

fn pair_pos_lookup(sub: &[u8], left: GlyphId, right: GlyphId) -> PairPosResult {
    if sub.len() < 4 {
        return PairPosResult::NotCovered;
    }
    let format = read_u16(sub, 0);
    let coverage_off = read_u16(sub, 2) as usize;
    let Some(cov) = sub.get(coverage_off..) else {
        return PairPosResult::NotCovered;
    };
    let Some(cov_index) = coverage_index(cov, left) else {
        return PairPosResult::NotCovered;
    };

    match format {
        1 => {
            if sub.len() < 10 {
                return PairPosResult::NotCovered;
            }
            let v1_format = read_u16(sub, 4);
            let v2_format = read_u16(sub, 6);
            let set_count = read_u16(sub, 8) as usize;
            if cov_index as usize >= set_count {
                return PairPosResult::NoPair;
            }
            let at = 10 + cov_index as usize * 2;
            if at + 2 > sub.len() {
                return PairPosResult::NoPair;
            }
            let set_off = read_u16(sub, at) as usize;
            let Some(set) = sub.get(set_off..) else {
                return PairPosResult::NoPair;
            };
            if set.len() < 2 {
                return PairPosResult::NoPair;
            }
            let pair_count = read_u16(set, 0) as usize;
            let rec_size = 2 + value_size(v1_format) + value_size(v2_format);
            for k in 0..pair_count {
                let at = 2 + k * rec_size;
                if at + rec_size > set.len() {
                    break;
                }
                if read_u16(set, at) == right.0 {
                    return PairPosResult::Value(x_advance(set, at + 2, v1_format));
                }
            }
            PairPosResult::NoPair
        }
        2 => {
            if sub.len() < 16 {
                return PairPosResult::NotCovered;
            }
            let v1_format = read_u16(sub, 4);
            let v2_format = read_u16(sub, 6);
            let cd1_off = read_u16(sub, 8) as usize;
            let cd2_off = read_u16(sub, 10) as usize;
            let class1_count = read_u16(sub, 12) as usize;
            let class2_count = read_u16(sub, 14) as usize;
            let c1 = sub
                .get(cd1_off..)
                .map(|d| glyph_class(d, left))
                .unwrap_or(0) as usize;
            let c2 = sub
                .get(cd2_off..)
                .map(|d| glyph_class(d, right))
                .unwrap_or(0) as usize;
            // The JS indexes `classRecords[class1][class2]` directly and
            // returns whatever it finds (zero included), so this always ends
            // the walk once the left glyph is covered.
            if c1 >= class1_count || c2 >= class2_count {
                return PairPosResult::Value(0);
            }
            let rec_size = value_size(v1_format) + value_size(v2_format);
            let at = 16 + (c1 * class2_count + c2) * rec_size;
            if at + rec_size > sub.len() {
                return PairPosResult::Value(0);
            }
            PairPosResult::Value(x_advance(sub, at, v1_format))
        }
        // Unknown posFormat: the JS switch has no default, so it falls out of
        // the switch and continues with the next subtable.
        _ => PairPosResult::NoPair,
    }
}

fn value_size(format: u16) -> usize {
    2 * (format & 0xFF).count_ones() as usize
}

/// The `xAdvance` field of a ValueRecord, or 0 when the format omits it —
/// `pair.value1 && pair.value1.xAdvance || 0`.
fn x_advance(d: &[u8], at: usize, format: u16) -> i16 {
    if format & 0x0004 == 0 {
        return 0;
    }
    // Fields are emitted in bit order; xAdvance is bit 2.
    let before = (format & 0x0003).count_ones() as usize;
    let off = at + before * 2;
    if off + 2 > d.len() {
        return 0;
    }
    read_i16(d, off)
}

/// `Layout.getCoverageIndex`, returning `None` for -1.
fn coverage_index(cov: &[u8], glyph: GlyphId) -> Option<u16> {
    if cov.len() < 4 {
        return None;
    }
    match read_u16(cov, 0) {
        1 => {
            let count = read_u16(cov, 2) as usize;
            // Binary search over a sorted glyph array, as `binSearch` does.
            let (mut lo, mut hi) = (0usize, count);
            while lo < hi {
                let mid = (lo + hi) / 2;
                let at = 4 + mid * 2;
                if at + 2 > cov.len() {
                    return None;
                }
                let g = read_u16(cov, at);
                if g == glyph.0 {
                    return Some(mid as u16);
                } else if g < glyph.0 {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            None
        }
        2 => {
            let count = read_u16(cov, 2) as usize;
            for i in 0..count {
                let at = 4 + i * 6;
                if at + 6 > cov.len() {
                    return None;
                }
                let start = read_u16(cov, at);
                let end = read_u16(cov, at + 2);
                if glyph.0 >= start && glyph.0 <= end {
                    return Some(read_u16(cov, at + 4) + glyph.0 - start);
                }
            }
            None
        }
        _ => None,
    }
}

/// `Layout.getGlyphClass` — 0 when the glyph is unlisted.
fn glyph_class(d: &[u8], glyph: GlyphId) -> u16 {
    if d.len() < 4 {
        return 0;
    }
    match read_u16(d, 0) {
        1 => {
            let start = read_u16(d, 2);
            let count = read_u16(d, 4) as usize;
            if glyph.0 >= start && (glyph.0 as usize) < start as usize + count {
                let at = 6 + (glyph.0 - start) as usize * 2;
                if at + 2 <= d.len() {
                    return read_u16(d, at);
                }
            }
            0
        }
        2 => {
            let count = read_u16(d, 2) as usize;
            for i in 0..count {
                let at = 4 + i * 6;
                if at + 6 > d.len() {
                    return 0;
                }
                if glyph.0 >= read_u16(d, at) && glyph.0 <= read_u16(d, at + 2) {
                    return read_u16(d, at + 4);
                }
            }
            0
        }
        _ => 0,
    }
}

/// A per-`char` memo over `glyph_for_char`/`advance_width`/`bounding_box`,
/// standing in for the JS `_glyphCache`. Kept separate from `VectorFont` so the
/// font itself stays `Sync` and borrow-free.
#[derive(Default)]
pub struct GlyphCache {
    advances: HashMap<char, f64>,
    bboxes: HashMap<char, Option<BBox>>,
}

impl GlyphCache {
    pub fn advance_width(&mut self, font: &VectorFont, ch: char) -> f64 {
        *self
            .advances
            .entry(ch)
            .or_insert_with(|| font.advance_width(ch))
    }

    pub fn bounding_box(&mut self, font: &VectorFont, ch: char) -> Option<BBox> {
        *self
            .bboxes
            .entry(ch)
            .or_insert_with(|| font.bounding_box(ch))
    }
}
