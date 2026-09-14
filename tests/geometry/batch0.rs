//! `SphereGeometry` and `TorusKnotGeometry` — ported before this branch, given
//! the same gates as the rest so a later edit cannot drift them.

use super::support::*;
use std::f64::consts::PI;
use three_rs::geometries::{sphere_geometry, sphere_geometry_full, torus_knot_geometry};

include!("samples/batch0.rs");

#[test]
fn sphere_std_tests() {
    let params = [
        (1.0, 32usize, 16usize, 0.0, PI * 2.0, 0.0, PI),
        (10.0, 32, 16, 0.0, PI * 2.0, 0.0, PI),
        (10.0, 20, 16, 0.0, PI * 2.0, 0.0, PI),
        (10.0, 20, 30, 0.0, PI * 2.0, 0.0, PI),
        (10.0, 20, 30, 0.5, PI * 2.0, 0.0, PI),
        (10.0, 20, 30, 0.5, 1.0, 0.0, PI),
        (10.0, 20, 30, 0.5, 1.0, 0.4, PI),
        (10.0, 20, 30, 0.5, 1.0, 0.4, 2.0),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("SphereGeometry #{i}");
        let g = sphere_geometry_full(p.0, p.1, p.2, p.3, p.4, p.5, p.6);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }
}

#[test]
fn sphere_samples() {
    check_sample(&SPHERE_DEFAULT, &sphere_geometry(1.0, 32, 16));
    check_sample(
        &SPHERE_FULL,
        &sphere_geometry_full(10.0, 20, 30, 0.5, 1.0, 0.4, 2.0),
    );
    // webgpu_lights_phong, webgpu_lights_physical
    check_sample(&SPHERE_PHONG, &sphere_geometry(0.1, 16, 8));
    // webgpu_postprocessing_dof
    check_sample(&SPHERE_DOF, &sphere_geometry(60.0, 20, 10));
}

#[test]
fn torus_knot_std_tests() {
    let params = [
        (1.0, 0.4, 64usize, 8usize, 2.0, 3.0),
        (10.0, 0.4, 64, 8, 2.0, 3.0),
        (10.0, 20.0, 64, 8, 2.0, 3.0),
        (10.0, 20.0, 30, 8, 2.0, 3.0),
        (10.0, 20.0, 30, 10, 2.0, 3.0),
        (10.0, 20.0, 30, 10, 3.0, 2.0),
    ];
    for (i, p) in params.iter().enumerate() {
        let label = format!("TorusKnotGeometry #{i}");
        let g = torus_knot_geometry(p.0, p.1, p.2, p.3, p.4, p.5);
        run_std_geometry_tests(&label, &g);
        check_index_is_narrowest(&label, &g);
    }
}

#[test]
fn torus_knot_samples() {
    check_sample(&TK_DEFAULT, &torus_knot_geometry(1.0, 0.4, 64, 8, 2.0, 3.0));
    check_sample(&TK_FULL, &torus_knot_geometry(10.0, 20.0, 30, 10, 3.0, 2.0));
    // webgpu_depth_texture (rung 1)
    check_sample(&TK_DEPTH, &torus_knot_geometry(1.0, 0.3, 128, 64, 2.0, 3.0));
    // webgpu_shadowmap
    check_sample(
        &TK_SHADOWMAP,
        &torus_knot_geometry(25.0, 8.0, 75, 80, 2.0, 3.0),
    );
}
