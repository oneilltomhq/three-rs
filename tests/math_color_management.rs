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

// The port's own: `NoColorSpace` sits in the same enum as the defined spaces,
// and `convert()` treats it as three.js treats the falsy `''`.
#[test]
fn no_color_space_converts_nothing() {
    use three_rs::math::{Color, ColorSpace};

    let cm = ColorManagement::default();
    let mut c = Color::new(0.5, 0.25, 0.125);
    cm.convert(&mut c, ColorSpace::Srgb, ColorSpace::NoColorSpace);
    cm.convert(&mut c, ColorSpace::NoColorSpace, ColorSpace::LinearSrgb);
    assert_eq!(c, Color::new(0.5, 0.25, 0.125));

    cm.convert(&mut c, ColorSpace::LinearSrgb, ColorSpace::Srgb);
    assert!(c.r > 0.5, "linear to sRGB encodes: {c:?}");
    cm.convert(&mut c, ColorSpace::Srgb, ColorSpace::LinearSrgb);
    // Three's curves use truncated constants (`0.41666`), so the round trip
    // is close, not exact.
    assert!((c.r - 0.5).abs() < 1e-4, "and decodes back: {c:?}");
}

#[test]
fn get_transfer() {
    use three_rs::math::ColorSpace;

    let cm = ColorManagement::default();
    let linear = cm.get_transfer(ColorSpace::LinearSrgb);
    assert_eq!(cm.get_transfer(ColorSpace::NoColorSpace), linear);
    assert_ne!(cm.get_transfer(ColorSpace::Srgb), linear);
}
