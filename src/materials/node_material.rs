//! Port of `three.js/src/materials/nodes/NodeMaterial.js` — `setup()` and the
//! `setupX()` methods it dispatches to. This is where a material turns into the
//! statements the builder flows into the two shader stages.

use super::environment;
use super::phong::{self, LightDesc};
use super::physical::{self, Physical};
use super::{Blending, MaterialKind, MeshBasicNodeMaterial, Side, ToneMapping};
use crate::lights::LightKind;
use crate::nodes::node::Type;
use crate::nodes::tsl::FogNode;
use crate::nodes::tsl::*;
use crate::nodes::{MaterialFlow, NodeRef};

/// The per-render-object facts three.js reads off `builder.object` and
/// `builder.geometry` during setup.
///
/// `Hash` is the scene-dependent half of the render object's cache key —
/// `RenderObject.getDynamicCacheKey()`: the lights (with their shadow maps),
/// the instancing branch and the morph entry. Everything `setup()` reads that
/// is not on the material is here, so the derived hash is complete by
/// construction: a field added to this struct is in the key.
#[derive(Clone, Debug, Default, Hash)]
pub struct SetupContext {
    /// `scene.environmentNode` — the scene-level environment map, which
    /// `NodeMaterial.setupEnvironment()` falls back to when the material has
    /// no `envNode` of its own. `None` for every pass that is not a scene
    /// draw (the background quad, the shadow pass, `render_quad`).
    pub environment: Option<environment::PmremHandle>,
    /// `Some(count)` when the object is an `InstancedMesh`, which is what makes
    /// `NodeMaterial.setupPosition()` insert the `InstanceNode` transform.
    pub instance_count: Option<usize>,
    /// `InstancedMesh.instanceColor` is present, so `range()` resolves against
    /// the instance index.
    pub instanced: bool,
    /// `Some(count)` when the object is an `InstancedMesh` that has had
    /// `setColorAt()` called on it — `NodeMaterial.setupDiffuseColor()`'s
    /// `if ( object.instanceColor )` branch. The count sizes the instanced
    /// vertex buffer; the colours themselves travel with the draw, not with
    /// the node, so two meshes sharing a material cannot share a fill.
    pub instance_color: Option<usize>,
    /// The pass' lights in `Scene.lights` order — the default `LightsNode` list
    /// when the material sets no `lights_node`. Each entry carries the light's
    /// kind and, when this object receives its shadow, the shadow map to sample;
    /// both change the generated code, so they are part of the program's cache
    /// key by construction.
    pub lights: Vec<LightDesc>,
    /// `getEntry( geometry )` when the geometry has morph attributes: what
    /// `NodeMaterial.setupPosition()` needs to emit `morphReference()`.
    pub morph: Option<crate::nodes::morph::MorphEntry>,
    /// `Some` when the object is a `SkinnedMesh` with a skeleton, which is what
    /// makes `NodeMaterial.setupPosition()` insert `skinning( object )`.
    pub skin: Option<crate::nodes::skinning::SkinEntry>,
    /// `object.isBatchedMesh`: the three data textures `batch()` reads.
    pub batch: Option<crate::nodes::batch::BatchEntry>,
    /// A `LineSegmentsGeometry`'s interleaved instanced attributes, which
    /// `Line2NodeMaterial` reads as `instanceStart` / `instanceEnd` and
    /// `instanceColorStart` / `instanceColorEnd`. See
    /// [`crate::nodes::lines`] for why they travel here rather than on the
    /// geometry.
    pub line_segments: Option<crate::nodes::lines::LineSegmentsAttributes>,
    /// `renderer.getMRT()` and the names of the bound render target's colour
    /// attachments — the pass-level half of `NodeMaterial.setup()`'s MRT
    /// branch, which only runs `if ( renderTarget !== null )`.
    ///
    /// `None` is every draw into a single-attachment target, and leaves the
    /// fragment stage's `OutputStruct { color }` shape alone.
    pub mrt: Option<MrtContext>,
    /// `geometry.getAttribute( 'color' ).itemSize` —
    /// `VertexColorNode.generate()`'s
    /// `builder.getTypeFromAttribute( geometryAttribute )`. 4 is glTF `COLOR_0`
    /// as `VEC4` and is read whole; anything else — including 0, a geometry
    /// with no `color` attribute — takes the three-component form, widened to a
    /// `vec4` with an alpha of 1. It is part of the program's cache key,
    /// because it changes the vertex attribute's WGSL type.
    pub vertex_color_size: usize,
    /// `builder.context.getOutput` — the renderer's context node, which
    /// `DirectRenderPipeline` sets so that the output transform is applied
    /// **inside every material's fragment shader** instead of in a quad of its
    /// own. See [`OutputContext`].
    pub output: Option<OutputContext>,
}

/// `context.getOutput( materialOutputNode, builder )`.
///
/// three.js passes a closure, which can look at the builder and hand the
/// material's own output straight back when the draw is not going to the
/// output target. The port makes that decision on the renderer instead — the
/// hook only ever reaches a material that is being drawn into the canvas, so
/// what travels here is just the node the closure would return: the
/// pipeline's `outputNode`, already wrapped in `renderOutput( …, toneMapping,
/// outputColorSpace )`, reading the material's result back through the
/// `Output` property (`output_property()`).
///
/// It is part of `SetupContext`, so it is part of the program's cache key by
/// construction, which is what keeps a material drawn with the hook and the
/// same material drawn without it on two different programs.
#[derive(Clone, Debug)]
pub struct OutputContext {
    pub node: NodeRef,
}

impl std::hash::Hash for OutputContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.node.key().hash(state);
    }
}

/// What a draw into an MRT render target knows about it: the pass's MRT node
/// (`renderer.getMRT()`) and `renderTarget.textures.map( t => t.name )`, which
/// is what `MRTNode.setup()` resolves its output names against.
#[derive(Clone, Debug, Default, Hash)]
pub struct MrtContext {
    pub node: crate::nodes::MrtNode,
    pub attachments: Vec<String>,
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
    material.color_node = Some(match (&source.color_node, &source.map, &source.mask_node) {
        // `hasMap || hasColorNode || hasCastShadowNode || hasMaskNode` is
        // false: the override material keeps its own `vec4( 0, 0, 0, 1 )`,
        // which is a flat four-component constant rather than a join.
        (None, None, None) => vec4(0.0, 0.0, 0.0, 1.0),
        (color, map, _) => {
            // The colour node's alpha when there is one, else the map's.
            let alpha = match (color, map) {
                (Some(color), _) => float(1.0).mul(to_vec4(color.clone()).w()),
                (None, Some(map)) => float(1.0).mul(texture(map).a()),
                (None, None) => float(1.0),
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

/// `NodeMaterial.setupDiffuseColor()` — the step every lighting model shares.
///
/// `materialColor` is `MaterialNode.COLOR`, which is the material's colour
/// *times the map's texel* when the material has one (`MaterialNode.js:126`):
/// that is why `DiffuseColor` is assigned a `vec4` product rather than a `vec3`
/// promoted to one, and why an unlit `MeshBasicNodeMaterial { map }` is
/// textured on exactly the same terms as a Standard one. A `colorNode`
/// replaces the pair outright, map included —
/// `this.colorNode ? vec4( this.colorNode ) : materialColor`.
fn setup_diffuse_color(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    fragment: &mut Vec<NodeRef>,
) {
    let color = match &material.color_node {
        Some(node) => to_vec4(node.clone()),
        // `materialColor` is a vec3 (times `map` when there is one) and stays
        // one until `diffuseColor.assign()` widens it, which is what puts the
        // `vec4<f32>( … , 1.0 )` on the outside of the instance-colour product
        // rather than around the uniform alone.
        None => {
            let base = material_color();
            match &material.map {
                Some(map) => base.mul(texture(map)),
                None => base,
            }
        }
    };

    // `if ( this.vertexColors === true && geometry.hasAttribute( 'color' ) )
    // colorNode = colorNode.mul( vertexColor() )` — the same step for every
    // lighting model, which is what lets one `LineSegments` carry a hue per
    // vertex. See `vertex_color()` for the `hasAttribute` half.
    // `Line2NodeMaterial` is the exception: a `LineSegmentsGeometry` has no
    // `color` attribute (it carries `instanceColorStart` / `instanceColorEnd`),
    // so three's `geometry.hasAttribute( 'color' )` is false and the base
    // multiply is skipped. Its own `setupDiffuseColor()` selects the colour per
    // end instead — see [`crate::materials::line2::setup_diffuse_color`].
    let color = match material.vertex_colors && material.kind != MaterialKind::Line2 {
        true => color.mul(vertex_color(ctx.vertex_color_size)),
        false => color,
    };

    // `if ( object.instanceColor ) colorNode = instanceColor.mul( colorNode )`
    // — the varying multiplies on the *left*, which is what puts
    // `vInstanceColor` first in the dump's `DiffuseColor = vec4<f32>( (
    // vInstanceColor * object.nodeUniform2 ), 1.0 )`.
    let color = match ctx.instance_color {
        Some(count) => instance_color(count).mul(color),
        None => color,
    };

    // `if ( object.isBatchedMesh && object._colorsTexture )
    // colorNode = batchColor.mul( colorNode )` — the varying is the *left*
    // operand, which is what puts `vBatchColor` first in the generated line.
    // three.js runs this after the instanced-colour step, so the port does too.
    let color = match ctx.batch.as_ref().and_then(|b| b.colors.as_ref()) {
        Some(_) => crate::nodes::batch::batch_color().mul(color),
        None => color,
    };

    fragment.push(diffuse_color().assign(color));

    // `const opacityNode = this.opacityNode ? float( this.opacityNode ) :
    // materialOpacity` — a node *replaces* the uniform rather than scaling it,
    // so a material with an `opacityNode` never reads `material.opacity`.
    let opacity = match &material.opacity_node {
        Some(node) => to_float(node.clone()),
        None => material_opacity(),
    };
    fragment.push(diffuse_color().w().assign(diffuse_color().w().mul(opacity)));

    // `if ( this.alphaTestNode !== null || this.alphaTest > 0 )
    // diffuseColor.a.lessThanEqual( alphaTestNode ).discard()`. It is *after*
    // the opacity multiply and *before* the opaque clamp, so the alpha the test
    // sees is the one the opacity node produced, and the fragments that survive
    // still get `w = 1.0` on an opaque material — the alpha-test teapot is not
    // `transparent`, and the dump shows both lines.
    if let Some(node) = &material.alpha_test_node {
        let alpha_test = to_float(node.clone());
        fragment.push(if_then(
            diffuse_color().w().less_than_equal(alpha_test),
            vec![discard()],
        ));
    }

    // `builder.isOpaque()` — the material is not transparent, blending is
    // NormalBlending and alphaToCoverage is off. A transparent or blended
    // material keeps its per-fragment alpha instead, all the way to `Output`.
    if material.is_opaque() {
        fragment.push(diffuse_color().w().assign(float(1.0)));
    }

    // `Line2NodeMaterial.setupDiffuseColor()` runs `super.setupDiffuseColor()`
    // first and then adds the coverage multiply and the per-end colour. Its
    // `blending = NoBlending` is why `is_opaque()` above is false and the
    // `DiffuseColor.w = 1.0` line is absent from three's dump.
    if material.kind == MaterialKind::Line2 {
        if let Some(attributes) = &ctx.line_segments {
            crate::materials::line2::setup_diffuse_color(
                material.alpha_to_coverage,
                material.vertex_colors,
                attributes,
                fragment,
            );
        }
    }
}

/// `float( node )` — `NodeBuilder.format()` narrowing to one component, which
/// is what `setupDiffuseColor()` wraps `opacityNode` and `alphaTestNode` in.
fn to_float(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::F32 => node,
        Type::Vec2 | Type::Vec3 | Type::Vec4 => node.x(),
        _ => node.to(Type::F32),
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
    // `MaterialNode.NORMAL`: with no `normalNode` of its own, a material with a
    // `bumpMap` normal-maps through `BumpMapNode`.
    // `MaterialNode.NORMAL`: `normalMap` first, then `bumpMap`, then the
    // geometry's own `normalView`.
    let normal = with_material_side(material.side, || {
        match (
            &material.normal_node,
            &material.normal_map,
            &material.bump_map,
        ) {
            (Some(node), _, _) => Some(node.clone()),
            (None, Some(map), _) => Some(normal_map_scaled(texture(map), material_normal_scale())),
            (None, None, Some(bump)) => Some(bump_map(bump, material_bump_scale())),
            (None, None, None) => None,
        }
    });
    // `builder.context.setupPositionView = () => this.setupPositionView(
    // builder )` — the seam `SpriteNodeMaterial` overrides. Built here, before
    // either stage is flowed, exactly as `NodeMaterial.setup()` installs it.
    let position_view = match material.kind {
        MaterialKind::Sprite => Some(setup_position_view_sprite(material)),
        MaterialKind::Points => Some(setup_position_view_points(material)),
        _ => None,
    };
    with_material_normal(normal, material.flat_shading, material.side, || {
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

    // `Line2NodeMaterial.setupPosition()` overrides the whole thing and calls
    // `super.setupPosition()` last, so its `positionLocal.assign()` is the
    // first statement of the vertex flow — ahead of morphing and skinning,
    // neither of which a fat line has.
    if material.kind == MaterialKind::Line2 {
        if let Some(attributes) = &ctx.line_segments {
            pre_vertex.push(crate::materials::line2::setup_position(attributes));
        }
    }

    // --- setupPosition: the `context.position` stack, flowed into the vertex
    // stage before either stage's own flow. Morphing, skinning and batching
    // plug in here too.
    // `if ( object.morphTargetInfluences ) morphReference( object ).append()` —
    // first in `setupPosition`, before skinning, displacement, batching and
    // instancing.
    if let Some(entry) = &ctx.morph {
        pre_vertex.extend(crate::nodes::morph::morph_reference(entry));
    }

    // `if ( object.isSkinnedMesh === true ) skinning( object ).append()` —
    // second, so the bind matrix sees the morphed position.
    if let Some(entry) = &ctx.skin {
        pre_vertex.extend(crate::nodes::skinning::skinning(entry));
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
    // `if ( object.isBatchedMesh ) batch( object )` — `NodeMaterial.js:792`,
    // after morphing and displacement and before instancing.
    if let Some(entry) = &ctx.batch {
        pre_vertex.extend(crate::nodes::batch::batch(entry));
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
        setup_phong(material, ctx, true, &mut fragment)
    } else if material.kind == MaterialKind::Lambert {
        setup_phong(material, ctx, false, &mut fragment)
    } else if material.kind == MaterialKind::Standard || material.kind == MaterialKind::Physical {
        setup_standard(material, ctx, &mut fragment)
    } else if material.kind == MaterialKind::Normal {
        // `MeshNormalNodeMaterial.setupDiffuseColor()` replaces the base
        // implementation outright: no `colorNode`, no vertex colours, no alpha
        // test, and no `builder.isOpaque()` clamp — the opacity node (or the
        // `materialOpacity` uniform) lands directly in the `vec4`'s `w`.
        //
        // "By convention, a normal packed to RGB is in sRGB color space", so
        // the packed value is decoded into the working space on the way in;
        // that is where `sRGBTransferEOTF` comes from, and it is the only
        // material on the ladder that emits the EOTF rather than the OETF.
        let opacity = match &material.opacity_node {
            Some(node) => to_float(node.clone()),
            None => material_opacity(),
        };
        fragment.push(diffuse_color().assign(srgb_to_working(vec4_join(vec![
            pack_normal_to_rgb(normal_view()),
            opacity,
        ]))));
        vec4_join(vec![diffuse_color().xyz(), diffuse_color().w()]).max(float(0.0))
    } else {
        setup_diffuse_color(material, ctx, &mut fragment);

        let outgoing = if let Some(env_map) = &material.env_map {
            // `BasicLightingModel` with an indirect environment contribution.
            fragment.push(
                indirect_diffuse().assign(
                    vec4_join(vec![indirect_diffuse(), float(1.0)])
                        .add(vec4(1.0, 1.0, 1.0, 0.0))
                        .xyz(),
                ),
            );
            fragment.push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));
            fragment.push(indirect_diffuse().assign(indirect_diffuse().mul(diffuse_color().xyz())));
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

        // `setupLighting()`'s EMISSIVE tail. `MeshBasicMaterial` has no
        // `emissive` colour, so an unlit material reaches it only through
        // `emissiveNode`: `EmissiveColor = vec3( emissiveNode )`, then
        // `outgoingLight = outgoingLight + EmissiveColor`.
        let outgoing = match &material.emissive_node {
            Some(node) => {
                fragment.push(emissive_color().assign(to_vec3(node.clone())));
                outgoing.add(emissive_color())
            }
            None => outgoing,
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

    // `NodeMaterial.setupOutput()`'s second half: `if ( this.premultipliedAlpha
    // === true ) outputNode = premultiplyAlpha( outputNode )`, after the fog
    // and before the output assignment. The SSAA accumulation quad is the one
    // caller — its dump is `output.color = fn0( fn1( … ) )` with `fn0` the
    // premultiply and `fn1` `unpremultiplyAlpha` from its own fragment node.
    let output = if material.premultiplied_alpha {
        premultiply_alpha(output)
    } else {
        output
    };

    // `NodeMaterial.setup()`'s MRT branch, which runs only `if ( renderTarget
    // !== null )`: the pass's MRT and the material's are merged with the
    // material's winning, and the merged node — not the colour — becomes the
    // fragment stage's result. The `Output` property assignment above has
    // already happened, which is what lets the MRT's `output` member read it
    // back (`output.m0 = Output;` in three's dump).
    let mrt = ctx.mrt.as_ref().map(|context| {
        let merged = match &material.mrt_node {
            Some(material_mrt) => context.node.merge(material_mrt),
            None => context.node.clone(),
        };
        merged.members(&context.attachments)
    });

    // --- the vertex flow
    let position = match &material.vertex_node {
        Some(node) => node.clone(),
        None => model_view_projection(),
    };

    // `if ( builder.context.getOutput ) resultNode = builder.context.getOutput(
    // resultNode, builder );` — `DirectRenderPipeline`'s hook, which does
    // `output.assign( materialOutputNode )` and then returns its own node. The
    // assign is a *second* write to the `Output` property, on top of
    // `NodeMaterial.setup()`'s own, and three's dump has both lines.
    //
    // The MRT branch runs only with a render target bound, and the hook only
    // without one, so the two never meet.
    let (output_assign, output_node) = match &ctx.output {
        Some(context) => (
            Some(
                material
                    .output_node
                    .clone()
                    .unwrap_or_else(|| output.clone()),
            ),
            Some(context.node.clone()),
        ),
        None => (None, material.output_node.clone()),
    };

    MaterialFlow {
        pre_vertex_statements: pre_vertex,
        fragment_statements: fragment,
        output,
        output_assign,
        output_node,
        mrt,
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
        vec![
            mv_position.xy().add(rotate(aligned, rotation)),
            mv_position.zw(),
        ],
    )
}

/// `PointsNodeMaterial.setupPositionView()`
/// (`src/materials/nodes/PointsNodeMaterial.js:81-87`):
///
/// ```ignore
/// return modelViewMatrix.mul( vec3( this.positionNode || positionLocal ) ).xyz;
/// ```
///
/// It is `NodeMaterial`'s own formula with one difference that shows in the
/// generated WGSL: the position node is read *again* here rather than through
/// the `positionLocal` var `setupPosition()` just assigned it to, so the
/// vertex shader indexes the storage buffer twice. `mul` on a `mat4` and a
/// `vec3` is `NodeBuilder.format( snippet, 'vec3', 'vec4' )`, i.e.
/// `vec4<f32>( v, 1.0 )` — spelt out here, as
/// [`transform_direction`](crate::nodes::tsl::transform_direction) does.
fn setup_position_view_points(material: &MeshBasicNodeMaterial) -> NodeRef {
    let position = match &material.position_node {
        Some(node) => to_vec3(node.clone()),
        None => position_local(),
    };
    model_view_matrix()
        .mul(vec4_join(vec![position, float(1.0)]))
        .xyz()
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

/// `scene.background = <a generated PMREM>`: the skybox sphere sampling the
/// cubeUV atlas, which is the `isNode` branch with
/// `NodeManager.getBackgroundNode()`'s `pmremTexture( background )` inside it
/// and the node context supplying the two accessors.
///
/// `getUV: () => backgroundRotation.mul( normalWorldGeometry )` and
/// `getTextureLevel: () => backgroundBlurriness` are the context; the port has
/// no node context, so they are passed as arguments, which is the same graph.
/// `backgroundRotation` is a `mat4`, so the product is a `vec4` and
/// `PMREMNode`'s Y flip swizzles out of it — three's own dump reads
/// `vec3( nodeVar3.x, - nodeVar3.y, nodeVar3.z )` off exactly that.
pub fn background_pmrem_color_node(pmrem: &crate::materials::environment::PmremHandle) -> NodeRef {
    let uv = background_rotation().mul(vec4_join(vec![normal_world_geometry(), float(1.0)]));
    background_node_color_node(pmrem.sample(uv, background_blurriness()))
}

/// `Background.update()`'s `isNode` branch:
/// `vec4( backgroundNode ).mul( backgroundIntensity )`.
pub fn background_node_color_node(node: NodeRef) -> NodeRef {
    vec4_join(vec![node, float(1.0)]).mul(background_intensity())
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
    let view_proj =
        camera_projection_matrix().mul(vec4_join(vec![view_position.xyz(), float(1.0)]));
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
        // `outputNode.toneMapping( toneMapping )` — `ToneMappingNode` keeps the
        // alpha and tone maps the colour with `toneMappingExposure`.
        ToneMapping::Linear => vec4_join(vec![
            linear_tone_mapping(unpremultiplied.clone().rgb(), tone_mapping_exposure()),
            unpremultiplied.a(),
        ]),
        ToneMapping::Reinhard => vec4_join(vec![
            reinhard_tone_mapping(unpremultiplied.clone().rgb(), tone_mapping_exposure()),
            unpremultiplied.a(),
        ]),
        ToneMapping::AcesFilmic => vec4_join(vec![
            aces_filmic_tone_mapping(unpremultiplied.clone().rgb(), tone_mapping_exposure()),
            unpremultiplied.a(),
        ]),
        ToneMapping::Neutral => vec4_join(vec![
            neutral_tone_mapping(unpremultiplied.clone().rgb(), tone_mapping_exposure()),
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
///
/// `specular == false` is `MeshLambertNodeMaterial`, which is this same flow
/// under `new PhongLightingModel( false )`. Three things fall out of the flag,
/// all of them visible in `dump-postprocessing_ca/m07`:
///
/// * `MeshPhongNodeMaterial.setupVariants()`'s `shininess` and `specularColor`
///   are not emitted — they are the Phong *material*'s, not the model's;
/// * `PhongLightingModel.direct()` skips its `directSpecular.addAssign`;
/// * and so `directSpecular` / `indirectSpecular` are first *read* in the
///   shared tail, which is where three's `PropertyNode` declares them, after
///   `totalDiffuse` rather than before the light loop.
fn setup_phong(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    specular: bool,
    fragment: &mut Vec<NodeRef>,
) -> NodeRef {
    setup_diffuse_color(material, ctx, fragment);

    // setupVariants: `PhongLightingModel` reads these three properties.
    if specular {
        fragment.push(shininess().assign(max(material_shininess(), float(0.0001))));
        let specular_value = match &material.specular_node {
            Some(node) => node.clone().xyz(),
            None => material_specular(),
        };
        fragment.push(specular_color().assign(specular_value));
    }
    fragment.push(emissive_color().assign(material_emissive().mul(material_emissive_intensity())));

    let outgoing = if material.lights {
        // `LightsNode`: the scene's lights, or the selective subset the
        // material's `lights( [ … ] )` node names.
        let lights = material_lights(material, ctx);
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
        if specular {
            fragment.push(direct_specular().assign(vec3(0.0, 0.0, 0.0)));
        }
        for light in &lights {
            if light.kind == LightKind::Ambient {
                continue;
            }
            phong::direct_light(
                light,
                material.received_shadow_position_node.as_ref(),
                specular,
                fragment,
            );
        }

        // The tail every lit material shares.
        fragment.push(indirect_diffuse().assign(vec3(0.0, 0.0, 0.0)));
        if ambient.is_empty() {
            fragment.push(irradiance().assign(vec3(0.0, 0.0, 0.0)));
        }
        fragment.push(
            indirect_diffuse().assign(
                vec4_join(vec![indirect_diffuse(), float(1.0)])
                    .add(
                        vec4_join(vec![irradiance(), float(1.0)])
                            .mul(diffuse_color().mul(phong::RECIPROCAL_PI)),
                    )
                    .xyz(),
            ),
        );
        fragment.push(indirect_diffuse().assign(indirect_diffuse().mul(ambient_occlusion())));
        fragment.push(total_diffuse().assign(direct_diffuse().add(indirect_diffuse())));
        if !specular {
            // Lambert reads both specular accumulators for the first time
            // here, so this is where three declares them.
            fragment.push(direct_specular().assign(vec3(0.0, 0.0, 0.0)));
            fragment.push(indirect_specular().assign(vec3(0.0, 0.0, 0.0)));
        }
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

/// `LightsNode`'s list as one material sees it: the scene's lights, or the
/// selective subset the material's `lights( [ … ] )` node names.
fn material_lights(material: &MeshBasicNodeMaterial, ctx: &SetupContext) -> Vec<LightDesc> {
    match &material.lights_node {
        Some(subset) => ctx
            .lights
            .iter()
            .filter(|light| subset.contains(&light.index))
            .cloned()
            .collect(),
        None => ctx.lights.clone(),
    }
}

/// `NodeMaterial.setupAmbientOcclusion()` — `AmbientOcclusion.assign( aoNode )`
/// when, and only when, the material has an `aoMap`.
///
/// `materialAO` is `texture( aoMap ).r.sub( 1 ).mul( aoMapIntensity ).add( 1 )`.
/// Nothing is emitted without the map: three leaves `aoNode` null, so the
/// property is never declared and the lighting model's own `ambientOcclusion`
/// var stays a bare `1`.
fn setup_ambient_occlusion(material: &MeshBasicNodeMaterial, fragment: &mut Vec<NodeRef>) {
    let Some(map) = &material.ao_map else {
        return;
    };

    fragment.push(
        ambient_occlusion_property().assign(
            texture(map)
                .x()
                .sub(float(1.0))
                .mul(material_ao_map_intensity())
                .add(float(1.0)),
        ),
    );
}

/// `MaterialNode.EMISSIVE` — `emissive * emissiveIntensity`, times the
/// `emissiveMap` when there is one.
///
/// The map multiply is three's `emissiveNode.mul( this.getTexture( scope ) )`:
/// a `vec3` times a `vec4`, which `MathNode`'s type resolution widens to
/// `vec4( emissive, 1 ) * tex` before the assignment to the `vec3` property
/// takes `.xyz`. That is what the dump's
/// `( vec4<f32>( ( emissive * intensity ), 1.0 ) * tex ).xyz` is, and why the
/// port builds the `vec4` explicitly rather than multiplying three components.
fn material_emissive_value(material: &MeshBasicNodeMaterial) -> NodeRef {
    let emissive = material_emissive().mul(material_emissive_intensity());

    match &material.emissive_map {
        Some(map) => vec4_join(vec![emissive, float(1.0)])
            .mul(texture(map))
            .xyz(),
        None => emissive,
    }
}

/// `MeshStandardNodeMaterial`'s fragment flow: `setupDiffuseColor`,
/// `setupVariants` (metalness / roughness / specular / diffuse contribution),
/// then the `LightsNode` loop with `PhysicalLightingModel`. Read off
/// `handoff/scouts/rung8/MeshStandardMaterial_1{7,8,9}.frag-r186.wgsl`.
fn setup_standard(
    material: &MeshBasicNodeMaterial,
    ctx: &SetupContext,
    fragment: &mut Vec<NodeRef>,
) -> NodeRef {
    // --- setupDiffuseColor
    setup_diffuse_color(material, ctx, fragment);

    // --- setupAmbientOcclusion, between `setupDiffuseColor` and
    // `setupVariants` exactly as `NodeMaterial.setup()` orders them.
    setup_ambient_occlusion(material, fragment);

    // --- setupVariants. `metalnessNode` is reached twice — once for the
    // `Metalness` property and once for `DiffuseContribution` — so the node is
    // shared, exactly as three.js shares the `materialMetalness` node.
    let metalness_node = match &material.metalness_map {
        // glTF packing: metalness in blue, roughness in green.
        Some(map) => material_metalness().mul(texture(map).z()),
        None => material_metalness(),
    };
    fragment.push(metalness().assign(metalness_node.clone()));

    let roughness_node = match &material.roughness_map {
        Some(map) => material_roughness().mul(texture(map).y()),
        None => material_roughness(),
    };
    fragment.push(roughness().assign(physical::get_roughness(roughness_node)));

    if material.kind == MaterialKind::Physical {
        // `MeshPhysicalNodeMaterial.setupSpecular()`: F0 from the index of
        // refraction rather than the dielectric constant 0.04, tinted by
        // `specularColor` (times `specularColorMap`) and scaled by
        // `specularIntensity` — which is also the F90. This block is the entire
        // delta over the Standard material above.
        fragment.push(ior().assign(material_ior()));
        // `pow2( ior.sub( 1 ).div( ior.add( 1 ) ) )` — the quotient is read
        // twice, so it is a var.
        let f0 = to_var(None, ior().sub(float(1.0)).div(ior().add(float(1.0))));
        let specular = match &material.specular_color_map {
            Some(map) => material_specular_color().mul(texture(map).xyz()),
            None => material_specular_color(),
        };
        fragment.push(
            specular_color().assign(
                f0.clone()
                    .mul(f0)
                    .mul(specular)
                    .min(vec3(1.0, 1.0, 1.0))
                    .mul(material_specular_intensity()),
            ),
        );
        fragment.push(specular_color_blended().assign(mix(
            specular_color(),
            diffuse_color().xyz(),
            metalness(),
        )));
        fragment.push(specular_f90().assign(mix(
            material_specular_intensity(),
            float(1.0),
            metalness(),
        )));
    } else {
        // `setupSpecular()`: a dielectric F0 of 0.04, blended towards the
        // albedo by metalness, and an F90 of 1.
        fragment.push(specular_color().assign(vec3(0.04, 0.04, 0.04)));
        fragment.push(specular_color_blended().assign(mix(
            vec3(0.04, 0.04, 0.04),
            diffuse_color().rgb(),
            metalness(),
        )));
        fragment.push(specular_f90().assign(float(1.0)));
    }
    fragment
        .push(diffuse_contribution().assign(diffuse_color().rgb().mul(metalness_node.one_minus())));

    fragment.push(emissive_color().assign(material_emissive_value(material)));

    let outgoing = if material.lights {
        let model = Physical::start();
        let lights = material_lights(material, ctx);

        // `LightingContextNode`'s five accumulators. three.js declares each at
        // the point of its first use; hoisting the zeros here is the one
        // reordering in this flow (see `docs/nodes.md` §8) and reads nothing
        // before it is written either way.
        fragment.push(direct_diffuse().assign(vec3(0.0, 0.0, 0.0)));
        fragment.push(direct_specular().assign(vec3(0.0, 0.0, 0.0)));
        fragment.push(irradiance().assign(vec3(0.0, 0.0, 0.0)));
        fragment.push(indirect_diffuse().assign(vec3(0.0, 0.0, 0.0)));
        fragment.push(indirect_specular().assign(vec3(0.0, 0.0, 0.0)));

        for light in &lights {
            physical::direct_light(
                &model,
                light,
                material.received_shadow_position_node.as_ref(),
                fragment,
            );
        }

        model.indirect_diffuse(fragment);
        // `EnvironmentNode` is a lighting node, so its two `addAssign`s land
        // between `indirectDiffuse()` and `indirectSpecular()` — and with them
        // the declarations of `radiance` and `iblIrradiance`.
        let env = material.pmrem_env.as_ref().or(ctx.environment.as_ref());
        if let Some(environment) = env {
            environment::setup(environment, fragment);
        }
        // `AONode( context.ambientOcclusion )`, the last entry
        // `setupLightsNode()` pushes: `ambientOcclusion.mulAssign( aoNode )`,
        // where `aoNode` is the `AmbientOcclusion` property
        // `setup_ambient_occlusion` wrote above. The var's `float( 1 )`
        // initialiser is emitted here, at its first read.
        if material.ao_map.is_some() {
            fragment.push(
                ambient_occlusion().assign(ambient_occlusion().mul(ambient_occlusion_property())),
            );
        }
        model.indirect_specular(env.is_some(), fragment);
        model.ambient_occlusion(material.ao_map.is_some(), fragment);

        fragment.push(total_diffuse().assign(direct_diffuse().add(indirect_diffuse())));
        fragment.push(total_specular().assign(direct_specular().add(indirect_specular())));
        fragment.push(outgoing_light().assign(total_diffuse().add(total_specular())));
        outgoing_light()
    } else {
        diffuse_color().xyz()
    };

    vec4_join(vec![outgoing.add(emissive_color()), diffuse_color().w()]).max(float(0.0))
}
