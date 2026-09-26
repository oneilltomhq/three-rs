//! `ShapeGeometry`, `ExtrudeGeometry` and `TubeGeometry`.
//!
//! three.js' unit files for these three hold only `Extending` / `Instancing`
//! / `type`, with `runStdGeometryTests` commented out; the port instantiates
//! the defaults and the geometries those files build in `beforeEach`, and runs
//! the standard checks on them anyway. The numbers themselves are compared
//! against three.js in full by `tests/geometries_shape_oracle.rs`.

use super::support::*;
use three_rs::extras::{Curve, LineCurve3, Shape};
use three_rs::geometries::{
    extrude_geometry, extrude_geometry_default_shape, shape_geometry, shape_geometry_default_shape,
    tube_geometry, tube_geometry_default_path, ExtrudeGeometryOptions,
};
use three_rs::math::Vector3;

/// The `beforeEach` triangle of `ShapeGeometry.tests.js`.
fn triangle_shape() -> Shape {
    let mut shape = Shape::new();
    shape
        .move_to(0.0, -1.0)
        .line_to(1.0, 1.0)
        .line_to(-1.0, 1.0);
    shape
}

#[test]
fn shape_geometry_instancing() {
    let g = shape_geometry(&shape_geometry_default_shape(), 12);
    run_std_geometry_tests("ShapeGeometry()", &g);
    assert_eq!(g.position().unwrap().count(), 3);
}

#[test]
fn shape_geometry_std_tests() {
    let g = shape_geometry(&triangle_shape(), 12);
    run_std_geometry_tests("ShapeGeometry(triangle)", &g);
    check_index_is_narrowest("ShapeGeometry(triangle)", &g);
    assert!(g.groups.is_empty(), "a single shape adds no group");
}

#[test]
fn extrude_geometry_instancing() {
    let g = extrude_geometry(
        &[extrude_geometry_default_shape()],
        &ExtrudeGeometryOptions::default(),
    );
    run_std_geometry_tests("ExtrudeGeometry()", &g);
    assert!(g.index.is_none(), "ExtrudeGeometry is non-indexed");
    assert_eq!(g.groups.len(), 2, "lid and side groups");
}

#[test]
fn extrude_geometry_triangle() {
    let g = extrude_geometry(
        &[triangle_shape()],
        &ExtrudeGeometryOptions {
            bevel_enabled: false,
            ..Default::default()
        },
    );
    run_std_geometry_tests("ExtrudeGeometry(triangle)", &g);
    // 2 lids of one triangle each, 3 side quads of two triangles each.
    assert_eq!(g.position().unwrap().count(), 2 * 3 + 3 * 6);
}

#[test]
fn tube_geometry_instancing() {
    let (g, frames) = tube_geometry(&tube_geometry_default_path(), 64, 1.0, 8, false);
    run_std_geometry_tests("TubeGeometry()", &g);
    check_index_is_narrowest("TubeGeometry()", &g);
    assert_eq!(frames.tangents.len(), 65);
}

#[test]
fn tube_geometry_std_tests() {
    let path = LineCurve3::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    assert_eq!(path.type_name(), "LineCurve3");
    let (g, _) = tube_geometry(&path, 64, 1.0, 8, false);
    run_std_geometry_tests("TubeGeometry(LineCurve3)", &g);
    check_index_is_narrowest("TubeGeometry(LineCurve3)", &g);
}
