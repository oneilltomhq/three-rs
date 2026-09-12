//! Port of `three.js/test/unit/src/math/SphericalHarmonics3.tests.js`.

use three_rs::math::SphericalHarmonics3;

// INSTANCING
#[test]
fn instancing() {
    let object = SphericalHarmonics3::default();
    assert_eq!(
        object.coefficients.len(),
        9,
        "Can instantiate a SphericalHarmonics3."
    );
}

// PUBLIC
#[test]
fn is_spherical_harmonics3() {
    assert!(
        SphericalHarmonics3::IS_SPHERICAL_HARMONICS3,
        "SphericalHarmonics3.isSphericalHarmonics3 should be true"
    );
}

// PUBLIC - STATIC
