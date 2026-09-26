//! Port of `three.js/test/unit/src/cameras/OrthographicCamera.tests.js`.
//!
//! Skipped: `Extending`, `Instancing` and `type` (no class hierarchy or type
//! tags in the Rust port). `clone` is Rust's derived `Clone`.
//!
//! three's file has no view-offset case, so the `set_view_offset` tests at the
//! end check against matrices three itself printed: `new OrthographicCamera(
//! … )`, `setViewOffset( … )`, `projectionMatrix.elements`, run under node at
//! the vendored commit.

use three_rs::cameras::OrthographicCamera;
use three_rs::math::CoordinateSystem;
use three_rs::RenderCamera;

/// three.js' default coordinate system is WebGL; this crate's constructor
/// defaults to WebGPU, so the reference matrices need it set back.
fn webgl_camera(
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    near: f64,
    far: f64,
) -> OrthographicCamera {
    let mut cam = OrthographicCamera::new(left, right, top, bottom, near, far);
    cam.coordinate_system = CoordinateSystem::WebGL;
    cam.update_projection_matrix();
    cam
}

#[test]
fn is_orthographic_camera() {
    let object = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 2000.0);
    assert!(
        object.is_orthographic_camera(),
        "OrthographicCamera.isOrthographicCamera should be true"
    );
    assert!(!object.is_perspective_camera());
}

#[test]
fn update_projection_matrix() {
    let (left, right, top, bottom, near, far) = (-1.0, 1.0, 1.0, -1.0, 1.0, 3.0);
    let cam = webgl_camera(left, right, top, bottom, near, far);

    // updateProjectionMatrix is called in constructor
    let p_matrix = cam.projection_matrix.elements;

    // orthographic projection is given my the 4x4 Matrix
    // 2/r-l		0			 0		-(l+r/r-l)
    //   0		2/t-b		 0		-(t+b/t-b)
    //   0			0		-2/f-n	-(f+n/f-n)
    //   0			0			 0				1

    assert_eq!(p_matrix[0], 2.0 / (right - left), "m[0,0] === 2 / (r - l)");
    assert_eq!(p_matrix[5], 2.0 / (top - bottom), "m[1,1] === 2 / (t - b)");
    assert_eq!(p_matrix[10], -2.0 / (far - near), "m[2,2] === -2 / (f - n)");
    assert_eq!(
        p_matrix[12],
        -((right + left) / (right - left)),
        "m[3,0] === -(r+l/r-l)"
    );
    assert_eq!(
        p_matrix[13],
        -((top + bottom) / (top - bottom)),
        "m[3,1] === -(t+b/b-t)"
    );
    assert_eq!(
        p_matrix[14],
        -((far + near) / (far - near)),
        "m[3,2] === -(f+n/f-n)"
    );
}

#[test]
fn clone() {
    let (left, right, top, bottom, near, far) = (-1.5, 1.5, 1.0, -1.0, 0.1, 42.0);
    let cam = OrthographicCamera::new(left, right, top, bottom, near, far);

    let cloned_cam = cam.clone();

    assert_eq!(cam.left, cloned_cam.left, "left is equal");
    assert_eq!(cam.right, cloned_cam.right, "right is equal");
    assert_eq!(cam.top, cloned_cam.top, "top is equal");
    assert_eq!(cam.bottom, cloned_cam.bottom, "bottom is equal");
    assert_eq!(cam.near, cloned_cam.near, "near is equal");
    assert_eq!(cam.far, cloned_cam.far, "far is equal");
    assert_eq!(cam.zoom, cloned_cam.zoom, "zoom is equal");
    assert_eq!(cam.view, cloned_cam.view, "view is equal");
}

/// A quarter-size window of a zoomed frame, WebGL depth:
/// `cam.zoom = 1.5; cam.setViewOffset( 1600, 900, 400, 300, 800, 450 )`.
#[test]
fn set_view_offset_window() {
    let mut cam = webgl_camera(-2.0, 2.0, 1.5, -1.5, 0.1, 100.0);
    cam.zoom = 1.5;
    cam.set_view_offset(1600.0, 900.0, 400.0, 300.0, 800.0, 450.0);

    assert_eq!(
        cam.projection_matrix.elements,
        [
            1.4999999999999998,
            0.0,
            0.0,
            0.0,
            0.0,
            2.0,
            0.0,
            0.0,
            0.0,
            0.0,
            -0.02002002002002002,
            0.0,
            -3.330669073875469e-16,
            0.33333333333333326,
            -1.002002002002002,
            1.0,
        ]
    );
}

/// A sub-pixel jitter — the TRAA/SSAA use — in the port's WebGPU depth range,
/// through the `RenderCamera` trait:
/// `setViewOffset( 800, 500, 0.3125, -0.1875, 800, 500 )`.
#[test]
fn set_view_offset_jitter_through_the_trait() {
    let mut cam = OrthographicCamera::new(-2.0, 2.0, 1.5, -1.5, 0.1, 100.0);
    let plain = cam.projection_matrix;

    let camera: &mut dyn RenderCamera = &mut cam;
    camera.set_view_offset(800.0, 500.0, 0.3125, -0.1875, 800.0, 500.0);

    assert_eq!(
        camera.projection_matrix().elements,
        [
            0.5,
            0.0,
            0.0,
            0.0,
            0.0,
            0.6666666666666666,
            0.0,
            0.0,
            0.0,
            0.0,
            -0.01001001001001001,
            0.0,
            -0.0007812499999999556,
            -0.0007500000000000284,
            -0.001001001001001001,
            1.0,
        ]
    );

    // `clearViewOffset()` restores the plain matrix and keeps the view,
    // disabled.
    camera.clear_view_offset();
    assert_eq!(camera.projection_matrix(), plain);
    assert!(cam.view.is_some_and(|view| !view.enabled));
    assert_eq!(cam.projection_matrix_inverse, {
        let mut inverse = plain;
        inverse.invert();
        inverse
    });
}

/// `clearViewOffset()` on a camera that never had one is a plain
/// `updateProjectionMatrix()`.
#[test]
fn clear_view_offset_without_a_view() {
    let mut cam = OrthographicCamera::new(-2.0, 2.0, 1.5, -1.5, 0.1, 100.0);
    let plain = cam.projection_matrix;
    cam.clear_view_offset();
    assert!(cam.view.is_none());
    assert_eq!(cam.projection_matrix, plain);
}
