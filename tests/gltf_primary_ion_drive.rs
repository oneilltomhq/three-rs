//! `GLTFLoader` against `PrimaryIonDrive.glb`, the whole scene of
//! `webgpu_postprocessing_bloom`.
//!
//! The oracle is three.js' own `GLTFLoader` parse of the same file, dumped from
//! node with no GPU and no network by
//! `tests/fixtures/webgpu_postprocessing_bloom/oracle.mjs` (its header says how
//! to re-run it). Everything asserted here — the node tree, the TRS and both
//! matrices, every attribute's byte hash, the built material records, the clip
//! before and after `optimize()`, and the world matrices after
//! `mixer.update( 0 )` — is a number three produced, not a number this port
//! produced and someone then blessed.
//!
//! Attribute arrays are compared by FNV-1a-32 over their raw little-endian
//! bytes, the same trick `tests/hdr/oracle.json` uses: the file stays 33 KB
//! instead of 30 MB, and a mismatch still says *where* through the `first`
//! probes, which are asserted first.

use std::path::PathBuf;

use serde_json::Value;

use three_rs::animation::{AnimationClip, AnimationMixer};
use three_rs::core::{Index, Node};
use three_rs::loaders::{GLTFLoader, Gltf};
use three_rs::materials::{MaterialKind, MeshBasicNodeMaterial, Side};

const TOLERANCE: f64 = 1e-6;

fn oracle() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/webgpu_postprocessing_bloom/primaryiondrive.json");
    let text = std::fs::read_to_string(&path).expect("the oracle fixture");
    serde_json::from_str(&text).expect("the oracle fixture parses")
}

fn load() -> Gltf {
    let path = three_rs::testing::three_js_dir().join("examples/models/gltf/PrimaryIonDrive.glb");
    GLTFLoader::load(path).expect("PrimaryIonDrive.glb loads")
}

/// The scene in `traverse()` order, which is the order the oracle's `nodes` is
/// in.
fn traversal(gltf: &Gltf) -> Vec<Node> {
    let mut out = Vec::new();
    gltf.scene.traverse(&mut |node| out.push(node.clone()));
    out
}

/// FNV-1a-32 over raw bytes — `oracle.mjs`'s `hash()`.
fn fnv1a(bytes: impl Iterator<Item = u8>) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

fn hash_f32(values: &[f32]) -> u32 {
    fnv1a(values.iter().flat_map(|v| v.to_le_bytes()))
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("an array of numbers")
        .iter()
        .map(|v| v.as_f64().expect("a number"))
        .collect()
}

#[track_caller]
fn close(got: f64, expected: f64, what: &str) {
    assert!(
        (got - expected).abs() < TOLERANCE,
        "{what}: {got} vs three's {expected}"
    );
}

#[track_caller]
fn close_all(got: &[f64], expected: &Value, what: &str) {
    let expected = numbers(expected);
    assert_eq!(got.len(), expected.len(), "{what}: length");
    for (i, (got, expected)) in got.iter().zip(&expected).enumerate() {
        close(*got, *expected, &format!("{what}[{i}]"));
    }
}

/// §5.1 item 1 — the node tree. `nodeCount`, then `(name, type, parent)` in
/// traverse order: the two unnamed nodes and the six-deep transform chain make
/// this the first thing that would go wrong.
#[test]
fn node_tree_matches_three() {
    let gltf = load();
    let oracle = oracle();

    assert_eq!(
        gltf.scene.borrow().name,
        oracle["sceneName"].as_str().unwrap()
    );

    let nodes = traversal(&gltf);
    assert_eq!(nodes.len(), oracle["nodeCount"].as_u64().unwrap() as usize);

    for (node, expected) in nodes.iter().zip(oracle["nodes"].as_array().unwrap()) {
        let object = node.borrow();
        let name = expected["name"].as_str().unwrap();
        assert_eq!(object.name, name, "node name");

        // `Group` on the scene root, `Mesh` on a drawable, `Object3D` on the
        // pure-transform nodes.
        assert_eq!(
            object.object_type,
            expected["type"].as_str().unwrap(),
            "{name}: type"
        );

        let parent = object
            .parent
            .as_ref()
            .and_then(|p| p.upgrade())
            .map(|p| p.borrow().name.clone());
        assert_eq!(
            parent.as_deref(),
            expected["parent"].as_str(),
            "{name}: parent"
        );

        assert_eq!(
            object.visible,
            expected["visible"].as_bool().unwrap(),
            "{name}: visible"
        );
        assert_eq!(
            object.cast_shadow,
            expected["castShadow"].as_bool().unwrap(),
            "{name}: castShadow"
        );
        assert_eq!(
            object.receive_shadow,
            expected["receiveShadow"].as_bool().unwrap(),
            "{name}: receiveShadow"
        );
        assert_eq!(
            object.render_order,
            expected["renderOrder"].as_f64().unwrap(),
            "{name}: renderOrder"
        );
    }
}

/// §5.1 item 2 — the TRS a node came out of `applyMatrix4`/`fromArray` with, and
/// both matrices after `updateMatrixWorld( true )`.
#[test]
fn transforms_match_three() {
    let gltf = load();
    let oracle = oracle();

    gltf.scene.update_matrix_world(true);

    for (node, expected) in traversal(&gltf)
        .iter()
        .zip(oracle["nodes"].as_array().unwrap())
    {
        let object = node.borrow();
        let name = expected["name"].as_str().unwrap();

        close_all(
            &object.position.to_array(),
            &expected["position"],
            &format!("{name} position"),
        );
        close_all(
            &object.quaternion.to_array(),
            &expected["quaternion"],
            &format!("{name} quaternion"),
        );
        close_all(
            &object.scale.to_array(),
            &expected["scale"],
            &format!("{name} scale"),
        );
        close_all(
            &object.matrix.to_array(),
            &expected["matrix"],
            &format!("{name} matrix"),
        );
        close_all(
            &object.matrix_world.to_array(),
            &expected["matrixWorld"],
            &format!("{name} matrixWorld"),
        );
    }
}

/// §5.1 item 3 — every attribute's item size, count, array type and byte hash,
/// and the index's. The index is the one to watch: this file's indices are
/// `UNSIGNED_INT`, and three keeps them `Uint32Array` however small the values
/// are.
#[test]
fn geometry_matches_three() {
    let gltf = load();
    let oracle = oracle();

    let mut meshes = 0;
    for (node, expected) in traversal(&gltf)
        .iter()
        .zip(oracle["nodes"].as_array().unwrap())
    {
        let Some(expected) = expected.get("geometry") else {
            continue;
        };
        meshes += 1;

        let name = node.borrow().name.clone();
        let object = node.borrow();
        let geometry = object
            .payload
            .geometry()
            .unwrap_or_else(|| panic!("{name}: no geometry"));

        assert!(
            geometry.groups.is_empty(),
            "{name}: groups — three has none for a single-primitive mesh"
        );
        assert_eq!(
            geometry.morph_targets_relative,
            expected["morphTargetsRelative"].as_bool().unwrap(),
            "{name}: morphTargetsRelative"
        );
        assert_eq!(
            geometry.morph_attributes().count(),
            0,
            "{name}: morphAttributes"
        );

        for (attribute_name, expected) in expected["attributes"].as_object().unwrap() {
            if attribute_name == "index" {
                let index = geometry
                    .index
                    .as_ref()
                    .unwrap_or_else(|| panic!("{name}: no index"));
                assert_eq!(
                    index.count(),
                    expected["count"].as_u64().unwrap() as usize,
                    "{name}: index count"
                );

                let (kind, hash) = match index {
                    Index::U16(values) => (
                        "Uint16Array",
                        fnv1a(values.iter().flat_map(|v| v.to_le_bytes())),
                    ),
                    Index::U32(values) => (
                        "Uint32Array",
                        fnv1a(values.iter().flat_map(|v| v.to_le_bytes())),
                    ),
                };
                assert_eq!(
                    kind,
                    expected["array"].as_str().unwrap(),
                    "{name}: index array type"
                );

                let first: Vec<f64> = match index {
                    Index::U16(values) => values.iter().take(6).map(|&v| v as f64).collect(),
                    Index::U32(values) => values.iter().take(6).map(|&v| v as f64).collect(),
                };
                close_all(&first, &expected["first"], &format!("{name} index first"));
                assert_eq!(
                    hash,
                    expected["hash"].as_u64().unwrap() as u32,
                    "{name}: index hash"
                );
                continue;
            }

            let attribute = geometry
                .get_attribute(attribute_name)
                .unwrap_or_else(|| panic!("{name}: no {attribute_name} attribute"));

            assert_eq!(
                attribute.item_size,
                expected["itemSize"].as_u64().unwrap() as usize,
                "{name}: {attribute_name} itemSize"
            );
            assert_eq!(
                attribute.count(),
                expected["count"].as_u64().unwrap() as usize,
                "{name}: {attribute_name} count"
            );
            assert!(
                !expected["normalized"].as_bool().unwrap(),
                "{name}: {attribute_name} — three did not normalize this accessor"
            );
            assert_eq!(
                "Float32Array",
                expected["array"].as_str().unwrap(),
                "{name}: {attribute_name} array type"
            );

            let values = attribute.array();
            let probes: Vec<f64> = values
                .iter()
                .take(attribute.item_size * 2)
                .map(|&v| v as f64)
                .collect();
            close_all(
                &probes,
                &expected["first"],
                &format!("{name} {attribute_name} first"),
            );

            assert_eq!(
                hash_f32(&values),
                expected["hash"].as_u64().unwrap() as u32,
                "{name}: {attribute_name} bytes"
            );
        }
    }

    assert_eq!(meshes, 6, "six primitives, each its own glTF node");
}

fn side_of(side: Side) -> u64 {
    match side {
        Side::Front => 0,
        Side::Back => 1,
        Side::Double => 2,
    }
}

/// §5.1 item 4 — the whole material record per mesh. This is what
/// `assignFinalMaterial`, `alphaMode` and `emissiveFactor` are gated on:
/// `vertexColors` on all three materials, `transparent` / `depthWrite` /
/// `opacity` on `HoloFillDark`, `side: 2` on the other two, the emissive
/// factors, and the `normalScale.y = -1` that the tangent-less `circle2`
/// primitive earns from `useDerivativeTangents`.
#[test]
fn materials_match_three() {
    let gltf = load();
    let oracle = oracle();

    let mut seen = 0;
    for (node, expected) in traversal(&gltf)
        .iter()
        .zip(oracle["nodes"].as_array().unwrap())
    {
        let Some(expected) = expected.get("material") else {
            continue;
        };
        seen += 1;

        let name = node.borrow().name.clone();
        let object = node.borrow();
        let material: &MeshBasicNodeMaterial = object
            .payload
            .material()
            .unwrap_or_else(|| panic!("{name}: no material"));

        // `getMaterialType()` returns `MeshStandardMaterial` and this asset uses
        // neither `KHR_materials_ior` nor `_specular`, so nothing is promoted to
        // physical.
        assert_eq!(
            expected["type"].as_str().unwrap(),
            "MeshStandardMaterial",
            "{name}: the oracle's material type"
        );
        assert_eq!(material.kind, MaterialKind::Standard, "{name}: kind");

        assert_eq!(
            side_of(material.side),
            expected["side"].as_u64().unwrap(),
            "{name}: side"
        );
        assert_eq!(
            material.transparent,
            expected["transparent"].as_bool().unwrap(),
            "{name}: transparent"
        );
        assert_eq!(
            material.depth_write,
            expected["depthWrite"].as_bool().unwrap(),
            "{name}: depthWrite"
        );
        assert_eq!(
            material.vertex_colors,
            expected["vertexColors"].as_bool().unwrap(),
            "{name}: vertexColors"
        );
        assert_eq!(
            material.flat_shading,
            expected["flatShading"].as_bool().unwrap(),
            "{name}: flatShading"
        );
        close(
            material.opacity,
            expected["opacity"].as_f64().unwrap(),
            &format!("{name} opacity"),
        );
        close(
            material.metalness,
            expected["metalness"].as_f64().unwrap(),
            &format!("{name} metalness"),
        );
        close(
            material.roughness,
            expected["roughness"].as_f64().unwrap(),
            &format!("{name} roughness"),
        );
        close(
            material.emissive_intensity,
            expected["emissiveIntensity"].as_f64().unwrap(),
            &format!("{name} emissiveIntensity"),
        );

        // `alphaTest` is 0 on all three; the port has only the node form, so the
        // gate is that no discard node was installed.
        assert_eq!(
            expected["alphaTest"].as_f64().unwrap(),
            0.0,
            "{name}: the oracle's alphaTest"
        );
        assert!(material.alpha_test_node.is_none(), "{name}: alphaTestNode");

        let color = numbers(&expected["color"]);
        close(material.color.r, color[0], &format!("{name} color.r"));
        close(material.color.g, color[1], &format!("{name} color.g"));
        close(material.color.b, color[2], &format!("{name} color.b"));

        let emissive = numbers(&expected["emissive"]);
        close(
            material.emissive.r,
            emissive[0],
            &format!("{name} emissive.r"),
        );
        close(
            material.emissive.g,
            emissive[1],
            &format!("{name} emissive.g"),
        );
        close(
            material.emissive.b,
            emissive[2],
            &format!("{name} emissive.b"),
        );

        let normal_scale = numbers(&expected["normalScale"]);
        close(
            material.normal_scale.x,
            normal_scale[0],
            &format!("{name} normalScale.x"),
        );
        close(
            material.normal_scale.y,
            normal_scale[1],
            &format!("{name} normalScale.y"),
        );

        for map in [
            "map",
            "normalMap",
            "roughnessMap",
            "metalnessMap",
            "specularColorMap",
        ] {
            assert!(expected[map].is_null(), "{name}: the oracle's {map}");
        }
        assert!(material.map.is_none(), "{name}: map");
        assert!(material.normal_map.is_none(), "{name}: normalMap");
        assert!(material.roughness_map.is_none(), "{name}: roughnessMap");
        assert!(material.metalness_map.is_none(), "{name}: metalnessMap");
        assert!(
            material.specular_color_map.is_none(),
            "{name}: specularColorMap"
        );
    }

    assert_eq!(seen, 6);
}

/// `assignFinalMaterial`'s variant flags, read back off the materials the loader
/// installed. The two primitives that share the `constant2` glTF material do
/// *not* share a variant — `circle2_constant2_0` has no `TANGENT` attribute, so
/// only it earns the `useDerivativeTangents` clone and its `normalScale.y = -1`.
/// Keying the cache on the glTF material index alone would give one of them the
/// other's frame.
#[test]
fn the_material_variant_flags_follow_the_geometry() {
    let gltf = load();

    let mut by_name = std::collections::HashMap::new();
    gltf.scene.traverse(&mut |node| {
        let object = node.borrow();
        if let Some(material) = object.payload.material() {
            by_name.insert(
                object.name.clone(),
                (
                    material.normal_scale.y,
                    material.vertex_colors,
                    material.flat_shading,
                ),
            );
        }
    });

    assert_eq!(by_name["circle1_constant2_0"], (1.0, true, false));
    assert_eq!(by_name["circle2_constant2_0"], (-1.0, true, false));
    assert_eq!(by_name["circle_constant1_0"], by_name["geo1_constant1_0"]);
    assert_eq!(
        by_name["circle_HoloFillDark_0"],
        by_name["geo1_HoloFillDark_0"]
    );
}

fn clip_record(clip: &AnimationClip) -> Vec<(String, usize, usize, u32, u32)> {
    clip.tracks
        .iter()
        .map(|track| {
            let times: Vec<f32> = track.times.iter().map(|&t| t as f32).collect();
            let values: Vec<f32> = track.values.iter().map(|&v| v as f32).collect();
            (
                track.name.clone(),
                track.times.len(),
                track.values.len() / track.times.len(),
                hash_f32(&times),
                hash_f32(&values),
            )
        })
        .collect()
}

#[track_caller]
fn assert_clip(clip: &AnimationClip, expected: &Value) {
    assert_eq!(clip.name, expected["name"].as_str().unwrap(), "clip name");
    close(
        clip.duration,
        expected["duration"].as_f64().unwrap(),
        "clip duration",
    );

    let tracks = expected["tracks"].as_array().unwrap();
    let got = clip_record(clip);
    assert_eq!(got.len(), tracks.len(), "track count");

    for ((name, times, value_size, times_hash, values_hash), expected) in got.iter().zip(tracks) {
        let what = expected["name"].as_str().unwrap();
        assert_eq!(name, what, "track name");
        assert_eq!(
            *times,
            expected["times"].as_u64().unwrap() as usize,
            "{what}: keyframes"
        );
        assert_eq!(
            *value_size,
            expected["valueSize"].as_u64().unwrap() as usize,
            "{what}: valueSize"
        );
        assert_eq!(
            *times_hash,
            expected["timesHash"].as_u64().unwrap() as u32,
            "{what}: times"
        );
        assert_eq!(
            *values_hash,
            expected["valuesHash"].as_u64().unwrap() as u32,
            "{what}: values"
        );
    }
}

/// §5.1 item 5 — the clip as loaded, and after `optimize()`. `optimize()` is not
/// a no-op here: `circle1` and `circle2` drop from 879 keyframes to 690, and the
/// example plays the optimized clip.
#[test]
fn animation_matches_three_before_and_after_optimize() {
    let gltf = load();
    let oracle = oracle();

    assert_eq!(gltf.animations.len(), 1);
    assert_clip(&gltf.animations[0], &oracle["animations"][0]);

    let mut optimized = gltf.animations[0].clone();
    optimized.optimize();
    assert_clip(&optimized, &oracle["animationsOptimized"][0]);
}

/// §5.1 item 6 — the graded frame. `timer.getDelta()` is 0 under the harness, so
/// the example's `mixer.update( delta )` is `update( 0 )` on the optimized clip;
/// these twenty world matrices are the scene the renderer is handed.
#[test]
fn world_matrices_at_t0_match_three() {
    let gltf = load();
    let oracle = oracle();

    let mut clip = gltf.animations[0].clone();
    clip.optimize();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&clip, None, None);
    mixer.play(action);
    mixer.update(0.0);

    gltf.scene.update_matrix_world(true);

    for (node, expected) in traversal(&gltf)
        .iter()
        .zip(oracle["atT0"].as_array().unwrap())
    {
        let object = node.borrow();
        assert_eq!(object.name, expected["name"].as_str().unwrap());
        close_all(
            &object.matrix_world.to_array(),
            &expected["matrixWorld"],
            &format!("{} matrixWorld at t=0", object.name),
        );
    }
}
