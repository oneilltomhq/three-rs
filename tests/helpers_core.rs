//! The core helpers gate (issue #300): `tools/helpers_reference.mjs` runs
//! three.js' own `src/helpers/` classes from the pinned checkout under node
//! and prints what each builds; this test builds the same helpers with the
//! same inputs and compares the two node for node: type, visibility,
//! `matrixAutoUpdate`, position, quaternion, scale, local and world matrices,
//! every geometry attribute and the index, and the material's type
//! (`LineBasicMaterial` or `MeshBasicMaterial`; see `material_type`), side,
//! colour and flags.
//!
//! The reference is computed live, not checked in, so the comparison is
//! always against three.js itself. A missing `node` or checkout fails the test
//! rather than skipping it: without them there is no gate.
//!
//! Each scenario below mirrors the one of the same name in the script; a
//! change to one is a change to both. Where three's `root.updateMatrixWorld()`
//! runs a helper's `updateMatrixWorld` override, the port calls the scene's
//! update and then the helper's `update_matrix_world` (see
//! `three_rs::helpers`).
//!
//! Tolerances: geometry arrays are `Float32Array`s on both sides, filled from
//! `f64` arithmetic whose order may differ, so they get a relative 1e-6;
//! everything else is `f64` on both sides and gets 1e-9.
//!
//! Three's helpers all set `toneMapped: false`, which the port's materials
//! have no field for: the test checks the reference says `false` and drops
//! it from the comparison.

use std::collections::BTreeMap;
use std::process::Command;

use serde_json::{json, Map, Value};
use three_rs::core::{Index, Node, Object3D};
use three_rs::geometries::{box_geometry, sphere_geometry};
use three_rs::helpers::{
    ArrowHelper, AxesHelper, Box3Helper, BoxHelper, DirectionalLightHelper, HemisphereLightHelper,
    PlaneHelper, PointLightHelper, PolarGridHelper, SkeletonHelper, SpotLightHelper,
};
use three_rs::lights::{DirectionalLight, HemisphereLight, PointLight, SpotLight};
use three_rs::materials::{MaterialKind, MeshBasicNodeMaterial, Side};
use three_rs::math::{Box3, Color, Plane, Vector3};
use three_rs::objects::{Bone, Group, Mesh};
use three_rs::testing::three_js_dir;

// --- Reference ---------------------------------------------------------------

fn reference() -> Value {
    let three_dir = three_js_dir();
    let module = three_dir.join("build/three.module.js");
    assert!(
        module.is_file(),
        "the helpers gate needs the pinned three.js checkout: {} is missing \
         (set THREE_JS_DIR)",
        module.display()
    );

    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/helpers_reference.mjs");
    let output = Command::new("node")
        .arg(script)
        .arg(&three_dir)
        .output()
        .unwrap_or_else(|error| panic!("the helpers gate needs node on PATH: {error}"));
    assert!(
        output.status.success(),
        "{script} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("the reference script prints JSON")
}

// --- The port's side of the dump ----------------------------------------------

fn floats(values: impl IntoIterator<Item = f64>) -> Value {
    Value::Array(values.into_iter().map(|v| json!(v)).collect())
}

fn color(c: Color) -> Value {
    floats([c.r, c.g, c.b])
}

/// Three's `material.type` for a port material. `LineBasicNodeMaterial` is an
/// alias of `MeshBasicNodeMaterial` with no kind of its own (both are
/// `MaterialKind::Basic`); what tells `::line()` from `::new()` is
/// `refraction_ratio`, 0 where three's class has no `refractionRatio`
/// (`LineBasicMaterial`) and `MeshBasicMaterial`'s 0.98 otherwise. Any other
/// combination is named by its parts, so it fails the comparison.
fn material_type(m: &MeshBasicNodeMaterial) -> String {
    match (m.kind, m.refraction_ratio) {
        (MaterialKind::Basic, ratio) if ratio == 0.0 => "LineBasicMaterial".to_owned(),
        (MaterialKind::Basic, ratio) if ratio == 0.98 => "MeshBasicMaterial".to_owned(),
        (kind, ratio) => format!("{kind:?} with refraction_ratio {ratio}"),
    }
}

/// Three's `FrontSide`, `BackSide` and `DoubleSide` constants.
fn side(side: Side) -> Value {
    json!(match side {
        Side::Front => 0,
        Side::Back => 1,
        Side::Double => 2,
    })
}

/// `dump( object )` in the script, without `material.toneMapped`.
fn dump(node: &Node) -> Value {
    let object = node.borrow();

    let geometry = object.geometry().map_or(Value::Null, |geometry| {
        let index = match &geometry.index {
            None => Value::Null,
            Some(Index::U16(values)) => json!(values),
            Some(Index::U32(values)) => json!(values),
        };
        let attributes: BTreeMap<_, _> = geometry
            .attributes()
            .map(|(name, attribute)| {
                (
                    name.to_owned(),
                    json!({
                        "itemSize": attribute.item_size,
                        "array": floats(attribute.array().iter().map(|&v| f64::from(v))),
                    }),
                )
            })
            .collect();
        json!({ "index": index, "attributes": attributes })
    });

    let material = object.material().map_or(Value::Null, |m| {
        json!({
            "type": material_type(m),
            "side": side(m.side),
            "color": color(m.color),
            "opacity": m.opacity,
            "transparent": m.transparent,
            "depthTest": m.depth_test,
            "depthWrite": m.depth_write,
            "vertexColors": m.vertex_colors,
            "wireframe": m.wireframe,
            "fog": m.fog,
        })
    });

    let q = object.quaternion;
    let value = json!({
        "type": object.object_type,
        "visible": object.visible,
        "matrixAutoUpdate": object.matrix_auto_update,
        "position": floats([object.position.x, object.position.y, object.position.z]),
        "quaternion": floats([q.x, q.y, q.z, q.w]),
        "scale": floats([object.scale.x, object.scale.y, object.scale.z]),
        "matrix": floats(object.matrix.elements),
        "matrixWorld": floats(object.matrix_world.elements),
        "geometry": geometry,
        "material": material,
    });
    drop(object);

    let mut value = value;
    value["children"] = Value::Array(node.children().iter().map(dump).collect());
    value
}

// --- Comparison ----------------------------------------------------------------

fn compare(path: &str, expected: &Value, actual: &Value, failures: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Number(e), Value::Number(a)) => {
            let (e, a) = (e.as_f64().unwrap(), a.as_f64().unwrap());
            let tolerance = if path.contains(".geometry.") {
                1e-6 * e.abs().max(a.abs()).max(1.0)
            } else {
                1e-9 * e.abs().max(a.abs()).max(1.0)
            };
            if (e - a).abs() > tolerance {
                failures.push(format!("{path}: three {e}, port {a}"));
            }
        }
        (Value::Array(e), Value::Array(a)) => {
            if e.len() != a.len() {
                failures.push(format!(
                    "{path}: three has {} items, port {}",
                    e.len(),
                    a.len()
                ));
                return;
            }
            for (i, (e, a)) in e.iter().zip(a).enumerate() {
                compare(&format!("{path}[{i}]"), e, a, failures);
            }
        }
        (Value::Object(e), Value::Object(a)) => {
            let mut e: Map<String, Value> = e.clone();
            if path.ends_with(".material") {
                match e.remove("toneMapped") {
                    Some(Value::Bool(false)) => {}
                    other => failures.push(format!(
                        "{path}.toneMapped: three {other:?}, the port only has false"
                    )),
                }
            }
            let keys: std::collections::BTreeSet<&String> = e.keys().chain(a.keys()).collect();
            for key in keys {
                match (e.get(key), a.get(key)) {
                    (Some(ev), Some(av)) => compare(&format!("{path}.{key}"), ev, av, failures),
                    (Some(_), None) => failures.push(format!("{path}.{key}: missing in the port")),
                    (None, Some(_)) => failures.push(format!("{path}.{key}: not in three")),
                    (None, None) => unreachable!(),
                }
            }
        }
        _ if expected == actual => {}
        _ => failures.push(format!("{path}: three {expected}, port {actual}")),
    }
}

// --- Scenarios -------------------------------------------------------------------

fn transformed_root() -> Node {
    let root = Object3D::new_node();
    {
        let mut object = root.borrow_mut();
        object.position.set(0.25, -1.0, 2.0);
        object.set_rotation(0.1, 0.2, 0.3);
        object.scale.set(1.5, 1.5, 1.5);
    }
    root
}

fn normalized(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3::new(x, y, z).normalized()
}

fn hex(hex: u32) -> Color {
    Color::from_hex(hex)
}

fn light_target(light: &Node) -> Node {
    light
        .borrow()
        .light()
        .and_then(|light| light.target.clone())
        .expect("the light has a target")
}

fn set_light_color(light: &Node, color: Color) {
    light.borrow_mut().light_mut().unwrap().light.color = color;
}

fn stages(stages: Vec<(&str, Value)>) -> Value {
    Value::Object(stages.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

fn axes_default() -> Value {
    let helper = AxesHelper::default();
    stages(vec![("built", dump(&helper.node))])
}

fn axes_colors() -> Value {
    let helper = AxesHelper::new(2.5);
    let built = dump(&helper.node);
    helper.set_colors(hex(0x123456), hex(0xabcdef), hex(0xff8800));
    stages(vec![("built", built), ("colored", dump(&helper.node))])
}

fn arrow_default() -> Value {
    let root = transformed_root();
    let helper = ArrowHelper::default();
    root.add(&helper.node);
    root.update_matrix_world(false);
    stages(vec![("built", dump(&helper.node))])
}

fn arrow_full() -> Value {
    let root = transformed_root();
    let helper = ArrowHelper::new(
        normalized(1.0, 2.0, 3.0),
        Vector3::new(1.0, -2.0, 0.5),
        3.0,
        hex(0x00ff88),
        Some(0.7),
        Some(0.4),
    );
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    helper.set_length(5.0, None, None);
    helper.set_color(hex(0x112233));
    helper.set_direction(&normalized(-1.0, 0.5, 0.25));
    root.update_matrix_world(false);

    stages(vec![("built", built), ("changed", dump(&helper.node))])
}

fn arrow_up() -> Value {
    let helper = ArrowHelper::new(
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        1.0,
        hex(0xffff00),
        None,
        None,
    );
    helper.node.update_matrix_world(false);
    stages(vec![("built", dump(&helper.node))])
}

fn arrow_down() -> Value {
    let helper = ArrowHelper::new(
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        2.0,
        hex(0xffff00),
        None,
        None,
    );
    helper.node.update_matrix_world(false);
    stages(vec![("built", dump(&helper.node))])
}

fn box_helper() -> Value {
    let root = transformed_root();
    let group = Group::new();
    group.borrow_mut().position.set(-1.0, 0.0, 0.0);
    let mesh = Mesh::new(
        std::rc::Rc::new(box_geometry(1.0, 2.0, 3.0, 1, 1, 1)),
        MeshBasicNodeMaterial::new(),
    );
    {
        let mut object = mesh.borrow_mut();
        object.position.set(1.0, 2.0, 3.0);
        object.set_rotation(0.3, 0.4, 0.5);
        object.scale.set(1.0, 2.0, 0.5);
    }
    group.add(&mesh);
    root.add(&group);

    let mut helper = BoxHelper::new(Some(mesh.clone()), hex(0xff0000));
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    mesh.borrow_mut().position.set(-2.0, 0.0, 1.0);
    helper.update();
    root.update_matrix_world(false);
    let moved = dump(&helper.node);

    let sphere = Mesh::new(
        std::rc::Rc::new(sphere_geometry(1.0, 32, 16)),
        MeshBasicNodeMaterial::new(),
    );
    sphere.borrow_mut().position.set(0.0, 3.0, 0.0);
    root.add(&sphere);
    root.update_matrix_world(false);
    helper.set_from_object(sphere);

    stages(vec![
        ("built", built),
        ("moved", moved),
        ("sphere", dump(&helper.node)),
    ])
}

fn box3_helper() -> Value {
    let root = transformed_root();
    let mut helper = Box3Helper::new(
        Box3::new(Vector3::new(-1.0, 0.0, 2.0), Vector3::new(3.0, 4.0, 5.0)),
        hex(0x00ffff),
    );
    root.add(&helper.node);
    let built = dump(&helper.node);

    root.update_matrix_world(false);
    helper.update_matrix_world(false);
    let updated = dump(&helper.node);

    helper.box3.min.set(0.0, -2.0, -1.0);
    helper.box3.max.set(0.5, 2.0, 1.0);
    root.update_matrix_world(false);
    helper.update_matrix_world(false);

    stages(vec![
        ("built", built),
        ("updated", updated),
        ("resized", dump(&helper.node)),
    ])
}

fn plane_helper() -> Value {
    let root = transformed_root();
    let mut helper = PlaneHelper::new(
        Plane::new(normalized(1.0, 1.0, 0.0), -2.0),
        3.0,
        hex(0xff00ff),
    );
    root.add(&helper.node);
    root.update_matrix_world(false);
    helper.update_matrix_world(false);
    let updated = dump(&helper.node);

    helper.plane.normal.set(0.0, 0.0, 1.0);
    helper.plane.constant = 1.5;
    helper.size = 2.0;
    root.update_matrix_world(false);
    helper.update_matrix_world(false);

    stages(vec![("updated", updated), ("changed", dump(&helper.node))])
}

fn polar_default() -> Value {
    let node = PolarGridHelper::new(10.0, 16, 8, 64, hex(0x444444), hex(0x888888));
    stages(vec![("built", dump(&node))])
}

fn polar_custom() -> Value {
    let node = PolarGridHelper::new(5.0, 6, 3, 12, hex(0xff0000), hex(0x00ff00));
    stages(vec![("built", dump(&node))])
}

fn polar_one_sector() -> Value {
    let node = PolarGridHelper::new(2.0, 1, 2, 8, hex(0x444444), hex(0x888888));
    stages(vec![("built", dump(&node))])
}

fn directional() -> Value {
    let root = transformed_root();
    let light = DirectionalLight::new(hex(0xffaa33), 2.0);
    light.borrow_mut().position.set(5.0, 10.0, 7.5);
    light_target(&light)
        .borrow_mut()
        .position
        .set(1.0, 0.0, -2.0);
    root.add(&light);

    let helper = DirectionalLightHelper::new(&light, 2.0, None);
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    set_light_color(&light, hex(0x3366ff));
    light.borrow_mut().position.set(-3.0, 4.0, 1.0);
    helper.update();
    root.update_matrix_world(false);

    stages(vec![("built", built), ("updated", dump(&helper.node))])
}

fn directional_colored() -> Value {
    let root = Object3D::new_node();
    let light = DirectionalLight::new(hex(0xffffff), 1.0);
    root.add(&light);

    let helper = DirectionalLightHelper::new(&light, 1.0, Some(hex(0xaabbcc)));
    root.add(&helper.node);
    root.update_matrix_world(false);

    stages(vec![("built", dump(&helper.node))])
}

fn hemisphere() -> Value {
    let root = transformed_root();
    let light = HemisphereLight::new(hex(0x123456), hex(0xabc012), 0.6);
    light.borrow_mut().position.set(2.0, 3.0, -1.0);
    root.add(&light);

    let helper = HemisphereLightHelper::new(&light, 1.5, None);
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    set_light_color(&light, hex(0xff0000));
    light.borrow_mut().light_mut().unwrap().ground_color = hex(0x0000ff);
    light.borrow_mut().position.set(0.0, -1.0, 1.0);
    helper.update();
    root.update_matrix_world(false);

    stages(vec![("built", built), ("updated", dump(&helper.node))])
}

fn hemisphere_colored() -> Value {
    let root = Object3D::new_node();
    let light = HemisphereLight::new(hex(0x123456), hex(0xabc012), 0.6);
    root.add(&light);

    let helper = HemisphereLightHelper::new(&light, 1.0, Some(hex(0xabc012)));
    root.add(&helper.node);
    root.update_matrix_world(false);

    stages(vec![("built", dump(&helper.node))])
}

fn point() -> Value {
    let root = transformed_root();
    let light = PointLight::new(hex(0x00ff00), 1.0, 100.0);
    light.borrow_mut().position.set(1.0, 2.0, 3.0);
    root.add(&light);

    let helper = PointLightHelper::new(&light, 0.5, None);
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    set_light_color(&light, hex(0xff00ff));
    helper.update();
    root.update_matrix_world(false);

    stages(vec![("built", built), ("updated", dump(&helper.node))])
}

fn point_colored() -> Value {
    let root = Object3D::new_node();
    let light = PointLight::new(hex(0x00ff00), 1.0, 0.0);
    root.add(&light);

    let helper = PointLightHelper::new(&light, 1.0, Some(hex(0xaaaaaa)));
    root.add(&helper.node);
    root.update_matrix_world(false);

    stages(vec![("built", dump(&helper.node))])
}

fn spot() -> Value {
    let root = transformed_root();
    let light = SpotLight::new(hex(0xff8844), 1.0);
    light.borrow_mut().position.set(3.0, 5.0, 2.0);
    light_target(&light)
        .borrow_mut()
        .position
        .set(0.0, 0.0, -1.0);
    {
        let mut object = light.borrow_mut();
        let spot = object.light_mut().unwrap();
        spot.angle = 0.4;
        spot.distance = 20.0;
    }
    root.add(&light);

    // Built with no parent, so `update()` copies the light's world matrix;
    // after `add()` the next `update()` takes the parent's out of it.
    let helper = SpotLightHelper::new(&light, None);
    root.add(&helper.node);
    root.update_matrix_world(false);
    let built = dump(&helper.node);

    helper.update();
    root.update_matrix_world(false);

    stages(vec![("built", built), ("updated", dump(&helper.node))])
}

fn spot_colored() -> Value {
    let light = SpotLight::new(hex(0xffffff), 1.0);
    let helper = SpotLightHelper::new(&light, Some(hex(0x00ffff)));
    helper.node.update_matrix_world(false);

    stages(vec![("built", dump(&helper.node))])
}

struct Skeleton {
    root: Node,
    character: Node,
    b0: Node,
    b1: Node,
}

fn skeleton() -> Skeleton {
    let root = transformed_root();
    let character = Group::new();
    character.borrow_mut().position.set(0.0, 1.0, 0.0);
    root.add(&character);

    let bone = |x: f64, y: f64, z: f64| {
        let bone = Bone::new();
        bone.borrow_mut().position.set(x, y, z);
        bone
    };

    let b0 = bone(0.0, 1.0, 0.0);
    character.add(&b0);

    let b1 = bone(0.0, 1.0, 0.5);
    b1.borrow_mut().set_rotation(0.0, 0.0, 0.3);
    b0.add(&b1);

    let b2 = bone(0.5, 1.0, 0.0);
    b1.add(&b2);

    let b3 = bone(-1.0, 0.5, 0.0);
    b0.add(&b3);

    // A bone under a non-bone: in the list, but no segment to its parent.
    let holder = Object3D::new_node();
    holder.borrow_mut().position.set(0.0, 0.0, 1.0);
    b1.add(&holder);
    let b4 = bone(0.0, 0.5, 0.0);
    holder.add(&b4);

    Skeleton {
        root,
        character,
        b0,
        b1,
    }
}

fn skeleton_scenario() -> Value {
    let Skeleton {
        root,
        character,
        b1,
        ..
    } = skeleton();
    let helper = SkeletonHelper::new(&character);
    root.add(&helper.node);
    root.update_matrix_world(false);
    helper.update_matrix_world(false);
    let built = dump(&helper.node);

    b1.borrow_mut().set_rotation(0.5, 0.0, -0.2);
    root.update_matrix_world(false);
    helper.update_matrix_world(false);

    stages(vec![("built", built), ("posed", dump(&helper.node))])
}

fn skeleton_bone_root() -> Value {
    let Skeleton { root, b0, .. } = skeleton();
    let helper = SkeletonHelper::new(&b0);
    root.add(&helper.node);
    root.update_matrix_world(false);
    helper.update_matrix_world(false);

    stages(vec![("built", dump(&helper.node))])
}

/// Builds one scenario's stages, as the script's function of the same name.
type Scenario = fn() -> Value;

#[test]
fn core_helpers_match_three() {
    let reference = reference();
    let reference = reference.as_object().expect("the reference is an object");

    let scenarios: Vec<(&str, Scenario)> = vec![
        ("axes_default", axes_default),
        ("axes_colors", axes_colors),
        ("arrow_default", arrow_default),
        ("arrow_full", arrow_full),
        ("arrow_up", arrow_up),
        ("arrow_down", arrow_down),
        ("box_helper", box_helper),
        ("box3_helper", box3_helper),
        ("plane_helper", plane_helper),
        ("polar_default", polar_default),
        ("polar_custom", polar_custom),
        ("polar_one_sector", polar_one_sector),
        ("directional", directional),
        ("directional_colored", directional_colored),
        ("hemisphere", hemisphere),
        ("hemisphere_colored", hemisphere_colored),
        ("point", point),
        ("point_colored", point_colored),
        ("spot", spot),
        ("spot_colored", spot_colored),
        ("skeleton", skeleton_scenario),
        ("skeleton_bone_root", skeleton_bone_root),
    ];

    // Every scenario the script runs is checked here, and the other way round.
    let mut names: Vec<&str> = scenarios.iter().map(|(name, _)| *name).collect();
    let mut reference_names: Vec<&str> = reference.keys().map(String::as_str).collect();
    names.sort_unstable();
    reference_names.sort_unstable();
    assert_eq!(
        names, reference_names,
        "the scenario lists have drifted apart"
    );

    let mut failures = Vec::new();
    for (name, build) in scenarios {
        compare(name, &reference[name], &build(), &mut failures);
    }

    assert!(
        failures.is_empty(),
        "{} differences from three.js:\n{}",
        failures.len(),
        failures
            .iter()
            .take(80)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
