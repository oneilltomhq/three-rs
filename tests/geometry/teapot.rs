//! `TeapotGeometry` (addon). three.js has no unit test for it, so the gate is
//! the bit-exactness sample plus the standard data checks.

use super::support::*;
use three_rs::geometries::{teapot_geometry, teapot_geometry_full};

include!("samples/teapot.rs");

#[test]
fn teapot_std_tests() {
    for (label, g) in [
        ("TeapotGeometry()", teapot_geometry(50.0, 10)),
        // webgpu_lights_phong
        ("TeapotGeometry(.8,18)", teapot_geometry(0.8, 18)),
        // webgpu_materials
        ("TeapotGeometry(50,18)", teapot_geometry(50.0, 18)),
        ("TeapotGeometry(1,2)", teapot_geometry(1.0, 2)),
        (
            "TeapotGeometry(3,4,false,false,true,false,false)",
            teapot_geometry_full(3.0, 4, false, false, true, false, false),
        ),
    ] {
        run_std_geometry_tests(label, &g);
    }

    // `segments` is clamped to at least 2
    assert_eq!(
        teapot_geometry(1.0, 0).position().unwrap().array,
        teapot_geometry(1.0, 2).position().unwrap().array
    );
}

#[test]
fn teapot_samples() {
    check_sample(&TEAPOT_08_18, &teapot_geometry(0.8, 18));
    check_sample(&TEAPOT_50_18, &teapot_geometry(50.0, 18));
    check_sample(&TEAPOT_DEFAULT, &teapot_geometry(50.0, 10));
    check_sample(&TEAPOT_MIN, &teapot_geometry(1.0, 2));
    check_sample(
        &TEAPOT_FLAGS,
        &teapot_geometry_full(3.0, 4, false, false, true, false, false),
    );
}
