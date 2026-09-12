Raw dump of webgpu_postprocessing_masking from the real page (Chrome 152,
--use-angle=vulkan, Intel Iris Xe), taken with the temporary scout script
test/e2e/_dump_rung9.mjs (since deleted).

moduleNN_*.wgsl  every GPUDevice.createShaderModule code, in creation order
pipelines.json   every createRenderPipeline descriptor incl. resolved bind group layouts
layouts.json     every createBindGroupLayout descriptor
passes.json      every beginRenderPass: attachments, views, formats, load/store, clears, draws
textures.json    every createTexture: label, size, format, sampleCount, mipLevelCount, usage
actual_full.png  800x500 page screenshot at the dumped frame

The named copies next to PLAN.md are: module00/01 -> scene_basic.{vert,frag}.wgsl,
module03/04 -> output_quad.{vert,frag}.wgsl, module02 -> mipmap.wgsl.
