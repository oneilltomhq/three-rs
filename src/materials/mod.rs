//! Ports of `three.js/src/materials` (node materials only — under
//! `WebGPURenderer` every material is a `NodeMaterial`).

use crate::math::Color;
use crate::textures::DepthTexture;

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

/// Port of `three.js/src/materials/nodes/MeshBasicNodeMaterial.js` (rung 1
/// subset). Defaults mirror `Material.js`: white, opaque, `FrontSide`,
/// depth test on with `LessEqualDepth`, depth write on.
#[derive(Clone, Debug, Default)]
pub struct MeshBasicNodeMaterial {
    pub color_node: Option<ColorNode>,
}

impl MeshBasicNodeMaterial {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pipeline/shader variant this material needs.
    pub fn shader_key(&self) -> ShaderKey {
        match &self.color_node {
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
    /// `MeshBasicNodeMaterial` with `colorNode = texture( depthTexture )`,
    /// drawn as a `QuadMesh`.
    DepthTextureQuad,
    /// `MeshBasicNodeMaterial` with
    /// `colorNode = mix( normalWorld, range( … ), oscSine( time.mul( 0.1 ) ) )`.
    NormalWorldRangeMix,
    /// The `NodeMaterial` `Renderer._renderOutput()` builds for its output
    /// `QuadMesh`: `fragmentNode = nodes.getOutputNode( frameBufferTexture )`.
    OutputColorTransform,
}
