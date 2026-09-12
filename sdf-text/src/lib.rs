//! `sdf-text` — a Rust port of lib3's `src/sdf-text/` module (signed-distance-
//! field text rendered from real font outlines) plus its `src/sdf/edt.js`
//! dependency.
//!
//! The port is graded against golden data dumped from the JavaScript; see
//! `tests/golden/README.md` for the dump commands and `README.md` for the
//! deviation and skip registers.
//!
//! Ladder position: steps 1–3 of the plan (font metrics and outlines, the
//! raster + EDT + atlas, and layout) are implemented here. Steps 4–6 are
//! renderer work and `BatchedText`, which need the GPU and are not in this
//! crate yet — `text::Text` carries the layout surface they will plug into.

pub mod edt;
pub mod raster;
pub mod vector_font;
pub mod vector_font_atlas;

pub use edt::{compute_sdf, compute_sdf_default, edt_1d, edt_2d};
pub use vector_font::{BBox, PathCommand, VectorFont};
pub use vector_font_atlas::{GlyphMetrics, VectorFontAtlas};

/// The constant set used by lib3's *test* (`src/sdf/index.js`), which matches
/// neither `FontAtlas` (128/64/8/16) nor `VectorFontAtlas` (256/64/—/32). The
/// ported test layers 2–4 use these, so they are kept as their own namespace
/// rather than being folded into either atlas.
pub mod sdf_defaults {
    pub const GLYPH_SIZE: u32 = 64;
    pub const SDF_SIZE: u32 = 32;
    pub const SDF_PADDING: u32 = 4;
    pub const MAX_DISTANCE: f64 = 8.0;
    pub const ALPHA_THRESHOLD: u8 = 128;
}
