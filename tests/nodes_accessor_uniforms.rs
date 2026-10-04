//! The bytes behind the accessor batch's new uniform sources (`docs/nodes.md`
//! §67.3), against hand-computed values. No GPU: each test builds a program,
//! finds the member the accessor declared and writes it through
//! [`UniformContext::bytes`], as the renderer does before a draw.
//!
//! The drawn object's world matrix is `T( 1, 2, 3 ) · Ry( 90° ) · S( 2, 3, 4
//! )`, a non-uniform scale under a rotation, and the camera sits off the
//! origin at `( 4, 0, 10 )`. Column-major:
//!
//! ```text
//! world = | 0 0 4 1 |      view = | 1 0 0 -4  |
//!         | 0 3 0 2 |             | 0 1 0  0  |
//!         |-2 0 0 3 |             | 0 0 1 -10 |
//!         | 0 0 0 1 |             | 0 0 0  1  |
//! ```

use std::f64::consts::FRAC_PI_2;
use std::rc::Rc;

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::{Color, Matrix4, Vector3};
use three_rs::nodes::tsl::*;
use three_rs::nodes::{BindingDesc, NodeBuilder, NodeRef, Object3DScope, Type, UniformSource};
use three_rs::renderer::UniformContext;
use three_rs::{Mesh, Object3D, PerspectiveCamera};

const EPSILON: f32 = 1e-5;

fn matrix(elements: [f64; 16]) -> Matrix4 {
    let mut m = Matrix4::identity();
    m.elements = elements;
    m
}

/// `T( 1, 2, 3 ) · Ry( 90° ) · S( 2, 3, 4 )`.
fn model_world() -> Matrix4 {
    matrix([
        0.0, 0.0, -2.0, 0.0, //
        0.0, 3.0, 0.0, 0.0, //
        4.0, 0.0, 0.0, 0.0, //
        1.0, 2.0, 3.0, 1.0,
    ])
}

/// The view matrix of a camera at `( 4, 0, 10 )` looking down `-z`.
fn camera_view() -> Matrix4 {
    matrix([
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        -4.0, 0.0, -10.0, 1.0,
    ])
}

/// A unit box, whose bounding sphere has radius `√3 / 2`.
fn drawn_mesh() -> three_rs::Node {
    Mesh::new(
        Rc::new(three_rs::box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        None,
    )
}

/// The floats `node`'s first uniform matching `source` holds, written with
/// `ctx`. `node` is the material's `fragmentNode`, cast to a `vec4`'s worth
/// so any accessor fits.
fn uniform_value(
    node: NodeRef,
    ctx: &UniformContext,
    source: impl Fn(&UniformSource) -> bool,
) -> Vec<f32> {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(node);
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    for binding in program.groups.iter().flatten() {
        let BindingDesc::Uniforms { members, size, .. } = binding else {
            continue;
        };
        let Some(member) = members.iter().find(|m| source(&m.source)) else {
            continue;
        };
        let floats = match member.ty {
            Type::F32 => 1,
            Type::Vec3 => 3,
            Type::Mat3 => 12,
            Type::Mat4 => 16,
            ty => panic!("unexpected member type {ty:?}"),
        };
        let bytes = ctx.bytes(members, *size);
        let start = member.offset as usize;
        return bytes[start..start + 4 * floats]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| f32::from_le_bytes(c))
            .collect();
    }
    panic!("no uniform with that source")
}

fn assert_close(got: &[f32], want: &[f32], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: {got:?} vs {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!(
            (g - w).abs() < EPSILON,
            "{what}: got {got:?}, want {want:?}"
        );
    }
}

/// `vec4( v, 1 )` for a `vec3`, or the scalar splatted.
fn as_output(node: NodeRef) -> NodeRef {
    match node.ty() {
        Type::Vec3 => vec4_join(vec![node, float(1.0)]),
        _ => vec4_join(vec![node.clone(), node.clone(), node.clone(), node]),
    }
}

fn object_scope(scope: Object3DScope) -> impl Fn(&UniformSource) -> bool {
    move |source| matches!(source, UniformSource::Object3D { scope: s, .. } if *s == scope)
}

#[test]
fn model_scopes_of_the_drawn_object() {
    let mesh = drawn_mesh();
    let object = mesh.borrow();
    let ctx = UniformContext {
        object: Some(&*object),
        model_world: model_world(),
        camera_view: camera_view(),
        ..UniformContext::default()
    };
    let cases: [(NodeRef, Object3DScope, &[f32], &str); 5] = [
        // The translation column.
        (
            model_position(),
            Object3DScope::Position,
            &[1.0, 2.0, 3.0],
            "modelPosition",
        ),
        // The column lengths: the non-uniform scale, through the rotation.
        (
            model_scale(),
            Object3DScope::Scale,
            &[2.0, 3.0, 4.0],
            "modelScale",
        ),
        // The third column `( 4, 0, 0 )`, normalised.
        (
            model_direction(),
            Object3DScope::Direction,
            &[1.0, 0.0, 0.0],
            "modelDirection",
        ),
        // `( 1, 2, 3 )` through the view matrix.
        (
            model_view_position(),
            Object3DScope::ViewPosition,
            &[-3.0, 2.0, -7.0],
            "modelViewPosition",
        ),
        // `√3 / 2` times the largest axis scale, 4.
        (
            model_radius(),
            Object3DScope::Radius,
            &[2.0 * 3f32.sqrt()],
            "modelRadius",
        ),
    ];
    for (node, scope, want, what) in cases {
        let got = uniform_value(as_output(node), &ctx, object_scope(scope));
        assert_close(&got, want, what);
    }
}

#[test]
fn model_direction_of_a_drawn_camera_is_negated() {
    // `Camera.getWorldDirection()` negates `Object3D`'s.
    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
    let object = camera.node.borrow();
    let ctx = UniformContext {
        object: Some(&*object),
        model_world: model_world(),
        ..UniformContext::default()
    };
    let got = uniform_value(
        as_output(model_direction()),
        &ctx,
        object_scope(Object3DScope::Direction),
    );
    assert_close(&got, &[-1.0, 0.0, 0.0], "modelDirection of a camera");
}

/// `T( 1, 2, 3 ) · Ry( 90° ) · S( 2, 3, 4 )` as a scene object, its world
/// matrix not yet computed.
fn target() -> three_rs::Node {
    let target = Object3D::new_node();
    {
        let mut object = target.borrow_mut();
        object.position.set(1.0, 2.0, 3.0);
        object.scale.set(2.0, 3.0, 4.0);
        object.set_rotation_from_axis_angle(&Vector3::new(0.0, 1.0, 0.0), FRAC_PI_2);
    }
    target
}

#[test]
fn object_scopes_of_an_explicit_object() {
    let target = target();
    target.update_world_matrix(true, false);
    let mesh = drawn_mesh();
    let object = mesh.borrow();
    // The drawn object's own matrix must not leak into the target's values.
    let ctx = UniformContext {
        object: Some(&*object),
        model_world: Matrix4::identity(),
        camera_view: camera_view(),
        ..UniformContext::default()
    };
    let cases: [(NodeRef, Object3DScope, &[f32], &str); 4] = [
        (
            object_position(&target),
            Object3DScope::Position,
            &[1.0, 2.0, 3.0],
            "objectPosition",
        ),
        (
            object_scale(&target),
            Object3DScope::Scale,
            &[2.0, 3.0, 4.0],
            "objectScale",
        ),
        (
            object_view_position(&target),
            Object3DScope::ViewPosition,
            &[-3.0, 2.0, -7.0],
            "objectViewPosition",
        ),
        // The *drawn* object's bounding sphere, the target's largest scale.
        (
            object_radius(&target),
            Object3DScope::Radius,
            &[2.0 * 3f32.sqrt()],
            "objectRadius",
        ),
    ];
    for (node, scope, want, what) in cases {
        let got = uniform_value(as_output(node), &ctx, object_scope(scope));
        assert_close(&got, want, what);
    }
}

#[test]
fn object_direction_refreshes_the_world_matrix() {
    // Never updated: `getWorldDirection()` runs `updateWorldMatrix( true,
    // false )` first, so the rotation is seen.
    let target = target();
    let got = uniform_value(
        as_output(object_direction(&target)),
        &UniformContext::default(),
        object_scope(Object3DScope::Direction),
    );
    assert_close(&got, &[1.0, 0.0, 0.0], "objectDirection");
}

#[test]
fn object_direction_of_a_camera_is_negated() {
    // A camera under a parent turned 90° about y, neither updated: the
    // parent's rotation is refreshed in, and the camera looks down its -z,
    // `Ry( 90° ) · ( 0, 0, -1 ) = ( -1, 0, 0 )`.
    let parent = Object3D::new_node();
    parent
        .borrow_mut()
        .set_rotation_from_axis_angle(&Vector3::new(0.0, 1.0, 0.0), FRAC_PI_2);
    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
    parent.add(&camera.node);
    let got = uniform_value(
        as_output(object_direction(&camera.node)),
        &UniformContext::default(),
        object_scope(Object3DScope::Direction),
    );
    assert_close(&got, &[-1.0, 0.0, 0.0], "objectDirection of a camera");
}

#[test]
fn highp_model_view_matrices() {
    let ctx = UniformContext {
        model_world: model_world(),
        camera_view: camera_view(),
        ..UniformContext::default()
    };
    // `view · world`: the world matrix with its translation moved by
    // `( -4, 0, -10 )`.
    let got = uniform_value(
        highp_model_view_matrix().mul(vec4(0.0, 0.0, 0.0, 1.0)),
        &ctx,
        |source| matches!(source, UniformSource::HighpModelViewMatrix),
    );
    assert_close(
        &got,
        &[
            0.0, 0.0, -2.0, 0.0, //
            0.0, 3.0, 0.0, 0.0, //
            4.0, 0.0, 0.0, 0.0, //
            -3.0, 2.0, -7.0, 1.0,
        ],
        "highpModelViewMatrix",
    );
    // The inverse transpose of `Ry( 90° ) · S( 2, 3, 4 )` is `Ry( 90° ) ·
    // S( 1/2, 1/3, 1/4 )`; a translation-only view leaves it alone. Columns
    // padded to four floats.
    let got = uniform_value(
        vec4_join(vec![
            highp_model_normal_view_matrix().mul(vec3(0.0, 0.0, 1.0)),
            float(1.0),
        ]),
        &ctx,
        |source| matches!(source, UniformSource::HighpModelNormalViewMatrix),
    );
    assert_close(
        &got,
        &[
            0.0,
            0.0,
            -0.5,
            0.0, //
            0.0,
            1.0 / 3.0,
            0.0,
            0.0, //
            0.25,
            0.0,
            0.0,
            0.0,
        ],
        "highpModelNormalViewMatrix",
    );
}

#[test]
fn camera_normal_matrix_is_the_identity() {
    // `WebGPURenderer` never sets `camera.normalMatrix`; whatever the camera.
    let ctx = UniformContext {
        camera_view: camera_view(),
        camera_world: matrix([
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            4.0, 0.0, 10.0, 1.0,
        ]),
        ..UniformContext::default()
    };
    let got = uniform_value(
        vec4_join(vec![
            camera_normal_matrix().mul(vec3(0.0, 0.0, 1.0)),
            float(1.0),
        ]),
        &ctx,
        |source| matches!(source, UniformSource::CameraNormalMatrix),
    );
    assert_close(
        &got,
        &[
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ],
        "cameraNormalMatrix",
    );
}

#[test]
fn material_refraction_ratio_is_the_material_s() {
    let ctx = UniformContext {
        material_refraction_ratio: 0.75,
        ..UniformContext::default()
    };
    let got = uniform_value(as_output(material_refraction_ratio()), &ctx, |source| {
        matches!(source, UniformSource::MaterialRefractionRatio)
    });
    assert_close(&got, &[0.75], "materialRefractionRatio");

    // 0.98 where three's material has `refractionRatio`, 0 elsewhere.
    let white = Color::new(1.0, 1.0, 1.0);
    for (material, want, kind) in [
        (MeshBasicNodeMaterial::new(), 0.98, "basic"),
        (MeshBasicNodeMaterial::lambert(white), 0.98, "lambert"),
        (MeshBasicNodeMaterial::phong(white), 0.98, "phong"),
        (
            MeshBasicNodeMaterial::standard(white, 1.0, 0.0),
            0.0,
            "standard",
        ),
        (
            MeshBasicNodeMaterial::physical(white, 1.0, 0.0),
            0.0,
            "physical",
        ),
        (MeshBasicNodeMaterial::toon(white, None), 0.0, "toon"),
        (MeshBasicNodeMaterial::normal(), 0.0, "normal"),
        (MeshBasicNodeMaterial::sprite(), 0.0, "sprite"),
        (MeshBasicNodeMaterial::points(), 0.0, "points"),
        (MeshBasicNodeMaterial::line(white), 0.0, "line"),
        (MeshBasicNodeMaterial::line2(white), 0.0, "line2"),
    ] {
        assert_eq!(material.refraction_ratio, want, "{kind}");
    }
}
