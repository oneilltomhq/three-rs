//! Port of `three.js/test/unit/src/cameras/PerspectiveCamera.tests.js`.
//!
//! Skipped: `Extending`, `Instancing`, `type`, `isPerspectiveCamera` (no class
//! hierarchy or type tags in the Rust port) and `clone` (no `copy`/`clone` yet —
//! nothing in the ladder clones a camera).
//!
//! The extra `set_view_offset` / `film` cases at the end are not in three's test
//! file; they are invariants of three's own implementation (a view offset
//! covering the whole frustum must reproduce the plain matrix, and
//! `clearViewOffset` must restore it), used here to pin the ported code.

mod support;

use three_rs::cameras::PerspectiveCamera;
use three_rs::math::{CoordinateSystem, Matrix4};

fn matrix_equals4(a: &Matrix4, b: &Matrix4, tolerance: f64) -> bool {
    a.elements
        .iter()
        .zip(b.elements.iter())
        .all(|(x, y)| (x - y).abs() <= tolerance)
}

/// three.js' default coordinate system is WebGL; `WebGPURenderer` overrides it,
/// and so does this crate's constructor, so the reference matrices below need it
/// set back.
fn webgl_camera(fov: f64, aspect: f64, near: f64, far: f64) -> PerspectiveCamera {
    let mut cam = PerspectiveCamera::new(fov, aspect, near, far);
    cam.coordinate_system = CoordinateSystem::WebGL;
    cam.update_projection_matrix();
    cam
}

#[test]
fn update_projection_matrix() {
    let cam = webgl_camera(75.0, 16.0 / 9.0, 0.1, 300.0);

    // Calculated by hand via glMatrix.perspective(75, 16 / 9, 0.1, 300.0) to get
    // a reference matrix from plain WebGL.
    let reference = Matrix4::from_rows(
        0.7330642938613892,
        0.0,
        0.0,
        0.0,
        0.0,
        1.3032253980636597,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.000666856765747,
        -0.2000666856765747,
        0.0,
        0.0,
        -1.0,
        0.0,
    );

    assert!(
        matrix_equals4(&reference, &cam.projection_matrix, 0.000001),
        "{:?}",
        cam.projection_matrix.elements
    );
}

#[test]
fn get_effective_fov() {
    let mut cam = webgl_camera(75.0, 16.0 / 9.0, 0.1, 300.0);

    support::close(cam.get_effective_fov(), 75.0, 1e-9, "zoom == 1");

    cam.zoom = 2.0;
    assert!(cam.get_effective_fov() < 75.0, "zooming in narrows the FOV");
}

#[test]
fn film_width_height() {
    // landscape: the film height is the gauge divided by the aspect
    let cam = webgl_camera(50.0, 2.0, 0.1, 2000.0);
    assert_eq!(cam.get_film_width(), 35.0);
    assert_eq!(cam.get_film_height(), 17.5);

    // portrait: the film width is the gauge times the aspect
    let cam = webgl_camera(50.0, 0.5, 0.1, 2000.0);
    assert_eq!(cam.get_film_width(), 17.5);
    assert_eq!(cam.get_film_height(), 35.0);
}

#[test]
fn focal_length_round_trip() {
    let mut cam = webgl_camera(50.0, 1.0, 0.1, 2000.0);
    let focal = cam.get_focal_length();

    cam.set_focal_length(focal);
    support::close(cam.fov, 50.0, 1e-9, "fov survives the round trip");
}

#[test]
fn set_clear_view_offset() {
    let mut cam = webgl_camera(75.0, 16.0 / 9.0, 0.1, 300.0);
    let plain = cam.projection_matrix;

    // A view that covers the whole frustum must reproduce the plain matrix.
    cam.set_view_offset(1600.0, 900.0, 0.0, 0.0, 1600.0, 900.0);
    assert!(
        matrix_equals4(&cam.projection_matrix, &plain, 1e-12),
        "full-frustum view offset"
    );

    // A genuine sub-view must not.
    cam.set_view_offset(1600.0, 900.0, 800.0, 0.0, 800.0, 450.0);
    assert!(!matrix_equals4(&cam.projection_matrix, &plain, 1e-6));

    // And clearing it restores the plain matrix (the aspect is unchanged here,
    // since set_view_offset sets aspect = fullWidth / fullHeight = 16 / 9).
    cam.clear_view_offset();
    assert!(
        matrix_equals4(&cam.projection_matrix, &plain, 1e-12),
        "clear_view_offset restores"
    );
}

#[test]
fn projection_matrix_inverse() {
    let cam = webgl_camera(75.0, 16.0 / 9.0, 0.1, 300.0);

    let mut product = Matrix4::identity();
    product.multiply_matrices(&cam.projection_matrix, &cam.projection_matrix_inverse);

    assert!(matrix_equals4(&product, &Matrix4::identity(), 1e-9));
}

#[test]
fn view_size() {
    // At the near plane the view height is 2 * near * tan( fov / 2 ).
    let cam = webgl_camera(90.0, 2.0, 1.0, 100.0);
    let size = cam.get_view_size(1.0);

    support::close(size.y, 2.0, 1e-9, "height at distance 1");
    support::close(size.x, 4.0, 1e-9, "width at distance 1");
}
