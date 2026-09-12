//! Port of `three.js/test/unit/src/scenes/Scene.tests.js`.
//!
//! `Scene` holds an `Object3D` (three.js' `extends Object3D`) but keeps a flat
//! `Child` list rather than the `Object3DNode` tree, because `src/renderer`
//! walks that list directly — see `docs/scene-graph.md`. So `Extending` ports as
//! "the scene has its `Object3D`", and the `environment`/`fog`/`backgroundBlurriness`
//! assertions have nothing to port yet.

use three_rs::Scene;

#[test]
fn instancing() {
    let object = Scene::new();
    assert_eq!(object.children.len(), 0, "Can instantiate a Scene.");
}

#[test]
fn extending() {
    let object = Scene::new();
    assert_eq!(
        object.object.object_type, "Scene",
        "Scene extends from Object3D"
    );
}

#[test]
fn is_scene() {
    let object = Scene::new();
    assert!(object.object.is_scene, "Scene.isScene should be true");
}

#[test]
fn constructor_defaults() {
    let object = Scene::new();
    assert!(object.background.is_none(), "Scene.background is null");
    assert!(
        object.override_material.is_none(),
        "Scene.overrideMaterial is null"
    );
}
