//! Per-material pipeline creation — the rung-1 stand-in for
//! `WebGPUPipelineUtils` + `NodeManager`. One pipeline per
//! (shader variant, colour format, sample count, depth format).

use std::collections::HashMap;

use crate::materials::ShaderKey;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub shader: ShaderKey,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
    /// The instance count baked into the shader's `array<…, N>` declarations —
    /// three.js bakes the same number into `buffer( array, type, count )`.
    pub instance_count: u32,
}

pub struct Shaders {
    pub basic: wgpu::ShaderModule,
    pub quad: wgpu::ShaderModule,
    pub output_color_transform: wgpu::ShaderModule,
    /// Keyed by instance count, because the count is baked into the source.
    normal_world_range_mix: HashMap<u32, wgpu::ShaderModule>,
}

impl Shaders {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            basic: device.create_shader_module(wgpu::include_wgsl!("shaders/basic.wgsl")),
            quad: device.create_shader_module(wgpu::include_wgsl!("shaders/quad.wgsl")),
            output_color_transform: device
                .create_shader_module(wgpu::include_wgsl!("shaders/output_color_transform.wgsl")),
            normal_world_range_mix: HashMap::new(),
        }
    }

    /// Builds (once per instance count) the `normal_world_range_mix` module,
    /// substituting the array length the way three.js' node builder does.
    pub fn ensure_normal_world_range_mix(&mut self, device: &wgpu::Device, instance_count: u32) {
        self.normal_world_range_mix
            .entry(instance_count)
            .or_insert_with(|| {
                let source = include_str!("shaders/normal_world_range_mix.wgsl")
                    .replace("INSTANCE_COUNT", &instance_count.to_string());

                device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("three-rs normal_world_range_mix"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                })
            });
    }

    pub fn normal_world_range_mix(&self, instance_count: u32) -> &wgpu::ShaderModule {
        &self.normal_world_range_mix[&instance_count]
    }
}

pub struct Layouts {
    /// `basic.wgsl`: camera uniforms + a dynamically offset per-object uniform.
    pub basic: wgpu::BindGroupLayout,
    /// `quad.wgsl`: a depth texture + its sampler.
    pub quad: wgpu::BindGroupLayout,
    /// `output_color_transform.wgsl`: the render uniforms (for `viewportSize`)
    /// plus the framebuffer target's colour texture and its sampler.
    pub output_color_transform: wgpu::BindGroupLayout,
    /// `normal_world_range_mix.wgsl`: camera + per-object (dynamic) + the frame
    /// uniforms + the instance matrix buffer + the `range()` buffer.
    pub normal_world_range_mix: wgpu::BindGroupLayout,
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

        let uniform = |binding: u32,
                       visibility: wgpu::ShaderStages,
                       has_dynamic_offset: bool| {
            wgpu::BindGroupLayoutEntry {
                binding,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset,
                    min_binding_size: None,
                },
                count: None,
            }
        };

        let both = wgpu::ShaderStages::VERTEX_FRAGMENT;

        let normal_world_range_mix =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("three-rs normal_world_range_mix bindings"),
                entries: &[
                    uniform(0, both, false),
                    uniform(1, wgpu::ShaderStages::VERTEX, true),
                    uniform(2, wgpu::ShaderStages::FRAGMENT, false),
                    uniform(3, wgpu::ShaderStages::VERTEX, false),
                    uniform(4, wgpu::ShaderStages::FRAGMENT, false),
                ],
            });

        let output_color_transform =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("three-rs output_color_transform bindings"),
                entries: &[
                    uniform(0, wgpu::ShaderStages::FRAGMENT, false),
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        Self {
            basic,
            quad,
            output_color_transform,
            normal_world_range_mix,
        }
    }
}

pub const POSITION_ATTRIBUTE: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x3,
    offset: 0,
    shader_location: 0,
}];

pub const NORMAL_ATTRIBUTE: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x3,
    offset: 0,
    shader_location: 1,
}];

pub fn create_pipeline(
    device: &wgpu::Device,
    shaders: &mut Shaders,
    layouts: &Layouts,
    key: PipelineKey,
) -> wgpu::RenderPipeline {
    if key.shader == ShaderKey::NormalWorldRangeMix {
        shaders.ensure_normal_world_range_mix(device, key.instance_count);
    }

    let shaders = &*shaders;

    let normal_world_range_mix_buffers = [
        Some(wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_ATTRIBUTE,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: 12,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &NORMAL_ATTRIBUTE,
        }),
    ];

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
        ShaderKey::OutputColorTransform => (
            &shaders.output_color_transform,
            &layouts.output_color_transform,
            &[],
        ),
        ShaderKey::NormalWorldRangeMix => (
            shaders.normal_world_range_mix(key.instance_count),
            &layouts.normal_world_range_mix,
            &normal_world_range_mix_buffers,
        ),
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
