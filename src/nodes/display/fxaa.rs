//! Port of `three.js/examples/jsm/tsl/display/FXAANode.js`.
//!
//! The JS builds one `Fn` with a layout, `FxaaPixelShader( uv, texSize )`,
//! out of a handful of helper arrow functions and layout-less `Fn`s
//! (`Sample`, `SampleLuminance`, `SampleLuminanceOffset`) that inline at
//! their call sites. The port keeps the same helpers as Rust closures and
//! functions, in the same order, so the statements land in the same order
//! as in three's dump (`webgpu_postprocessing_fxaa`, `m05`).

use std::rc::Rc;

use crate::nodes::node::{FnDef, SettableValue, Type};
use crate::nodes::tsl::{
    block, boolean, break_loop, call, float, if_else, if_then, int, loop_range, shader_fn,
    smoothstep, texture_bias, to_var, uint, uniform_array_f32, uniform_settable, uv, vec2,
    vec2_join, vec3, UniformArray,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// `fxaa( node )` — FXAA 3.11-style anti-aliasing over an sRGB input.
///
/// Three.js runs its argument through `convertToTexture()`; the port takes
/// the texture and the caller converts first, as with
/// [`sobel`](super::sobel).
pub fn fxaa(map: &Texture) -> FxaaNode {
    FxaaNode::new(map)
}

/// `FXAANode`.
pub struct FxaaNode {
    map: Texture,
    /// `this._invSize`, written from the input's size in `updateBefore()`.
    inv_size: SettableValue,
    node: NodeRef,
}

/// The luminance neighbourhood `SampleLuminanceNeighborhood()` returns.
#[derive(Clone)]
struct Luminance {
    m: NodeRef,
    n: NodeRef,
    e: NodeRef,
    s: NodeRef,
    w: NodeRef,
    ne: NodeRef,
    nw: NodeRef,
    se: NodeRef,
    sw: NodeRef,
    highest: NodeRef,
    contrast: NodeRef,
}

/// What `DetermineEdge()` returns.
struct Edge {
    is_horizontal: NodeRef,
    pixel_step: NodeRef,
    opposite_luminance: NodeRef,
    gradient: NodeRef,
}

impl FxaaNode {
    /// `new FXAANode( node )`.
    pub fn new(map: &Texture) -> Self {
        let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);

        let def = fxaa_pixel_shader(map);
        // `fxaa = Fn( () => ApplyFXAA( uvNode, this._invSize ) )`, with
        // `uvNode` the RTT texture node's own `uv()`.
        let node = call(&def, vec![uv(), inv_size_node]);

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

    /// `FXAANode.updateBefore()`: `invSize` from the input's current size.
    /// Call it once a frame, after whatever sizes the input.
    pub fn update(&self) {
        let (width, height) = self.map.size();
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
    }
}

/// `ApplyFXAA`, the `Fn` whose layout names it `FxaaPixelShader`.
fn fxaa_pixel_shader(map: &Texture) -> Rc<FnDef> {
    let map = map.clone();
    shader_fn(
        Some("FxaaPixelShader"),
        vec![("uv", Type::Vec2), ("texSize", Type::Vec2)],
        Type::Vec4,
        move |args| {
            let (uv, tex_size) = (args[0].clone(), args[1].clone());

            // `EDGE_STEP_COUNT = float( 6 )`: three writes a `Loop` bound as
            // the constant's value in the index type, so the header reads
            // `i < 6` — the port hands `loop_range` the `int` directly.
            let edge_steps = uniform_array_f32(&[1.0, 1.5, 2.0, 2.0, 2.0, 4.0]);

            // `textureNode.bias( - 100 ).sample( uv )`.
            let sample = |uv: NodeRef| texture_bias(&map, uv, float(-100.0));
            let sample_luminance = |uv: NodeRef| sample(uv).rgb().dot(vec3(0.3, 0.59, 0.11));
            let sample_luminance_offset = |uv: &NodeRef, u: f64, v: f64| {
                let shifted_uv = uv.add(tex_size.mul(vec2(u, v)));
                sample_luminance(shifted_uv)
            };

            let l = {
                let m = sample_luminance(uv.clone());

                let n = sample_luminance_offset(&uv, 0.0, -1.0);
                let e = sample_luminance_offset(&uv, 1.0, 0.0);
                let s = sample_luminance_offset(&uv, 0.0, 1.0);
                let w = sample_luminance_offset(&uv, -1.0, 0.0);

                let ne = sample_luminance_offset(&uv, 1.0, -1.0);
                let nw = sample_luminance_offset(&uv, -1.0, -1.0);
                let se = sample_luminance_offset(&uv, 1.0, 1.0);
                let sw = sample_luminance_offset(&uv, -1.0, 1.0);

                // `max( s, e, n, w, m )` folds left.
                let highest = s.max(&e).max(&n).max(&w).max(&m);
                let lowest = s.min(&e).min(&n).min(&w).min(&m);
                let contrast = highest.sub(&lowest);

                Luminance {
                    m,
                    n,
                    e,
                    s,
                    w,
                    ne,
                    nw,
                    se,
                    sw,
                    highest,
                    contrast,
                }
            };

            let final_uv = to_var(None, uv.clone());

            // `ShouldSkipPixel( l )`.
            let threshold = float(0.0312).max(float(0.063).mul(&l.highest));
            let should_skip = l.contrast.less_than(threshold);

            let pixel_blend = determine_pixel_blend_factor(&l);
            let (edge, mut edge_statements) = determine_edge(&tex_size, &l);
            let (edge_blend, edge_blend_statements) = determine_edge_blend_factor(
                &tex_size,
                &l,
                &edge,
                &uv,
                &edge_steps,
                &sample_luminance,
            );
            edge_statements.extend(edge_blend_statements);

            let final_blend = pixel_blend.max(&edge_blend);

            edge_statements.push(if_else(
                edge.is_horizontal.clone(),
                vec![final_uv
                    .y()
                    .add_assign(edge.pixel_step.mul(final_blend.clone()))],
                vec![final_uv.x().add_assign(edge.pixel_step.mul(final_blend))],
            ));

            block(
                vec![
                    final_uv.clone(),
                    if_then(should_skip.not(), edge_statements),
                ],
                sample(final_uv),
            )
        },
    )
}

/// `DeterminePixelBlendFactor( l )`.
fn determine_pixel_blend_factor(l: &Luminance) -> NodeRef {
    let mut f = float(2.0).mul(l.s.add(&l.e).add(&l.n).add(&l.w));
    f = f.add(l.se.add(&l.sw).add(&l.ne).add(&l.nw));
    f = f.mul(1.0 / 12.0);
    f = f.sub(&l.m).abs();
    f = f
        .div(l.contrast.max(float(0.0)))
        .clamp(float(0.0), float(1.0));

    let blend_factor = smoothstep(0.0, 1.0, f);
    // `_SubpixelBlending = float( 1.0 )`.
    blend_factor.mul(&blend_factor).mul(float(1.0))
}

/// `DetermineEdge( texSize, l )` — the edge, and the statements its vars
/// and `If` put on the stack.
fn determine_edge(tex_size: &NodeRef, l: &Luminance) -> (Edge, Vec<NodeRef>) {
    let horizontal = l.s.add(&l.n).sub(l.m.mul(2.0)).abs().mul(2.0).add(
        l.se.add(&l.ne)
            .sub(l.e.mul(2.0))
            .abs()
            .add(l.sw.add(&l.nw).sub(l.w.mul(2.0)).abs()),
    );

    let vertical = l.e.add(&l.w).sub(l.m.mul(2.0)).abs().mul(2.0).add(
        l.se.add(&l.sw)
            .sub(l.s.mul(2.0))
            .abs()
            .add(l.ne.add(&l.nw).sub(l.n.mul(2.0)).abs()),
    );

    let is_horizontal = horizontal.greater_than_equal(vertical);

    let p_luminance = is_horizontal.select(l.s.clone(), l.e.clone());
    let n_luminance = is_horizontal.select(l.n.clone(), l.w.clone());
    let p_gradient = p_luminance.sub(&l.m).abs();
    let n_gradient = n_luminance.sub(&l.m).abs();

    let pixel_step = to_var(None, is_horizontal.select(tex_size.y(), tex_size.x()));
    let opposite_luminance = to_var(None, float(0.0));
    let gradient = to_var(None, float(0.0));

    let statements = vec![
        pixel_step.clone(),
        opposite_luminance.clone(),
        gradient.clone(),
        if_else(
            p_gradient.less_than(&n_gradient),
            vec![
                pixel_step.assign(pixel_step.negate()),
                opposite_luminance.assign(n_luminance),
                gradient.assign(n_gradient),
            ],
            vec![
                opposite_luminance.assign(p_luminance),
                gradient.assign(p_gradient),
            ],
        ),
    ];

    (
        Edge {
            is_horizontal,
            pixel_step,
            opposite_luminance,
            gradient,
        },
        statements,
    )
}

/// `DetermineEdgeBlendFactor( texSize, l, e, uv )` — the blend factor, and
/// the statements that compute it.
fn determine_edge_blend_factor(
    tex_size: &NodeRef,
    l: &Luminance,
    e: &Edge,
    uv: &NodeRef,
    edge_steps: &UniformArray,
    sample_luminance: &dyn Fn(NodeRef) -> NodeRef,
) -> (NodeRef, Vec<NodeRef>) {
    let mut statements = Vec::new();

    let uv_edge = to_var(None, uv.clone());
    let edge_step = to_var(None, vec2(0.0, 0.0));
    statements.extend([
        uv_edge.clone(),
        edge_step.clone(),
        if_else(
            e.is_horizontal.clone(),
            vec![
                uv_edge.y().add_assign(e.pixel_step.mul(0.5)),
                edge_step.assign(vec2_join(vec![tex_size.x(), float(0.0)])),
            ],
            vec![
                uv_edge.x().add_assign(e.pixel_step.mul(0.5)),
                edge_step.assign(vec2_join(vec![float(0.0), tex_size.y()])),
            ],
        ),
    ]);

    let edge_luminance = l.m.add(&e.opposite_luminance).mul(0.5);
    let gradient_threshold = e.gradient.mul(0.25);

    // One direction of the edge walk: `puv` stepping forward with `addAssign`,
    // or `nuv` stepping back with `subAssign`.
    let mut walk = |forward: bool| {
        let step = |target: &NodeRef, amount: NodeRef| {
            if forward {
                target.add(amount)
            } else {
                target.sub(amount)
            }
        };
        let step_assign = |target: &NodeRef, amount: NodeRef| {
            if forward {
                target.add_assign(amount)
            } else {
                target.sub_assign(amount)
            }
        };

        let puv = to_var(
            None,
            step(&uv_edge, edge_step.mul(edge_steps.element_x(uint(0)))),
        );
        let luminance_delta = to_var(None, sample_luminance(puv.clone()).sub(&edge_luminance));
        let at_end = to_var(
            None,
            luminance_delta
                .abs()
                .greater_than_equal(&gradient_threshold),
        );

        let lp = loop_range("i", int(1), int(6), |i| {
            vec![
                if_then(at_end.clone(), vec![break_loop()]),
                step_assign(&puv, edge_step.mul(edge_steps.element_x(i.clone()))),
                luminance_delta.assign(sample_luminance(puv.clone()).sub(&edge_luminance)),
                at_end.assign(
                    luminance_delta
                        .abs()
                        .greater_than_equal(&gradient_threshold),
                ),
            ]
        });

        // `EDGE_GUESS = float( 8.0 )`.
        let guess = if_then(
            at_end.not(),
            vec![step_assign(&puv, edge_step.mul(float(8.0)))],
        );

        statements.extend([
            puv.clone(),
            luminance_delta.clone(),
            at_end.clone(),
            lp,
            guess,
        ]);
        (puv, luminance_delta)
    };

    let (puv, p_luminance_delta) = walk(true);
    let (nuv, n_luminance_delta) = walk(false);

    let p_distance = to_var(None, float(0.0));
    let n_distance = to_var(None, float(0.0));
    let shortest_distance = to_var(None, float(0.0));
    let delta_sign = to_var(None, boolean(false));
    let blend_factor = to_var(None, float(0.0));

    statements.extend([
        p_distance.clone(),
        n_distance.clone(),
        if_else(
            e.is_horizontal.clone(),
            vec![
                p_distance.assign(puv.x().sub(uv.x())),
                n_distance.assign(uv.x().sub(nuv.x())),
            ],
            vec![
                p_distance.assign(puv.y().sub(uv.y())),
                n_distance.assign(uv.y().sub(nuv.y())),
            ],
        ),
        shortest_distance.clone(),
        delta_sign.clone(),
        if_else(
            p_distance.less_than_equal(&n_distance),
            vec![
                shortest_distance.assign(&p_distance),
                delta_sign.assign(p_luminance_delta.greater_than_equal(0.0)),
            ],
            vec![
                shortest_distance.assign(&n_distance),
                delta_sign.assign(n_luminance_delta.greater_than_equal(0.0)),
            ],
        ),
        blend_factor.clone(),
        if_else(
            delta_sign.equal(l.m.sub(&edge_luminance).greater_than_equal(0.0)),
            vec![blend_factor.assign(float(0.0))],
            vec![blend_factor
                .assign(float(0.5).sub(shortest_distance.div(p_distance.add(&n_distance))))],
        ),
    ]);

    (blend_factor, statements)
}
