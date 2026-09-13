//! Port of `three.js/src/materials/nodes/NodeMaterial.js` — `setup()` and the
//! `setupX()` methods it dispatches to. This is where a material turns into the
//! statements the builder flows into the two shader stages.

use super::phong::{self, LightDesc};
use crate::lights::LightKind;
use super::{Blending, MaterialKind, MeshBasicNodeMaterial, Side, ToneMapping};
use crate::nodes::node::Type;
use crate::nodes::tsl::*;
use crate::nodes::tsl::FogNode;
use crate::nodes::{MaterialFlow, NodeRef};

/// The per-render-object facts three.js reads off `builder.object` and
/// `builder.geometry` during setup.
#[derive(Clone, Debug, Default)]
pub struct SetupContext {
    /// `Some(count)` when the object is an `InstancedMesh`, which is what makes
    /// `NodeMaterial.setupPosition()` insert the `InstanceNode` transform.
    pub instance_count: Option<usize>,
    /// `InstancedMesh.instanceColor` is present, so `range()` resolves against
    /// the instance index.
    pub instanced: bool,
    /// The pass' lights in `Scene.lights` order — the default `LightsNode` list
    /// when the material sets no `lights_node`. Each entry carries the light's
    /// kind and, when this object receives its shadow, the shadow map to sample;
    /// both change the generated code, so they are part of the program's cache
    /// key by construction.
    pub lights: Vec<LightDesc>,
    /// `getEntry( geometry )` when the geometry has morph attributes: what
    /// `NodeMaterial.setupPosition()` needs to emit `morphReference()`.
    pub morph: Option<crate::nodes::morph::MorphEntry>,
}

/// `Renderer._getShadowNodes( material )` composed with
/// `ShadowBaseNode._getShadowMaterial()`: the per-object shadow-pass material.
///
/// three.js mutates one shared `ShadowMaterial` per light in place; the port
/// builds a fresh material per object instead, which is the same thing because
/// the program is keyed on the generated WGSL.
pub fn shadow_material(source: &MeshBasicNodeMaterial) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "ShadowMaterial";
    material.blending = Blending::No;
    material.fog = false;
    // `overrideMaterial.transparent = material.transparent`, and
    // `_shadowSide[ material.side ]` — the shadow pass draws the *back* faces
    // of a front-sided material, which is where the shadow pipelines'
    // `frontFace: cw` comes from.
    material.transparent = source.transparent;
    material.side = match source.side {
        Side::Front => Side::Back,
        Side::Back => Side::Front,
        // `_shadowSide = { [ DoubleSide ]: DoubleSide }`.
        Side::Double => Side::Double,
    };

    // `shadowRGB = vec3( 0 )`, `shadowAlpha = float( 1 )`, and the source
    // material's own colour only contributes its alpha.
    material.color_node = Some(match (&source.color_node, &source.mask_node) {
        // `hasMap || hasColorNode || hasCastShadowNode || hasMaskNode` is
        // false: the override material keeps its own `vec4( 0, 0, 0, 1 )`,
        // which is a flat four-component constant rather than a join.
        (None, None) => vec4(0.0, 0.0, 0.0, 1.0),
        (color, _) => {
            let alpha = match color {
                Some(color) => float(1.0).mul(to_vec4(color.clone()).w()),
                None => float(1.0),
            };
            vec4_join(vec![vec3(0.0, 0.0, 0.0), alpha])
        }
    });
    // `Fn( ( [ color ] ) => { maskNode.not().discard(); return color; } )`.
    material.mask_node = source.mask_node.clone();
    material
}

/// `vec4( node )` the way `setupDiffuseColor` builds it: a scalar splats, a
/// `vec3` gains an alpha of 1, a `vec4` passes through.
fn to_vec4(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::Vec4 => node,
        Type::Vec3 => vec4_join(vec![node, float(1.0)]),
        _ => node.to(Type::Vec4),
    }
}

/// `NodeMaterial.setup()`.
pub fn setup(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    fog: Option<&FogNode>,
) -> MaterialFlow {
    // `builder.context.setupNormal = () => subBuild( this.setupNormal( builder
    // ), 'NORMAL' )` — installed for the whole of the material's setup, so that
    // every `normalView` the lighting flow reaches resolves to this material's
    // normal map. See `docs/nodes.md` §7.
    // `builder.context.setupPositionView = () => this.setupPositionView(
    // builder )` — the seam `SpriteNodeMaterial` overrides. Built here, before
    // either stage is flowed, exactly as `NodeMaterial.setup()` installs it.
    let position_view = match material.kind {
        MaterialKind::Sprite => Some(setup_position_view_sprite(material)),
        _ => None,
    };
    with_material_normal(material.normal_node.clone(), material.flat_shading, || {
        with_material_position_view(position_view, || setup_inner(material, ctx, fog))
    })
}

fn setup_inner(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    fog: Option<&FogNode>,
) -> MaterialFlow {
    let mut pre_vertex = Vec::new();
    let mut fragment = Vec::new();

    // --- setupPosition: the `context.position` stack, flowed into the vertex
    // stage before either stage's own flow. Morphing, skinning and batching
    // plug in here too.
    // `if ( object.morphTargetInfluences ) morphReference( object ).append()` —
    // first in `setupPosition`, before skinning, displacement, batching and
    // instancing.
    if let Some(entry) = &ctx.morph {
        pre_vertex.extend(crate::nodes::morph::morph_reference(entry));
    }

    //
    // `material.positionNode` is applied *before* the instance transform. three
    // r186 does the reverse (`NodeMaterial.js:798` instancing, then `:804`
    // `positionLocal.assign( positionNode )`), which throws the instance matrix
    // away; lib3's `BatchedText` was written against the order here. On the unit
    // `PlaneGeometry( 1, 1 )` the two formulations agree vertex for vertex, so
    // this also reproduces d33's baked-matrix workaround exactly. Recorded as a
    // deviation in `docs/nodes.md` §10 and plan §5.2.
    if let Some(node) = &material.position_node {
        pre_vertex.push(position_local().assign(to_vec3(node.clone())));
    }
    if let Some(count) = ctx.instance_count {
        let matrix = instance_matrix(count);
        pre_vertex.push(
            position_local().assign(
                matrix
                    .mul(vec4_join(vec![position_local(), float(1.0)]))
                    .xyz(),
            ),
        );
        // `InstanceNode`: the normal is transformed by the inverse transpose of
        // the instance matrix' upper 3x3.
        let m3 = join(
            Type::Mat3,
            vec![
                matrix.element(0).xyz(),
                matrix.element(1).xyz(),
                matrix.element(2).xyz(),
            ],
        );
        let inv_t = transpose(inverse_mat3(m3));
        pre_vertex.push(normal_local().assign(inv_t.mul(normal_local()).normalize()));
    }

    // --- setupDiscard: `If( maskNode.not(), () => Discard() )`, the first
    // thing in the fragment flow, before `setupDiffuseColor`.
    if let Some(mask) = &material.mask_node {
        fragment.push(discard_if(mask.clone()));
    }

    // --- the fragment flow
    let output = if let Some(fragment_node) = &material.fragment_node {
        fragment_node.clone()
    } else if material.kind == MaterialKind::Phong {
        setup_phong(material, ctx, &mut fragment)
    } else {
        // setupDiffuseColor
        let color = match &material.color_node {
            Some(node) => to_vec4(node.clone()),
            None => vec4_join(vec![material_color(), float(1.0)]),
        };
        fragment.push(diffuse_color().assign(color));
        fragment.push(
            diffuse_color()
                .w()
                .assign(diffuse_color().w().mul(material_opacity())),
        );
        // `builder.isOpaque()` — the material is not transparent, blending is
        // NormalBlending and alphaToCoverage is off. A transparent or blended
        // material keeps its per-fragment alpha instead, all the way to
        // `Output`.
        if material.is_opaque() {
            fragment.push(diffuse_color().w().assign(float(1.0)));
        }

        let outgoing = if let Some(env_map) = &material.env_map {
            // `BasicLightingModel` with an indirect environment contribution.
            fragment.push(indirect_diffuse().assign(
                vec4_join(vec![indirect_diffuse(), float(1.0)])
                    .add(vec4(1.0, 1.0, 1.0, 0.0))
                    .xyz(),
            ));
            fragment
                .push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));
            fragment
                .push(indirect_diffuse().assign(indirect_diffuse().mul(diffuse_color().xyz())));
            fragment.push(total_diffuse().assign(direct_diffuse().add(indirect_diffuse())));
            fragment.push(total_specular().assign(direct_specular().add(indirect_specular())));
            fragment.push(outgoing_light().assign(total_diffuse().add(total_specular())));

            // `MeshBasicNodeMaterial.setupEnvironment()` →
            // `BasicEnvironmentNode( cubeTexture( envMap ) )`, sampled along the
            // reflect vector and blended in by `reflectivity`.
            let dir = material_env_rotation().mul(vec4_join(vec![reflect_vector(), float(1.0)]));
            let env = cube_texture(env_map, dir);
            fragment.push(outgoing_light().assign(mix(
                outgoing_light(),
                outgoing_light().mul(env.xyz()),
                float(1.0).mul(material_reflectivity()),
            )));
            outgoing_light()
        } else {
            // `setupOutgoingLight()` with `lights === false`.
            diffuse_color().xyz()
        };

        // `basicOutput = vec4( outgoingLight, diffuseColor.a ).max( 0 )`.
        vec4_join(vec![outgoing, diffuse_color().w()]).max(float(0.0))
    };

    // `NodeMaterial.setupOutput()`: `scene.fogNode` mixes over the colour only,
    // leaving the alpha — `vec4( mix( output.rgb, fogColor, factor ), output.a )`.
    let output = match fog.filter(|_| material.fog) {
        Some(fog) => {
            fragment.push(output_property().assign(output));
            let mixed = vec4_join(vec![
                mix(
                    output_property().xyz(),
                    fog.color.clone(),
                    fog.factor.clone(),
                ),
                output_property().w(),
            ]);
            // The builder's own `emit_output_property` writes the second
            // `Output = …`, so pushing it here too would double the line.
            mixed
        }
        None => output,
    };

    // --- the vertex flow
    let position = match &material.vertex_node {
        Some(node) => node.clone(),
        None => model_view_projection(),
    };

    MaterialFlow {
        pre_vertex_statements: pre_vertex,
        fragment_statements: fragment,
        output,
        emit_output_property: material.fragment_node.is_none(),
        vertex_statements: Vec::new(),
        position,
    }
}

/// `vec3( node )`: a scalar splats, a wider vector narrows, a `vec3` passes
/// through — `NodeBuilder.format()`'s two cases.
fn to_vec3(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::Vec3 => node,
        Type::Vec4 => node.xyz(),
        _ => node.to(Type::Vec3),
    }
}

/// `vec2( node )`.
fn to_vec2(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::Vec2 => node,
        Type::Vec3 | Type::Vec4 => node.xy(),
        _ => node.to(Type::Vec2),
    }
}

/// `SpriteNodeMaterial.setupPositionView()`
/// (`src/materials/nodes/SpriteNodeMaterial.js:110`) — the sprite vertex
/// shader, in view space:
///
/// ```ignore
/// const mvPosition = modelViewMatrix.mul( vec3( positionNode || 0 ) );
/// let scale = vec2( modelWorldMatrix[ 0 ].xyz.length(),
///                   modelWorldMatrix[ 1 ].xyz.length() );
/// if ( scaleNode !== null ) scale = scale.mul( vec2( scaleNode ) );
/// if ( camera.isPerspectiveCamera && sizeAttenuation === false )
///     scale = scale.mul( mvPosition.z.negate() );
/// let alignedPosition = positionGeometry.xy;       // object.center is unset
/// alignedPosition = alignedPosition.mul( scale );
/// const rotation = float( rotationNode || materialRotation );
/// return vec4( mvPosition.xy.add( rotate( alignedPosition, rotation ) ),
///              mvPosition.zw );
/// ```
///
/// `object.center` is an `InstancedMesh`-less `Sprite` field, so the
/// `alignedPosition.sub( center.sub( 0.5 ) )` step never fires here.
fn setup_position_view_sprite(material: &MeshBasicNodeMaterial) -> NodeRef {
    let position = match &material.position_node {
        Some(node) => to_vec3(node.clone()),
        None => vec3(0.0, 0.0, 0.0),
    };
    let mv_position = model_view_matrix().mul(position);

    let mut scale = join(
        Type::Vec2,
        vec![
            length(model_world_matrix().element(0).xyz()),
            length(model_world_matrix().element(1).xyz()),
        ],
    );
    if let Some(scale_node) = &material.scale_node {
        scale = scale.mul(to_vec2(scale_node.clone()));
    }
    if !material.size_attenuation {
        // The perspective-camera branch; the ladder's only sprite material
        // leaves `sizeAttenuation` at its default `true`, which omits it.
        scale = scale.mul(mv_position.z().negate());
    }

    let aligned = position_geometry().xy().mul(scale);
    let rotation = match &material.rotation_node {
        Some(node) => node.clone(),
        None => material_rotation(),
    };

    join(
        Type::Vec4,
        vec![mv_position.xy().add(rotate(aligned, rotation)), mv_position.zw()],
    )
}

/// `Background.update()`'s skybox material: the cube map sampled along
/// `normalWorldGeometry` with the background rotation and LOD, on a sphere
/// pinned to the far plane.
pub fn background_color_node(map: &crate::textures::CubeTexture) -> NodeRef {
    let dir = material_env_rotation()
        .mul(background_rotation().mul(vec4_join(vec![normal_world_geometry(), float(1.0)])));
    let sample = cube_texture_level(map, dir, background_blurriness());
    sample.mul(background_intensity())
}

/// `Background.update()`'s `isNode` branch with a plain `color()` node:
/// `vec4( color ).mul( backgroundIntensity )`.
pub fn background_node_color_node(color: crate::math::Color) -> NodeRef {
    vec4_join(vec![color.into(), float(1.0)]).mul(background_intensity())
}

pub fn background_vertex_node() -> NodeRef {
    let is_ortho = camera_projection_matrix()
        .element(3)
        .element(3)
        .equal(float(1.0));
    let ortho_scale = float(1.0)
        .div(camera_projection_matrix().element(1).element(1))
        .mul(float(3.0));
    let modified = is_ortho.select(position_local().mul(ortho_scale), position_local());
    let view_position = model_view_matrix().mul(vec4_join(vec![modified, float(0.0)]));
    let view_proj = camera_projection_matrix().mul(vec4_join(vec![view_position.xyz(), float(1.0)]));
    view_proj.set_z(view_proj.w())
}

/// `QuadMesh.render()`'s temporary `vertexNode`: a full-screen triangle driven
/// entirely by `vertexIndex`.
pub fn quad_vertex_node() -> NodeRef {
    let x = const_array(vec![-1.0, -1.0, 3.0]).element_node(vertex_index());
    let y = const_array(vec![3.0, -1.0, -1.0]).element_node(vertex_index());
    join(Type::Vec4, vec![x, y, float(0.0), float(1.0)])
}

/// `transpose( m )`.
fn transpose(m: NodeRef) -> NodeRef {
    math_call("transpose", m)
}

/// `WGSLNodeBuilder`'s `inverse( mat3 )` polyfill.
fn inverse_mat3(m: NodeRef) -> NodeRef {
    math_call("tsl_inverse_mat3", m)
}

fn math_call(name: &'static str, m: NodeRef) -> NodeRef {
    let ty = m.ty();
    crate::nodes::NodeRef::new(crate::nodes::Node::Math {
        name,
        args: vec![m],
        ty,
    })
}

/// `RangeNode` on an instanced mesh: one `vec4` per instance, from a uniform
/// buffer indexed by a flat `instanceIndex` varying or, past the uniform buffer
/// limit, from an instanced vertex attribute.
pub fn instanced_range(
    min: impl Into<crate::nodes::tsl::RangeValue>,
    max: impl Into<crate::nodes::tsl::RangeValue>,
    count: usize,
) -> NodeRef {
    crate::nodes::tsl::instanced_range(min, max, count)
}

/// `Renderer._renderOutput()`'s material: the framebuffer texture sampled at
/// the fragment coordinate, through `renderOutput()`.
pub fn output_fragment_node(
    framebuffer: &crate::textures::Texture,
    tone_mapping: ToneMapping,
) -> NodeRef {
    let coord = frag_coord().xy().div(viewport_size());
    let color = texture_uv(framebuffer, coord);
    render_output(color, tone_mapping)
}

/// `RenderOutputNode.setup()` with an sRGB output space: the alpha clamp and
/// unpremultiply, then `toneMapping` — which `ToneMappingNode` applies to the
/// colour only — then the sRGB OETF and the premultiply back.
pub fn render_output(color: NodeRef, tone_mapping: ToneMapping) -> NodeRef {
    let clamped = vec4_join(vec![color.rgb(), color.a().clamp(float(0.0), float(1.0))]);
    let unpremultiplied = unpremultiply_alpha(clamped);
    let mapped = match tone_mapping {
        ToneMapping::None => unpremultiplied,
        ToneMapping::AcesFilmic => vec4_join(vec![
            aces_filmic_tone_mapping(unpremultiplied.clone().rgb(), tone_mapping_exposure()),
            unpremultiplied.a(),
        ]),
    };
    let encoded = vec4_join(vec![srgb_transfer_oetf(mapped.clone().rgb()), mapped.a()]);
    premultiply_alpha(encoded)
}

/// `MeshPhongNodeMaterial`'s fragment flow: `setupDiffuseColor`,
/// `setupVariants` (shininess / specular / emissive), then either the
/// `LightsNode` loop with `PhongLightingModel` or, when `lights === false`,
/// nothing but the diffuse colour. Read off the dumps in
/// `docs/rung5-progress.md` §4; the fog step (§5) is not wired yet.
fn setup_phong(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    fragment: &mut Vec<NodeRef>,
) -> NodeRef {
    // setupDiffuseColor
    let color = match &material.color_node {
        Some(node) => to_vec4(node.clone()),
        None => vec4_join(vec![material_color(), float(1.0)]),
    };
    fragment.push(diffuse_color().assign(color));
    fragment.push(
        diffuse_color()
            .w()
            .assign(diffuse_color().w().mul(material_opacity())),
    );
    // `builder.isOpaque()`
    if material.is_opaque() {
        fragment.push(diffuse_color().w().assign(float(1.0)));
    }

    // setupVariants: `PhongLightingModel` reads these three properties.
    fragment.push(shininess().assign(max(material_shininess(), float(0.0001))));
    let specular = match &material.specular_node {
        Some(node) => node.clone().xyz(),
        None => material_specular(),
    };
    fragment.push(specular_color().assign(specular));
    fragment.push(
        emissive_color().assign(material_emissive().mul(material_emissive_intensity())),
    );

    let outgoing = if material.lights {
        // `LightsNode`: the scene's lights, or the selective subset the
        // material's `lights( [ … ] )` node names.
        let lights: Vec<LightDesc> = match &material.lights_node {
            Some(subset) => ctx
                .lights
                .iter()
                .filter(|light| subset.contains(&light.index))
                .cloned()
                .collect(),
            None => ctx.lights.clone(),
        };
        let ambient: Vec<usize> = lights
            .iter()
            .filter(|light| light.kind == LightKind::Ambient)
            .map(|light| light.index)
            .collect();

        // `AmbientLightNode` sorts first in `LightsNode`'s list, and its
        // `irradiance.addAssign()` is what forces `irradiance = vec3( 0 )` up
        // here rather than down in the indirect tail.
        if !ambient.is_empty() {
            phong::ambient_lights(&ambient, fragment);
        }

        fragment.push(direct_diffuse().assign(vec3(0.0, 0.0, 0.0)));
        fragment.push(direct_specular().assign(vec3(0.0, 0.0, 0.0)));
        for light in &lights {
            if light.kind == LightKind::Ambient {
                continue;
            }
            phong::direct_light(
                light,
                material.received_shadow_position_node.as_ref(),
                fragment,
            );
        }

        // The tail every lit material shares.
        fragment.push(indirect_diffuse().assign(vec3(0.0, 0.0, 0.0)));
        if ambient.is_empty() {
            fragment.push(irradiance().assign(vec3(0.0, 0.0, 0.0)));
        }
        fragment.push(indirect_diffuse().assign(
            vec4_join(vec![indirect_diffuse(), float(1.0)])
                .add(vec4_join(vec![irradiance(), float(1.0)]).mul(
                    diffuse_color().mul(phong::RECIPROCAL_PI),
                ))
                .xyz(),
        ));
        fragment.push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));
        fragment.push(total_diffuse().assign(direct_diffuse().add(indirect_diffuse())));
        fragment.push(total_specular().assign(direct_specular().add(indirect_specular())));
        fragment.push(outgoing_light().assign(total_diffuse().add(total_specular())));
        outgoing_light()
    } else {
        // `lights = false` — the example's light spheres.
        diffuse_color().xyz()
    };

    // `Output = max( vec4( outgoingLight + EmissiveColor, DiffuseColor.w ), 0 )`.
    vec4_join(vec![outgoing.add(emissive_color()), diffuse_color().w()]).max(float(0.0))
}
