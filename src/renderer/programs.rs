//! The renderer's half of the node system: turning a built `NodeProgram` into
//! wgpu shader modules, bind-group layouts, a pipeline and, per draw, the bind
//! groups and uniform bytes. This is `WebGPUPipelineUtils` +
//! `WebGPUBindingUtils` + `Bindings.updateBinding()`, generically driven by the
//! descriptors the node builder produced — there is nothing per-material here.

use crate::materials::Side;
use crate::math::{Color, Matrix3, Matrix4, Vector2, Vector3};
use crate::nodes::wgsl::TextureKind;
use crate::nodes::{BindingDesc, NodeProgram, Type, UniformMember, UniformSource};

/// Everything about a pass that the pipeline has to bake in, beyond the shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderState {
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
    pub side: Side,
    pub depth_test: bool,
    pub depth_write: bool,
    /// `WebGPUPipelineUtils.createRenderPipeline()`'s `materialBlending`, i.e.
    /// `MeshBasicNodeMaterial::blend_state()`. Part of the key because an
    /// additive and an opaque pipeline share one program.
    pub blend: Option<wgpu::BlendState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub program: u64,
    pub state: RenderState,
}

/// A compiled material: the two shader modules plus the layouts its bind groups
/// are built against.
pub struct Program {
    pub node: NodeProgram,
    vertex_module: wgpu::ShaderModule,
    fragment_module: wgpu::ShaderModule,
    pub layouts: Vec<wgpu::BindGroupLayout>,
    pipeline_layout: wgpu::PipelineLayout,
}

impl Program {
    pub fn new(device: &wgpu::Device, node: NodeProgram) -> Self {
        let vertex_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("three-rs vertex"),
            source: wgpu::ShaderSource::Wgsl(node.vertex_wgsl.clone().into()),
        });
        let fragment_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("three-rs fragment"),
            source: wgpu::ShaderSource::Wgsl(node.fragment_wgsl.clone().into()),
        });

        let layouts: Vec<wgpu::BindGroupLayout> = node
            .groups
            .iter()
            .map(|bindings| {
                let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings
                    .iter()
                    .enumerate()
                    .map(|(binding, desc)| layout_entry(binding as u32, desc))
                    .collect();
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("three-rs bindings"),
                    entries: &entries,
                })
            })
            .collect();

        let refs: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().map(Some).collect();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("three-rs pipeline layout"),
            bind_group_layouts: &refs,
            immediate_size: 0,
        });

        Self {
            node,
            vertex_module,
            fragment_module,
            layouts,
            pipeline_layout,
        }
    }

    /// `WebGPUPipelineUtils.createRenderPipeline()`.
    pub fn create_pipeline(
        &self,
        device: &wgpu::Device,
        state: RenderState,
    ) -> wgpu::RenderPipeline {
        // `WebGPUAttributeUtils.createShaderVertexBuffers()`: the attributes
        // grouped into buffers — one per geometry attribute, one per instanced
        // buffer, in first-use order. The layouts are computed from the same
        // `AttributeSlot`s the renderer binds from, so a slot and its buffer
        // cannot disagree.
        let descs = self.node.vertex_buffers();

        let attributes: Vec<Vec<wgpu::VertexAttribute>> = descs
            .iter()
            .map(|desc| {
                desc.attributes
                    .iter()
                    .map(|(location, ty, offset)| wgpu::VertexAttribute {
                        format: vertex_format(*ty),
                        offset: *offset,
                        shader_location: *location,
                    })
                    .collect()
            })
            .collect();

        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = descs
            .iter()
            .zip(attributes.iter())
            .map(|(desc, attributes)| {
                Some(wgpu::VertexBufferLayout {
                    array_stride: desc.array_stride,
                    step_mode: if desc.instanced {
                        wgpu::VertexStepMode::Instance
                    } else {
                        wgpu::VertexStepMode::Vertex
                    },
                    attributes: attributes.as_slice(),
                })
            })
            .collect();

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("three-rs pipeline"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.vertex_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.fragment_module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.color_format,
                    // `undefined` for an opaque `NormalBlending` material, which
                    // is every rung up to 9; see `materials::blending`.
                    blend: state.blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // `_getPrimitiveState()`: `FrontSide` → CCW front faces,
                // `BackSide` → CW, both culling the back face.
                front_face: match state.side {
                    Side::Front | Side::Double => wgpu::FrontFace::Ccw,
                    Side::Back => wgpu::FrontFace::Cw,
                },
                // `DoubleSide` → `cullMode: 'none'`.
                cull_mode: match state.side {
                    Side::Double => None,
                    _ => Some(wgpu::Face::Back),
                },
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: state.depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: Some(state.depth_write),
                // `Material.depthFunc` defaults to `LessEqualDepth`; with
                // `depthTest` off `_getDepthCompare()` returns `'always'`.
                depth_compare: Some(if state.depth_test {
                    wgpu::CompareFunction::LessEqual
                } else {
                    wgpu::CompareFunction::Always
                }),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: state.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
    }
}

fn layout_entry(binding: u32, desc: &BindingDesc) -> wgpu::BindGroupLayoutEntry {
    match desc {
        BindingDesc::Uniforms { visibility, .. } | BindingDesc::Buffer { visibility, .. } => {
            wgpu::BindGroupLayoutEntry {
                binding,
                visibility: visibility.stages(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        }
        BindingDesc::Texture {
            kind, visibility, ..
        } => wgpu::BindGroupLayoutEntry {
            binding,
            visibility: visibility.stages(),
            ty: wgpu::BindingType::Texture {
                sample_type: match kind {
                    // The morph data texture is `rgba32float`, which wgpu only
                    // lets a pipeline filter with the `FLOAT32_FILTERABLE`
                    // feature. It is only ever `textureLoad`ed, so declaring it
                    // non-filterable costs nothing and keeps the feature set to
                    // the WebGPU baseline three.js targets.
                    TextureKind::Float2DArray => {
                        wgpu::TextureSampleType::Float { filterable: false }
                    }
                    TextureKind::Depth2D | TextureKind::DepthCompare2D => {
                        wgpu::TextureSampleType::Depth
                    }
                    _ => wgpu::TextureSampleType::Float { filterable: true },
                },
                view_dimension: match kind {
                    TextureKind::Cube => wgpu::TextureViewDimension::Cube,
                    TextureKind::Float2DArray => wgpu::TextureViewDimension::D2Array,
                    _ => wgpu::TextureViewDimension::D2,
                },
                multisampled: false,
            },
            count: None,
        },
        BindingDesc::Sampler {
            kind, visibility, ..
        } => wgpu::BindGroupLayoutEntry {
            binding,
            visibility: visibility.stages(),
            ty: wgpu::BindingType::Sampler(match kind {
                TextureKind::Depth2D => wgpu::SamplerBindingType::NonFiltering,
                TextureKind::DepthCompare2D => wgpu::SamplerBindingType::Comparison,
                _ => wgpu::SamplerBindingType::Filtering,
            }),
            count: None,
        },
    }
}

fn vertex_format(ty: Type) -> wgpu::VertexFormat {
    match ty {
        Type::Vec2 => wgpu::VertexFormat::Float32x2,
        Type::Vec3 => wgpu::VertexFormat::Float32x3,
        Type::Vec4 => wgpu::VertexFormat::Float32x4,
        Type::F32 => wgpu::VertexFormat::Float32,
        other => panic!("three-rs: {other:?} is not a vertex attribute type"),
    }
}

/// Everything a `UniformSource` can be resolved against: the camera of the pass,
/// the object being drawn, its material, and the renderer's frame state. The
/// defaults are three.js': an identity model matrix, a white opaque material,
/// `Scene`'s background rotation/blurriness/intensity and `Texture`'s identity
/// uv transform.
/// One light as the uniform writer sees it: three.js' `LightsNode` resolves a
/// `PointLight` to exactly these four values per render.
#[derive(Clone, Copy, Debug)]
pub struct LightState {
    /// `light.color * light.intensity`, in the working colour space.
    pub color: Color,
    /// The light's world position through the camera's view matrix.
    pub view_position: Vector3,
    /// `light.distance` — the shader's `cutoffDistance`.
    pub distance: f64,
    /// `light.decay`.
    pub decay: f64,
    /// `light.matrixWorld`'s translation — `lightPosition()`.
    pub world_position: Vector3,
    /// `light.target.matrixWorld`'s translation — `lightTargetPosition()`.
    pub target_position: Vector3,
    /// `cos( light.angle )` — `SpotLightNode.update()`.
    pub cone_cos: f64,
    /// `cos( light.angle * ( 1 - light.penumbra ) )`.
    pub penumbra_cos: f64,
    /// `light.shadow.matrix` — bias ∘ projection ∘ the shadow camera's
    /// `matrixWorldInverse`.
    pub shadow_matrix: Matrix4,
    /// `light.shadow.bias`.
    pub shadow_bias: f64,
    /// `light.shadow.normalBias`.
    pub shadow_normal_bias: f64,
    /// `light.shadow.radius`.
    pub shadow_radius: f64,
    /// `light.shadow.mapSize`.
    pub shadow_map_size: Vector2,
    /// `light.shadow.intensity`.
    pub shadow_intensity: f64,
}

impl Default for LightState {
    fn default() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0),
            view_position: Vector3::new(0.0, 0.0, 0.0),
            distance: 0.0,
            decay: 2.0,
            world_position: Vector3::new(0.0, 0.0, 0.0),
            target_position: Vector3::new(0.0, 0.0, 0.0),
            cone_cos: 0.0,
            penumbra_cos: 0.0,
            shadow_matrix: Matrix4::identity(),
            shadow_bias: 0.0,
            shadow_normal_bias: 0.0,
            shadow_radius: 1.0,
            shadow_map_size: Vector2::new(512.0, 512.0),
            shadow_intensity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UniformContext<'a> {
    pub camera_projection: Matrix4,
    pub camera_view: Matrix4,
    pub camera_world: Matrix4,
    pub model_world: Matrix4,
    pub material_color: Color,
    pub material_opacity: f64,
    pub material_rotation: f64,
    pub material_reflectivity: f64,
    pub material_shininess: f64,
    pub material_specular: Color,
    pub material_emissive: Color,
    pub material_emissive_intensity: f64,
    pub env_rotation: Matrix4,
    pub background_rotation: Matrix4,
    pub background_blurriness: f64,
    pub background_intensity: f64,
    pub texture_matrix: Matrix3,
    pub viewport: Vector2,
    pub time: f64,
    /// `renderer.toneMappingExposure`.
    pub tone_mapping_exposure: f64,
    /// The lights of the pass, in `Scene.lights` order. Borrowed so the context
    /// stays `Copy` and can be spread with `..camera_uniforms` per draw.
    pub lights: &'a [LightState],
    /// `Morph.js`' `base` uniform: `1 - Σ morphTargetInfluences`.
    pub morph_base: f64,
    /// `mesh.morphTargetInfluences` — the `uniformArray` contents.
    pub morph_influences: &'a [f64],
}

impl Default for UniformContext<'_> {
    fn default() -> Self {
        Self {
            camera_projection: Matrix4::identity(),
            camera_view: Matrix4::identity(),
            camera_world: Matrix4::identity(),
            model_world: Matrix4::identity(),
            material_color: Color::new(1.0, 1.0, 1.0),
            material_opacity: 1.0,
            material_rotation: 0.0,
            material_reflectivity: 1.0,
            material_shininess: 30.0,
            material_specular: Color::new(
                0x11 as f64 / 255.0,
                0x11 as f64 / 255.0,
                0x11 as f64 / 255.0,
            ),
            material_emissive: Color::new(0.0, 0.0, 0.0),
            material_emissive_intensity: 1.0,
            env_rotation: Matrix4::identity(),
            background_rotation: Matrix4::identity(),
            background_blurriness: 0.0,
            background_intensity: 1.0,
            texture_matrix: Matrix3::identity(),
            viewport: Vector2::new(0.0, 0.0),
            time: 0.0,
            tone_mapping_exposure: 1.0,
            lights: &[],
            morph_base: 1.0,
            morph_influences: &[],
        }
    }
}

impl UniformContext<'_> {
    /// `Bindings.updateBinding()`: the bytes of one generated uniform struct,
    /// each member written at the offset the builder gave it.
    pub fn bytes(&self, members: &[UniformMember], size: u32) -> Vec<u8> {
        let mut data = vec![0u8; size as usize];

        for member in members {
            let values: Vec<f32> = match &member.source {
                UniformSource::CameraProjectionMatrix => {
                    self.camera_projection.to_f32_array().to_vec()
                }
                UniformSource::CameraViewMatrix => self.camera_view.to_f32_array().to_vec(),
                UniformSource::CameraWorldMatrix => self.camera_world.to_f32_array().to_vec(),
                UniformSource::ModelWorldMatrix => self.model_world.to_f32_array().to_vec(),
                UniformSource::ModelNormalMatrix => {
                    let mut normal_matrix = Matrix3::identity();
                    normal_matrix.get_normal_matrix(&self.model_world);
                    normal_matrix.to_padded_f32_array().to_vec()
                }
                UniformSource::MaterialColor => vec![
                    self.material_color.r as f32,
                    self.material_color.g as f32,
                    self.material_color.b as f32,
                ],
                UniformSource::MaterialOpacity => vec![self.material_opacity as f32],
                UniformSource::MaterialRotation => vec![self.material_rotation as f32],
                UniformSource::MaterialReflectivity => vec![self.material_reflectivity as f32],
                UniformSource::MaterialShininess => vec![self.material_shininess as f32],
                UniformSource::MaterialSpecular => vec![
                    self.material_specular.r as f32,
                    self.material_specular.g as f32,
                    self.material_specular.b as f32,
                ],
                UniformSource::MaterialEmissive => vec![
                    self.material_emissive.r as f32,
                    self.material_emissive.g as f32,
                    self.material_emissive.b as f32,
                ],
                UniformSource::MaterialEmissiveIntensity => {
                    vec![self.material_emissive_intensity as f32]
                }
                UniformSource::TextureMatrix => self.texture_matrix.to_padded_f32_array().to_vec(),
                UniformSource::EnvRotationMatrix => self.env_rotation.to_f32_array().to_vec(),
                UniformSource::BackgroundRotation => {
                    self.background_rotation.to_f32_array().to_vec()
                }
                UniformSource::BackgroundBlurriness => vec![self.background_blurriness as f32],
                UniformSource::BackgroundIntensity => vec![self.background_intensity as f32],
                UniformSource::Time => vec![self.time as f32],
                UniformSource::ViewportSize => {
                    vec![self.viewport.x as f32, self.viewport.y as f32]
                }
                UniformSource::LightColorIntensity(i) => {
                    let light = &self.lights[*i];
                    vec![
                        light.color.r as f32,
                        light.color.g as f32,
                        light.color.b as f32,
                    ]
                }
                UniformSource::LightCutoffDistance(i) => vec![self.lights[*i].distance as f32],
                UniformSource::LightDecay(i) => vec![self.lights[*i].decay as f32],
                UniformSource::LightViewPosition(i) => {
                    let p = self.lights[*i].view_position;
                    vec![p.x as f32, p.y as f32, p.z as f32]
                }
                // `Morph.js`' `OnObjectUpdate`: `base.value = 1 - Σ influences`
                // (or 1 when `morphTargetsRelative`).
                UniformSource::MorphBase => vec![self.morph_base as f32],
                UniformSource::LightWorldPosition(i) => {
                    let p = self.lights[*i].world_position;
                    vec![p.x as f32, p.y as f32, p.z as f32]
                }
                UniformSource::LightTargetPosition(i) => {
                    let p = self.lights[*i].target_position;
                    vec![p.x as f32, p.y as f32, p.z as f32]
                }
                UniformSource::LightConeCos(i) => vec![self.lights[*i].cone_cos as f32],
                UniformSource::LightPenumbraCos(i) => vec![self.lights[*i].penumbra_cos as f32],
                UniformSource::ShadowMatrix(i) => {
                    self.lights[*i].shadow_matrix.to_f32_array().to_vec()
                }
                UniformSource::ShadowBias(i) => vec![self.lights[*i].shadow_bias as f32],
                UniformSource::ShadowNormalBias(i) => {
                    vec![self.lights[*i].shadow_normal_bias as f32]
                }
                UniformSource::ShadowRadius(i) => vec![self.lights[*i].shadow_radius as f32],
                UniformSource::ShadowMapSize(i) => {
                    let s = self.lights[*i].shadow_map_size;
                    vec![s.x as f32, s.y as f32]
                }
                UniformSource::ShadowIntensity(i) => vec![self.lights[*i].shadow_intensity as f32],
                UniformSource::ToneMappingExposure => vec![self.tone_mapping_exposure as f32],
                UniformSource::Value(values) => values.iter().map(|&v| v as f32).collect(),
            };

            let offset = member.offset as usize;
            data[offset..offset + values.len() * 4]
                .copy_from_slice(bytemuck::cast_slice(&values));
        }

        data
    }
}
