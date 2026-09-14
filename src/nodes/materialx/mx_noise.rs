//! Port of `three.js/src/nodes/materialx/MaterialXNoise.js` — the Perlin /
//! fractal noise chain, plus the `MaterialXNodes.js` wrappers that give it its
//! defaults.
//!
//! Three's file is machine-converted from MaterialX's GLSL, and the conversion
//! left a very particular shape behind: every `Fn()` body opens by copying its
//! parameters into locals **in reverse parameter order**
//!
//! ```ignore
//! const k = int( k_immutable ).toVar();
//! const x = uint( x_immutable ).toVar();
//! ```
//!
//! so `mx_rotl32`'s emitted WGSL begins `nodeVar0 = k; nodeVar1 = x;`. That is
//! visible in every dumped shader, so it is reproduced here: `reverse_vars()`
//! builds the `toVar()` copies last-parameter-first and returns them as the
//! leading statements of a `block()`, and the bodies below reference those vars
//! rather than the raw parameters.
//!
//! The other conversion artefact is `mx_select` / `mx_negate_if`: both return
//! `select( … ).uniformFlow()`, and `ConditionalNode.generate()` allocates its
//! result property *before* it notices the uniform-flow context, so the
//! function declares one `var` it never assigns. `unassigned_var()` reproduces
//! that dangling declaration.

use std::rc::Rc;

use crate::nodes::node::{FnDef, Lazy, NodeRef, Type};
use crate::nodes::tsl::{
    block, call, float, int, join, loop_n, property, shader_fn, to_var, uint, vec3, vec3_join,
    wgsl_select,
};

// ---------------------------------------------------------------------------
// the two conversion artefacts
// ---------------------------------------------------------------------------

/// The reverse-order parameter copies every converted MaterialX body opens
/// with. Returns `(statements, vars)`: `statements` is the copies in emission
/// order (last parameter first), `vars` is indexed like `params`.
fn reverse_vars(params: &[NodeRef]) -> (Vec<NodeRef>, Vec<NodeRef>) {
    let statements: Vec<NodeRef> = params
        .iter()
        .rev()
        .map(|p| to_var(None, p.clone()))
        .collect();
    let vars: Vec<NodeRef> = statements.iter().rev().cloned().collect();
    (statements, vars)
}

/// `ConditionalNode.generate()`'s `nodeProperty`, allocated unconditionally and
/// then left unassigned on the `uniformFlow` path — a bare `var nodeVarN : T;`.
/// The port numbers `nodeVarN` from a counter that `property()` does not touch,
/// so the name is spelled out at the one position it can occupy.
fn unassigned_var(name: &'static str, ty: Type) -> NodeRef {
    property(name, ty)
}

macro_rules! mx_fn {
    ($rust:ident, $wgsl:literal, $params:expr, $ret:expr, $body:expr) => {
        fn $rust() -> Rc<FnDef> {
            thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
            CELL.with(|c| c.get(|| shader_fn(Some($wgsl), $params, $ret, $body)))
        }
    };
}

// ---------------------------------------------------------------------------
// scalar helpers
// ---------------------------------------------------------------------------

mx_fn!(
    mx_select_def,
    "mx_select",
    vec![("b", Type::Bool), ("t", Type::F32), ("f", Type::F32)],
    Type::F32,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        stmts.push(unassigned_var("nodeVar3", Type::F32));
        block(stmts, wgsl_select(v[2].clone(), v[1].clone(), v[0].clone()))
    }
);

mx_fn!(
    mx_negate_if_def,
    "mx_negate_if",
    vec![("val", Type::F32), ("b", Type::Bool)],
    Type::F32,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        stmts.push(unassigned_var("nodeVar2", Type::F32));
        let (val, b) = (v[0].clone(), v[1].clone());
        block(stmts, wgsl_select(val.clone(), val.negate(), b))
    }
);

mx_fn!(
    mx_floor_def,
    "mx_floor",
    vec![("x", Type::F32)],
    Type::I32,
    |params| {
        let (stmts, v) = reverse_vars(params);
        block(stmts, v[0].floor().to(Type::I32))
    }
);

/// `mx_floorfrac( x, i )` — an `Fn()` with no layout, so it is inlined: the
/// `int` out-parameter takes `mx_floor( x )` and the fraction is returned.
fn mx_floorfrac(x: NodeRef, i: &NodeRef) -> NodeRef {
    let x = to_var(None, x);
    block(
        vec![x.clone(), i.assign(call(&mx_floor_def(), vec![x.clone()]))],
        x.sub(i.to(Type::F32)),
    )
}

mx_fn!(
    mx_fade_def,
    "mx_fade",
    vec![("t", Type::F32)],
    Type::F32,
    |params| {
        let (stmts, v) = reverse_vars(params);
        let t = v[0].clone();
        block(
            stmts,
            t.mul(&t)
                .mul(&t)
                .mul(t.mul(t.mul(float(6.0)).sub(float(15.0))).add(float(10.0))),
        )
    }
);

// ---------------------------------------------------------------------------
// the Jenkins one-at-a-time hash (`u32`, wrapping)
// ---------------------------------------------------------------------------

mx_fn!(
    mx_rotl32_def,
    "mx_rotl32",
    vec![("x", Type::U32), ("k", Type::I32)],
    Type::U32,
    |params| {
        let (stmts, v) = reverse_vars(params);
        let (x, k) = (v[0].clone(), v[1].clone());
        block(
            stmts,
            x.shift_left(k.to(Type::U32))
                .bit_or(x.shift_right(int(32).sub(&k).to(Type::U32))),
        )
    }
);

mx_fn!(
    mx_bjfinal_def,
    "mx_bjfinal",
    vec![("a", Type::U32), ("b", Type::U32), ("c", Type::U32)],
    Type::U32,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        let (a, b, c) = (v[0].clone(), v[1].clone(), v[2].clone());
        let rotl = mx_rotl32_def();
        let rot = |n: &NodeRef, k: i32| call(&rotl, vec![n.clone(), int(k as i64)]);

        stmts.push(c.assign(c.bit_xor(&b)));
        stmts.push(c.assign(c.sub(rot(&b, 14))));
        stmts.push(a.assign(a.bit_xor(&c)));
        stmts.push(a.assign(a.sub(rot(&c, 11))));
        stmts.push(b.assign(b.bit_xor(&a)));
        stmts.push(b.assign(b.sub(rot(&a, 25))));
        stmts.push(c.assign(c.bit_xor(&b)));
        stmts.push(c.assign(c.sub(rot(&b, 16))));
        stmts.push(a.assign(a.bit_xor(&c)));
        stmts.push(a.assign(a.sub(rot(&c, 4))));
        stmts.push(b.assign(b.bit_xor(&a)));
        stmts.push(b.assign(b.sub(rot(&a, 14))));
        stmts.push(c.assign(c.bit_xor(&b)));
        stmts.push(c.assign(c.sub(rot(&b, 24))));

        block(stmts, c)
    }
);

mx_fn!(
    mx_hash_int_2_def,
    "mx_hash_int_2",
    vec![("x", Type::I32), ("y", Type::I32), ("z", Type::I32)],
    Type::U32,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        let len = to_var(None, uint(3));
        let a = to_var(None, uint(0));
        let b = to_var(None, uint(0));
        let c = to_var(None, uint(0));
        stmts.extend([len.clone(), a.clone(), b.clone(), c.clone()]);

        // `0xdeadbeef + ( len << 2 ) + 13`, seeded into all three words.
        let seed = uint(0xdead_beef).add(len.shift_left(uint(2))).add(uint(13));
        stmts.push(a.assign(b.assign(c.assign(seed))));
        stmts.push(a.assign(a.add(v[0].to(Type::U32))));
        stmts.push(b.assign(b.add(v[1].to(Type::U32))));
        stmts.push(c.assign(c.add(v[2].to(Type::U32))));

        block(stmts, call(&mx_bjfinal_def(), vec![a, b, c]))
    }
);

mx_fn!(
    mx_hash_vec3_1_def,
    "mx_hash_vec3_1",
    vec![("x", Type::I32), ("y", Type::I32), ("z", Type::I32)],
    Type::UVec3,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        let h = to_var(
            None,
            call(
                &mx_hash_int_2_def(),
                vec![v[0].clone(), v[1].clone(), v[2].clone()],
            ),
        );
        // `uvec3()` — the three low bytes of the hash, one per component.
        let result = to_var(None, join(Type::UVec3, vec![uint(0), uint(0), uint(0)]));
        stmts.extend([h.clone(), result.clone()]);
        stmts.push(result.x().assign(h.bit_and(uint(0xFF))));
        stmts.push(
            result
                .y()
                .assign(h.shift_right(uint(8)).bit_and(uint(0xFF))),
        );
        stmts.push(
            result
                .z()
                .assign(h.shift_right(uint(16)).bit_and(uint(0xFF))),
        );
        block(stmts, result)
    }
);

// ---------------------------------------------------------------------------
// gradients
// ---------------------------------------------------------------------------

mx_fn!(
    mx_gradient_float_1_def,
    "mx_gradient_float_1",
    vec![
        ("hash", Type::U32),
        ("x", Type::F32),
        ("y", Type::F32),
        ("z", Type::F32)
    ],
    Type::F32,
    |params| {
        let (mut stmts, v) = reverse_vars(params);
        let (hash, x, y, z) = (v[0].clone(), v[1].clone(), v[2].clone(), v[3].clone());
        let select = mx_select_def();

        let h = to_var(None, hash.bit_and(uint(15)));
        let u = to_var(
            None,
            call(&select, vec![h.less_than(uint(8)), x.clone(), y.clone()]),
        );
        let w = to_var(
            None,
            call(
                &select,
                vec![
                    h.less_than(uint(4)),
                    y.clone(),
                    call(
                        &select,
                        vec![
                            h.equal(uint(12)).or(h.equal(uint(14))),
                            x.clone(),
                            z.clone(),
                        ],
                    ),
                ],
            ),
        );
        stmts.extend([h.clone(), u.clone(), w.clone()]);

        let negate_if = mx_negate_if_def();
        block(
            stmts,
            call(&negate_if, vec![u, h.bit_and(uint(1)).to(Type::Bool)])
                .add(call(&negate_if, vec![w, h.bit_and(uint(2)).to(Type::Bool)])),
        )
    }
);

mx_fn!(
    mx_gradient_vec3_1_def,
    "mx_gradient_vec3_1",
    vec![
        ("hash", Type::UVec3),
        ("x", Type::F32),
        ("y", Type::F32),
        ("z", Type::F32)
    ],
    Type::Vec3,
    |params| {
        let (stmts, v) = reverse_vars(params);
        let (hash, x, y, z) = (v[0].clone(), v[1].clone(), v[2].clone(), v[3].clone());
        let g = mx_gradient_float_1_def();
        let component = |c: NodeRef| call(&g, vec![c, x.clone(), y.clone(), z.clone()]);
        block(
            stmts,
            vec3_join(vec![
                component(hash.x()),
                component(hash.y()),
                component(hash.z()),
            ]),
        )
    }
);

mx_fn!(
    mx_gradient_scale3d_0_def,
    "mx_gradient_scale3d_0",
    vec![("v", Type::F32)],
    Type::F32,
    |params| {
        let (stmts, v) = reverse_vars(params);
        block(stmts, float(0.982).mul(&v[0]))
    }
);

mx_fn!(
    mx_gradient_scale3d_1_def,
    "mx_gradient_scale3d_1",
    vec![("v", Type::Vec3)],
    Type::Vec3,
    |params| {
        let (stmts, v) = reverse_vars(params);
        block(stmts, float(0.982).mul(&v[0]))
    }
);

// ---------------------------------------------------------------------------
// trilinear interpolation
// ---------------------------------------------------------------------------

/// `mxTrilerpValue` — shared by the `float` and `vec3` overloads; the operand
/// padding (`vec3<f32>( r1 ) * …`) falls out of `OperatorNode`'s own widening.
fn trilerp_body(params: &[NodeRef]) -> NodeRef {
    let (mut stmts, v) = reverse_vars(params);
    let (s, t, r) = (v[8].clone(), v[9].clone(), v[10].clone());
    let s1 = to_var(None, float(1.0).sub(&s));
    let t1 = to_var(None, float(1.0).sub(&t));
    let r1 = to_var(None, float(1.0).sub(&r));
    stmts.extend([s1.clone(), t1.clone(), r1.clone()]);

    let face = |a: &NodeRef, b: &NodeRef, c: &NodeRef, d: &NodeRef| {
        t1.mul(a.mul(&s1).add(b.mul(&s)))
            .add(t.mul(c.mul(&s1).add(d.mul(&s))))
    };
    let lo = face(&v[0], &v[1], &v[2], &v[3]);
    let hi = face(&v[4], &v[5], &v[6], &v[7]);

    block(stmts, r1.mul(lo).add(r.mul(hi)))
}

fn trilerp_params(ty: Type) -> Vec<(&'static str, Type)> {
    vec![
        ("v0", ty),
        ("v1", ty),
        ("v2", ty),
        ("v3", ty),
        ("v4", ty),
        ("v5", ty),
        ("v6", ty),
        ("v7", ty),
        ("s", Type::F32),
        ("t", Type::F32),
        ("r", Type::F32),
    ]
}

mx_fn!(
    mx_trilerp_0_def,
    "mx_trilerp_0",
    trilerp_params(Type::F32),
    Type::F32,
    trilerp_body
);

mx_fn!(
    mx_trilerp_1_def,
    "mx_trilerp_1",
    trilerp_params(Type::Vec3),
    Type::Vec3,
    trilerp_body
);

// ---------------------------------------------------------------------------
// Perlin noise
// ---------------------------------------------------------------------------

/// The body both `mx_perlin_noise_*_1` overloads share: `hash` produces the
/// eight corner hashes, `gradient` turns one into a gradient, `trilerp` blends
/// them, and `scale` is the `0.982` normalisation.
fn perlin_body(
    params: &[NodeRef],
    hash: &Rc<FnDef>,
    gradient: &Rc<FnDef>,
    trilerp: &Rc<FnDef>,
    scale: &Rc<FnDef>,
) -> NodeRef {
    let (mut stmts, v) = reverse_vars(params);
    let p = v[0].clone();

    let xi = to_var(None, int(0));
    let yi = to_var(None, int(0));
    let zi = to_var(None, int(0));
    stmts.extend([xi.clone(), yi.clone(), zi.clone()]);

    let fx = to_var(None, mx_floorfrac(p.x(), &xi));
    let fy = to_var(None, mx_floorfrac(p.y(), &yi));
    let fz = to_var(None, mx_floorfrac(p.z(), &zi));
    stmts.extend([fx.clone(), fy.clone(), fz.clone()]);

    let fade = mx_fade_def();
    let u = to_var(None, call(&fade, vec![fx.clone()]));
    let w = to_var(None, call(&fade, vec![fy.clone()]));
    let q = to_var(None, call(&fade, vec![fz.clone()]));
    stmts.extend([u.clone(), w.clone(), q.clone()]);

    // Every `X + 1` / `fx - 1.0` has to be a *fresh* node: a shared one would
    // be reached twice and `TempNode.hasDependencies()` would hoist it into a
    // var of its own.
    let cell = |b: bool, c: &NodeRef| {
        if b {
            c.add(int(1))
        } else {
            c.clone()
        }
    };
    let frac = |b: bool, f: &NodeRef| {
        if b {
            f.sub(float(1.0))
        } else {
            f.clone()
        }
    };
    let corner = |dx: bool, dy: bool, dz: bool| {
        call(
            gradient,
            vec![
                call(hash, vec![cell(dx, &xi), cell(dy, &yi), cell(dz, &zi)]),
                frac(dx, &fx),
                frac(dy, &fy),
                frac(dz, &fz),
            ],
        )
    };

    let result = to_var(
        None,
        call(
            trilerp,
            vec![
                corner(false, false, false),
                corner(true, false, false),
                corner(false, true, false),
                corner(true, true, false),
                corner(false, false, true),
                corner(true, false, true),
                corner(false, true, true),
                corner(true, true, true),
                u,
                w,
                q,
            ],
        ),
    );
    stmts.push(result.clone());

    block(stmts, call(scale, vec![result]))
}

mx_fn!(
    mx_perlin_noise_float_1_def,
    "mx_perlin_noise_float_1",
    vec![("p", Type::Vec3)],
    Type::F32,
    |params| perlin_body(
        params,
        &mx_hash_int_2_def(),
        &mx_gradient_float_1_def(),
        &mx_trilerp_0_def(),
        &mx_gradient_scale3d_0_def(),
    )
);

mx_fn!(
    mx_perlin_noise_vec3_1_def,
    "mx_perlin_noise_vec3_1",
    vec![("p", Type::Vec3)],
    Type::Vec3,
    |params| perlin_body(
        params,
        &mx_hash_vec3_1_def(),
        &mx_gradient_vec3_1_def(),
        &mx_trilerp_1_def(),
        &mx_gradient_scale3d_1_def(),
    )
);

// ---------------------------------------------------------------------------
// fractal noise
// ---------------------------------------------------------------------------

fn fractal_params() -> Vec<(&'static str, Type)> {
    vec![
        ("p", Type::Vec3),
        ("octaves", Type::I32),
        ("lacunarity", Type::F32),
        ("diminish", Type::F32),
    ]
}

/// `octaves` is copied into a var last, not third: its `VarNode` is only
/// reached when the `Loop`'s count is built, which is after `result` and
/// `amplitude`. The dumps show `nodeVar5 = octaves;` below `nodeVar4 = 1.0;`.
fn fractal_body(params: &[NodeRef], zero: NodeRef, noise: &Rc<FnDef>) -> NodeRef {
    let p = to_var(None, params[0].clone());
    let octaves = to_var(None, params[1].clone());
    let lacunarity = to_var(None, params[2].clone());
    let diminish = to_var(None, params[3].clone());
    let result = to_var(None, zero);
    let amplitude = to_var(None, float(1.0));

    let body = loop_n("i", octaves.clone(), |_| {
        vec![
            result.assign(result.add(amplitude.mul(call(noise, vec![p.clone()])))),
            amplitude.assign(amplitude.mul(&diminish)),
            p.assign(p.mul(&lacunarity)),
        ]
    });

    block(
        vec![
            diminish,
            lacunarity,
            p,
            result.clone(),
            amplitude,
            octaves,
            body,
        ],
        result,
    )
}

mx_fn!(
    mx_fractal_noise_float_def,
    "mx_fractal_noise_float",
    fractal_params(),
    Type::F32,
    |params| fractal_body(params, float(0.0), &mx_perlin_noise_float_1_def())
);

mx_fn!(
    mx_fractal_noise_vec3_def,
    "mx_fractal_noise_vec3",
    fractal_params(),
    Type::Vec3,
    |params| fractal_body(params, vec3(0.0, 0.0, 0.0), &mx_perlin_noise_vec3_1_def())
);

// ---------------------------------------------------------------------------
// `MaterialXNodes.js` — the public wrappers
// ---------------------------------------------------------------------------

/// `mx_fractal_noise_float( position, octaves = 3, lacunarity = 2, diminish = .5, amplitude = 1 )`
/// — `fractal_noise_float( position, int( octaves ), lacunarity, diminish ).mul( amplitude )`.
/// The amplitude multiply is always emitted, even at `1.0`.
pub fn mx_fractal_noise_float(
    position: NodeRef,
    octaves: i32,
    lacunarity: f64,
    diminish: f64,
    amplitude: f64,
) -> NodeRef {
    call(
        &mx_fractal_noise_float_def(),
        vec![
            position,
            int(octaves as i64),
            float(lacunarity),
            float(diminish),
        ],
    )
    .mul(float(amplitude))
}

/// `mx_fractal_noise_vec3( position, octaves = 3, lacunarity = 2, diminish = .5, amplitude = 1 )`.
pub fn mx_fractal_noise_vec3(
    position: NodeRef,
    octaves: i32,
    lacunarity: f64,
    diminish: f64,
    amplitude: f64,
) -> NodeRef {
    call(
        &mx_fractal_noise_vec3_def(),
        vec![
            position,
            int(octaves as i64),
            float(lacunarity),
            float(diminish),
        ],
    )
    .mul(float(amplitude))
}
