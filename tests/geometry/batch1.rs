//! `BoxGeometry`, `PlaneGeometry`, `CylinderGeometry`, `ConeGeometry`.
//!
//! Ports of `three.js/test/unit/src/geometries/{Box,Plane,Cylinder,Cone}Geometry.tests.js`
//! (the parameter sets are theirs) plus bit-exactness samples generated from
//! three.js itself.

use super::support::*;
use three_rs::geometries::{
    box_geometry, box_geometry_default, box_geometry_with_groups, cone_geometry, cone_geometry_full,
    cylinder_geometry, cylinder_geometry_full, plane_geometry, Group,
};

include!("samples/batch1.rs");

#[test]
fn box_geometry_std_tests() {
    // the three.js unit test's parameter set
    for (label, g) in [
        ("BoxGeometry()", box_geometry_default()),
        ("BoxGeometry(10,20,30)", box_geometry(10.0, 20.0, 30.0, 1, 1, 1)),
        (
            "BoxGeometry(10,20,30,2,3,4)",
            box_geometry(10.0, 20.0, 30.0, 2, 3, 4),
        ),
    ] {
        run_std_geometry_tests(label, &g);
        check_index_is_narrowest(label, &g);
    }

    let (g, groups) = box_geometry_with_groups(10.0, 20.0, 30.0, 2, 3, 4);
    check_groups_cover_index("BoxGeometry(10,20,30,2,3,4)", &g, &groups);
    assert_eq!(groups.len(), 6, "a box has one group per side");
}

#[test]
fn box_geometry_samples() {
    let (g, groups) = box_geometry_with_groups(1.0, 1.0, 1.0, 1, 1, 1);
    check_sample_with_groups(&BOX_DEFAULT, &g, &groups);

    let (g, groups) = box_geometry_with_groups(10.0, 20.0, 30.0, 1, 1, 1);
    check_sample_with_groups(&BOX_10_20_30, &g, &groups);

    let (g, groups) = box_geometry_with_groups(10.0, 20.0, 30.0, 2, 3, 4);
    check_sample_with_groups(&BOX_SEGMENTED, &g, &groups);

    // webgpu_morphtargets
    let (g, groups) = box_geometry_with_groups(2.0, 2.0, 2.0, 32, 32, 32);
    check_sample_with_groups(&BOX_MORPH, &g, &groups);
}

#[test]
fn plane_geometry_std_tests() {
    for (label, g) in [
        ("PlaneGeometry()", plane_geometry(1.0, 1.0, 1, 1)),
        ("PlaneGeometry(10)", plane_geometry(10.0, 1.0, 1, 1)),
        ("PlaneGeometry(10,30)", plane_geometry(10.0, 30.0, 1, 1)),
        ("PlaneGeometry(10,30,3)", plane_geometry(10.0, 30.0, 3, 1)),
        ("PlaneGeometry(10,30,3,5)", plane_geometry(10.0, 30.0, 3, 5)),
    ] {
        run_std_geometry_tests(label, &g);
    }
}

#[test]
fn plane_geometry_samples() {
    check_sample(&PLANE_DEFAULT, &plane_geometry(1.0, 1.0, 1, 1));
    check_sample(&PLANE_10_30_3_5, &plane_geometry(10.0, 30.0, 3, 5));
    // webgpu_shadowmap, webgpu_postprocessing_fog
    check_sample(&PLANE_200, &plane_geometry(200.0, 200.0, 1, 1));
    // webgpu_postprocessing_3dlut / _retro
    check_sample(&PLANE_1_1_16_64, &plane_geometry(1.0, 1.0, 16, 64));
}

#[test]
fn cylinder_geometry_std_tests() {
    let full = [
        (1.0, 1.0, 1.0, 32usize, 1usize, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 20, 1, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 20, 30, false, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 20, 30, true, 0.0, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 20, 30, true, 0.1, std::f64::consts::PI * 2.0),
        (10.0, 20.0, 30.0, 20, 30, true, 0.1, 2.0),
    ];

    for (i, p) in full.iter().enumerate() {
        let label = format!("CylinderGeometry #{i}");
        let (g, groups) = cylinder_geometry_full(p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
        check_groups_cover_index(&label, &g, &groups);
        // capped cylinders get torso + two caps, open-ended ones only the torso
        assert_eq!(groups.len(), if p.5 { 1 } else { 3 }, "{label}: group count");
    }
}

#[test]
fn cylinder_geometry_samples() {
    let (g, groups) = cylinder_geometry_full(
        1.0,
        1.0,
        1.0,
        32,
        1,
        false,
        0.0,
        std::f64::consts::PI * 2.0,
    );
    check_sample_with_groups(&CYLINDER_DEFAULT, &g, &groups);

    let (g, groups) = cylinder_geometry_full(10.0, 20.0, 30.0, 20, 30, true, 0.1, 2.0);
    check_sample_with_groups(&CYLINDER_FULL, &g, &groups);

    // webgpu_shadowmap
    let (g, groups) = cylinder_geometry_full(
        0.75,
        0.75,
        7.0,
        32,
        1,
        false,
        0.0,
        std::f64::consts::PI * 2.0,
    );
    check_sample_with_groups(&CYLINDER_SHADOWMAP, &g, &groups);
    // the convenience wrapper must agree with the full form
    assert_eq!(
        cylinder_geometry(0.75, 0.75, 7.0, 32).position().unwrap().array,
        g.position().unwrap().array
    );
}

#[test]
fn cone_geometry_std_tests() {
    let (g, groups) = cone_geometry_full(1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    run_std_geometry_tests("ConeGeometry()", &g);
    check_groups_cover_index("ConeGeometry()", &g, &groups);
    // radiusTop is 0, so the top cap is skipped: torso + bottom cap
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[1].material_index, 2);
}

#[test]
fn cone_geometry_samples() {
    let (g, groups) = cone_geometry_full(1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    check_sample_with_groups(&CONE_DEFAULT, &g, &groups);

    // webgpu_mesh_batch
    let (g, groups) = cone_geometry_full(1.0, 2.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    check_sample_with_groups(&CONE_BATCH, &g, &groups);
    assert_eq!(
        cone_geometry(1.0, 2.0, 32, 1).position().unwrap().array,
        g.position().unwrap().array
    );
}
