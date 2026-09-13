# Golden data for the d33 treemap gate

`d3_treemap_labels.json` is the label arithmetic of
`/home/tom/src/projects/d33/rung0/examples/d3_treemap.html`, dumped from that
page's own JavaScript on 2026-09-13. It is the ground truth for
`tests/d33_treemap_labels.rs`; nothing was written into the d33 or lib3 trees.

Produced by `dump_d3_treemap_labels.mjs`, which is committed next to it:

```bash
node tests/golden/dump_d3_treemap_labels.mjs tests/golden/d3_treemap_labels.json
```

It imports, by absolute path and read-only:

| what | from |
|---|---|
| `Vector3` / `Box3` / `Sphere` / `PerspectiveCamera` / `Quaternion` | `lib3/node_modules/three` 0.184.0 |
| `d3.treemap`, `d3.treemapBinary`, `d3.hierarchy`, `d3.format` | d33's bundled `rung0/examples/jsm/vendor/d3.js` (d3 7.9) |
| `VectorFont` (advances, kerning, unitsPerEm) | `lib3/src/sdf-text/VectorFont.js` + `lib3/node_modules/opentype.js` |
| `flare.json` | `~/src/vendor/d3-gallery/notebooks/hierarchies/treemap.v2/files/flare.json` |
| `Roboto-Regular.ttf` | `d33/rung0/examples/fonts/` (md5-identical to lib3's and to this repo's copy) |

`frameCamera` (`d33/rung0/src/frame.js`) and `basisQuaternion`
(`d33/rung0/src/labels.js`) are copied into the script verbatim rather than
imported, because both files resolve `three` as a bare specifier and d33 has no
`three` in `node_modules` — it serves the vendored build over HTTP. three
0.184.0 is not the 186dev build the page runs, but nothing here touches anything
beyond `Vector3`/`Matrix4`/`Quaternion`/`Sphere`/`PerspectiveCamera` arithmetic,
which is unchanged between the two.

The page's own numbers are reproduced by the dump: 220 leaves, 30 labels,
`fontSize` 0.49829423116133487, `pxPerUnit` 10.034232161080606 — the page logs
`fontSize: 0.498 px/unit: 10.03 cells: 220 leaves: 220 labels: 30`.

Floats are written by `JSON.stringify`, which round-trips IEEE-754 doubles
exactly.

## Fields

| field | meaning |
|---|---|
| `WIDTH` … `LABEL_PAD_PX` | the page's constants, so a drift in either copy shows up |
| `INNER_WIDTH` / `INNER_HEIGHT` | the d33 harness viewport (`400 × 250 @ viewScale 2`, dpr 1) |
| `camera` | `fov`, `aspect`, and the `near` / `far` / `position` / `quaternion` / `target` `frameCamera` settles on |
| `pxPerUnit`, `fontSize`, `pad` | the label em derivation |
| `flatQuaternion` | `basisQuaternion( (1,0,0), (0,0,-1) )` |
| `leaves` | all 220 `root.leaves()`, with `name`, `depth`, `value`, `pkg` and the rounded treemap rect |
| `labelled` | the 30 leaves that pass the fit test, with the split `lines`, the `widest` measured line, and the world `anchor` |

## The image gate

The `< 0.1 %` comparison the ladder plan asks for is against
`d33/rung0/examples/screenshots/d3_treemap.jpg` (400 × 250, the d33 harness'
own reference). It is **not reachable on this branch** and the test does not
assert it — see the header of `examples/d33_treemap_labels.rs` for the list:
the page's tiles are `MeshStandardNodeMaterial` under a `HemisphereLight` and a
`DirectionalLight` (PBR is rung 8; neither light exists in this port), and every
tile is outlined with a `THREE.Line` + `LineBasicNodeMaterial` (no line topology
in this port at all). The test still renders the frame, runs the same
`tests/e2e/compare.mjs` over it and prints the number, so the gap is measured
rather than assumed.
