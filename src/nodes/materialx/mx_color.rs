//! Port of `three.js/src/nodes/materialx/MaterialXColor.js` (HSV) and
//! `MaterialXColorTransform.js` (the sRGB texture decode).
//!
//! Unlike the noise bodies these were written by hand in three, so there are
//! no reverse-order parameter copies: `mx_hsvtorgb` reads `hsv` directly.

use std::rc::Rc;

use crate::nodes::node::{FnDef, Lazy, Node, NodeRef, Type};
use crate::nodes::tsl::{
    block, call, float, if_else, if_then, int, mix, shader_fn, to_var, vec3, vec3_join,
};

/// `trunc( x )`. Built here rather than in `tsl` because nothing else uses it
/// yet; it is a plain `MathNode` like `floor`.
fn trunc(x: NodeRef) -> NodeRef {
    let ty = x.ty();
    NodeRef::new(Node::Math {
        name: "trunc",
        args: vec![x],
        ty,
    })
}

/// `If( c0, a0 ).ElseIf( c1, a1 )… .Else( last )`: each `ElseIf` nests a
/// whole `If` inside the previous else block, as `StackNode.ElseIf()` does.
fn if_chain(arms: Vec<(NodeRef, Vec<NodeRef>)>, last: Vec<NodeRef>) -> NodeRef {
    arms.into_iter()
        .rev()
        .fold(last, |else_body, (cond, body)| {
            vec![if_else(cond, body, else_body)]
        })
        .pop()
        .expect("at least one arm")
}

mx_fn!(
    mx_hsvtorgb_def,
    "mx_hsvtorgb",
    vec![("hsv", Type::Vec3)],
    Type::Vec3,
    |params| {
        let hsv = &params[0];
        let (s, v) = (hsv.y(), hsv.z());
        let result = to_var(None, vec3(0.0, 0.0, 0.0));
        // `hi` is not a var in three (only `h` is), so `int( trunc( h ) )` is
        // written out again in every condition.
        //
        // Divergence (docs/nodes.md §8): three turns f / p / q / t into vars
        // the second time they are generated and then reads those vars from
        // *sibling* branches, where they were never assigned, so its
        // `mx_hsvtorgb` is wrong for hue sectors 2 to 5. The port hoists each
        // shared value inside the branch that uses it.
        let h = to_var(None, hsv.x().sub(hsv.x().floor()).mul(float(6.0)));
        let hi = trunc(h.clone()).to(Type::I32);
        let f = h.sub(hi.to(Type::F32));
        let p = v.mul(s.one_minus());
        let q = v.mul(s.mul(&f).one_minus());
        let t = v.mul(s.mul(f.one_minus()).one_minus());
        let arm = |i: i64, rgb: [&NodeRef; 3]| {
            (
                hi.equal(int(i)),
                vec![result.assign(vec3_join(rgb.iter().map(|c| (*c).clone()).collect()))],
            )
        };
        let chain = if_chain(
            vec![
                arm(0, [&v, &t, &p]),
                arm(1, [&q, &v, &p]),
                arm(2, [&p, &v, &t]),
                arm(3, [&p, &q, &v]),
                arm(4, [&t, &p, &v]),
            ],
            vec![result.assign(vec3_join(vec![v.clone(), p.clone(), q.clone()]))],
        );
        let body = vec![
            result.clone(),
            if_else(
                s.less_than(float(0.0001)),
                vec![result.assign(vec3_join(vec![v.clone(), v.clone(), v.clone()]))],
                vec![h, chain],
            ),
        ];
        block(body, result)
    }
);

mx_fn!(
    mx_rgbtohsv_def,
    "mx_rgbtohsv",
    vec![("c", Type::Vec3)],
    Type::Vec3,
    |params| {
        let c = to_var(None, params[0].clone());
        let r = to_var(None, c.x());
        let g = to_var(None, c.y());
        let b = to_var(None, c.z());
        let mincomp = to_var(None, r.min(g.min(&b)));
        let maxcomp = to_var(None, r.max(g.max(&b)));
        let delta = to_var(None, maxcomp.sub(&mincomp));
        let h = to_var(None, float(0.0));
        let s = to_var(None, float(0.0));
        let v = to_var(None, float(0.0));
        let hue = if_chain(
            vec![
                (
                    r.greater_than_equal(&maxcomp),
                    vec![h.assign(g.sub(&b).div(&delta))],
                ),
                (
                    g.greater_than_equal(&maxcomp),
                    vec![h.assign(float(2.0).add(b.sub(&r).div(&delta)))],
                ),
            ],
            vec![h.assign(float(4.0).add(r.sub(&g).div(&delta)))],
        );
        let body = vec![
            c.clone(),
            r.clone(),
            g.clone(),
            b.clone(),
            mincomp,
            maxcomp.clone(),
            delta.clone(),
            h.clone(),
            s.clone(),
            v.clone(),
            v.assign(&maxcomp),
            if_else(
                maxcomp.greater_than(float(0.0)),
                vec![s.assign(delta.div(&maxcomp))],
                vec![s.assign(float(0.0))],
            ),
            if_else(
                s.less_than_equal(float(0.0)),
                vec![h.assign(float(0.0))],
                vec![
                    hue,
                    h.mul_assign(float(1.0 / 6.0)),
                    if_then(h.less_than(float(0.0)), vec![h.add_assign(float(1.0))]),
                ],
            ),
        ];
        block(body, vec3_join(vec![h, s, v]))
    }
);

mx_fn!(
    mx_srgb_texture_to_lin_rec709_def,
    "mx_srgb_texture_to_lin_rec709",
    vec![("color", Type::Vec3)],
    Type::Vec3,
    |params| {
        let color = to_var(None, params[0].clone());
        let is_above = to_var(
            None,
            color
                .greater_than(vec3(0.04045, 0.04045, 0.04045))
                .to(Type::BVec3),
        );
        let lin_seg = to_var(None, color.div(float(12.92)));
        let pow_seg = to_var(
            None,
            color
                .add(vec3(0.055, 0.055, 0.055))
                .max(vec3(0.0, 0.0, 0.0))
                .div(float(1.055))
                .pow(vec3(2.4, 2.4, 2.4)),
        );
        let body = vec![color, is_above.clone(), lin_seg.clone(), pow_seg.clone()];
        // `MathNode` builds a non-scalar `mix()` weight as the output type, so
        // the `bvec3` arrives as `vec3<f32>( isAbove )`.
        block(body, mix(lin_seg, pow_seg, is_above.to(Type::Vec3)))
    }
);

/// `mx_hsvtorgb( hsv )`.
pub(super) fn mx_hsvtorgb(hsv: NodeRef) -> NodeRef {
    call(&mx_hsvtorgb_def(), vec![hsv])
}

/// `mx_rgbtohsv( c )`.
pub(super) fn mx_rgbtohsv(c: NodeRef) -> NodeRef {
    call(&mx_rgbtohsv_def(), vec![c])
}

/// `mx_srgb_texture_to_lin_rec709( color )`.
pub(super) fn mx_srgb_texture_to_lin_rec709(color: NodeRef) -> NodeRef {
    call(&mx_srgb_texture_to_lin_rec709_def(), vec![color])
}
