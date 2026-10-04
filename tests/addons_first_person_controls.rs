//! Grades [`three_rs::addons::controls::FirstPersonControls`] against three.js'
//! `examples/jsm/controls/FirstPersonControls.js` itself.
//!
//! `tools/first_person_controls_reference.mjs` runs the JS class under node
//! through seven scripted input sequences and writes the camera's position and
//! quaternion after every step to `tests/fixtures/first_person_controls.json`.
//! This file replays the identical sequences through the port and asserts
//! every number matches to [`TOLERANCE`].
//!
//! The fixture is committed, so the comparison runs everywhere. Where there is
//! also a three.js checkout and node, the script is run again into a scratch
//! file and the port is held to that too, which catches a fixture that has
//! gone stale against the pinned three.js.

use std::path::{Path, PathBuf};
use std::process::Command;

use three_rs::addons::controls::{FirstPersonControls, KeyCode, MouseButton};
use three_rs::cameras::PerspectiveCamera;
use three_rs::core::Object3D;
use three_rs::math::Vector3;

/// How close the port has to be. Both sides are f64 doing the same operations
/// in the same order; the slack is for the last bits of `sin` / `cos` / `acos`
/// differing between V8 and Rust's libm, compounded over a few dozen frames.
const TOLERANCE: f64 = 1e-9;

const DT: f64 = 1.0 / 60.0;

const SCRIPT: &str = "tools/first_person_controls_reference.mjs";
const FIXTURE: &str = "tests/fixtures/first_person_controls.json";

#[derive(Debug, Clone, Copy)]
struct Step {
    position: [f64; 3],
    quaternion: [f64; 4],
}

impl Step {
    fn of(camera: &PerspectiveCamera) -> Self {
        let object = camera.node.borrow();
        let (p, q) = (object.position, object.quaternion);
        Self {
            position: [p.x, p.y, p.z],
            quaternion: [q.x, q.y, q.z, q.w],
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
        }
    }
}

/// The camera every scenario starts from, matching the reference script's.
fn camera() -> PerspectiveCamera {
    let mut camera = PerspectiveCamera::new(45.0, 800.0 / 500.0, 0.25, 200.0);
    camera.node.borrow_mut().position.set(3.0, 4.0, 5.0);
    camera.look_at(&Vector3::new(-1.0, 1.5, 0.5));
    camera.update_matrix_world();
    camera
}

/// Runs `update( delta )` `n` times, recording after each.
fn frames(
    controls: &mut FirstPersonControls,
    camera: &mut PerspectiveCamera,
    steps: &mut Vec<Step>,
    n: usize,
    delta: f64,
) {
    for _ in 0..n {
        controls.update(&camera.node, delta);
        steps.push(Step::of(camera));
    }
}

fn keys() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    controls.movement_speed = 3.0;
    let mut steps = vec![Step::of(&c)];

    controls.key_down(KeyCode::KeyW);
    frames(&mut controls, &mut c, &mut steps, 6, DT);

    controls.key_down(KeyCode::KeyD);
    frames(&mut controls, &mut c, &mut steps, 6, DT);

    controls.key_up(KeyCode::KeyW);
    controls.key_up(KeyCode::KeyD);
    controls.key_down(KeyCode::KeyR);
    controls.key_down(KeyCode::ArrowDown);
    controls.key_down(KeyCode::ArrowLeft);
    frames(&mut controls, &mut c, &mut steps, 6, DT);

    controls.key_up(KeyCode::KeyR);
    controls.key_up(KeyCode::ArrowDown);
    controls.key_up(KeyCode::ArrowLeft);
    controls.key_down(KeyCode::KeyF);
    controls.key_down(KeyCode::KeyQ); // not bound: ignored
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    controls.key_up(KeyCode::KeyF);
    frames(&mut controls, &mut c, &mut steps, 8, DT);

    steps
}

fn mouse_look() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    controls.look_speed = 0.1;
    let mut steps = vec![Step::of(&c)];

    controls.pointer_down(MouseButton::Left, 400.0, 250.0);
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.pointer_move(460.0, 220.0);
    frames(&mut controls, &mut c, &mut steps, 8, DT);

    controls.pointer_move(300.0, 330.0);
    frames(&mut controls, &mut c, &mut steps, 8, DT);

    controls.pointer_up(MouseButton::Left);
    assert!(!controls.mouse_drag_on());
    controls.pointer_move(900.0, 900.0); // no drag: ignored
    frames(&mut controls, &mut c, &mut steps, 8, DT);

    steps
}

fn constrain_vertical() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    controls.look_speed = 0.5;
    controls.constrain_vertical = true;
    controls.vertical_min = 1.0;
    controls.vertical_max = 2.0;
    let mut steps = vec![Step::of(&c)];

    controls.pointer_down(MouseButton::Right, 400.0, 250.0);
    controls.pointer_move(350.0, -400.0);
    frames(&mut controls, &mut c, &mut steps, 12, 0.05);

    controls.pointer_move(450.0, 900.0);
    frames(&mut controls, &mut c, &mut steps, 12, 0.05);

    controls.pointer_up(MouseButton::Right);
    controls.pointer_down(MouseButton::Middle, 100.0, 100.0);
    controls.pointer_move(140.0, 60.0);
    frames(&mut controls, &mut c, &mut steps, 6, 0.05);
    controls.pointer_up(MouseButton::Middle);
    frames(&mut controls, &mut c, &mut steps, 3, 0.05);

    steps
}

fn auto_forward_height() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    controls.auto_forward = true;
    controls.height_speed = true;
    controls.height_coef = 2.0;
    controls.height_min = 1.0;
    controls.height_max = 10.0;
    controls.movement_speed = 5.0;
    controls.damping_factor = 0.3;
    controls.look_vertical = false;
    controls.look_speed = 0.2;
    let mut steps = vec![Step::of(&c)];

    frames(&mut controls, &mut c, &mut steps, 6, DT);

    controls.key_down(KeyCode::KeyS);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_down(MouseButton::Left, 400.0, 250.0); // key held: only looks
    controls.pointer_move(330.0, 300.0);
    frames(&mut controls, &mut c, &mut steps, 6, DT);

    controls.key_up(KeyCode::KeyS);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.pointer_up(MouseButton::Left);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    // A right drag moves back at movementSpeed, not the height-scaled
    // forwardSpeed.
    controls.pointer_down(MouseButton::Right, 400.0, 250.0);
    controls.pointer_move(430.0, 260.0);
    frames(&mut controls, &mut c, &mut steps, 6, DT);
    controls.pointer_up(MouseButton::Right);

    // W drives at forwardSpeed; with R it climbs past heightMax, where
    // forwardSpeed stops growing.
    controls.key_down(KeyCode::KeyW);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    controls.key_down(KeyCode::KeyR);
    frames(&mut controls, &mut c, &mut steps, 24, 0.1);
    assert!(steps.iter().any(|s| s.position[1] > controls.height_max));

    // F, and autoForward along the downward look, drop it below heightMin.
    controls.key_up(KeyCode::KeyW);
    controls.key_up(KeyCode::KeyR);
    controls.key_down(KeyCode::KeyF);
    frames(&mut controls, &mut c, &mut steps, 12, 0.1);
    assert!(steps.iter().any(|s| s.position[1] < controls.height_min));

    controls.key_up(KeyCode::KeyF);
    frames(&mut controls, &mut c, &mut steps, 4, DT);

    steps
}

fn touch() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    controls.look_speed = 0.05;
    let mut steps = vec![Step::of(&c)];

    controls.touch_start(200.0, 200.0);
    controls.pointer_move(260.0, 180.0);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.touch_start(500.0, 300.0);
    controls.pointer_move(470.0, 330.0);
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.touch_end();
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    controls.touch_end();
    frames(&mut controls, &mut c, &mut steps, 5, DT);

    steps
}

fn look_at() -> Vec<Step> {
    let mut c = camera();
    let mut controls = FirstPersonControls::new(&c.node);
    let mut steps = vec![Step::of(&c)];

    controls.look_at(&c.node, &Vector3::new(7.0, -2.0, 1.0));
    steps.push(Step::of(&c));
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.key_down(KeyCode::KeyW);
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    controls.look_at(&c.node, &Vector3::new(-4.0, 6.0, -3.0));
    steps.push(Step::of(&c));
    frames(&mut controls, &mut c, &mut steps, 3, DT);

    controls.enabled = false;
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    controls.enabled = true;
    frames(&mut controls, &mut c, &mut steps, 2, DT);

    steps
}

fn parented() -> Vec<Step> {
    let group = Object3D::new_node();
    group.borrow_mut().position.set(1.0, 2.0, -1.0);
    group
        .borrow_mut()
        .quaternion
        .set(0.1, -0.25, 0.08, 0.96)
        .normalize();
    let mut c = camera();
    group.add(&c.node);
    group.update_matrix_world(false);
    let mut controls = FirstPersonControls::new(&c.node);
    controls.look_speed = 0.1;
    let mut steps = vec![Step::of(&c)];

    controls.look_at(&c.node, &Vector3::new(0.0, 0.0, 0.0));
    steps.push(Step::of(&c));

    controls.key_down(KeyCode::KeyA);
    controls.pointer_down(MouseButton::Left, 400.0, 250.0);
    controls.pointer_move(380.0, 300.0);
    frames(&mut controls, &mut c, &mut steps, 8, DT);

    steps
}

fn scenarios() -> Vec<(&'static str, Vec<Step>)> {
    vec![
        ("keys", keys()),
        ("mouse_look", mouse_look()),
        ("constrain_vertical", constrain_vertical()),
        ("auto_forward_height", auto_forward_height()),
        ("touch", touch()),
        ("look_at", look_at()),
        ("parented", parented()),
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
    if !three
        .join("examples/jsm/controls/FirstPersonControls.js")
        .exists()
    {
        return skip(&format!(
            "no checkout at {} (set THREE_JS_DIR)",
            three.display()
        ));
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("first_person_controls.json");
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
    let c = camera();
    let controls = FirstPersonControls::new(&c.node);
    assert!(controls.enabled);
    assert_eq!(controls.movement_speed, 1.0);
    assert_eq!(controls.look_speed, 0.005);
    assert_eq!(controls.damping_factor, 0.1);
    assert!(controls.look_vertical);
    assert!(!controls.auto_forward);
    assert!(!controls.height_speed);
    assert_eq!(controls.height_coef, 1.0);
    assert_eq!(controls.height_min, 0.0);
    assert_eq!(controls.height_max, 1.0);
    assert!(!controls.constrain_vertical);
    assert_eq!(controls.vertical_min, 0.0);
    assert_eq!(controls.vertical_max, std::f64::consts::PI);
    assert!(!controls.mouse_drag_on());
}
