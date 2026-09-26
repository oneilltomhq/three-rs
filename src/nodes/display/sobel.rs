//! Port of `three.js/examples/jsm/tsl/display/SobelOperatorNode.js`.

use crate::math::Matrix3;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    float, luminance, mat3, texture_uv, uniform_settable, uv, vec2, vec3_join, vec4_join,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `sobel( node )` — a Sobel edge detector over the luminance of `map`.
pub struct SobelOperatorNode {
    map: Texture,
    /// `this._invSize`, written from the input's size in `updateBefore()`.
    inv_size: SettableValue,
    node: NodeRef,
}

/// `sobel( node )`. Three.js runs its argument through `convertToTexture()`;
/// the port takes the texture and the caller converts first, as with
/// [`gaussian_blur`](super::gaussian_blur).
pub fn sobel(map: &Texture) -> SobelOperatorNode {
    SobelOperatorNode::new(map)
}

impl SobelOperatorNode {
    pub fn new(map: &Texture) -> Self {
        let (texel, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let uv_node = uv();
        let sample = |offset: (f64, f64)| {
            let coord = uv_node
                .clone()
                .add(texel.clone().mul(vec2(offset.0, offset.1)));
            luminance(texture_uv(map, coord).xyz())
        };

        // kernel definition (in glsl matrices are filled in column-major order)
        let gx = mat3(Matrix3 {
            elements: [-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0],
        }); // x direction kernel
        let gy = mat3(Matrix3 {
            elements: [-1.0, 0.0, 1.0, -2.0, 0.0, 2.0, -1.0, 0.0, 1.0],
        }); // y direction kernel

        // fetch the 3x3 neighbourhood of a fragment

        // first column
        let tx0y0 = sample((-1.0, -1.0));
        let tx0y1 = sample((-1.0, 0.0));
        let tx0y2 = sample((-1.0, 1.0));

        // second column
        let tx1y0 = sample((0.0, -1.0));
        let tx1y1 = sample((0.0, 0.0));
        let tx1y2 = sample((0.0, 1.0));

        // third column
        let tx2y0 = sample((1.0, -1.0));
        let tx2y1 = sample((1.0, 0.0));
        let tx2y2 = sample((1.0, 1.0));

        // `add( G[ 0 ][ 0 ].mul( tx0y0 ), G[ 1 ][ 0 ].mul( tx1y0 ), … )` —
        // `add()` with nine arguments folds left.
        let gradient = |g: &NodeRef| {
            let taps = [
                ((0, 0), &tx0y0),
                ((1, 0), &tx1y0),
                ((2, 0), &tx2y0),
                ((0, 1), &tx0y1),
                ((1, 1), &tx1y1),
                ((2, 1), &tx2y1),
                ((0, 2), &tx0y2),
                ((1, 2), &tx1y2),
                ((2, 2), &tx2y2),
            ];
            let mut sum: Option<NodeRef> = None;
            for ((col, row), tap) in taps {
                let term = g.element(col).element(row).mul((*tap).clone());
                sum = Some(match sum {
                    Some(sum) => sum.add(term),
                    None => term,
                });
            }
            sum.expect("three-rs: nine taps")
        };

        // gradient value in x direction
        let value_gx = gradient(&gx);
        // gradient value in y direction
        let value_gy = gradient(&gy);

        // magnitude of the total gradient
        let g = value_gx
            .clone()
            .mul(value_gx)
            .add(value_gy.clone().mul(value_gy))
            .sqrt();

        let node = vec4_join(vec![vec3_join(vec![g]), float(1.0)]);

        Self {
            map: map.clone(),
            inv_size,
            node,
        }
    }

    /// The node for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `SobelOperatorNode.updateBefore()`: `invSize` from the input's current
    /// size. Call it once a frame, after whatever sizes the input.
    pub fn update(&self) {
        let (width, height) = self.map.size();
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
    }
}
