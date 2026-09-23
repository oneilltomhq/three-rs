//! Grades [`three_rs::addons::controls::OrbitControls`] against three.js'
//! `examples/jsm/controls/OrbitControls.js` itself.
//!
//! `tools/orbit_controls_reference.mjs` runs the JS class under node and
//! prints, for a dozen scripted event sequences, what it did to the camera.
//! This file replays the identical sequences through the port and asserts
//! every number matches to 1e-9 — the ladder's approach for the renderer
//! (three.js' own comparator, never a second implementation) applied to a
//! class that has no pixels.
//!
//! Skipped, with a note, when there is no three.js checkout or no node; the
//! same condition the rung harness skips on.

use std::path::Path;
use std::process::Command;

use three_rs::addons::controls::{
    Key, KeyEvent, MouseButton, OrbitControls, PointerEvent, WheelDelta, WheelEvent,
};
use three_rs::cameras::PerspectiveCamera;

/// The element the reference script stubs, and the frame the ladder grades.
const ELEMENT_WIDTH: f64 = 800.0;
const ELEMENT_HEIGHT: f64 = 500.0;

/// How close the port has to be. Both sides are f64 doing the same operations
/// in the same order, so the only differences are the last bits of
/// reassociation; anything structural is far larger than this.
const TOLERANCE: f64 = 1e-9;

/// A recorded step: one entry of the reference script's per-scenario array.
#[derive(Debug, Clone, PartialEq)]
struct Step {
    position: [f64; 3],
    quaternion: [f64; 4],
    target: [f64; 3],
    distance: f64,
    polar: f64,
    azimuth: f64,
    changed: Option<bool>,
}

impl Step {
    /// What the port's state reads as, in the reference's shape.
    fn of(controls: &OrbitControls, camera: &PerspectiveCamera, changed: Option<bool>) -> Self {
        let object = camera.node.borrow();
        Self {
            position: [object.position.x, object.position.y, object.position.z],
            quaternion: [
                object.quaternion.x,
                object.quaternion.y,
                object.quaternion.z,
                object.quaternion.w,
            ],
            target: [controls.target.x, controls.target.y, controls.target.z],
            distance: controls.get_distance(camera),
            polar: controls.get_polar_angle(),
            azimuth: controls.get_azimuthal_angle(),
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
        let number = |key: &str| -> f64 {
            value[key]
                .as_f64()
                .unwrap_or_else(|| panic!("reference step has no number `{key}`"))
        };

        let position = array("position");
        let quaternion = array("quaternion");
        let target = array("target");
        Self {
            position: [position[0], position[1], position[2]],
            quaternion: [quaternion[0], quaternion[1], quaternion[2], quaternion[3]],
            target: [target[0], target[1], target[2]],
            distance: number("distance"),
            polar: number("polar"),
            azimuth: number("azimuth"),
            changed: value["changed"].as_bool(),
        }
    }
}

/// The camera every scenario starts from, matching the reference script's.
fn camera() -> PerspectiveCamera {
    let mut camera = PerspectiveCamera::new(45.0, ELEMENT_WIDTH / ELEMENT_HEIGHT, 0.25, 200.0);
    camera.node.borrow_mut().position.set(3.0, 4.0, 5.0);
    camera.update_matrix_world();
    camera
}

fn controls_on(camera: &mut PerspectiveCamera) -> OrbitControls {
    let mut controls = OrbitControls::new(camera);
    controls.set_element_size(ELEMENT_WIDTH, ELEMENT_HEIGHT);
    controls
}

fn pointer(button: MouseButton, x: f64, y: f64) -> PointerEvent {
    PointerEvent {
        pointer_id: 1,
        button,
        client_x: x,
        client_y: y,
        ..Default::default()
    }
}

fn wheel(x: f64, y: f64, delta_y: f64, delta_mode: WheelDelta, ctrl_key: bool) -> WheelEvent {
    WheelEvent {
        client_x: x,
        client_y: y,
        delta_y,
        delta_mode,
        ctrl_key,
    }
}

fn arrow(key: Key, shift_key: bool) -> KeyEvent {
    KeyEvent {
        key,
        ctrl_key: false,
        meta_key: false,
        shift_key,
    }
}

/// Runs the reference script and returns its scenarios.
fn reference() -> Option<serde_json::Map<String, serde_json::Value>> {
    let three = three_rs::testing::three_js_dir();
    if !three
        .join("examples/jsm/controls/OrbitControls.js")
        .exists()
    {
        eprintln!(
            "skipping: no three.js checkout at {} (set THREE_JS_DIR)",
            three.display()
        );
        return None;
    }

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/orbit_controls_reference.mjs");
    let output = match Command::new("node").arg(&script).arg(&three).output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("skipping: cannot run node ({error})");
            return None;
        }
    };
    assert!(
        output.status.success(),
        "the reference script failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
            .expect("the reference script's JSON");
    Some(json.as_object().expect("a JSON object").clone())
}

/// Asserts the port's steps are the reference's, and says which number of
/// which step of which scenario is not if they are ever not.
fn assert_matches(name: &str, expected: &serde_json::Value, actual: &[Step]) {
    let expected: Vec<Step> = expected
        .as_array()
        .unwrap_or_else(|| panic!("{name}: the reference has no steps"))
        .iter()
        .map(Step::from_json)
        .collect();

    assert_eq!(
        expected.len(),
        actual.len(),
        "{name}: the reference recorded {} steps, the port {}",
        expected.len(),
        actual.len()
    );

    for (index, (want, got)) in expected.iter().zip(actual).enumerate() {
        let check = |field: &str, want: f64, got: f64| {
            assert!(
                (want - got).abs() <= TOLERANCE,
                "{name} step {index}: {field} is {got}, three.js says {want} \
                 (off by {:e})\n  three.js: {want:?}\n  the port: {got:?}",
                (want - got).abs()
            );
        };

        for (axis, component) in ["x", "y", "z"].iter().enumerate() {
            check(
                &format!("position.{component}"),
                want.position[axis],
                got.position[axis],
            );
            check(
                &format!("target.{component}"),
                want.target[axis],
                got.target[axis],
            );
        }
        for (axis, component) in ["x", "y", "z", "w"].iter().enumerate() {
            check(
                &format!("quaternion.{component}"),
                want.quaternion[axis],
                got.quaternion[axis],
            );
        }
        check("distance", want.distance, got.distance);
        check("polar angle", want.polar, got.polar);
        check("azimuthal angle", want.azimuth, got.azimuth);

        assert_eq!(
            want.changed, got.changed,
            "{name} step {index}: update() returned {:?}, three.js returns {:?}",
            got.changed, want.changed
        );
    }

    println!("{name}: {} steps match three.js", actual.len());
}

/// 1. A plain left drag: rotate, no damping.
fn rotate() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.pointer_down(&mut camera, &pointer(MouseButton::Left, 400.0, 250.0));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 460.0, 220.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 500.0, 300.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    let changed = controls.update(&mut camera, None);
    steps.push(Step::of(&controls, &camera, Some(changed)));

    steps
}

/// 2. The same drag with damping, then six updates with no input.
fn rotate_damped() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.enable_damping = true;
    controls.damping_factor = 0.1;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.pointer_down(&mut camera, &pointer(MouseButton::Left, 400.0, 250.0));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 470.0, 210.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    for _ in 0..6 {
        let changed = controls.update(&mut camera, None);
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, Some(changed)));
    }

    steps
}

/// 3. The wheel, against both distance clamps and through every `deltaMode`.
fn wheel_dolly() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.min_distance = 4.0;
    controls.max_distance = 9.0;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    for _ in 0..8 {
        controls.wheel(
            &mut camera,
            &wheel(400.0, 250.0, -120.0, WheelDelta::Pixel, false),
        );
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    for _ in 0..12 {
        controls.wheel(
            &mut camera,
            &wheel(400.0, 250.0, 120.0, WheelDelta::Pixel, false),
        );
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    for event in [
        wheel(400.0, 250.0, -3.0, WheelDelta::Line, false),
        wheel(400.0, 250.0, -1.0, WheelDelta::Page, false),
        wheel(400.0, 250.0, -10.0, WheelDelta::Pixel, true),
    ] {
        controls.wheel(&mut camera, &event);
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    steps
}

/// A right drag, panning in one of the two modes.
fn pan(screen_space_panning: bool) -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.screen_space_panning = screen_space_panning;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.pointer_down(&mut camera, &pointer(MouseButton::Right, 400.0, 250.0));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Right, 480.0, 190.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    controls.pointer_move(&mut camera, &pointer(MouseButton::Right, 420.0, 300.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    steps
}

/// 6. Auto-rotate, with and without a delta, and with a negative speed.
fn auto_rotate() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.auto_rotate = true;
    controls.auto_rotate_speed = 1.0;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    for _ in 0..4 {
        let changed = controls.update(&mut camera, None);
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, Some(changed)));
    }

    for _ in 0..4 {
        let changed = controls.update(&mut camera, Some(1.0 / 60.0));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, Some(changed)));
    }

    controls.auto_rotate_speed = -0.1;
    for _ in 0..2 {
        let changed = controls.update(&mut camera, Some(1.0 / 60.0));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, Some(changed)));
    }

    steps
}

/// 7. Dragged hard into both ends of a polar clamp.
fn polar_clamp() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.min_polar_angle = std::f64::consts::PI / 4.0;
    controls.max_polar_angle = std::f64::consts::PI / 1.5;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.pointer_down(&mut camera, &pointer(MouseButton::Left, 400.0, 250.0));
    for i in 0..5 {
        let y = 250.0 - f64::from(i + 1) * 120.0;
        controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 400.0, y));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    for i in 0..10 {
        let y = -600.0 + f64::from(i + 1) * 120.0;
        controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 400.0, y));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    steps
}

/// 8. The azimuth clamp, whose `update()` has two branches.
fn azimuth_clamp() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.min_azimuth_angle = -0.3;
    controls.max_azimuth_angle = 0.9;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.pointer_down(&mut camera, &pointer(MouseButton::Left, 400.0, 250.0));
    for i in 0..8 {
        let x = 400.0 + f64::from(i + 1) * 140.0;
        controls.pointer_move(&mut camera, &pointer(MouseButton::Left, x, 250.0));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    steps
}

/// 9. The arrow keys: pan, and pan faster with a modifier.
fn keys() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    let mut steps = vec![Step::of(&controls, &camera, None)];

    let arrows = [
        Key::ArrowLeft,
        Key::ArrowUp,
        Key::ArrowRight,
        Key::ArrowDown,
    ];
    for shift_key in [false, true] {
        for key in arrows {
            controls.key(&mut camera, &arrow(key, shift_key));
            camera.update_matrix_world();
            steps.push(Step::of(&controls, &camera, None));
        }
    }

    steps
}

/// 10. A modified left drag (a pan) and a middle drag (a dolly).
fn modified_buttons() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    let mut steps = vec![Step::of(&controls, &camera, None)];

    let shifted = |x: f64, y: f64| PointerEvent {
        shift_key: true,
        ..pointer(MouseButton::Left, x, y)
    };
    controls.pointer_down(&mut camera, &shifted(400.0, 250.0));
    controls.pointer_move(&mut camera, &shifted(470.0, 300.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    controls.pointer_up(&pointer(MouseButton::Left, 470.0, 300.0));
    controls.pointer_down(&mut camera, &pointer(MouseButton::Middle, 400.0, 250.0));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Middle, 400.0, 340.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Middle, 400.0, 180.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    steps
}

/// 11. `zoomToCursor`: down the pointer ray, not towards the target.
fn zoom_to_cursor() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    controls.zoom_to_cursor = true;
    let mut steps = vec![Step::of(&controls, &camera, None)];

    for (x, y) in [(200.0, 120.0), (650.0, 400.0), (400.0, 250.0)] {
        controls.wheel(&mut camera, &wheel(x, y, -120.0, WheelDelta::Pixel, false));
        camera.update_matrix_world();
        steps.push(Step::of(&controls, &camera, None));
    }

    steps
}

/// 12. `saveState()` / `reset()`.
fn save_and_reset() -> Vec<Step> {
    let mut camera = camera();
    let mut controls = controls_on(&mut camera);
    let mut steps = vec![Step::of(&controls, &camera, None)];

    controls.save_state(&camera);

    controls.pointer_down(&mut camera, &pointer(MouseButton::Left, 400.0, 250.0));
    controls.pointer_move(&mut camera, &pointer(MouseButton::Left, 520.0, 160.0));
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    controls.reset(&mut camera);
    camera.update_matrix_world();
    steps.push(Step::of(&controls, &camera, None));

    steps
}

#[test]
fn the_port_does_what_three_js_does() {
    let Some(reference) = reference() else {
        return;
    };

    let scenarios: Vec<(&str, Vec<Step>)> = vec![
        ("rotate", rotate()),
        ("rotate_damped", rotate_damped()),
        ("wheel_dolly", wheel_dolly()),
        ("pan_screen_space", pan(true)),
        ("pan_world_space", pan(false)),
        ("auto_rotate", auto_rotate()),
        ("polar_clamp", polar_clamp()),
        ("azimuth_clamp", azimuth_clamp()),
        ("keys", keys()),
        ("modified_buttons", modified_buttons()),
        ("zoom_to_cursor", zoom_to_cursor()),
        ("save_and_reset", save_and_reset()),
    ];

    // Neither side may quietly stop testing something: every scenario the
    // reference records has to be replayed here, and vice versa.
    let mut replayed: Vec<&str> = scenarios.iter().map(|(name, _)| *name).collect();
    let mut recorded: Vec<&str> = reference.keys().map(String::as_str).collect();
    replayed.sort_unstable();
    recorded.sort_unstable();
    assert_eq!(
        recorded, replayed,
        "the reference script and this test disagree about which scenarios exist"
    );

    for (name, steps) in &scenarios {
        assert_matches(name, &reference[*name], steps);
    }
}
