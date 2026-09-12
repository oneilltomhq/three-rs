//! `WebGPUTexturePassUtils`: the mipmap shader module, its bind-group layout
//! and the transfer pipeline `generateMipmaps()` runs. This is the one piece of
//! hand-written WGSL three.js itself hand-writes, so it stays.

pub struct MipmapShader {
    pub module: wgpu::ShaderModule,
    pub layout: wgpu::BindGroupLayout,
}

impl MipmapShader {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::include_wgsl!("shaders/mipmap.wgsl"));

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("three-rs mipmap bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        Self { module, layout }
    }
}

/// `WebGPUTexturePassUtils.getTransferPipeline( format, '2d-array' )`.
pub fn create_mipmap_pipeline(
    device: &wgpu::Device,
    shader: &MipmapShader,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("three-rs mipmap pipeline layout"),
        bind_group_layouts: &[Some(&shader.layout)],
        immediate_size: 0,
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("three-rs mipmap pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader.module,
            entry_point: Some("mainVS"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader.module,
            entry_point: Some("main_2d_array"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        // `_renderPipelineDescriptor.primitive` is left at its defaults:
        // triangle-list, CCW front faces, no culling.
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}
