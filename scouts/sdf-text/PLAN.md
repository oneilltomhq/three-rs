# Scout — porting lib3 `sdf-text` to Rust for three-rs

Scouted 2026-09-12. Sources read:

- `/home/tom/src/projects/lib3/src/sdf-text/` @ working tree (7 files, 1499 lines)
  plus its EDT dependency `/home/tom/src/projects/lib3/src/sdf/edt.js` and
  `/home/tom/src/projects/lib3/src/sdf/index.js`.
- `/home/tom/src/projects/lib3/test/sdf-pipeline.test.mjs` (214 lines, six layers).
- `/home/tom/src/projects/d33` (bare-worktree repo; `ex-treemap`, `ex-sunburst`,
  `ex-tree-of-life` exist only as **branches** — read with
  `git --git-dir=/home/tom/src/projects/d33/.bare show <branch>:<path>`).
- three-rs: the crate lives on branch worktrees, **`src/` at the worktree root, not
  `port/src`** — `/home/tom/src/projects/three-rs/port` (fc5d3e7, rungs 0–4 green) and
  `/home/tom/src/projects/three-rs/rung5` (7a3a30f, the most advanced; the only branch
  with `dpdx`/`dpdy`). `/home/tom/src/projects/three-rs/main` holds only the docs.
- `~/src/vendor/three.js` @ 148ef33 (tag r186 — the 2026-09-13 repin, `rung0/REPIN-r186.md`)
  for NodeMaterial/InstanceNode semantics. d33's workaround comment names the old pin
  3d010ef (`r186dev`); `NodeMaterial.setupPosition`'s ordering is the same on both, so
  §5.2 holds across the repin.

**Headline finding (§5, and it changes the design):** under three r186
`NodeMaterial.setupPosition` applies `positionNode` **after** the instance transform
(`src/materials/nodes/NodeMaterial.js:798` then `:804-806`), so `positionNode`
*overwrites* `positionLocal` and the per-text instance matrix is discarded. lib3's
`BatchedText` relies on the opposite order. d33 measured this and worked around it by
baking each glyph quad into its instance matrix. The two formulations are
**geometrically identical**, so the Rust port should implement `position_node`
*correctly* (before the instance matrix) and still match d33's pixels. See §5.2.

---

## 1. Module-by-module inventory of the JS

Public surface, `/home/tom/src/projects/lib3/src/sdf-text/index.js:1-6`:
`Text`, `BatchedText`, `FontAtlas`, `VectorFont`, `VectorFontAtlas`, `layoutText`,
`layoutTextVector`, `emptyRenderInfo`.

Data flow (vector mode — the only mode d33 uses):

```
TTF bytes ─VectorFont.load─► opentype.js Font ─┬─► layoutTextVector ──► TextRenderInfo
                                               │      (advances, kerning, ink bboxes)
                                               └─► VectorFontAtlas.rasterizeGlyph
                                                      Path2D fill @256px
                                                      ─► computeSDF (EDT)
                                                      ─► point-downsample to 64×64 tile
                                                      ─► Float32 R-only atlas texture
BatchedText.sync() ── packs per-glyph instance attributes (aGlyphUV, aGlyphBounds,
                      aColor, aOpacity) + instanceMatrix ── NodeMaterial (TSL) ── draw
```

### 1.1 `VectorFont.js` (107 lines) — font metrics

- `static async load(source)` (`:46-70`): accepts URL string / `ArrayBuffer` /
  `ArrayBufferView`; `fetch` for a URL; **lazy** `await import("opentype.js")` with
  `ns.default ?? ns` UMD interop (`:66-67`); `opentype.parse(buffer)`.
  WOFF2/brotli unsupported (comment `:9-10`).
- Constructor (`:17-39`) reads metrics with this exact precedence (all `||`, so a
  **0 value falls through** — a real behavioural detail to copy):
  `ascender = os2.sTypoAscender || hhea.ascender || font.ascender`;
  `descender = os2.sTypoDescender || hhea.descender || font.descender`;
  `lineGap = os2.sTypoLineGap || hhea.lineGap || 0`;
  `capHeight = os2.sCapHeight || 0`; `xHeight = os2.sxHeight || 0`;
  `unitsPerEm = font.unitsPerEm`.
  For `examples/assets/fonts/Roboto-Regular.ttf` (the only font d33 and lib3 use):
  `unitsPerEm 2048`, `sTypoAscender 2146`, `sTypoDescender -555`, `sTypoLineGap 0`,
  `sCapHeight 1456`, `sxHeight 1082`; `hhea` is identical (2146 / −555 / 0).
- `glyphForChar(char)` (`:76-83`) — `font.charToGlyph(char)`, memoised in `_glyphCache`.
- `advanceWidth(char)` (`:86-89`) — `glyph.advanceWidth` in font units (`'A'` → 1336,
  `' '` → 507).
- `boundingBox(char)` (`:92-98`) — `glyph.getBoundingBox()`, **Y-up font units**,
  returns `null` unless `x2 > x1 && y2 > y1` (so `' '` → null: its bbox is 0,0,0,0).
- `kerning(l, r)` (`:101-106`) — `font.getKerningValue(lg, rg)`.

  **Exact opentype.js semantics to replicate** (`opentype.js@2.0.0`
  `dist/opentype.mjs:15443-15450` and `:9846-9873`): if `font.position.defaultKerningTables`
  is truthy it uses **GPOS only and never falls back to the `kern` table**;
  `defaultKerningTables` = `getLookupTables(defaultScript, undefined, 'kern', 2)` and is
  `undefined` only when there is no `gpos` table at all. The GPOS walk returns the
  **first** coverage hit: PairPos format 1 → linear scan of the pair set for
  `secondGlyph === rightIndex`, `value1.xAdvance || 0`; format 2 → `classRecords[class1][class2].value1.xAdvance || 0`
  (note format 2 *returns* on the first covered subtable even when the class pair is
  zero — no continue). Roboto: `gpos` present, `kern` absent, 1 default kerning table;
  `kerning('A','V') === -87`.

### 1.2 `TextBuilder.js` (435 lines) — layout, two flavours

`layoutText(params)` (`:103-221`) is the **Canvas-metrics** flavour:
`OffscreenCanvas(8,8)` measured at `MEASURE_FONT_PX = 64` then scaled (`:7-17`,
`:119`); per-character `ctx.measureText` using `actualBoundingBoxLeft/Right/Ascent/Descent`
(`:153-166`). In **node there is no `OffscreenCanvas`**, so `_measureCanvas` is `null`,
`getMeasureCtx` returns `null`, and the fallback widths `str.length * 0.6 * scale`
(`:77`) / `MEASURE_FONT_PX * 0.6` (`:145`) and ±0.8/0.2·fontSize ink box (`:170-171`)
are what the lib3 test actually exercises. Not needed by d33; port it only to keep
test layer 5 honest (§3).

`layoutTextVector(params)` (`:307-422`) is the flavour the port needs:

- `scale = fontSize / font.unitsPerEm`; `ascender/descender = font.ascender/descender * scale`.
- `vecLineAdvance` (`:240-246`): `lineHeight === "normal" || null` →
  `(ascender - descender + lineGap) * scale`; else `parseFloat(lineHeight) * fontSize`
  (so d33's `lineHeight = 0.9` → `0.9 * fontSize`).
- `vecMeasureRun` (`:249-258`): `Σ advance(ch)*scale`, and **between** characters
  `+ kerning(ch_i, ch_{i+1})*scale + letterSpacing` (note: letterSpacing only between,
  not after).
- `vecBreakLines` (`:260-287`): split on `\n`; greedy word wrap on `/(\s+)/` keeping the
  separators as "words"; only when `maxWidth` is finite and > 0. d33 passes
  `maxWidth = Infinity`, so this is just `text.split("\n")`.
- Pen loop (`:347-383`): kerning is applied **before** the glyph
  (`if (i > 0) penX += kerning(prev, ch) * scale`); quad = ink bbox
  `[penX + bbox.x1*scale, baselineY + bbox.y1*scale, penX + bbox.x2*scale, baselineY + bbox.y2*scale]`;
  blank/space → `[penX, baselineY+descender, penX+advance, baselineY+ascender]`;
  then `penX += advance + letterSpacing`. `baselineY -= lineAdvance` per line
  (**Y-up, lines go down**).
- Anchors, `resolveAnchor` (`:25-37`): numeric → `-anchor`;
  x: `left`→0, `center`→`-w/2`, `right`→`-w`; y: `top`→`-h`, `middle`/`center`→`-h/2`,
  `bottom`→0; **anything else → 0**. d33 sets `anchorX = 'start' | 'end'`
  (`src/labels.js:191`), which match nothing and therefore behave as `left`/0 — a live
  quirk the port must reproduce, not fix (§5.4).
- Output `TextRenderInfo` (typedef `:424-435`): `glyphBounds: Float32Array(n*4)`
  (anchored), `glyphs: [{char, bounds}]`, `glyphCount`, `blockBounds`, `visibleBounds`
  (same as blockBounds), `lineHeight`, `ascender`, `descender`.
- `emptyRenderInfo` (`:224-236`) — all-zero info used while the font loads.

Depends on: opentype.js (via `VectorFont`) only. No Three, no Canvas in the vector path.

### 1.3 `VectorFontAtlas.js` (209 lines) — the rasteriser + SDF

Constants (`:7-10`): `TILE = 64`, `RASTER = 256`, `PAD_FRAC = 0.18`,
`MAX_DISTANCE = RASTER * 0.125 = 32` (raster px). Default `atlasSize = 1024`
→ `cols = rows = 16` → 256 glyph tiles.

- Constructor (`:27-51`): `OffscreenCanvas(256,256)` 2d ctx with
  `willReadFrequently: true`; `atlasData = new Float32Array(1024*1024)`;
  `THREE.DataTexture(atlasData, 1024, 1024, THREE.RedFormat, THREE.FloatType)` with
  `min/magFilter = LinearFilter`. (DataTexture defaults: `flipY = false`,
  `generateMipmaps = false`, `wrap = ClampToEdge`, `colorSpace = NoColorSpace`.)
- `setFont(font)` (`:62-66`) → `reset()` (`:68-73`, clears map, zeroes the atlas,
  `nextSlot = 0`). `ensureGlyphs(chars)` (`:79-88`), `getGlyph(char)` (`:90-97`),
  `hasGlyph` (`:75-77`). Slot allocation is **insertion-ordered** (`nextSlot++`),
  so atlas layout depends on the iteration order of the `Set` built in
  `BatchedText.sync` (`BatchedText.js:343-355`) — insertion order over members, then
  over glyphs. Reproduce that order exactly or UVs diverge.
- `_buildPath2D(commands, offX, offY, s, minX, minY)` (`:100-131`): affine
  `mx = offX + (x - minX)*s`, `my = offY + (y - minY)*s`, mapping opentype commands
  `M/L/C/Q/Z` to `moveTo/lineTo/bezierCurveTo/quadraticCurveTo/closePath`.
- `rasterizeGlyph(char)` (`:133-208`), step by step — this is the part that must match
  the JS bit-for-bit-ish:
  1. `clearRect(0,0,256,256)`.
  2. `path = glyph.getPath(0, 0, unitsPerEm)` — **fontSize == unitsPerEm so scale is
     exactly 1** and coordinates stay integral; opentype's `getPath` emits **Y-down**
     (baseline at 0, ink at negative y). Roboto is TrueType, so command types are
     `M`, `L`, `Q` only (verified for `A`, `o`, `e`); a CFF font would add `C`.
  3. `pb = path.getBoundingBox()` — opentype computes **exact curve extrema**, not the
     `glyf` header bbox: `Path.getBoundingBox` (`dist/opentype.mjs:628`) →
     `BoundingBox.addQuad` (`:318`, elevates the quadratic to a cubic with the 2/3–1/3
     rule) → `addBezier` (`:284`, solves the derivative quadratic per axis and adds
     `derive(...)` at each root in (0,1)). **ttf-parser's `glyph_bounding_box`/
     `outline_glyph` return the stored `glyf` bbox and will not always agree — compute
     the extrema yourself with this same formula.**
  4. `avail = 256 - 2*(256*0.18) = 163.84`; `s = avail / max(gw, gh)`;
     `rw = gw*s`, `rh = gh*s`; `offX = (256-rw)/2`, `offY = (256-rh)/2`. So every glyph
     is scaled so its **larger** ink dimension is 163.84 px and centred (the SDF spread
     is therefore relative to each glyph's own size — see §5.5).
  5. `ctx.fillStyle = "white"; ctx.fill(p2d)` — **default nonzero winding**.
  6. `viewBox = [offX/256, offY/256, (offX+rw)/256, (offY+rh)/256]` (normalised, Y-down).
  7. `imgData = ctx.getImageData(0,0,256,256)`; `computeSDF(imgData.data, 256, 256)`.
  8. Downsample to 64×64 by **point sampling, no averaging**:
     `srcX = min(floor((sx+0.5)*4), 255)` → exactly raster column `4*sx + 2`.
  9. Encode: `normalized = 0.5 - dist/(2*32)`, clamped to [0,1]. 0.5 at the edge,
     >0.5 inside (dist is negative inside), <0.5 outside.
  10. Write to `atlasData[(row*64+sy)*1024 + col*64+sx]`; return
      `{u: col*64/1024, v: row*64/1024, w: 64/1024, h: 64/1024, viewBox}`.
- Blank/unmapped glyph: `viewBox` stays `[0,0,0,0]` and the tile is pure
  `0.5 - INF` → clamped 0.

### 1.4 The EDT — `/home/tom/src/projects/lib3/src/sdf/edt.js` (88 lines)

**Felzenszwalb & Huttenlocher** exact Euclidean distance transform (named in the header
comment `:1`), the standard lower-envelope-of-parabolas 1-D pass run over columns then
rows. Not 8SSEDT, not a Chebyshev approximation — it is exact.

- `INF = 1e20` (`:4`).
- `edt1d(f, d, v, z, n)` (`:6-32`): intersection
  `s = (f[q] - f[r] + q*q - r*r) / (2*q - 2*r)`; hull pop while `s <= z[k]`;
  second pass `d[q] = (q - v[k])^2 + f[v[k]]`.
  (Note the `do…while(k >= 0)` loop reads `v[k]` with `k` possibly −1 after a pop —
  harmless in JS because `v[-1]` is `undefined` → the expression is `NaN` → `NaN > z[k]`
  is false → `k--` again… in practice `k` never goes below 0 for these inputs. A Rust
  port must use the canonical `while k >= 0 && s <= z[k] { k -= 1 }` form.)
- `computeSDF(imageData, width, height, alphaThreshold = 128)` (`:38-67`):
  reads **only the alpha channel** (`imageData[i*4+3]`); `a >= 128` → inside;
  two `Float64Array` grids (`outside`, `inside`) seeded 0/INF; `edt2d` on both;
  result `sqrt(outside[i]) - sqrt(inside[i])` in **`Float64Array`**, negative inside.
  Distances are in raster pixels.
- `edt2d(grid, w, h)` (`:69-88`): columns first (`f[y] = grid[y*w+x]`), then rows.
  Scratch arrays sized `maxDim` / `maxDim+1`.

Because the image is **binarised at alpha ≥ 128**, the rasteriser's anti-aliasing only
matters where coverage is near 50% — see §5.1.

### 1.5 `FontAtlas.js` (159 lines) — the canvas-font atlas (not used by d33)

Same shape as `VectorFontAtlas` but rasterises with `ctx.fillText` at
`GLYPH_SIZE = 128`, `SDF_SIZE = 64`, `SDF_PADDING = 8`, `MAX_DISTANCE = 16`
(`:128-131`), `textBaseline = "middle"`, `textAlign = "center"` (`:222-224`), and
derives `viewBox` by scanning for ink with `alpha > 8` (`:232-253`). Depends on a real
system font stack → **not portable and not needed**. Port it only as a stub that errors.

### 1.6 `Text.js` (124 lines)

`class Text extends THREE.Object3D`. `LAYOUT_DEFAULTS` (`:8-20`) =
`text '' / fontSize 1 / fontFamily 'monospace' / fontWeight 'normal' /
fontStyle 'normal' / letterSpacing 0 / lineHeight 'normal' / anchorX 0 / anchorY 0 /
textAlign 'left' / maxWidth Infinity`; each becomes an accessor that sets
`_needsSync = true` on change (`:112-124`). `color = new THREE.Color(0xffffff)`,
`_opacity = 1`. `opacity` setter writes through to `_batchedText.setOpacityAt` without
invalidating layout (`:59-65`). `sync()` (`:72-109`) early-returns when clean; in
vector mode with no font yet it stores `emptyRenderInfo` and **stays dirty**.

### 1.7 `BatchedText.js` (459 lines) — the renderer

`class BatchedText extends THREE.InstancedMesh`, constructed (`:36-113`) with
`PlaneGeometry(1,1)` + `new THREE.NodeMaterial()` where
`transparent = true`, `depthWrite = false`, `side = DoubleSide`, and
`count = maxGlyphCount` instances (then immediately `this.count = 0`, `:112`).

Per-glyph instanced attributes (`:87-109`), all `DynamicDrawUsage`:

| attribute | items | contents |
|---|---|---|
| `aGlyphUV` | 4 | atlas sub-rect `(u, v, w, h)`; `w == 0` marks a blank glyph |
| `aGlyphBounds` | 4 | padded quad `(x0, y0, x1, y1)` in text-local space |
| `aColor` | 3 | per-member colour, replicated per glyph |
| `aOpacity` | 1 | per-member opacity, replicated per glyph |

plus the inherited `instanceMatrix` (mat4 per glyph) written by `_writeGlyphMatrices`
(`:334-340`) as **the owning `Text`'s `matrixWorld`, identical for every glyph of a
member**.

`buildMaterial(material)` (`:137-195`) — the TSL chain, imported from `three/tsl`
(`:2-16`): `texture, uv, smoothstep, attribute, Fn, vec3, vec4, float, select, fwidth,
max, uniform, mix`:

```js
positionNode = Fn(() => vec3( mix(aGlyphBounds.x, aGlyphBounds.z, uv().x),
                              mix(aGlyphBounds.y, aGlyphBounds.w, uv().y), 0 ))()
atlasUV      = vec2( aGlyphUV.x + uv().x * aGlyphUV.z,
                     aGlyphUV.y + (1 - uv().y) * aGlyphUV.w )   // V flipped: atlas is Y-down
sdfValue     = texture(atlas).sample(atlasUV).r
aaWidth      = fwidth(sdfValue) * 0.5
fillAlpha    = smoothstep(0.5 - aaWidth, 0.5 + aaWidth, sdfValue)
outlineAlpha = smoothstep((0.5 - outlineWidth) - aaWidth, (0.5 - outlineWidth) + aaWidth, sdfValue)
outlineOnly  = max(outlineAlpha - fillAlpha, 0)
alpha        = select(aGlyphUV.z == 0, 0, max(fillAlpha, outlineOnly)) * aOpacity
haloRGB      = mix(aColor, outlineColorUniform, outlineColorMix)   // mix = 1 when set
colorNode    = vec4( mix(haloRGB, aColor, fillAlpha), alpha )
```

Uniforms: `outlineWidth` (default `options.outlineWidth ?? 0.03`, `:45`),
`_outlineColorUniform` (Color, default black), `_outlineColorMix` (0 or 1).

`sync(callback)` (`:342-458`) is the packing pass: collect every member's glyph chars
into a `Set` (spaces excluded, `:352`), `atlas.ensureGlyphs(chars)`, then per member
per glyph write the attributes. The quad and the sampled sub-rect are **both** grown by
`GLYPH_QUAD_PAD = 0.12` of their own size (`:23`, `:404-426`) so the halo is not
clipped, keeping the ink↔ink mapping; the sub-rect grow is clamped to [0,1] of the
viewBox (`:418-421`). Blank glyphs get the unpadded bounds and a zero UV rect
(`:427-437`). Finally `count = glyphCount` and every attribute's `needsUpdate = true`.

Also: `addText` / `removeText` / `getTextAt` (`:202-241`), `setColorAt` (`:243-263`),
`setOpacityAt` (`:270-283`), `outlineColor` accessor (`:286-301`),
`setMatrixAt(memberId, matrix)` overridden (`:309-326`) to mean *set the member's world
matrix* when `memberId < memberCount`, falling through to the real
`InstancedMesh.setMatrixAt` otherwise, `resetAtlas()` (`:128-135`).

Three.js / TSL dependencies to be matched in Rust: `InstancedMesh`,
`InstancedBufferAttribute` × 4, `PlaneGeometry(1,1)`, `DataTexture(RedFormat, FloatType,
LinearFilter)`, `NodeMaterial` with `positionNode` + `colorNode`, `transparent`,
`depthWrite = false`, `DoubleSide`, `Color`, and the TSL nodes
`texture().sample`, `uv`, `attribute`, `uniform`, `Fn`, `vec3/vec4/float`, `mix`,
`smoothstep`, `fwidth`, `max`, `select`, `.equal`.

### 1.8 How d33 uses it

`src/labels.js` (identical on all three branches) is the only consumer:

- `:23` `import { BatchedText, Text } from 'lib3/sdf-text';` — the import map in every
  example page (`examples/d3_treemap.html:25`) points `lib3/sdf-text` at
  `/lib3/sdfText.js`, served from `$LIB3_DIR/dist` by `test/e2e/server.js:26-29`.
- `hierarchyLabels(root, ids, positions, opts)` (`:124-…`): one `Text` per hierarchy
  node, `fontSize` in world units, `anchorY = 'middle'`, `color`,
  `outlineWidth = 0.25` / `outlineColor = 0xffffff` by default; the batch is sized
  exactly (`maxTextCount = nodes.length`, `maxGlyphCount = Σ text.replace(/\n/g,'').length`,
  `:149-155`); `frustumCulled = false` (`:160`); `renderOrder = 1`.
- `class LabelBatch extends BatchedText` (`:41-75`) — the workaround, verbatim
  comment at `:25-39`, summarised: drop `material.positionNode = null` and override
  `_writeGlyphMatrices` to bake `world × translate(centre) × scale(w, h)` (with the same
  `GLYPH_QUAD_PAD = 0.12`) into each glyph's instance matrix.
- ex-treemap (`examples/d3_treemap.html`): `SIDE 20`, `S = 20/WIDTH`, `THICK 0.3`,
  `LABEL_PX 5`, `LINE 0.9`, `LABEL_PAD_PX 3`, `FONT '/examples/fonts/Roboto-Regular.ttf'`;
  `fontSize = LABEL_PX / pxPerUnit` where
  `pxPerUnit = 250 / (2*dist*tan(fov*π/360))` (`:206-207`); labels are multi-line
  (`name.split(/(?=[A-Z][a-z])|\s+/g)` + `d3.format(',d')(value)`, `:233`) and only
  emitted where the block fits the tile (`:237`); after `hierarchyLabels` returns, every
  label is switched to `anchorY = 'top'`, `lineHeight = LINE` (`:263-268`), then
  `labels.batched.sync()`.
- One font only, everywhere, in every branch: `examples/fonts/Roboto-Regular.ttf`.

---

## 2. The Rust crate shape

### 2.1 Workspace placement

Root `Cargo.toml` of the working branch (`/home/tom/src/projects/three-rs/rung5/Cargo.toml`)
has `[workspace] members = [".", "d3/hierarchy"]`. Add the module as a third member,
mirroring how `d3/hierarchy` sits beside the root crate:

```toml
[workspace]
members = [".", "d3/hierarchy", "sdf-text"]
```

`sdf-text/Cargo.toml`:

```toml
[package]
name = "sdf-text"
version = "0.1.0"
edition = "2021"

[dependencies]
three-rs   = { path = ".." }          # Texture, InstancedMesh, nodes/tsl, Color, Matrix4
ttf-parser = { version = "0.25", features = ["opentype-layout"] }
tiny-skia  = { version = "0.11", default-features = false, features = ["std", "simd"] }
bytemuck   = { version = "1", features = ["derive"] }
```

`three-rs` must not depend on `sdf-text` (keeps the rung ladder's crate clean); the
example/e2e binary depends on both. All four crates are already in the local cargo
cache (`ttf-parser-0.25.1`, `owned_ttf_parser-0.25.1`, `tiny-skia-0.11.4`,
`ab_glyph-0.2.32`, `bytemuck`, `png-0.17.16`), so this builds offline.

Proposed modules (one per JS file, same names):

```
sdf-text/src/lib.rs              pub use: VectorFont, VectorFontAtlas, Text, BatchedText,
                                 layout_text_vector, empty_render_info
sdf-text/src/vector_font.rs      §1.1 + the GPOS kern walk
sdf-text/src/text_builder.rs     §1.2 layout_text_vector + TextRenderInfo
sdf-text/src/edt.rs              §1.4 edt_1d / edt_2d / compute_sdf
sdf-text/src/raster.rs           §1.3 steps 2-6 (outline → 256px coverage mask)
sdf-text/src/vector_font_atlas.rs §1.3 steps 7-10 + the atlas texture
sdf-text/src/text.rs             §1.6
sdf-text/src/batched_text.rs     §1.7 (attribute packing + the node chain)
```

### 2.2 Font parsing: **`ttf-parser` 0.25 with `opentype-layout`**

| candidate | verdict |
|---|---|
| **`ttf-parser`** | **Chosen.** Zero-alloc, `Face::outline_glyph(GlyphId, &mut dyn OutlineBuilder)` gives `move_to/line_to/quad_to/curve_to/close` — a 1:1 match for opentype's `M/L/Q/C/Z`. `glyph_hor_advance`, `units_per_em`, `typographic_ascender/descender/line_gap`, `capital_height`, `x_height`, `glyph_index(char)`. With `opentype-layout` it exposes the parsed **GPOS** `PairAdjustment` subtables (format 1 pair sets and format 2 class defs), which is exactly what opentype.js's `getKerningValue` walks — and `tables().kern` for the legacy path. |
| `owned_ttf_parser` | Only adds a self-referential owning wrapper over the same parser. Take it if lifetimes fight you (`OwnedFace`); no behavioural difference. |
| `fontdue` | Rasterises and caches but exposes no path commands and no GPOS — wrong layer. |
| `ab_glyph` | Outlines only as its own curve enum, no kerning beyond `kern`, and its rasteriser is its own AA model. Reject. |

Things to implement by hand because the crate does not hand them over:

1. **Ink bbox with exact curve extrema** (opentype's `Path.getBoundingBox`) — do *not*
   use `glyph_bounding_box()` or `outline_glyph()`'s returned `Rect` (both are the
   `glyf` header box). Elevate each quad to a cubic with the 2/3–1/3 rule and solve the
   derivative per axis, in `f64`, exactly as in §1.3 step 3.
2. **The GPOS `kern` lookup walk** with opentype.js's first-hit semantics (§1.1),
   including "GPOS present ⇒ never consult `kern`".
3. **The `||` metric precedence** (a zero `sTypoAscender` must fall through to `hhea`).

### 2.3 The 256px rasteriser: **`tiny-skia` `Path` + `FillRule::Winding`, anti-aliased**

`ctx.fill(path2d)` is Chromium Skia, nonzero winding, analytic AA, 8-bit coverage in
the alpha channel. `tiny-skia` is a direct port of Skia's raster pipeline, so
`PathBuilder::{move_to, line_to, quad_to, cubic_to, close}` →
`Pixmap::fill_path(&path, &white_paint, FillRule::Winding, Transform::identity(), None)`
with `paint.anti_alias = true` is the closest available match; read back
`pixmap.data()[i*4+3]` (tiny-skia is premultiplied RGBA, and with opaque white the alpha
channel is the coverage either way).

**Why near-match is enough, and where it isn't:** `compute_sdf` binarises at
`alpha >= 128` (§1.4), so the rasteriser only has to agree on the *set of texels whose
coverage ≥ ~50%*. Rounding to 8 bits means the JS threshold is effectively
`coverage >= 127.5/255 ≈ 0.5`. Texels whose true coverage sits within a few percent of
0.5 are the only ones at risk; each flip moves the EDT by at most ~1 raster px, i.e.
`1/(2*32) = 0.0156` in encoded SDF units, at a handful of positions, then gets
point-sampled away 15 times out of 16 by the 4:1 downsample. This is well inside the
grader's 0.1 threshold. **Fallback if step 2 of the ladder (§6) shows drift:** replace
tiny-skia with an own scanline filler computing *exact analytic area coverage* per texel
(signed-area accumulation over the flattened outline, nonzero winding) and threshold at
0.5 — deterministic, no dependency, and it removes the question entirely. Budget this
as the likely outcome; it is ~200 lines and it is the one place where "close enough"
is a judgement call rather than a fact.

Do **not** flatten curves more coarsely than Skia: use a
`(tolerance = 0.25 px)` recursive subdivision, or let tiny-skia do it.

### 2.4 The EDT and its parameters

Straight port of §1.4, `f64` throughout (the JS uses `Float64Array`; `f32` would change
`sqrt` results in the last bits and those feed the encode). Exact parameters for the
vector path:

| parameter | value | source |
|---|---|---|
| raster size | 256 × 256 | `VectorFontAtlas.js:8` |
| ink padding | `PAD_FRAC = 0.18` → `avail = 163.84` px | `:9`, `:154-155` |
| alpha threshold | 128 (`>=`) | `edt.js:42`, `:50` |
| `INF` | `1e20` | `edt.js:4` |
| MAX_DISTANCE | 32 raster px (`RASTER * 0.125`) | `:10` |
| encode | `clamp(0.5 - dist/64, 0, 1)` | `:192-193` |
| tile | 64 × 64, point-sampled at `4*sx+2` | `:7`, `:184-189` |
| atlas | 1024 × 1024, 16 × 16 = 256 tiles, insertion-order slots | `:28`, `:36-38`, `:134` |
| quad/sub-rect pad | `GLYPH_QUAD_PAD = 0.12` of own size | `BatchedText.js:23` |

Note the lib3 **test** uses a third, unrelated constant set —
`/home/tom/src/projects/lib3/src/sdf/index.js:3-9`: `GLYPH_SIZE 64, SDF_SIZE 32,
SDF_PADDING 4, MAX_DISTANCE 8, ALPHA_THRESHOLD 128` — which matches neither
`FontAtlas` (128/64/8/16) nor `VectorFontAtlas` (256/64/—/32). Keep `SDF_DEFAULTS` as
a separate `pub mod test_defaults` so the ported test layers 2–4 stay faithful (§3).

### 2.5 Texture format

JS: `DataTexture(Float32Array, 1024, 1024, RedFormat, FloatType)` → WebGPU `r32float`,
linear min/mag, no mipmaps, `flipY = false`, ClampToEdge, NoColorSpace.

three-rs can express this today: `Texture::set_format(wgpu::TextureFormat::R32Float)`
is public (`/home/tom/src/projects/three-rs/rung5/src/textures/texture.rs`),
`generate_mipmaps = false`, `wrap` is ClampToEdge-only anyway, `TextureFilter::Linear`
exists, and `Renderer::ensure_texture_2d`'s hardcoded `bytes_per_row: width * 4`
(`src/renderer/mod.rs:938`) is *coincidentally correct* for R32Float. Upload the atlas
as `bytemuck::cast_slice::<f32, u8>(&atlas_data).to_vec()`.

**The blocker:** `programs.rs:176-220` declares every non-depth texture as
`TextureSampleType::Float { filterable: true }` with a `Filtering` sampler, and
`r32float` is not filterable unless the adapter+device enable
`wgpu::Features::FLOAT32_FILTERABLE` — which three-rs never requests. Three options,
in order of preference:

1. **Request `FLOAT32_FILTERABLE`** in the device descriptor (Intel Iris Xe/Mesa and
   lavapipe both expose it) and keep the format identical to the JS. Highest fidelity,
   smallest diff surface. Add an adapter-capability assert so the failure is loud, not
   silent black (HANDOFF's "failures are silent wrong output" trap).
2. `R16Float` + filterable — loses ~3 mantissa bits on a value in [0,1]; the encode
   granularity is already 1/64 per raster px, so this is almost certainly invisible, but
   it is a deviation from the JS.
3. `R8Unorm` — 1/255 quantisation of the SDF. `fwidth`-based AA on a quantised field
   will band on near-horizontal/vertical glyph edges. Avoid.

Do **not** use `SampleMode::Load`/`texture_load` (the existing non-filterable escape
hatch): the shader relies on *bilinear* interpolation of the SDF, which is the whole
reason the 64px tile looks sharp at 5px text.

---

## 3. The grader

Two independent gates: the six lib3 layers as Rust unit tests against golden JSON, and
a pinned-frame pixel comparison against a d33 label page.

### 3.1 The six layers → Rust tests

| lib3 layer (`test/sdf-pipeline.test.mjs`) | Rust home | golden data |
|---|---|---|
| 1 — EDT via `computeSDF` (`:29-66`) | `sdf-text/src/edt.rs` `#[test]` | `golden/edt_*.json` — full `f64` fields for the 10×10 square, 4×4 empty, 4×4 filled, 16×16 circle fixtures, compared exactly (`==`) or at 1e-12 |
| 2 — SDF normalisation (`:68-80`) | `edt.rs` / `test_defaults` | none needed; three closed-form asserts, but keep `MAX_DISTANCE = 8` from `SDF_DEFAULTS`, not 32 |
| 3 — atlas UV math (`:82-110`) | `vector_font_atlas.rs` | none needed; `ATLAS_SIZE 512`, `SDF_SIZE 32`, `cols 16` as in the JS |
| 4 — shader-math simulation (`:112-144`) | `batched_text.rs` as a CPU mirror of the node chain | `golden/shader_alpha.json` — but note the JS layer uses a **fixed `edgeWidth = 0.1`**, not `fwidth`; port it as a separate `shader_alpha_fixed_width()` helper so the real node chain stays `fwidth`-based |
| 5 — text layout (`:146-170`) | `text_builder.rs` | `golden/layout_canvas_fallback.json` (the no-`OffscreenCanvas` branch, §1.2) **and** `golden/layout_vector.json` (the real one) |
| 6 — per-member opacity + outline colour (`:172-214`) | `text.rs` + `batched_text.rs` | none needed; the JS asserts are API behaviour (`opacity` writes through, does not dirty layout) plus two `mix` identities |

Layers 1–4 and 6 are pure arithmetic and port directly. Layers 5 and everything
atlas-shaped need goldens dumped from the JS.

#### Dump commands — pure-node layers (no browser)

Run from `/home/tom/src/projects/lib3` (that is where `node_modules` lives). Output goes
to this scout directory; nothing is written into lib3.

```bash
OUT=/home/tom/src/projects/three-rs/scouts/sdf-text/golden
mkdir -p "$OUT"
cd /home/tom/src/projects/lib3

# L1 — EDT fields for the four fixtures the lib3 test uses.
node --input-type=module -e '
import { computeSDF } from "./src/sdf/edt.js";
import { writeFileSync } from "node:fs";
const out = process.env.OUT;
const mk = (w,h,f) => { const img = new Uint8ClampedArray(w*h*4);
  for (let y=0;y<h;y++) for (let x=0;x<w;x++) if (f(x,y)) img[(y*w+x)*4+3]=255; return img; };
const cases = {
  square10:  { w:10,h:10, img: mk(10,10,(x,y)=>x>=2&&x<8&&y>=2&&y<8) },
  empty4:    { w:4, h:4,  img: mk(4,4,()=>false) },
  filled4:   { w:4, h:4,  img: mk(4,4,()=>true) },
  circle16:  { w:16,h:16, img: mk(16,16,(x,y)=>(x-7.5)**2+(y-7.5)**2<=36) },
};
const dump = {};
for (const [k,c] of Object.entries(cases))
  dump[k] = { w:c.w, h:c.h, alpha:[...c.img].filter((_,i)=>i%4===3),
              sdf:[...computeSDF(c.img, c.w, c.h)] };
writeFileSync(out+"/edt_fixtures.json", JSON.stringify(dump));
console.log("wrote edt_fixtures.json");
' 

# L5a — layoutText in the node (no-OffscreenCanvas) fallback branch.
node --input-type=module -e '
import { layoutText } from "./src/sdf-text/TextBuilder.js";
import { writeFileSync } from "node:fs";
const cases = [
  { text:"Hi!", fontSize:1 }, { text:"ab\ncd", fontSize:1 },
  { text:"ABC", fontSize:1, anchorX:"left" }, { text:"ABC", fontSize:1, anchorX:"center" },
  { text:"ABC", fontSize:1, anchorX:"right" }, { text:"A B", fontSize:2, letterSpacing:0.1 },
];
const dump = cases.map(p => { const i = layoutText(p);
  return { params:p, glyphCount:i.glyphCount, chars:i.glyphs.map(g=>g.char),
           glyphBounds:[...i.glyphBounds], blockBounds:i.blockBounds,
           lineHeight:i.lineHeight, ascender:i.ascender, descender:i.descender }; });
writeFileSync(process.env.OUT+"/layout_canvas_fallback.json", JSON.stringify(dump));
console.log("wrote layout_canvas_fallback.json", typeof OffscreenCanvas);
'

# L5b — layoutTextVector with Roboto, plus the font metrics and per-glyph data the
# Rust VectorFont must reproduce (advances, kerning, ink bboxes, path commands).
node --input-type=module -e '
import { VectorFont } from "./src/sdf-text/VectorFont.js";
import { layoutTextVector } from "./src/sdf-text/TextBuilder.js";
import { readFileSync, writeFileSync } from "node:fs";
const buf = readFileSync("examples/assets/fonts/Roboto-Regular.ttf");
const font = await VectorFont.load(new Uint8Array(buf));
const chars = [...new Set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 ,.()-")];
const glyphs = {};
for (const ch of chars) {
  const g = font.glyphForChar(ch);
  const p = g.getPath(0, 0, font.unitsPerEm);
  glyphs[ch] = { advance: font.advanceWidth(ch), bbox: font.boundingBox(ch),
                 pathBBox: p.getBoundingBox(), commands: p.commands };
}
const pairs = {};
for (const a of chars) for (const b of chars) {
  const k = font.kerning(a, b); if (k) pairs[a + b] = k;
}
const cases = [
  { text:"Hi!", fontSize:1 },
  { text:"AVATAR", fontSize:1 },
  { text:"flare", fontSize:0.5, anchorY:"middle" },
  { text:"AgglomerativeCluster\n456", fontSize:0.37, anchorY:"top", lineHeight:0.9 },
  { text:"Wave", fontSize:1, letterSpacing:0.05, anchorX:"center", anchorY:"middle" },
  { text:"start-anchored", fontSize:1, anchorX:"start" },
];
const layouts = cases.map(p => { const i = layoutTextVector({ ...p, font });
  return { params:p, glyphCount:i.glyphCount, chars:i.glyphs.map(g=>g.char),
           glyphBounds:[...i.glyphBounds], blockBounds:i.blockBounds,
           lineHeight:i.lineHeight, ascender:i.ascender, descender:i.descender }; });
writeFileSync(process.env.OUT+"/font_roboto.json", JSON.stringify(
  { unitsPerEm:font.unitsPerEm, ascender:font.ascender, descender:font.descender,
    lineGap:font.lineGap, capHeight:font.capHeight, xHeight:font.xHeight,
    hasGpos: !!font.font.tables.gpos, hasKern: !!font.font.tables.kern,
    glyphs, kernPairs:pairs }));
writeFileSync(process.env.OUT+"/layout_vector.json", JSON.stringify(layouts));
console.log("wrote font_roboto.json + layout_vector.json");
'
```

#### Dump command — the atlas (needs a browser: `Path2D` + `OffscreenCanvas`)

`VectorFontAtlas` cannot run in node (`new OffscreenCanvas` throws), so the atlas golden
must come out of headless Chrome. Reuse lib3's existing dev server and an existing page
as the module origin, so nothing is added to the lib3 tree:

```bash
# terminal A
cd /home/tom/src/projects/lib3 && npx vite --mode examples --port 5199 --strictPort

# terminal B
cd /home/tom/src/projects/lib3
OUT=/home/tom/src/projects/three-rs/scouts/sdf-text/golden node --input-type=module -e '
import { chromium } from "playwright-core";
import { writeFileSync } from "node:fs";
const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH ?? "/usr/bin/google-chrome",
  headless: false,
  // Match d33 test/e2e/puppeteer.js:176-201 so the 2-D raster path is the same
  // software Skia that produced the reference JPEGs.
  args: ["--headless=new","--hide-scrollbars","--enable-unsafe-swiftshader",
         "--use-gl=angle","--use-angle=swiftshader","--disable-gpu-rasterization",
         "--ignore-gpu-blocklist","--no-sandbox"],
});
const page = await browser.newPage();
await page.goto("http://localhost:5199/examples/sdf-text-vector/", { waitUntil: "domcontentloaded" });
const data = await page.evaluate(async () => {
  const { VectorFont } = await import("/src/sdf-text/VectorFont.js");
  const { VectorFontAtlas } = await import("/src/sdf-text/VectorFontAtlas.js");
  const font = await VectorFont.load("/examples/assets/fonts/Roboto-Regular.ttf");
  const atlas = new VectorFontAtlas({ atlasSize: 1024 });
  atlas.setFont(font);
  const chars = [..."AVo.e5 flare"];         // insertion order == slot order
  const tiles = {};
  for (const ch of chars) {
    const m = atlas.getGlyph(ch);
    const col = Math.round(m.u * 1024 / 64), row = Math.round(m.v * 1024 / 64);
    const tile = new Float32Array(64 * 64);
    for (let y = 0; y < 64; y++) for (let x = 0; x < 64; x++)
      tile[y * 64 + x] = atlas.atlasData[(row * 64 + y) * 1024 + col * 64 + x];
    tiles[ch] = { metrics: m, slot: row * 16 + col,
                  tile: btoa(String.fromCharCode(...new Uint8Array(tile.buffer))) };
  }
  // Also dump one raw 256px coverage mask so the rasteriser can be graded alone.
  const g = font.glyphForChar("a");
  const path = g.getPath(0, 0, font.unitsPerEm), pb = path.getBoundingBox();
  const gw = pb.x2 - pb.x1, gh = pb.y2 - pb.y1, avail = 256 - 2 * (256 * 0.18);
  const s = avail / Math.max(gw, gh);
  const offX = (256 - gw * s) / 2, offY = (256 - gh * s) / 2;
  const p2d = atlas._buildPath2D(path.commands, offX, offY, s, pb.x1, pb.y1);
  atlas.ctx.clearRect(0, 0, 256, 256);
  atlas.ctx.fillStyle = "white";
  atlas.ctx.fill(p2d);
  const alpha = new Uint8Array(256 * 256);
  const img = atlas.ctx.getImageData(0, 0, 256, 256).data;
  for (let i = 0; i < 256 * 256; i++) alpha[i] = img[i * 4 + 3];
  return { tiles, raster_a: btoa(String.fromCharCode(...alpha)),
           raster_a_params: { offX, offY, s, minX: pb.x1, minY: pb.y1 } };
});
writeFileSync(process.env.OUT + "/atlas_roboto.json", JSON.stringify(data));
await browser.close();
console.log("wrote atlas_roboto.json");
'
```

Rust side: decode the base64 into `Vec<f32>`/`Vec<u8>` in the test, compare the
rasteriser's 256×256 **binarised** mask (`alpha >= 128`) exactly, and the 64×64 tiles
to within a stated epsilon (start at 0 — aim for exact — and only loosen with a
recorded reason).

### 3.2 Pinned-frame pixel comparison against d33

**Page: `d3_treemap.html` on branch `ex-treemap`** — it is the densest label page, the
labels lie flat in the world (no billboard, so no camera-quaternion coupling), and it is
multi-line with `anchorY: 'top'`, which exercises the anchor and line-pitch paths.
(`d3_sunburst` adds radial orientation and the mirror flip; `d3_tree-of-life` adds the
cylinder + shim proxies. Both are better *second* targets.)

- **Frame:** exactly one. `test/e2e/deterministic-injection.js` patches
  `requestAnimationFrame` to a `setInterval` poll that fires the callback **once**, only
  after the page sets `window._renderStarted = true`, then sets `_renderFinished = true`.
- **Determinism (same file):** `Math.random` is replaced by
  `x = Math.sin(seed++) * 10000; x - Math.floor(x)` with `seed = Math.PI/4`;
  `Date.now`, `Date.prototype.getTime` and `performance.now` all return the constant `0`;
  `window.TESTING = true`; the original is kept as `Math._random`. `determinism.js`
  additionally rewrites the served three.js build to call `Math._random()` and to
  neutralise `trackTimestamp`. This is bit-for-bit the injection three-rs's rung 0
  already replicates (HANDOFF "The grader") — **nothing new to build here**, and the
  treemap page itself uses no `Math.random`, so only the pinned clock matters.
- **Geometry:** viewport `400 × 250` at `viewScale 2` → render `800 × 500`, then
  `Image.read(png).scale(1/2)` → compare at `400 × 250`;
  `pixelThreshold 0.1`, `maxDifferentPixels 0.1 %`
  (`test/e2e/puppeteer.js:40-53`, `:204`). Identical constants to three-rs's own
  `tests/e2e/compare.mjs`, so the same comparator works unchanged.
- **The golden:** d33 grabs the canvas with `toDataURL('image/png')` rather than
  `page.screenshot()` (`puppeteer.js:600-644`), then writes
  `examples/screenshots/d3_treemap.jpg` at quality 95. Dump a **PNG**, not the JPEG, to
  keep the JS→Rust comparison free of a second JPEG round-trip:

```bash
# Produce (or refresh) d33's own reference and the raw PNG, from a worktree of ex-treemap:
cd /home/tom/src/projects/d33
git worktree add ex-treemap ex-treemap          # if not already checked out
cd ex-treemap && npm ci
THREE_DIR=~/src/vendor/three.js node test/e2e/puppeteer.js --make d3_treemap
# → examples/screenshots/d3_treemap.jpg  (+ marks/d3_treemap.json, gates/d3_treemap.json)
node test/e2e/puppeteer.js --twice d3_treemap    # proves the page is deterministic here
cp examples/screenshots/d3_treemap.jpg \
   /home/tom/src/projects/three-rs/scouts/sdf-text/golden/d3_treemap.jpg
```

For an 800×500 lossless golden, add a one-off `--make`-adjacent capture by running the
same harness and copying the pre-downscale buffer — `runOnce` already holds it as `png`
(`puppeteer.js:582-595`); the cheapest honest route is to run the harness once with
`E2E_KEEP_PNG=1` added locally **in a throwaway worktree** (d33 is read-only for this
scout) or simply to accept the JPEG, since the comparator reads JPEGs for every other
rung already.

- **Rust side:** a new `examples/d33_treemap_labels.rs` in the three-rs crate (scene
  ported from `d3_treemap.html`, layout from the existing `d3-hierarchy` member crate,
  which already has `treemap.rs`), registered as a `[[example]]` plus a
  `#[path = "../../examples/d33_treemap_labels.rs"] mod …` + test fn in
  `tests/e2e/main.rs`, comparing against `golden/d3_treemap.jpg` instead of a
  three.js screenshot. `compare.mjs` takes explicit paths already, so only the
  `expected` argument changes.

---

## 4. What three-rs is missing for `BatchedText`

Inventory taken on `/home/tom/src/projects/three-rs/rung5` (7a3a30f) and
`/home/tom/src/projects/three-rs/port` (fc5d3e7).

| need | state | where |
|---|---|---|
| `InstancedMesh` | **exists** | `src/objects/instanced_mesh.rs`; `set_matrix_at`, `count`, `instance_matrix: InstancedBufferAttribute` (flat `Vec<f32>`, item_size 16). `instance_color` is declared but consumed nowhere. |
| instance matrix actually drawn | **exists, but as a uniform** | `src/renderer/mod.rs:298-312` (draw item), `:736-750` (`BufferSource::InstanceMatrix` → **UNIFORM** buffer), `src/nodes/tsl.rs:1082` `instance_matrix(count)` indexed by `@builtin(instance_index)`, draw `0..instance_count` at `:560-562`. **64 KiB uniform cap ⇒ ~1024 instances max**, and the buffer is rebuilt every frame. The treemap page needs ~1–2 k glyph quads, so this cap is a hard blocker. |
| **per-instance vertex attributes** (`aGlyphUV`, `aGlyphBounds`, `aColor`, `aOpacity`) | **missing entirely** | No `VertexStepMode::Instance` anywhere; `attribute()` resolves to per-vertex buffers only. This is the single largest piece of new renderer work: add instanced vertex buffers (or a storage buffer + `instance_index`, which also lifts the 1024 cap). |
| `PlaneGeometry(1,1)` with `uv` | **exists** | `src/geometries/plane.rs` |
| float texture upload | **nearly** | `Texture::set_format` is public; `ensure_texture_2d` (`src/renderer/mod.rs:870-960`) hardcodes `bytes_per_row = width*4` (:938) and the `flip_y` stride (:915) — correct for `R32Float`, wrong for any wider float format. |
| R32Float **filtering** | **blocked** | `src/renderer/programs.rs:176-220` declares `TextureSampleType::Float{filterable:true}` + `Filtering` sampler; `FLOAT32_FILTERABLE` is never requested in the device descriptor. See §2.5. |
| `fwidth` | **missing on every branch** | `dpdx`/`dpdy` exist **only on rung5**: `src/nodes/tsl.rs:275` and `:282`, whitelisted in the single-arg math pass at `src/nodes/builder.rs:709`. `fwidth(x) = abs(dpdx(x)) + abs(dpdy(x))` — but **`abs` is also missing** from tsl.rs (as are `fract`, `sqrt`, `step`). Two one-line additions + the builder whitelist. |
| `smoothstep`, `mix`, `max`, `select`, `.equal` | **exist** | `tsl.rs:266` (smoothstep — note its return type is hardcoded `Type::F32`, fine here), `:204`/`:515` (mix), `:414`/`:500` (max), `select()` lowers to if/else, comparison ops present. |
| `positionNode` | **missing** | `src/materials/mod.rs:40` has exactly one material struct, `MeshBasicNodeMaterial` (`MeshPhongNodeMaterial` is a type alias) with `color_node`, `vertex_node`, `fragment_node`, `specular_node`, `env_map`, `lights_node`; `normal_node` exists but is not read. No `position_node`, no `opacity_node`. |
| per-fragment alpha | **thrown away** | `src/materials/node_material.rs:93` (and again ~:275) hardcodes `diffuse_color().w().assign(float(1.0))` behind an `isOpaque()` comment. Either add the non-opaque branch or drive the whole fragment through `fragment_node`. |
| alpha blending | **missing** | `src/renderer/programs.rs:133-135`: `blend: None`, comment "Opaque material: three.js emits no blend state." No `transparent` field on the material. |
| transparency sort | **missing** | `render_list_order()` `src/renderer/mod.rs:1509-1532` is a stable painter sort over the **opaque list only**; "no transparent list" is a documented rung-4 gap in `handoff/RUNGS.md`. `depth_write`/`depth_test` *are* honoured (`programs.rs:152-164`, set at `mod.rs:459`), which is what `depthWrite = false` needs. |
| `DoubleSide` | check | `side` handling was not surfaced in the inventory; the treemap labels face up and the camera looks down, so single-sided may pass — verify rather than assume. |
| e2e harness | **exists, extend by copy-paste** | `tests/e2e/main.rs` (253 lines, `[[test]] name = "e2e"`), `tests/e2e/compare.mjs` wrapping three's own `test/e2e/image.js`. |

**What rung 5 provides of this list:** only `dpdx`/`dpdy` (plus `smoothstep`, `floor`,
`sign`, `exp2`, `length`, `mod`, and the Phong/light machinery that is irrelevant here).
Everything else in the "missing" rows is new work. Nothing in
`handoff/RUNGS.md` or any `docs/*.md` mentions SDF, text, or sprites — this is entirely
new ground. Note also that rung 11 of the ladder (`webgpu_mesh_batch`, `BatchedMesh`) is
listed as "what crush's BatchedText sits on"; `lib3`'s `BatchedText` actually sits on
**`InstancedMesh`** (rung 2, already green), so this module does **not** need rung 11.

**Suggested sequencing against the ladder:** the blending + transparent-list work is the
same work rung 9 (`webgpu_postprocessing_masking`) and any later transparent example
needs, so it is worth doing as a shared renderer rung rather than inside `sdf-text`.
The instanced-vertex-attribute work is sdf-text-specific today but also lifts rung 2's
1024-instance cap.

---

## 5. Traps

### 5.1 Canvas anti-aliasing — less dangerous than it looks, in one specific way

`computeSDF` binarises at `alpha >= 128`, so only the ~50 %-coverage texels are in play
(§2.3). But two things *are* environment-specific and must be pinned when dumping
goldens: (a) whether Chromium rasterises the `OffscreenCanvas` 2-D context on the GPU or
in software Skia — pass `--disable-gpu-rasterization` and match d33's flag list
(`test/e2e/puppeteer.js:176-201`, which forces SwiftShader for everything); (b) Skia's
curve flattening tolerance, which decides where a near-tangent edge lands. d33's
`src/labels.js:18-20` asserts the atlas is "rasterised by Chromium's software Skia" and
that two-run identity in the harness is the proof — so the golden is reproducible, but
only under those flags.

### 5.2 `positionNode` vs the instance matrix — the central design decision

Under three r186, `NodeMaterial.setupPosition` runs `instancedMesh(object)`
(`~/src/vendor/three.js/src/materials/nodes/NodeMaterial.js:798`, which does
`positionLocal.assign(instanceMatrixNode.mul(positionLocal).xyz)` —
`src/nodes/accessors/Instance.js:206-207`) and *then*
`positionLocal.assign(subBuild(this.positionNode, …))` (`:804-806`). The assignment
**discards** the instance transform. lib3's `BatchedText` was written against
three 0.184 where the order worked out, so under the vendored three every member lands
at the origin — exactly what d33 measured and documented verbatim at
`src/labels.js:25-39`.

d33's fix: `material.positionNode = null` and bake
`world × translate((x0+x1)/2, (y0+y1)/2, 0) × scale(x1-x0, y1-y0, 1)` into each glyph's
instance matrix (`src/labels.js:41-75`).

**Recommendation: implement `position_node` properly in three-rs — applied *before* the
instance matrix — and do not replicate three's ordering bug.** Justification: on a unit
`PlaneGeometry(1,1)` (positions −0.5…+0.5, `uv = position + 0.5`), d33's baked matrix and
lib3's `mix(bounds.x, bounds.z, uv.x)` produce **the same world position for every
vertex**, so a correct implementation matches d33's golden pixels exactly while also
making `setMatrixAt(memberId, …)` (the billboard path that `d3_sunburst` and
`d3_tree-of-life` use) work as written. Concretely: in `materials::setup`, assign
`position_node` into `positionLocal` *before* the `instance_matrix` multiply, i.e. the
reverse of `NodeMaterial.js:798/804`. Record the deviation in `docs/nodes.md` next to
the other divergences — the grader here is d33's output, not three's, so this is not a
grader-honesty violation, but it must be written down.

Keep `_write_glyph_matrices` writing the member's plain `matrix_world` (lib3's
`BatchedText.js:334-340`), *not* d33's baked form.

### 5.3 Premultiplied alpha, sRGB, filtering

- The atlas is `NoColorSpace` single-channel data; no sRGB transfer anywhere on the
  sample path. three-rs carries `ColorSpace` on textures and relies on
  `rgba8unorm-srgb` formats — make sure the R32Float atlas is **not** tagged sRGB.
- `aColor` in the JS comes from `THREE.Color`, which under `.set(0x222222)` applies
  `SRGBToLinear` when `THREE.ColorManagement.enabled` (default true in r152+). d33 sets
  `color: 0x222222` and `outlineColor: 0xffffff`. Reproduce the **linear** values, not
  the hex bytes: `(0x22/255)^2.4`-ish via three's exact `SRGBToLinear` polynomial.
  Getting this wrong is a uniform colour shift over every glyph — the most likely
  single cause of a failing first diff.
- Output: d33 renders through WebGPURenderer's default output transform
  (linear → sRGB OETF, and premultiply/unpremultiply around it). three-rs already has
  `materials::output_fragment_node` + `tsl::srgb_transfer_oetf` /
  `premultiply_alpha` / `unpremultiply_alpha` (`src/nodes/tsl.rs:1199-1250`). With
  blending added, make sure the blend factors match three's
  (`SrcAlpha, OneMinusSrcAlpha` for non-premultiplied `NormalBlending`) and that the
  premultiply step sits where three puts it, or the halo edges will be wrong by exactly
  one alpha multiply.
- The golden was produced with SwiftShader's bilinear filter; three-rs's grader runs on
  the Intel adapter (rung 0). Bilinear on `r32float` should be exact on both, but this
  is the kind of difference that shows up as a 1-texel-wide ring around small glyphs —
  check the diff image before blaming the SDF.

### 5.4 Layout quirks that must be preserved, not fixed

- `anchorX: 'start' | 'end'` (d33 `src/labels.js:191`) hits **no branch** of
  `resolveAnchor` (`TextBuilder.js:25-37`) and therefore behaves as offset 0, i.e.
  left-anchored, for both leaves and internal nodes. Port the fall-through.
- The `||` chains in `VectorFont`'s constructor treat 0 as absent (§1.1).
- `letterSpacing` is added *between* glyphs in the measure pass but *after* every glyph
  in the pen loop (`TextBuilder.js:254` vs `:382`) — a real asymmetry that shifts
  `textAlign: center/right` by one space. d33 uses neither, but the ported tests will.
- Atlas slot order is insertion order over a JS `Set` (§1.3). Any reordering changes
  every glyph's UV.
- `BatchedText.sync` excludes spaces from `ensureGlyphs` but still emits an instance for
  them with a zero UV rect (`BatchedText.js:352`, `:433-436`), and `count` includes
  them. Instance counts must match exactly or the painter order shifts.
- `MAX_DISTANCE` is `RASTER * 0.125` and the glyph is scaled so its *larger* ink
  dimension is 163.84 px — so the SDF spread, and therefore the world-space thickness of
  the `outlineWidth` halo, **varies per glyph** (a wide `m` gets a thinner halo than an
  `l`). Do not "fix" it.
- d33 passes `outlineWidth: 0.2` (treemap) / `0.25` (default) in SDF units against
  lib3's own default of `0.03` — i.e. a halo ~7× the lib3 default. At
  `0.5 - 0.2 = 0.3` the outline edge is well inside the encoded range (0 at 32 raster px
  out), so it is valid, but the halo reaches far into the padded sub-rect and the
  `GLYPH_QUAD_PAD = 0.12` clamp at `BatchedText.js:418-421` can clip it at tile edges.
  Expect visible halo clipping in the golden — reproduce it, do not round it off.

### 5.5 Other

- `ttf-parser`'s bbox ≠ opentype's bbox (§2.2 item 1). This silently shifts every glyph
  quad *and* the atlas `viewBox`, and it will look like a layout bug.
- `edt1d`'s `k--` underflow (§1.4) is benign in JS and a panic in Rust.
- `f64` everywhere in the EDT and the bbox extrema solver; `f32` only at the final
  encode into the atlas (`Float32Array`) and in `glyphBounds` (`Float32Array`) — match
  those truncation points exactly.
- 1024-instance uniform cap (§4) will silently clip the treemap's labels if the
  instanced-attribute work is skipped.
- d33 is a bare-worktree repo and the three example branches are **not checked out**;
  `git --git-dir=…/.bare show <branch>:<path>` is the read path, and running the harness
  needs `git worktree add`.

---

## 6. Ladder

Five steps. Each is a separate worker with a checkable gate; steps 1–3 need no GPU.

| # | step | gate |
|---|---|---|
| **1** | **Font outlines and metrics match opentype.js.** `sdf-text/src/vector_font.rs` + the exact-extrema bbox solver + the GPOS `kern` walk, graded against `golden/font_roboto.json` (§3.1): every advance, every kern pair, every ink bbox, and every path command for ~67 Roboto glyphs. | all three dumps match exactly (integers) / to 1e-9 (bbox extrema) |
| **2** | **256px raster + EDT + atlas tile match.** `edt.rs` (port of §1.4, `f64`), `raster.rs` (tiny-skia, nonzero winding, AA), `vector_font_atlas.rs` (steps 4–10). Graded against `golden/edt_fixtures.json` (exact) and `golden/atlas_roboto.json`: the 256×256 binarised mask exactly, the 64×64 `f32` tiles and `viewBox`/`u`/`v`/`w`/`h` metrics to epsilon 0. | mask identical; tiles identical, or the §2.3 own-scanline-filler fallback lands them |
| **3** | **`layout_text_vector` matches.** `text_builder.rs` + `text.rs`, graded against `golden/layout_vector.json` and `golden/layout_canvas_fallback.json`, plus the six lib3 layers ported as `#[test]`s (§3.1) including layers 2/3/4's `SDF_DEFAULTS` constants and layer 6's opacity write-through. | every `glyphBounds`/`blockBounds`/`lineHeight` float bit-equal after `f32` truncation; all six layers green |
| **4** | **Renderer capabilities: one glyph on screen.** The renderer work in §4 — instanced vertex attributes (or a storage buffer, lifting the 1024 cap), `FLOAT32_FILTERABLE` + R32Float atlas upload, `abs`/`fwidth` in `tsl.rs`, `position_node` applied **before** the instance matrix (§5.2), per-fragment alpha out of `node_material.rs:93`, alpha blending in `programs.rs:133`, a transparent bucket in `render_list_order`. Target: a hand-written `examples/sdf_text_one_glyph.rs` drawing a single `'a'` quad from the ported atlas, compared against a PNG dumped from an equivalent lib3 page. | the single glyph's AA edge and halo within the 0.1/0.1 % grader budget |
| **5** | **`BatchedText` assembles: a multi-glyph, multi-member label block.** `batched_text.rs` — attribute packing, `GLYPH_QUAD_PAD`, blank-glyph instances, insertion-order slots, `set_color_at`/`set_opacity_at`/`outline_color`, `sync()`. Target: `examples/sdf_text_block.rs` reproducing lib3's `examples/sdf-text-vector/` page (Roboto, `outlineWidth 0.04`, several `Text` members at different sizes and anchors) against a PNG dumped from that page under the §3.2 flags. | grader pass; `--twice`-style two-run identity on the Rust side |
| **6** | **d33 treemap labels pixel-match.** `examples/d33_treemap_labels.rs`: port `d3_treemap.html` (flare.json → `d3-hierarchy`'s `treemap.rs`, the tile meshes, `frameCamera`, the `pxPerUnit`/`LABEL_PX` em, the fit test, `anchorY: 'top'` + `lineHeight: 0.9`), plus the `Color` sRGB→linear conversion of §5.3, compared against `golden/d3_treemap.jpg` at 400×250. | `< 0.1 %` different pixels, and the director has looked at actual/expected/diff |

Step 4 is the long one and is mostly *renderer* work that later rungs want anyway
(blending, a transparent list, instanced attributes); consider splitting it into
"blending + transparent list" and "instanced attributes + float atlas" as two parallel
workers, since the two touch disjoint files (`programs.rs`/`render_list_order` vs
`renderer/mod.rs` buffers + `nodes/tsl.rs`).
