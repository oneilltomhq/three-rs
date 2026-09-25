//! Port of `three.js/test/unit/src/objects/Sprite.tests.js`, plus
//! `Sprite.raycast()` cases whose expected values come from three.js under node
//! (`WebGPUCoordinateSystem` camera, the same scene).

use three_rs::cameras::PerspectiveCamera;
use three_rs::core::{Node, Object3D, Raycaster};
use three_rs::math::Vector2;
use three_rs::objects::Sprite;

#[test]
fn extending() {
    let sprite = Sprite::new(None);
    sprite.add(&Object3D::new_node());
    assert_eq!(sprite.children().len(), 1, "Sprite extends from Object3D");
}

#[test]
fn instancing() {
    let object = Sprite::new(None);
    assert!(
        object.borrow().payload.is_sprite(),
        "Can instantiate a Sprite."
    );
}

#[test]
fn type_name() {
    assert_eq!(
        Sprite::new(None).borrow().object_type,
        "Sprite",
        "Sprite.type should be Sprite"
    );
}

#[test]
fn is_sprite() {
    assert!(
        Sprite::new(None).borrow().payload.is_sprite(),
        "Sprite.isSprite should be true"
    );
}

// Not in three's suite: every sprite shares one quad, and `center` starts in
// its middle.
#[test]
fn shared_geometry_and_center() {
    let a = Sprite::new(None);
    let b = Sprite::new(None);
    let (a, b) = (a.borrow(), b.borrow());
    let (a, b) = (a.payload.sprite().unwrap(), b.payload.sprite().unwrap());
    assert!(std::rc::Rc::ptr_eq(&a.geometry, &b.geometry));
    assert_eq!(a.center, Vector2::new(0.5, 0.5));
    assert_eq!(a.geometry.index.as_ref().unwrap().count(), 6);
    assert!(
        a.material.transparent,
        "SpriteMaterial is transparent by default"
    );
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn raycast() {
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 5.0);
    camera.update_matrix_world();

    let sprite = Sprite::new(None);
    sprite.borrow_mut().scale.set(2.0, 2.0, 1.0);
    sprite.update_matrix_world(false);

    let mut raycaster = Raycaster::default();
    let mut intersects = Vec::new();
    sprite.raycast(&raycaster, &mut intersects);
    assert!(intersects.is_empty(), "no camera, no hit");

    raycaster.set_from_camera(&Vector2::new(0.1, 0.05), &camera);
    let hits = raycaster.intersect_object(&sprite, false);
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert!(Node::ptr_eq(&hit.object, &sprite));
    assert!(close(hit.distance, 5.010405838519137));
    assert!(close(hit.point.x, 0.28867513459481237) && close(hit.point.y, 0.14433756729740618));
    let uv = hit.uv.unwrap();
    assert!(close(uv.x, 0.6443375672974062) && close(uv.y, 0.5721687836487032));
    assert!(hit.face.is_none());

    // `center` moves the anchor, `rotation` turns the quad about it.
    {
        let mut object = sprite.borrow_mut();
        let payload = object.payload.sprite_mut().unwrap();
        payload.center = Vector2::new(0.0, 0.0);
        payload.material.rotation = 0.3;
    }
    let hits = raycaster.intersect_object(&sprite, false);
    let uv = hits[0].uv.unwrap();
    assert!(close(uv.x, 0.15921827864919713) && close(uv.y, 0.026290804678692985));

    // Without size attenuation the quad is scaled by its view depth.
    sprite
        .borrow_mut()
        .payload
        .sprite_mut()
        .unwrap()
        .material
        .size_attenuation = false;
    let hits = raycaster.intersect_object(&sprite, false);
    let uv = hits[0].uv.unwrap();
    assert!(close(uv.x, 0.03184365572983944) && close(uv.y, 0.005258160935738592));

    // A ray that misses both triangles.
    raycaster.set_from_camera(&Vector2::new(-0.9, -0.9), &camera);
    assert!(raycaster.intersect_object(&sprite, false).is_empty());
}
