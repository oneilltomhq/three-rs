//! `RoundedBoxGeometry` (addon) and the `toNonIndexed()` it is built on.
//! No three.js unit test exists, so the bit-exactness sample is the gate.

use super::support::*;
use three_rs::geometries::{
    box_geometry, rounded_box_geometry, rounded_box_geometry_with_groups, to_non_indexed, Group,
};

include!("samples/rounded_box.rs");

#[test]
fn to_non_indexed_expands_the_index() {
    let indexed = box_geometry(1.0, 1.0, 1.0, 1, 1, 1);
    let flat = to_non_indexed(&indexed);

    let index_count = indexed.index.as_ref().unwrap().count();
    assert!(flat.index.is_none());
    assert_eq!(flat.position.as_ref().unwrap().count(), index_count);
    assert_eq!(flat.normal.as_ref().unwrap().count(), index_count);
    assert_eq!(flat.uv.as_ref().unwrap().count(), index_count);

    // every expanded vertex is the indexed one it came from
    let src = indexed.position.as_ref().unwrap();
    let dst = flat.position.as_ref().unwrap();
    let index = match indexed.index.as_ref().unwrap() {
        three_rs::core::Index::U16(v) => v.iter().map(|&i| i as usize).collect::<Vec<_>>(),
        three_rs::core::Index::U32(v) => v.iter().map(|&i| i as usize).collect::<Vec<_>>(),
    };
    for (i, &src_i) in index.iter().enumerate() {
        assert_eq!(dst.get_x(i), src.get_x(src_i));
        assert_eq!(dst.get_y(i), src.get_y(src_i));
        assert_eq!(dst.get_z(i), src.get_z(src_i));
    }

    // a geometry with no index comes back unchanged
    let none = to_non_indexed(&flat);
    assert_eq!(none.position.unwrap().array, dst.array);
}

#[test]
fn rounded_box_std_tests() {
    for (label, g) in [
        ("RoundedBoxGeometry()", rounded_box_geometry(1.0, 1.0, 1.0, 2, 0.1)),
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
        ("RoundedBoxGeometry(2,3,4,0,0.5)", rounded_box_geometry(2.0, 3.0, 4.0, 0, 0.5)),
    ] {
        run_std_geometry_tests(label, &g);
    }
}

#[test]
fn rounded_box_samples() {
    for (sample, (g, groups)) in [
        (&RBOX_DEFAULT, rounded_box_geometry_with_groups(1.0, 1.0, 1.0, 2, 0.1)),
        // webgpu_postprocessing_ao
        (&RBOX_SEAT, rounded_box_geometry_with_groups(0.9, 0.25, 0.8, 4, 0.06)),
        (&RBOX_BACKREST, rounded_box_geometry_with_groups(0.9, 0.6, 0.12, 4, 0.04)),
        // the un-rounded escape hatch keeps the index, so it is still a plain box
        (&RBOX_NOROUND, rounded_box_geometry_with_groups(2.0, 3.0, 4.0, 0, 0.5)),
        // radius is clamped to half the shortest side
        (&RBOX_CLAMPED, rounded_box_geometry_with_groups(1.0, 1.0, 1.0, 2, 10.0)),
    ] {
        check_sample_with_groups(sample, &g, &groups);
    }
}
