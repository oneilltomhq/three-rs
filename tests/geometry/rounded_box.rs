//! `RoundedBoxGeometry` (addon). No three.js unit test exists, so the
//! bit-exactness sample is the gate (the `toNonIndexed()` it is built on is
//! tested in tests/core_buffer_geometry.rs).

use super::support::*;
use three_rs::core::Group;
use three_rs::geometries::rounded_box_geometry;

include!("samples/rounded_box.rs");

#[test]
fn rounded_box_std_tests() {
    for (label, g) in [
        (
            "RoundedBoxGeometry()",
            rounded_box_geometry(1.0, 1.0, 1.0, 2, 0.1),
        ),
        // webgpu_postprocessing_ao
        (
            "RoundedBoxGeometry(0.9,0.25,0.8,4,0.06)",
            rounded_box_geometry(0.9, 0.25, 0.8, 4, 0.06),
        ),
        (
            "RoundedBoxGeometry(0.9,0.6,0.12,4,0.04)",
            rounded_box_geometry(0.9, 0.6, 0.12, 4, 0.04),
        ),
        // segments 0 -> totalSegments 1 -> the addon returns the unit box
        (
            "RoundedBoxGeometry(2,3,4,0,0.5)",
            rounded_box_geometry(2.0, 3.0, 4.0, 0, 0.5),
        ),
    ] {
        run_std_geometry_tests(label, &g);
    }
}

#[test]
fn rounded_box_samples() {
    for (sample, g) in [
        (&RBOX_DEFAULT, rounded_box_geometry(1.0, 1.0, 1.0, 2, 0.1)),
        // webgpu_postprocessing_ao
        (&RBOX_SEAT, rounded_box_geometry(0.9, 0.25, 0.8, 4, 0.06)),
        (
            &RBOX_BACKREST,
            rounded_box_geometry(0.9, 0.6, 0.12, 4, 0.04),
        ),
        // the un-rounded escape hatch keeps the index, so it is still a plain box
        (&RBOX_NOROUND, rounded_box_geometry(2.0, 3.0, 4.0, 0, 0.5)),
        // radius is clamped to half the shortest side
        (&RBOX_CLAMPED, rounded_box_geometry(1.0, 1.0, 1.0, 2, 10.0)),
    ] {
        check_sample(sample, &g);
    }
}
