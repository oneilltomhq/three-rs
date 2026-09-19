//! The renderer's half of the node system: turning a built `NodeProgram` into
//! wgpu shader modules, bind-group layouts, a pipeline and, per draw, the bind
//! groups and uniform bytes. This is `WebGPUPipelineUtils` +
//! `WebGPUBindingUtils` + `Bindings.updateBinding()`, generically driven by the
//! descriptors the node builder produced — there is nothing per-material here.

use crate::materials::Side;
use crate::math::{Color, Matrix3, Matrix4, Vector2, Vector3, Vector4};
use crate::nodes::node::BufferSource;
use crate::nodes::wgsl::TextureKind;
use crate::nodes::{BindingDesc, NodeProgram, Type, UniformMember, UniformSource};

/// Everything about a pass that the pipeline has to bake in, beyond the shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderState {
    pub color_format: wgpu::TextureFormat,
    /// How many colour attachments the pass has — `renderTarget.textures.length`,
    /// 1 for everything but an MRT pass. The fragment stage declares one
    /// `@location` per attachment, so a pipeline with the wrong count is a
    /// validation error rather than wrong pixels; it is part of the key for the
    /// same reason the format is.
    pub color_attachments: u32,
    /// One entry per colour attachment **past the first**: its format and the
    /// blend state `MRTNode.getBlendMode( texture.name )` gives it.
    ///
    /// Three sizes the `targets` array from `renderObject.context.textures` and
    /// takes each entry's format from that texture, so an MRT whose attachments
    /// have different types — `webgpu_postprocessing_bloom_emissive`'s
    /// `rgba16float` colour beside an `rgba8unorm` emissive — is one pipeline
    /// with two different target formats. A fixed array keeps `RenderState`
    /// `Copy` and part of the pipeline key; `MAX_EXTRA_COLOR_ATTACHMENTS` is
    /// the port's ceiling, not WebGPU's (which is 8).
    pub extra_color_targets: [Option<ExtraColorTarget>; MAX_EXTRA_COLOR_ATTACHMENTS],
    pub depth_format: Option<wgpu::TextureFormat>,
    pub sample_count: u32,
    pub side: Side,
    pub depth_test: bool,
    pub depth_write: bool,
    /// `WebGPUPipelineUtils.createRenderPipeline()`'s `materialBlending`, i.e.
    /// `MeshBasicNodeMaterial::blend_state()`. Part of the key because an
    /// additive and an opaque pipeline share one program.
    pub blend: Option<wgpu::BlendState>,
    /// `WebGPUUtils.getPrimitiveTopology( object, material )`. It comes from
    /// the *object*, not the material, so one `LineBasicNodeMaterial` shared by
    /// a `Line` and a `LineSegments` needs two pipelines off one program —
    /// which is exactly why this is part of the key.
    pub topology: wgpu::PrimitiveTopology,
    /// `_getPrimitiveState()`: set only for an indexed `Line` that is not a
    /// `LineSegments`, from the index array's type.
    pub strip_index_format: Option<wgpu::IndexFormat>,
}

/// How many colour attachments past the first a pass may have here.
pub const MAX_EXTRA_COLOR_ATTACHMENTS: usize = 3;

/// One MRT colour attachment past the first, as the pipeline descriptor wants
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExtraColorTarget {
    pub format: wgpu::TextureFormat,
    pub blend: Option<wgpu::BlendState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub program: u64,
    pub state: RenderState,
}

/// A compiled material: the two shader modules plus the layouts its bind groups
/// are built against — `NodeBuilderState`'s compiled half, shared by every
/// render object whose `cache_key` matches.
///
/// Deliberately *not* the `NodeProgram` it was compiled from. A binding
/// description names a texture and an instanced attribute's array; holding the
/// first-seen draw's copy here would keep those alive for the life of the
/// renderer (#56) and, worse, would be the wrong copy for the second material
/// that hashes to this program. The descriptions the per-draw resolution needs
/// live with the material that owns them, in `Renderer::node_builder_states`.
pub struct Program {
    vertex_module: wgpu::ShaderModule,
    fragment_module: wgpu::ShaderModule,
    pub layouts: Vec<wgpu::BindGroupLayout>,
    pipeline_layout: wgpu::PipelineLayout,
    /// `WebGPUAttributeUtils.createShaderVertexBuffers()`'s result, shape only:
    /// stride, step mode and attributes per buffer, with the buffer itself —
    /// the `Rc<InstanceBuffer>` a `VertexBufferDesc` carries — left behind.
    vertex_layouts: Vec<VertexLayout>,
}

/// One `GPUVertexBufferLayout` as the pipeline bakes it in.
struct VertexLayout {
    array_stride: u64,
    step_mode: wgpu::VertexStepMode,
    attributes: Vec<wgpu::VertexAttribute>,
}

impl Program {
    pub fn new(device: &wgpu::Device, node: &NodeProgram) -> Self {
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

        // `WebGPUAttributeUtils.createShaderVertexBuffers()`: the attributes
        // grouped into buffers — one per geometry attribute, one per instanced
        // buffer, in first-use order. The layouts are computed from the same
        // `AttributeSlot`s the renderer binds from, so a slot and its buffer
        // cannot disagree.
        let vertex_layouts = node
            .vertex_buffers()
            .iter()
            .map(|desc| VertexLayout {
                array_stride: desc.array_stride,
                step_mode: if desc.instanced {
                    wgpu::VertexStepMode::Instance
                } else {
                    wgpu::VertexStepMode::Vertex
                },
                attributes: desc
                    .attributes
                    .iter()
                    .map(|(location, ty, offset)| wgpu::VertexAttribute {
                        format: vertex_format(*ty),
                        offset: *offset,
                        shader_location: *location,
                    })
                    .collect(),
            })
            .collect();

        Self {
            vertex_module,
            fragment_module,
            layouts,
            pipeline_layout,
            vertex_layouts,
        }
    }

    /// `WebGPUPipelineUtils.createRenderPipeline()`.
    pub fn create_pipeline(
        &self,
        device: &wgpu::Device,
        state: RenderState,
    ) -> wgpu::RenderPipeline {
        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = self
            .vertex_layouts
            .iter()
            .map(|layout| {
                Some(wgpu::VertexBufferLayout {
                    array_stride: layout.array_stride,
                    step_mode: layout.step_mode,
                    attributes: layout.attributes.as_slice(),
                })
            })
            .collect();

        let targets: Vec<Option<wgpu::ColorTargetState>> = (0..state.color_attachments)
            .map(|i| {
                // Attachment 0 is the material's own format and blending;
                // `MRTNode.blendModes.output` is `MaterialBlending`, which
                // resolves to exactly that. Every attachment past it carries
                // its own format and whatever `setBlendMode` gave it.
                let extra = (i > 0)
                    .then(|| state.extra_color_targets[i as usize - 1])
                    .flatten();
                Some(wgpu::ColorTargetState {
                    format: extra.map_or(state.color_format, |extra| extra.format),
                    // `undefined` for an opaque `NormalBlending` material, which
                    // is every rung up to 9; see `materials::blending`.
                    blend: extra.map_or(state.blend, |extra| extra.blend),
                    write_mask: wgpu::ColorWrites::ALL,
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
                // One target per colour attachment. `MRTNode.blendModes` gives
                // three a per-output blend — `MaterialBlending` for `output`
                // and `NoBlending` for the rest — but every MRT material on
                // this ladder is opaque, where both resolve to no blend state
                // at all, so the material's own is used for each.
                targets: &targets,
            }),
            primitive: wgpu::PrimitiveState {
                topology: state.topology,
                strip_index_format: state.strip_index_format,
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
        BindingDesc::Uniforms { visibility, .. } => wgpu::BindGroupLayoutEntry {
            binding,
            visibility: visibility.stages(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        BindingDesc::Buffer {
            visibility, source, ..
        } => wgpu::BindGroupLayoutEntry {
            binding,
            visibility: visibility.stages(),
            ty: wgpu::BindingType::Buffer {
                // The same rule as the WGSL declaration: `read_write` in
                // compute, `read` in the vertex and fragment stages, which is
                // why the page asks for `maxStorageBuffersInVertexStage: 1`.
                ty: match source {
                    BufferSource::Storage => wgpu::BufferBindingType::Storage {
                        read_only: !visibility.compute,
                    },
                    _ => wgpu::BufferBindingType::Uniform,
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
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
                    TextureKind::Float2DArray | TextureKind::FloatData2D => {
                        wgpu::TextureSampleType::Float { filterable: false }
                    }
                    // `r32uint` — `BatchedMesh._indirectTexture`.
                    TextureKind::Uint2D => wgpu::TextureSampleType::Uint,
                    TextureKind::Depth2D | TextureKind::DepthCompare2D | TextureKind::DepthCube => {
                        wgpu::TextureSampleType::Depth
                    }
                    _ => wgpu::TextureSampleType::Float { filterable: true },
                },
                view_dimension: match kind {
                    TextureKind::Cube | TextureKind::DepthCube => wgpu::TextureViewDimension::Cube,
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
                TextureKind::DepthCompare2D | TextureKind::DepthCube => {
                    wgpu::SamplerBindingType::Comparison
                }
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
        // `WebGPUAttributeUtils.createAttribute()` reads the format off the
        // attribute's own typed array: `skinIndex` is a `Uint32Array` by the
        // time it reaches the GPU, so `uvec4` is `uint32x4`, not a float format
        // the shader casts.
        Type::UVec4 => wgpu::VertexFormat::Uint32x4,
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
    /// `HemisphereLight.groundColor * intensity`, in the working colour space.
    pub ground_color: Color,
    /// `cos( light.angle )` — `SpotLightNode.update()`.
    pub cone_cos: f64,
    /// `cos( light.angle * ( 1 - light.penumbra ) )`.
    pub penumbra_cos: f64,
    /// `light.shadow.matrix` — bias ∘ projection ∘ the shadow camera's
    /// `matrixWorldInverse`, or `makeTranslation( -lightWorldPos )` for a
    /// point light.
    pub shadow_matrix: Matrix4,
    /// `light.shadow.camera.near` / `.far` — `PointShadowNode`'s depth test.
    pub shadow_camera_near: f64,
    pub shadow_camera_far: f64,
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
            ground_color: Color::new(0.0, 0.0, 0.0),
            cone_cos: 0.0,
            penumbra_cos: 0.0,
            shadow_matrix: Matrix4::identity(),
            shadow_camera_near: 0.5,
            shadow_camera_far: 500.0,
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
    /// `camera.projectionMatrixInverse`, kept beside the projection rather than
    /// inverted here: a camera maintains it in `updateProjectionMatrix()` and
    /// the two must not drift.
    pub camera_projection_inverse: Matrix4,
    pub model_world: Matrix4,
    pub material_color: Color,
    pub material_opacity: f64,
    pub material_rotation: f64,
    /// `material.linewidth`.
    pub material_line_width: f64,
    pub material_reflectivity: f64,
    pub material_shininess: f64,
    pub material_specular: Color,
    pub material_emissive: Color,
    pub material_emissive_intensity: f64,
    pub material_metalness: f64,
    pub material_roughness: f64,
    pub material_bump_scale: f64,
    /// `MeshPhysicalMaterial.ior` / `.specularIntensity` / `.specularColor`,
    /// and `MeshStandardMaterial.normalScale`.
    pub material_ior: f64,
    pub material_specular_intensity: f64,
    pub material_specular_color: Color,
    pub material_normal_scale: Vector2,
    pub env_rotation: Matrix4,
    /// `material.envMapIntensity` — 1 on every material this rung builds.
    pub material_env_intensity: f64,
    /// `MeshStandardMaterial.aoMapIntensity`.
    pub material_ao_map_intensity: f64,
    pub background_rotation: Matrix4,
    pub background_blurriness: f64,
    pub background_intensity: f64,
    /// `viewportSize` — the bound target's dimensions.
    pub viewport_size: Vector2,
    /// `viewport` — `( x, y, width, height )` of the pass rectangle, in
    /// physical pixels. The whole target when nothing set a viewport.
    pub viewport: Vector4,
    /// `screenDPR` — `renderer.getPixelRatio()`.
    pub screen_dpr: f64,
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
    /// `SkinnedMesh.bindMatrix` / `.bindMatrixInverse`.
    pub bind_matrix: Matrix4,
    pub bind_matrix_inverse: Matrix4,
    /// `skeleton.boneMatrices` — the flat `mat4` array the bone buffer holds,
    /// already updated for this frame.
    pub bone_matrices: &'a [f32],
}

impl Default for UniformContext<'_> {
    fn default() -> Self {
        Self {
            camera_projection: Matrix4::identity(),
            camera_view: Matrix4::identity(),
            camera_world: Matrix4::identity(),
            camera_projection_inverse: Matrix4::identity(),
            model_world: Matrix4::identity(),
            material_color: Color::new(1.0, 1.0, 1.0),
            material_opacity: 1.0,
            material_rotation: 0.0,
            material_line_width: 1.0,
            material_reflectivity: 1.0,
            material_shininess: 30.0,
            material_specular: Color::new(
                0x11 as f64 / 255.0,
                0x11 as f64 / 255.0,
                0x11 as f64 / 255.0,
            ),
            material_emissive: Color::new(0.0, 0.0, 0.0),
            material_emissive_intensity: 1.0,
            // `MeshStandardMaterial` defaults.
            material_metalness: 0.0,
            material_roughness: 1.0,
            material_bump_scale: 1.0,
            // `MeshPhysicalMaterial`'s defaults.
            material_ior: 1.5,
            material_specular_intensity: 1.0,
            material_specular_color: Color::new(1.0, 1.0, 1.0),
            material_normal_scale: Vector2::new(1.0, 1.0),
            env_rotation: Matrix4::identity(),
            material_env_intensity: 1.0,
            material_ao_map_intensity: 1.0,
            background_rotation: Matrix4::identity(),
            background_blurriness: 0.0,
            background_intensity: 1.0,
            viewport_size: Vector2::new(0.0, 0.0),
            viewport: Vector4::new(0.0, 0.0, 0.0, 0.0),
            screen_dpr: 1.0,
            time: 0.0,
            tone_mapping_exposure: 1.0,
            lights: &[],
            morph_base: 1.0,
            morph_influences: &[],
            bind_matrix: Matrix4::identity(),
            bind_matrix_inverse: Matrix4::identity(),
            bone_matrices: &[],
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
                UniformSource::CameraProjectionMatrixInverse => {
                    self.camera_projection_inverse.to_f32_array().to_vec()
                }
                // `modelWorldMatrixInverse`: `self.value.copy(
                // object.matrixWorld ).invert()`, per object.
                UniformSource::ModelWorldMatrixInverse => {
                    let mut inverse = self.model_world;
                    inverse.invert();
                    inverse.to_f32_array().to_vec()
                }
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
                UniformSource::MaterialEnvIntensity => vec![self.material_env_intensity as f32],
                UniformSource::MaterialAoMapIntensity => {
                    vec![self.material_ao_map_intensity as f32]
                }
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
                UniformSource::MaterialLineWidth => vec![self.material_line_width as f32],
                UniformSource::MaterialMetalness => vec![self.material_metalness as f32],
                UniformSource::MaterialRoughness => vec![self.material_roughness as f32],
                UniformSource::MaterialBumpScale => vec![self.material_bump_scale as f32],
                UniformSource::MaterialIor => vec![self.material_ior as f32],
                UniformSource::MaterialSpecularIntensity => {
                    vec![self.material_specular_intensity as f32]
                }
                UniformSource::MaterialSpecularColor => vec![
                    self.material_specular_color.r as f32,
                    self.material_specular_color.g as f32,
                    self.material_specular_color.b as f32,
                ],
                UniformSource::MaterialNormalScale => vec![
                    self.material_normal_scale.x as f32,
                    self.material_normal_scale.y as f32,
                ],
                UniformSource::EnvRotationMatrix => self.env_rotation.to_f32_array().to_vec(),
                UniformSource::BackgroundRotation => {
                    self.background_rotation.to_f32_array().to_vec()
                }
                UniformSource::BackgroundBlurriness => vec![self.background_blurriness as f32],
                UniformSource::BackgroundIntensity => vec![self.background_intensity as f32],
                UniformSource::Time => vec![self.time as f32],
                UniformSource::ViewportSize => {
                    vec![self.viewport_size.x as f32, self.viewport_size.y as f32]
                }
                UniformSource::Viewport => vec![
                    self.viewport.x as f32,
                    self.viewport.y as f32,
                    self.viewport.z as f32,
                    self.viewport.w as f32,
                ],
                UniformSource::ScreenDpr => vec![self.screen_dpr as f32],
                UniformSource::LightColorIntensity(i) => {
                    let light = &self.lights[*i];
                    vec![
                        light.color.r as f32,
                        light.color.g as f32,
                        light.color.b as f32,
                    ]
                }
                UniformSource::LightWorldPosition(i) => {
                    let p = self.lights[*i].world_position;
                    vec![p.x as f32, p.y as f32, p.z as f32]
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
                UniformSource::BindMatrix => self.bind_matrix.to_f32_array().to_vec(),
                UniformSource::BindMatrixInverse => {
                    self.bind_matrix_inverse.to_f32_array().to_vec()
                }
                UniformSource::LightTargetPosition(i) => {
                    let p = self.lights[*i].target_position;
                    vec![p.x as f32, p.y as f32, p.z as f32]
                }
                UniformSource::LightGroundColor(i) => {
                    let c = self.lights[*i].ground_color;
                    vec![c.r as f32, c.g as f32, c.b as f32]
                }
                UniformSource::LightConeCos(i) => vec![self.lights[*i].cone_cos as f32],
                UniformSource::LightPenumbraCos(i) => vec![self.lights[*i].penumbra_cos as f32],
                UniformSource::ShadowMatrix(i) => {
                    self.lights[*i].shadow_matrix.to_f32_array().to_vec()
                }
                UniformSource::ShadowCameraNear(i) => {
                    vec![self.lights[*i].shadow_camera_near as f32]
                }
                UniformSource::ShadowCameraFar(i) => vec![self.lights[*i].shadow_camera_far as f32],
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
                // Read per draw, so `sampleWeight.value = …` between two
                // `render_quad()` calls reaches the second one's buffer.
                UniformSource::Settable(cell) => cell.get().iter().map(|&v| v as f32).collect(),
            };

            let offset = member.offset as usize;
            // The one member whose bits are not an f32's: `ComputeNode`'s
            // element count, `uniform( count, 'uint' )`. It rides
            // `UniformSource::Value` like every other baked value, so the type
            // is what says how to write it. Exact for counts below 2^24, which
            // is every count a `dispatchWorkgroups` limit of 65535 groups of 64
            // can reach anyway.
            if member.ty == Type::U32 {
                let raw: Vec<u32> = values.iter().map(|&v| v as u32).collect();
                data[offset..offset + raw.len() * 4].copy_from_slice(bytemuck::cast_slice(&raw));
                continue;
            }
            data[offset..offset + values.len() * 4].copy_from_slice(bytemuck::cast_slice(&values));
        }

        data
    }
}

/// A compiled `ComputeNode` — `WebGPUPipelineUtils.createComputePipeline()`.
///
/// The same shape as [`Program`] with one module and no vertex or render state:
/// a compute pipeline has nothing a pass can vary, so the program cache *is*
/// the pipeline cache and there is no second `PipelineKey` level.
pub struct ComputeProgramGpu {
    pub layouts: Vec<wgpu::BindGroupLayout>,
    pub pipeline: wgpu::ComputePipeline,
}

impl ComputeProgramGpu {
    pub fn new(device: &wgpu::Device, program: &crate::nodes::ComputeProgram) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("three-rs compute"),
            source: wgpu::ShaderSource::Wgsl(program.wgsl.clone().into()),
        });

        let layouts: Vec<wgpu::BindGroupLayout> = program
            .groups
            .iter()
            .map(|bindings| {
                let entries: Vec<wgpu::BindGroupLayoutEntry> = bindings
                    .iter()
                    .enumerate()
                    .map(|(binding, desc)| layout_entry(binding as u32, desc))
                    .collect();
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("three-rs compute bindings"),
                    entries: &entries,
                })
            })
            .collect();

        let refs: Vec<Option<&wgpu::BindGroupLayout>> = layouts.iter().map(Some).collect();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("three-rs compute pipeline layout"),
            bind_group_layouts: &refs,
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("three-rs compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        ComputeProgramGpu { layouts, pipeline }
    }
}
