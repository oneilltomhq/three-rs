//! `BoxGeometry`, `PlaneGeometry`, `CylinderGeometry`, `ConeGeometry`.
//!
//! Ports of `three.js/test/unit/src/geometries/{Box,Plane,Cylinder,Cone}Geometry.tests.js`
//! (the parameter sets are theirs) plus bit-exactness samples generated from
//! three.js itself.

use super::support::*;
use three_rs::core::Group;
use three_rs::geometries::{
    box_geometry, box_geometry_default, cone_geometry, cone_geometry_full, cylinder_geometry,
    cylinder_geometry_full, plane_geometry,
};

include!("samples/batch1.rs");

#[test]
fn box_geometry_std_tests() {
    // the three.js unit test's parameter set
    for (label, g) in [
        ("BoxGeometry()", box_geometry_default()),
        (
            "BoxGeometry(10,20,30)",
            box_geometry(10.0, 20.0, 30.0, 1, 1, 1),
        ),
        (
            "BoxGeometry(10,20,30,2,3,4)",
            box_geometry(10.0, 20.0, 30.0, 2, 3, 4),
        ),
    ] {
        run_std_geometry_tests(label, &g);
        check_index_is_narrowest(label, &g);
    }

    let g = box_geometry(10.0, 20.0, 30.0, 2, 3, 4);
    check_groups_cover_index("BoxGeometry(10,20,30,2,3,4)", &g);
    assert_eq!(g.groups.len(), 6, "a box has one group per side");
}

#[test]
fn box_geometry_samples() {
    check_sample(&BOX_DEFAULT, &box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    check_sample(&BOX_10_20_30, &box_geometry(10.0, 20.0, 30.0, 1, 1, 1));
    check_sample(&BOX_SEGMENTED, &box_geometry(10.0, 20.0, 30.0, 2, 3, 4));
    // webgpu_morphtargets
    check_sample(&BOX_MORPH, &box_geometry(2.0, 2.0, 2.0, 32, 32, 32));
    // `box_geometry_default()` is `new BoxGeometry()`, groups included
    check_sample(&BOX_DEFAULT, &box_geometry_default());
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
        (
            1.0,
            1.0,
            1.0,
            32usize,
            1usize,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            1.0,
            1.0,
            32,
            1,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            1.0,
            32,
            1,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            30.0,
            32,
            1,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            30.0,
            20,
            1,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            30.0,
            20,
            30,
            false,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            30.0,
            20,
            30,
            true,
            0.0,
            std::f64::consts::PI * 2.0,
        ),
        (
            10.0,
            20.0,
            30.0,
            20,
            30,
            true,
            0.1,
            std::f64::consts::PI * 2.0,
        ),
        (10.0, 20.0, 30.0, 20, 30, true, 0.1, 2.0),
    ];

    for (i, p) in full.iter().enumerate() {
        let label = format!("CylinderGeometry #{i}");
        let g = cylinder_geometry_full(p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
        check_groups_cover_index(&label, &g);
        // capped cylinders get torso + two caps, open-ended ones only the torso
        assert_eq!(
            g.groups.len(),
            if p.5 { 1 } else { 3 },
            "{label}: group count"
        );
    }
}

#[test]
fn cylinder_geometry_samples() {
    let g = cylinder_geometry_full(1.0, 1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    check_sample(&CYLINDER_DEFAULT, &g);

    let g = cylinder_geometry_full(10.0, 20.0, 30.0, 20, 30, true, 0.1, 2.0);
    check_sample(&CYLINDER_FULL, &g);

    // webgpu_shadowmap
    let g = cylinder_geometry_full(
        0.75,
        0.75,
        7.0,
        32,
        1,
        false,
        0.0,
        std::f64::consts::PI * 2.0,
    );
    check_sample(&CYLINDER_SHADOWMAP, &g);
    // the convenience wrapper must agree with the full form, groups included
    let wrapper = cylinder_geometry(0.75, 0.75, 7.0, 32);
    assert_eq!(
        *wrapper.position().unwrap().array(),
        *g.position().unwrap().array()
    );
    assert_eq!(wrapper.groups, g.groups);
}

#[test]
fn cone_geometry_std_tests() {
    let g = cone_geometry_full(1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    run_std_geometry_tests("ConeGeometry()", &g);
    check_groups_cover_index("ConeGeometry()", &g);
    // radiusTop is 0, so the top cap is skipped: torso + bottom cap
    assert_eq!(g.groups.len(), 2);
    assert_eq!(g.groups[1].material_index, 2);
}

#[test]
fn cone_geometry_samples() {
    let g = cone_geometry_full(1.0, 1.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    check_sample(&CONE_DEFAULT, &g);

    // webgpu_mesh_batch
    let g = cone_geometry_full(1.0, 2.0, 32, 1, false, 0.0, std::f64::consts::PI * 2.0);
    check_sample(&CONE_BATCH, &g);
    let wrapper = cone_geometry(1.0, 2.0, 32, 1);
    assert_eq!(
        *wrapper.position().unwrap().array(),
        *g.position().unwrap().array()
    );
    assert_eq!(wrapper.groups, g.groups);
}

/// Only `BoxGeometry` and `CylinderGeometry` (so `ConeGeometry` too) call
/// `addGroup()` in `three.js/src/geometries`; every other generator leaves
/// `BufferGeometry.groups` empty. The fold-in of the old `*_with_groups()`
/// tuples must not have given any of them one.
#[test]
fn only_box_and_cylinder_set_groups() {
    use three_rs::geometries::*;

    assert_eq!(box_geometry_default().groups.len(), 6);
    assert_eq!(cylinder_geometry(1.0, 1.0, 1.0, 32).groups.len(), 3);
    assert_eq!(cone_geometry(1.0, 1.0, 32, 1).groups.len(), 2);
    // the addon extends BoxGeometry, so it inherits the six sides
    assert_eq!(rounded_box_geometry(1.0, 1.0, 1.0, 2, 0.1).groups.len(), 6);

    for (label, g) in [
        ("CapsuleGeometry", capsule_geometry(1.0, 1.0, 4, 8, 1)),
        ("CircleGeometry", circle_geometry(1.0, 32)),
        ("LatheGeometry", lathe_geometry(&lathe_default_points(), 12)),
        ("PlaneGeometry", plane_geometry(1.0, 1.0, 1, 1)),
        ("IcosahedronGeometry", icosahedron_geometry(1.0, 0)),
        ("DodecahedronGeometry", dodecahedron_geometry(1.0, 0)),
        ("OctahedronGeometry", octahedron_geometry(1.0, 0)),
        ("TetrahedronGeometry", tetrahedron_geometry(1.0, 0)),
        ("QuadGeometry", quad_geometry()),
        ("RingGeometry", ring_geometry(0.5, 1.0, 32, 1)),
        ("SphereGeometry", sphere_geometry(1.0, 32, 16)),
        ("TeapotGeometry", teapot_geometry(1.0, 4)),
        ("TorusGeometry", torus_geometry(1.0, 0.4, 12, 48)),
        (
            "TorusKnotGeometry",
            torus_knot_geometry(1.0, 0.4, 64, 8, 2.0, 3.0),
        ),
    ] {
        assert!(g.groups.is_empty(), "{label}: three.js sets no groups");
    }
}
