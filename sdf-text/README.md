# `sdf-text`

A Rust port of lib3's `src/sdf-text/` module — signed-distance-field text
rasterised from real font outlines — plus the `src/sdf/edt.js` it depends on.

This crate covers **steps 1–6 of the port ladder**. Steps 1–3 — font metrics and
outlines, the raster → EDT → atlas pipeline, and layout — are pure arithmetic and
need neither a GPU nor a network:

```
cargo test -p sdf-text      # 52 tests, no GPU, no network
```

Steps 4–6 are the renderer half: `batched_text::BatchedText` (the instanced
draw, the `R32Float` atlas texture, the `positionNode`, the outline/halo
material), with the examples in `examples/` and their gates in `tests/` —

| step | example | gate |
|---|---|---|
| 4 | — | `tests/sdf_text_glyph.rs`: one glyph quad, checked against the atlas tile's own SDF resampled on the CPU (not against an image) |
| 5 | `examples/sdf_text_block.rs`, lib3's `examples/sdf-text-vector` page | `tests/sdf_text_block.rs`: the attribute packing leaf by leaf, two-run frame identity, and the whole 800 × 500 frame against the atlas SDF |
| 6 | `examples/d33_treemap_labels.rs`, d33's `examples/d3_treemap.html` | `tests/d33_treemap_labels.rs` against `tests/golden/d3_treemap_labels.json`, dumped from that page's own JS |

Those three need a GPU, and must be run single-threaded
(`cargo test -p sdf-text -- --test-threads=1`). Step 6's image comparison is
three-rs's own e2e comparator (`three_rs::testing::compare`), run over d33's
reference frame; its treemap layout comes from the
[d3-hierarchy](https://github.com/oneilltomhq/d3-hierarchy) crate, a
dev-dependency.

## What is ported

| module | JS original | status |
|---|---|---|
| `vector_font` | `VectorFont.js` | metrics, glyph lookup, advances, ink bboxes, GPOS kerning, outline command lists |
| `edt` | `../sdf/edt.js` | `edt1d`, `edt2d`, `computeSDF` |
| `raster` | the `Path2D` + canvas `fill` inside `VectorFontAtlas.rasterizeGlyph` | an exact-area scanline filler |
| `vector_font_atlas` | `VectorFontAtlas.js` | the pixel data and per-glyph metrics (not the texture object) |
| `text_builder` | `TextBuilder.js` | `layoutTextVector` fully; `layoutText` in its no-canvas branch |
| `text` | `Text.js` | the layout property surface, the dirty flag, `sync`, opacity write-through |
| `sdf_defaults` | `../sdf/index.js` | the constant set lib3's *test* uses (64/32/4/8/128) |
| `batched_text` | `BatchedText.js` | the instanced glyph draw: attribute packing, `GLYPH_QUAD_PAD`, blank-glyph instances, insertion-order atlas slots, `setMatrixAt` / `setColorAt` / `setOpacityAt`, the outline/halo TSL chain |

## The grader

The only ground truth is `tests/golden/`, dumped from the JavaScript itself; the
exact dump commands are in `tests/golden/README.md`. **The target is what the JS
does, including where that is eccentric** — opentype.js's redundant contour
segments, its `||` metric chains that treat `0` as absent, `resolveAnchor`'s
missing `else`, the asymmetric `letterSpacing`. Nothing here is "fixed".

On top of the goldens, all six layers of `lib3/test/sdf-pipeline.test.mjs` are
ported assertion for assertion, with that file's own separate constant set
(`sdf_defaults`: `MAX_DISTANCE` 8, not the atlas's 32) and its own fixed-width
shader helper (`edgeWidth` 0.1) kept as test code, because that is what they are
in lib3.

| gate | golden | verdict |
|---|---|---|
| font metrics, 68 advances, 68 ink bboxes, 68 Y-down path bboxes, 1436 path commands, 4624 kern pairs | `font_roboto.json` | **exact** (`==`) |
| `computeSDF` on four fixtures, full `f64` fields | `edt_fixtures.json` | **exact** (`to_bits`) |
| atlas `u`/`v`/`w`/`h`, `viewBox`, slot assignment, the raster affine | `atlas_roboto.json` | **exact** |
| 64×64 `f32` tiles, 11 glyphs | `atlas_roboto.json` | **ε = 2/64**, measured; `'l'` and `' '` exact |
| 256×256 binarised coverage mask for `'a'` | `atlas_roboto.json` | **52 of 65 536 texels flip** (0.079 %) |
| `layoutTextVector`, 6 cases | `layout_vector.json` | **exact** (`to_bits`, `f32` for `glyphBounds` and `f64` for the rest) |
| `layoutText` fallback branch, 6 cases | `layout_canvas_fallback.json` | **exact** (`to_bits`) |

The two epsilons are both in the rasteriser and both have one cause, which
`src/raster.rs`'s module doc documents in full with the experiments behind it.
In short: Chromium's canvas coverage **is** exact-area — an axis-aligned probe
rectangle at fractional coordinates agrees with this filler to within
`floor(255c)` vs `round(255c)` on every texel, and the two glyphs in the golden
whose outlines are entirely axis-aligned (`'l'`, `' '`) come out bit-exact — but
its sloped and curved boundaries sit ≈0.05 px off the analytic position. No
sub-scanline count, sample phase, or x quantisation reproduces that, and a
flattening-tolerance sweep rules out curve flattening. It is almost certainly
Skia's `SkFDot6`/`SkFixed` edge quantisation, which is a property of one browser
build rather than of the JS, so it is recorded rather than chased. Both epsilons
are asserted as *bounds that must not grow*, alongside assertions that the error
stays on the 50 % contour and that at most 20 % of any tile differs at all —
the mistakes that actually matter (a winding-rule slip, a half-texel offset, a
Y-flip) all produce errors in the thousands and cannot pass.

## Deviations, and why

1. **`owned_ttf_parser` instead of opentype.js, with `glyf` and GPOS read by
   hand.** The plan (§2.2) blesses the owning wrapper. The hand-rolling is forced
   three ways: opentype's contour walk starts at each contour's *last* point and
   emits one segment per point, so it produces a trailing duplicate segment and a
   degenerate `L` at every on-curve point ending a `Q`, where ttf-parser's
   `OutlineBuilder` emits the clean form; opentype's `Path.getBoundingBox` solves
   exact curve extrema while ttf-parser returns the stored `glyf` header box; and
   ttf-parser keeps `LookupSubtables::kind` private, so the GPOS lookup-type
   filter is impossible through its API. `FaceTables` also has no `loca`, so that
   and `head`'s `indexToLocFormat` are read from `raw_face()`.
2. **`edt_1d`'s hull-pop loop is the canonical guarded form.** The JS is a
   `do … while (k >= 0)` that can read `v[-1]`, getting `undefined → NaN`, which
   makes `s > z[k]` false and pops again. In JS that is benign; in Rust it is a
   panic. The guarded form has the same result (`s` keeps the value computed
   against `v[0]`, and the following `k += 1` puts it back at slot 0) and the
   tests are bit-exact, so the behaviour is identical for every reachable input.
3. **`compute_sdf` takes the alpha plane, not an RGBA buffer.** The JS indexes
   `imageData[i * 4 + 3]` and reads nothing else.
4. **The rasteriser is an exact-area filler, not Skia.** See above;
   `src/raster.rs` carries the evidence. tiny-skia was evaluated and dropped, so
   the crate has no rasterisation dependency at all.
5. **Strings are iterated as `char`s, not UTF-16 code units.** `str.length` and
   `str[i]` in the JS are code units, so astral-plane text would be measured and
   laid out one surrogate half at a time. The port lays out whole code points.
   This differs only for non-BMP text, where the JS's behaviour is broken rather
   than merely different; the goldens are all BMP.
6. **`Text` is not an `Object3D`, and `color` is `[f64; 3]`.** The crate has no
   `three-rs` dependency, and nothing in steps 1–3 reads a transform. `Text`
   should become an `Object3D` at step 5, when `BatchedText` needs
   `matrix_world`. `color` is documented as **linear**: the JS `THREE.Color`
   applies `SRGBToLinear` on `set(hex)` because `ColorManagement` is on by
   default, and that conversion belongs with the renderer step that consumes it
   (plan §5.3 — getting it wrong is a uniform colour shift over every glyph).
7. **`opacity` writes through to an `OpacitySink` trait object**, because
   `BatchedText.setOpacityAt` is step 5. Same signature, same early-return on an
   unchanged value, same `memberId >= 0` guard.
8. **`Text::layouts_performed()` is new.** lib3's layer-6 test asserts JS object
   *identity* (`assert.equal(t.textRenderInfo, info)`) to prove `sync` did not
   recompute. `PartialEq` cannot express that, so a counter does.
9. **`lineHeight`, `anchorX`/`anchorY` and `textAlign` are enums.** The JS takes
   `string | number` and runs `parseFloat` / keyword matching. The enums carry
   exactly the states the JS can reach, and `LineHeight::parse` reproduces
   `parseFloat` semantics including `"1.5em" → 1.5` and `"abc" → normal`.
   `Anchor::Named` deliberately keeps the fall-through: any unrecognised keyword
   resolves to `0`, which is what `anchorX: 'start'` does.
10. **`sync()` has no callback and no renderer argument.** Layout is synchronous;
    the JS's `callback?.()` is an idiom for a path that never awaits, and
    `_renderer` was already unused.
11. **`VectorFontAtlas` and `layout_text_vector` take the font as an argument**
    rather than holding it as a field, which is what lets the "font has not
    loaded yet" state (`Option::None`) be a parameter instead of a mutable field.
    The JS behaviour — glyphs requested before the font arrives are blank, and
    `setFont` resets the atlas so they are regenerated — is preserved and tested.
12. **`BatchedText` owns its members.** In the JS `addText` stores the batch on
    `text._batchedText` so `Text.opacity`'s setter can write through. An `Rc`
    cycle between the batch and its members is exactly what Rust is built to
    refuse, so the batch holds each member — its `Text` and its `Object3D` — and
    hands out `member_node` / `text_at_mut` instead. The write-through sink
    (deviation 7) is `None` for a member, and `set_opacity_at` does the writing.
13. **Per-instance data travels with the node graph, not with four dirty
    flags.** The JS keeps `_needsUpdate` booleans on four
    `InstancedBufferAttribute`s and uploads whichever changed. Here the four
    arrays hang off the material's node graph as
    `tsl::instanced_data_attribute` buffers, so a `build_material()` is the one
    thing that has to happen after `sync()`. The bytes on the GPU are the same.
14. **The `Fn( … )()` wrappers around `positionNode` and `colorNode` are
    dropped.** They take no parameters, so three inlines them; the emitted WGSL
    is the same.
15. **`glyph_path` (Y-up) emits `Close`; `glyph_path_y_down` does not.**
    opentype's `getPath` emits no `Z` at all (it only closes a path when it is
    stroked), so the Y-down form that feeds the rasteriser and the golden matches
    it exactly. The Y-up form keeps `Close` because it is the form a consumer
    outside this port would want; nothing graded uses it.

## Skip register

Not ported, and why:

- **`VectorFont.load`** — URL `fetch`, the lazy `import('opentype.js')`, and WOFF
  decompression. `VectorFont::parse(Vec<u8>, src)` takes bytes; loading is the
  caller's business and the golden is a local `.ttf`.
- **The legacy `kern`-table kerning path.** opentype's `getKerningValue` uses
  GPOS whenever GPOS exists and never falls back; Roboto has GPOS, so the
  `kern`-table branch is unreachable for every graded input. A font without GPOS
  would get zero kerning here and non-zero in the JS.
- **`matchedPoints` composite glyphs.** `glyf` components placed by point
  matching rather than by offset. opentype supports it; Roboto does not use it,
  so there is no way to grade an implementation. Components placed by offset
  (including nested ones, with a recursion guard) work.
- **`layoutText`'s real canvas branch.** It needs `OffscreenCanvas`
  `measureText` — a full text-shaping stack with `actualBoundingBox*` — and it is
  not the path the SDF atlas uses. Node has no `OffscreenCanvas` either, so the
  JS itself runs the fallback branch there; that branch is ported and graded
  against a golden dumped from the JS in that same environment, which is why
  layer 5 is honest rather than vacuous.
- **`FontAtlas.js`** — the older canvas-rasterised atlas (128/64/8/16 constants).
  Superseded by `VectorFontAtlas` for every lib3 and d33 use; its constants are
  not even the ones lib3's own test uses.
- **What `BatchedText` inherits from `THREE.InstancedMesh`** beyond the draw
  itself — `raycast`, `dispose`. This port's `InstancedMesh` has neither, and
  nothing in lib3's or d33's pages picks a glyph. `computeBoundingSphere` *is*
  ported (three-rs `InstancedMesh::compute_bounding_sphere`), and `sync()`
  computes the batch's own sphere over the members' glyph quads: without it the
  batch node is culled as a point at its own origin, which is a black frame and
  no warning (#42). d33's page turns frustum culling off for its batch; it no
  longer has to.
- **`BatchedText`'s `_baseMaterial` constructor argument.** It is dead in the JS
  too — the constructor names it and never reads it, and both call sites pass
  `undefined`.
- **`options.font` and the `ready` promise.** The JS constructor takes a font
  URL and `await`s `VectorFont.load`; `VectorFont::load` is already in this
  register, so `set_font(Rc<VectorFont>)` takes the parsed font and there is
  nothing to await. `_vectorMode` is therefore always on: the canvas-raster
  branch needs `FontAtlas.js`, also in this register.
- **`removeText(text)`'s `indexOf` lookup.** Deviation 12: the batch owns its
  members, so there is no external `Text` handle to search for and
  `remove_text(member_id)` takes the id the JS would have found. The slot
  handling — clear it, and shrink `_memberCount` only when the *last* member
  went — is the JS's.
- **`index.js`'s re-export shape.** Rust modules and `pub use` in `lib.rs` cover
  it.

## The bundled font

`tests/assets/Roboto-Regular.ttf` (Roboto Regular 2.001047, Copyright 2015
Google Inc., Apache-2.0) ships inside the published crate, so the examples and
the doc snippets run as written from a `cargo add sdf-text` checkout as well as
from this repository. The notice and the licence text are in `LICENSE-Roboto`;
the crate's own code stays MIT (`LICENSE`). The golden data under
`tests/golden/` is *not* packaged — those tests only run from a repository
checkout.
