//! `webgpu_tsl_interoperability`'s two library pieces, without a GPU.
//!
//! A `wgslFn` is copied through verbatim, so the node system never sees what
//! the body does. Both of these are about the parts of the shader the node
//! system still has to get right *around* that body: the varying the body
//! assigns to has to be declared, and the value the body returns has to be
//! widened to the `vec4` the entry point writes. `docs/nodes.md` §19.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::nodes::tsl::{attribute, call_wgsl, varying_property, wgsl_fn};
use three_rs::nodes::{NodeBuilder, Type};

const CRT_VERTEX: &str = "fn crtVertex( position: vec3f, uv: vec2f ) -> vec3<f32> {
	varyings.vUv = uv;
	return position;
}";

const RETURNS_VEC3: &str = "fn crtFragment( vUv: vec2f ) -> vec3<f32> {
	return vec3<f32>( vUv, 0.0 );
}";

/// `wgslFn( source, [ vUv ] )`: the include is the only thing that tells the
/// builder the varying exists, because the assignment to it is inside a string.
#[test]
fn a_varying_include_declares_the_varying() {
    let v_uv = varying_property("vUv", Type::Vec2, false);
    let vertex = wgsl_fn(CRT_VERTEX, vec![v_uv.clone()]);

    let mut material = MeshBasicNodeMaterial::new();
    material.position_node = Some(call_wgsl(
        &vertex,
        vec![
            ("position", attribute("position", Type::Vec3)),
            ("uv", attribute("uv", Type::Vec2)),
        ],
    ));

    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);

    assert!(
        program
            .vertex_wgsl
            .contains("@location( 0 ) vUv : vec2<f32>"),
        "the include must put vUv in VaryingsStruct:\n{}",
        program.vertex_wgsl
    );
    // And the body itself is copied through, tabs and all, so the assignment
    // the declaration is for is still there.
    assert!(program.vertex_wgsl.contains("varyings.vUv = uv;"));
}

/// Without the include nothing declares it, and the hand-written body would
/// reference a struct member that does not exist. Three behaves the same way —
/// this pins that the include is load-bearing rather than decorative.
#[test]
fn without_the_include_the_varying_is_not_declared() {
    let vertex = wgsl_fn(CRT_VERTEX, vec![]);

    let mut material = MeshBasicNodeMaterial::new();
    material.position_node = Some(call_wgsl(
        &vertex,
        vec![
            ("position", attribute("position", Type::Vec3)),
            ("uv", attribute("uv", Type::Vec2)),
        ],
    ));

    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);

    assert!(!program.vertex_wgsl.contains("vUv : vec2<f32>"));
}

/// `fragmentNode` short-circuits the whole material, so whatever it returns is
/// what `output.color` gets — and `output.color` is a `vec4`.
#[test]
fn a_vec3_fragment_node_is_widened_to_vec4() {
    let v_uv = varying_property("vUv", Type::Vec2, false);
    let fragment = wgsl_fn(RETURNS_VEC3, vec![]);

    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(call_wgsl(&fragment, vec![("vUv", v_uv)]));

    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);

    assert!(
        program
            .fragment_wgsl
            .contains("output.color = vec4<f32>( crtFragment( vUv ), 1.0 );"),
        "{}",
        program.fragment_wgsl
    );
}
