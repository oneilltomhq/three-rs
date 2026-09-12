//! Ports of `three.js/src/materials/nodes` — under `WebGPURenderer` every
//! material is a `NodeMaterial`, so this is the only material path.

mod node_material;

pub use node_material::{
    background_color_node, background_vertex_node, instanced_range, output_fragment_node,
    quad_vertex_node, setup, SetupContext,
};

use crate::math::Color;
use crate::nodes::NodeRef;
use crate::textures::CubeTexture;

/// `three.js/src/constants.js` sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Front,
    Back,
}

/// Port of `MeshBasicNodeMaterial.js` + the `NodeMaterial.js` / `Material.js`
/// fields the ladder uses. Defaults mirror three.js: white, opaque,
/// `FrontSide`, depth test on with `LessEqualDepth`, depth write on,
/// `reflectivity = 1`.
#[derive(Clone, Debug)]
pub struct MeshBasicNodeMaterial {
    pub color: Color,
    pub opacity: f64,
    pub reflectivity: f64,
    /// `MeshBasicMaterial.envMap` — `setupEnvironment()` turns it into
    /// `BasicEnvironmentNode( cubeTexture( envMap ) )`.
    pub env_map: Option<CubeTexture>,
    pub color_node: Option<NodeRef>,
    /// `NodeMaterial.vertexNode` — replaces the whole clip-position flow.
    pub vertex_node: Option<NodeRef>,
    /// `NodeMaterial.fragmentNode` — replaces the whole fragment flow.
    pub fragment_node: Option<NodeRef>,
    pub side: Side,
    pub depth_test: bool,
    pub depth_write: bool,
    /// `Background`'s material samples the cube map through the background
    /// uniforms rather than an env map.
    pub name: &'static str,
}

impl Default for MeshBasicNodeMaterial {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0),
            opacity: 1.0,
            reflectivity: 1.0,
            env_map: None,
            color_node: None,
            vertex_node: None,
            fragment_node: None,
            side: Side::Front,
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
}
