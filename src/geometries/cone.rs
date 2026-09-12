//! Port of `three.js/src/geometries/ConeGeometry.js`.

use crate::core::BufferGeometry;
use crate::geometries::{cylinder_geometry_full, Group};

/// `new ConeGeometry( radius, height, radialSegments, heightSegments )` with the
/// default `openEnded`/`thetaStart`/`thetaLength`.
pub fn cone_geometry(
    radius: f64,
    height: f64,
    radial_segments: usize,
    height_segments: usize,
) -> BufferGeometry {
    cone_geometry_full(
        radius,
        height,
        radial_segments,
        height_segments,
        false,
        0.0,
        std::f64::consts::PI * 2.0,
    )
    .0
}

pub fn cone_geometry_full(
    radius: f64,
    height: f64,
    radial_segments: usize,
    height_segments: usize,
    open_ended: bool,
    theta_start: f64,
    theta_length: f64,
) -> (BufferGeometry, Vec<Group>) {
    cylinder_geometry_full(
        0.0,
        radius,
        height,
        radial_segments,
        height_segments,
        open_ended,
        theta_start,
        theta_length,
    )
}
