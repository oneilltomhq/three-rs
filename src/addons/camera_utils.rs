//! Port of `three.js/examples/jsm/utils/CameraUtils.js`, whose one export is
//! [`frame_corners`].

use crate::cameras::PerspectiveCamera;
use crate::math::math_utils::js_min;
use crate::math::{Quaternion, Vector3, RAD2DEG};

/// `CameraUtils.frameCorners( camera, bottomLeftCorner, bottomRightCorner,
/// topLeftCorner, estimateViewFrustum )`: aim `camera` at the rectangle the
/// three corners span and give it the off-axis projection that frames it
/// exactly, as seen from `camera.position`.
///
/// Like three's, this ignores the camera's own projection parameters, so a
/// later [`update_projection_matrix`](PerspectiveCamera::update_projection_matrix)
/// undoes it. The matrix is the one three.js writes whatever the camera's
/// `coordinateSystem`: OpenGL's `[-1, 1]` depth range, `(f + n) / (n - f)`
/// and `2fn / (n - f)` in the third row.
///
/// `estimate_view_frustum` sets `camera.fov` to a conservative estimate
/// that encloses the rectangle; nothing else of the camera changes but its
/// `quaternion` (and so `rotation`), `projectionMatrix` and
/// `projectionMatrixInverse`.
pub fn frame_corners(
    camera: &mut PerspectiveCamera,
    bottom_left_corner: &Vector3,
    bottom_right_corner: &Vector3,
    top_left_corner: &Vector3,
    estimate_view_frustum: bool,
) {
    let (pa, pb, pc) = (bottom_left_corner, bottom_right_corner, top_left_corner);
    let pe = camera.node.borrow().position; // eye position
    let n = camera.near; // distance of near clipping plane
    let f = camera.far; // distance of far clipping plane

    let mut vr = Vector3::default(); // right axis of screen
    vr.sub_vectors(pb, pa).normalize();
    let mut vu = Vector3::default(); // up axis of screen
    vu.sub_vectors(pc, pa).normalize();
    let mut vn = Vector3::default(); // normal vector of screen
    vn.cross_vectors(&vr, &vu).normalize();

    let mut va = Vector3::default(); // from pe to pa
    va.sub_vectors(pa, &pe);
    let mut vb = Vector3::default(); // from pe to pb
    vb.sub_vectors(pb, &pe);
    let mut vc = Vector3::default(); // from pe to pc
    vc.sub_vectors(pc, &pe);

    let d = -va.dot(&vn); // distance from eye to screen
    let l = vr.dot(&va) * n / d; // distance to left screen edge
    let r = vr.dot(&vb) * n / d; // distance to right screen edge
    let b = vu.dot(&va) * n / d; // distance to bottom screen edge
    let t = vu.dot(&vc) * n / d; // distance to top screen edge

    // Set the camera rotation to match the focal plane to the corners' plane
    let mut quat = Quaternion::default();
    quat.set_from_unit_vectors(&Vector3::new(0.0, 1.0, 0.0), &vu);
    let mut z = Vector3::new(0.0, 0.0, 1.0);
    z.apply_quaternion(&quat);
    {
        let mut object = camera.node.borrow_mut();
        object
            .quaternion
            .set_from_unit_vectors(&z, &vn)
            .multiply(&quat);
        // `Quaternion.onChange` → `rotation.setFromQuaternion()`.
        object.sync_rotation_from_quaternion();
    }

    // Set the off-axis projection matrix to match the corners
    camera.projection_matrix.set(
        2.0 * n / (r - l),
        0.0,
        (r + l) / (r - l),
        0.0,
        0.0,
        2.0 * n / (t - b),
        (t + b) / (t - b),
        0.0,
        0.0,
        0.0,
        (f + n) / (n - f),
        2.0 * f * n / (n - f),
        0.0,
        0.0,
        -1.0,
        0.0,
    );
    camera.projection_matrix_inverse = camera.projection_matrix;
    camera.projection_matrix_inverse.invert();

    // FoV estimation to fix frustum culling
    if estimate_view_frustum {
        // Set fieldOfView to a conservative estimate
        // to make frustum tall/wide enough to encompass it
        let mut width = Vector3::default();
        width.sub_vectors(pb, pa);
        let mut height = Vector3::default();
        height.sub_vectors(pc, pa);
        camera.fov = RAD2DEG / js_min(1.0, camera.aspect)
            * ((width.length() + height.length()) / va.length()).atan();
    }
}
