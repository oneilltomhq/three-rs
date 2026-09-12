# Rung log

| rung | example | result | commit on `port` | notes |
|---|---|---|---|---|
| 0 | grader | calibrated | bdbb76a (handoff) | see rung0/RUNG0.md |
| 1 | webgpu_depth_texture | PASS 0/100000 px, Intel Iris Xe | 2362510 | max RGB distance 15.6 of 44 limit; hand-written WGSL in src/renderer/shaders/{basic,quad}.wgsl to replace at rung 4; in-page devicePixelRatio is 1 (grader sets no deviceScaleFactor) |
| 2 | webgpu_instance_mesh | PASS 45/100000 px (three itself scores 60 vs the same JPEG) | f15e158 | key finding: scene renders into an internal rgba16float MSAA target in linear, then a separate output pass applies sRGB (Renderer.needsFrameBufferTarget / _renderOutput); converting inside the scene pass fails at edges. New WGSL: normal_world_range_mix.wgsl, output_color_transform.wgsl |
| 3 | webgpu_materials_basic | PASS 0/100000 px (max RGB distance 43.0 of 44 limit, single JPEG-ringing pixels on edges) | f4c47da | cube texture rgba8unorm-srgb (GPU does sRGB→linear, no colour-space node), Three's mipmap blit is per-face 2d-array bilinear box, background is a BackSide SphereGeometry(1,32,32) drawn first with depthCompare always. New WGSL: basic_envmap, background_cube, mipmap. Last rung with hand-written WGSL. |
