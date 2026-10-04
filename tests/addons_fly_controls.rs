//! Grades [`three_rs::addons::controls::FlyControls`] against three.js'
//! `examples/jsm/controls/FlyControls.js` itself.
//!
//! `tools/fly_controls_reference.mjs` runs the JS class under node through
//! five scripted input sequences and writes, after every step, the camera's
//! position and quaternion and whether `change` was dispatched to
//! `tests/fixtures/fly_controls.json`. This file replays the identical
//! sequences through the port and asserts every number matches to
//! [`TOLERANCE`] and every `change` matches `update()`'s return value.
//!
//! The fixture is committed, so the comparison runs everywhere. Where there is
//! also a three.js checkout and node, the script is run again into a scratch
//! file and the port is held to that too, which catches a fixture that has
//! gone stale against the pinned three.js.

use std::path::{Path, PathBuf};
use std::process::Command;

use three_rs::addons::controls::{FlyControls, KeyCode, MouseButton};
use three_rs::cameras::PerspectiveCamera;
use three_rs::math::Vector3;

/// How close the port has to be. Both sides are f64 doing the same operations
/// in the same order; the slack is for the last bits of reassociation.
const TOLERANCE: f64 = 1e-9;

const DT: f64 = 1.0 / 60.0;

const SCRIPT: &str = "tools/fly_controls_reference.mjs";
const FIXTURE: &str = "tests/fixtures/fly_controls.json";

/// The reference script's element: its size, and the page offset the script
/// adds to every pointer position (and the port, which takes element-relative
/// positions, never sees).
const ELEMENT_WIDTH: f64 = 640.0;
const ELEMENT_HEIGHT: f64 = 360.0;

#[derive(Debug, Clone, Copy)]
struct Step {
    position: [f64; 3],
    quaternion: [f64; 4],
    changed: Option<bool>,
}

impl Step {
    fn of(camera: &PerspectiveCamera, changed: Option<bool>) -> Self {
        let object = camera.node.borrow();
        let (p, q) = (object.position, object.quaternion);
        Self {
            position: [p.x, p.y, p.z],
            quaternion: [q.x, q.y, q.z, q.w],
            changed,
        }
    }

    fn from_json(value: &serde_json::Value) -> Self {
        let array = |key: &str| -> Vec<f64> {
            value[key]
                .as_array()
                .unwrap_or_else(|| panic!("reference step has no array `{key}`"))
                .iter()
                .map(|n| n.as_f64().expect("reference step is not numeric"))
                .collect()
        };
        let p = array("position");
        let q = array("quaternion");
        Self {
            position: [p[0], p[1], p[2]],
            quaternion: [q[0], q[1], q[2], q[3]],
            changed: value["changed"].as_bool(),
        }
    }
}

/// The camera every scenario starts from, matching the reference script's.
fn camera() -> PerspectiveCamera {
    let mut camera = PerspectiveCamera::new(45.0, ELEMENT_WIDTH / ELEMENT_HEIGHT, 0.25, 200.0);
    camera.node.borrow_mut().position.set(3.0, 4.0, 5.0);
    camera.look_at(&Vector3::new(-1.0, 1.5, 0.5));
    camera.update_matrix_world();
    camera
}

fn controls() -> FlyControls {
    let mut controls = FlyControls::new();
    controls.set_element_size(ELEMENT_WIDTH, ELEMENT_HEIGHT);
    controls
}

/// Runs `update( delta )` `n` times, recording after each.
fn frames(
    controls: &mut FlyControls,
    camera: &mut PerspectiveCamera,
    steps: &mut Vec<Step>,
    n: usize,
    delta: f64,
) {
    for _ in 0..n {
        let changed = controls.update(&camera.node, delta);
        steps.push(Step::of(camera, Some(changed)));
    }
}

fn keys() -> Vec<Step> {
    let mut c = camera();
    let mut controls = controls();
    controls.movement_speed = 4.0;
    controls.roll_speed = 0.6;
    let mut steps = vec![Step::of(&c, None)];

    controls.key_down(KeyCode::KeyW, false);
    controls.key_down(KeyCode::KeyA, false);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.key_down(KeyCode::ShiftLeft, false);
    assert_eq!(controls.movement_speed_multiplier, 0.1);
    controls.key_down(KeyCode::KeyR, false);
    controls.key_down(KeyCode::ArrowUp, false);
    controls.key_down(KeyCode::ArrowLeft, false);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.key_up(KeyCode::ShiftLeft);
    assert_eq!(controls.movement_speed_multiplier, 1.0);
    controls.key_up(KeyCode::KeyW);
    controls.key_up(KeyCode::KeyA);
    controls.key_up(KeyCode::KeyR);
    controls.key_up(KeyCode::ArrowUp);
    controls.key_up(KeyCode::ArrowLeft);
    controls.key_down(KeyCode::KeyS, false);
    controls.key_down(KeyCode::KeyD, false);
    controls.key_down(KeyCode::KeyF, false);
    controls.key_down(KeyCode::ArrowDown, false);
    controls.key_down(KeyCode::ArrowRight, false);
    controls.key_down(KeyCode::KeyQ, false);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.key_up(KeyCode::KeyQ);
    controls.key_down(KeyCode::KeyE, false);
    controls.key_down(KeyCode::Other, false); // `KeyZ`
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    // Everything up: nothing moves and `change` stops.
    for key in [
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::KeyF,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::KeyE,
    ] {
        controls.key_up(key);
    }
    controls.key_down(KeyCode::KeyW, true); // Alt held: ignored
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    steps
}

fn pointer_steer() -> Vec<Step> {
    let mut c = camera();
    let mut controls = controls();
    controls.roll_speed = 0.4;
    controls.movement_speed = 2.0;
    let mut steps = vec![Step::of(&c, None)];

    controls.pointer_move(620.0, 130.0);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_down(MouseButton::Left);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_move(150.0, 420.0);
    controls.pointer_down(MouseButton::Right);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_up(MouseButton::Left);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_cancel();
    controls.pointer_move(320.0, 180.0); // dead centre: no turn
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    steps
}

fn drag_to_look() -> Vec<Step> {
    let mut c = camera();
    let mut controls = controls();
    controls.drag_to_look = true;
    controls.roll_speed = 0.5;
    let mut steps = vec![Step::of(&c, None)];

    controls.pointer_move(700.0, 50.0); // not dragging: ignored
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.pointer_down(MouseButton::Left);
    controls.pointer_move(90.0, 380.0);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.pointer_up(MouseButton::Left);
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.pointer_down(MouseButton::Right);
    controls.pointer_move(500.0, 100.0);
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    controls.pointer_cancel();
    controls.pointer_move(10.0, 10.0); // cancelled: ignored
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    steps
}

fn auto_forward() -> Vec<Step> {
    let mut c = camera();
    let mut controls = controls();
    controls.auto_forward = true;
    controls.movement_speed = 3.0;
    let mut steps = vec![Step::of(&c, None)];

    // autoForward only reaches the move vector through
    // update_movement_vector().
    controls.key_down(KeyCode::KeyR, false);
    controls.key_up(KeyCode::KeyR);
    frames(&mut controls, &mut c, &mut steps, 3, 0.25);

    controls.key_down(KeyCode::KeyS, false);
    frames(&mut controls, &mut c, &mut steps, 3, 0.25);

    controls.key_up(KeyCode::KeyS);
    frames(&mut controls, &mut c, &mut steps, 3, 0.25);

    steps
}

fn disabled() -> Vec<Step> {
    let mut c = camera();
    let mut controls = controls();
    let mut steps = vec![Step::of(&c, None)];

    controls.key_down(KeyCode::KeyD, false);
    controls.enabled = false;
    controls.key_down(KeyCode::KeyW, false);
    controls.key_up(KeyCode::KeyD);
    controls.pointer_move(0.0, 0.0);
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.enabled = true;
    frames(&mut controls, &mut c, &mut steps, 2, DT);
    frames(&mut controls, &mut c, &mut steps, 3, 0.0002);
    frames(&mut controls, &mut c, &mut steps, 1, 0.5);

    steps
}

fn scenarios() -> Vec<(&'static str, Vec<Step>)> {
    vec![
        ("keys", keys()),
        ("pointer_steer", pointer_steer()),
        ("drag_to_look", drag_to_look()),
        ("auto_forward", auto_forward()),
        ("disabled", disabled()),
    ]
}

fn parse(text: &str, origin: &str) -> serde_json::Map<String, serde_json::Value> {
    let json: serde_json::Value =
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{origin} is not JSON: {e}"));
    json.as_object()
        .unwrap_or_else(|| panic!("{origin} is not a JSON object"))
        .clone()
}

/// Under `CI` a missing checkout or node is a failure, as in
/// `tests/loaders_webp.rs`; locally it is a skip.
fn skip(reason: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    if std::env::var_os("CI").is_some() {
        panic!("the three.js re-run cannot run: {reason}");
    }
    eprintln!("not re-running three.js: {reason}");
    None
}

/// Runs the reference script into a scratch file, or says why it cannot.
fn live_reference() -> Option<serde_json::Map<String, serde_json::Value>> {
    let three = three_rs::testing::three_js_dir();
    if !three.join("examples/jsm/controls/FlyControls.js").exists() {
        return skip(&format!(
            "no checkout at {} (set THREE_JS_DIR)",
            three.display()
        ));
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("fly_controls.json");
    let output = match Command::new("node")
        .arg(root.join(SCRIPT))
        .arg(&three)
        .arg(&out)
        .output()
    {
        Ok(output) => output,
        Err(error) => return skip(&format!("cannot run node ({error})")),
    };
    assert!(
        output.status.success(),
        "the reference script failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::fs::read_to_string(&out).expect("the reference script's output");
    Some(parse(&text, "the reference script's output"))
}

fn assert_matches(origin: &str, reference: &serde_json::Map<String, serde_json::Value>) {
    const FIELDS: [&str; 7] = [
        "position.x",
        "position.y",
        "position.z",
        "quaternion.x",
        "quaternion.y",
        "quaternion.z",
        "quaternion.w",
    ];

    let ported = scenarios();
    assert_eq!(
        reference.len(),
        ported.len(),
        "{origin} has {} scenarios, the test {}",
        reference.len(),
        ported.len()
    );

    for (name, actual) in ported {
        let expected: Vec<Step> = reference[name]
            .as_array()
            .unwrap_or_else(|| panic!("{origin} has no scenario `{name}`"))
            .iter()
            .map(Step::from_json)
            .collect();
        assert_eq!(
            expected.len(),
            actual.len(),
            "{name}: {origin} recorded {} steps, the port {}",
            expected.len(),
            actual.len()
        );

        for (index, (want, got)) in expected.iter().zip(&actual).enumerate() {
            let values = want
                .position
                .iter()
                .zip(&got.position)
                .chain(want.quaternion.iter().zip(&got.quaternion));
            for (field, (w, g)) in FIELDS.iter().zip(values) {
                assert!(
                    (w - g).abs() <= TOLERANCE,
                    "{name} step {index}: {field} is {g}, {origin} says {w} (off by {:e})\n  \
                     three.js: {want:?}\n  the port: {got:?}",
                    (w - g).abs()
                );
            }
            assert_eq!(
                want.changed, got.changed,
                "{name} step {index}: `change` dispatched is {:?} in {origin}, \
                 update() returned {:?}",
                want.changed, got.changed
            );
        }
    }
}

#[test]
fn matches_the_committed_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert_matches(FIXTURE, &parse(&text, FIXTURE));
}

#[test]
fn matches_three_js_run_now() {
    if let Some(reference) = live_reference() {
        assert_matches("three.js run now", &reference);
    }
}

#[test]
fn defaults_are_three_js_defaults() {
    let controls = FlyControls::new();
    assert!(controls.enabled);
    assert_eq!(controls.movement_speed, 1.0);
    assert_eq!(controls.roll_speed, 0.005);
    assert!(!controls.drag_to_look);
    assert!(!controls.auto_forward);
    assert_eq!(controls.move_vector(), Vector3::ZERO);
    assert_eq!(controls.rotation_vector(), Vector3::ZERO);
}

/// Writing `move_state` directly and calling the two update methods is the
/// same as the handlers doing it.
#[test]
fn move_state_drives_the_vectors() {
    let mut controls = FlyControls::new();
    controls.move_state.forward = 1.0;
    controls.move_state.right = 1.0;
    controls.move_state.yaw_left = 0.25;
    controls.move_state.roll_right = 1.0;
    controls.update_movement_vector();
    controls.update_rotation_vector();
    assert_eq!(controls.move_vector(), Vector3::new(1.0, 0.0, -1.0));
    assert_eq!(controls.rotation_vector(), Vector3::new(0.0, 0.25, -1.0));
}
