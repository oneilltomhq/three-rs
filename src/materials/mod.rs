//! Ports of `three.js/src/materials/nodes` — under `WebGPURenderer` every
//! material is a `NodeMaterial`, so this is the only material path.

pub(crate) mod blending;
mod dfg_lut;
pub mod environment;
pub mod lighting_model;
pub(crate) mod line2;
mod node_material;
pub mod phong;
pub mod physical;
pub(crate) mod toon;
pub mod transmission;

pub use node_material::{
    background_color_node, background_environment_color_node, background_node_color_node,
    background_pmrem_color_node, background_vertex_node, instanced_range, output_fragment_node,
    quad_vertex_node, render_output, setup, shadow_material, shadow_material_for,
    tone_mapping_node, MrtContext, OutputContext, SetupContext,
};

pub(crate) use blending::BlendMode;
pub use blending::{BlendEquation, BlendFactor, Blending};

use std::cell::Cell;

use crate::math::Color;
use crate::nodes::NodeRef;
use crate::textures::{CubeTexture, Texture};

/// `Material.id` — three.js' module-level `let _materialId = 0` counter, handed
/// out in construction order. It is what the renderer's program cache keys a
/// material on (`RenderObjects.get()` chains on the material object itself),
/// so it has to behave like an object identity even though the material is a
/// value here:
///
/// - every `MeshBasicNodeMaterial::new()` / `default()` gets a fresh id;
/// - **`clone()` gets a fresh id too**, as `Material.clone()` does — `new
///   this.constructor().copy( this )` is a new object with a new `id`. A clone
///   that later diverges (a different `map`, say) therefore can never be
///   served the original's program.
///
/// The renderer snapshots a material into its render list by cloning it, and
/// takes the key from the source before it does, so that per-frame copy is not
/// a new material as far as the cache is concerned.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(usize);

impl MaterialId {
    fn next() -> Self {
        thread_local! {
            static MATERIAL_ID: Cell<usize> = const { Cell::new(0) };
        }
        MATERIAL_ID.with(|id| {
            let next = id.get();
            id.set(next + 1);
            MaterialId(next)
        })
    }

    /// The number itself, for keying on.
    pub fn get(&self) -> usize {
        self.0
    }
}

/// A fresh id, never a copy — see the type's docs.
impl Clone for MaterialId {
    fn clone(&self) -> Self {
        Self::next()
    }
}

/// `three.js/src/constants.js` sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// `FrontSide`.
    Front,
    /// `BackSide`.
    Back,
    /// `DoubleSide` — `_getPrimitiveState()` leaves `cullMode` at `'none'`, so
    /// the front-face winding no longer matters. `BatchedText`'s material.
    Double,
}

/// `three.js/src/constants.js` depth functions — `Material.depthFunc`, which
/// `WebGPUPipelineUtils._getDepthCompare()` maps one-to-one onto a WebGPU
/// compare function when `depthTest` is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DepthFunc {
    /// `NeverDepth`.
    Never,
    /// `AlwaysDepth`.
    Always,
    /// `LessDepth`.
    Less,
    /// `LessEqualDepth` — three's default.
    #[default]
    LessEqual,
    /// `EqualDepth`.
    Equal,
    /// `GreaterEqualDepth`.
    GreaterEqual,
    /// `GreaterDepth`.
    Greater,
    /// `NotEqualDepth`.
    NotEqual,
}

/// `three.js/src/constants.js` tone-mapping modes — the ones the port needs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToneMapping {
    /// `NoToneMapping`.
    #[default]
    None,
    /// `ReinhardToneMapping`.
    Reinhard,
    /// `ACESFilmicToneMapping`.
    AcesFilmic,
    /// `LinearToneMapping` — `clamp( color * exposure, 0, 1 )`.
    Linear,
    /// `NeutralToneMapping` — the Khronos PBR Neutral tone mapper.
    Neutral,
    /// `AgXToneMapping` — Blender's AgX, through Rec. 2020.
    AgX,
}

/// Which `NodeMaterial` subclass this is — i.e. which `setupLightingModel()`
/// the fragment flow runs. three.js models it with subclasses; here the flow is
/// one function with a `match`, so a new lighting model cannot be silently
/// skipped in one of the setup steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MaterialKind {
    /// `MeshBasicNodeMaterial` — `BasicLightingModel`, `lights = false`.
    #[default]
    Basic,
    /// `MeshPhongNodeMaterial` — `PhongLightingModel`.
    Phong,
    /// `MeshLambertNodeMaterial` — `new PhongLightingModel( false )`, three's
    /// own comment being "( specular ) -> force lambert". The same flow as
    /// [`Phong`](MaterialKind::Phong) with the Blinn-Phong specular lobe and
    /// the two material properties that feed it removed.
    Lambert,
    /// `SpriteNodeMaterial` — `BasicLightingModel` like `Basic`, but it
    /// overrides `setupPositionView()` with the billboarded quad.
    Sprite,
    /// `MeshStandardNodeMaterial` — `PhysicalLightingModel` with a constant
    /// dielectric F0 of 0.04 and an F90 of 1.
    Standard,
    /// `MeshPhysicalNodeMaterial` — the same lighting model, but
    /// `setupSpecular()` derives F0 from the `ior` and modulates it by
    /// `specularColor` / `specularColorMap` / `specularIntensity`. That block
    /// is the whole delta over `Standard`; `GLTFLoader` picks it whenever the
    /// asset uses `KHR_materials_specular` or `KHR_materials_ior`.
    Physical,
    /// `PointsNodeMaterial extends SpriteNodeMaterial`. `setupVertex()` returns
    /// `super.setupVertex()` — the plain MVP path — when `object.isPoints`, so
    /// the sprite quad expansion is *not* taken; the only thing it keeps from
    /// `SpriteNodeMaterial` is `setupPositionView()` and `transparent = true`.
    Points,
    /// `MeshNormalNodeMaterial` — no lighting model at all. It overrides
    /// `setupDiffuseColor()` outright with the packed view-space normal, so it
    /// takes neither the `colorNode` path nor the opacity / alpha-test /
    /// opaque-clamp tail.
    Normal,
    /// `Line2NodeMaterial` — the fat line. `BasicLightingModel` like `Basic`,
    /// plus an overridden `setupPosition()` (the screen-space quad) and
    /// `setupDiffuseColor()` (round-cap coverage, per-end instance colour).
    /// See `crate::materials::line2`.
    Line2,
    /// `MeshToonNodeMaterial` — `ToonLightingModel`: the Lambert flow with
    /// the direct light's `dotNL` replaced by a lookup in
    /// [`gradient_map`](MeshBasicNodeMaterial::gradient_map) (or a fixed
    /// two-step ramp without one). See `crate::materials::toon`.
    Toon,
}

/// Port of `MeshBasicNodeMaterial.js` + the `NodeMaterial.js` / `Material.js`
/// fields the ladder uses. Defaults mirror three.js: white, opaque,
/// `FrontSide`, depth test on with `LessEqualDepth`, depth write on,
/// `reflectivity = 1`.
///
/// # Changing a material after its first frame
///
/// The renderer builds a material's shader program once and looks it up by
/// `id` and `version` from then on (`docs/scene-graph.md`, "Program cache"),
/// exactly as `WebGPURenderer` does. So the rule is three.js':
///
/// - a field the *program* depends on — any node (`color_node`,
///   `position_node`, `fragment_node`, …), any map or `env_map`, `kind`,
///   `lights`, `lights_node`, `lighting_model`, `flat_shading`, `fog`, `transparent`,
///   `blending`, `alpha_to_coverage`, `world_units`, `size_attenuation`, `mask_node`, `cast_shadow_node` — needs
///   [`set_needs_update`](Self::set_needs_update) after it changes, which is
///   `material.needsUpdate = true`. Without it the old program keeps drawing.
/// - a field the program reads as a **uniform** — `color`, `opacity`,
///   `specular`, `shininess`, `emissive`, `emissive_intensity`, `metalness`,
///   `roughness`, `bump_scale`, `rotation`, `reflectivity` — is uploaded every
///   frame and needs nothing, as in three.js.
/// - `side`, `depth_test`, `depth_write` and the blend factors are pipeline
///   state, keyed per draw, and need nothing either.
#[derive(Clone, Debug)]
pub struct MeshBasicNodeMaterial {
    /// `Material.id`. Read-only in spirit; see [`MaterialId`] for why a clone
    /// gets a new one.
    pub id: MaterialId,
    /// `Material.version` — "starts at 0 and counts how many times
    /// `needsUpdate` is set to true". Part of the program cache key.
    pub version: u32,
    /// Which `NodeMaterial` subclass this stands in for — see [`MaterialKind`].
    pub kind: MaterialKind,
    /// `Material.color` — `MeshBasicMaterial`'s diffuse colour.
    pub color: Color,
    /// `Material.opacity`.
    pub opacity: f64,
    /// `MeshBasicMaterial.reflectivity` — the mix factor `BasicEnvironmentNode`
    /// blends the reflected environment colour in by.
    pub reflectivity: f64,
    /// `MeshBasicMaterial.envMap` — `setupEnvironment()` turns it into
    /// `BasicEnvironmentNode( cubeTexture( envMap ) )`.
    pub env_map: Option<CubeTexture>,
    /// `MeshStandardMaterial.envMap` on a PBR material, which
    /// `NodeMaterial.setupEnvironment()` turns into `EnvironmentNode(
    /// pmremTexture( envMap ) )` rather than a plain cube read. The handle
    /// comes from a
    /// [`PmremEnvironment`](crate::nodes::pmrem_node::PmremEnvironment), which
    /// owns the generated atlas; `scene.environment` reaches the same place by
    /// being copied onto every material that has none of its own.
    pub pmrem_env: Option<environment::PmremHandle>,
    /// `NodeMaterial.colorNode` — replaces the `materialColor` uniform in
    /// `setupDiffuseColor()`.
    pub color_node: Option<NodeRef>,
    /// `NodeMaterial.opacityNode` — replaces the `materialOpacity` uniform in
    /// `setupDiffuseColor()`, so `DiffuseColor.a` is multiplied by a node's
    /// value instead. `float( opacityNode )` narrows a wider node to its first
    /// component, which is how `opacityNode = texture( map )` becomes
    /// `DiffuseColor.w * texel.x`.
    pub opacity_node: Option<NodeRef>,
    /// `NodeMaterial.alphaTestNode` — `diffuseColor.a.lessThanEqual( node
    /// ).discard()` at the end of `setupDiffuseColor()`.
    ///
    /// Takes precedence over [`alpha_test`](Self::alpha_test), as in three.
    pub alpha_test_node: Option<NodeRef>,
    /// `Material.alphaTest` — when positive, `setupDiffuseColor()` discards
    /// against the `materialAlphaTest` object uniform
    /// (`webgpu_shadowmap_pointlight`'s spheres use 0.5). The shadow pass
    /// copies it onto its override material, so cut-away texels cast no
    /// shadow either.
    pub alpha_test: f64,
    /// `Material.alphaHash` — when set, `setupDiffuseColor()` discards the
    /// fragments whose alpha is below `getAlphaHashThreshold( positionLocal )`
    /// (see `crate::nodes::alpha_hash`), the stochastic stand-in for
    /// blending that `webgpu_materials_alphahash` renders.
    pub alpha_hash: bool,
    /// `Material.alphaMap` — `materialOpacity` becomes `opacity * texture(
    /// alphaMap )` (`MaterialNode.OPACITY`), a `vec4` product that the alpha
    /// assign narrows back to its `.x`. Copied onto the shadow pass's
    /// override material like [`alpha_test`](Self::alpha_test).
    pub alpha_map: Option<Texture>,
    /// `NodeMaterial.emissiveNode` — `setupLighting()`'s EMISSIVE tail:
    /// `EmissiveColor = vec3( emissiveNode )` and `outgoingLight +=
    /// EmissiveColor`. On a Phong or Standard material the `materialEmissive`
    /// uniform already takes that path; on an unlit `MeshBasicNodeMaterial`
    /// this node is the only way in, which is what `webgpu_instance_uniform`
    /// uses it for.
    pub emissive_node: Option<NodeRef>,
    /// `MeshPhongMaterial.specular` / `.shininess` / `.emissive` /
    /// `.emissiveIntensity`. The dumps show all four reaching the shader as
    /// object-group uniforms on every Phong material, even the ones the example
    /// never sets.
    /// `MeshPhongMaterial.specular` — the Blinn-Phong specular tint. Read by
    /// [`Phong`](MaterialKind::Phong) only; [`Lambert`](MaterialKind::Lambert)
    /// forces the specular lobe off.
    pub specular: Color,
    /// `MeshPhongMaterial.shininess` — the Blinn-Phong specular exponent.
    pub shininess: f64,
    /// `Material.emissive` — the uniform tail `NodeMaterial.setupLighting()`
    /// adds to `outgoingLight` on a lit material.
    pub emissive: Color,
    /// `Material.emissiveIntensity` — multiplies [`emissive`](Self::emissive).
    pub emissive_intensity: f64,
    /// `Material.flatShading` — `NodeMaterial.setupNormal()` picks `normalFlat`
    /// (the screen-space derivative frame) over the interpolated vertex normal,
    /// so there is no `normal` attribute and no normal varying at all.
    pub flat_shading: bool,
    /// `NodeMaterial.lights`. `false` is the light spheres' material: no
    /// lighting flow at all, `outgoingLight = DiffuseColor.rgb`.
    pub lights: bool,
    /// `material.lightsNode = lights( [ light1 ] )` — the selective-lights
    /// form. Indices into `Scene.lights`, since a `PointLight` is owned by the
    /// scene here rather than shared through an `Rc`. `None` means "every light
    /// in the scene", which is what `LightsNode` defaults to.
    pub lights_node: Option<Vec<usize>>,
    /// The `lightingModel` of `material.lightsNode = lights( [ … ] ).context(
    /// { lightingModel } )` — a user-defined
    /// [`LightingModel`](lighting_model::LightingModel) that the `LightsNode`
    /// drives in place of a built-in one.
    ///
    /// `LightingContextNode.setup()` reads `this.lightingModel ||
    /// builder.context.lightingModel`, so the material's own
    /// `setupLightingModel()` wins: this is read only by the kinds that have
    /// none of their own — `Points`, `Sprite`, and `Basic` standing in for a
    /// bare `NodeMaterial` — and ignored, as in three, by Phong, Lambert,
    /// Standard and Physical. A program input, like `lights_node`.
    pub lighting_model: Option<std::rc::Rc<dyn lighting_model::LightingModel>>,
    /// `material.maskNode` — `NodeMaterial.setupDiscard()` turns it into
    /// `If( mask.not(), () => Discard() )` at the top of the fragment.
    pub mask_node: Option<NodeRef>,
    /// `material.castShadowNode` — the colour this material casts into the
    /// shadow map's colour target, which receivers tint their shadow by when
    /// `renderer.shadowMap.transmitted` is on (`docs/nodes.md` §43).
    /// `Renderer._getShadowNodes()` reads its `.rgb` and `.a`.
    pub cast_shadow_node: Option<NodeRef>,
    /// `material.receivedShadowPositionNode` —
    /// `ShadowBaseNode.setupShadowPosition()` assigns it to
    /// `shadowPositionWorld` instead of `positionWorld`.
    pub received_shadow_position_node: Option<NodeRef>,
    /// `Material.fog` — `false` on the shadow-pass material, which is why the
    /// shadow programs carry no fog mix even though `scene.fog` is set.
    pub fog: bool,
    /// `MeshStandardMaterial.metalness` / `.roughness` and the three maps the
    /// example uses. `map` multiplies the diffuse colour; `roughnessMap` takes
    /// its green channel and `metalnessMap` its blue, which is the glTF
    /// packing three.js follows.
    pub metalness: f64,
    /// `MeshStandardMaterial.roughness`.
    pub roughness: f64,
    /// `Material.map` — the base colour texture; multiplies
    /// [`color`](Self::color).
    pub map: Option<Texture>,
    /// `Material.vertexColors` — `setupDiffuseColor()` multiplies the diffuse
    /// colour by `vertexColor()`, so one `LineSegments` (or one mesh) can carry
    /// a colour per vertex instead of one per draw call. The geometry needs a
    /// three-component `color` attribute; see
    /// [`vertex_color`](crate::nodes::tsl::vertex_color) for the one divergence
    /// from three's node.
    pub vertex_colors: bool,
    /// `MeshStandardMaterial.roughnessMap` — its green channel multiplies
    /// [`roughness`](Self::roughness).
    pub roughness_map: Option<Texture>,
    /// `MeshStandardMaterial.metalnessMap` — its blue channel multiplies
    /// [`metalness`](Self::metalness).
    pub metalness_map: Option<Texture>,
    /// `MeshStandardMaterial.emissiveMap` — `MaterialNode.EMISSIVE`'s
    /// `emissiveNode.mul( texture )`, where `emissiveNode` is already
    /// `emissive * emissiveIntensity`. The multiply is a `vec4` one in three
    /// (`vec4( emissive, 1 ) * tex`) and the `.xyz` is taken afterwards, which
    /// is why the port builds it the same way rather than multiplying `vec3`s.
    pub emissive_map: Option<Texture>,
    /// `MeshToonMaterial.gradientMap` — the ramp `ToonLightingModel` reads
    /// the direct light's `dotNL * 0.5 + 0.5` through. Read by the `Toon`
    /// kind only.
    pub gradient_map: Option<Texture>,
    /// `MeshStandardMaterial.aoMap` / `.aoMapIntensity` — `materialAO`,
    /// `tex.r.sub( 1 ).mul( aoMapIntensity ).add( 1 )`, assigned to the
    /// `AmbientOcclusion` property by `NodeMaterial.setupAmbientOcclusion()`.
    pub ao_map: Option<Texture>,
    /// `MeshStandardMaterial.aoMapIntensity` — multiplies
    /// [`ao_map`](Self::ao_map)'s contribution.
    pub ao_map_intensity: f64,
    /// `MeshStandardMaterial.bumpMap` / `.bumpScale` — `BumpMapNode`.
    pub bump_map: Option<Texture>,
    /// `MeshStandardMaterial.bumpScale` — scales [`bump_map`](Self::bump_map)'s
    /// derivative before it perturbs the normal.
    pub bump_scale: f64,
    /// `MeshPhysicalMaterial`'s own fields. Nothing on this rung sets them, but
    /// rung 10's glTF materials will: `KHR_materials_specular` is
    /// `specularIntensity` + `specularColor`, and `ior` drives the dielectric
    /// F0. They are carried here so the material API does not have to change
    /// shape when `MaterialKind::Physical` arrives.
    pub clearcoat: f64,
    /// `MeshPhysicalMaterial.clearcoatRoughness`.
    pub clearcoat_roughness: f64,
    /// `MeshPhysicalMaterial.clearcoatMap` — `MaterialNode.CLEARCOAT`
    /// multiplies `clearcoat` by the texel's red channel.
    pub clearcoat_map: Option<Texture>,
    /// `MeshPhysicalMaterial.clearcoatRoughnessMap` —
    /// `MaterialNode.CLEARCOAT_ROUGHNESS` multiplies `clearcoatRoughness` by
    /// the texel's green channel.
    pub clearcoat_roughness_map: Option<Texture>,
    /// `MeshPhysicalMaterial.sheen` / `.sheenColor` / `.sheenRoughness` —
    /// `KHR_materials_sheen`. `sheen` is the intensity and `sheen_color` the
    /// tint; `MaterialNode.SHEEN` is `sheenColor.mul( sheen )` and the shader
    /// carries both as separate uniforms, which is why the multiply is in the
    /// WGSL and not here.
    ///
    /// `sheen > 0` is `MeshPhysicalNodeMaterial.useSheen`: it is what turns the
    /// whole sheen half of `PhysicalLightingModel` on, so a sheen colour with a
    /// zero intensity generates exactly the shader it did before.
    pub sheen: f64,
    /// `MeshPhysicalMaterial.sheenColor` — the sheen lobe's tint.
    pub sheen_color: Color,
    /// `MeshPhysicalMaterial.sheenRoughness`.
    pub sheen_roughness: f64,
    /// `MeshPhysicalMaterial.diffuseRoughness` — `KHR_materials_diffuse_roughness`.
    /// `diffuse_roughness > 0` is `MeshPhysicalNodeMaterial.useDiffuseRoughness`,
    /// which swaps the Lambert diffuse lobe of `PhysicalLightingModel` for
    /// the energy-preserving Oren–Nayar one (EON). `diffuseRoughnessMap` is
    /// not ported: no graded page carries one.
    pub diffuse_roughness: f64,
    /// `MeshPhysicalMaterial.ior` — drives the dielectric F0 that
    /// `setupSpecular()` derives before `specular_color` modulates it.
    pub ior: f64,
    /// `MeshPhysicalMaterial.specularIntensity` — `KHR_materials_specular`.
    pub specular_intensity: f64,
    /// `MeshPhysicalMaterial.specularColor` — `KHR_materials_specular`.
    pub specular_color: Color,
    /// `material.specularNode`.
    pub specular_node: Option<NodeRef>,
    /// `MeshStandardMaterial.normalMap` and `.normalScale`. `GLTFLoader`
    /// flips `normalScale.y` on a geometry that has a normal map but no
    /// `tangent` attribute (`useDerivativeTangents`), which is where Michelle's
    /// `( 1, -1 )` comes from — it is not in the asset.
    pub normal_map: Option<Texture>,
    /// `Material.normalScale` — scales [`normal_map`](Self::normal_map)'s `x`
    /// and `y` before it perturbs the surface normal.
    pub normal_scale: crate::math::Vector2,
    /// `MeshPhysicalMaterial.specularColorMap` — multiplies `specularColor`.
    pub specular_color_map: Option<Texture>,
    /// `MeshPhysicalMaterial.anisotropy` / `.anisotropyRotation` /
    /// `.anisotropyMap` — `KHR_materials_anisotropy`. The pair reaches the
    /// shader as one `materialAnisotropyVector` uniform,
    /// `vec2( anisotropy * cos( rotation ), anisotropy * sin( rotation ) )`,
    /// exactly as three's `MeshPhysicalNodeMaterial` builds it.
    pub anisotropy: f64,
    /// `MeshPhysicalMaterial.anisotropyRotation`, in radians.
    pub anisotropy_rotation: f64,
    /// `MeshPhysicalMaterial.anisotropyMap` — its red and green channels give
    /// the per-texel anisotropy strength and rotation.
    pub anisotropy_map: Option<Texture>,
    /// `MeshPhysicalMaterial.clearcoatNormalMap` / `.clearcoatNormalScale`.
    /// The clearcoat lobe's own normal, through the same TBN sub-build the
    /// base normal map uses.
    pub clearcoat_normal_map: Option<Texture>,
    /// `MeshPhysicalMaterial.clearcoatNormalScale` — scales
    /// [`clearcoat_normal_map`](Self::clearcoat_normal_map)'s `x` and `y`.
    pub clearcoat_normal_scale: crate::math::Vector2,
    /// `MeshPhysicalMaterial.transmission` / `.thickness` /
    /// `.attenuationDistance` / `.attenuationColor` —
    /// `KHR_materials_transmission` and `KHR_materials_volume`. A non-zero
    /// `transmission` moves the object into the renderer's transmission pass.
    pub transmission: f64,
    /// `MeshPhysicalMaterial.thickness` — `KHR_materials_volume`; the modelled
    /// thickness of the transmissive medium.
    pub thickness: f64,
    /// `MeshPhysicalMaterial.thicknessMap` — `MaterialNode.THICKNESS`
    /// multiplies `thickness` by the texel's green channel.
    pub thickness_map: Option<Texture>,
    /// `MeshPhysicalMaterial.attenuationDistance` — `KHR_materials_volume`;
    /// the distance light travels through the medium before it is tinted by
    /// [`attenuation_color`](Self::attenuation_color).
    pub attenuation_distance: f64,
    /// `MeshPhysicalMaterial.attenuationColor` — `KHR_materials_volume`.
    pub attenuation_color: Color,
    /// `material.normalNode` — e.g. `normalMap( texture( map ) )`.
    pub normal_node: Option<NodeRef>,
    /// `NodeMaterial.positionNode` — replaces `positionLocal`.
    ///
    /// **Deviation from r186, deliberate** (plan §5.2, `docs/nodes.md` §10):
    /// three's `setupPosition()` assigns the instance transform *first* and
    /// `positionNode` *after*, so `positionNode` discards the instance matrix.
    /// The port assigns `positionNode` first and then the instance transform.
    /// `SpriteNodeMaterial` reads it a second time in `setupPositionView()`.
    pub position_node: Option<NodeRef>,
    /// `SpriteNodeMaterial.scaleNode` / `.rotationNode`.
    pub scale_node: Option<NodeRef>,
    /// `SpriteNodeMaterial.rotationNode` — replaces the `materialRotation`
    /// uniform.
    pub rotation_node: Option<NodeRef>,
    /// `PointsNodeMaterial.sizeNode` — the point size in pixels when the
    /// material draws a [`Sprite`](crate::objects::Sprite) (instanced quads)
    /// rather than `Points`. Read by `setupVertexSprite()` only.
    pub size_node: Option<NodeRef>,
    /// `SpriteMaterial.rotation` — the `materialRotation` uniform.
    pub rotation: f64,
    /// `LineBasicMaterial.linewidth` — the `materialLineWidth` uniform.
    ///
    /// A hairline `Line` ignores it, as three.js says: "WebGL and WebGPU ignore
    /// this setting and always render line primitives with a width of one
    /// pixel". It is read by the fat-line material, which turns each segment
    /// into a screen-space quad and so can honour a width.
    pub linewidth: f64,
    /// `SpriteNodeMaterial.sizeAttenuation`. `true` (the default) is the branch
    /// that *omits* the `mvPosition.z.negate()` scale factor.
    pub size_attenuation: bool,
    /// `NodeMaterial.vertexNode` — replaces the whole clip-position flow.
    pub vertex_node: Option<NodeRef>,
    /// `NodeMaterial.fragmentNode` — replaces the whole fragment flow.
    pub fragment_node: Option<NodeRef>,
    /// `NodeMaterial.outputNode` — replaces only what `output.color` is written
    /// from. The standard flow still runs and still writes the `Output`
    /// property, so the custom node can read `DiffuseColor`, `Output` and the
    /// normal accessors.
    pub output_node: Option<NodeRef>,
    /// `NodeMaterial.backdropNode` — what the diffuse light is replaced
    /// with: `LightsNode.setup()` assigns it to `totalDiffuse` on a lit
    /// material, `setupLighting()` makes it the outgoing light on an unlit
    /// one. Usually a [`viewport_shared_texture`] tap — the scene behind the
    /// object, read back. A material with one goes in the transparent list
    /// whatever [`transparent`](Self::transparent) says, so that it draws
    /// after what it reads.
    ///
    /// [`viewport_shared_texture`]: crate::nodes::display::viewport_shared_texture
    pub backdrop_node: Option<NodeRef>,
    /// `NodeMaterial.backdropAlphaNode` — blend [`backdrop_node`](Self::backdrop_node)
    /// over the lit diffuse by this much instead of replacing it.
    pub backdrop_alpha_node: Option<NodeRef>,
    /// `NodeMaterial.mrtNode` — the material's own MRT overrides, merged over
    /// the renderer's (the pass's) by `NodeMaterial.setup()`. Read only when a
    /// render target with more than one colour attachment is bound, which is
    /// what makes it inert for every material that is not drawn into one.
    ///
    /// `webgpu_postprocessing_bloom_selective` gives each of its fifty spheres
    /// a material whose `mrtNode` is `mrt( { bloomIntensity: uniform( 0 or 1 )
    /// } )`, over the pass's `mrt( { output, bloomIntensity: float( 0 ) } )`.
    pub mrt_node: Option<crate::nodes::MrtNode>,
    /// `MeshStandardNodeMaterial.metalnessNode` / `.roughnessNode` — the two
    /// `setupVariants()` inputs, replacing the `materialMetalness` /
    /// `materialRoughness` uniforms (and their maps). `webgpu_deferred`'s
    /// resolve material reads both out of the G-buffer.
    pub metalness_node: Option<NodeRef>,
    /// `MeshStandardNodeMaterial.roughnessNode` — see
    /// [`metalness_node`](Self::metalness_node).
    pub roughness_node: Option<NodeRef>,
    /// `NodeMaterial.depthNode` — `setupDepth()`'s value, written to the
    /// fragment stage's `@builtin( frag_depth )` output. Set only when the
    /// material is drawn with a depth buffer (three checks
    /// `depthWrite || depthTest` and the target's `depthBuffer`).
    pub depth_node: Option<NodeRef>,
    /// `NodeMaterial.contextNode = overrideNodes( [ … ] )` — the three
    /// accessors `webgpu_deferred`'s resolve material reads out of the
    /// G-buffer instead of out of the geometry. See `docs/nodes.md` §27.
    pub context_overrides: Option<crate::nodes::tsl::OverrideNodes>,
    /// `Material.side` — which face(s) are drawn.
    pub side: Side,
    /// `Material.visible` — `_projectObject()` skips an object whose material is
    /// not visible.
    pub visible: bool,
    /// `Material.transparent` — which of the render list's two arrays the object
    /// goes into, and so whether it is sorted front-to-back or back-to-front.
    pub transparent: bool,
    /// `Material.forceSinglePass` — a transparent `DoubleSide` material is
    /// drawn once, both faces together, instead of as three's back-then-front
    /// pair.
    pub force_single_pass: bool,
    /// `Material.wireframe` — only `MeshBasicMaterial` and its kin expose it.
    /// It reaches no shader: `WebGPUUtils.getPrimitiveTopology()` turns a
    /// wireframe `Mesh` into a `line-list`, and `Geometries.getIndex()`
    /// swaps the geometry's index for `getWireframeIndex()`'s — each
    /// triangle's three edges as six indices (see `Renderer`'s
    /// `wireframe_index`).
    pub wireframe: bool,
    /// `Material.blending` — `NormalBlending` by default, which together with
    /// `transparent: false` is what keeps a pipeline blend-state-free.
    pub blending: Blending,
    /// `Material.premultipliedAlpha` — selects the other half of the
    /// `_getBlending()` table.
    pub premultiplied_alpha: bool,
    /// `Material.alphaToCoverage` — read by `builder.isOpaque()`, by
    /// `Line2NodeMaterial`'s `alphaLine`, and by the pipeline's
    /// `alphaToCoverageEnabled` (with more than one sample).
    pub alpha_to_coverage: bool,
    /// `Line2NodeMaterial.worldUnits` (`_useWorldUnits`) — the fat line's
    /// `linewidth` is in world units rather than screen pixels. Read by
    /// `setup()` and by `LineSegments2.raycast()`; ignored by every other
    /// material. A program input: set it before the first frame, or call
    /// [`set_needs_update`](Self::set_needs_update).
    pub world_units: bool,
    /// `Material.blendSrc` / `.blendDst` / `.blendEquation` and the three
    /// `*Alpha` overrides (`None` is Three's `null`), read only under
    /// `CustomBlending`.
    pub blend_src: BlendFactor,
    /// `Material.blendDst`.
    pub blend_dst: BlendFactor,
    /// `Material.blendEquation`.
    pub blend_equation: BlendEquation,
    /// `Material.blendSrcAlpha`.
    pub blend_src_alpha: Option<BlendFactor>,
    /// `Material.blendDstAlpha`.
    pub blend_dst_alpha: Option<BlendFactor>,
    /// `Material.blendEquationAlpha`.
    pub blend_equation_alpha: Option<BlendEquation>,
    /// `Material.depthTest`.
    pub depth_test: bool,
    /// `Material.depthWrite`.
    pub depth_write: bool,
    /// `Material.depthFunc` — the depth compare while `depth_test` is on.
    pub depth_func: DepthFunc,
    /// `Background`'s material samples the cube map through the background
    /// uniforms rather than an env map.
    pub name: &'static str,
}

impl Default for MeshBasicNodeMaterial {
    fn default() -> Self {
        Self {
            id: MaterialId::next(),
            version: 0,
            kind: MaterialKind::Basic,
            color: Color::new(1.0, 1.0, 1.0),
            opacity: 1.0,
            // `MeshPhongMaterial`'s own defaults: specular 0x111111,
            // shininess 30, emissive black, emissiveIntensity 1. `new Color(
            // 0x111111 )` is `setHex( hex, SRGBColorSpace )`, so the stored
            // value is the *linear* 0.0056, not 17/255 — eleven times dimmer,
            // and visible as a blown-out highlight on any Phong material that
            // does not override `specularNode`.
            specular: Color::from_hex(0x111111),
            shininess: 30.0,
            emissive: Color::new(0.0, 0.0, 0.0),
            emissive_intensity: 1.0,
            flat_shading: false,
            lights: false,
            lights_node: None,
            lighting_model: None,
            mask_node: None,
            cast_shadow_node: None,
            received_shadow_position_node: None,
            fog: true,
            // `MeshStandardMaterial` defaults.
            metalness: 0.0,
            roughness: 1.0,
            map: None,
            vertex_colors: false,
            roughness_map: None,
            metalness_map: None,
            emissive_map: None,
            gradient_map: None,
            ao_map: None,
            ao_map_intensity: 1.0,
            bump_map: None,
            bump_scale: 1.0,
            // `MeshPhysicalMaterial` defaults.
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            clearcoat_map: None,
            clearcoat_roughness_map: None,
            sheen: 0.0,
            sheen_color: Color::new(0.0, 0.0, 0.0),
            sheen_roughness: 1.0,
            diffuse_roughness: 0.0,
            ior: 1.5,
            specular_intensity: 1.0,
            specular_color: Color::new(1.0, 1.0, 1.0),
            specular_node: None,
            normal_map: None,
            normal_scale: crate::math::Vector2::new(1.0, 1.0),
            specular_color_map: None,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            anisotropy_map: None,
            clearcoat_normal_map: None,
            clearcoat_normal_scale: crate::math::Vector2::new(1.0, 1.0),
            transmission: 0.0,
            // three's `MeshPhysicalMaterial` defaults: no volume at all.
            thickness: 0.0,
            thickness_map: None,
            attenuation_distance: f64::INFINITY,
            attenuation_color: Color::new(1.0, 1.0, 1.0),
            normal_node: None,
            position_node: None,
            reflectivity: 1.0,
            env_map: None,
            pmrem_env: None,
            color_node: None,
            opacity_node: None,
            alpha_test_node: None,
            alpha_test: 0.0,
            alpha_hash: false,
            alpha_map: None,
            emissive_node: None,
            scale_node: None,
            size_node: None,
            rotation_node: None,
            rotation: 0.0,
            linewidth: 1.0,
            size_attenuation: true,
            vertex_node: None,
            metalness_node: None,
            roughness_node: None,
            depth_node: None,
            context_overrides: None,
            fragment_node: None,
            output_node: None,
            backdrop_node: None,
            backdrop_alpha_node: None,
            mrt_node: None,
            side: Side::Front,
            visible: true,
            transparent: false,
            force_single_pass: false,
            wireframe: false,
            blending: Blending::Normal,
            premultiplied_alpha: false,
            alpha_to_coverage: false,
            world_units: false,
            blend_src: BlendFactor::SrcAlpha,
            blend_dst: BlendFactor::OneMinusSrcAlpha,
            blend_equation: BlendEquation::Add,
            blend_src_alpha: None,
            blend_dst_alpha: None,
            blend_equation_alpha: None,
            depth_test: true,
            depth_write: true,
            depth_func: DepthFunc::LessEqual,
            name: "",
        }
    }
}

impl MeshBasicNodeMaterial {
    /// `new MeshBasicMaterial()` — a white, opaque [`Basic`](MaterialKind::Basic)
    /// material. Equivalent to [`Default::default`].
    pub fn new() -> Self {
        Self::default()
    }

    /// The fields set on this material that nothing in the port reads for its
    /// [`kind`](Self::kind), by their three.js names — the audit in
    /// `docs/api.md` ("Material fields the port does not read"). The renderer
    /// logs each one once per material when it builds the program; call
    /// [`check_supported`](Self::check_supported) to fail on them instead.
    ///
    /// A field three's own class for that kind lacks (a `clearcoat` on a
    /// Standard material, say) is not listed: three ignores it too.
    pub fn unsupported_fields(&self) -> Vec<&'static str> {
        use MaterialKind::*;
        let mut fields = Vec::new();
        let pbr = matches!(self.kind, Standard | Physical);
        // `MeshPhongMaterial.envMap` / `MeshLambertMaterial.envMap` (the
        // `combine` blend) and a plain cube `envMap` on a PBR material, which
        // three would PMREM on the fly: only the Basic path samples it; a PBR
        // material takes `pmrem_env` instead.
        if self.env_map.is_some() && self.kind != Basic {
            fields.push("envMap");
        }
        // A PMREM environment is only read by `PhysicalLightingModel`.
        if self.pmrem_env.is_some() && !pbr {
            fields.push("envMap (PMREM)");
        }
        // `setupAmbientOcclusion()` is only wired into the Standard flow.
        if self.ao_map.is_some() && !pbr {
            fields.push("aoMap");
        }
        fields
    }

    /// `Err(Error::Unsupported)` for the first of
    /// [`unsupported_fields`](Self::unsupported_fields), so an application
    /// that would rather fail than draw a material differently from three can
    /// ask before its first frame.
    pub fn check_supported(&self) -> Result<(), crate::Error> {
        match self.unsupported_fields().first() {
            Some(&field) => Err(crate::Error::Unsupported {
                field,
                kind: self.kind,
            }),
            None => Ok(()),
        }
    }

    /// `material.needsUpdate = true`: `Material.js`' setter, which bumps
    /// `version` so the next frame builds the program afresh. Call it after
    /// changing anything the program depends on — the struct docs list what
    /// does and does not.
    pub fn set_needs_update(&mut self) {
        self.version += 1;
    }

    /// The blending fields `WebGPUPipelineUtils._getBlending()` reads, gathered
    /// into the struct the table takes.
    pub(crate) fn blend_mode(&self) -> BlendMode {
        BlendMode {
            blending: self.blending,
            premultiplied_alpha: self.premultiplied_alpha,
            blend_src: self.blend_src,
            blend_dst: self.blend_dst,
            blend_equation: self.blend_equation,
            blend_src_alpha: self.blend_src_alpha,
            blend_dst_alpha: self.blend_dst_alpha,
            blend_equation_alpha: self.blend_equation_alpha,
        }
    }

    /// `WebGPUPipelineUtils.createRenderPipeline()`'s `materialBlending`: the
    /// blend state of the colour target, or `None` when the gate says the
    /// pipeline gets none at all.
    pub(crate) fn blend_state(&self) -> Option<wgpu::BlendState> {
        let mode = self.blend_mode();
        if blending::needs_blend_state(&mode, self.transparent) {
            blending::blending(&mode)
        } else {
            None
        }
    }

    /// `RenderList.push()`'s choice of list: `transparent === true ||
    /// transmission > 0 || backdropNode` — a material that reads the frame
    /// has to be drawn after it.
    pub(crate) fn in_transparent_list(&self) -> bool {
        self.transparent || self.transmission > 0.0 || self.backdrop_node.is_some()
    }

    /// `NodeBuilder.isOpaque()` — `transparent === false && blending ===
    /// NormalBlending && alphaToCoverage === false`. What decides whether
    /// `setupDiffuseColor()` ends with `diffuseColor.a = 1.0`.
    pub(crate) fn is_opaque(&self) -> bool {
        !self.transparent && self.blending == Blending::Normal && !self.alpha_to_coverage
    }

    /// `new SpriteNodeMaterial()`. `SpriteNodeMaterial`'s constructor sets
    /// `transparent = true` explicitly — "In Sprites, the transparent property
    /// is enabled by default" — which also takes it out of `isOpaque()`, so the
    /// fragment flow keeps its per-fragment alpha.
    pub fn sprite() -> Self {
        Self {
            kind: MaterialKind::Sprite,
            transparent: true,
            ..Self::default()
        }
    }

    /// `new PointsNodeMaterial()`.
    ///
    /// `PointsNodeMaterial extends SpriteNodeMaterial`
    /// (`src/materials/nodes/PointsNodeMaterial.js`), so it inherits
    /// `transparent = true` — which puts a `Points` object in the *transparent*
    /// render list, gives its pipeline a `NormalBlending` blend state, and
    /// takes it out of `is_opaque` so the fragment flow
    /// keeps its per-fragment alpha. `alphaToCoverage` stays **off**: three.js'
    /// own dump of this pipeline has `alphaToCoverageEnabled: false`.
    pub fn points() -> Self {
        Self {
            kind: MaterialKind::Points,
            transparent: true,
            ..Self::default()
        }
    }

    /// `new LineBasicNodeMaterial( { color } )`.
    ///
    /// **No `MaterialKind` of its own, and that is a finding, not a shortcut.**
    /// `LineBasicNodeMaterial` is a bare `NodeMaterial` that only calls
    /// `setDefaultValues( new LineBasicMaterial() )`, and every default it
    /// brings that this port models — `color` white, `opacity` 1, `fog` true,
    /// `lights` false (`NodeMaterial`'s own default), `transparent` false — is
    /// already `MeshBasicNodeMaterial`'s. `linewidth` / `linecap` / `linejoin`
    /// are SVGRenderer-only ("WebGL and WebGPU ignore this setting and always
    /// render line primitives with a width of one pixel"). `vertexColors` is on
    /// the shared material, since `setupDiffuseColor()` is where three reads
    /// it — see [`MeshBasicNodeMaterial::vertex_colors`]. Three's own dump of this
    /// material from the d33 page (`docs/lines/LineBasicNodeMaterial_27.*`) is
    /// the `Basic` program statement for statement, so a second kind would
    /// generate identical WGSL.
    ///
    /// What makes a line a line is the *object*, not the material:
    /// `WebGPUUtils.getPrimitiveTopology( object, material )` reads
    /// `object.isLine` / `object.isLineSegments`.
    pub fn line(color: Color) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }

    /// `new Line2NodeMaterial( { color, linewidth, vertexColors } )` — the
    /// fat line.
    ///
    /// `Line2NodeMaterial.setDefaultValues( new LineBasicMaterial() )` and then
    /// `this.blending = NoBlending` — three sets it in the constructor because
    /// the material writes its own coverage through the `discard` and must not
    /// have the result blended a second time. That default has teeth here: it
    /// makes `is_opaque` false, so the fragment flow does
    /// **not** emit `DiffuseColor.w = 1.0` (`docs/nodes.md` §8).
    ///
    /// `this._useAlphaToCoverage = true` is the constructor's other default,
    /// so `alpha_to_coverage` starts true; `webgpu_lines_fat` turns it off.
    pub fn line2(color: Color) -> Self {
        Self {
            kind: MaterialKind::Line2,
            color,
            blending: Blending::No,
            alpha_to_coverage: true,
            ..Self::default()
        }
    }

    /// `new MeshNormalNodeMaterial()` — `MeshNormalMaterial` under
    /// `WebGPURenderer`.
    ///
    /// The whole subclass is one overridden `setupDiffuseColor()`:
    /// `diffuseColor = colorSpaceToWorking( vec4( packNormalToRGB( normalView
    /// ), opacity ), SRGBColorSpace )`. Nothing else about it differs from a
    /// `MeshBasicNodeMaterial`, so it is a [`MaterialKind`] rather than a type.
    pub fn normal() -> Self {
        Self {
            kind: MaterialKind::Normal,
            ..Self::default()
        }
    }

    /// `new MeshPhongNodeMaterial( { color } )`. `NodeMaterial.lights` is true
    /// for every lit material, which is what puts the `LightsNode` flow in the
    /// fragment stage.
    pub fn phong(color: Color) -> Self {
        Self {
            kind: MaterialKind::Phong,
            color,
            lights: true,
            ..Self::default()
        }
    }

    /// `new MeshLambertNodeMaterial( { color } )`.
    ///
    /// `MeshLambertMaterial`'s own defaults over `Material`'s are all shared
    /// with the Phong ones the struct already carries, so the constructor is
    /// the Phong one under a different [`MaterialKind`].
    pub fn lambert(color: Color) -> Self {
        Self {
            kind: MaterialKind::Lambert,
            color,
            lights: true,
            ..Self::default()
        }
    }
}

impl MeshBasicNodeMaterial {
    /// `new MeshToonNodeMaterial( { color, gradientMap } )`.
    ///
    /// `MeshToonMaterial`'s defaults over `Material`'s — white, `emissive`
    /// black at intensity 1, no maps — are all ones the struct already
    /// carries, and `lights = true` is the constructor's own.
    pub fn toon(color: Color, gradient_map: Option<Texture>) -> Self {
        Self {
            kind: MaterialKind::Toon,
            color,
            gradient_map,
            lights: true,
            ..Self::default()
        }
    }

    /// `new MeshStandardNodeMaterial( { color, roughness, metalness } )`.
    pub fn standard(color: Color, roughness: f64, metalness: f64) -> Self {
        Self {
            kind: MaterialKind::Standard,
            color,
            roughness,
            metalness,
            lights: true,
            ..Self::default()
        }
    }

    /// `new MeshPhysicalNodeMaterial()` — a Standard material plus
    /// `setupSpecular()`'s ior/specular block. `GLTFLoader` builds one whenever
    /// the asset declares `KHR_materials_specular` or `KHR_materials_ior`.
    pub fn physical(color: Color, roughness: f64, metalness: f64) -> Self {
        Self {
            kind: MaterialKind::Physical,
            ..Self::standard(color, roughness, metalness)
        }
    }
}

/// three.js' name for a `NodeMaterial` whose kind is `Phong`. The struct is
/// shared because `WebGPURenderer` treats every material as a `NodeMaterial`
/// and the renderer must hold them in one list.
pub type MeshPhongNodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for a `NodeMaterial` whose kind is `Lambert`. The struct is
/// shared for the reason [`MeshPhongNodeMaterial`] is.
pub type MeshLambertNodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for a `NodeMaterial` whose kind is `Toon` — see
/// [`MeshBasicNodeMaterial::toon`].
pub type MeshToonNodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for a `NodeMaterial` whose kind is `Sprite`.
pub type SpriteNodeMaterial = MeshBasicNodeMaterial;
/// three.js' name for a `NodeMaterial` whose kind is `Points`.
pub type PointsNodeMaterial = MeshBasicNodeMaterial;
/// three.js' name for a `NodeMaterial` whose kind is `Standard` — one struct,
/// one renderer list.
pub type MeshStandardNodeMaterial = MeshBasicNodeMaterial;
/// three.js' name for a `NodeMaterial` whose kind is `Physical` — see
/// [`MeshBasicNodeMaterial::physical`].
pub type MeshPhysicalNodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for a `NodeMaterial` whose kind is `Normal` — see
/// [`MeshBasicNodeMaterial::normal`].
pub type MeshNormalNodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for a `NodeMaterial` whose kind is `Line2` — see
/// [`MeshBasicNodeMaterial::line2`] and [`crate::addons::lines`].
pub type Line2NodeMaterial = MeshBasicNodeMaterial;

/// three.js' name for the `NodeMaterial` a `Line` / `LineSegments` draws with.
/// It carries no state of its own — see [`MeshBasicNodeMaterial::line`].
pub type LineBasicNodeMaterial = MeshBasicNodeMaterial;
