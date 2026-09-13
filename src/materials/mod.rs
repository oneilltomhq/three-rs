//! Ports of `three.js/src/materials/nodes` — under `WebGPURenderer` every
//! material is a `NodeMaterial`, so this is the only material path.

mod dfg_lut;
mod node_material;
pub mod phong;
pub mod physical;

pub use node_material::{
    background_color_node, background_vertex_node, instanced_range, output_fragment_node,
    quad_vertex_node, render_output, setup, LightKind, SetupContext, ToneMapping, MAX_LIGHTS,
};

use crate::math::Color;
use crate::nodes::NodeRef;
use crate::textures::{CubeTexture, Texture};

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
    /// `MeshStandardNodeMaterial` / `MeshPhysicalNodeMaterial` —
    /// `PhysicalLightingModel`.
    Standard,
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
    /// `NodeMaterial.lights`. `false` is the light spheres' material: no
    /// lighting flow at all, `outgoingLight = DiffuseColor.rgb`.
    pub lights: bool,
    /// `material.lightsNode = lights( [ light1 ] )` — the selective-lights
    /// form. Indices into `Scene.lights`, since a `PointLight` is owned by the
    /// scene here rather than shared through an `Rc`. `None` means "every light
    /// in the scene", which is what `LightsNode` defaults to.
    pub lights_node: Option<Vec<usize>>,
    /// `MeshStandardMaterial.metalness` / `.roughness` and the three maps the
    /// example uses. `map` multiplies the diffuse colour; `roughnessMap` takes
    /// its green channel and `metalnessMap` its blue, which is the glTF
    /// packing three.js follows.
    pub metalness: f64,
    pub roughness: f64,
    pub map: Option<Texture>,
    pub roughness_map: Option<Texture>,
    pub metalness_map: Option<Texture>,
    /// `MeshStandardMaterial.bumpMap` / `.bumpScale` — `BumpMapNode`.
    pub bump_map: Option<Texture>,
    pub bump_scale: f64,
    /// `MeshPhysicalMaterial`'s own fields. Nothing on this rung sets them, but
    /// rung 10's glTF materials will: `KHR_materials_specular` is
    /// `specularIntensity` + `specularColor`, and `ior` drives the dielectric
    /// F0. They are carried here so the material API does not have to change
    /// shape when `MaterialKind::Physical` arrives.
    pub clearcoat: f64,
    pub clearcoat_roughness: f64,
    pub sheen: Color,
    pub sheen_roughness: f64,
    pub ior: f64,
    pub specular_intensity: f64,
    pub specular_color: Color,
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
            lights: false,
            lights_node: None,
            // `MeshStandardMaterial` defaults.
            metalness: 0.0,
            roughness: 1.0,
            map: None,
            roughness_map: None,
            metalness_map: None,
            bump_map: None,
            bump_scale: 1.0,
            // `MeshPhysicalMaterial` defaults.
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            sheen: Color::new(0.0, 0.0, 0.0),
            sheen_roughness: 1.0,
            ior: 1.5,
            specular_intensity: 1.0,
            specular_color: Color::new(1.0, 1.0, 1.0),
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

impl MeshBasicNodeMaterial {
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
}

/// three.js' name for a `NodeMaterial` whose kind is `Phong`. The struct is
/// shared because `WebGPURenderer` treats every material as a `NodeMaterial`
/// and the renderer must hold them in one list.
pub type MeshPhongNodeMaterial = MeshBasicNodeMaterial;

/// Likewise for `Standard` / `Physical` — one struct, one renderer list.
pub type MeshStandardNodeMaterial = MeshBasicNodeMaterial;
pub type MeshPhysicalNodeMaterial = MeshBasicNodeMaterial;
