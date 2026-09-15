//! `PerspectiveCamera::fit()` and `OrthographicCamera::fit()` — issue #51.
//!
//! Not a port: three.js has no camera fit, so these are the invariants a fit
//! has to hold. The scene is an off-centre, non-cubic box seen from an
//! arbitrary direction, and the test is the one a consumer cares about: project
//! all eight corners and look at where they land in normalised device
//! coordinates.
//!
//! Every case asserts that the *tightest* corner sits at `1 - margin`, not
//! merely inside the frame. That is what tells a real fit apart from a bounding
//! sphere fit, which leaves the tightest corner short of the edge and wastes the
//! difference — the failure the issue was filed for.
//! `a_bounding_sphere_fit_would_fail_these_tests` is that claim, checked.

mod support;

use support::close;

use three_rs::cameras::{OrthographicCamera, PerspectiveCamera};
use three_rs::math::{Box3, Vector3};

/// An off-centre box with three different side lengths.
fn scene_box() -> Box3 {
    Box3::new(Vector3::new(3.0, -2.0, 5.0), Vector3::new(9.0, 1.5, 13.0))
}

fn corners(box_: &Box3) -> Vec<Vector3> {
    let mut corners = Vec::with_capacity(8);
    for i in 0..8 {
        corners.push(Vector3::new(
            if i & 4 == 0 { box_.min.x } else { box_.max.x },
            if i & 2 == 0 { box_.min.y } else { box_.max.y },
            if i & 1 == 0 { box_.min.z } else { box_.max.z },
        ));
    }
    corners
}

/// An arbitrary view direction: not an axis, not the diagonal.
const EYE: Vector3 = Vector3::new(-4.0, 7.0, -2.0);

/// The largest `|x|` and `|y|` in normalised device coordinates over the box's
/// corners, and the largest `z`, having checked every corner is inside the
/// frustum.
#[track_caller]
fn projected_extents(box_: &Box3, camera: &impl three_rs::cameras::RenderCamera) -> (f64, f64) {
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;

    for corner in corners(box_) {
        let mut ndc = corner;
        ndc.project(camera);

        assert!(
            ndc.x >= -1.000001 && ndc.x <= 1.000001,
            "corner {corner:?} is off frame horizontally: {ndc:?}"
        );
        assert!(
            ndc.y >= -1.000001 && ndc.y <= 1.000001,
            "corner {corner:?} is off frame vertically: {ndc:?}"
        );
        // WebGPU clip space: the depth range is [0, 1], so this also checks
        // `near` and `far` bracket the box.
        assert!(
            ndc.z >= -0.000001 && ndc.z <= 1.000001,
            "corner {corner:?} is outside near/far: {ndc:?}"
        );

        max_x = max_x.max(ndc.x.abs());
        max_y = max_y.max(ndc.y.abs());
    }

    (max_x, max_y)
}

#[test]
fn perspective_fit() {
    let box_ = scene_box();

    // A wide frame binds vertically, a tall one horizontally: both axes get to
    // be the axis that decides the distance.
    for aspect in [16.0 / 9.0, 1.0, 0.5] {
        for margin in [0.0, 0.1, 0.25] {
            let mut camera = PerspectiveCamera::new(42.0, aspect, 0.1, 1000.0);
            camera.node.borrow_mut().position = EYE;
            camera.look_at(&box_.get_center());

            camera.fit(&box_, margin);

            let (max_x, max_y) = projected_extents(&box_, &camera);
            let tightest = max_x.max(max_y);

            close(
                tightest,
                1.0 - margin,
                1e-9,
                &format!("perspective aspect {aspect} margin {margin}: tightest corner"),
            );
        }
    }
}

#[test]
fn perspective_fit_keeps_the_view_direction() {
    let box_ = scene_box();

    let mut camera = PerspectiveCamera::new(42.0, 16.0 / 9.0, 0.1, 1000.0);
    camera.node.borrow_mut().position = EYE;
    camera.look_at(&box_.get_center());
    camera.update_matrix_world();

    let before = camera.node.borrow().matrix_world;
    let mut direction_before = Vector3::ZERO;
    direction_before.set_from_matrix_column(&before, 2);

    camera.fit(&box_, 0.0);

    let after = camera.node.borrow().matrix_world;
    let mut direction_after = Vector3::ZERO;
    direction_after.set_from_matrix_column(&after, 2);

    close(
        direction_before
            .normalized()
            .dot(&direction_after.normalized()),
        1.0,
        1e-12,
        "the camera still looks the way it did",
    );

    // And it moved along that direction only: the offset from the box centre is
    // parallel to it.
    let mut offset = Vector3::ZERO;
    offset.set_from_matrix_position(&after);
    offset.sub(&box_.get_center());
    close(
        offset.normalized().dot(&direction_after.normalized()),
        1.0,
        1e-12,
        "the camera sits on the axis through the box centre",
    );
}

/// The same box and camera, framed by the textbook bounding-sphere fit
/// instead: its tightest corner falls well short of the frame edge, so the
/// assertion above — tightest corner *at* the edge — really does rule that fit
/// out.
#[test]
fn a_bounding_sphere_fit_would_fail_these_tests() {
    let box_ = scene_box();

    let mut camera = PerspectiveCamera::new(42.0, 16.0 / 9.0, 0.1, 1000.0);
    camera.node.borrow_mut().position = EYE;
    camera.look_at(&box_.get_center());
    camera.fit(&box_, 0.0);

    // `distance = radius / sin( fov / 2 )`, which frames the sphere around the
    // box rather than the box.
    let sphere = box_.get_bounding_sphere();
    let sphere_distance = sphere.radius / (42.0_f64.to_radians() * 0.5).sin();

    let matrix_world = camera.node.borrow().matrix_world;
    let mut forward = Vector3::ZERO;
    forward.set_from_matrix_column(&matrix_world, 2);
    forward.normalize();

    let mut position = forward;
    position
        .multiply_scalar(sphere_distance)
        .add(&box_.get_center());
    camera.node.borrow_mut().position = position;
    // Standing further back than the fit did, so give the frustum room.
    camera.near = 0.1;
    camera.far = 1000.0;
    camera.update_projection_matrix();
    camera.update_matrix_world();

    let (max_x, max_y) = projected_extents(&box_, &camera);
    let tightest = max_x.max(max_y);

    assert!(
        tightest < 0.95,
        "the sphere fit leaves the tightest corner at {tightest}, so the fit \
         assertions above are not satisfiable by one"
    );
}

#[test]
fn orthographic_fit() {
    let box_ = scene_box();

    for (width, height) in [(16.0, 9.0), (1.0, 1.0), (2.0, 4.0)] {
        for margin in [0.0, 0.1, 0.25] {
            for zoom in [1.0, 2.5] {
                let mut camera = OrthographicCamera::new(
                    -width / 2.0,
                    width / 2.0,
                    height / 2.0,
                    -height / 2.0,
                    0.1,
                    100.0,
                );
                camera.zoom = zoom;
                camera.object.position = EYE;
                camera.look_at(&box_.get_center());

                camera.fit(&box_, margin);

                let (max_x, max_y) = projected_extents(&box_, &camera);
                let tightest = max_x.max(max_y);

                close(
                    tightest,
                    1.0 - margin,
                    1e-9,
                    &format!(
                        "orthographic {width}x{height} zoom {zoom} margin {margin}: \
                         tightest corner"
                    ),
                );

                // The frame keeps the shape it was given.
                close(
                    (camera.right - camera.left) / (camera.top - camera.bottom),
                    width / height,
                    1e-12,
                    "the frame's aspect ratio",
                );
            }
        }
    }
}

#[test]
fn fit_ignores_an_empty_box() {
    let mut camera = PerspectiveCamera::new(42.0, 1.0, 0.1, 1000.0);
    camera.node.borrow_mut().position = EYE;
    camera.fit(&Box3::default(), 0.0);
    assert_eq!(camera.node.borrow().position, EYE);

    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 100.0);
    camera.object.position = EYE;
    camera.fit(&Box3::default(), 0.0);
    assert_eq!(camera.object.position, EYE);
}
