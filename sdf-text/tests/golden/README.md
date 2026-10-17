# Golden data for `sdf-text`

Everything here was dumped from the JavaScript originals on 2026-09-13. The Rust
port is graded against these files; they are the only ground truth.

Two JavaScript projects are involved:

- **lib3** — the maintainer's library of TSL nodes for three.js WebGPU, whose `src/sdf-text/`
  and `src/sdf/edt.js` this crate ports. It is public:
  [github.com/oneilltomhq/lib3](https://github.com/oneilltomhq/lib3) (MIT), and
  on npm as [`@oneilltom/lib3`](https://www.npmjs.com/package/@oneilltom/lib3),
  whose `@oneilltom/lib3/sdf-text` export carries `VectorFont`.
- **d3-three** (formerly *d33*) — the maintainer's port of d3 gallery examples to
  three.js, whose `examples/d3_treemap.html` labels its tiles with lib3's
  `BatchedText`. It is **not public yet**. Nothing below needs it: the one
  golden taken from it (`d3_treemap_labels.json`) regenerates from public
  packages, and only the image gate in `tests/d3_treemap_labels.rs` reads its
  reference screenshot, skipping when it is absent.

Nothing was written into either tree — all commands are read-only there.

Font: `Roboto-Regular.ttf`, the only font lib3 and d3-three use. lib3's
`examples/assets/fonts/` copy, d3-three's `examples/fonts/` copy and this
crate's `tests/assets/` copy are md5-identical
(`8f793587dcf03f31c551c5b60d175fc2`).
opentype.js version: the one in lib3's `node_modules` (`^2.0.0`, 2.0.0 at the time).

| file | contents | produced by |
|---|---|---|
| `edt_fixtures.json` | `computeSDF` output (`f64`) + the input alpha plane for the four lib3-test fixtures (`square10`, `empty4`, `filled4`, `circle16`) | node, command 1 |
| `layout_canvas_fallback.json` | `layoutText` for 6 cases, taken in node where `OffscreenCanvas` is `undefined` so the `0.6`-width / ±0.8/0.2 fallback branch runs | node, command 2 |
| `font_roboto.json` | font metrics, plus per-glyph advance / ink bbox / `Path.getBoundingBox()` / full command list for 67 chars, plus every non-zero kern pair over those chars | node, command 3 |
| `layout_vector.json` | `layoutTextVector` for 6 cases | node, command 3 |
| `atlas_roboto.json` | `VectorFontAtlas` 64×64 `f32` tiles + metrics for `"AVo.e5 flare"` in insertion order, and one raw 256×256 coverage-alpha mask for `'a'` | headless Chrome, command 4 |

Floats are written by `JSON.stringify`, which round-trips IEEE-754 doubles exactly.
The `tile` and `raster_a` fields are base64 of the raw little-endian bytes
(`Float32Array` and `Uint8Array` respectively).

## Commands

All four run from a checkout of
[oneilltomhq/lib3](https://github.com/oneilltomhq/lib3) (that is where
`node_modules` lives), with

```bash
# from the root of the three-rs checkout:
export OUT="$PWD/sdf-text/tests/golden"
# LIB3_DIR is a lib3 checkout, e.g. git clone https://github.com/oneilltomhq/lib3
cd "$LIB3_DIR" && npm ci
```

### 1 — `edt_fixtures.json`

```bash
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
```

### 2 — `layout_canvas_fallback.json`

```bash
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
console.log("wrote layout_canvas_fallback.json; OffscreenCanvas:", typeof OffscreenCanvas);
'
# prints: OffscreenCanvas: undefined   <- proof the fallback branch was the one taken
```

### 3 — `font_roboto.json` + `layout_vector.json`

```bash
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

### 4 — `atlas_roboto.json` (needs a browser: `Path2D` + `OffscreenCanvas`)

`VectorFontAtlas` cannot run in node (`new OffscreenCanvas` throws), so this one comes
out of headless Chrome driven by `playwright-core`, which **is** installed in lib3
(`devDependencies`, `^1.61.1`). The browser is the system `/usr/bin/google-chrome`
(or `CHROME_PATH`); the flag list matches the one in d3-three's `test/e2e/puppeteer.js`
at the time, so the 2-D raster path is the same software Skia that produced
d3-three's reference images.

Terminal A — lib3's own dev server, which resolves the bare `three/webgpu` specifier
inside `VectorFontAtlas.js`:

```bash
cd "$LIB3_DIR"
npx vite --mode examples --port 5199 --strictPort
```

Terminal B:

```bash
cd "$LIB3_DIR"   # with OUT exported as above
node --input-type=module -e '
import { chromium } from "playwright-core";
import { writeFileSync } from "node:fs";
const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH ?? "/usr/bin/google-chrome",
  headless: false,
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

Notes on the captured run:

- The page logs `No available adapters.` / `WebGPU is not available, running under
  WebGL2 backend.` — irrelevant here: nothing in this dump touches the WebGPU renderer,
  only the 2-D `OffscreenCanvas` path and `computeSDF`.
- Slot order in the dump is `A V o . e 5 ' ' f l a r` (slots 0..10); `'e'` occurs twice
  in the string and is rasterised once. **Read the `slot` field, not the JSON key
  order** — `JSON.stringify` emits the integer-like key `"5"` first.
- `' '` (space) maps to a real Roboto glyph with an empty outline: its `viewBox` stays
  `[0,0,0,0]` and its tile is all zeros, but it still consumes slot 6.

## `d3_treemap_labels.json`, for the d3-three treemap gate

`d3_treemap_labels.json` is the label arithmetic of d3-three's
`examples/d3_treemap.html`, dumped from that page's own JavaScript on
2026-09-13. It is the ground truth for `tests/d3_treemap_labels.rs` (step 6).

Produced by `dump_d3_treemap_labels.mjs`, which is committed next to it with a
`package.json` and `package-lock.json` that pin its dependencies. From this
directory:

```bash
npm ci
node dump_d3_treemap_labels.mjs d3_treemap_labels.json
```

The output is byte-identical to the committed file. Each input is a public
stand-in for what the page itself loads:

| what | from | stands for |
|---|---|---|
| `Vector3` / `Box3` / `Sphere` / `PerspectiveCamera` / `Quaternion` | npm `three` 0.184.0 | the page's vendored three.js |
| `d3.treemap`, `d3.treemapBinary`, `d3.hierarchy`, `d3.format` | npm `d3` 7.9.0 | d3-three's esbuild bundle `examples/jsm/vendor/d3.js` (of `d3` `^7.9.0`) |
| `VectorFont` (advances, kerning, unitsPerEm) | npm `@oneilltom/lib3` 0.5.0, `@oneilltom/lib3/sdf-text` (opentype.js bundled) | the page's `import { VectorFont } from 'lib3/sdf-text'` |
| `flare.json` | `$D3_GALLERY_DIR/notebooks/hierarchies/treemap.v2/files/flare.json` (default `~/src/vendor/d3-gallery`) | the page's `fetch( '/gallery/hierarchies/treemap.v2/files/flare.json' )`, served from a d3-gallery checkout |
| `Roboto-Regular.ttf` | `../assets/Roboto-Regular.ttf` (this crate's copy) | d3-three's `examples/fonts/Roboto-Regular.ttf`, md5-identical |

`frameCamera` (d3-three's `src/frame.js`) and `basisQuaternion`
(`src/labels.js`) are copied into the script verbatim rather than imported,
because both files resolve `three` as a bare specifier against the vendored
build the page serves over HTTP, and d3-three is not public. three
0.184.0 is not the 186dev build the page runs, but nothing here touches anything
beyond `Vector3`/`Matrix4`/`Quaternion`/`Sphere`/`PerspectiveCamera` arithmetic,
which is unchanged between the two.

The page's own numbers are reproduced by the dump: 220 leaves, 30 labels,
`fontSize` 0.49829423116133487, `pxPerUnit` 10.034232161080606 — the page logs
`fontSize: 0.498 px/unit: 10.03 cells: 220 leaves: 220 labels: 30`.

Floats are written by `JSON.stringify`, which round-trips IEEE-754 doubles
exactly.

### Fields

| field | meaning |
|---|---|
| `WIDTH` … `LABEL_PAD_PX` | the page's constants, so a drift in either copy shows up |
| `INNER_WIDTH` / `INNER_HEIGHT` | the d3-three harness viewport (`400 × 250 @ viewScale 2`, dpr 1) |
| `camera` | `fov`, `aspect`, and the `near` / `far` / `position` / `quaternion` / `target` `frameCamera` settles on |
| `pxPerUnit`, `fontSize`, `pad` | the label em derivation |
| `flatQuaternion` | `basisQuaternion( (1,0,0), (0,0,-1) )` |
| `leaves` | all 220 `root.leaves()`, with `name`, `depth`, `value`, `pkg` and the rounded treemap rect |
| `labelled` | the 30 leaves that pass the fit test, with the split `lines`, the `widest` measured line, and the world `anchor` |

### The image gate

The `< 0.1 %` comparison the ladder plan asks for is against d3-three's
`examples/screenshots/d3_treemap.jpg` (400 × 250, the d3-three harness' own
reference). The test reads it from `$D3_THREE_DIR/examples/screenshots/`
(default `~/src/projects/d3-three/main`) and skips the comparison, saying so,
when it is not there; it is not copied into this repo. It is **not reached yet** — 156 of 100000 pixels, 0.156 % — but
it is now asserted under a ceiling rather than only printed, and the remaining
gap is one deviation, not two: the page's tiles are `MeshStandardNodeMaterial`
under a `HemisphereLight` and a `DirectionalLight` where this example still uses
`MeshBasicNodeMaterial`, and every one of the 156 surviving pixels is on the
slab's near silhouette, where those lights shade the tiles' side walls. The
per-tile `THREE.Line` + `LineBasicNodeMaterial` outlines *are* drawn since the
`lines` branch (2551 → 156, `docs/lines-progress.md`).
