//! Ports of `three.js/src/materials` (node materials only — under
//! `WebGPURenderer` every material is a `NodeMaterial`).

use crate::math::Color;
use crate::textures::{CubeTexture, DepthTexture};

/// The `colorNode` of a material — one variant per node graph the ladder has
/// reached so far.
///
/// This enum is scaffolding: rung 4 ports the TSL node graph and this becomes a
/// real node reference.
#[derive(Clone, Debug)]
pub enum ColorNode {
    /// `texture( depthTexture )`.
    DepthTexture(DepthTexture),
    /// `mix( normalWorld, range( min, max ), oscSine( time.mul( 0.1 ) ) )`.
    ///
    /// `range()` resolves, per instance, to a `vec4` of
    /// `MathUtils.lerp( min[ c ], max[ c ], Math.random() )` per component —
    /// see `RangeNode.setup()`.
    NormalWorldRangeMix { min: Color, max: Color },
}

/// Port of `three.js/src/materials/nodes/MeshBasicNodeMaterial.js` (rung 3
/// subset). Defaults mirror `Material.js` + `MeshBasicMaterial.js`: white,
/// opaque, `FrontSide`, depth test on with `LessEqualDepth`, depth write on,
/// `reflectivity = 1`, `combine = MultiplyOperation`.
#[derive(Clone, Debug)]
pub struct MeshBasicNodeMaterial {
    /// `MeshBasicMaterial.color`, in the working (linear-sRGB) colour space —
    /// the `materialColor` uniform the generated WGSL calls `diffuse`.
    pub color: Color,
    /// `Material.opacity`.
    pub opacity: f64,
    /// `MeshBasicMaterial.reflectivity`.
    pub reflectivity: f64,
    /// `MeshBasicMaterial.envMap` — `MeshBasicNodeMaterial.setupEnvironment()`
    /// turns it into `BasicEnvironmentNode( cubeTexture( envMap ) )`.
    pub env_map: Option<CubeTexture>,
    pub color_node: Option<ColorNode>,
}

impl Default for MeshBasicNodeMaterial {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0),
            opacity: 1.0,
            reflectivity: 1.0,
            env_map: None,
            color_node: None,
        }
    }
}

impl MeshBasicNodeMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pipeline/shader variant this material needs.
    pub fn shader_key(&self) -> ShaderKey {
        match &self.color_node {
            None if self.env_map.is_some() => ShaderKey::BasicEnvMap,
            None => ShaderKey::Basic,
            Some(ColorNode::DepthTexture(_)) => ShaderKey::DepthTextureQuad,
            Some(ColorNode::NormalWorldRangeMix { .. }) => ShaderKey::NormalWorldRangeMix,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShaderKey {
    /// `MeshBasicNodeMaterial` with no `colorNode`: flat material colour.
    Basic,
    /// `MeshBasicNodeMaterial` with an `envMap`: the `BasicEnvironmentNode` /
    /// `CubeMapNode` reflection path.
    BasicEnvMap,
    /// `MeshBasicNodeMaterial` with `colorNode = texture( depthTexture )`,
    /// drawn as a `QuadMesh`.
    DepthTextureQuad,
    /// `MeshBasicNodeMaterial` with
    /// `colorNode = mix( normalWorld, range( … ), oscSine( time.mul( 0.1 ) ) )`.
    NormalWorldRangeMix,
    /// The `NodeMaterial` `Renderer._renderOutput()` builds for its output
    /// `QuadMesh`: `fragmentNode = nodes.getOutputNode( frameBufferTexture )`.
    OutputColorTransform,
    /// The `NodeMaterial` `Background.update()` builds for a `CubeTexture`
    /// `scene.background`, drawn on a `SphereGeometry( 1, 32, 32 )` skybox.
    BackgroundCube,
}
