//! Port of `three.js/examples/jsm/tsl/display/Lut3DNode.js` — colour grading
//! through a 3D lookup table, as `webgpu_postprocessing_3dlut` applies the
//! tables `LutCubeLoader`, `Lut3dlLoader` and `LutImageLoader` read.
//!
//! Not ported: swapping the table after the node is built. Upstream's page
//! writes `lutPass.lutNode.value = lut.texture3D` every frame, which rebinds
//! the texture under the same program; a texture is an identity in the
//! port's graph (as `TransitionNode`'s mix texture is), so the table is fixed
//! when the node is built and a page that changes it builds a new node. The
//! `size` uniform stays settable, as upstream's `lutPass.size.value` is.

use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{float, mix, uniform_settable, vec4_join, Texture3DNode};
use crate::nodes::NodeRef;

/// `lut3D( node, lut, size, intensity )`.
pub fn lut_3d(input: NodeRef, lut: &Texture3DNode, size: f64, intensity: NodeRef) -> Lut3DNode {
    Lut3DNode::new(input, lut, size, intensity)
}

/// `Lut3DNode`: `mix( base, vec4( lut.sample( uvw ).rgb, base.a ), intensity )`,
/// where `uvw` pulls `base.rgb` in by half a texel so the edge texels are
/// sampled at their centres.
#[derive(Clone, Debug)]
pub struct Lut3DNode {
    output: NodeRef,
    size: SettableValue,
}

impl Lut3DNode {
    /// `new Lut3DNode( inputNode, lutNode, size, intensityNode )`: `size` is
    /// wrapped in a `uniform()`, `intensity` is used as given (the page passes
    /// `uniform( 1 )`).
    pub fn new(input: NodeRef, lut: &Texture3DNode, size: f64, intensity: NodeRef) -> Self {
        let (size_node, size) = uniform_settable(Type::F32, vec![size]);

        let base = input;

        // pull the sample in by half a pixel so the sample begins at the
        // center of the edge pixels.
        let pixel_width = float(1.0).div(size_node.clone());
        let half_pixel_width = float(0.5).div(size_node);
        let uvw = half_pixel_width
            .to(Type::Vec3)
            .add(base.rgb().mul(float(1.0).sub(pixel_width)));

        let lut_value = vec4_join(vec![lut.sample(uvw).rgb(), base.a()]);

        let output = mix(base, lut_value, intensity);
        Self { output, size }
    }

    /// The node to hand to `render_pipeline.output_node`.
    pub fn node(&self) -> NodeRef {
        self.output.clone()
    }

    /// `lutPass.size` — the table side, `lut.texture3D.image.width`.
    pub fn size(&self) -> &SettableValue {
        &self.size
    }
}
