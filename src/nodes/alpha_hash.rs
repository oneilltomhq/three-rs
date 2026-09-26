//! Port of `three.js/src/nodes/functions/material/getAlphaHashThreshold.js`
//! (Wyman & McGuire 2017, "Hashed Alpha Testing"), which
//! `NodeMaterial.setupDiffuseColor()` discards against when
//! `material.alphaHash` is set.

use std::rc::Rc;

use crate::nodes::node::Type;
use crate::nodes::node::{FnDef, Lazy, NodeRef};
use crate::nodes::tsl::{
    call, ceil, dpdx, dpdy, exp2, float, floor, fract, inline_fn, length, log2, max, shader_fn,
    to_var, vec2_join, vec3_join,
};

/// `ALPHA_HASH_SCALE` — "Derived from trials only, and may be changed."
const ALPHA_HASH_SCALE: f64 = 0.05;

/// `hash2D` — an `Fn()` with no layout, so it inlines at each call site.
fn hash2d(value: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            inline_fn(1, Type::F32, |args| {
                let value = args[0].clone();
                // `fract( mul( 1.0e4, sin( mul( 17.0, value.x ).add( mul( 0.1,
                // value.y ) ) ) ).mul( add( 0.1, abs( sin( mul( 13.0, value.y
                // ).add( value.x ) ) ) ) ) )`
                float(1.0e4)
                    .mul(
                        float(17.0)
                            .mul(value.x())
                            .add(float(0.1).mul(value.y()))
                            .sin(),
                    )
                    .mul(float(0.1).add(float(13.0).mul(value.y()).add(value.x()).sin().abs()))
                    .fract()
            })
        })
    });
    call(&def, vec![value])
}

/// `hash3D` — `hash2D( vec2( hash2D( value.xy ), value.z ) )`, inlined.
fn hash3d(value: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            inline_fn(1, Type::F32, |args| {
                let value = args[0].clone();
                hash2d(vec2_join(vec![hash2d(value.xy()), value.z()]))
            })
        })
    });
    call(&def, vec![value])
}

/// `getAlphaHashThreshold( position )` — declared with `setLayout( { name:
/// 'getAlphaHashThreshold', type: 'float', inputs: [ position: vec3 ] } )`,
/// so it is a real WGSL `fn` and `hash2D` / `hash3D` inline into its body.
pub fn get_alpha_hash_threshold(position: NodeRef) -> NodeRef {
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|c| {
        c.get(|| {
            shader_fn(
                Some("getAlphaHashThreshold"),
                vec![("position", Type::Vec3)],
                Type::F32,
                |args| {
                    let position = args[0].clone();

                    // Find the discretized derivatives of our coordinates
                    let max_deriv = max(length(dpdx(position.xyz())), length(dpdy(position.xyz())));

                    let pix_scale = to_var(
                        Some("pixScale"),
                        float(1.0).div(float(ALPHA_HASH_SCALE).mul(max_deriv)),
                    );

                    // Find two nearest log-discretized noise scales
                    let pix_scales = vec2_join(vec![
                        exp2(floor(log2(pix_scale.clone()))),
                        exp2(ceil(log2(pix_scale.clone()))),
                    ]);

                    // Compute alpha thresholds at our two noise scales
                    let alpha = vec2_join(vec![
                        hash3d(floor(pix_scales.x().mul(position.xyz()))),
                        hash3d(floor(pix_scales.y().mul(position.xyz()))),
                    ]);

                    // Factor to interpolate lerp with
                    let lerp_factor = fract(log2(pix_scale));

                    // Interpolate alpha threshold from noise at two scales
                    let x = lerp_factor
                        .one_minus()
                        .mul(alpha.x())
                        .add(lerp_factor.mul(alpha.y()));

                    // Pass into CDF to compute uniformly distrib threshold
                    let a = lerp_factor.min(lerp_factor.one_minus());
                    let one = || float(1.0);
                    let cases = vec3_join(vec![
                        x.mul(x.clone())
                            .div(float(2.0).mul(a.clone()).mul(one().sub(a.clone()))),
                        x.sub(float(0.5).mul(a.clone())).div(one().sub(a.clone())),
                        one().sub(
                            one()
                                .sub(x.clone())
                                .mul(one().sub(x.clone()))
                                .div(float(2.0).mul(a.clone()).mul(one().sub(a.clone()))),
                        ),
                    ]);

                    // Find our final, uniformly distributed alpha threshold (ατ)
                    let threshold = x.less_than(a.one_minus()).select(
                        x.less_than(a.clone()).select(cases.x(), cases.y()),
                        cases.z(),
                    );

                    // Avoids ατ == 0. Could also do ατ =1-ατ
                    threshold.clamp(float(1.0e-6), float(1.0))
                },
            )
        })
    });
    call(&def, vec![position])
}
