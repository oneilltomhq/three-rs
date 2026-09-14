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
    // Mirrors three.js's `assert.ok( a.isSphericalHarmonics3 )`: this pins the
    // public `IS_SPHERICAL_HARMONICS3` constant's value, which happens to be
    // `true` today but is not statically guaranteed to stay that way.
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(
            SphericalHarmonics3::IS_SPHERICAL_HARMONICS3,
            "SphericalHarmonics3.isSphericalHarmonics3 should be true"
        );
    }
}

// PUBLIC - STATIC
