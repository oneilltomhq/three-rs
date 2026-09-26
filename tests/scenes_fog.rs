//! Port of `three.js/test/unit/src/scenes/Fog.tests.js`.
//!
//! `new Fog( color, near = 1, far = 1000 )`'s optional arguments are three
//! constructors here: `Default`, `with_color` and `new`. The JS only asserts
//! that each call returns an object; the port checks the defaults it fills in
//! as well, since those are what `new Fog()` actually promises.

use three_rs::math::Color;
use three_rs::{Fog, SceneFog};

#[test]
fn instancing() {
    // no params
    let object = Fog::default();
    assert_eq!(
        object.color,
        Color::new(1.0, 1.0, 1.0),
        "Can instantiate a Fog."
    );
    assert_eq!((object.near, object.far), (1.0, 1000.0));

    // color
    let object_color = Fog::with_color(Color::from_hex(0xffffff));
    assert_eq!(
        (object_color.near, object_color.far),
        (1.0, 1000.0),
        "Can instantiate a Fog with color."
    );

    // color, near, far
    let object_all = Fog::new(Color::from_hex(0xffffff), 0.015, 100.0);
    assert_eq!(
        (object_all.near, object_all.far),
        (0.015, 100.0),
        "Can instantiate a Fog with color, near, far."
    );
}

#[test]
fn is_fog() {
    let object = Fog::default();
    assert!(object.is_fog(), "Fog.isFog should be true");

    // `scene.fog = fog` keeps the flag, and a `Fog` is not a `FogExp2`.
    let slot = SceneFog::from(object);
    assert!(slot.is_fog());
    assert!(!slot.is_fog_exp2());
}
