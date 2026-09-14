//! Port of `three.js/test/unit/src/core/Object3D.tests.js`.
//!
//! The Rust `Object3D` has no parent/children yet (the scene graph is flat in
//! the ladder so far), so every test that builds a hierarchy is skipped:
//! `add/remove/removeFromParent/clear`, `attach`, `getObjectById/ByName/
//! ByProperty`, `getObjectsByProperty`, `traverse*`, `updateMatrixWorld` and
//! `updateWorldMatrix` (both are parent/child matrices), and the parent halves
//! of `getWorldPosition`, `localToWorld` and `worldToLocal`. Also skipped:
//! `Extending`, `Instancing`, `type`, `isObject3D`, `DEFAULT_MATRIX_AUTO_UPDATE`
//! (no global defaults or auto-update flag), `toJSON`, `clone`, `copy`, and
//! `localTransformVariableInstantiation` (a JS-only aliasing check).

mod support;

use support::{close, EPS, X, Y, Z};
use three_rs::core::Object3D;
use three_rs::math::{Euler, Matrix4, Quaternion, Vector3};

const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

fn euler_equals(a: &Euler, b: &Euler, tolerance: f64) -> bool {
    (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs() < tolerance
}

/// QUnit's `assert.numEqual`: tolerance 0.1.
#[track_caller]
fn num_equal(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 0.1, "{what}: {a} vs {b}");
}

#[test]
fn default_up() {
    let object = Object3D::default();
    assert_eq!(object.up, Vector3::new(0.0, 1.0, 0.0), "Y-up");
}

#[test]
fn apply_matrix4() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    let expected_pos = Vector3::new(X, Y, Z);
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let expected_quat = Quaternion::new(sqrt, 0.0, 0.0, sqrt);

    m.make_rotation_x(std::f64::consts::PI / 2.0);
    m.set_position(X, Y, Z);

    a.apply_matrix4(&m);

    assert_eq!(a.position, expected_pos, "Position has the expected values");
    assert!(
        (a.quaternion.x - expected_quat.x).abs() <= EPS
            && (a.quaternion.y - expected_quat.y).abs() <= EPS
            && (a.quaternion.z - expected_quat.z).abs() <= EPS,
        "Quaternion has the expected values"
    );
}

#[test]
fn apply_quaternion() {
    let mut a = Object3D::default();
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let quat = Quaternion::new(0.0, sqrt, 0.0, sqrt);
    let expected = Quaternion::new(sqrt / 2.0, sqrt / 2.0, 0.0, 0.0);

    a.quaternion.set(0.25, 0.25, 0.25, 0.25);
    a.apply_quaternion(&quat);

    assert!(
        (a.quaternion.x - expected.x).abs() <= EPS
            && (a.quaternion.y - expected.y).abs() <= EPS
            && (a.quaternion.z - expected.z).abs() <= EPS,
        "Quaternion has the expected values"
    );
}

#[test]
fn set_rotation_from_axis_angle() {
    let mut a = Object3D::default();
    let mut axis = Vector3::new(0.0, 1.0, 0.0);
    let pi = std::f64::consts::PI;
    let mut expected = Euler::new(-pi, 0.0, -pi);
    let mut euler = Euler::default();

    a.set_rotation_from_axis_angle(&axis, pi);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after rotation"
    );

    axis.set(1.0, 0.0, 0.0);
    expected.set(0.0, 0.0, 0.0);

    a.set_rotation_from_axis_angle(&axis, 0.0);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after zeroing"
    );
}

#[test]
fn set_rotation_from_euler() {
    let mut a = Object3D::default();
    let rotation = Euler::new(45.0 / RAD_TO_DEG, 0.0, std::f64::consts::PI);
    let expected = rotation;
    let mut euler = Euler::default();

    a.set_rotation_from_euler(&rotation);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &expected, EPS),
        "Correct values after rotation"
    );
}

#[test]
fn set_rotation_from_matrix() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    let eye = Vector3::new(0.0, 0.0, 0.0);
    let target = Vector3::new(0.0, 1.0, -1.0);
    let up = Vector3::new(0.0, 1.0, 0.0);
    let mut euler = Euler::default();

    m.look_at(&eye, &target, &up);
    a.set_rotation_from_matrix(&m);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    num_equal(euler.x * RAD_TO_DEG, 45.0, "Correct rotation angle");
}

#[test]
fn set_rotation_from_quaternion() {
    let mut a = Object3D::default();
    let pi = std::f64::consts::PI;
    let mut rotation = Quaternion::default();
    rotation.set_from_euler(&Euler::new(pi, 0.0, -pi));
    let mut euler = Euler::default();

    a.set_rotation_from_quaternion(&rotation);
    let q = a.get_world_quaternion();
    euler.set_from_quaternion(&q, euler.order);
    assert!(
        euler_equals(&euler, &Euler::new(pi, 0.0, -pi), EPS),
        "Correct values after rotation"
    );
}

#[test]
fn rotate_x() {
    let mut obj = Object3D::default();
    let angle = 1.562;
    obj.rotate_x(angle);
    num_equal(obj.rotation.x, angle, "x is equal");
}

#[test]
fn rotate_y() {
    let mut obj = Object3D::default();
    let angle = -0.346;
    obj.rotate_y(angle);
    num_equal(obj.rotation.y, angle, "y is equal");
}

#[test]
fn rotate_z() {
    let mut obj = Object3D::default();
    let angle = 1.0;
    obj.rotate_z(angle);
    num_equal(obj.rotation.z, angle, "z is equal");
}

#[test]
fn translate_on_axis() {
    let mut obj = Object3D::default();
    obj.translate_on_axis(&Vector3::new(1.0, 0.0, 0.0), 1.0);
    obj.translate_on_axis(&Vector3::new(0.0, 1.0, 0.0), 1.23);
    obj.translate_on_axis(&Vector3::new(0.0, 0.0, 1.0), -4.56);

    assert_eq!(obj.position, Vector3::new(1.0, 1.23, -4.56));
}

#[test]
fn translate_x_y_z() {
    let mut obj = Object3D::default();
    obj.translate_x(1.234);
    num_equal(obj.position.x, 1.234, "x is equal");

    let mut obj = Object3D::default();
    obj.translate_y(1.234);
    num_equal(obj.position.y, 1.234, "y is equal");

    let mut obj = Object3D::default();
    obj.translate_z(1.234);
    num_equal(obj.position.z, 1.234, "z is equal");
}

#[test]
fn local_to_world() {
    // three's test builds a parent/child pair; without a scene graph, the
    // single-object half of it: a translated, rotated object maps its own local
    // origin to its world position.
    let mut obj = Object3D::default();
    obj.position.set(2.0, 3.0, 4.0);
    obj.rotate_y(std::f64::consts::PI / 2.0);

    let mut v = Vector3::new(0.0, 0.0, 1.0);
    obj.local_to_world(&mut v);

    close(v.x, 3.0, EPS, "x");
    close(v.y, 3.0, EPS, "y");
    close(v.z, 4.0, EPS, "z");
}

#[test]
fn world_to_local() {
    let mut obj = Object3D::default();
    obj.position.set(2.0, 3.0, 4.0);
    obj.rotate_y(std::f64::consts::PI / 2.0);

    let mut v = Vector3::new(3.0, 3.0, 4.0);
    obj.world_to_local(&mut v);

    close(v.x, 0.0, EPS, "x");
    close(v.y, 0.0, EPS, "y");
    close(v.z, 1.0, EPS, "z");
}

#[test]
fn look_at() {
    let mut obj = Object3D::default();
    obj.look_at(&Vector3::new(0.0, -1.0, 1.0));

    num_equal(obj.rotation.x * RAD_TO_DEG, 45.0, "x is equal");
}

#[test]
fn get_world_position() {
    let mut a = Object3D::default();
    let expected = Vector3::new(X, Y, Z);

    a.translate_x(X);
    a.translate_y(Y);
    a.translate_z(Z);

    assert_eq!(
        a.get_world_position(),
        expected,
        "WorldPosition as expected for single object"
    );
}

#[test]
fn get_world_scale() {
    let mut a = Object3D::default();
    let mut m = Matrix4::identity();
    m.make_scale(X, Y, Z);
    let expected = Vector3::new(X, Y, Z);

    a.apply_matrix4(&m);

    assert_eq!(a.get_world_scale(), expected, "WorldScale as expected");
}

#[test]
fn get_world_direction() {
    let mut a = Object3D::default();
    let sqrt = 0.5 * 2.0_f64.sqrt();
    let expected = Vector3::new(0.0, -sqrt, sqrt);

    a.look_at(&Vector3::new(0.0, -1.0, 1.0));
    let direction = a.get_world_direction();

    assert!(
        (direction.x - expected.x).abs() <= EPS
            && (direction.y - expected.y).abs() <= EPS
            && (direction.z - expected.z).abs() <= EPS,
        "Direction has the expected values: {direction:?}"
    );
}

#[test]
fn update_matrix() {
    let mut a = Object3D::default();
    a.position.set(2.0, 3.0, 4.0);
    a.quaternion.set(5.0, 6.0, 7.0, 8.0);
    a.scale.set(9.0, 10.0, 11.0);

    assert_eq!(
        a.matrix.elements,
        [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        "Updating position, quaternion or scale has no effect until update_matrix()"
    );

    a.update_matrix();

    assert_eq!(
        a.matrix.elements,
        [
            -1521.0, 1548.0, -234.0, 0.0, -520.0, -1470.0, 1640.0, 0.0, 1826.0, 44.0, -1331.0, 0.0,
            2.0, 3.0, 4.0, 1.0
        ],
        "matrix is calculated from position, quaternion and scale"
    );

    assert!(
        a.matrix_world_needs_update,
        "The flag indicating world matrix needs to be updated should be true"
    );
}

// ---------------------------------------------------------------------------
// The scene-graph half: `Object3DNode` on `Node = Rc<RefCell<Object3D>>`.
// ---------------------------------------------------------------------------

use std::rc::Rc;
use three_rs::core::{Node, Object3DNode};

/// `new Object3D()` as a scene-graph node.
fn node() -> Node {
    Object3D::new_node()
}

fn named(name: &str) -> Node {
    let object = node();
    object.borrow_mut().name = name.to_string();
    object
}

#[track_caller]
fn same(a: &Node, b: &Node, what: &str) {
    assert!(Rc::ptr_eq(a, b), "{what}");
}

/// `matrixEquals4` from `test/unit/utils/math-constants.js`'s sibling helpers.
#[track_caller]
fn matrix_equals4(a: &Matrix4, b: &Matrix4, what: &str) {
    for i in 0..16 {
        close(
            a.elements[i],
            b.elements[i],
            0.0001,
            &format!("{what}[{i}]"),
        );
    }
}

#[test]
fn type_name() {
    let object = Object3D::default();
    assert_eq!(
        object.object_type, "Object3D",
        "Object3D.type should be Object3D"
    );
}

#[test]
fn default_matrix_auto_update() {
    let object = Object3D::default();
    assert!(
        object.matrix_auto_update,
        ".matrixAutoUpdate of a new object inherits Object3D.DEFAULT_MATRIX_AUTO_UPDATE = true"
    );
}

#[test]
fn add_remove_remove_from_parent_clear() {
    let a = node();
    let child1 = node();
    let child2 = node();

    assert_eq!(a.children().len(), 0, "Starts with no children");

    a.add(&child1);
    assert_eq!(a.children().len(), 1, "The first child was added");
    same(&a.children()[0], &child1, "It's the right one");

    a.add(&child2);
    assert_eq!(a.children().len(), 2, "The second child was added");
    same(&a.children()[1], &child2, "It's the right one");
    same(&a.children()[0], &child1, "The first one is still there");

    a.remove(&child1);
    assert_eq!(a.children().len(), 1, "The first child was removed");
    same(&a.children()[0], &child2, "The second one is still there");

    a.add(&child1);
    // `a.remove( child1, child2 )` — Rust has no varargs.
    a.remove(&child1);
    a.remove(&child2);
    assert_eq!(a.children().len(), 0, "Both children were removed at once");

    child1.add(&child2);
    assert_eq!(
        child1.children().len(),
        1,
        "The second child was added to the first one"
    );
    a.add(&child2);
    assert_eq!(
        a.children().len(),
        1,
        "The second one was added to the parent (no remove)"
    );
    same(
        &a.children()[0],
        &child2,
        "The second one is now the parent's child again",
    );
    assert_eq!(
        child1.children().len(),
        0,
        "The first one no longer has any children"
    );

    a.add(&child1);
    assert_eq!(
        a.children().len(),
        2,
        "The first child was added to the parent"
    );
    a.clear();
    assert_eq!(a.children().len(), 0, "All children were removed");
    assert!(child1.parent().is_none(), "First child has no parent");
    assert!(child2.parent().is_none(), "Second child has no parent");

    a.add(&child1);
    assert_eq!(a.children().len(), 1, "The child was added to the parent");
    child1.remove_from_parent();
    assert_eq!(a.children().len(), 0, "The child was removed");
    assert!(child1.parent().is_none(), "Child has no parent");
}

#[test]
fn add_self_is_ignored() {
    // `Object3D.add: object can't be added as a child of itself.` — three.js
    // logs an error and returns; there is no console here, so only the
    // behaviour is checked.
    let a = node();
    a.add(&a.clone());
    assert_eq!(a.children().len(), 0, "An object is not its own child");
}

#[test]
fn attach() {
    use std::f64::consts::PI;

    let object = node();
    let old_parent = node();
    let new_parent = node();

    // Attach to a parent

    object.borrow_mut().position.set(1.0, 2.0, 3.0);
    object
        .borrow_mut()
        .set_rotation(PI / 2.0, PI / 3.0, PI / 4.0);
    object.borrow_mut().scale.set(2.0, 3.0, 4.0);
    new_parent.borrow_mut().position.set(4.0, 5.0, 6.0);
    new_parent
        .borrow_mut()
        .set_rotation(PI / 5.0, PI / 6.0, PI / 7.0);
    new_parent.borrow_mut().scale.set(5.0, 5.0, 5.0);

    object.update_matrix_world(false);
    new_parent.update_matrix_world(false);
    let expected_matrix_world = object.borrow().matrix_world;

    new_parent.attach(&object);

    assert!(
        object
            .parent()
            .is_some_and(|parent| Rc::ptr_eq(&parent, &new_parent))
            && !old_parent
                .children()
                .iter()
                .any(|child| Rc::ptr_eq(child, &object)),
        "object is a child of a new parent"
    );

    matrix_equals4(
        &expected_matrix_world,
        &object.borrow().matrix_world,
        "object's world matrix is maintained",
    );

    // Attach to a new parent from an old parent

    object.borrow_mut().position.set(1.0, 2.0, 3.0);
    object
        .borrow_mut()
        .set_rotation(PI / 2.0, PI / 3.0, PI / 4.0);
    object.borrow_mut().scale.set(2.0, 3.0, 4.0);
    old_parent.borrow_mut().position.set(4.0, 5.0, 6.0);
    old_parent
        .borrow_mut()
        .set_rotation(PI / 5.0, PI / 6.0, PI / 7.0);
    old_parent.borrow_mut().scale.set(5.0, 5.0, 5.0);
    new_parent.borrow_mut().position.set(7.0, 8.0, 9.0);
    new_parent
        .borrow_mut()
        .set_rotation(PI / 8.0, PI / 9.0, PI / 10.0);
    new_parent.borrow_mut().scale.set(6.0, 6.0, 6.0);

    old_parent.add(&object);
    old_parent.update_matrix_world(false);
    new_parent.update_matrix_world(false);
    let expected_matrix_world = object.borrow().matrix_world;

    new_parent.attach(&object);

    assert!(
        object
            .parent()
            .is_some_and(|parent| Rc::ptr_eq(&parent, &new_parent))
            && new_parent
                .children()
                .iter()
                .any(|child| Rc::ptr_eq(child, &object))
            && !old_parent
                .children()
                .iter()
                .any(|child| Rc::ptr_eq(child, &object)),
        "object is no longer a child of an old parent and is a child of a new parent now"
    );

    matrix_equals4(
        &expected_matrix_world,
        &object.borrow().matrix_world,
        "object's world matrix is maintained even it had a parent",
    );
}

#[test]
fn get_object_by_id_by_name_by_property() {
    let parent = node();
    let child_name = named("foo");
    let child_id = node(); // id = parent.id + 2
    let child_nothing = node();

    // `parent.prop = true` has no Rust equivalent; the stand-in property is the
    // object's own name, which `getObjectByProperty` also reaches.
    parent.borrow_mut().name = "parent".to_string();
    parent.add(&child_name);
    parent.add(&child_id);
    parent.add(&child_nothing);

    same(
        &parent
            .get_object_by_property(&|object| object.name == "parent")
            .unwrap(),
        &parent,
        "Get parent by its own property",
    );
    same(
        &parent.get_object_by_name("foo").unwrap(),
        &child_name,
        "Get child by name",
    );
    let id = parent.borrow().id + 2;
    same(
        &parent.get_object_by_id(id).unwrap(),
        &child_id,
        "Get child by Id",
    );
    assert!(
        parent
            .get_object_by_property(&|object| object.name == "no-value")
            .is_none(),
        "Unknown property results in undefined"
    );
}

#[test]
fn get_objects_by_property() {
    let parent = node();
    let child_name = named("foo");
    let child_nothing = node();
    let child_name2 = named("foo");
    let child_name3 = named("foo");

    child_name2.add(&child_name3);
    child_name.add(&child_name2);
    parent.add(&child_name);
    parent.add(&child_nothing);

    let found = parent.get_objects_by_property(&|object| object.name == "foo");
    assert_eq!(
        found.len(),
        3,
        "Count the number of children with name \"foo\""
    );
    assert!(
        !found.iter().any(|object| object.borrow().name != "foo"),
        "Get all children with name \"foo\""
    );
}

#[test]
fn node_get_world_position() {
    let a = node();
    let b = node();
    let expected_single = Vector3::new(X, Y, Z);
    let expected_parent = Vector3::new(X, Y, 0.0);
    let expected_child = Vector3::new(X, Y, 7.0);

    a.borrow_mut().translate_x(X);
    a.borrow_mut().translate_y(Y);
    a.borrow_mut().translate_z(Z);

    assert_eq!(
        a.get_world_position(),
        expected_single,
        "WorldPosition as expected for single object"
    );

    // translate child and then parent
    b.borrow_mut().translate_z(7.0);
    a.add(&b);
    a.borrow_mut().translate_z(-Z);

    assert_eq!(
        a.get_world_position(),
        expected_parent,
        "WorldPosition as expected for parent"
    );
    assert_eq!(
        b.get_world_position(),
        expected_child,
        "WorldPosition as expected for child"
    );
}

#[test]
fn node_local_to_world() {
    use std::f64::consts::PI;

    let expected_position = Vector3::new(5.0, -1.0, -4.0);

    let parent = node();
    let child = node();

    parent.borrow_mut().position.set(1.0, 0.0, 0.0);
    parent.borrow_mut().set_rotation(0.0, PI / 2.0, 0.0);
    parent.borrow_mut().scale.set(2.0, 1.0, 1.0);

    child.borrow_mut().position.set(0.0, 1.0, 0.0);
    child.borrow_mut().set_rotation(PI / 2.0, 0.0, 0.0);
    child.borrow_mut().scale.set(1.0, 2.0, 1.0);

    parent.add(&child);
    parent.update_matrix_world(false);

    let mut v = Vector3::new(2.0, 2.0, 2.0);
    child.local_to_world(&mut v);

    assert!(
        (v.x - expected_position.x).abs() <= EPS
            && (v.y - expected_position.y).abs() <= EPS
            && (v.z - expected_position.z).abs() <= EPS,
        "local vector is converted to world: {v:?}"
    );
}

#[test]
fn node_world_to_local() {
    use std::f64::consts::PI;

    let expected_position = Vector3::new(-1.0, 0.5, -1.0);

    let parent = node();
    let child = node();

    parent.borrow_mut().position.set(1.0, 0.0, 0.0);
    parent.borrow_mut().set_rotation(0.0, PI / 2.0, 0.0);
    parent.borrow_mut().scale.set(2.0, 1.0, 1.0);

    child.borrow_mut().position.set(0.0, 1.0, 0.0);
    child.borrow_mut().set_rotation(PI / 2.0, 0.0, 0.0);
    child.borrow_mut().scale.set(1.0, 2.0, 1.0);

    parent.add(&child);
    parent.update_matrix_world(false);

    let mut v = Vector3::new(2.0, 2.0, 2.0);
    child.world_to_local(&mut v);

    assert!(
        (v.x - expected_position.x).abs() <= EPS
            && (v.y - expected_position.y).abs() <= EPS
            && (v.z - expected_position.z).abs() <= EPS,
        "world vector is converted to local: {v:?}"
    );
}

#[test]
fn node_look_at() {
    let obj = node();
    obj.look_at(&Vector3::new(0.0, -1.0, 1.0));

    num_equal(obj.borrow().rotation.x * RAD_TO_DEG, 45.0, "x is equal");
}

#[test]
fn traverse_traverse_visible_traverse_ancestors() {
    let a = named("parent");
    let b = named("child");
    let c = named("childchild 1");
    let d = named("childchild 2");

    c.borrow_mut().visible = false;

    b.add(&c);
    b.add(&d);
    a.add(&b);

    let mut names = Vec::new();
    a.traverse(&mut |object| names.push(object.borrow().name.clone()));
    assert_eq!(
        names,
        ["parent", "child", "childchild 1", "childchild 2"],
        "Traversed objects in expected order"
    );

    let mut names = Vec::new();
    a.traverse_visible(&mut |object| names.push(object.borrow().name.clone()));
    assert_eq!(
        names,
        ["parent", "child", "childchild 2"],
        "Traversed visible objects in expected order"
    );

    let mut names = Vec::new();
    c.traverse_ancestors(&mut |object| names.push(object.borrow().name.clone()));
    assert_eq!(
        names,
        ["child", "parent"],
        "Traversed ancestors in expected order"
    );
}

/// A `Matrix4` that is the identity with `setPosition( x, y, z )` applied.
fn translation(x: f64, y: f64, z: f64) -> Matrix4 {
    let mut m = Matrix4::identity();
    m.set_position(x, y, z);
    m
}

#[test]
fn update_matrix_world() {
    let parent = node();
    let child = node();

    // -- Standard usage test

    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    child.borrow_mut().position.set(4.0, 5.0, 6.0);
    parent.add(&child);

    parent.update_matrix_world(false);

    assert_eq!(
        parent.borrow().matrix.elements,
        translation(1.0, 2.0, 3.0).elements,
        "updateMatrixWorld() updates local matrix"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "updateMatrixWorld() updates world matrix"
    );
    assert_eq!(
        child.borrow().matrix.elements,
        translation(4.0, 5.0, 6.0).elements,
        "updateMatrixWorld() updates children's local matrix"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        translation(5.0, 7.0, 9.0).elements,
        "updateMatrixWorld() updates children's world matrices from their parent world matrix and their local matrices"
    );
    assert!(
        !(parent.borrow().matrix_world_needs_update || child.borrow().matrix_world_needs_update),
        "The flag indicating world matrix needs to be updated should be false after updating world matrix"
    );

    // -- No sync between local position/quaternion/scale/matrix and world matrix

    parent.borrow_mut().position.set(0.0, 0.0, 0.0);
    parent.borrow_mut().update_matrix();

    assert_eq!(
        parent.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Updating position, quaternion, scale, or local matrix has no effect to world matrix until calling updateWorldMatrix()"
    );

    // -- matrixAutoUpdate = false test

    // Resetting local and world matrices to the origin
    child.borrow_mut().position.set(0.0, 0.0, 0.0);
    parent.update_matrix_world(false);

    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    parent.borrow_mut().matrix_auto_update = false;
    child.borrow_mut().matrix_auto_update = false;
    parent.update_matrix_world(false);

    assert_eq!(
        parent.borrow().matrix.elements,
        Matrix4::identity().elements,
        "updateMatrixWorld() doesn't update local matrix if matrixAutoUpdate is false"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        Matrix4::identity().elements,
        "World matrix isn't updated because local matrix isn't updated and the flag indicating world matrix needs to be updated didn't rise"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        Matrix4::identity().elements,
        "No effect to child world matrix if parent local and world matrices and child local matrix are not updated"
    );

    // -- matrixWorldAutoUpdate = false test

    parent.borrow_mut().position.set(3.0, 2.0, 1.0);
    parent.borrow_mut().update_matrix();

    parent.borrow_mut().matrix_auto_update = true;
    child.borrow_mut().matrix_auto_update = true;
    parent.borrow_mut().matrix_world_needs_update = true;
    child.borrow_mut().matrix_world_auto_update = false;
    parent.update_matrix_world(false);

    assert_eq!(
        child.borrow().matrix_world.elements,
        Matrix4::identity().elements,
        "No effect to child world matrix when matrixWorldAutoUpdate is set to false"
    );

    // -- Propagation to children world matrices test

    child.borrow_mut().position.set(0.0, 0.0, 0.0);
    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    child.borrow_mut().matrix_world_auto_update = true;
    parent.update_matrix_world(false);

    assert_eq!(
        child.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Updating parent world matrix has effect to children world matrices even if children local matrices aren't changed"
    );

    // -- force argument test

    // Resetting the local and world matrices to the origin
    child.borrow_mut().position.set(0.0, 0.0, 0.0);
    child.borrow_mut().matrix_auto_update = true;
    parent.update_matrix_world(false);

    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    parent.borrow_mut().update_matrix();
    parent.borrow_mut().matrix_auto_update = false;
    parent.borrow_mut().matrix_world_needs_update = false;

    parent.update_matrix_world(true);

    assert_eq!(
        parent.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "force = true forces to update world matrix even if local matrix is not changed"
    );

    // -- Restriction test: No effect to parent matrices

    // Resetting the local and world matrices to the origin
    parent.borrow_mut().position.set(0.0, 0.0, 0.0);
    child.borrow_mut().position.set(0.0, 0.0, 0.0);
    parent.borrow_mut().matrix_auto_update = true;
    child.borrow_mut().matrix_auto_update = true;
    parent.update_matrix_world(false);

    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    child.borrow_mut().position.set(4.0, 5.0, 6.0);

    child.update_matrix_world(false);

    assert_eq!(
        parent.borrow().matrix.elements,
        Matrix4::identity().elements,
        "updateMatrixWorld() doesn't update parent local matrix"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        Matrix4::identity().elements,
        "updateMatrixWorld() doesn't update parent world matrix"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        translation(4.0, 5.0, 6.0).elements,
        "updateMatrixWorld() calculates world matrix from the current parent world matrix"
    );
}

#[test]
fn update_world_matrix() {
    let object = node();
    let parent = node();
    let child = node();

    let identity = Matrix4::identity();

    parent.add(&object);
    object.add(&child);

    parent.borrow_mut().position.set(1.0, 2.0, 3.0);
    object.borrow_mut().position.set(4.0, 5.0, 6.0);
    child.borrow_mut().position.set(7.0, 8.0, 9.0);

    // Update the world matrix of an object

    object.update_world_matrix(false, false);

    assert_eq!(
        parent.borrow().matrix.elements,
        identity.elements,
        "No effect to parents' local matrices"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        identity.elements,
        "No effect to parents' world matrices"
    );
    assert_eq!(
        object.borrow().matrix.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's local matrix is updated"
    );
    assert_eq!(
        object.borrow().matrix_world.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's world matrix is updated"
    );
    assert_eq!(
        child.borrow().matrix.elements,
        identity.elements,
        "No effect to children's local matrices"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        identity.elements,
        "No effect to children's world matrices"
    );

    // Update the world matrices of an object and its parents

    object.borrow_mut().matrix = identity;
    object.borrow_mut().matrix_world = identity;

    object.update_world_matrix(true, false);

    assert_eq!(
        parent.borrow().matrix.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Parents' local matrices are updated"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Parents' world matrices are updated"
    );
    assert_eq!(
        object.borrow().matrix.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's local matrix is updated"
    );
    assert_eq!(
        object.borrow().matrix_world.elements,
        translation(5.0, 7.0, 9.0).elements,
        "Object's world matrix is updated"
    );
    assert_eq!(
        child.borrow().matrix.elements,
        identity.elements,
        "No effect to children's local matrices"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        identity.elements,
        "No effect to children's world matrices"
    );

    // Update the world matrices of an object and its children

    parent.borrow_mut().matrix = identity;
    parent.borrow_mut().matrix_world = identity;
    object.borrow_mut().matrix = identity;
    object.borrow_mut().matrix_world = identity;

    object.update_world_matrix(false, true);

    assert_eq!(
        parent.borrow().matrix.elements,
        identity.elements,
        "No effect to parents' local matrices"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        identity.elements,
        "No effect to parents' world matrices"
    );
    assert_eq!(
        object.borrow().matrix.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's local matrix is updated"
    );
    assert_eq!(
        object.borrow().matrix_world.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's world matrix is updated"
    );
    assert_eq!(
        child.borrow().matrix.elements,
        translation(7.0, 8.0, 9.0).elements,
        "Children's local matrices are updated"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        translation(11.0, 13.0, 15.0).elements,
        "Children's world matrices are updated"
    );

    // Update the world matrices of an object and its parents and children

    object.borrow_mut().matrix = identity;
    object.borrow_mut().matrix_world = identity;
    child.borrow_mut().matrix = identity;
    child.borrow_mut().matrix_world = identity;

    object.update_world_matrix(true, true);

    assert_eq!(
        parent.borrow().matrix.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Parents' local matrices are updated"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "Parents' world matrices are updated"
    );
    assert_eq!(
        object.borrow().matrix.elements,
        translation(4.0, 5.0, 6.0).elements,
        "Object's local matrix is updated"
    );
    assert_eq!(
        object.borrow().matrix_world.elements,
        translation(5.0, 7.0, 9.0).elements,
        "Object's world matrix is updated"
    );
    assert_eq!(
        child.borrow().matrix.elements,
        translation(7.0, 8.0, 9.0).elements,
        "Children's local matrices are updated"
    );
    assert_eq!(
        child.borrow().matrix_world.elements,
        translation(12.0, 15.0, 18.0).elements,
        "Children's world matrices are updated"
    );

    // object.matrixAutoUpdate = false test

    object.borrow_mut().matrix = identity;
    object.borrow_mut().matrix_world = identity;

    object.borrow_mut().matrix_auto_update = false;
    object.borrow_mut().matrix_world_needs_update = true;
    object.update_world_matrix(true, false);

    assert_eq!(
        object.borrow().matrix.elements,
        identity.elements,
        "No effect to object's local matrix if matrixAutoUpdate is false"
    );
    assert_eq!(
        object.borrow().matrix_world.elements,
        translation(1.0, 2.0, 3.0).elements,
        "object's world matrix is updated even if matrixAutoUpdate is false"
    );

    // object.matrixWorldAutoUpdate = false test

    parent.borrow_mut().matrix_world_auto_update = false;
    child.borrow_mut().matrix_world_auto_update = false;

    child.borrow_mut().matrix_world = identity;
    parent.borrow_mut().matrix_world = identity;

    child.update_world_matrix(true, true);

    assert_eq!(
        child.borrow().matrix_world.elements,
        identity.elements,
        "No effect to child's world matrix if matrixWorldAutoUpdate is false"
    );
    assert_eq!(
        parent.borrow().matrix_world.elements,
        identity.elements,
        "No effect to parent's world matrix if matrixWorldAutoUpdate is false"
    );
}
