//! `TorusGeometry`, `PolyhedronGeometry` and the four platonic solids,
//! `CircleGeometry`, `RingGeometry`, `LatheGeometry`, `CapsuleGeometry`.
//!
//! Parameter sets are the ones in
//! `three.js/test/unit/src/geometries/*.tests.js`.

use super::support::*;
use std::f64::consts::PI;
use three_rs::geometries::{
    capsule_geometry, circle_geometry, circle_geometry_full, dodecahedron_geometry,
    icosahedron_geometry, lathe_default_points, lathe_geometry, lathe_geometry_full,
    octahedron_geometry, polyhedron_geometry, ring_geometry, ring_geometry_full, tetrahedron_geometry,
    torus_geometry, torus_geometry_full,
};

include!("samples/batch2.rs");

#[test]
fn torus_std_tests() {
    let params = [
        (1.0, 0.4, 12usize, 48usize, PI * 2.0, 0.0, PI * 2.0),
        (10.0, 0.4, 12, 48, PI * 2.0, 0.0, PI * 2.0),
        (10.0, 20.0, 12, 48, PI * 2.0, 0.0, PI * 2.0),
        (10.0, 20.0, 30, 48, PI * 2.0, 0.0, PI * 2.0),
        (10.0, 20.0, 30, 10, PI * 2.0, 0.0, PI * 2.0),
        (10.0, 20.0, 30, 10, 2.0, 0.0, PI * 2.0),
        (10.0, 20.0, 30, 10, 2.0, PI * 0.5, PI * 2.0),
        (10.0, 20.0, 30, 10, 2.0, PI * 0.5, PI),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("TorusGeometry #{i}");
        let g = torus_geometry_full(p.0, p.1, p.2, p.3, p.4, p.5, p.6);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }
}

#[test]
fn torus_samples() {
    check_sample(&TORUS_DEFAULT, &torus_geometry(1.0, 0.4, 12, 48));
    check_sample(
        &TORUS_FULL,
        &torus_geometry_full(10.0, 20.0, 30, 10, 2.0, PI * 0.5, PI),
    );
    // webgpu_postprocessing_masking / _outline
    check_sample(&TORUS_MASKING, &torus_geometry(3.0, 1.0, 16, 32));
    check_sample(&TORUS_OUTLINE, &torus_geometry(1.0, 0.3, 16, 100));
}

#[test]
fn polyhedron_std_tests() {
    // the PolyhedronGeometry unit test's own tetrahedron
    let vertices = [
        1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0,
    ];
    let indices = [2, 1, 0, 0, 3, 2, 1, 3, 0, 2, 3, 1];
    let g = polyhedron_geometry(&vertices, &indices, 1.0, 0);
    run_std_geometry_tests("PolyhedronGeometry(tetra)", &g);
    assert!(g.index.is_none(), "PolyhedronGeometry is non-indexed");

    for (label, g) in [
        ("IcosahedronGeometry()", icosahedron_geometry(1.0, 0)),
        ("IcosahedronGeometry(10)", icosahedron_geometry(10.0, 0)),
        ("IcosahedronGeometry(1,1)", icosahedron_geometry(1.0, 1)),
        ("OctahedronGeometry()", octahedron_geometry(1.0, 0)),
        ("OctahedronGeometry(10,2)", octahedron_geometry(10.0, 2)),
        ("TetrahedronGeometry()", tetrahedron_geometry(1.0, 0)),
        ("TetrahedronGeometry(10,3)", tetrahedron_geometry(10.0, 3)),
        ("DodecahedronGeometry()", dodecahedron_geometry(1.0, 0)),
        ("DodecahedronGeometry(10,2)", dodecahedron_geometry(10.0, 2)),
    ] {
        run_std_geometry_tests(label, &g);
    }
}

#[test]
fn polyhedron_samples() {
    check_sample(&ICOSA_DEFAULT, &icosahedron_geometry(1.0, 0));
    check_sample(&ICOSA_10, &icosahedron_geometry(10.0, 0));
    check_sample(&ICOSA_1_1, &icosahedron_geometry(1.0, 1));
    // webgpu_postprocessing_bloom_selective
    check_sample(&ICOSA_1_15, &icosahedron_geometry(1.0, 15));
    // webgpu_postprocessing_ca
    check_sample(&OCTA_25, &octahedron_geometry(2.5, 0));
    check_sample(&OCTA_10_2, &octahedron_geometry(10.0, 2));
    // webgpu_postprocessing_fxaa / _radial_blur
    check_sample(&TETRA_DEFAULT, &tetrahedron_geometry(1.0, 0));
    check_sample(&TETRA_10_3, &tetrahedron_geometry(10.0, 3));
    check_sample(&DODECA_DEFAULT, &dodecahedron_geometry(1.0, 0));
    check_sample(&DODECA_10_2, &dodecahedron_geometry(10.0, 2));

    let vertices = [
        1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0,
    ];
    let indices = [2, 1, 0, 0, 3, 2, 1, 3, 0, 2, 3, 1];
    check_sample(
        &POLY_UNIT_TEST,
        &polyhedron_geometry(&vertices, &indices, 1.0, 0),
    );
}

#[test]
fn circle_std_tests() {
    let params = [
        (1.0, 32usize, 0.0, PI * 2.0),
        (10.0, 32, 0.0, PI * 2.0),
        (10.0, 20, 0.0, PI * 2.0),
        (10.0, 20, 0.1, PI * 2.0),
        (10.0, 20, 0.1, 0.2),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("CircleGeometry #{i}");
        let g = circle_geometry_full(p.0, p.1, p.2, p.3);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }

    // `segments` is clamped to at least 3
    assert_eq!(
        circle_geometry(1.0, 0).position().unwrap().array,
        circle_geometry(1.0, 3).position().unwrap().array
    );
}

#[test]
fn circle_samples() {
    check_sample(&CIRCLE_DEFAULT, &circle_geometry(1.0, 32));
    check_sample(&CIRCLE_FULL, &circle_geometry_full(10.0, 20, 0.1, 0.2));
    // webgpu_postprocessing_ssr
    check_sample(&CIRCLE_SSR, &circle_geometry(2.0, 64));
}

#[test]
fn ring_std_tests() {
    let params = [
        (0.5, 1.0, 32usize, 1usize, 0.0, PI * 2.0),
        (10.0, 1.0, 32, 1, 0.0, PI * 2.0),
        (10.0, 60.0, 32, 1, 0.0, PI * 2.0),
        (10.0, 60.0, 12, 1, 0.0, PI * 2.0),
        (10.0, 60.0, 12, 14, 0.0, PI * 2.0),
        (10.0, 60.0, 12, 14, 0.1, PI * 2.0),
        (10.0, 60.0, 12, 14, 0.1, 2.0),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("RingGeometry #{i}");
        let g = ring_geometry_full(p.0, p.1, p.2, p.3, p.4, p.5);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }
}

#[test]
fn ring_samples() {
    check_sample(&RING_DEFAULT, &ring_geometry(0.5, 1.0, 32, 1));
    check_sample(&RING_FULL, &ring_geometry_full(10.0, 60.0, 12, 14, 0.1, 2.0));
}

#[test]
fn lathe_std_tests() {
    let g = lathe_geometry(&lathe_default_points(), 12);
    run_std_geometry_tests("LatheGeometry()", &g);
    check_index_is_narrowest("LatheGeometry()", &g);

    // the unit test's degenerate case: an empty profile leaves every buffer empty
    let g = lathe_geometry(&[], 0);
    assert_eq!(g.position().unwrap().count(), 0);
    assert_eq!(g.index.unwrap().count(), 0);

    // phiLength is clamped into [ 0, 2PI ]
    assert_eq!(
        lathe_geometry_full(&lathe_default_points(), 12, 0.0, 100.0)
            .position()
            .unwrap()
            .array
            .clone(),
        lathe_geometry_full(&lathe_default_points(), 12, 0.0, PI * 2.0)
            .position()
            .unwrap()
            .array
            .clone()
    );
}

#[test]
fn lathe_samples() {
    check_sample(&LATHE_DEFAULT, &lathe_geometry(&lathe_default_points(), 12));
    check_sample(
        &LATHE_PROFILE,
        &lathe_geometry(
            &[(0.1, -1.0), (0.5, -0.5), (0.4, 0.2), (0.2, 1.0)],
            24,
        ),
    );
}

#[test]
fn capsule_std_tests() {
    let params = [
        (1.0, 1.0, 4usize, 8usize, 1usize),
        (2.0, 1.0, 4, 8, 1),
        (2.0, 2.0, 4, 8, 1),
        (2.0, 2.0, 20, 8, 1),
        (2.0, 2.0, 20, 20, 1),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("CapsuleGeometry #{i}");
        let g = capsule_geometry(p.0, p.1, p.2, p.3, p.4);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }
}

#[test]
fn capsule_samples() {
    check_sample(&CAPSULE_DEFAULT, &capsule_geometry(1.0, 1.0, 4, 8, 1));
    check_sample(&CAPSULE_FULL, &capsule_geometry(2.0, 2.0, 20, 20, 1));
}
