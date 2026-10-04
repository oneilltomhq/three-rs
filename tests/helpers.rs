//! Ports of `test/unit/src/helpers/*.tests.js` for the helpers in
//! `three_rs::helpers` (issue #300).
//!
//! Three's "Extending" tests become checks on the node's payload (a
//! `LineSegments`, `Line` or `Mesh` is a payload kind here, not a subclass),
//! "Instancing" is the construction itself, and "type" is
//! `Object3D::object_type`. The `dispose` tests have nothing to port: the port
//! frees geometry and materials on drop and has no `dispose`. `BoxHelper`'s
//! "Standard geometry tests" (`runStdGeometryTests`: `clone()` and a JSON
//! round trip) need `BufferGeometry.clone()` and `toJSON()`, which the port
//! lacks; the box and sphere helpers its `beforeEach` builds are checked for
//! their corners instead. `CameraHelper` and `GridHelper` were ported before
//! this file and are covered here too.
//!
//! `tests/helpers_core.rs` compares the helpers' geometry and transforms
//! with three's own output.

use std::rc::Rc;

use three_rs::core::Node;
use three_rs::geometries::{box_geometry_default, sphere_geometry};
use three_rs::helpers::{
    ArrowHelper, AxesHelper, Box3Helper, BoxHelper, CameraHelper, DirectionalLightHelper,
    GridHelper, HemisphereLightHelper, PlaneHelper, PointLightHelper, PolarGridHelper,
    SkeletonHelper, SpotLightHelper,
};
use three_rs::lights::{DirectionalLight, HemisphereLight, PointLight, SpotLight};
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::{Box3, Color, Plane};
use three_rs::objects::{Bone, Mesh, Payload};
use three_rs::PerspectiveCamera;

fn hex(hex: u32) -> Color {
    Color::from_hex(hex)
}

fn object_type(node: &Node) -> &'static str {
    node.borrow().object_type
}

fn is_line_segments(node: &Node) -> bool {
    node.borrow().is_line_segments()
}

/// `object instanceof Line` but not `LineSegments`.
fn is_plain_line(node: &Node) -> bool {
    let object = node.borrow();
    object.is_line() && !object.is_line_segments()
}

/// A plain `Object3D`: nothing to draw of its own.
fn is_plain_object3d(node: &Node) -> bool {
    matches!(node.borrow().payload, Payload::None)
}

#[test]
fn arrow_helper() {
    let object = ArrowHelper::default();
    assert!(
        is_plain_object3d(&object.node),
        "ArrowHelper extends from Object3D"
    );
    assert_eq!(object_type(&object.node), "ArrowHelper");
    assert!(is_plain_line(&object.line));
    assert!(object.cone.borrow().is_mesh());
}

#[test]
fn axes_helper() {
    let object = AxesHelper::default();
    assert!(
        is_line_segments(&object.node),
        "AxesHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object.node), "AxesHelper");
}

#[test]
fn box3_helper() {
    let object = Box3Helper::new(Box3::default(), hex(0xffff00));
    assert!(
        is_line_segments(&object.node),
        "Box3Helper extends from LineSegments"
    );
    assert_eq!(object_type(&object.node), "Box3Helper");
}

#[test]
fn box_helper() {
    let object = BoxHelper::default();
    assert!(
        is_line_segments(&object.node),
        "BoxHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object.node), "BoxHelper");
}

/// The helpers `BoxHelper.tests.js`' `beforeEach` builds, over a default
/// `BoxGeometry` and a default `SphereGeometry`: eight corners, each at
/// ±0.5 and ±1 respectively.
#[test]
fn box_helper_box_and_sphere() {
    for (geometry, half) in [
        (box_geometry_default(), 0.5_f32),
        (sphere_geometry(1.0, 32, 16), 1.0),
    ] {
        let mesh = Mesh::new(Rc::new(geometry), MeshBasicNodeMaterial::new());
        let helper = BoxHelper::new(Some(mesh), hex(0xffff00));

        let node = helper.node.borrow();
        let geometry = node.geometry().unwrap();
        assert_eq!(geometry.index.as_ref().unwrap().count(), 24);
        let position = geometry.get_attribute("position").unwrap();
        assert_eq!(position.count(), 8);
        for &value in position.array().iter() {
            assert!(
                (value.abs() - half).abs() < 1e-6,
                "corner coordinate {value}"
            );
        }
    }
}

#[test]
fn camera_helper() {
    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 2000.0);
    let object = CameraHelper::new(&camera);
    assert!(
        is_line_segments(&object.node),
        "CameraHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object.node), "CameraHelper");
}

#[test]
fn directional_light_helper() {
    let light = DirectionalLight::new(hex(0xaaaaaa), 1.0);
    let object = DirectionalLightHelper::new(&light, 1.0, Some(hex(0xaaaaaa)));
    assert!(
        is_plain_object3d(&object.node),
        "DirectionalLightHelper extends from Object3D"
    );
    assert_eq!(object_type(&object.node), "DirectionalLightHelper");
}

#[test]
fn grid_helper() {
    let object = GridHelper::new(10.0, 10, hex(0x444444), hex(0x888888));
    assert!(
        is_line_segments(&object),
        "GridHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object), "GridHelper");
}

#[test]
fn hemisphere_light_helper() {
    let light = HemisphereLight::new(hex(0x123456), hex(0xffffff), 1.0);
    let object = HemisphereLightHelper::new(&light, 1.0, Some(hex(0xabc012)));
    assert!(
        is_plain_object3d(&object.node),
        "HemisphereLightHelper extends from Object3D"
    );
    assert_eq!(object_type(&object.node), "HemisphereLightHelper");
}

#[test]
fn plane_helper() {
    let object = PlaneHelper::new(Plane::default(), 1.0, hex(0xffff00));
    assert!(is_plain_line(&object.node), "PlaneHelper extends from Line");
    assert_eq!(object_type(&object.node), "PlaneHelper");
}

#[test]
fn point_light_helper() {
    let light = PointLight::new(hex(0xaaaaaa), 1.0, 0.0);
    let object = PointLightHelper::new(&light, 1.0, Some(hex(0xaaaaaa)));
    assert!(
        object.node.borrow().is_mesh(),
        "PointLightHelper extends from Mesh"
    );
    assert_eq!(object_type(&object.node), "PointLightHelper");
}

#[test]
fn polar_grid_helper() {
    let object = PolarGridHelper::new(10.0, 16, 8, 64, hex(0x444444), hex(0x888888));
    assert!(
        is_line_segments(&object),
        "PolarGridHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object), "PolarGridHelper");
}

#[test]
fn skeleton_helper() {
    let bone = Bone::new();
    let object = SkeletonHelper::new(&bone);
    assert!(
        is_line_segments(&object.node),
        "SkeletonHelper extends from LineSegments"
    );
    assert_eq!(object_type(&object.node), "SkeletonHelper");
    const { assert!(SkeletonHelper::IS_SKELETON_HELPER) };
    assert_eq!(object.bones.len(), 1, "a lone bone is its own bone list");
}

#[test]
fn spot_light_helper() {
    let light = SpotLight::new(hex(0xaaaaaa), 1.0);
    let object = SpotLightHelper::new(&light, Some(hex(0xaaaaaa)));
    assert!(
        is_plain_object3d(&object.node),
        "SpotLightHelper extends from Object3D"
    );
    assert_eq!(object_type(&object.node), "SpotLightHelper");
}
