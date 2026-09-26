//! Port of `three.js/src/nodes/materialx/MaterialXCore.js` — the two rotation
//! helpers the unified noises and `mx_place2d` build on. Both are plain JS
//! functions, not `Fn()`s, so their nodes land directly in the caller.

use crate::nodes::node::{NodeRef, Type};
use crate::nodes::tsl::float;

/// Degrees to radians, as three writes it: `Math.PI / 180.0`.
const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// `mx_rotate2d( input, amount = 0 )` — rotates `input` by `amount` degrees.
pub fn mx_rotate2d(input: NodeRef, amount: impl Into<NodeRef>) -> NodeRef {
    let input = input.to(Type::Vec2);
    let amount = amount.into().to(Type::F32);
    let radians = amount.mul(float(DEG_TO_RAD));
    let (sa, ca) = (radians.sin(), radians.cos());
    let (x, y) = (input.x(), input.y());
    crate::nodes::tsl::vec2_join(vec![ca.mul(&x).add(sa.mul(&y)), ca.mul(&y).sub(sa.mul(&x))])
}

/// `mx_rotate3d( input, amount = 0, axis = vec3( 0, 1, 0 ) )` — Rodrigues'
/// rotation of `input` by `amount` degrees about `axis`, in MaterialX's
/// row-vector convention (so the cross product is `input × axis`).
pub fn mx_rotate3d(
    input: NodeRef,
    amount: impl Into<NodeRef>,
    axis: impl Into<NodeRef>,
) -> NodeRef {
    let input = input.to(Type::Vec3);
    let amount = amount.into().to(Type::F32);
    let axis = axis.into().to(Type::Vec3).normalize();
    let radians = amount.mul(float(DEG_TO_RAD));
    let (s, c) = (radians.sin(), radians.cos());
    let oc = float(1.0).sub(&c);
    input
        .mul(&c)
        .add(input.cross(&axis).mul(s))
        .add(axis.mul(axis.dot(&input).mul(oc)))
}
