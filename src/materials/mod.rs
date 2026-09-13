//! Ports of `three.js/src/materials/nodes` — under `WebGPURenderer` every
//! material is a `NodeMaterial`, so this is the only material path.

pub mod blending;
mod node_material;
pub mod phong;

pub use node_material::{
    background_color_node, background_vertex_node, instanced_range, output_fragment_node,
    quad_vertex_node, render_output, setup, SetupContext,
};

pub use blending::{
    blend_factor, blend_operation, BlendEquation, BlendFactor, BlendMode, Blending,
};

use crate::math::Color;
use crate::nodes::NodeRef;
use crate::textures::CubeTexture;

/// `three.js/src/constants.js` sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Front,
    Back,
}

/// Which `NodeMaterial` subclass this is — i.e. which `setupLightingModel()`
/// the fragment flow runs. three.js models it with subclasses; here the flow is
/// one function with a `match`, so a new lighting model cannot be silently
/// skipped in one of the setup steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaterialKind {
    /// `MeshBasicNodeMaterial` — `BasicLightingModel`, `lights = false`.
    #[default]
    Basic,
    /// `MeshPhongNodeMaterial` — `PhongLightingModel`.
    Phong,
}

/// Port of `MeshBasicNodeMaterial.js` + the `NodeMaterial.js` / `Material.js`
/// fields the ladder uses. Defaults mirror three.js: white, opaque,
/// `FrontSide`, depth test on with `LessEqualDepth`, depth write on,
/// `reflectivity = 1`.
#[derive(Clone, Debug)]
pub struct MeshBasicNodeMaterial {
    pub kind: MaterialKind,
    pub color: Color,
    pub opacity: f64,
    pub reflectivity: f64,
    /// `MeshBasicMaterial.envMap` — `setupEnvironment()` turns it into
    /// `BasicEnvironmentNode( cubeTexture( envMap ) )`.
    pub env_map: Option<CubeTexture>,
    pub color_node: Option<NodeRef>,
    /// `MeshPhongMaterial.specular` / `.shininess` / `.emissive` /
    /// `.emissiveIntensity`. The dumps show all four reaching the shader as
    /// object-group uniforms on every Phong material, even the ones the example
    /// never sets.
    pub specular: Color,
    pub shininess: f64,
    pub emissive: Color,
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
    /// `material.specularNode`.
    pub specular_node: Option<NodeRef>,
    /// `material.normalNode` — e.g. `normalMap( texture( map ) )`.
    pub normal_node: Option<NodeRef>,
    /// `NodeMaterial.vertexNode` — replaces the whole clip-position flow.
    pub vertex_node: Option<NodeRef>,
    /// `NodeMaterial.fragmentNode` — replaces the whole fragment flow.
    pub fragment_node: Option<NodeRef>,
    pub side: Side,
    /// `Material.visible` — `_projectObject()` skips an object whose material is
    /// not visible.
    pub visible: bool,
    /// `Material.transparent` — which of the render list's two arrays the object
    /// goes into, and so whether it is sorted front-to-back or back-to-front.
    pub transparent: bool,
    /// `Material.blending` — `NormalBlending` by default, which together with
    /// `transparent: false` is what keeps a pipeline blend-state-free.
    pub blending: Blending,
    /// `Material.premultipliedAlpha` — selects the other half of the
    /// `_getBlending()` table.
    pub premultiplied_alpha: bool,
    /// `Material.alphaToCoverage`. Only `builder.isOpaque()` reads it so far;
    /// the pipeline's `alphaToCoverageEnabled` is still hardcoded false.
    pub alpha_to_coverage: bool,
    /// `Material.blendSrc` / `.blendDst` / `.blendEquation` and the three
    /// `*Alpha` overrides (`None` is Three's `null`), read only under
    /// `CustomBlending`.
    pub blend_src: BlendFactor,
    pub blend_dst: BlendFactor,
    pub blend_equation: BlendEquation,
    pub blend_src_alpha: Option<BlendFactor>,
    pub blend_dst_alpha: Option<BlendFactor>,
    pub blend_equation_alpha: Option<BlendEquation>,
    pub depth_test: bool,
    pub depth_write: bool,
    /// `Background`'s material samples the cube map through the background
    /// uniforms rather than an env map.
    pub name: &'static str,
}

impl Default for MeshBasicNodeMaterial {
    fn default() -> Self {
        Self {
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
            specular_node: None,
            normal_node: None,
            reflectivity: 1.0,
            env_map: None,
            color_node: None,
            vertex_node: None,
            fragment_node: None,
            side: Side::Front,
            visible: true,
            transparent: false,
            blending: Blending::Normal,
            premultiplied_alpha: false,
            alpha_to_coverage: false,
            blend_src: BlendFactor::SrcAlpha,
            blend_dst: BlendFactor::OneMinusSrcAlpha,
            blend_equation: BlendEquation::Add,
            blend_src_alpha: None,
            blend_dst_alpha: None,
            blend_equation_alpha: None,
            depth_test: true,
            depth_write: true,
            name: "",
        }
    }
}

impl MeshBasicNodeMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// The blending fields `WebGPUPipelineUtils._getBlending()` reads, gathered
    /// into the struct the table takes.
    pub fn blend_mode(&self) -> BlendMode {
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
    pub fn blend_state(&self) -> Option<wgpu::BlendState> {
        let mode = self.blend_mode();
        if blending::needs_blend_state(&mode, self.transparent) {
            blending::blending(&mode)
        } else {
            None
        }
    }

    /// `NodeBuilder.isOpaque()` — `transparent === false && blending ===
    /// NormalBlending && alphaToCoverage === false`. What decides whether
    /// `setupDiffuseColor()` ends with `diffuseColor.a = 1.0`.
    pub fn is_opaque(&self) -> bool {
        !self.transparent && self.blending == Blending::Normal && !self.alpha_to_coverage
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
}

/// three.js' name for a `NodeMaterial` whose kind is `Phong`. The struct is
/// shared because `WebGPURenderer` treats every material as a `NodeMaterial`
/// and the renderer must hold them in one list.
pub type MeshPhongNodeMaterial = MeshBasicNodeMaterial;
