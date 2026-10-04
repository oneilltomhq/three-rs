//! `StereoCamera`, `CameraUtils.frameCorners()` and the stereo display
//! passes' camera set-up, against three.js itself.
//!
//! `tools/stereo_camera_reference.mjs` runs three r187dev's own
//! `StereoCamera`, `frameCorners()`, `AnaglyphPassNode` and `StereoPassNode`
//! under node on the inputs below and records what they leave in their
//! cameras and uniforms; its output is committed as
//! `tests/fixtures/stereo_camera/three_r187dev.json`. Each test here feeds
//! the port the same inputs and compares, to a relative 1e-12.
//!
//! The source cameras are given `CoordinateSystem::WebGl`: a three.js
//! `PerspectiveCamera` starts in WebGL's, which `StereoCamera.update()`
//! copies into its eyes, while the port's starts in WebGPU's.

use std::cell::RefCell;
use std::rc::Rc;

use serde_json::Value;
use three_rs::addons::camera_utils::frame_corners;
use three_rs::cameras::{PerspectiveCamera, StereoCamera};
use three_rs::math::{CoordinateSystem, Matrix4, Vector3};
use three_rs::nodes::display::{
    anaglyph_matrices, anaglyph_pass, stereo_pass, AnaglyphAlgorithm, AnaglyphColorMode,
};
use three_rs::Scene;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/stereo_camera/three_r187dev.json"
    );
    let text = std::fs::read_to_string(path).expect("the stereo camera fixture");
    let value: Value = serde_json::from_str(&text).expect("the fixture is JSON");
    assert_eq!(
        value["revision"], "187dev",
        "the fixture's three.js revision"
    );
    value
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("an array, not {value}"))
        .iter()
        .map(|v| v.as_f64().expect("a number"))
        .collect()
}

#[track_caller]
fn assert_close(actual: &[f64], expected: &Value, what: &str) {
    let expected = numbers(expected);
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
        let tolerance = 1e-12 * e.abs().max(1.0);
        assert!(
            (a - e).abs() <= tolerance,
            "{what}[{i}]: port {a:e}, three {e:e} (all: port {actual:?}, three {expected:?})"
        );
    }
}

#[track_caller]
fn assert_scalar(actual: f64, expected: &Value, what: &str) {
    assert_close(&[actual], &Value::Array(vec![expected.clone()]), what);
}

fn source(camera: PerspectiveCamera) -> PerspectiveCamera {
    let mut camera = camera;
    camera.coordinate_system = CoordinateSystem::WebGl;
    camera.update_projection_matrix();
    camera
}

/// `pageCamera()`: `webgpu_display_stereo`'s camera at the graded frame.
fn page_camera() -> PerspectiveCamera {
    let mut camera = source(PerspectiveCamera::new(60.0, 800.0 / 500.0, 0.1, 100.0));
    camera.node.borrow_mut().position.z = 3.0;
    camera.update_matrix_world();
    camera
}

/// `posedCamera()`: every input `StereoCamera`'s cache keys on away from
/// its default.
fn posed_camera() -> PerspectiveCamera {
    let mut camera = source(PerspectiveCamera::new(45.0, 2.0, 0.5, 50.0));
    camera.focus = 5.0;
    camera.zoom = 2.0;
    {
        let mut object = camera.node.borrow_mut();
        object.position.set(1.0, 2.0, 3.0);
        object.set_rotation(0.3, -0.4, 0.1);
    }
    camera.update_projection_matrix();
    camera.update_matrix_world();
    camera
}

#[track_caller]
fn assert_stereo_step(stereo: &StereoCamera, step: &Value, what: &str) {
    for (eye, side) in [(&stereo.camera_l, "left"), (&stereo.camera_r, "right")] {
        let expected = &step[side];
        let object = eye.node.borrow();
        assert_close(
            &eye.projection_matrix.elements,
            &expected["projection"],
            &format!("{what} {side} projectionMatrix"),
        );
        assert_close(
            &object.matrix.elements,
            &expected["matrix"],
            &format!("{what} {side} matrix"),
        );
        assert_eq!(
            Value::Bool(object.matrix_world_needs_update),
            expected["matrix_world_needs_update"],
            "{what} {side} matrixWorldNeedsUpdate"
        );
        assert_eq!(
            Value::Bool(object.matrix_auto_update),
            expected["matrix_auto_update"],
            "{what} {side} matrixAutoUpdate"
        );
        assert_eq!(
            Value::from(object.layers.mask),
            expected["layers"],
            "{what} {side} layers.mask"
        );
    }
}

#[test]
fn stereo_camera_matches_three_on_the_page_camera() {
    let fixture = fixture();
    let camera = page_camera();

    let mut stereo = StereoCamera::new();
    assert_eq!(stereo.aspect, 1.0);
    assert_eq!(stereo.eye_sep, 0.064);
    stereo.update(&camera);
    assert_stereo_step(&stereo, &fixture["stereo"]["page"][0], "page");

    // `StereoPassNode`'s `stereo.aspect = 0.5`.
    let mut half = StereoCamera::new();
    half.aspect = 0.5;
    half.update(&camera);
    assert_stereo_step(
        &half,
        &fixture["stereo"]["page_half_aspect"][0],
        "page, aspect 0.5",
    );
}

#[test]
fn stereo_camera_caches_its_projections_as_three_does() {
    let fixture = fixture();
    let steps = &fixture["stereo"]["posed"];
    let mut camera = posed_camera();
    let mut stereo = StereoCamera::new();
    stereo.eye_sep = 0.1;

    // 0: the first update always builds the projections.
    stereo.update(&camera);
    assert_stereo_step(&stereo, &steps[0], "posed 0");

    // 1: the camera moves; only the eye matrices follow.
    camera.node.borrow_mut().position.set(-2.0, 0.5, 4.0);
    camera.update_matrix_world();
    stereo.update(&camera);
    assert_stereo_step(&stereo, &steps[1], "posed 1");

    // 2: `filmOffset` is not keyed: the eyes keep their projections.
    camera.film_offset = 2.0;
    camera.update_projection_matrix();
    stereo.update(&camera);
    assert_stereo_step(&stereo, &steps[2], "posed 2");

    // 3: `zoom` is: rebuilt from the current projection, film offset and all.
    camera.zoom = 1.0;
    camera.update_projection_matrix();
    stereo.update(&camera);
    assert_stereo_step(&stereo, &steps[3], "posed 3");

    // 4: so is the stereo camera's own `eyeSep`.
    stereo.eye_sep = 0.02;
    stereo.update(&camera);
    assert_stereo_step(&stereo, &steps[4], "posed 4");
}

#[track_caller]
fn assert_framed(camera: &PerspectiveCamera, expected: &Value, what: &str) {
    assert_close(
        &camera.projection_matrix.elements,
        &expected["projection"],
        &format!("{what} projectionMatrix"),
    );
    assert_close(
        &camera.projection_matrix_inverse.elements,
        &expected["projection_inverse"],
        &format!("{what} projectionMatrixInverse"),
    );
    let q = camera.node.borrow().quaternion;
    assert_close(
        &[q.x, q.y, q.z, q.w],
        &expected["quaternion"],
        &format!("{what} quaternion"),
    );
    assert_scalar(camera.fov, &expected["fov"], &format!("{what} fov"));
}

#[test]
fn frame_corners_matches_three() {
    let fixture = fixture();

    let mut camera = PerspectiveCamera::new(50.0, 1.5, 0.2, 30.0);
    camera.node.borrow_mut().position.set(0.3, -0.2, 4.0);
    frame_corners(
        &mut camera,
        &Vector3::new(-1.0, -0.5, 0.0),
        &Vector3::new(1.0, -0.5, 0.0),
        &Vector3::new(-1.0, 0.5, 0.0),
        true,
    );
    assert_framed(&camera, &fixture["frame_corners"]["estimated"], "estimated");

    // A tilted screen, no estimate: `fov` is left alone.
    let mut camera = PerspectiveCamera::new(70.0, 0.5, 0.05, 10.0);
    camera.node.borrow_mut().position.set(1.0, 1.0, 2.0);
    frame_corners(
        &mut camera,
        &Vector3::new(-1.0, -1.0, -0.5),
        &Vector3::new(1.2, -0.8, 0.3),
        &Vector3::new(-1.1, 0.9, -0.2),
        false,
    );
    assert_framed(&camera, &fixture["frame_corners"]["tilted"], "tilted");
    assert_eq!(camera.fov, 70.0);
}

fn scene() -> Rc<RefCell<Scene>> {
    Rc::new(RefCell::new(Scene::new()))
}

#[test]
fn anaglyph_defaults_and_matrices_match_three() {
    let fixture = fixture();
    let defaults = &fixture["anaglyph"]["defaults"];
    let node = anaglyph_pass(scene(), Rc::new(RefCell::new(page_camera())));

    assert_scalar(node.eye_sep(), &defaults["eye_sep"], "eyeSep");
    assert_scalar(
        node.plane_distance(),
        &defaults["plane_distance"],
        "planeDistance",
    );
    assert_eq!(node.algorithm().as_str(), defaults["algorithm"]);
    assert_eq!(node.color_mode().as_str(), defaults["color_mode"]);
    assert_scalar(
        node.stereo().aspect,
        &defaults["stereo_aspect"],
        "stereo.aspect",
    );
    assert_close(
        &node.color_matrix_left().elements,
        &defaults["color_matrix_left"],
        "default left matrix",
    );
    assert_close(
        &node.color_matrix_right().elements,
        &defaults["color_matrix_right"],
        "default right matrix",
    );

    let matrices = &fixture["anaglyph"]["matrices"];
    assert_eq!(
        matrices.as_object().map(|m| m.len()),
        Some(AnaglyphAlgorithm::ALL.len()),
        "algorithms"
    );
    for algorithm in AnaglyphAlgorithm::ALL {
        for color_mode in AnaglyphColorMode::ALL {
            let expected = &matrices[algorithm.as_str()][color_mode.as_str()];
            let what = format!("{} / {}", algorithm.as_str(), color_mode.as_str());
            assert!(expected.is_object(), "{what}: in the fixture");

            node.set_algorithm(algorithm);
            node.set_color_mode(color_mode);
            assert_close(
                &node.color_matrix_left().elements,
                &expected["left"],
                &format!("{what} left"),
            );
            assert_close(
                &node.color_matrix_right().elements,
                &expected["right"],
                &format!("{what} right"),
            );

            let (left, right) = anaglyph_matrices(algorithm, color_mode);
            assert_eq!(
                left,
                node.color_matrix_left(),
                "{what}: anaglyph_matrices left"
            );
            assert_eq!(
                right,
                node.color_matrix_right(),
                "{what}: anaglyph_matrices right"
            );
        }
    }
}

#[track_caller]
fn assert_anaglyph_eye(eye: &PerspectiveCamera, expected: &Value, what: &str) {
    assert_eq!(
        eye.coordinate_system,
        CoordinateSystem::WebGpu,
        "{what} coordinateSystem"
    );
    assert_close(
        &eye.projection_matrix.elements,
        &expected["projection"],
        &format!("{what} projectionMatrix"),
    );
    assert_close(
        &eye.projection_matrix_inverse.elements,
        &expected["projection_inverse"],
        &format!("{what} projectionMatrixInverse"),
    );
    assert_close(
        &eye.matrix_world_inverse.elements,
        &expected["matrix_world_inverse"],
        &format!("{what} matrixWorldInverse"),
    );
    let object = eye.node.borrow();
    assert_close(
        &object.matrix_world.elements,
        &expected["matrix_world"],
        &format!("{what} matrixWorld"),
    );
    let (p, q) = (object.position, object.quaternion);
    assert_close(
        &[p.x, p.y, p.z],
        &expected["position"],
        &format!("{what} position"),
    );
    assert_close(
        &[q.x, q.y, q.z, q.w],
        &expected["quaternion"],
        &format!("{what} quaternion"),
    );
    assert_scalar(eye.fov, &expected["fov"], &format!("{what} fov"));
    assert_scalar(eye.near, &expected["near"], &format!("{what} near"));
    assert_scalar(eye.far, &expected["far"], &format!("{what} far"));
}

#[test]
fn anaglyph_eyes_match_three() {
    let fixture = fixture();

    // The page: `anaglyph.eyeSep = 0.064; anaglyph.planeDistance = 3`.
    let node = anaglyph_pass(scene(), Rc::new(RefCell::new(page_camera())));
    node.set_eye_sep(0.064);
    node.set_plane_distance(3.0);
    node.update_stereo_camera(CoordinateSystem::WebGpu);
    {
        let stereo = node.stereo();
        let expected = &fixture["anaglyph"]["page"];
        assert_anaglyph_eye(&stereo.camera_l, &expected["left"], "page left");
        assert_anaglyph_eye(&stereo.camera_r, &expected["right"], "page right");
    }

    // The node's defaults, under a posed camera.
    let node = anaglyph_pass(scene(), Rc::new(RefCell::new(posed_camera())));
    node.update_stereo_camera(CoordinateSystem::WebGpu);
    let stereo = node.stereo();
    let expected = &fixture["anaglyph"]["posed"];
    assert_anaglyph_eye(&stereo.camera_l, &expected["left"], "posed left");
    assert_anaglyph_eye(&stereo.camera_r, &expected["right"], "posed right");
}

/// An eye's world matrix is composed by `updateStereoCamera()` and must
/// survive the `updateMatrixWorld()` every render starts with:
/// `matrixAutoUpdate` is off and nothing asks for a world update.
#[test]
fn anaglyph_eyes_keep_their_world_matrices_through_a_render_update() {
    let node = anaglyph_pass(scene(), Rc::new(RefCell::new(posed_camera())));
    node.update_stereo_camera(CoordinateSystem::WebGpu);
    let mut stereo = node.stereo();
    let before: Matrix4 = stereo.camera_l.node.borrow().matrix_world;
    assert_ne!(before, Matrix4::identity());
    stereo.camera_l.update_matrix_world();
    assert_eq!(stereo.camera_l.node.borrow().matrix_world, before);
}

#[test]
fn stereo_pass_halves_the_aspect() {
    let fixture = fixture();
    let node = stereo_pass(scene(), Rc::new(RefCell::new(page_camera())));
    assert_scalar(
        node.stereo().aspect,
        &fixture["stereo_pass"]["stereo_aspect"],
        "stereoPass.stereo.aspect",
    );
}
