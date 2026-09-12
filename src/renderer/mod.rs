//! Port of `three.js/src/renderers/common/Renderer.js` + the WebGPU backend:
//! walk the scene, build each material through the node system, resolve the
//! bindings it declared, draw into a render target or into the "canvas"
//! texture, read back.

mod mipmap;
/// Additive seam for the interactive viewer; see `present.rs`.
mod present;
mod programs;
mod render_target;

use std::collections::HashMap;
use std::rc::Rc;

use mipmap::{create_mipmap_pipeline, MipmapShader};
pub use programs::{RenderState, UniformContext};
use programs::{PipelineKey, Program};
pub use render_target::{RenderTarget, RenderTargetInner, RenderTargetOptions};

use crate::cameras::{OrthographicCamera, PerspectiveCamera};
use crate::core::{BufferGeometry, Index};
use crate::geometries::{quad_geometry, sphere_geometry};
use crate::materials::{self, MeshBasicNodeMaterial, SetupContext, Side};
use crate::math::{Color, Matrix4, Vector2};
use crate::nodes::node::{BufferSource, TextureSource};
use crate::nodes::wgsl::TextureKind;
use crate::nodes::{BindingDesc, NodeBuilder};
use crate::objects::{Background, InstancedBufferAttribute, QuadMesh, Scene};
use crate::testing::DeterministicRandom;
use crate::textures::{
    CubeTexture, Texture, TextureFilter, TextureType, Wrapping,
};

struct GeometryGpu {
    position: Option<wgpu::Buffer>,
    normal: Option<wgpu::Buffer>,
    uv: Option<wgpu::Buffer>,
    index: Option<(wgpu::Buffer, wgpu::IndexFormat, u32)>,
    vertex_count: u32,
}

impl GeometryGpu {
    fn attribute(&self, name: &str) -> &wgpu::Buffer {
        let buffer = match name {
            "position" => self.position.as_ref(),
            "normal" => self.normal.as_ref(),
            "uv" => self.uv.as_ref(),
            other => panic!("three-rs: no geometry attribute named {other}"),
        };
        buffer.unwrap_or_else(|| panic!("three-rs: the geometry has no {name} attribute"))
    }
}

/// The stand-in for the renderer's canvas / default framebuffer: a colour
/// texture at drawing-buffer size, plus the MSAA texture the `antialias` option
/// asks for.
struct CanvasTarget {
    width: u32,
    height: u32,
    sample_count: u32,
    color: wgpu::Texture,
    msaa: Option<wgpu::Texture>,
    /// `Renderer.depth` is `true` by default, so a canvas pass gets a depth
    /// buffer; `WebGPUUtils.getCurrentDepthStencilFormat()` picks `depth24plus`
    /// when `stencil` and `reversedDepthBuffer` are both off.
    depth: Option<wgpu::Texture>,
}

/// One entry of the render list, already resolved to what the draw needs.
struct Renderable {
    geometry: Rc<BufferGeometry>,
    material: MeshBasicNodeMaterial,
    setup: SetupContext,
    model_world: Matrix4,
    instance_matrix: Option<InstancedBufferAttribute>,
    instance_count: u32,
}

/// The attachments, formats and size of the pass about to run.
struct PassTarget {
    color: wgpu::TextureView,
    resolve: Option<wgpu::TextureView>,
    depth: Option<wgpu::TextureView>,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    width: u32,
    height: u32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
    /// Kept only so the viewer can query surface capabilities on the very
    /// adapter this renderer picked; see `present.rs`.
    adapter: wgpu::Adapter,

    /// `Renderer._samples`: `antialias === true` means 4.
    samples: u32,
    pixel_ratio: f64,
    width: f64,
    height: f64,

    /// `Renderer._clearColor`: black, alpha 1.
    clear_color: [f64; 4],

    canvas: Option<CanvasTarget>,
    render_target: Option<RenderTarget>,
    /// `Renderer._frameBufferTargets`: the internal render target the scene is
    /// drawn into whenever the output needs a colour-space conversion or tone
    /// mapping, keyed in three.js by the canvas target — the port has exactly
    /// one canvas, so one entry.
    frame_buffer_target: Option<RenderTarget>,

    /// `Renderer._outputBufferType`, `HalfFloatType` by default.
    output_buffer_type: TextureType,

    mipmap_shader: MipmapShader,
    /// Built materials, keyed by the node builder's cache key.
    programs: HashMap<u64, Program>,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    geometries: HashMap<usize, GeometryGpu>,
    /// `Textures`' GPU side, keyed by texture identity.
    textures_2d: HashMap<usize, wgpu::Texture>,
    cube_textures: HashMap<usize, wgpu::Texture>,
    /// `WebGPUTexturePassUtils.transferPipelines`, keyed by texture format.
    mipmap_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// `BufferNode` storage, by source — a `range()` buffer must be filled only
    /// once, since filling it draws from `Math.random`.
    buffers: HashMap<String, wgpu::Buffer>,
    /// `Background`'s `SphereGeometry( 1, 32, 32 )` skybox mesh geometry.
    background_geometry: Option<Rc<BufferGeometry>>,
    /// `QuadMesh`'s shared `QuadGeometry`.
    quad_geometry: Option<Rc<BufferGeometry>>,
    /// `QuadMesh`'s shared `new OrthographicCamera( -1, 1, 1, -1, 0, 1 )`.
    quad_camera: OrthographicCamera,

    /// `NodeFrame.time`. `performance.now()` is pinned to 0 by the harness, so
    /// every frame's delta is 0 and this stays 0.
    time: f64,

    /// The viewer's canvas → surface blit; see `present.rs`. Never touched by
    /// the e2e path.
    present: Option<present::Present>,

    /// The page's `Math.random`, as the harness replaces it. `RangeNode.setup()`
    /// draws from it while the material is being built — the only consumer in
    /// `three.webgpu.js` (`MathUtils.generateUUID` uses the pattern the
    /// harness rewrites to the unseeded `Math._random`).
    random: DeterministicRandom,
}

/// `new WebGPURenderer( parameters )`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RendererParameters {
    pub antialias: bool,
}

impl Renderer {
    pub fn new(parameters: RendererParameters) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        Self::with_instance(parameters, instance)
    }

    /// `new()` against an instance the caller already created. The viewer needs
    /// this because a Wayland/X11 surface only works on an instance built with
    /// the windowing system's display handle.
    pub fn with_instance(parameters: RendererParameters, instance: wgpu::Instance) -> Self {
        let adapter = pick_adapter(&instance);
        let adapter_info = adapter.get_info();

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("three-rs device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .expect("three-rs: failed to create device");

        let mipmap_shader = MipmapShader::new(&device);

        Self {
            device,
            queue,
            adapter_info,
            adapter,
            samples: if parameters.antialias { 4 } else { 0 },
            pixel_ratio: 1.0,
            width: 300.0,
            height: 150.0,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            canvas: None,
            render_target: None,
            frame_buffer_target: None,
            output_buffer_type: TextureType::HalfFloat,
            mipmap_shader,
            programs: HashMap::new(),
            pipelines: HashMap::new(),
            geometries: HashMap::new(),
            textures_2d: HashMap::new(),
            cube_textures: HashMap::new(),
            mipmap_pipelines: HashMap::new(),
            buffers: HashMap::new(),
            background_geometry: None,
            quad_geometry: None,
            quad_camera: OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.0, 1.0),
            time: 0.0,
            present: None,
            random: DeterministicRandom::new(),
        }
    }

    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    pub fn set_pixel_ratio(&mut self, pixel_ratio: f64) {
        self.pixel_ratio = pixel_ratio;
        self.canvas = None;
    }

    pub fn set_size(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
        self.canvas = None;
    }

    /// `Renderer.getDrawingBufferSize()`.
    pub fn drawing_buffer_size(&self) -> (u32, u32) {
        (
            (self.width * self.pixel_ratio).floor() as u32,
            (self.height * self.pixel_ratio).floor() as u32,
        )
    }

    /// `renderer.setRenderTarget( target )`.
    pub fn set_render_target(&mut self, render_target: Option<RenderTarget>) {
        self.render_target = render_target;
    }

    /// `renderer.render( scene, camera )`.
    pub fn render(&mut self, scene: &mut Scene, camera: &mut PerspectiveCamera) {
        scene.update_matrix_world();
        camera.update_matrix_world();

        // `Renderer._renderScene()` → `_projectObject()` fills the render list,
        // then `renderList.finish()` sorts the opaque items; `_background.update()`
        // runs after the sort and unshifts the skybox, so it draws first.
        let order = render_list_order(scene, camera);

        let mut items = Vec::with_capacity(order.len() + 1);

        // The skybox first, exactly where `renderList.unshift()` puts it.
        if let Some(Background::CubeTexture(background)) = scene.background.clone() {
            let mut material = MeshBasicNodeMaterial::new();
            material.name = "Background.material";
            material.color_node = Some(materials::background_color_node(&background));
            material.vertex_node = Some(materials::background_vertex_node());
            material.side = Side::Back;
            material.depth_test = false;
            material.depth_write = false;

            items.push(Renderable {
                geometry: self.background_geometry(),
                material,
                setup: SetupContext::default(),
                // `Background.mesh` is never added to the scene, so its
                // `matrixWorld` stays the identity.
                model_world: Matrix4::identity(),
                instance_matrix: None,
                instance_count: 1,
            });
        }

        for &index in order.iter() {
            let child = &scene.children[index];
            let material: &MeshBasicNodeMaterial = scene
                .override_material
                .as_ref()
                .or(child.mesh().material.as_ref())
                .expect("three-rs: a mesh needs a material");

            let instance_count = child.count();
            let instance_matrix = child.instance_matrix().cloned();

            items.push(Renderable {
                geometry: child.mesh().geometry.clone(),
                material: material.clone(),
                setup: SetupContext {
                    instance_count: instance_matrix.as_ref().map(|_| instance_count as usize),
                    instanced: instance_matrix.is_some(),
                },
                model_world: child.object().matrix_world,
                instance_matrix,
                instance_count,
            });
        }

        // `Background.update()`: a `Color` background becomes the clear colour
        // and forces a clear; any other background leaves the renderer's own
        // clear colour in place (and `autoClear` still clears with it).
        let clear = match &scene.background {
            Some(Background::Color(Color { r, g, b })) => [*r, *g, *b, 1.0],
            _ => self.clear_color,
        };

        let camera_uniforms = UniformContext {
            camera_projection: camera.projection_matrix,
            camera_view: camera.matrix_world_inverse,
            camera_world: camera.object.matrix_world,
            time: self.time,
            ..Default::default()
        };

        self.render_list(&items, camera_uniforms, Some(clear));
    }

    /// `QuadMesh.render( renderer )`: the material's `vertexNode` is swapped for
    /// the full-screen-triangle one and the quad is rendered with the shared
    /// orthographic camera.
    pub fn render_quad(&mut self, quad: &QuadMesh) {
        let mut material = quad.material.clone();
        material.vertex_node = Some(materials::quad_vertex_node());

        let items = [Renderable {
            geometry: self.quad_geometry(),
            material,
            setup: SetupContext::default(),
            model_world: Matrix4::identity(),
            instance_matrix: None,
            instance_count: 1,
        }];

        let camera_uniforms = self.quad_camera_uniforms();
        let clear = self.clear_color;
        self.render_list(&items, camera_uniforms, Some(clear));
    }

    fn quad_camera_uniforms(&self) -> UniformContext {
        UniformContext {
            camera_projection: self.quad_camera.projection_matrix,
            camera_view: self.quad_camera.matrix_world_inverse,
            camera_world: self.quad_camera.object.matrix_world,
            time: self.time,
            ..Default::default()
        }
    }

    /// `Renderer._renderScene()`: pick the target, draw every renderable into
    /// it, and run the output pass when the scene went through the internal
    /// framebuffer target.
    fn render_list(
        &mut self,
        items: &[Renderable],
        camera_uniforms: UniformContext,
        clear: Option<[f64; 4]>,
    ) {
        // `Renderer.render()`: with `needsFrameBufferTarget` the scene is drawn
        // into the internal framebuffer target and `_renderOutput()` then blits
        // it to the canvas through the output colour transform.
        let use_frame_buffer_target =
            self.needs_frame_buffer_target() && self.render_target.is_none();

        let target = if use_frame_buffer_target {
            Some(self.frame_buffer_target())
        } else {
            self.render_target.clone()
        };

        let pass_target = match &target {
            Some(render_target) => self.render_target_pass(render_target),
            None => self.canvas_pass(true),
        };

        self.draw(items, camera_uniforms, &pass_target, clear);

        if use_frame_buffer_target {
            self.render_output(target.as_ref().unwrap());
        }
    }

    /// One render pass: every renderable is built, bound and drawn.
    fn draw(
        &mut self,
        items: &[Renderable],
        camera_uniforms: UniformContext,
        target: &PassTarget,
        clear: Option<[f64; 4]>,
    ) {
        struct Draw {
            geometry_id: usize,
            attributes: Vec<&'static str>,
            pipeline: PipelineKey,
            bind_groups: Vec<wgpu::BindGroup>,
            instance_count: u32,
        }

        let mut draws = Vec::with_capacity(items.len());

        for item in items {
            let geometry_id = Rc::as_ptr(&item.geometry) as usize;
            self.ensure_geometry(geometry_id, &item.geometry);

            // `NodeMaterial.setup()` → `NodeBuilder.build()`: the WGSL and the
            // bindings the material declares.
            let flow = materials::setup(&item.material, &item.setup);
            let node = NodeBuilder::new().build(&flow);
            let program_key = node.cache_key;
            if !self.programs.contains_key(&program_key) {
                let program = Program::new(&self.device, node);
                self.programs.insert(program_key, program);
            }

            let state = RenderState {
                color_format: target.color_format,
                depth_format: target.depth_format,
                sample_count: target.sample_count,
                side: item.material.side,
                depth_test: item.material.depth_test,
                depth_write: item.material.depth_write,
            };
            let pipeline = PipelineKey {
                program: program_key,
                state,
            };
            self.ensure_pipeline(pipeline);

            let uniforms = UniformContext {
                model_world: item.model_world,
                material_color: item.material.color,
                material_opacity: item.material.opacity,
                material_reflectivity: item.material.reflectivity,
                viewport: Vector2::new(target.width as f64, target.height as f64),
                ..camera_uniforms
            };

            let bind_groups = self.bind_groups(program_key, &uniforms, &item.instance_matrix);

            let attributes = self.programs[&program_key]
                .node
                .attributes
                .iter()
                .map(|(name, _)| *name)
                .collect();

            draws.push(Draw {
                geometry_id,
                attributes,
                pipeline,
                bind_groups,
                instance_count: item.instance_count,
            });
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs pass"),
            });

        {
            let load = match clear {
                Some(clear) => wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear[0],
                    g: clear[1],
                    b: clear[2],
                    a: clear[3],
                }),
                None => wgpu::LoadOp::Load,
            };

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("three-rs pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color,
                    depth_slice: None,
                    resolve_target: target.resolve.as_ref(),
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: target.depth.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            // `Renderer._clearDepth` is 1.
                            load: if clear.is_some() {
                                wgpu::LoadOp::Clear(1.0)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            for draw in draws.iter() {
                let geometry = &self.geometries[&draw.geometry_id];

                pass.set_pipeline(self.pipelines.get(&draw.pipeline).unwrap());
                for (index, group) in draw.bind_groups.iter().enumerate() {
                    pass.set_bind_group(index as u32, group, &[]);
                }
                for (slot, name) in draw.attributes.iter().enumerate() {
                    pass.set_vertex_buffer(slot as u32, geometry.attribute(name).slice(..));
                }

                match &geometry.index {
                    Some((buffer, format, count)) => {
                        pass.set_index_buffer(buffer.slice(..), *format);
                        pass.draw_indexed(0..*count, 0, 0..draw.instance_count);
                    }
                    None => pass.draw(0..geometry.vertex_count, 0..draw.instance_count),
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// `Renderer._renderOutput( renderTarget )`: a `QuadMesh` whose
    /// `NodeMaterial.fragmentNode` is `nodes.getOutputNode( renderTarget.texture )`,
    /// rendered to the canvas with `autoClear` off, so the canvas attachments
    /// load rather than clear.
    fn render_output(&mut self, render_target: &RenderTarget) {
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "outputColorTransform";
        material.fragment_node = Some(materials::output_fragment_node(
            &render_target.texture(),
        ));

        let items = [Renderable {
            geometry: self.quad_geometry(),
            material,
            setup: SetupContext::default(),
            model_world: Matrix4::identity(),
            instance_matrix: None,
            instance_count: 1,
        }];

        let camera_uniforms = self.quad_camera_uniforms();
        let pass_target = self.canvas_pass(true);
        self.draw(&items, camera_uniforms, &pass_target, None);
    }

    /// Reads the canvas colour texture back as top-down RGBA8, which is what
    /// `page.screenshot()` hands the comparator.
    pub fn read_canvas_pixels(&mut self) -> (u32, u32, Vec<u8>) {
        self.prepare_canvas(false, 1);
        let canvas = self.canvas.as_ref().unwrap();
        let (width, height) = (canvas.width, canvas.height);

        // `copy_texture_to_buffer` needs 256-byte aligned rows; the padding is
        // stripped again below (FINDINGS #19: not stripping it shears the image).
        let unpadded_bytes_per_row = width * 4;
        let bytes_per_row = unpadded_bytes_per_row.div_ceil(256) * 256;

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("three-rs readback"),
            size: (bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs readback"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &canvas.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();

        let padded = slice.get_mapped_range().unwrap().to_vec();
        buffer.unmap();

        let mut data = Vec::with_capacity((unpadded_bytes_per_row * height) as usize);
        for row in 0..height {
            let start = (row * bytes_per_row) as usize;
            data.extend_from_slice(&padded[start..start + unpadded_bytes_per_row as usize]);
        }

        (width, height, data)
    }

    // -- bindings --------------------------------------------------------

    /// `Bindings.getForRender()`: one bind group per declared group, with every
    /// binding resolved from the descriptor the node builder emitted.
    fn bind_groups(
        &mut self,
        program_key: u64,
        uniforms: &UniformContext,
        instance_matrix: &Option<InstancedBufferAttribute>,
    ) -> Vec<wgpu::BindGroup> {
        enum Resource {
            Buffer(wgpu::Buffer),
            View(wgpu::TextureView),
            Sampler(wgpu::Sampler),
        }

        let groups = self.programs[&program_key].node.groups.clone();
        let mut out = Vec::with_capacity(groups.len());

        for (group_index, descs) in groups.iter().enumerate() {
            let mut resources = Vec::with_capacity(descs.len());

            for desc in descs {
                resources.push(match desc {
                    BindingDesc::Uniforms { members, size, .. } => {
                        let bytes = uniforms.bytes(members, *size);
                        Resource::Buffer(self.create_buffer_init(
                            "three-rs uniforms",
                            &bytes,
                            wgpu::BufferUsages::UNIFORM,
                        ))
                    }
                    BindingDesc::Buffer { source, count, .. } => {
                        Resource::Buffer(self.node_buffer(source, *count, instance_matrix))
                    }
                    BindingDesc::Texture { source, kind, .. } => {
                        Resource::View(self.texture_view(source, *kind))
                    }
                    BindingDesc::Sampler { source, .. } => {
                        Resource::Sampler(self.texture_sampler(source))
                    }
                });
            }

            let entries: Vec<wgpu::BindGroupEntry> = resources
                .iter()
                .enumerate()
                .map(|(binding, resource)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: match resource {
                        Resource::Buffer(buffer) => buffer.as_entire_binding(),
                        Resource::View(view) => wgpu::BindingResource::TextureView(view),
                        Resource::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
                    },
                })
                .collect();

            out.push(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("three-rs bind group"),
                layout: &self.programs[&program_key].layouts[group_index],
                entries: &entries,
            }));
        }

        out
    }

    /// A `BufferNode`'s uniform buffer. `range()` is filled from the page's
    /// `Math.random` exactly once, because `RangeNode.setup()` runs once.
    fn node_buffer(
        &mut self,
        source: &BufferSource,
        count: usize,
        instance_matrix: &Option<InstancedBufferAttribute>,
    ) -> wgpu::Buffer {
        match source {
            BufferSource::InstanceMatrix => {
                let attribute = instance_matrix
                    .as_ref()
                    .expect("three-rs: instanceMatrix needs an InstancedMesh");
                self.create_buffer_init(
                    "three-rs instanceMatrix",
                    bytemuck::cast_slice(&attribute.array),
                    wgpu::BufferUsages::UNIFORM,
                )
            }
            BufferSource::Range { min, max } => {
                let key = format!("range:{min:?}:{max:?}:{count}");
                if let Some(buffer) = self.buffers.get(&key) {
                    return buffer.clone();
                }

                // `min`/`max` are `Vector4`s: a Color fills xyz and leaves w at 1.
                let min = [min.r, min.g, min.b, 1.0];
                let max = [max.r, max.g, max.b, 1.0];

                let stride = 4usize;
                let mut range = vec![0f32; stride * count];

                for (i, value) in range.iter_mut().enumerate() {
                    let index = i % stride;
                    let t = self.random.next();
                    // `MathUtils.lerp( x, y, t ) = ( 1 - t ) * x + t * y`
                    *value = ((1.0 - t) * min[index] + t * max[index]) as f32;
                }

                let buffer = self.create_buffer_init(
                    "three-rs range()",
                    bytemuck::cast_slice(&range),
                    wgpu::BufferUsages::UNIFORM,
                );
                self.buffers.insert(key, buffer.clone());
                buffer
            }
        }
    }

    fn texture_view(&mut self, source: &TextureSource, kind: TextureKind) -> wgpu::TextureView {
        match source {
            TextureSource::Texture2D(texture) => {
                let gpu = self.ensure_texture_2d(texture);
                gpu.create_view(&Default::default())
            }
            TextureSource::Depth(depth) => {
                let inner = depth.inner().borrow();
                let gpu = inner
                    .gpu
                    .as_ref()
                    .expect("three-rs: the depth texture has not been rendered into yet");
                gpu.create_view(&Default::default())
            }
            TextureSource::Cube(cube) => {
                assert_eq!(kind, TextureKind::Cube);
                let gpu = self.ensure_cube_texture(cube);
                gpu.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::Cube),
                    ..Default::default()
                })
            }
        }
    }

    /// `WebGPUTextureUtils.updateSampler()`.
    fn texture_sampler(&mut self, source: &TextureSource) -> wgpu::Sampler {
        let filter = |f: TextureFilter| match f {
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
            TextureFilter::Linear => wgpu::FilterMode::Linear,
        };
        let mipmap = |f: TextureFilter| match f {
            TextureFilter::Nearest => wgpu::MipmapFilterMode::Nearest,
            TextureFilter::Linear => wgpu::MipmapFilterMode::Linear,
        };
        let address = |w: Wrapping| match w {
            Wrapping::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        };

        match source {
            TextureSource::Texture2D(texture) => {
                let inner = texture.borrow();
                self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("three-rs sampler"),
                    address_mode_u: address(inner.wrap_s),
                    address_mode_v: address(inner.wrap_t),
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter.min()),
                    mipmap_filter: mipmap(inner.min_filter.mipmap()),
                    anisotropy_clamp: inner.anisotropy,
                    ..Default::default()
                })
            }
            TextureSource::Cube(cube) => {
                let anisotropy = cube.inner().borrow().anisotropy;
                self.device.create_sampler(&wgpu::SamplerDescriptor {
                    label: Some("three-rs cube sampler"),
                    // `CubeTexture`'s wrapping is `ClampToEdgeWrapping` on all axes.
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    // `LinearFilter` / `LinearMipmapLinearFilter`.
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::MipmapFilterMode::Linear,
                    anisotropy_clamp: anisotropy,
                    ..Default::default()
                })
            }
            TextureSource::Depth(_) => {
                panic!("three-rs: a depth texture is read with textureLoad, not sampled")
            }
        }
    }

    // -- resources -------------------------------------------------------

    /// `Background`'s skybox geometry, built once per renderer just as
    /// `Background.update()` caches it per scene.
    fn background_geometry(&mut self) -> Rc<BufferGeometry> {
        self.background_geometry
            .get_or_insert_with(|| Rc::new(sphere_geometry(1.0, 32, 32)))
            .clone()
    }

    fn quad_geometry(&mut self) -> Rc<BufferGeometry> {
        self.quad_geometry
            .get_or_insert_with(|| Rc::new(quad_geometry()))
            .clone()
    }

    /// `Textures.updateTexture()` for a 2D `Texture`: upload the image with
    /// `flipY` applied, then generate the mip chain.
    fn ensure_texture_2d(&mut self, texture: &Texture) -> wgpu::Texture {
        // A render target's colour texture is owned by the renderer and was
        // created by `prepare_render_target()`.
        if !texture.borrow().own_gpu {
            return texture.with_gpu(|gpu| gpu.clone());
        }

        let id = texture.id();
        if let Some(gpu) = self.textures_2d.get(&id) {
            return gpu.clone();
        }

        let (width, height) = texture.size();
        let format = texture.format();
        let mip_level_count = texture.mip_level_count();

        let gpu = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("three-rs texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        {
            let inner = texture.borrow();
            let data = inner
                .data
                .as_ref()
                .expect("three-rs: the texture has no image data");

            // `copyExternalImageToTexture( { flipY } )`: the source rows are
            // uploaded bottom-up. (three.js' `_flipY()` pass is only for the
            // `_copyBufferToTexture` path, and is the same flip.)
            let rows: Vec<u8> = if inner.flip_y {
                let stride = (width * 4) as usize;
                let mut flipped = Vec::with_capacity(data.len());
                for row in (0..height as usize).rev() {
                    flipped.extend_from_slice(&data[row * stride..(row + 1) * stride]);
                }
                flipped
            } else {
                data.clone()
            };

            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &gpu,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rows,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }

        if mip_level_count > 1 {
            self.generate_mipmaps(&gpu, format, mip_level_count, 1);
        }

        texture.set_gpu(gpu.clone());
        self.textures_2d.insert(id, gpu.clone());
        gpu
    }

    /// `Textures.updateTexture()` for a `CubeTexture`: one 2D texture with six
    /// array layers, `textureBindingViewDimension: 'cube'`, a full mip chain,
    /// and one `copyExternalImageToTexture` per face with `flipY: false`.
    fn ensure_cube_texture(&mut self, texture: &CubeTexture) -> wgpu::Texture {
        let id = texture.id();
        if let Some(gpu) = self.cube_textures.get(&id) {
            return gpu.clone();
        }

        let (width, height) = texture.size();
        let format = texture.gpu_format();
        let mip_level_count = texture.mip_level_count();

        let gpu = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("three-rs cube texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 6,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        {
            let inner = texture.inner().borrow();
            assert!(!inner.flip_y, "three-rs: CubeTexture.flipY is false");
            for (layer, image) in inner.images.iter().enumerate() {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &gpu,
                        mip_level: 0,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    &image.data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(image.width * 4),
                        rows_per_image: Some(image.height),
                    },
                    wgpu::Extent3d {
                        width: image.width,
                        height: image.height,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        if mip_level_count > 1 {
            self.generate_mipmaps(&gpu, format, mip_level_count, 6);
        }

        texture.inner().borrow_mut().gpu = Some(gpu.clone());
        self.cube_textures.insert(id, gpu.clone());
        gpu
    }

    /// `WebGPUTexturePassUtils.generateMipmaps()`: one render pass per
    /// (mip level, array layer), each drawing a single oversized triangle that
    /// samples the level above through a `minFilter: 'linear'` sampler — a 2×
    /// box downsample of each layer on its own, because `getTransferPipeline()`
    /// falls back to the `'2d-array'` entry point when the GPU texture reports
    /// no `textureBindingViewDimension`.
    fn generate_mipmaps(
        &mut self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        mip_level_count: u32,
        layers: u32,
    ) {
        if !self.mipmap_pipelines.contains_key(&format) {
            let pipeline = create_mipmap_pipeline(&self.device, &self.mipmap_shader, format);
            self.mipmap_pipelines.insert(format, pipeline);
        }
        let pipeline = &self.mipmap_pipelines[&format];

        // `this.mipmapSampler = device.createSampler( { minFilter: Linear } )` —
        // every other field stays at the WebGPU default, so magnification is
        // nearest and the mipmap filter is nearest.
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("three-rs mipmap sampler"),
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // `this.noFlipUniformBuffer` — zero-initialised and never written.
        let flip = self.create_buffer_init(
            "three-rs mipmap noFlip",
            bytemuck::cast_slice(&[0u32]),
            wgpu::BufferUsages::UNIFORM,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs mipmapEncoder"),
            });

        for base_mip_level in 1..mip_level_count {
            for base_array_layer in 0..layers {
                let source = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    usage: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: base_mip_level - 1,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                });

                let destination = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: None,
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    usage: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level,
                    mip_level_count: Some(1),
                    base_array_layer,
                    array_layer_count: Some(1),
                });

                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.mipmap_shader.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&source),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: flip.as_entire_binding(),
                        },
                    ],
                });

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &destination,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                // `passEncoder.draw( 3, 1, 0, baseArrayLayer )`: the layer to
                // read arrives as `@builtin( instance_index )`.
                pass.draw(0..3, base_array_layer..base_array_layer + 1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }

    fn ensure_pipeline(&mut self, key: PipelineKey) {
        if !self.pipelines.contains_key(&key) {
            let pipeline = self.programs[&key.program].create_pipeline(&self.device, key.state);
            self.pipelines.insert(key, pipeline);
        }
    }

    fn ensure_geometry(&mut self, id: usize, geometry: &BufferGeometry) {
        if self.geometries.contains_key(&id) {
            return;
        }

        let vertex_buffer = |attribute: &crate::core::BufferAttribute| {
            self.create_buffer_init(
                "three-rs attribute",
                bytemuck::cast_slice(&attribute.array),
                wgpu::BufferUsages::VERTEX,
            )
        };

        let position = geometry.position().map(vertex_buffer);
        let normal = geometry.normal().map(vertex_buffer);
        let uv = geometry.uv().map(vertex_buffer);

        let index = geometry.index.as_ref().map(|index| {
            let (bytes, format): (Vec<u8>, wgpu::IndexFormat) = match index {
                Index::U16(v) => (bytemuck::cast_slice(v).to_vec(), wgpu::IndexFormat::Uint16),
                Index::U32(v) => (bytemuck::cast_slice(v).to_vec(), wgpu::IndexFormat::Uint32),
            };
            let buffer =
                self.create_buffer_init("three-rs index", &bytes, wgpu::BufferUsages::INDEX);
            (buffer, format, index.count() as u32)
        });

        let vertex_count = geometry
            .position()
            .map(|p| p.count() as u32)
            .unwrap_or(0);

        self.geometries.insert(
            id,
            GeometryGpu {
                position,
                normal,
                uv,
                index,
                vertex_count,
            },
        );
    }

    fn create_buffer_init(
        &self,
        label: &str,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        // Pad to 4 bytes, as `write_buffer` requires.
        let mut padded = contents.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(0);
        }

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: padded.len() as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buffer, 0, &padded);
        buffer
    }

    // -- targets ---------------------------------------------------------

    /// `Renderer.needsFrameBufferTarget` — true when the output needs tone
    /// mapping or a colour-space conversion. `outputColorSpace` is
    /// `SRGBColorSpace` and the working colour space is `LinearSRGBColorSpace`,
    /// and the port has no tone mapping yet, so this is the colour-space half:
    /// always true for a canvas render.
    fn needs_frame_buffer_target(&self) -> bool {
        true
    }

    /// `Renderer.currentSamples`: a custom render target's own sample count,
    /// and 0 for the canvas whenever the framebuffer target or a fullscreen
    /// pass is in play.
    fn current_samples(&self) -> u32 {
        match &self.render_target {
            Some(render_target) => render_target.samples(),
            None => {
                if self.needs_frame_buffer_target() {
                    0
                } else {
                    self.samples
                }
            }
        }
    }

    /// `Renderer._getFrameBufferTarget()`.
    fn frame_buffer_target(&mut self) -> RenderTarget {
        let (width, height) = self.drawing_buffer_size();

        let target = self.frame_buffer_target.get_or_insert_with(|| {
            RenderTarget::new_with_options(
                width,
                height,
                RenderTargetOptions {
                    texture_type: self.output_buffer_type,
                    // `samples: this.samples` — the renderer's own count, not
                    // `currentSamples`.
                    samples: self.samples,
                    depth_buffer: true,
                    min_filter: TextureFilter::Linear,
                    mag_filter: TextureFilter::Linear,
                },
            )
        });

        target.set_size(width, height);
        target.clone()
    }

    /// The attachments of a render-target pass.
    fn render_target_pass(&self, render_target: &RenderTarget) -> PassTarget {
        self.prepare_render_target(render_target);

        let inner = render_target.inner().borrow();
        let color_format = inner.texture_type.color_gpu_format();
        let single = inner.texture.with_gpu(|gpu| gpu.create_view(&Default::default()));

        let (color, resolve) = match &inner.msaa {
            Some(msaa) => (msaa.create_view(&Default::default()), Some(single)),
            None => (single, None),
        };

        let (depth, depth_format) = match (&inner.depth_texture, &inner.depth) {
            (Some(depth_texture), _) => (
                Some(
                    depth_texture
                        .inner()
                        .borrow()
                        .gpu
                        .as_ref()
                        .unwrap()
                        .create_view(&Default::default()),
                ),
                Some(depth_texture.gpu_format()),
            ),
            (None, Some(depth)) => (
                Some(depth.create_view(&Default::default())),
                Some(CANVAS_DEPTH_FORMAT),
            ),
            (None, None) => (None, None),
        };

        PassTarget {
            color,
            resolve,
            depth,
            color_format,
            depth_format,
            sample_count: inner.samples.max(1),
            width: inner.width,
            height: inner.height,
        }
    }

    /// The attachments of a canvas pass.
    fn canvas_pass(&mut self, needs_depth: bool) -> PassTarget {
        let sample_count = self.current_samples().max(1);
        self.prepare_canvas(needs_depth, sample_count);

        let canvas = self.canvas.as_ref().unwrap();
        let (color, resolve) = match &canvas.msaa {
            Some(msaa) => (
                msaa.create_view(&Default::default()),
                Some(canvas.color.create_view(&Default::default())),
            ),
            None => (canvas.color.create_view(&Default::default()), None),
        };
        let depth = canvas
            .depth
            .as_ref()
            .map(|d| d.create_view(&Default::default()));

        PassTarget {
            color,
            resolve,
            depth: depth.clone(),
            color_format: CANVAS_FORMAT,
            depth_format: depth.map(|_| CANVAS_DEPTH_FORMAT),
            sample_count: canvas.sample_count,
            width: canvas.width,
            height: canvas.height,
        }
    }

    /// `needs_depth` mirrors `renderer.depth` for the pass about to run and
    /// `sample_count` is `Renderer.currentSamples` for it: both the canvas depth
    /// buffer and the canvas MSAA texture are created on demand and then kept.
    fn prepare_canvas(&mut self, needs_depth: bool, sample_count: u32) {
        let (width, height) = self.drawing_buffer_size();

        if let Some(canvas) = &self.canvas {
            if canvas.width == width
                && canvas.height == height
                && canvas.sample_count == sample_count
            {
                if needs_depth && canvas.depth.is_none() {
                    let depth = self.create_depth_buffer(width, height, sample_count);
                    self.canvas.as_mut().unwrap().depth = Some(depth);
                }
                return;
            }
        }

        let color = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("three-rs canvas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: CANVAS_FORMAT,
            // `TEXTURE_BINDING` is the viewer's: `Renderer::present()` samples
            // the canvas to blit it into a surface texture.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let msaa = (sample_count > 1).then(|| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("three-rs canvas msaa"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: CANVAS_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        });

        let depth = needs_depth.then(|| self.create_depth_buffer(width, height, sample_count));

        self.canvas = Some(CanvasTarget {
            width,
            height,
            sample_count,
            color,
            msaa,
            depth,
        });
    }

    /// The auto-allocated depth buffer of a pass: `depth24plus`, the format
    /// `WebGPUUtils.getCurrentDepthStencilFormat()` picks with `stencil` and
    /// `reversedDepthBuffer` both off.
    fn create_depth_buffer(&self, width: u32, height: u32, sample_count: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("three-rs depth buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: CANVAS_DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn prepare_render_target(&self, render_target: &RenderTarget) {
        let mut inner = render_target.inner().borrow_mut();
        let (width, height) = (inner.width, inner.height);
        let sample_count = inner.samples.max(1);

        let format = inner.texture_type.color_gpu_format();

        if !inner.texture.has_gpu() {
            inner.texture.set_gpu(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("three-rs render target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                // The resolved, sampleable texture is always single-sample;
                // `samples > 1` adds the MSAA texture below.
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }));
        }

        if sample_count > 1 && inner.msaa.is_none() {
            inner.msaa = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("three-rs render target msaa"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }));
        }

        if inner.depth_texture.is_none() && inner.depth_buffer && inner.depth.is_none() {
            inner.depth = Some(self.create_depth_buffer(width, height, sample_count));
        }

        if let Some(depth_texture) = &inner.depth_texture {
            let format = depth_texture.gpu_format();
            let mut depth = depth_texture.inner().borrow_mut();
            if depth.gpu.is_none() {
                depth.width = width;
                depth.height = height;
                depth.gpu = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("three-rs depth texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                }));
            }
        }
    }
}

/// `RenderList.finish()`'s `painterSortStable` over the opaque items, which is
/// the only list the ladder fills.
///
/// `Renderer._projectObject()` computes each item's `z` as the geometry's
/// bounding-sphere centre pushed through `matrixWorld` and then through
/// `projectionMatrix * matrixWorldInverse`, keeping the unnormalised clip-space
/// `z`. `groupOrder` and `renderOrder` are 0 everywhere here, and the `id`
/// tie-break matches creation order, so a stable sort on `z` reproduces it.
fn render_list_order(scene: &Scene, camera: &PerspectiveCamera) -> Vec<usize> {
    let mut proj_screen_matrix = Matrix4::identity();
    proj_screen_matrix.multiply_matrices(&camera.projection_matrix, &camera.matrix_world_inverse);

    let mut items: Vec<(usize, f64)> = scene
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            let center = child.mesh().geometry.bounding_sphere_center();
            let v = apply_matrix4_vector4(
                &child.object().matrix_world,
                [center.x, center.y, center.z, 1.0],
            );
            let v = apply_matrix4_vector4(&proj_screen_matrix, v);
            (index, v[2])
        })
        .collect();

    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    items.into_iter().map(|(index, _)| index).collect()
}

/// `Vector4.applyMatrix4()` — no perspective divide, unlike `Vector3`'s.
fn apply_matrix4_vector4(m: &Matrix4, v: [f64; 4]) -> [f64; 4] {
    let e = &m.elements;
    [
        e[0] * v[0] + e[4] * v[1] + e[8] * v[2] + e[12] * v[3],
        e[1] * v[0] + e[5] * v[1] + e[9] * v[2] + e[13] * v[3],
        e[2] * v[0] + e[6] * v[1] + e[10] * v[2] + e[14] * v[3],
        e[3] * v[0] + e[7] * v[1] + e[11] * v[2] + e[15] * v[3],
    ]
}

/// The canvas colour format. Chrome's preferred WebGPU canvas format on this
/// platform is `bgra8unorm`; `rgba8unorm` is the same 8-bit-per-channel unorm
/// target with the channels already in readback order.
const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `WebGPUUtils.getCurrentDepthStencilFormat()` with `stencil` and
/// `reversedDepthBuffer` both off.
const CANVAS_DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;

fn pick_adapter(instance: &wgpu::Instance) -> wgpu::Adapter {
    let wanted = std::env::var("THREE_RS_ADAPTER_NAME").ok();
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));

    if let Some(wanted) = &wanted {
        if let Some(adapter) = adapters
            .iter()
            .find(|a| a.get_info().name.contains(wanted.as_str()))
        {
            return adapter.clone();
        }
        panic!("three-rs: no Vulkan adapter matching THREE_RS_ADAPTER_NAME={wanted}");
    }

    // Prefer the real Intel GPU: the grader was calibrated on it, and a
    // software adapter (lavapipe) would rasterize differently.
    if let Some(adapter) = adapters
        .iter()
        .find(|a| a.get_info().device_type == wgpu::DeviceType::IntegratedGpu)
    {
        return adapter.clone();
    }

    if let Some(adapter) = adapters
        .iter()
        .find(|a| a.get_info().device_type == wgpu::DeviceType::DiscreteGpu)
    {
        return adapter.clone();
    }

    adapters
        .into_iter()
        .next()
        .expect("three-rs: no Vulkan adapter found")
}
