# Rung log

| rung | example | result | commit on `port` | notes |
|---|---|---|---|---|
| 0 | grader | calibrated | bdbb76a (handoff) | see rung0/RUNG0.md |
| 1 | webgpu_depth_texture | PASS 0/100000 px, Intel Iris Xe | 2362510 | max RGB distance 15.6 of 44 limit; hand-written WGSL in src/renderer/shaders/{basic,quad}.wgsl to replace at rung 4; in-page devicePixelRatio is 1 (grader sets no deviceScaleFactor) |
