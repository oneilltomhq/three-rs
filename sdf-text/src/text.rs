//! The `Text` member — a port of lib3's `src/sdf-text/Text.js`.
//!
//! In the JS this is a `THREE.Object3D` subclass whose eleven layout properties
//! are installed by a loop over `LAYOUT_DEFAULTS` that generates a getter and a
//! setter for each, the setter comparing with `!==` and setting `_needsSync`.
//! There is no metaprogramming here, so the accessors are written out; the
//! behaviour is the same, and the comparison is `!=` on the same values.
//!
//! Two things the JS does that this deliberately does **not**:
//!
//! - it does not extend an `Object3D`. `sdf-text` has no `three-rs` dependency
//!   (see the README's dependency note), and nothing in steps 1–3 reads the
//!   transform. When `BatchedText` lands — step 5 — it will need
//!   `matrix_world`, and that is the point at which `Text` should become an
//!   `Object3D`. Until then, pretending to be a scene node would be a lie about
//!   what works.
//! - `color` is three numbers, not a `THREE.Color`. The JS `Color` applies
//!   `SRGBToLinear` on `set(hex)` because `ColorManagement` is on by default,
//!   and getting that conversion wrong is a uniform colour shift over every
//!   glyph (plan §5.3). That conversion belongs with the renderer step that
//!   actually consumes it, so this field is documented as *linear* and left for
//!   the caller to fill.
//!
//! # Opacity write-through
//!
//! `set opacity` is the one setter that does not dirty the layout: it returns
//! early on an unchanged value and otherwise forwards to
//! `BatchedText.setOpacityAt(memberId, value)`, which writes one entry of a
//! per-glyph instanced attribute. `BatchedText` is step 5 and is not in this
//! crate, so the write-through target is the [`OpacitySink`] trait — one method,
//! the same signature. That keeps the semantics the ported layer-6 test checks
//! (write-through happens, exactly once, with the member id; and the layout is
//! not invalidated) testable now, and makes `BatchedText` a drop-in later.

use std::cell::RefCell;
use std::rc::Rc;

use crate::text_builder::{
    empty_render_info, layout_text, layout_text_vector, Anchor, LayoutParams, LineHeight,
    TextAlign, TextRenderInfo,
};
use crate::vector_font::VectorFont;

/// lib3's `LAYOUT_DEFAULTS` object. It is [`LayoutParams::default`]; this
/// function exists so the name from the JS is findable.
pub fn layout_defaults() -> LayoutParams {
    LayoutParams::default()
}

/// What `Text::set_opacity` writes through to — `BatchedText.setOpacityAt` in
/// the JS.
pub trait OpacitySink {
    fn set_opacity_at(&mut self, member_id: usize, opacity: f64);
}

pub struct Text {
    /// Member colour, **linear**, default white. See the module doc.
    pub color: [f64; 3],

    opacity: f64,
    needs_sync: bool,
    text_render_info: Option<TextRenderInfo>,
    batched_text: Option<Rc<RefCell<dyn OpacitySink>>>,
    /// `-1` until the member joins a batch, as in the JS.
    member_id: i64,

    /// Vector-outline mode: layout uses real font metrics instead of canvas.
    /// Both this and the font are set by the owning `BatchedText`.
    vector_mode: bool,
    vector_font: Option<Rc<VectorFont>>,

    params: LayoutParams,

    /// Not in the JS. Counts the layouts actually performed, so a test can tell
    /// "`sync` returned the same object" from "`sync` recomputed an equal one" —
    /// the JS test asserts object identity, which `PartialEq` cannot express.
    layouts_performed: u64,
}

impl Default for Text {
    fn default() -> Self {
        Text::new()
    }
}

impl Text {
    /// `new Text()` — every layout property at its `LAYOUT_DEFAULTS` value, and
    /// `_needsSync = true`, so the first `sync` always lays out.
    pub fn new() -> Self {
        Text {
            color: [1.0, 1.0, 1.0],
            opacity: 1.0,
            needs_sync: true,
            text_render_info: None,
            batched_text: None,
            member_id: -1,
            vector_mode: false,
            vector_font: None,
            params: LayoutParams::default(),
            layouts_performed: 0,
        }
    }

    // ── Layout properties ──────────────────────────────────────────────────
    //
    // One pair each, all with the same shape: the setter compares first and only
    // dirties when the value actually changed.

    pub fn text(&self) -> &str {
        &self.params.text
    }

    pub fn set_text(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.params.text != value {
            self.params.text = value;
            self.needs_sync = true;
        }
    }

    pub fn font_size(&self) -> f64 {
        self.params.font_size
    }

    pub fn set_font_size(&mut self, value: f64) {
        if self.params.font_size != value {
            self.params.font_size = value;
            self.needs_sync = true;
        }
    }

    pub fn font_family(&self) -> &str {
        &self.params.font_family
    }

    pub fn set_font_family(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.params.font_family != value {
            self.params.font_family = value;
            self.needs_sync = true;
        }
    }

    pub fn font_weight(&self) -> &str {
        &self.params.font_weight
    }

    pub fn set_font_weight(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.params.font_weight != value {
            self.params.font_weight = value;
            self.needs_sync = true;
        }
    }

    pub fn font_style(&self) -> &str {
        &self.params.font_style
    }

    pub fn set_font_style(&mut self, value: impl Into<String>) {
        let value = value.into();
        if self.params.font_style != value {
            self.params.font_style = value;
            self.needs_sync = true;
        }
    }

    pub fn letter_spacing(&self) -> f64 {
        self.params.letter_spacing
    }

    pub fn set_letter_spacing(&mut self, value: f64) {
        if self.params.letter_spacing != value {
            self.params.letter_spacing = value;
            self.needs_sync = true;
        }
    }

    pub fn line_height(&self) -> LineHeight {
        self.params.line_height
    }

    pub fn set_line_height(&mut self, value: LineHeight) {
        if self.params.line_height != value {
            self.params.line_height = value;
            self.needs_sync = true;
        }
    }

    pub fn anchor_x(&self) -> &Anchor {
        &self.params.anchor_x
    }

    pub fn set_anchor_x(&mut self, value: Anchor) {
        if self.params.anchor_x != value {
            self.params.anchor_x = value;
            self.needs_sync = true;
        }
    }

    pub fn anchor_y(&self) -> &Anchor {
        &self.params.anchor_y
    }

    pub fn set_anchor_y(&mut self, value: Anchor) {
        if self.params.anchor_y != value {
            self.params.anchor_y = value;
            self.needs_sync = true;
        }
    }

    pub fn text_align(&self) -> TextAlign {
        self.params.text_align
    }

    pub fn set_text_align(&mut self, value: TextAlign) {
        if self.params.text_align != value {
            self.params.text_align = value;
            self.needs_sync = true;
        }
    }

    pub fn max_width(&self) -> f64 {
        self.params.max_width
    }

    pub fn set_max_width(&mut self, value: f64) {
        if self.params.max_width != value {
            self.params.max_width = value;
            self.needs_sync = true;
        }
    }

    // ── The rest of the surface ────────────────────────────────────────────

    pub fn text_render_info(&self) -> Option<&TextRenderInfo> {
        self.text_render_info.as_ref()
    }

    pub fn needs_sync(&self) -> bool {
        self.needs_sync
    }

    /// `text._needsSync = true` — the owning `BatchedText` sets this directly in
    /// `addText` and `resetAtlas`. Not a JS *method*: in JS `_needsSync` is just
    /// a field the batch reaches into, and Rust privacy needs a door for it.
    pub fn mark_needs_sync(&mut self) {
        self.needs_sync = true;
    }

    /// How many layouts have actually run. Not in the JS — see the field doc.
    pub fn layouts_performed(&self) -> u64 {
        self.layouts_performed
    }

    /// Whole-text opacity (0..1).
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    /// Does **not** trigger re-layout; writes through to the owning batch's
    /// per-glyph opacity attribute when batched.
    pub fn set_opacity(&mut self, value: f64) {
        if self.opacity == value {
            return;
        }
        self.opacity = value;
        if let (Some(batch), true) = (self.batched_text.as_ref(), self.member_id >= 0) {
            batch
                .borrow_mut()
                .set_opacity_at(self.member_id as usize, value);
        }
    }

    /// `_batchedText` / `_memberId`, set by the owning batch. Both are needed
    /// before `set_opacity` writes anything through.
    pub fn attach_to_batch(&mut self, batch: Rc<RefCell<dyn OpacitySink>>, member_id: usize) {
        self.batched_text = Some(batch);
        self.member_id = member_id as i64;
    }

    pub fn detach_from_batch(&mut self) {
        self.batched_text = None;
        self.member_id = -1;
    }

    pub fn member_id(&self) -> i64 {
        self.member_id
    }

    /// `_vectorMode` / `_vectorFont`. Passing `None` for the font while the mode
    /// is on is the "still loading" state `sync` has a branch for.
    pub fn set_vector_mode(&mut self, on: bool) {
        self.vector_mode = on;
    }

    pub fn vector_mode(&self) -> bool {
        self.vector_mode
    }

    pub fn set_vector_font(&mut self, font: Option<Rc<VectorFont>>) {
        self.vector_font = font;
    }

    /// `sync(callback, renderer)` — lay out the glyphs, unless a cached layout is
    /// still valid.
    ///
    /// The callback is a JS idiom for "the layout may be asynchronous"; it never
    /// is, so it is dropped. The renderer argument was already unused in the JS.
    ///
    /// Note the vector-mode-without-a-font branch: it installs an empty layout
    /// and returns **without clearing the dirty flag**, so the next `sync` after
    /// the font arrives re-lays out. It also does not touch
    /// `layouts_performed`, because no layout happened.
    pub fn sync(&mut self) -> &TextRenderInfo {
        // The `if let` form clippy suggests here ties the returned borrow's
        // lifetime to a region that (per NLL) also covers the later
        // `self.text_render_info = Some(...)` assignment below, which the
        // borrow checker then rejects; the is_some()+unwrap() form does not.
        #[allow(clippy::unnecessary_unwrap)]
        if !self.needs_sync && self.text_render_info.is_some() {
            return self
                .text_render_info
                .as_ref()
                .expect("sdf-text: text_render_info.is_some() was just checked");
        }

        let params = self.params.clone();

        if self.vector_mode {
            match self.vector_font.clone() {
                None => {
                    // Font still loading — stay dirty so we re-layout once it
                    // arrives.
                    self.text_render_info = Some(empty_render_info(params));
                    return self
                        .text_render_info
                        .as_ref()
                        .expect("sdf-text: text_render_info was set on the line above");
                }
                Some(font) => {
                    self.text_render_info = Some(layout_text_vector(params, Some(&font)));
                }
            }
        } else {
            self.text_render_info = Some(layout_text(params));
        }

        self.layouts_performed += 1;
        self.needs_sync = false;
        self.text_render_info
            .as_ref()
            .expect("sdf-text: text_render_info was set by the branch above")
    }
}
