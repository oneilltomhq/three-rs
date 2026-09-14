//! `sdf-text` — a Rust port of lib3's `src/sdf-text/` module (signed-distance-
//! field text rendered from real font outlines) plus its `src/sdf/edt.js`
//! dependency.
//!
//! The port is graded against golden data dumped from the JavaScript; see
//! `tests/golden/README.md` for the dump commands and `README.md` for the
//! deviation and skip registers.
//!
//! Ladder position: steps 1–3 of the plan (font metrics and outlines, the
//! raster + EDT + atlas, and layout) need no GPU and are graded against golden
//! data. [`batched_text::BatchedText`] is steps 4–5: it depends on `three-rs`
//! for the scene graph, the node material and the `R32Float` atlas texture, so
//! `cargo test -p sdf-text` covers it only where the packing can be checked on
//! the CPU; the pixel gates live in `three-rs`' own `tests/`.
//!
//! # Member transforms
//!
//! A member's whole `matrix_world` reaches the GPU as its glyphs' instance
//! matrix, so rotation and scale work as well as position — a label laid flat
//! on a floor with `set_rotation( -PI / 2, 0, 0 )` on
//! [`BatchedText::member_node`] renders flat. Members are not billboarded and
//! are not position-only.
//!
//! # Frustum culling
//!
//! [`BatchedText::sync`] computes the batch's own bounding sphere over every
//! glyph quad of every member, in the batch node's space, and stores it on the
//! node — the way `BatchedMesh.computeBoundingSphere` does in three.js. The
//! frustum cull reads that sphere in preference to the geometry's, so a batch
//! node left at the origin with its members placed far away is culled on where
//! its glyphs actually are, not on where its node is. Members added or moved
//! after a `sync()` are only bounded by the next `sync()` (or by
//! [`BatchedText::set_matrix_at`], which updates the sphere as it goes), which
//! is the same contract as the attribute packing.

pub mod batched_text;
pub mod edt;
pub mod error;
pub mod raster;
pub mod text;
pub mod text_builder;
pub mod vector_font;
pub mod vector_font_atlas;

pub use batched_text::{BatchedText, BatchedTextOptions, GLYPH_QUAD_PAD};
pub use edt::{compute_sdf, compute_sdf_default, edt_1d, edt_2d};
pub use error::Error;
pub use text::{OpacitySink, Text};
pub use text_builder::{
    empty_render_info, layout_text, layout_text_vector, Anchor, LaidOutGlyph, LayoutParams,
    LineHeight, TextAlign, TextRenderInfo,
};
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
