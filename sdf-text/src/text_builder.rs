//! Synchronous text layout — a port of lib3's `src/sdf-text/TextBuilder.js`.
//!
//! Two layout functions, sharing one result type:
//!
//! - [`layout_text_vector`] is the real one: `unitsPerEm` scaling, advances from
//!   `hmtx`, kerning from GPOS, ink boxes from the outline, and
//!   ascender/descender from OS/2 or `hhea`. Everything it needs is in
//!   [`crate::vector_font`], so it ports across exactly and is graded
//!   bit-for-bit (after the `f32` truncation the JS does) against
//!   `tests/golden/layout_vector.json`.
//! - [`layout_text`] is the canvas-metrics one. Its measuring context is an
//!   `OffscreenCanvas`, which does not exist here — and, importantly, does not
//!   exist in node either, where `getMeasureCtx` returns `null` and the whole
//!   function falls into a hardcoded `0.6 em` advance with a `+0.8 / -0.2 em`
//!   box. **That null branch is what this port implements**, and it is what
//!   `tests/golden/layout_canvas_fallback.json` was dumped from, so the grading
//!   is honest: the same inputs give the same numbers as the JS does in the same
//!   environment. The browser branch is in the README's skip register — it needs
//!   a text-shaping stack, it is not what the SDF atlas path uses, and faking it
//!   would be worse than not having it.
//!
//! # Quirks that are ports, not bugs
//!
//! Every one of these is load-bearing for matching the golden, and four of them
//! are called out in the plan's §5.4:
//!
//! - **Unknown anchor keywords resolve to 0.** `resolveAnchor` has no `else`, so
//!   `anchorX: 'start'` and `'end'` — which is what d33 passes — fall through
//!   every branch and return `0`, i.e. left-aligned. [`Anchor::Named`] keeps
//!   that: any string that is not one of the six recognised keywords is a no-op
//!   offset. The `start-anchored` case in the golden exists to pin it.
//! - **`letter_spacing` is applied asymmetrically.** The measure pass
//!   ([`vec_measure_run`]) adds it only *between* glyphs, `len - 1` times; the
//!   pen loop adds it *after* every glyph, `len` times. So a line's measured
//!   width is one `letter_spacing` shorter than the pen actually travels, which
//!   shifts `text_align: center` by half a spacing and `right` by a whole one.
//! - **Lines march in −Y.** `baseline_y -= line_advance` each line while the
//!   boxes are Y-up, so the first line is the topmost and later lines have
//!   negative Y. Combined with the anchor being resolved from the *block*
//!   extent, this is why `anchor_y: 'top'` produces an all-negative-Y block.
//! - **Kerning is applied to the pen before the glyph it belongs to**, not after
//!   the previous one, so it tucks the current glyph toward its predecessor and
//!   is *not* included in the glyph's own advance. Net horizontal travel is the
//!   same; the per-glyph boxes are not.
//! - **`line_width` is zero in the fallback `layout_text`.** The JS guards the
//!   measure with `ctx ? … : 0`, but not the `align_offset` that consumes it, so
//!   `text_align: center` with the default `max_width: Infinity` yields an
//!   infinite offset and infinite glyph bounds. Reproduced, not clamped.
//! - **`f32` is the only truncation point**, and only for `glyph_bounds`.
//!   `block_bounds`, `visible_bounds`, `line_height`, `ascender` and `descender`
//!   stay `f64`, which is why the golden has `1.7999999999999998` in
//!   `blockBounds` next to `1.7999999523162842` in `glyphBounds` for the same
//!   edge.

use crate::vector_font::VectorFont;

/// Canvas metrics below ~6 px are unreliable, so the JS measures at this size
/// and scales. It survives into the fallback branch as the source of the
/// `0.6 em` advance (`MEASURE_FONT_PX * 0.6 * scale`).
pub const MEASURE_FONT_PX: f64 = 64.0;

/// `anchorX` / `anchorY`. The JS takes `number | string`; a number is negated
/// directly and a string is matched against the keyword list.
#[derive(Clone, Debug, PartialEq)]
pub enum Anchor {
    /// `typeof anchor === 'number'` → `-anchor`.
    Offset(f64),
    /// Anything else. Only `left`/`center`/`right` (x) and
    /// `top`/`middle`/`center`/`bottom` (y) do anything; every other string,
    /// `'start'` and `'end'` included, resolves to `0`.
    Named(String),
}

impl Anchor {
    pub fn named(s: &str) -> Self {
        Anchor::Named(s.to_string())
    }

    /// `resolveAnchor(anchor, extent, axis)`. `axis_x` picks the keyword set.
    fn resolve(&self, extent: f64, axis_x: bool) -> f64 {
        match self {
            Anchor::Offset(n) => -n,
            Anchor::Named(name) => {
                let s = name.as_str();
                if axis_x {
                    match s {
                        "left" => 0.0,
                        "center" => -extent * 0.5,
                        "right" => -extent,
                        _ => 0.0,
                    }
                } else {
                    match s {
                        "top" => -extent,
                        "middle" | "center" => -extent * 0.5,
                        "bottom" => 0.0,
                        _ => 0.0,
                    }
                }
            }
        }
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Anchor::Offset(0.0)
    }
}

/// `lineHeight: 'normal' | number | string`.
///
/// The JS runs `parseFloat` over whatever it is given and falls back to the
/// natural height when the result is not finite, so a string like `"1.5"` and
/// the number `1.5` are the same input. [`LineHeight::parse`] does that
/// conversion, which leaves only the two outcomes the JS can actually reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    /// `'normal'`, `null`, or anything `parseFloat` cannot read.
    Normal,
    /// A multiple of `font_size`.
    Factor(f64),
}

impl LineHeight {
    /// `parseFloat(lineHeight)` plus the `Number.isFinite` guard.
    pub fn parse(s: &str) -> Self {
        if s == "normal" {
            return LineHeight::Normal;
        }
        match parse_float(s) {
            Some(n) if n.is_finite() => LineHeight::Factor(n),
            _ => LineHeight::Normal,
        }
    }

    /// `parseLineHeight(lineHeight, fontSize)` — the canvas path's version,
    /// where "natural" is a flat `1.2 em`.
    fn resolve_canvas(&self, font_size: f64) -> f64 {
        match self {
            LineHeight::Normal => font_size * 1.2,
            LineHeight::Factor(n) => n * font_size,
        }
    }
}

/// JS `parseFloat`: read a leading decimal literal, ignore the rest, `NaN` if
/// there is nothing to read.
fn parse_float(s: &str) -> Option<f64> {
    let t = s.trim_start();
    let bytes = t.as_bytes();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    while end < bytes.len() {
        let c = bytes[end];
        match c {
            b'+' | b'-' if end == 0 => {}
            b'+' | b'-' if seen_exp && matches!(bytes[end - 1], b'e' | b'E') => {}
            b'0'..=b'9' => seen_digit = true,
            b'.' if !seen_dot && !seen_exp => seen_dot = true,
            b'e' | b'E' if seen_digit && !seen_exp => seen_exp = true,
            _ => break,
        }
        end += 1;
    }
    if !seen_digit {
        return None;
    }
    // Back off a trailing exponent marker with no digits after it.
    let mut slice = &t[..end];
    while !slice.is_empty()
        && matches!(slice.as_bytes()[slice.len() - 1], b'e' | b'E' | b'+' | b'-')
    {
        slice = &slice[..slice.len() - 1];
    }
    slice.parse::<f64>().ok()
}

/// `textAlign`. The JS tests for `'center'` and `'right'` and does nothing
/// otherwise, so every other value — `'left'`, `'justify'`, a typo — is
/// [`TextAlign::Left`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// The layout parameter block. [`Default`] is lib3's `LAYOUT_DEFAULTS`
/// (`Text.js:8-20`) verbatim, including `max_width: Infinity`.
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutParams {
    pub text: String,
    pub font_size: f64,
    /// Canvas path only; the fallback branch never reads it.
    pub font_family: String,
    /// Canvas path only.
    pub font_weight: String,
    /// Canvas path only.
    pub font_style: String,
    pub letter_spacing: f64,
    pub line_height: LineHeight,
    pub anchor_x: Anchor,
    pub anchor_y: Anchor,
    pub text_align: TextAlign,
    pub max_width: f64,
}

impl Default for LayoutParams {
    fn default() -> Self {
        LayoutParams {
            text: String::new(),
            font_size: 1.0,
            font_family: "monospace".to_string(),
            font_weight: "normal".to_string(),
            font_style: "normal".to_string(),
            letter_spacing: 0.0,
            line_height: LineHeight::Normal,
            anchor_x: Anchor::Offset(0.0),
            anchor_y: Anchor::Offset(0.0),
            text_align: TextAlign::Left,
            max_width: f64::INFINITY,
        }
    }
}

/// One laid-out character and its Y-up box, before the anchor offset is folded
/// in. The JS keeps this list alongside the `Float32Array` so consumers can see
/// which character each quad belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutGlyph {
    pub ch: char,
    /// `[min_x, min_y, max_x, max_y]`, `f64`.
    pub bounds: [f64; 4],
}

/// `TextRenderInfo` — what `BatchedText` consumes.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRenderInfo {
    pub parameters: LayoutParams,
    /// `Float32Array(glyphCount * 4)`, anchored. The one place the JS truncates.
    pub glyph_bounds: Vec<f32>,
    pub glyphs: Vec<LaidOutGlyph>,
    pub glyph_count: usize,
    /// `[min_x, min_y, max_x, max_y]`, anchored, `f64`.
    pub block_bounds: [f64; 4],
    /// The JS sets this to the same four numbers as `block_bounds`.
    pub visible_bounds: [f64; 4],
    pub line_height: f64,
    pub ascender: f64,
    pub descender: f64,
}

/// `emptyRenderInfo(params)` — the layout a `Text` carries while its vector font
/// is still loading.
pub fn empty_render_info(params: LayoutParams) -> TextRenderInfo {
    TextRenderInfo {
        parameters: params,
        glyph_bounds: Vec::new(),
        glyphs: Vec::new(),
        glyph_count: 0,
        block_bounds: [0.0; 4],
        visible_bounds: [0.0; 4],
        line_height: 0.0,
        ascender: 0.0,
        descender: 0.0,
    }
}

// ── Shared line handling ───────────────────────────────────────────────────

/// `text.split("\n")`, which keeps empty leading and trailing fields.
fn split_lines(text: &str) -> Vec<String> {
    text.split('\n').map(|s| s.to_string()).collect()
}

/// `paragraph.split(/(\s+)/)` — a *capturing* split, so the whitespace runs stay
/// in the list as their own entries, alternating word, gap, word, gap, …
/// starting with a word that may be empty.
fn split_words_keeping_gaps(paragraph: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_space = false;
    for ch in paragraph.chars() {
        let is_space = is_js_whitespace(ch);
        if is_space != in_space {
            out.push(std::mem::take(&mut cur));
            in_space = is_space;
        }
        cur.push(ch);
    }
    out.push(cur);
    out
}

/// The character class JS's `\s` and `trimStart` use.
fn is_js_whitespace(ch: char) -> bool {
    matches!(
        ch,
        '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn trim_start_js(s: &str) -> &str {
    s.trim_start_matches(is_js_whitespace)
}

/// The shared shape of `breakLines` / `vecBreakLines`: identical control flow,
/// different measure function. Both drop `lineWidth` bookkeeping on the floor
/// (the JS assigns it and never reads it), so it is not modelled.
fn break_lines(text: &str, max_width: f64, measure: &mut dyn FnMut(&str) -> f64) -> Vec<String> {
    if !max_width.is_finite() || max_width <= 0.0 {
        return split_lines(text);
    }

    let mut lines: Vec<String> = Vec::new();
    for paragraph in split_lines(text) {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in split_words_keeping_gaps(&paragraph) {
            let test = format!("{line}{word}");
            let w = measure(&test);
            if !line.is_empty() && w > max_width {
                lines.push(std::mem::take(&mut line));
                line = trim_start_js(&word).to_string();
            } else {
                line = test;
            }
        }
        // `paragraph === ""` cannot be true here — the `continue` above took it
        // — so this is just "push a non-empty remainder". Kept in the JS's form.
        if !line.is_empty() || paragraph.is_empty() {
            lines.push(line);
        }
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// The tail both layout functions share: anchor resolution, the `f32` copy, and
/// the result assembly.
fn finish(
    params: LayoutParams,
    glyphs: Vec<LaidOutGlyph>,
    mut block: [f64; 4],
    line_height: f64,
    ascender: f64,
    descender: f64,
) -> TextRenderInfo {
    if glyphs.is_empty() {
        block = [0.0; 4];
    }
    let [block_min_x, block_min_y, block_max_x, block_max_y] = block;
    let block_width = block_max_x - block_min_x;
    let block_height = block_max_y - block_min_y;
    let offset_x = params.anchor_x.resolve(block_width, true) - block_min_x;
    let offset_y = params.anchor_y.resolve(block_height, false) - block_min_y;

    let mut glyph_bounds = Vec::with_capacity(glyphs.len() * 4);
    for g in &glyphs {
        glyph_bounds.push((g.bounds[0] + offset_x) as f32);
        glyph_bounds.push((g.bounds[1] + offset_y) as f32);
        glyph_bounds.push((g.bounds[2] + offset_x) as f32);
        glyph_bounds.push((g.bounds[3] + offset_y) as f32);
    }

    let anchored = [
        block_min_x + offset_x,
        block_min_y + offset_y,
        block_max_x + offset_x,
        block_max_y + offset_y,
    ];

    TextRenderInfo {
        parameters: params,
        glyph_bounds,
        glyph_count: glyphs.len(),
        glyphs,
        block_bounds: anchored,
        visible_bounds: anchored,
        line_height,
        ascender,
        descender,
    }
}

// ── The canvas path, in its no-canvas branch ───────────────────────────────

/// `measureRunWidth(null, str, …)` — `str.length * 0.6 * scale`.
///
/// `length` is UTF-16 code units in JS; this counts `char`s. They differ only
/// for astral-plane text, which the JS would also mis-handle (it would measure
/// and lay out each surrogate half separately). Recorded in the README.
fn measure_run_width_no_ctx(s: &str, scale: f64) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    s.chars().count() as f64 * 0.6 * scale
}

/// `layoutText(params)` in the environment where `OffscreenCanvas` is absent.
///
/// Every glyph gets a `0.6 em` advance and a `−0.2 em … +0.8 em` box, spaces
/// included (the `ch !== ' '` test is never reached, because `metrics` is
/// undefined and the `metrics && …` guard short-circuits first). `line_width`
/// is forced to `0`, which is what makes the centred and right-aligned cases
/// infinite at the default `max_width`.
pub fn layout_text(params: LayoutParams) -> TextRenderInfo {
    let font_size = params.font_size;
    let scale = font_size / MEASURE_FONT_PX;
    let line_advance = params.line_height.resolve_canvas(font_size);
    let lines = break_lines(&params.text, params.max_width, &mut |s| {
        measure_run_width_no_ctx(s, scale)
    });

    let mut glyphs: Vec<LaidOutGlyph> = Vec::new();
    let mut block = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut baseline_y = 0.0f64;

    for line in &lines {
        // `ctx ? measureRunWidth(…) : 0` — the measure is skipped, the consumer
        // of it is not.
        let line_width = 0.0f64;
        let align_offset = match params.text_align {
            TextAlign::Center => (params.max_width - line_width) * 0.5,
            TextAlign::Right => params.max_width - line_width,
            TextAlign::Left => 0.0,
        };
        let mut pen_x = align_offset;

        for ch in line.chars() {
            let char_width = MEASURE_FONT_PX * 0.6 * scale;
            let advance = char_width + params.letter_spacing;

            let min_x = pen_x;
            let max_x = pen_x + char_width;
            let min_y = baseline_y - font_size * 0.2;
            let max_y = baseline_y + font_size * 0.8;

            glyphs.push(LaidOutGlyph {
                ch,
                bounds: [min_x, min_y, max_x, max_y],
            });
            block[0] = block[0].min(min_x);
            block[1] = block[1].min(min_y);
            block[2] = block[2].max(max_x);
            block[3] = block[3].max(max_y);

            pen_x += advance;
        }

        baseline_y -= line_advance;
    }

    finish(
        params,
        glyphs,
        block,
        line_advance,
        font_size * 0.8,
        -font_size * 0.2,
    )
}

// ── The vector path ────────────────────────────────────────────────────────

/// `vecLineAdvance` — the natural height is the font's own
/// `ascender - descender + lineGap`, not a flat `1.2 em`.
fn vec_line_advance(line_height: LineHeight, font: &VectorFont, font_size: f64) -> f64 {
    let scale = font_size / font.units_per_em;
    let natural = (font.ascender - font.descender + font.line_gap) * scale;
    match line_height {
        LineHeight::Normal => natural,
        LineHeight::Factor(n) if n.is_finite() => n * font_size,
        LineHeight::Factor(_) => natural,
    }
}

/// `vecMeasureRun` — advance plus kerning plus spacing, with the spacing (and
/// the kerning) added only *between* glyphs. The pen loop in
/// [`layout_text_vector`] adds the spacing after *every* glyph instead; that
/// asymmetry is the JS's and is preserved.
pub fn vec_measure_run(font: &VectorFont, s: &str, letter_spacing: f64, scale: f64) -> f64 {
    let chars: Vec<char> = s.chars().collect();
    let mut w = 0.0;
    for i in 0..chars.len() {
        w += font.advance_width(chars[i]) * scale;
        if i + 1 < chars.len() {
            w += font.kerning(chars[i], chars[i + 1]) * scale + letter_spacing;
        }
    }
    w
}

/// `layoutTextVector({ font, …params })`.
///
/// `font` is a separate argument here rather than a field of [`LayoutParams`],
/// because the JS only puts it in `params` to spread it into one call — and then
/// it lands in the returned `parameters`, which nothing reads. Returns
/// [`empty_render_info`] when there is no font, exactly as `if (!font)` does.
pub fn layout_text_vector(params: LayoutParams, font: Option<&VectorFont>) -> TextRenderInfo {
    let Some(font) = font else {
        return empty_render_info(params);
    };

    let font_size = params.font_size;
    let scale = font_size / font.units_per_em;
    let ascender = font.ascender * scale;
    let descender = font.descender * scale;
    let line_advance = vec_line_advance(params.line_height, font, font_size);
    let letter_spacing = params.letter_spacing;
    let lines = break_lines(&params.text, params.max_width, &mut |s| {
        vec_measure_run(font, s, letter_spacing, scale)
    });

    let mut glyphs: Vec<LaidOutGlyph> = Vec::new();
    let mut block = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut baseline_y = 0.0f64;

    for line in &lines {
        let line_width = vec_measure_run(font, line, letter_spacing, scale);
        let align_offset = match params.text_align {
            TextAlign::Center => (params.max_width - line_width) * 0.5,
            TextAlign::Right => params.max_width - line_width,
            TextAlign::Left => 0.0,
        };
        let mut pen_x = align_offset;
        let chars: Vec<char> = line.chars().collect();

        for i in 0..chars.len() {
            let ch = chars[i];

            // Kerning tucks this glyph toward the previous one: it moves the pen
            // *before* the box is computed and is not part of this glyph's
            // advance.
            if i > 0 {
                pen_x += font.kerning(chars[i - 1], ch) * scale;
            }

            let advance = font.advance_width(ch) * scale;
            // A space is forced to the no-ink branch even if the font gives it a
            // box, which Roboto does not.
            let bbox = if ch == ' ' {
                None
            } else {
                font.bounding_box(ch)
            };

            let (min_x, min_y, max_x, max_y) = match bbox {
                Some(b) => (
                    pen_x + b.x1 * scale,
                    baseline_y + b.y1 * scale,
                    pen_x + b.x2 * scale,
                    baseline_y + b.y2 * scale,
                ),
                None => (
                    pen_x,
                    baseline_y + descender,
                    pen_x + advance,
                    baseline_y + ascender,
                ),
            };

            glyphs.push(LaidOutGlyph {
                ch,
                bounds: [min_x, min_y, max_x, max_y],
            });
            block[0] = block[0].min(min_x);
            block[1] = block[1].min(min_y);
            block[2] = block[2].max(max_x);
            block[3] = block[3].max(max_y);

            pen_x += advance + letter_spacing;
        }

        baseline_y -= line_advance;
    }

    finish(params, glyphs, block, line_advance, ascender, descender)
}
