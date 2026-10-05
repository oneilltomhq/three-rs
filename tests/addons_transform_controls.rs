//! Grades [`three_rs::addons::controls::TransformControls`] against three.js'
//! `examples/jsm/controls/TransformControls.js` itself.
//!
//! `tools/transform_controls_reference.mjs` runs the JS class under node
//! through eighteen scripted scenarios and writes each one's setup, actions
//! and results to `tests/fixtures/transform_controls.json`: after every
//! step, the events dispatched, the object's transform, `axis`, `mode`,
//! `dragging`, `rotationAngle`, which handles of the current mode are visible
//! and highlighted and the plane's orientation, and on marked steps every
//! handle's transform, colour and opacity, the root's and the plane's world
//! matrices and the controls' working vectors. This file is an interpreter
//! of those actions: it replays them through the port and asserts every
//! number matches to [`TOLERANCE`]. The fixture also holds the gizmo,
//! picker and helper graph three builds, which
//! [`the_gizmo_graph_is_three_js_graph`] compares node for node.
//!
//! The fixture is committed, so the comparison runs everywhere. Where there is
//! also a three.js checkout and node, the script is run again into a scratch
//! file and the port is held to that too, which catches a fixture that has
//! gone stale against the pinned three.js.
//!
//! # Branches no scenario reaches
//!
//! - The `console.error` for an attached object with no parent (three then
//!   throws in `pointerDown`, so it cannot be scripted past).
//!
//! Two that look unreachable are scripted: `pointerMove` with no plane hit
//! (`translate_world`, a ray above the horizon mid-way through an `XZ` drag,
//! when the plane is horizontal and finite), and rotating with an `axis` left
//! over from translate (`rotate_stale_axis`, an `XY` hovered there, then
//! `setMode('rotate')` and a drag, which applies the stale `rotationAxis` and
//! `rotationAngle`).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use three_rs::addons::controls::{
    Mode, Pointer, PointerType, Space, TransformCamera, TransformControls, TransformControlsEvent,
    TransformPointerEvent,
};
use three_rs::cameras::{OrthographicCamera, PerspectiveCamera};
use three_rs::core::{Object3D, ObjectRef};
use three_rs::materials::Side;
use three_rs::math::{Color, Quaternion, Vector3, Vector4};

/// How close the port has to be, relative to the value's size where that is
/// above 1. Both sides are f64 doing the same operations in the same order;
/// the slack is for the last bits of the trigonometry differing between V8
/// and Rust's libm, and for the fixture's twelve significant digits.
const TOLERANCE: f64 = 1e-9;

const SCRIPT: &str = "tools/transform_controls_reference.mjs";
const FIXTURE: &str = "tests/fixtures/transform_controls.json";

fn close(want: f64, got: f64) -> bool {
    (want - got).abs() <= TOLERANCE * want.abs().max(1.0) || (want.is_nan() && got.is_nan())
}

fn num(value: &Value) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("expected a number, the fixture has {value}"))
}

fn nums(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected an array, the fixture has {value}"))
        .iter()
        .map(num)
        .collect()
}

fn mode(name: &str) -> Mode {
    match name {
        "translate" => Mode::Translate,
        "rotate" => Mode::Rotate,
        "scale" => Mode::Scale,
        _ => panic!("unknown mode {name}"),
    }
}

fn event_name(event: &TransformControlsEvent) -> String {
    match event {
        TransformControlsEvent::MouseDown(m) => format!("mouseDown:{}", m.as_str()),
        TransformControlsEvent::MouseUp(m) => format!("mouseUp:{}", m.as_str()),
        other => other.type_name(),
    }
}

/// Sets an object's transform the way the script does:
/// `position.fromArray( p )`, `quaternion.fromArray( q ).normalize()`,
/// `scale.fromArray( s )`.
fn transform(object: &ObjectRef, t: &Value) {
    let mut o = object.borrow_mut();
    let p = nums(&t["p"]);
    let q = nums(&t["q"]);
    let s = nums(&t["s"]);
    o.position = Vector3::new(p[0], p[1], p[2]);
    let mut quaternion = Quaternion::new(q[0], q[1], q[2], q[3]);
    quaternion.normalize();
    o.quaternion = quaternion;
    o.scale = Vector3::new(s[0], s[1], s[2]);
}

/// The scene a scenario runs in: the object (in its parent, if any) first,
/// the helper (in its parent, if any) last, as in the script.
struct World {
    scene: ObjectRef,
    object: ObjectRef,
    controls: TransformControls,
    /// `materialLib.active.color`, for the highlight string.
    active: Color,
}

impl World {
    fn new(setup: &Value) -> Self {
        let scene = Object3D::new_node();
        let object = Object3D::new_node();
        transform(&object, &setup["object"]);
        let object_parent = if setup["objectParent"].is_null() {
            scene.add(&object);
            None
        } else {
            let parent = Object3D::new_node();
            transform(&parent, &setup["objectParent"]);
            parent.add(&object);
            scene.add(&parent);
            Some(parent)
        };

        let mut controls = TransformControls::new();
        let element = nums(&setup["element"]);
        controls.set_element_size(element[0], element[1]);
        let helper = controls.get_helper().clone();
        match &setup["helperParent"] {
            Value::Null => {
                scene.add(&helper);
            }
            Value::String(s) if s == "objectParent" => {
                object_parent
                    .as_ref()
                    .expect("the helper's parent is the object's")
                    .add(&helper);
            }
            t => {
                let group = Object3D::new_node();
                transform(&group, t);
                group.add(&helper);
                scene.add(&group);
            }
        }
        controls.attach(&object);

        Self {
            scene,
            object,
            controls,
            active: Color::from_hex(0xffff00),
        }
    }

    fn render(&mut self, camera: &mut impl TransformCamera) {
        self.scene.update_matrix_world(false);
        self.controls.update(camera);
    }

    /// The root's `[ gizmo, plane ]` and the gizmo's nine groups, gizmo,
    /// picker and helper, each translate, rotate and scale.
    fn group(&self, kind: usize, mode: Mode) -> ObjectRef {
        let index = match mode {
            Mode::Translate => 0,
            Mode::Rotate => 1,
            Mode::Scale => 2,
        };
        self.controls.get_helper().children()[0].children()[kind * 3 + index].clone()
    }

    fn plane(&self) -> ObjectRef {
        self.controls.get_helper().children()[1].clone()
    }

    /// Runs one step's action and returns the events it dispatched, or
    /// `None` for an assignment to a plain field, which dispatches nothing in
    /// the port and is not compared.
    fn act(&mut self, action: &Value, camera: &mut impl TransformCamera) -> Option<Vec<String>> {
        let c = &mut self.controls;
        let event = || TransformPointerEvent {
            pointer_type: match action["pointerType"].as_str() {
                Some("mouse") => PointerType::Mouse,
                Some("pen") => PointerType::Pen,
                Some("touch") => PointerType::Touch,
                other => panic!("pointer type {other:?}"),
            },
            button: num(&action["button"]) as i32,
            x: num(&action["x"]),
            y: num(&action["y"]),
        };
        let snap = |v: &Value| v.as_f64();
        let events = match action["op"].as_str().expect("an op") {
            "down" => c.on_pointer_down(&event(), camera),
            "move" => c.on_pointer_move(&event(), camera),
            "up" => c.on_pointer_up(&event()),
            "api" => {
                let pointer = (!action["pointer"].is_null()).then(|| Pointer {
                    x: num(&action["pointer"]["x"]),
                    y: num(&action["pointer"]["y"]),
                    button: num(&action["pointer"]["button"]) as i32,
                });
                let pointer = pointer.as_ref();
                match action["name"].as_str() {
                    Some("hover") => c.pointer_hover(pointer, camera),
                    Some("down") => c.pointer_down(pointer, camera),
                    Some("move") => c.pointer_move(pointer, camera),
                    Some("up") => c.pointer_up(pointer),
                    other => panic!("api {other:?}"),
                }
            }
            "set" => {
                let value = &action["value"];
                let flag = || value.as_bool().expect("a boolean");
                match action["key"].as_str().expect("a key") {
                    "mode" => c.set_mode(mode(value.as_str().expect("a mode"))),
                    "space" => c.set_space(match value.as_str() {
                        Some("local") => Space::Local,
                        _ => Space::World,
                    }),
                    "size" => c.set_size(num(value)),
                    "translationSnap" => c.set_translation_snap(snap(value)),
                    "rotationSnap" => c.set_rotation_snap(snap(value)),
                    "scaleSnap" => c.set_scale_snap(snap(value)),
                    key => {
                        match key {
                            "viewport" => {
                                c.viewport = (!value.is_null()).then(|| {
                                    let v = nums(value);
                                    Vector4::new(v[0], v[1], v[2], v[3])
                                })
                            }
                            "pointerLocked" => c.pointer_locked = flag(),
                            "enabled" => c.enabled = flag(),
                            "showX" => c.show_x = flag(),
                            "showY" => c.show_y = flag(),
                            "showZ" => c.show_z = flag(),
                            "showXY" => c.show_xy = flag(),
                            "showYZ" => c.show_yz = flag(),
                            "showXZ" => c.show_xz = flag(),
                            "showXYZE" => c.show_xyze = flag(),
                            "showE" => c.show_e = flag(),
                            "minX" => c.min_x = num(value),
                            "maxX" => c.max_x = num(value),
                            "minY" => c.min_y = num(value),
                            "maxY" => c.max_y = num(value),
                            "minZ" => c.min_z = num(value),
                            "maxZ" => c.max_z = num(value),
                            other => panic!("unknown property {other}"),
                        }
                        return None;
                    }
                }
            }
            "call" => match action["name"].as_str() {
                Some("reset") => c.reset(),
                Some("detach") => c.detach(),
                Some("attach") => c.attach(&self.object),
                other => panic!("call {other:?}"),
            },
            "colors" => {
                let hex: Vec<Color> = action["colors"]
                    .as_array()
                    .expect("four colours")
                    .iter()
                    .map(|v| Color::from_hex(v.as_u64().expect("a hex colour") as u32))
                    .collect();
                c.set_colors(hex[0], hex[1], hex[2], hex[3]);
                self.active = hex[3];
                Vec::new()
            }
            "render" => Vec::new(),
            other => panic!("unknown op {other}"),
        };
        Some(events.iter().map(event_name).collect())
    }
}

/// Collects mismatches instead of stopping at the first, so a failure shows
/// how far it spreads.
struct Check<'a> {
    context: String,
    failures: &'a mut Vec<String>,
}

impl Check<'_> {
    fn number(&mut self, field: &str, want: &Value, got: f64) {
        let w = if want.is_null() { f64::NAN } else { num(want) };
        if !close(w, got) {
            self.failures.push(format!(
                "{}: {field} is {got}, three.js says {w}",
                self.context
            ));
        }
    }

    fn numbers(&mut self, field: &str, want: &Value, got: &[f64]) {
        let w = want
            .as_array()
            .unwrap_or_else(|| panic!("{field}: no array"));
        if w.len() != got.len() {
            self.failures.push(format!(
                "{}: {field} has {} values, three.js {}",
                self.context,
                got.len(),
                w.len()
            ));
            return;
        }
        for (i, (w, g)) in w.iter().zip(got).enumerate() {
            self.number(&format!("{field}[{i}]"), w, *g);
        }
    }

    fn equal<T: PartialEq + std::fmt::Debug>(&mut self, field: &str, want: T, got: T) {
        if want != got {
            self.failures.push(format!(
                "{}: {field} is {got:?}, three.js says {want:?}",
                self.context
            ));
        }
    }
}

fn v3(v: Vector3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn q4(q: Quaternion) -> [f64; 4] {
    [q.x, q.y, q.z, q.w]
}

fn compare_step(world: &World, step: &Value, events: Option<Vec<String>>, check: &mut Check) {
    if let (Some(events), Some(want)) = (events, step["events"].as_array()) {
        let want: Vec<String> = want
            .iter()
            .map(|e| e.as_str().expect("an event name").to_string())
            .collect();
        check.equal("events", want, events);
    }

    {
        let o = world.object.borrow();
        check.numbers("position", &step["p"], &v3(o.position));
        check.numbers("quaternion", &step["q"], &q4(o.quaternion));
        check.numbers("scale", &step["s"], &v3(o.scale));
    }
    let c = &world.controls;
    check.equal(
        "axis",
        step["axis"].as_str().map(str::to_string),
        c.axis().map(|a| a.as_str().to_string()),
    );
    let m = mode(step["mode"].as_str().expect("a mode"));
    check.equal("mode", m, c.get_mode());
    check.equal("dragging", step["dragging"].as_bool(), Some(c.dragging()));
    check.number("rotationAngle", &step["angle"], c.rotation_angle());
    check.equal(
        "root visible",
        step["rootVisible"].as_bool(),
        Some(c.get_helper().borrow().visible),
    );
    check.numbers(
        "plane quaternion",
        &step["plane"],
        &q4(world.plane().borrow().quaternion),
    );

    let handles: Vec<(ObjectRef, bool)> = [1, 0, 2]
        .iter()
        .flat_map(|&kind| {
            world
                .group(kind, m)
                .children()
                .into_iter()
                .map(move |h| (h, kind == 1))
        })
        .collect();
    let visible: String = handles
        .iter()
        .map(|(h, _)| if h.borrow().visible { '1' } else { '0' })
        .collect();
    check.equal(
        "visible handles",
        step["visible"].as_str(),
        Some(visible.as_str()),
    );
    let highlight: String = world
        .group(0, m)
        .children()
        .iter()
        .map(|h| {
            let h = h.borrow();
            let material = h.material().expect("a handle material");
            let color = material.color;
            let lit = material.opacity == 1.0
                && color.r == world.active.r
                && color.g == world.active.g
                && color.b == world.active.b;
            if lit {
                '1'
            } else {
                '0'
            }
        })
        .collect();
    check.equal(
        "highlighted handles",
        step["highlight"].as_str(),
        Some(highlight.as_str()),
    );

    let full = &step["full"];
    if full.is_null() {
        return;
    }
    let want = full["handles"].as_array().expect("full handles");
    check.equal("handle count", want.len(), handles.len());
    for (i, ((handle, picker), want)) in handles.iter().zip(want).enumerate() {
        let want = want.as_array().expect("a handle row");
        let h = handle.borrow();
        let field = |f: &str| format!("handle {i} ({}) {f}", h.name);
        check.equal(&field("name"), want[0].as_str(), Some(h.name.as_str()));
        check.equal(&field("visible"), want[1].as_bool(), Some(h.visible));
        let mut got: Vec<f64> = v3(h.position)
            .into_iter()
            .chain(q4(h.quaternion))
            .chain(v3(h.scale))
            .collect();
        if !picker {
            let material = h.material().expect("a handle material");
            got.extend([
                material.color.r,
                material.color.g,
                material.color.b,
                material.opacity,
            ]);
        }
        check.numbers(
            &field("transform and colour"),
            &Value::from(want[2..].to_vec()),
            &got,
        );
    }
    check.numbers(
        "root matrixWorld",
        &full["root"],
        &c.get_helper().borrow().matrix_world.elements,
    );
    check.numbers(
        "plane matrixWorld",
        &full["plane"],
        &world.plane().borrow().matrix_world.elements,
    );
    check.numbers(
        "worldPosition",
        &full["worldPosition"],
        &v3(c.world_position()),
    );
    check.numbers(
        "worldQuaternion",
        &full["worldQuaternion"],
        &q4(c.world_quaternion()),
    );
    check.numbers("eye", &full["eye"], &v3(c.eye()));
    check.numbers("pointStart", &full["pointStart"], &v3(c.point_start()));
    check.numbers("pointEnd", &full["pointEnd"], &v3(c.point_end()));
    check.numbers(
        "rotationAxis",
        &full["rotationAxis"],
        &v3(c.rotation_axis()),
    );
}

fn replay(
    scenario: &Value,
    camera: &mut impl TransformCamera,
    origin: &str,
    failures: &mut Vec<String>,
) {
    let name = scenario["name"].as_str().expect("a scenario name");
    let mut world = World::new(&scenario["setup"]);
    world.render(camera);
    let steps = scenario["steps"].as_array().expect("steps");
    for (index, step) in steps.iter().enumerate() {
        let events = world.act(&step["do"], camera);
        if step["render"].as_bool() != Some(false) {
            world.render(camera);
        }
        let mut check = Check {
            context: format!("{origin}: {name} step {index} {}", step["do"]),
            failures,
        };
        compare_step(&world, step, events, &mut check);
    }
}

/// A scenario's perspective camera, placed as three placed it.
fn perspective_camera(setup: &Value) -> PerspectiveCamera {
    let c = &setup["camera"];
    let element = nums(&setup["element"]);
    let p = nums(&c["position"]);
    let q = nums(&c["quaternion"]);
    let mut camera = PerspectiveCamera::new(
        num(&c["fov"]),
        element[0] / element[1],
        num(&c["near"]),
        num(&c["far"]),
    );
    camera.zoom = num(&c["zoom"]);
    camera.update_projection_matrix();
    {
        let mut o = camera.node.borrow_mut();
        o.position = Vector3::new(p[0], p[1], p[2]);
        o.quaternion = Quaternion::new(q[0], q[1], q[2], q[3]);
    }
    camera.update_matrix_world();
    camera
}

fn run_scenario(scenario: &Value, origin: &str, failures: &mut Vec<String>) {
    let setup = &scenario["setup"];
    let c = &setup["camera"];
    let p = nums(&c["position"]);
    let q = nums(&c["quaternion"]);
    let (position, quaternion) = (
        Vector3::new(p[0], p[1], p[2]),
        Quaternion::new(q[0], q[1], q[2], q[3]),
    );
    match c["type"].as_str() {
        Some("perspective") => {
            let mut camera = perspective_camera(setup);
            replay(scenario, &mut camera, origin, failures);
        }
        Some("orthographic") => {
            let mut camera = OrthographicCamera::new(
                num(&c["left"]),
                num(&c["right"]),
                num(&c["top"]),
                num(&c["bottom"]),
                num(&c["near"]),
                num(&c["far"]),
            );
            camera.zoom = num(&c["zoom"]);
            camera.update_projection_matrix();
            camera.object.position = position;
            camera.object.quaternion = quaternion;
            camera.update_matrix_world();
            replay(scenario, &mut camera, origin, failures);
        }
        other => panic!("unknown camera {other:?}"),
    }
}

fn assert_matches(origin: &str, reference: &Value) {
    assert_eq!(
        reference["revision"], "187dev",
        "{origin} is not three.js r187dev"
    );
    let scenarios = reference["scenarios"].as_array().expect("scenarios");
    assert_eq!(
        scenarios.len(),
        18,
        "{origin} has {} scenarios",
        scenarios.len()
    );
    let mut failures = Vec::new();
    for scenario in scenarios {
        run_scenario(scenario, origin, &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} mismatches, the first:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn parse(text: &str, origin: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{origin} is not JSON: {e}"))
}

fn committed() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    parse(&text, FIXTURE)
}

/// Under `CI` a missing checkout or node is a failure, as in
/// `tests/loaders_webp.rs`; locally it is a skip.
fn skip(reason: &str) -> Option<Value> {
    if std::env::var_os("CI").is_some() {
        panic!("the three.js re-run cannot run: {reason}");
    }
    eprintln!("not re-running three.js: {reason}");
    None
}

/// Runs the reference script into a scratch file, or says why it cannot.
fn live_reference() -> Option<Value> {
    let three = three_rs::testing::three_js_dir();
    if !three
        .join("examples/jsm/controls/TransformControls.js")
        .exists()
    {
        return skip(&format!(
            "no checkout at {} (set THREE_JS_DIR)",
            three.display()
        ));
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("transform_controls.json");
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

#[test]
fn matches_the_committed_fixture() {
    assert_matches(FIXTURE, &committed());
}

#[test]
fn matches_three_js_run_now() {
    if let Some(reference) = live_reference() {
        assert_matches("three.js run now", &reference);
    }
}

/// The geometry fingerprints are sums over `f32` positions, compared to the
/// relative tolerance of an `f32`: the crate's geometry generators and
/// three's agree to the last f64 bit only up to libm, and a last-bit
/// difference can round one stored `f32` the other way.
const GEOMETRY_TOLERANCE: f64 = 1e-6;

fn graph_rows(controls: &TransformControls) -> Vec<(String, ObjectRef)> {
    let root = controls.get_helper();
    let gizmo = root.children()[0].clone();
    let mut rows = Vec::new();
    for (k, kind) in ["gizmo", "picker", "helper"].iter().enumerate() {
        for (m, mode) in ["translate", "rotate", "scale"].iter().enumerate() {
            let group = gizmo.children()[k * 3 + m].clone();
            rows.push((format!("{kind}.{mode}"), group.clone()));
            for handle in group.children() {
                rows.push((format!("{kind}.{mode}"), handle));
            }
        }
    }
    rows
}

#[test]
fn the_gizmo_graph_is_three_js_graph() {
    let reference = committed();
    let want = reference["graph"].as_array().expect("the graph");
    let controls = TransformControls::new();
    let root = controls.get_helper();
    let mut failures = Vec::new();

    {
        let mut check = Check {
            context: "root".to_string(),
            failures: &mut failures,
        };
        let r = root.borrow();
        check.equal("type", want[0]["type"].as_str(), Some(r.object_type));
        check.equal("visible", want[0]["visible"].as_bool(), Some(r.visible));
        let types: Vec<&str> = want[0]["children"]
            .as_array()
            .expect("root children")
            .iter()
            .map(|t| t.as_str().expect("a type"))
            .collect();
        let got: Vec<&str> = root
            .children()
            .iter()
            .map(|c| c.borrow().object_type)
            .collect();
        check.equal("children", types, got);
        let gizmo = root.children()[0].clone();
        check.equal(
            "gizmo type",
            want[1]["type"].as_str(),
            Some(gizmo.borrow().object_type),
        );
        check.equal(
            "gizmo children",
            want[1]["children"].as_u64(),
            Some(gizmo.children().len() as u64),
        );
    }

    let rows = graph_rows(&controls);
    let plane_row = want.last().expect("the plane row");
    let handle_rows = &want[2..want.len() - 1];
    assert_eq!(handle_rows.len(), rows.len(), "graph rows");
    let mut handles = 0;
    for (index, ((group, node), want)) in rows.iter().zip(handle_rows).enumerate() {
        let mut check = Check {
            context: format!("graph row {index} ({group} {})", want["name"]),
            failures: &mut failures,
        };
        check.equal("group", want["group"].as_str(), Some(group.as_str()));
        let n = node.borrow();
        check.equal("type", want["type"].as_str(), Some(n.object_type));
        if want.get("name").is_none() {
            check.equal("visible", want["visible"].as_bool(), Some(n.visible));
            check.equal(
                "children",
                want["children"].as_u64(),
                Some(node.children().len() as u64),
            );
            continue;
        }
        handles += 1;
        check.equal("name", want["name"].as_str(), Some(n.name.as_str()));
        // Three tags exactly the helper groups' handles `'helper'`; the port
        // keeps the tag with the handle, from the same maps.
        check.equal("tag", Some(want["tag"].as_str()), controls.handle_tag(node));
        check.equal(
            "renderOrder",
            want["renderOrder"].as_str(),
            Some("Infinity"),
        );
        check.equal("renderOrder value", f64::INFINITY, n.render_order);
        check.numbers(
            "transform",
            &want["transform"],
            &v3(n.position)
                .into_iter()
                .chain(q4(n.quaternion))
                .chain(v3(n.scale))
                .collect::<Vec<_>>(),
        );
        compare_material(&mut check, &want["material"], &n);
        compare_geometry(&mut check, &want["geometry"], &n);
    }
    assert_eq!(handles, 66, "handle count");

    {
        let plane = root.children()[1].clone();
        let n = plane.borrow();
        let mut check = Check {
            context: "plane".to_string(),
            failures: &mut failures,
        };
        check.equal("type", plane_row["type"].as_str(), Some(n.object_type));
        compare_material(&mut check, &plane_row["material"], &n);
        compare_geometry(&mut check, &plane_row["geometry"], &n);
    }

    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn compare_material(check: &mut Check, want: &Value, object: &Object3D) {
    let m = object.material().expect("a material");
    check.numbers("color", &want["color"], &[m.color.r, m.color.g, m.color.b]);
    check.number("opacity", &want["opacity"], m.opacity);
    check.equal(
        "transparent",
        want["transparent"].as_bool(),
        Some(m.transparent),
    );
    check.equal("depthTest", want["depthTest"].as_bool(), Some(m.depth_test));
    check.equal(
        "depthWrite",
        want["depthWrite"].as_bool(),
        Some(m.depth_write),
    );
    check.equal("fog", want["fog"].as_bool(), Some(m.fog));
    let side = match m.side {
        Side::Front => 0,
        Side::Back => 1,
        Side::Double => 2,
    };
    check.equal("side", want["side"].as_u64(), Some(side));
    check.equal(
        "material visible",
        want["visible"].as_bool(),
        Some(m.visible),
    );
    check.equal("wireframe", want["wireframe"].as_bool(), Some(m.wireframe));
}

fn compare_geometry(check: &mut Check, want: &Value, object: &Object3D) {
    let geometry = match &object.payload {
        three_rs::objects::Payload::Mesh(mesh) => mesh.geometry.clone(),
        three_rs::objects::Payload::Line(line) => line.geometry.clone(),
        _ => panic!("a handle is a mesh or a line"),
    };
    let position = geometry.position().expect("a position attribute");
    check.equal(
        "vertices",
        want["vertices"].as_u64(),
        Some(position.count() as u64),
    );
    check.equal(
        "indices",
        want["indices"].as_u64(),
        Some(geometry.index.as_ref().map_or(0, |i| i.count()) as u64),
    );
    let (mut x, mut y, mut z, mut w) = (0.0, 0.0, 0.0, 0.0);
    for i in 0..position.count() {
        let (px, py, pz) = (position.get_x(i), position.get_y(i), position.get_z(i));
        x += px;
        y += py;
        z += pz;
        w += (i + 1) as f64 * (px + 2.0 * py + 3.0 * pz);
    }
    let sums = nums(&want["sums"]);
    for (field, (want, got)) in ["Σx", "Σy", "Σz", "Σ(i+1)(x+2y+3z)"]
        .iter()
        .zip(sums.iter().zip([x, y, z, w]))
    {
        let scale = want.abs().max(1.0);
        if (want - got).abs() > GEOMETRY_TOLERANCE * scale {
            check.failures.push(format!(
                "{}: geometry {field} is {got}, three.js says {want}",
                check.context
            ));
        }
    }
}

/// `translate_world` replayed through its first drag's first move: an `X`
/// drag in progress, with the object already moved.
fn mid_drag() -> (World, PerspectiveCamera, Vec<Value>) {
    let reference = committed();
    let scenario = reference["scenarios"]
        .as_array()
        .expect("scenarios")
        .iter()
        .find(|s| s["name"] == "translate_world")
        .expect("the translate_world scenario")
        .clone();
    let mut camera = perspective_camera(&scenario["setup"]);
    let mut world = World::new(&scenario["setup"]);
    world.render(&mut camera);
    let steps = scenario["steps"].as_array().expect("steps").clone();
    let down = steps
        .iter()
        .position(|s| s["do"]["op"] == "down")
        .expect("a down");
    for step in &steps[..=down + 1] {
        world.act(&step["do"], &mut camera);
        world.render(&mut camera);
    }
    assert!(world.controls.dragging(), "the drag is in progress");
    (world, camera, steps[down + 2..].to_vec())
}

/// `reset()` after `detach()` mid-drag, where three throws on
/// `this.object.position` before dispatching anything: the port sends
/// nothing and changes nothing, `pointStart` included.
#[test]
fn reset_after_detach_mid_drag_does_nothing() {
    let (mut world, _camera, _) = mid_drag();
    let moved = world.object.borrow().position;
    let point_start = world.controls.point_start();
    assert_ne!(
        point_start,
        world.controls.point_end(),
        "the drag has moved"
    );
    world.controls.detach();
    assert!(
        world.controls.dragging(),
        "detach() leaves dragging, as in three"
    );
    assert_eq!(world.controls.reset(), Vec::new(), "reset() events");
    assert_eq!(world.object.borrow().position, moved, "the object moved");
    assert_eq!(world.controls.point_start(), point_start, "pointStart");
}

/// `disconnect()` removes the drag listener: a later move neither moves the
/// object nor sends `objectChange`, and, as three's `disconnect()` touches no
/// state, the drag stays `dragging` with its `axis` until an up.
#[test]
fn disconnect_stops_a_drag_from_outside() {
    let (mut world, mut camera, rest) = mid_drag();
    let next = rest
        .iter()
        .find(|s| s["do"]["op"] == "move")
        .expect("another move");
    let moved = world.object.borrow().position;
    let axis = world.controls.axis();
    world.controls.disconnect();
    let events = world.act(&next["do"], &mut camera).expect("events");
    assert_eq!(events, Vec::<String>::new(), "events after disconnect()");
    assert_eq!(world.object.borrow().position, moved, "the object moved");
    assert!(world.controls.dragging(), "dragging after disconnect()");
    assert_eq!(world.controls.axis(), axis, "axis after disconnect()");
}

#[test]
fn defaults_are_three_js_defaults() {
    let controls = TransformControls::new();
    assert!(controls.enabled);
    assert!(controls.object().is_none());
    assert_eq!(controls.axis(), None);
    assert_eq!(controls.get_mode(), Mode::Translate);
    assert_eq!(controls.translation_snap(), None);
    assert_eq!(controls.rotation_snap(), None);
    assert_eq!(controls.scale_snap(), None);
    assert_eq!(controls.space(), Space::World);
    assert_eq!(controls.size(), 1.0);
    assert!(!controls.dragging());
    assert!(controls.viewport.is_none());
    for flag in [
        controls.show_x,
        controls.show_y,
        controls.show_z,
        controls.show_xy,
        controls.show_yz,
        controls.show_xz,
        controls.show_xyze,
        controls.show_e,
    ] {
        assert!(flag);
    }
    for min in [controls.min_x, controls.min_y, controls.min_z] {
        assert_eq!(min, f64::NEG_INFINITY);
    }
    for max in [controls.max_x, controls.max_y, controls.max_z] {
        assert_eq!(max, f64::INFINITY);
    }
    assert_eq!(controls.rotation_angle(), 0.0);
    assert!(!controls.get_helper().borrow().visible);
}
