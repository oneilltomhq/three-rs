//! Additive seam for the interactive viewer (`src/bin/viewer.rs`).
//!
//! Nothing in here is part of the three.js port: it is the minimum the viewer
//! needs to drive the renderer against a window instead of a readback buffer.
//! Kept in its own file so that the node-system rewrite of `src/renderer/` can
//! be rebased past it — the only hooks into `mod.rs` are the `mod present;`
//! line, the `present` cache field, and `Renderer::with_instance()`.

use super::{Renderer, CANVAS_FORMAT};

/// The fullscreen blit that copies the finished canvas texture into a surface
/// texture view, cached per target format.
pub(super) struct Present {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

const BLIT_WGSL: &str = r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main( @builtin(vertex_index) index: u32 ) -> VsOut {
    var xy = array<vec2<f32>, 3>(
        vec2<f32>( -1.0, -1.0 ),
        vec2<f32>(  3.0, -1.0 ),
        vec2<f32>( -1.0,  3.0 ),
    );
    let p = xy[ index ];
    var out: VsOut;
    out.position = vec4<f32>( p, 0.0, 1.0 );
    // Framebuffer coordinates are top-down, clip space is bottom-up.
    out.uv = vec2<f32>( ( p.x + 1.0 ) * 0.5, ( 1.0 - p.y ) * 0.5 );
    return out;
}

@group(0) @binding(0) var canvas_texture: texture_2d<f32>;
@group(0) @binding(1) var canvas_sampler: sampler;

@fragment
fn fs_main( in: VsOut ) -> @location(0) vec4<f32> {
    return textureSampleLevel( canvas_texture, canvas_sampler, in.uv, 0.0 );
}
"#;

impl Renderer {
    /// The device the renderer created, so the viewer can configure a surface
    /// on it.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The adapter `pick_adapter()` chose, for surface capability queries.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// `NodeFrame.time`, in seconds. The e2e harness leaves it at 0; the viewer
    /// advances it so that time-driven node materials animate.
    pub fn set_time(&mut self, time: f64) {
        self.time = time;
    }

    /// Copies the finished canvas texture into `target`.
    ///
    /// The canvas holds sRGB-encoded bytes in an `rgba8unorm` texture (the
    /// output pass already applied the colour transform), so the bytes must be
    /// written through verbatim: pass a view whose format is *not* `-srgb`, or
    /// the hardware encodes them a second time.
    ///
    /// Returns false when there is nothing to present yet (no frame rendered).
    pub fn present(&mut self, target: &wgpu::TextureView, format: wgpu::TextureFormat) -> bool {
        if self.canvas.is_none() {
            return false;
        }

        let stale = match &self.present {
            Some(present) => present.format != format,
            None => true,
        };

        if stale {
            let module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("three-rs present blit"),
                    source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
                });

            let layout = self
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("three-rs present layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

            let pipeline_layout =
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("three-rs present pipeline layout"),
                        bind_group_layouts: &[Some(&layout)],
                        immediate_size: 0,
                    });

            let pipeline = self
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("three-rs present pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some("vs_main"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_main"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview_mask: None,
                    cache: None,
                });

            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("three-rs present sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });

            self.present = Some(Present {
                format,
                pipeline,
                layout,
                sampler,
            });
        }

        let present = self
            .present
            .as_ref()
            .expect("three-rs: the present pipeline was just built for this format");
        let canvas = self
            .canvas
            .as_ref()
            .expect("three-rs: present() returns early when there is no canvas");
        let canvas_view = canvas.color.create_view(&wgpu::TextureViewDescriptor {
            format: Some(CANVAS_FORMAT),
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("three-rs present bind group"),
            layout: &present.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&present.sampler),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("three-rs present"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("three-rs present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&present.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        true
    }
}
