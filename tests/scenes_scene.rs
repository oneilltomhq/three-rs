//! Port of `three.js/test/unit/src/scenes/Scene.tests.js`.
//!
//! `Scene` holds its `Object3D` as a scene-graph `Node` (three.js' `extends
//! Object3D`), so `Extending` ports as "the scene root is an `Object3D` node".
//! The `environment`/`backgroundBlurriness` assertions have nothing to port
//! yet; `Fog` and `FogExp2` have their own files (`scenes_fog.rs`,
//! `scenes_fog_exp2.rs`).

use three_rs::core::Object3D;
use three_rs::Scene;

#[test]
fn instancing() {
    let object = Scene::new();
    assert_eq!(object.children().len(), 0, "Can instantiate a Scene.");
}

#[test]
fn extending() {
    let object = Scene::new();
    assert_eq!(
        object.node.borrow().object_type,
        "Scene",
        "Scene extends from Object3D"
    );

    // Every `Object3D` method is available on the root, which is what the JS
    // `instanceof Object3D` assertion is checking for.
    let child = Object3D::new_node();
    object.add(&child);
    assert_eq!(object.children().len(), 1);
    assert!(
        child.parent().is_some(),
        "the child's parent link points back at the scene"
    );
}

#[test]
fn is_scene() {
    let object = Scene::new();
    assert!(
        object.node.borrow().is_scene,
        "Scene.isScene should be true"
    );
}

#[test]
fn constructor_defaults() {
    let object = Scene::new();
    assert!(object.background.is_none(), "Scene.background is null");
    assert!(
        object.override_material.is_none(),
        "Scene.overrideMaterial is null"
    );
    assert!(object.fog.is_none(), "Scene.fog is null");
}
