//! Port of `examples/jsm/tsl/display/depthAwareBlur.js`: one pass of a
//! separable, depth-aware (bilateral) blur of a screen-space signal's red
//! channel — `SSAONode`'s blur. Gated through SSAONode's blur quad
//! (`ssao_blur_matches_three` in `tests/nodes_display_wgsl.rs`, `docs/nodes.md`
//! §93).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::{
    abs, block, exp, float, int, loop_options, max, pass_depth_texture_uv,
    perspective_depth_to_view_z, to_const, to_var, uniform, uv,
};
use crate::cameras::PerspectiveCamera;
use crate::nodes::node::{LiveValue, NodeRef, Type, UniformGroup, UniformSource};
use crate::textures::DepthTexture;

/// `reference( name, 'float', camera )` — the camera's value at draw time.
fn camera_float_reference(
    camera: &Rc<RefCell<PerspectiveCamera>>,
    read: fn(&PerspectiveCamera) -> f64,
) -> NodeRef {
    let camera: Weak<RefCell<PerspectiveCamera>> = Rc::downgrade(camera);
    uniform(
        UniformSource::Live(LiveValue::new(move || {
            let camera = camera
                .upgrade()
                .expect("three-rs: a camera reference outlived its camera");
            let value = read(&camera.borrow());
            vec![value]
        })),
        Type::F32,
        UniformGroup::Object,
        None,
    )
}

/// `depthAwareBlur( inputNode, depthNode, directionNode, camera, sharpness,
/// radius )` — a `Fn()` without a layout, so it inlines.
///
/// Five taps at `uv() + direction * i`, `i` in `-2..=2`, each weighted by a
/// gaussian `exp( -i² / 2 )` times `exp( -|ΔviewZ| / radius * sharpness )`,
/// the view-Z difference from the centre pixel; returns the weighted mean of
/// `input( uv ).r`. Run it twice, horizontally then vertically.
///
/// `input` is `inputNode.sample`: the sampler of the signal to blur (for a
/// `texture( map )` node, [`texture_with_uv`](super::texture_with_uv) over
/// the map). `depth` is the scene's depth attachment, `direction` one texel
/// step along the blur axis (`vec2( 1 / width, 0 )`), `camera` the one the
/// scene is drawn with, and `sharpness` / `radius` three's defaults `2` /
/// `1` unless given. The port has no logarithmic depth buffer, so the depth
/// always goes through `perspectiveDepthToViewZ`.
pub fn depth_aware_blur(
    input: impl Fn(NodeRef) -> NodeRef,
    depth: &DepthTexture,
    direction: NodeRef,
    camera: &Rc<RefCell<PerspectiveCamera>>,
    sharpness: NodeRef,
    radius: NodeRef,
) -> NodeRef {
    let camera_near = camera_float_reference(camera, |c| c.near);
    let camera_far = camera_float_reference(camera, |c| c.far);

    let view_z = |uv_node: NodeRef| {
        let depth = pass_depth_texture_uv(depth, uv_node);
        perspective_depth_to_view_z(depth, camera_near.clone(), camera_far.clone())
    };

    let uv_node = uv();
    let center_view_z = to_var(None, view_z(uv_node.clone()));

    let sum = to_var(None, float(0.0));
    let weight_sum = to_var(None, float(0.0));

    let taps = loop_options("i", Type::I32, int(-2), int(3), "<", |i| {
        let fi = to_const(None, i.to_float());
        let sample_uv = to_const(None, uv_node.add(direction.mul(fi.clone())));

        let spatial_weight = exp(fi.mul(fi.clone()).mul(-0.5));
        let depth_weight = exp(abs(view_z(sample_uv.clone()).sub(center_view_z.clone()))
            .div(radius.clone())
            .mul(sharpness.clone())
            .negate());
        let weight = to_const(None, spatial_weight.mul(depth_weight));

        vec![
            fi,
            sample_uv.clone(),
            weight.clone(),
            sum.add_assign(input(sample_uv).x().mul(weight.clone())),
            weight_sum.add_assign(weight),
        ]
    });

    block(
        vec![center_view_z, sum.clone(), weight_sum.clone(), taps],
        sum.div(max(weight_sum, float(0.0001))),
    )
}
