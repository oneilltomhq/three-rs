//! `ObjLoader` against three.js' own `OBJLoader`.
//!
//! No GPU: `parse()` is CPU work. `tests/obj/oracle.json` is written by
//! `tests/obj/gen.mjs`, which runs `OBJLoader.parse` under node over the
//! vendor `models/obj/tree.obj` and over a few synthetic inputs that exercise
//! upstream's quirks. Each attribute is recorded as its count, an FNV-1a-64
//! over its `Float32Array` bytes, and its first values as bit patterns so NaN
//! survives JSON; groups and the materials' `flatShading` / `vertexColors`
//! are recorded whole.
//!
//! `tree.obj` is read from the vendor tree rather than copied in.
//!
//! Regenerate with `node tests/obj/gen.mjs` after a vendor bump.

use serde_json::Value;
use three_rs::core::ObjectRef;
use three_rs::loaders::{Obj, ObjLoader};
use three_rs::objects::Payload;
use three_rs::testing::three_js_dir;

fn oracle() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/obj/oracle.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("the oracle is in the tree"))
        .expect("the oracle is JSON")
}

// FNV-1a 64 over bytes, as `gen.mjs` computes it.
fn fnv1a64(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn check_mesh(case: &str, mesh: &ObjectRef, want: &Value) {
    let object = mesh.borrow();
    assert_eq!(
        object.object_type,
        want["type"].as_str().unwrap(),
        "{case}: type"
    );
    assert_eq!(object.name, want["name"].as_str().unwrap(), "{case}: name");
    let Payload::Mesh(m) = &object.payload else {
        panic!("{case}: not a mesh");
    };

    let attributes = want["attributes"].as_object().unwrap();
    let names: Vec<&str> = m.geometry.attributes().map(|(name, _)| name).collect();
    let mut want_names: Vec<&str> = attributes.keys().map(String::as_str).collect();
    let mut got_names = names.clone();
    want_names.sort_unstable();
    got_names.sort_unstable();
    assert_eq!(got_names, want_names, "{case}: attribute names");

    for (name, want) in attributes {
        let attribute = m.geometry.get_attribute(name).unwrap();
        let array = attribute.array();
        let bytes: Vec<u8> = array.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(
            attribute.count() as u64,
            want["count"].as_u64().unwrap(),
            "{case}: {name} count"
        );
        let head: Vec<u64> = array
            .iter()
            .take(18)
            .map(|v| u64::from(v.to_bits()))
            .collect();
        let want_head: Vec<u64> = want["head"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(head, want_head, "{case}: {name} first values (f32 bits)");
        assert_eq!(
            fnv1a64(&bytes),
            want["hash"].as_str().unwrap(),
            "{case}: {name} hash"
        );
    }

    let groups: Vec<[u64; 3]> = m
        .geometry
        .groups
        .iter()
        .map(|g| [g.start as u64, g.count as u64, g.material_index as u64])
        .collect();
    let want_groups: Vec<[u64; 3]> = want["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            let g = g.as_array().unwrap();
            [0, 1, 2].map(|i| g[i].as_u64().unwrap())
        })
        .collect();
    assert_eq!(groups, want_groups, "{case}: groups");

    let materials: Vec<(bool, bool)> = if m.materials.is_empty() {
        m.material
            .iter()
            .map(|material| (material.flat_shading, material.vertex_colors))
            .collect()
    } else {
        m.materials
            .iter()
            .map(|material| (material.flat_shading, material.vertex_colors))
            .collect()
    };
    let want_materials: Vec<(bool, bool)> = want["materials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["flatShading"].as_bool().unwrap(),
                w["vertexColors"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        materials, want_materials,
        "{case}: materials (flatShading, vertexColors)"
    );
}

fn check(case: &str, obj: &Obj, want: &Value) {
    let libraries: Vec<&str> = want["materialLibraries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        obj.material_libraries, libraries,
        "{case}: materialLibraries"
    );
    let meshes = obj.group.children();
    let want_meshes = want["meshes"].as_array().unwrap();
    assert_eq!(meshes.len(), want_meshes.len(), "{case}: mesh count");
    for (mesh, want) in meshes.iter().zip(want_meshes) {
        check_mesh(case, mesh, want);
    }
}

#[test]
fn tree_obj_matches_three() {
    let path = three_js_dir().join("examples/models/obj/tree.obj");
    let obj = ObjLoader::new().load(&path).expect("tree.obj parses");
    check("tree", &obj, &oracle()["tree"]);
}

#[test]
fn synthetic_inputs_match_three() {
    let oracle = oracle();
    for (case, want) in oracle["synthetic"].as_object().unwrap() {
        let obj = ObjLoader::new()
            .parse(want["text"].as_str().unwrap())
            .unwrap_or_else(|e| panic!("{case}: {e}"));
        check(case, &obj, want);
    }
}

#[test]
fn lines_and_points_are_refused() {
    for text in [
        "v 0 0 0\nv 1 0 0\nl 1 2\n",
        "v 0 0 0\np 1\n",
        "v 0 0 0\nv 1 0 0\n",
    ] {
        assert!(
            ObjLoader::new().parse(text).is_err(),
            "{text:?} builds LineSegments / Points upstream, which is not ported"
        );
    }
    // No content at all is an empty group, as upstream.
    let empty = ObjLoader::new().parse("# nothing\n").unwrap();
    assert!(empty.group.children().is_empty());
}
