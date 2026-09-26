//! Port of `three.js/test/unit/src/scenes/FogExp2.tests.js` (whose module is
//! misspelt `FoxExp2` upstream).
//!
//! `new FogExp2( color, density = 0.00025 )`'s optional argument is two
//! constructors here: `Default` / `with_color` and `new`.

use three_rs::math::Color;
use three_rs::{FogExp2, SceneFog};

#[test]
fn instancing() {
    // no params
    let object = FogExp2::default();
    assert_eq!(
        object.color,
        Color::new(1.0, 1.0, 1.0),
        "Can instantiate a FogExp2."
    );
    assert_eq!(object.density, 0.00025);

    // color
    let object_color = FogExp2::with_color(Color::from_hex(0xffffff));
    assert_eq!(
        object_color.density, 0.00025,
        "Can instantiate a FogExp2 with color."
    );

    // color, density
    let object_all = FogExp2::new(Color::from_hex(0xffffff), 0.00030);
    assert_eq!(
        object_all.density, 0.00030,
        "Can instantiate a FogExp2 with color, density."
    );
}

#[test]
fn is_fog_exp2() {
    let object = FogExp2::default();
    assert!(object.is_fog_exp2(), "FogExp2.isFogExp2 should be true");

    let slot = SceneFog::from(object);
    assert!(slot.is_fog_exp2());
    assert!(!slot.is_fog());
}
