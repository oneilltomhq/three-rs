//! Port of `three.js/test/unit/src/scenes/Scene.tests.js`.
//!
//! three.js' `Scene` extends `Object3D`; here it is still its own struct with a
//! flat child list (`Child`), because `src/renderer` walks that list directly —
//! see `docs/scene-graph.md`. So `Extending` and `isScene` do not port; what is
//! left is the instancing test plus the constructor defaults three.js' `Scene`
//! declares (`background`, `overrideMaterial`).

use three_rs::Scene;

#[test]
fn instancing() {
    let object = Scene::new();
    assert_eq!(object.children.len(), 0, "Can instantiate a Scene.");
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
