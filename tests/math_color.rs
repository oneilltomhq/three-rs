//! Port of `three.js/test/unit/src/math/Color.tests.js`.
//!
//! three.js toggles a global `ColorManagement.enabled`; most of these tests set
//! it to `false`, which makes `setHex`/`setRGB`/`getHex` skip the transfer
//! function. The Rust port has no global: the colour space is an explicit
//! argument, so `ColorManagement.enabled = false` maps to
//! `ColorSpace::LinearSRGB` (identity) and `enabled = true` with an sRGB input
//! maps to `ColorSpace::SRGB`.
//!
//! Skipped: `Color.NAMES`, `setColorName`, and all the `setStyle*` tests (the
//! port has no CSS colour parser or named-colour table — nothing in the ladder
//! needs one; the HSL/RGB expectations those tests check are covered here via
//! `set_hsl`/`set_rgb`). Also skipped: `isColor`, `clone` (`Copy`), `toJSON`,
//! `iterable`, and the `toArray(array, offset)` sparse-array assertions.

mod support;

use support::EPS;
use three_rs::math::{Color, ColorSpace};

/// QUnit's `assert.numEqual`: tolerance 0.1.
#[track_caller]
fn num_equal(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 0.1, "{what}: {a} vs {b}");
}

fn hex_linear(hex: u32) -> Color {
    let mut c = Color::new(0.0, 0.0, 0.0);
    c.set_hex(hex, ColorSpace::LinearSRGB);
    c
}

#[test]
fn instancing() {
    // `new Color()` is white.
    let c = Color::default();
    assert!(c.r > 0.0 && c.g > 0.0 && c.b > 0.0);

    let c = Color::new(1.0, 1.0, 1.0);
    assert_eq!((c.r, c.g, c.b), (1.0, 1.0, 1.0));
}

#[test]
fn set_scalar() {
    let mut c = Color::default();
    c.set_scalar(0.5);
    assert_eq!((c.r, c.g, c.b), (0.5, 0.5, 0.5));
}

#[test]
fn set_hex() {
    let c = hex_linear(0xFA8072);
    assert_eq!(c.get_hex(ColorSpace::LinearSRGB), 0xFA8072);
    assert_eq!(c.r, 0xFA as f64 / 0xFF as f64);
    assert_eq!(c.g, 0x80 as f64 / 0xFF as f64);
    assert_eq!(c.b, 0x72 as f64 / 0xFF as f64);
}

#[test]
fn set_rgb() {
    let mut c = Color::default();

    c.set_rgb(0.3, 0.5, 0.7, ColorSpace::LinearSRGB);
    assert_eq!((c.r, c.g, c.b), (0.3, 0.5, 0.7), "srgb-linear");

    c.set_rgb(0.3, 0.5, 0.7, ColorSpace::SRGB);
    assert_eq!(
        (
            format!("{:.3}", c.r),
            format!("{:.3}", c.g),
            format!("{:.3}", c.b)
        ),
        ("0.073".into(), "0.214".into(), "0.448".into()),
        "srgb"
    );
}

#[test]
fn set_hsl() {
    let mut c = Color::default();
    c.set_hsl(0.75, 1.0, 0.25, ColorSpace::LinearSRGB);
    let hsl = c.get_hsl(ColorSpace::LinearSRGB);

    assert_eq!(hsl.h, 0.75);
    assert_eq!(hsl.s, 1.0);
    assert_eq!(hsl.l, 0.25);
}

#[test]
fn copy() {
    // 'teal' == 0x008080
    let a = hex_linear(0x008080);
    let mut b = Color::default();
    b.copy(&a);
    assert_eq!(b.r, 0.0);
    assert_eq!(b.g, 0x80 as f64 / 255.0);
    assert_eq!(b.b, 0x80 as f64 / 255.0);
}

#[test]
fn copy_srgb_to_linear() {
    let mut c = Color::default();
    let mut c2 = Color::default();
    c2.set_rgb(0.3, 0.5, 0.9, ColorSpace::LinearSRGB);
    c.copy_srgb_to_linear(&c2);
    num_equal(c.r, 0.09, "Red");
    num_equal(c.g, 0.25, "Green");
    num_equal(c.b, 0.81, "Blue");
}

#[test]
fn copy_linear_to_srgb() {
    let mut c = Color::default();
    let mut c2 = Color::default();
    c2.set_rgb(0.09, 0.25, 0.81, ColorSpace::LinearSRGB);
    c.copy_linear_to_srgb(&c2);
    num_equal(c.r, 0.3, "Red");
    num_equal(c.g, 0.5, "Green");
    num_equal(c.b, 0.9, "Blue");
}

#[test]
fn convert_srgb_to_linear() {
    let mut c = Color::default();
    c.set_rgb(0.3, 0.5, 0.9, ColorSpace::LinearSRGB);
    c.convert_srgb_to_linear();
    num_equal(c.r, 0.09, "Red");
    num_equal(c.g, 0.25, "Green");
    num_equal(c.b, 0.81, "Blue");
}

#[test]
fn convert_linear_to_srgb() {
    let mut c = Color::default();
    c.set_rgb(4.0, 9.0, 16.0, ColorSpace::LinearSRGB);
    c.convert_linear_to_srgb();
    num_equal(c.r, 1.82, "Red");
    num_equal(c.g, 2.58, "Green");
    num_equal(c.b, 3.29, "Blue");
}

#[test]
fn get_hex() {
    // 'red'
    let c = hex_linear(0xFF0000);
    assert_eq!(c.get_hex(ColorSpace::LinearSRGB), 0xFF0000);
}

#[test]
fn get_hex_string() {
    // 'tomato'
    let c = hex_linear(0xFF6347);
    assert_eq!(c.get_hex_string(ColorSpace::LinearSRGB), "ff6347");
}

#[test]
fn get_hsl() {
    let c = hex_linear(0x80ffff);
    let hsl = c.get_hsl(ColorSpace::LinearSRGB);

    assert_eq!(hsl.h, 0.5, "hue");
    assert_eq!(hsl.s, 1.0, "saturation");
    assert_eq!((hsl.l * 100.0).round() / 100.0, 0.75, "lightness");
}

#[test]
fn get_rgb() {
    // 'plum' == 0xDDA0DD, authored in sRGB with colour management on
    let mut c = Color::default();
    c.set_hex(0xDDA0DD, ColorSpace::SRGB);

    let t = c.get_rgb(ColorSpace::LinearSRGB);
    assert_eq!(format!("{:.3}", t.r), "0.723", "r (srgb-linear)");
    assert_eq!(format!("{:.3}", t.g), "0.352", "g (srgb-linear)");
    assert_eq!(format!("{:.3}", t.b), "0.723", "b (srgb-linear)");

    let t = c.get_rgb(ColorSpace::SRGB);
    assert_eq!(format!("{:.3}", t.r), format!("{:.3}", 221.0 / 255.0), "r (srgb)");
    assert_eq!(format!("{:.3}", t.g), format!("{:.3}", 160.0 / 255.0), "g (srgb)");
    assert_eq!(format!("{:.3}", t.b), format!("{:.3}", 221.0 / 255.0), "b (srgb)");
}

#[test]
fn get_style() {
    let mut c = Color::default();
    c.set_hex(0xDDA0DD, ColorSpace::SRGB); // 'plum'
    assert_eq!(c.get_style(), "rgb(221,160,221)", "style: srgb");
}

#[test]
fn offset_hsl() {
    // 'hsl(120,50%,50%)' with colour management off
    let mut a = Color::default();
    a.set_hsl(120.0 / 360.0, 0.5, 0.5, ColorSpace::LinearSRGB);
    let b = Color::new(0.36, 0.84, 0.648);

    a.offset_hsl(0.1, 0.1, 0.1);

    support::close(a.r, b.r, EPS, "Check r");
    support::close(a.g, b.g, EPS, "Check g");
    support::close(a.b, b.b, EPS, "Check b");
}

#[test]
fn add() {
    let mut a = hex_linear(0x0000FF);
    let b = hex_linear(0xFF0000);
    let c = hex_linear(0xFF00FF);

    a.add(&b);
    assert!(a.equals(&c), "Check new value");
}

#[test]
fn add_colors() {
    let a = hex_linear(0x0000FF);
    let b = hex_linear(0xFF0000);
    let c = hex_linear(0xFF00FF);
    let mut d = Color::default();

    d.add_colors(&a, &b);
    assert!(d.equals(&c));
}

#[test]
fn add_scalar() {
    let mut a = Color::new(0.1, 0.0, 0.0);
    let b = Color::new(0.6, 0.5, 0.5);

    a.add_scalar(0.5);
    assert!(a.equals(&b), "Check new value");
}

#[test]
fn sub() {
    let mut a = hex_linear(0x0000CC);
    let b = hex_linear(0xFF0000);
    let c = hex_linear(0x0000AA);

    a.sub(&b);
    assert_eq!(
        a.get_hex(ColorSpace::LinearSRGB),
        0xCC,
        "Difference too large"
    );

    a.sub(&c);
    assert_eq!(a.get_hex(ColorSpace::LinearSRGB), 0x22, "Difference fine");
}

#[test]
fn multiply() {
    let mut a = Color::new(1.0, 0.0, 0.5);
    let b = Color::new(0.5, 1.0, 0.5);
    let c = Color::new(0.5, 0.0, 0.25);

    a.multiply(&b);
    assert!(a.equals(&c), "Check new value");
}

#[test]
fn multiply_scalar() {
    let mut a = Color::new(0.25, 0.0, 0.5);
    let b = Color::new(0.5, 0.0, 1.0);

    a.multiply_scalar(2.0);
    assert!(a.equals(&b), "Check new value");
}

#[test]
fn lerp() {
    let mut c = Color::default();
    let c2 = Color::default(); // white
    c.set_rgb(0.0, 0.0, 0.0, ColorSpace::LinearSRGB);
    c.lerp(&c2, 0.2);
    assert_eq!((c.r, c.g, c.b), (0.2, 0.2, 0.2));
}

#[test]
fn lerp_colors() {
    let a = Color::new(0.0, 0.0, 0.0);
    let b = Color::new(1.0, 1.0, 1.0);
    let mut c = Color::default();

    c.lerp_colors(&a, &b, 0.25);
    assert_eq!((c.r, c.g, c.b), (0.25, 0.25, 0.25));
}

#[test]
fn equals() {
    let mut a = Color::new(0.5, 0.0, 1.0);
    let b = Color::new(0.5, 1.0, 0.0);

    assert_eq!(a.r, b.r, "Components: r is equal");
    assert_ne!(a.g, b.g, "Components: g is not equal");
    assert_ne!(a.b, b.b, "Components: b is not equal");

    assert!(!a.equals(&b), "equals(): a not equal b");
    assert!(!b.equals(&a), "equals(): b not equal a");

    a.copy(&b);
    assert_eq!(a.r, b.r);
    assert_eq!(a.g, b.g);
    assert_eq!(a.b, b.b);

    assert!(a.equals(&b), "equals() after copy(): a equals b");
    assert!(b.equals(&a), "equals() after copy(): b equals a");
}

#[test]
fn from_array() {
    let mut a = Color::default();
    let array = [0.5, 0.6, 0.7, 0.0, 1.0, 0.0];

    a.from_array(&array, 0);
    assert_eq!((a.r, a.g, a.b), (0.5, 0.6, 0.7), "No offset");

    a.from_array(&array, 3);
    assert_eq!((a.r, a.g, a.b), (0.0, 1.0, 0.0), "With offset");
}

#[test]
fn to_array() {
    let (r, g, b) = (0.5, 1.0, 0.0);
    let a = Color::new(r, g, b);
    assert_eq!(a.to_array(), [r, g, b]);
}

#[test]
fn copy_hex() {
    let c2 = hex_linear(0xF5FFFA);
    let mut c = Color::default();
    c.copy(&c2);
    assert_eq!(
        c.get_hex(ColorSpace::LinearSRGB),
        c2.get_hex(ColorSpace::LinearSRGB)
    );
}

#[test]
fn set_with_num() {
    let mut c = Color::default();
    c.set_hex(0xFF0000, ColorSpace::LinearSRGB);
    assert_eq!((c.r, c.g, c.b), (1.0, 0.0, 0.0));
}

#[test]
fn set_hsl_red() {
    // the 'hsl(360,100%,50%)' expectation, via set_hsl
    let mut c = Color::default();
    c.set_hsl(1.0, 1.0, 0.5, ColorSpace::LinearSRGB);
    assert_eq!((c.r, c.g, c.b), (1.0, 0.0, 0.0));
}
