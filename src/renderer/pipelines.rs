//! Per-material pipeline creation — the rung-1 stand-in for
//! `WebGPUPipelineUtils` + `NodeManager`. One pipeline per
//! (shader variant, colour format, sample count, depth format).

use crate::materials::ShaderKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub shader: ShaderKey,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
}

pub struct Shaders {
    pub basic: wgpu::ShaderModule,
    pub quad: wgpu::ShaderModule,
}

impl Shaders {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            basic: device.create_shader_module(wgpu::include_wgsl!("shaders/basic.wgsl")),
            quad: device.create_shader_module(wgpu::include_wgsl!("shaders/quad.wgsl")),
        }
    }
}

pub struct Layouts {
    /// `basic.wgsl`: camera uniforms + a dynamically offset per-object uniform.
    pub basic: wgpu::BindGroupLayout,
    /// `quad.wgsl`: a depth texture + its sampler.
    pub quad: wgpu::BindGroupLayout,
}

impl Layouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let basic = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("three-rs basic bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let quad = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("three-rs quad bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        Self { basic, quad }
    }
}

pub fn create_pipeline(
    device: &wgpu::Device,
    shaders: &Shaders,
    layouts: &Layouts,
    key: PipelineKey,
) -> wgpu::RenderPipeline {
    let (module, bind_group_layout, vertex_buffers): (_, _, &[Option<wgpu::VertexBufferLayout>]) = match key.shader {
        ShaderKey::Basic => (
            &shaders.basic,
            &layouts.basic,
            &[Some(wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            })],
        ),
        ShaderKey::DepthTextureQuad => (&shaders.quad, &layouts.quad, &[]),
    };

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("three-rs pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("three-rs pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("main_vertex"),
            compilation_options: Default::default(),
            buffers: vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("main_fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: key.color_format,
                // `Material.transparent` is false and `blending` is off for an
                // opaque material: three.js emits no blend state.
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            // `WebGPUPipelineUtils`: `FrontSide` → CCW front faces, cull back.
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: key.depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(true),
            // `Material.depthFunc` defaults to `LessEqualDepth`.
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: key.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
