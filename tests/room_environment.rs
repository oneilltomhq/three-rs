//! `RoomEnvironment` against three.js' own.
//!
//! `tests/fixtures/room_environment.json` was printed by constructing
//! `examples/jsm/environments/RoomEnvironment.js` from the vendor checkout
//! under node and dumping every object's transform, every instance matrix,
//! the point light's four parameters and every material's fields. The
//! generator script lived in the scratchpad and is gone; nothing here derives
//! from a reference image.
//!
//! This is the gate the scout's `ENVIRONMENT-FAMILY.md` §2 asks for — "the
//! construction is trivially gated numerically: assert the 13 transforms, the
//! light parameters and the six emissive intensities against the JS" — and it
//! is the only gate the construction gets, because a permuted box or a light
//! at the wrong height still produces a plausible environment and a plausible
//! frame.

use serde_json::Value;
use three_rs::materials::{MaterialKind, Side};
use three_rs::math::ColorSpace;
use three_rs::{Matrix4, RoomEnvironment};

fn fixture() -> Value {
    let text = include_str!("fixtures/room_environment.json");
    serde_json::from_str(text).expect("the fixture is valid JSON")
}

fn floats(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("an array of numbers")
        .iter()
        .map(|v| v.as_f64().expect("a number"))
        .collect()
}

/// three.js prints its matrices in f64 and the port composes them in f64, so
/// the two agree to the last bit for the translations and to within one ulp of
/// the trigonometry for the rotated boxes. 1e-12 is far tighter than anything
/// a transcription error survives.
const EPS: f64 = 1e-12;

fn assert_matrix(label: &str, actual: &Matrix4, expected: &Value) {
    let expected = floats(expected);
    assert_eq!(
        expected.len(),
        16,
        "{label}: three's matrix has 16 elements"
    );
    for (i, want) in expected.iter().enumerate() {
        let got = actual.elements[i];
        assert!(
            (got - want).abs() < EPS,
            "{label}: element {i} is {got}, three's is {want}"
        );
    }
}

#[test]
fn the_room_is_three_s_room() {
    let fixture = fixture();
    let scene = RoomEnvironment::new();

    assert_eq!(fixture["name"], "RoomEnvironment");
    assert_eq!(scene.node.borrow().name, "RoomEnvironment");

    // `this.position.y = - 3.5` — the whole room is lifted, which is what puts
    // the cube camera at the origin inside it rather than on its floor.
    let root_position = floats(&fixture["position"]);
    let position = scene.node.borrow().position;
    assert_eq!(
        [position.x, position.y, position.z],
        [root_position[0], root_position[1], root_position[2]]
    );

    let children = scene.node.borrow().children.clone();
    assert_eq!(
        children.len(),
        fixture["children"].as_u64().unwrap() as usize,
        "one light, the room, the instanced boxes and six panels"
    );

    let objects = fixture["objects"].as_array().unwrap();
    assert_eq!(objects.len(), children.len());

    let mut lambert_intensities = Vec::new();
    let mut standard_count = 0;

    for (child, expected) in children.iter().zip(objects) {
        let mut object = child.borrow_mut();
        let label = expected["type"].as_str().unwrap().to_string();
        object.update_matrix();
        assert_matrix(&label, &object.matrix, &expected["matrix"]);

        if let Some(light) = expected.get("light").filter(|v| !v.is_null()) {
            let payload = object.payload.light().expect("a PointLight payload");
            assert_eq!(
                payload.light.color.get_hex(ColorSpace::SRGB),
                light["color"].as_u64().unwrap() as u32
            );
            assert_eq!(
                payload.light.intensity,
                light["intensity"].as_f64().unwrap()
            );
            assert_eq!(payload.distance, light["distance"].as_f64().unwrap());
            assert_eq!(payload.decay, light["decay"].as_f64().unwrap());
        }

        if let Some(material) = expected.get("material").filter(|v| !v.is_null()) {
            let actual = object.material().expect("a mesh material").clone();
            match material["type"].as_str().unwrap() {
                "MeshStandardMaterial" => {
                    standard_count += 1;
                    assert_eq!(actual.kind, MaterialKind::Standard, "{label}");
                    assert_eq!(actual.roughness, material["roughness"].as_f64().unwrap());
                    assert_eq!(actual.metalness, material["metalness"].as_f64().unwrap());
                }
                "MeshLambertMaterial" => {
                    // The emissive-only shim. `color` is black, so the Lambert
                    // term is provably dead — but the *kind* still has to be
                    // Lambert or the generated WGSL is a different program.
                    assert_eq!(actual.kind, MaterialKind::Lambert, "{label}");
                    lambert_intensities.push(actual.emissive_intensity);
                }
                other => panic!("unexpected material {other}"),
            }
            assert_eq!(
                actual.color.get_hex(ColorSpace::SRGB),
                material["color"].as_u64().unwrap() as u32,
                "{label}"
            );
            assert_eq!(
                actual.emissive.get_hex(ColorSpace::SRGB),
                material["emissive"].as_u64().unwrap() as u32,
                "{label}"
            );
            assert_eq!(
                actual.emissive_intensity,
                material["emissiveIntensity"].as_f64().unwrap(),
                "{label}"
            );
            let side = match material["side"].as_u64().unwrap() {
                0 => Side::Front,
                1 => Side::Back,
                other => panic!("unexpected side {other}"),
            };
            assert_eq!(actual.side, side, "{label}");

            // `geometry.deleteAttribute( 'uv' )`, shared by all eight meshes.
            let geometry = object.geometry().expect("a mesh geometry");
            let mut names: Vec<String> = geometry
                .attributes()
                .map(|(name, _)| name.to_string())
                .collect();
            names.sort();
            let expected_names: Vec<String> = expected["geometry"]["attributes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            assert_eq!(
                names, expected_names,
                "{label}: attributes after the uv delete"
            );
        }

        if let Some(count) = expected.get("count").filter(|v| !v.is_null()) {
            let instanced = object
                .payload
                .instanced_mesh()
                .expect("an InstancedMesh payload");
            assert_eq!(instanced.count, count.as_u64().unwrap() as usize);
            let expected_matrices = floats(&expected["instanceMatrix"]);
            for i in 0..instanced.count {
                let got = instanced.matrix_at(i);
                for e in 0..16 {
                    let want = expected_matrices[i * 16 + e];
                    assert!(
                        (got.elements[e] - want).abs() < EPS,
                        "instance {i} element {e} is {}, three's is {want}",
                        got.elements[e]
                    );
                }
            }
        }
    }

    assert_eq!(standard_count, 2, "the room box and the instanced boxes");
    assert_eq!(
        lambert_intensities,
        vec![50.0, 50.0, 17.0, 43.0, 20.0, 100.0],
        "the six emissive intensities, in the JS's order"
    );
}
