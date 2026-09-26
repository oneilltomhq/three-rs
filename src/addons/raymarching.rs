//! Port of `three.js/examples/jsm/tsl/utils/Raymarching.js` —
//! `RaymarchingBox`, the unit-cube ray march the volume examples build their
//! `colorNode` on.
//!
//! The mesh is a unit box drawn `BackSide`; each fragment rebuilds the eye ray
//! in the box's local space, clips it against the box (`hitBox`), and walks it
//! front to back in `steps` equal strides. What happens at each step is the
//! caller's: `callback` gets the ray position and the stride and returns the
//! statements of the loop body.

use crate::nodes::tsl::{
    camera_position, discard, float, if_then, loop_float, max, min_of, model_world_matrix_inverse,
    position_geometry, to_const, to_var, to_varying, vec2_join, vec3s, vec4_join,
};
use crate::nodes::NodeRef;

/// `hitBox( { orig, dir } )` — the slab test against `[ -0.5, 0.5 ]³`,
/// returning `vec2( tNear, tFar )`. An inlined `Fn()`, so its temps land in
/// the caller's flow.
///
/// The `to_const`s are not in the JS. Three at 5f610f5 turns an expression
/// that is read more than once into a `let nodeConstN` on its own; this
/// port's builder still promotes such an expression to a `var<private>`
/// (`docs/nodes.md` §8, "Usage-promoted temps"), so the addon asks for the
/// `let` where three's dump has one and the two flows match line for line.
fn hit_box(orig: &NodeRef, dir: &NodeRef) -> NodeRef {
    let box_min = vec3s(-0.5);
    let box_max = vec3s(0.5);

    let inv_dir = to_const(None, dir.reciprocal());

    let tmin_tmp = to_const(None, box_min.sub(orig).mul(&inv_dir));
    let tmax_tmp = to_const(None, box_max.sub(orig).mul(&inv_dir));

    let tmin = to_const(None, min_of(&tmin_tmp, &tmax_tmp));
    let tmax = to_const(None, max(&tmin_tmp, &tmax_tmp));

    let t0 = max(tmin.x(), max(tmin.y(), tmin.z()));
    let t1 = min_of(tmax.x(), min_of(tmax.y(), tmax.z()));

    vec2_join(vec![t0, t1])
}

/// `RaymarchingBox( steps, callback )`, as the statements it adds to the
/// caller's flow. `steps` is a float node (`uniform( 200 )` on the volume
/// pages); `callback( positionRay, stepSize )` returns the loop body, which
/// runs before the ray advances.
pub fn raymarching_box(
    steps: &NodeRef,
    callback: impl FnOnce(&NodeRef, &NodeRef) -> Vec<NodeRef>,
) -> Vec<NodeRef> {
    // `varying( vec3( modelWorldMatrixInverse.mul( vec4( cameraPosition, 1.0 ) ) ) )`
    let v_origin = to_varying(
        None,
        model_world_matrix_inverse()
            .mul(vec4_join(vec![camera_position(), float(1.0)]))
            .xyz(),
    );
    // `varying( positionGeometry.sub( vOrigin ) )`
    let v_direction = to_varying(None, position_geometry().sub(&v_origin));

    let ray_dir = to_const(None, v_direction.normalize());
    let bounds = to_var(None, hit_box(&v_origin, &ray_dir));

    let mut out = vec![bounds.clone()];

    // `bounds.x.greaterThan( bounds.y ).discard()`
    out.push(if_then(
        bounds.x().greater_than(bounds.y()),
        vec![discard()],
    ));

    out.push(bounds.assign(vec2_join(vec![max(bounds.x(), float(0.0)), bounds.y()])));

    let inc = to_var(None, ray_dir.abs().reciprocal());
    out.push(inc.clone());
    let step_size = to_var(None, min_of(inc.x(), min_of(inc.y(), inc.z())));
    out.push(step_size.clone());

    out.push(step_size.div_assign(steps.clone()));

    let position_ray = to_var(None, v_origin.add(bounds.x().mul(&ray_dir)));
    out.push(position_ray.clone());

    let step = step_size.clone();
    out.push(loop_float("i", bounds.x(), bounds.y(), step_size, |_| {
        let mut body = callback(&position_ray, &step);
        body.push(position_ray.add_assign(ray_dir.mul(&step)));
        body
    }));

    out
}
