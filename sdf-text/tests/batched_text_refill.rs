//! #82's gate: a `BatchedText` that has been rendered once must draw its *new*
//! glyphs after `remove_text` + `add_text` + `sync()`.
//!
//! The failure this pins down is silent. `sync()` ends in `build_material()`,
//! which builds fresh `instanced_data_attribute` nodes over fresh copies of
//! `aGlyphUV`, `aGlyphBounds`, `aColor` and `aOpacity` and hangs them on the
//! material's `position_node` and `color_node`. The renderer keys the built
//! program on `(material.id, material.version)` plus the instance count, and
//! re-uploads the instanced attributes from the arrays the *cached* program
//! closed over. Without a `set_needs_update()` after the rebuild, a refill
//! whose glyph count matches the previous fill hits the cache and draws the
//! old letters at the new positions — every label on the page garbled, no
//! warning.
//!
//! A render-twice test does not catch this; it needs a refill between the two
//! frames. So: render "AAAAAA" from a fresh batch, refill the same batch with
//! "BBBBBB" and render again, and compare that frame against "BBBBBB" from a
//! batch that was never filled with anything else.

use std::rc::Rc;

use sdf_text::{Anchor, BatchedText, BatchedTextOptions, Text, VectorFont};
use three_rs::{Color, OrthographicCamera, Renderer, RendererParameters, Scene};

const WIDTH: f64 = 256.0;
const HEIGHT: f64 = 64.0;

fn roboto() -> VectorFont {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/Roboto-Regular.ttf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    VectorFont::parse(bytes, "tests/assets/Roboto-Regular.ttf").unwrap()
}

fn empty_batch() -> BatchedText {
    let mut batch = BatchedText::new(
        8,
        256,
        BatchedTextOptions {
            outline_width: 0.0,
            outline_color: None,
            atlas_size: 256,
        },
    );
    batch.set_font(Rc::new(roboto()));
    batch
}

/// Empty the batch and give it one member reading `s`, at the same place every
/// time — the same glyph count either way, so the refill lands on the cached
/// program's key.
fn fill(batch: &mut BatchedText, s: &str) {
    for id in (0..batch.member_count()).rev() {
        batch.remove_text(id);
    }
    let mut text = Text::new();
    text.set_text(s);
    text.set_font_size(40.0);
    text.set_anchor_x(Anchor::named("left"));
    text.set_anchor_y(Anchor::named("middle"));
    let id = batch.add_text(text) as usize;
    batch.set_color_at(id, Color::new(1.0, 1.0, 1.0));
    batch
        .member_node(id)
        .expect("the member was just added")
        .borrow_mut()
        .position
        .set(4.0, 0.0, 0.0);
    batch.sync();
}

fn frame(renderer: &mut Renderer, camera: &mut OrthographicCamera, batch: &BatchedText) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(batch.node());
    batch.node().update_matrix_world(true);
    renderer.render(&mut scene, camera);
    renderer.read_canvas_pixels().unwrap().2
}

/// White glyphs on black: any non-black pixel is ink.
fn ink(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 64)
        .count()
}

#[test]
fn a_refilled_batch_draws_its_new_glyphs() {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH, HEIGHT);
    let mut camera = OrthographicCamera::new(0.0, WIDTH, HEIGHT / 2.0, -HEIGHT / 2.0, -1.0, 1.0);
    camera.update_projection_matrix();

    let mut batch = empty_batch();
    fill(&mut batch, "AAAAAA");
    let first = frame(&mut renderer, &mut camera, &batch);
    fill(&mut batch, "BBBBBB");
    let refilled = frame(&mut renderer, &mut camera, &batch);

    let mut fresh = empty_batch();
    fill(&mut fresh, "BBBBBB");
    let expected = frame(&mut renderer, &mut camera, &fresh);

    // The premise: the two strings look different, and both drew something.
    assert!(
        ink(&first) > 200,
        "\"AAAAAA\" drew nothing: {} ink",
        ink(&first)
    );
    assert!(
        ink(&expected) > 200,
        "\"BBBBBB\" drew nothing: {} ink",
        ink(&expected)
    );
    assert_ne!(first, expected, "the two fills should not look the same");

    assert!(
        refilled != first,
        "the refilled batch still draws its first fill ({} ink)",
        ink(&refilled)
    );
    assert_eq!(
        refilled,
        expected,
        "the refilled batch ({} ink) differs from a fresh \"BBBBBB\" ({} ink)",
        ink(&refilled),
        ink(&expected)
    );
}
