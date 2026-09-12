//! Port of `three.js/test/unit/src/math/ColorManagement.tests.js`.

use three_rs::math::ColorManagement;

// PROPERTIES
#[test]
fn enabled() {
    assert!(
        ColorManagement::default().enabled,
        "ColorManagement.enabled is true by default."
    );
}
