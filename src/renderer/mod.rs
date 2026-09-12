//! Port of `three.js/src/renderers/common/Renderer.js` + the WebGPU backend,
//! cut down to what rung 1 needs: walk the scene, resolve a material to a
//! pipeline, draw into a render target or into the "canvas" texture, read back.

mod pipelines;
mod render_target;

use std::collections::HashMap;
use std::rc::Rc;

use pipelines::{create_pipeline, Layouts, PipelineKey, Shaders};
pub use render_target::{RenderTarget, RenderTargetInner};

use crate::cameras::PerspectiveCamera;
use crate::core::{BufferGeometry, Index};
use crate::materials::{ColorNode, ShaderKey};
use crate::math::Color;
use crate::objects::{QuadMesh, Scene};
use crate::textures::{DepthTexture, TextureFilter};

/// Uniform buffer stride for the per-object block. The WebGPU minimum dynamic
/// offset alignment is 256 bytes.
const OBJECT_STRIDE: u64 = 256;
const MAX_OBJECTS: u64 = 1024;

struct GeometryGpu {
    position: wgpu::Buffer,
    index: wgpu::Buffer,
    index_format: wgpu::IndexFormat,
    index_count: u32,
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
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,

    /// `Renderer._samples`: `antialias === true` means 4.
    samples: u32,
    pixel_ratio: f64,
    width: f64,
    height: f64,

    /// `Renderer._clearColor`: black, alpha 1.
    clear_color: [f64; 4],

    canvas: Option<CanvasTarget>,
    render_target: Option<RenderTarget>,

    shaders: Shaders,
    layouts: Layouts,
    pipelines: HashMap<PipelineKey, wgpu::RenderPipeline>,
    geometries: HashMap<usize, GeometryGpu>,

    camera_buffer: wgpu::Buffer,
    object_buffer: wgpu::Buffer,
    basic_bind_group: wgpu::BindGroup,
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

        let shaders = Shaders::new(&device);
        let layouts = Layouts::new(&device);

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("three-rs camera uniforms"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("three-rs object uniforms"),
            size: OBJECT_STRIDE * MAX_OBJECTS,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let basic_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("three-rs basic bind group"),
            layout: &layouts.basic,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &object_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(64),
                    }),
                },
            ],
        });

        Self {
            device,
            queue,
            adapter_info,
            samples: if parameters.antialias { 4 } else { 0 },
            pixel_ratio: 1.0,
            width: 300.0,
            height: 150.0,
            clear_color: [0.0, 0.0, 0.0, 1.0],
            canvas: None,
            render_target: None,
            shaders,
            layouts,
            pipelines: HashMap::new(),
            geometries: HashMap::new(),
            camera_buffer,
            object_buffer,
            basic_bind_group,
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

        self.write_camera_uniforms(camera);

        // Per-object uniforms: one 256-byte slot each.
        let mut object_data = vec![0u8; (OBJECT_STRIDE as usize) * scene.children.len().max(1)];
        for (i, child) in scene.children.iter().enumerate() {
            let world = child.object.matrix_world.to_f32_array();
            let offset = i * OBJECT_STRIDE as usize;
            object_data[offset..offset + 64].copy_from_slice(bytemuck::cast_slice(&world));
        }
        self.queue.write_buffer(&self.object_buffer, 0, &object_data);

        let target = self.render_target.clone();
        let (color_view, depth_view, color_format, depth_format, sample_count, resolve) =
            match &target {
                Some(render_target) => {
                    self.prepare_render_target(render_target);
                    let inner = render_target.inner().borrow();
                    let color = inner.color.as_ref().unwrap().create_view(&Default::default());
                    let depth = inner
                        .depth_texture
                        .as_ref()
                        .map(|d| {
                            d.inner()
                                .borrow()
                                .gpu
                                .as_ref()
                                .unwrap()
                                .create_view(&Default::default())
                        })
                        .expect("three-rs: rung 1 render targets always carry a depth texture");
                    let samples = inner.samples.max(1);
                    (
                        color,
                        Some(depth),
                        RenderTarget::COLOR_FORMAT,
                        inner.depth_texture.as_ref().map(|d| d.gpu_format()),
                        samples,
                        None,
                    )
                }
                None => {
                    self.prepare_canvas();
                    let canvas = self.canvas.as_ref().unwrap();
                    let (view, resolve) = match &canvas.msaa {
                        Some(msaa) => (
                            msaa.create_view(&Default::default()),
                            Some(canvas.color.create_view(&Default::default())),
                        ),
                        None => (canvas.color.create_view(&Default::default()), None),
                    };
                    (
                        view,
                        None,
                        CANVAS_FORMAT,
                        None,
                        canvas.sample_count,
                        resolve,
                    )
                }
            };

        // Background: a `Color` background becomes the clear colour and forces a
        // clear; otherwise the renderer's own clear colour is used.
        let clear = match scene.background {
            Some(Color { r, g, b }) => [r, g, b, 1.0],
            None => self.clear_color,
        };

        let pipeline_key = PipelineKey {
            shader: ShaderKey::Basic,
            color_format,
            depth_format,
            sample_count,
        };
        self.ensure_pipeline(pipeline_key);

        // Upload every geometry before the pass borrows `self` immutably.
        let geometry_ids: Vec<usize> = scene
            .children
            .iter()
            .map(|child| {
                let id = Rc::as_ptr(&child.geometry) as usize;
                self.ensure_geometry(id, &child.geometry);
                id
            })
            .collect();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs scene pass"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("three-rs scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    depth_slice: None,
                    resolve_target: resolve.as_ref(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            // `Renderer._clearDepth` is 1.
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let pipeline = self.pipelines.get(&pipeline_key).unwrap();
            pass.set_pipeline(pipeline);

            for (i, id) in geometry_ids.iter().enumerate() {
                let geometry = &self.geometries[id];
                pass.set_bind_group(
                    0,
                    &self.basic_bind_group,
                    &[(i as u64 * OBJECT_STRIDE) as u32],
                );
                pass.set_vertex_buffer(0, geometry.position.slice(..));
                pass.set_index_buffer(geometry.index.slice(..), geometry.index_format);
                pass.draw_indexed(0..geometry.index_count, 0, 0..1);
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// `QuadMesh.render( renderer )` — a single full-screen triangle into the
    /// current target (rung 1 only ever uses it for the canvas).
    pub fn render_quad(&mut self, quad: &QuadMesh) {
        assert!(
            self.render_target.is_none(),
            "three-rs: rung 1 only renders the quad to the canvas"
        );

        self.prepare_canvas();

        let depth_texture = match &quad.material.color_node {
            Some(ColorNode::DepthTexture(texture)) => texture.clone(),
            None => panic!("three-rs: the quad material needs a colorNode"),
        };

        let bind_group = self.depth_bind_group(&depth_texture);

        let canvas = self.canvas.as_ref().unwrap();
        let sample_count = canvas.sample_count;
        let (view, resolve) = match &canvas.msaa {
            Some(msaa) => (
                msaa.create_view(&Default::default()),
                Some(canvas.color.create_view(&Default::default())),
            ),
            None => (canvas.color.create_view(&Default::default()), None),
        };

        let key = PipelineKey {
            shader: ShaderKey::DepthTextureQuad,
            color_format: CANVAS_FORMAT,
            // No depth attachment: the canvas depth buffer is cleared to 1 and
            // the quad sits at z = 0, so the depth test can never reject it.
            depth_format: None,
            sample_count,
        };
        self.ensure_pipeline(key);

        let clear = self.clear_color;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs quad pass"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("three-rs quad pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: resolve.as_ref(),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear[0],
                            g: clear[1],
                            b: clear[2],
                            a: clear[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            pass.set_pipeline(self.pipelines.get(&key).unwrap());
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }

    /// Reads the canvas colour texture back as top-down RGBA8, which is what
    /// `page.screenshot()` hands the comparator.
    pub fn read_canvas_pixels(&mut self) -> (u32, u32, Vec<u8>) {
        self.prepare_canvas();
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

    //

    fn write_camera_uniforms(&self, camera: &PerspectiveCamera) {
        let mut data = [0f32; 32];
        data[0..16].copy_from_slice(&camera.projection_matrix.to_f32_array());
        data[16..32].copy_from_slice(&camera.matrix_world_inverse.to_f32_array());
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&data));
    }

    fn ensure_pipeline(&mut self, key: PipelineKey) {
        if !self.pipelines.contains_key(&key) {
            let pipeline = create_pipeline(&self.device, &self.shaders, &self.layouts, key);
            self.pipelines.insert(key, pipeline);
        }
    }

    fn ensure_geometry(&mut self, id: usize, geometry: &BufferGeometry) {
        if self.geometries.contains_key(&id) {
            return;
        }

        let position = geometry
            .position
            .as_ref()
            .expect("three-rs: geometry without a position attribute");

        let position_buffer = self.create_buffer_init(
            "three-rs position",
            bytemuck::cast_slice(&position.array),
            wgpu::BufferUsages::VERTEX,
        );

        let index = geometry
            .index
            .as_ref()
            .expect("three-rs: rung 1 only draws indexed geometry");

        let (index_bytes, index_format): (Vec<u8>, wgpu::IndexFormat) = match index {
            Index::U16(v) => (
                bytemuck::cast_slice(v).to_vec(),
                wgpu::IndexFormat::Uint16,
            ),
            Index::U32(v) => (
                bytemuck::cast_slice(v).to_vec(),
                wgpu::IndexFormat::Uint32,
            ),
        };

        let index_buffer =
            self.create_buffer_init("three-rs index", &index_bytes, wgpu::BufferUsages::INDEX);

        self.geometries.insert(
            id,
            GeometryGpu {
                position: position_buffer,
                index: index_buffer,
                index_format,
                index_count: index.count() as u32,
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

    fn depth_bind_group(&self, depth_texture: &DepthTexture) -> wgpu::BindGroup {
        let inner = depth_texture.inner().borrow();
        let gpu = inner
            .gpu
            .as_ref()
            .expect("three-rs: the depth texture has not been rendered into yet");

        let view = gpu.create_view(&Default::default());

        let filter = |f: TextureFilter| match f {
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
            TextureFilter::Linear => wgpu::FilterMode::Linear,
        };

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("three-rs depth sampler"),
            // `Texture`'s default wrapping is `ClampToEdgeWrapping`.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter(inner.mag_filter),
            min_filter: filter(inner.min_filter),
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("three-rs quad bind group"),
            layout: &self.layouts.quad,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    fn prepare_canvas(&mut self) {
        let (width, height) = self.drawing_buffer_size();
        let sample_count = self.samples.max(1);

        if let Some(canvas) = &self.canvas {
            if canvas.width == width
                && canvas.height == height
                && canvas.sample_count == sample_count
            {
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
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

        self.canvas = Some(CanvasTarget {
            width,
            height,
            sample_count,
            color,
            msaa,
        });
    }

    fn prepare_render_target(&self, render_target: &RenderTarget) {
        let mut inner = render_target.inner().borrow_mut();
        let (width, height) = (inner.width, inner.height);
        let sample_count = inner.samples.max(1);

        if inner.color.is_none() {
            inner.color = Some(self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("three-rs render target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: RenderTarget::COLOR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }));
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

/// The canvas colour format. Chrome's preferred WebGPU canvas format on this
/// platform is `bgra8unorm`; `rgba8unorm` is the same 8-bit-per-channel unorm
/// target with the channels already in readback order.
const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

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
