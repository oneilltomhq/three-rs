//! `KHR_draco_mesh_compression` against three.js' own `DRACOLoader`.
//!
//! `tools/draco_reference.mjs` runs `GLTFLoader` + `DRACOLoader` (the JS
//! build of libdraco) under node over each Draco asset in the three.js
//! examples and prints every Draco primitive's attributes and index. Each
//! test here decodes the same asset and asserts, per primitive:
//!
//! * [`GLTFLoader::draco_primitives`], the typed arrays as three.js has them:
//!   the same attribute names, typed-array types, `itemSize`s and
//!   `normalized` flags, integers and the index exactly, floats to 1e-6;
//! * [`GLTFLoader::load`], the geometry this crate draws: each attribute
//!   widened to `f32` (normalized ones scaled into range) matches, to 1e-6,
//!   and the index is the same `Uint32Array`.
//!
//! Skipped, with a note, when there is no three.js checkout or no node.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use three_rs::core::Index;
use three_rs::loaders::draco::{DracoArray, DracoPrimitive};
use three_rs::loaders::GLTFLoader;

const TOLERANCE: f64 = 1e-6;

/// One attribute as the reference printed it.
struct Expected {
    type_name: String,
    item_size: usize,
    normalized: bool,
    /// The typed array's elements, widened to `f64` (exact for every type).
    values: Vec<f64>,
}

struct ExpectedPrimitive {
    mesh: usize,
    primitive: usize,
    index: Option<Vec<u32>>,
    attributes: BTreeMap<String, Expected>,
}

fn asset(name: &str) -> PathBuf {
    three_rs::testing::three_js_dir()
        .join("examples/models/gltf")
        .join(name)
}

fn base64(text: &str) -> Vec<u8> {
    let value = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("bad base64 character {c}"),
        }
    };
    let bytes = text.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= value(c) << (18 - 6 * i);
        }
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    out
}

/// The elements of a little-endian typed array, by constructor name.
fn elements(type_name: &str, bytes: &[u8]) -> Vec<f64> {
    macro_rules! read {
        ($t:ty) => {
            bytes
                .chunks_exact(std::mem::size_of::<$t>())
                .map(|b| <$t>::from_le_bytes(b.try_into().unwrap()) as f64)
                .collect()
        };
    }
    match type_name {
        "Int8Array" => read!(i8),
        "Uint8Array" => read!(u8),
        "Int16Array" => read!(i16),
        "Uint16Array" => read!(u16),
        "Int32Array" => read!(i32),
        "Uint32Array" => read!(u32),
        "Float32Array" => read!(f32),
        other => panic!("unexpected typed array {other}"),
    }
}

/// Runs the reference script over one asset.
fn reference(name: &str) -> Option<Vec<ExpectedPrimitive>> {
    let three = three_rs::testing::three_js_dir();
    let path = asset(name);
    if !path.exists() {
        eprintln!(
            "skipping: no {} in the three.js checkout at {} (set THREE_JS_DIR)",
            name,
            three.display()
        );
        return None;
    }

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/draco_reference.mjs");
    let output = match Command::new("node")
        .arg(&script)
        .arg(&three)
        .arg(&path)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("skipping: cannot run node ({error})");
            return None;
        }
    };
    assert!(
        output.status.success(),
        "the reference script failed on {name}:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the reference script's JSON");
    let primitives = json["primitives"].as_array().expect("primitives");

    Some(
        primitives
            .iter()
            .map(|p| ExpectedPrimitive {
                mesh: p["mesh"].as_u64().unwrap() as usize,
                primitive: p["primitive"].as_u64().unwrap() as usize,
                index: p["index"].as_str().map(|index| {
                    assert_eq!(p["indexType"], "Uint32Array");
                    base64(index)
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|&b| u32::from_le_bytes(b))
                        .collect()
                }),
                attributes: p["attributes"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(name, a)| {
                        let type_name = a["type"].as_str().unwrap().to_string();
                        let values = elements(&type_name, &base64(a["data"].as_str().unwrap()));
                        (
                            name.clone(),
                            Expected {
                                type_name,
                                item_size: a["itemSize"].as_u64().unwrap() as usize,
                                normalized: a["normalized"].as_bool().unwrap(),
                                values,
                            },
                        )
                    })
                    .collect(),
            })
            .collect(),
    )
}

/// Compares `got` with `want`: exactly for integers, to [`TOLERANCE`] for
/// floats. Says where and by how much when they differ.
fn compare(what: &str, want: &[f64], got: &[f64], exact: bool) -> Option<String> {
    if want.len() != got.len() {
        return Some(format!(
            "{what}: three.js has {} values, the port {}",
            want.len(),
            got.len()
        ));
    }
    let mut worst = (0usize, 0.0f64);
    let mut bad = 0usize;
    for (i, (w, g)) in want.iter().zip(got).enumerate() {
        let off = (w - g).abs();
        let fails = if exact {
            off != 0.0
        } else {
            off.is_nan() || off > TOLERANCE
        };
        if fails {
            bad += 1;
            if off.is_nan() || off > worst.1 {
                worst = (i, off);
            }
        }
    }
    (bad > 0).then(|| {
        format!(
            "{what}: {bad} of {} values differ; worst is [{}]: three.js {}, the port {} (off by {:e})",
            want.len(),
            worst.0,
            want[worst.0],
            got[worst.0],
            worst.1
        )
    })
}

fn is_float(array: &DracoArray) -> bool {
    matches!(array, DracoArray::F32(_))
}

/// `getNormalizedComponentScale`, applied the way the loader applies it.
fn widened(e: &Expected) -> Vec<f64> {
    let scale = match (e.normalized, e.type_name.as_str()) {
        (true, "Int8Array") => 1.0 / 127.0,
        (true, "Uint8Array") => 1.0 / 255.0,
        (true, "Int16Array") => 1.0 / 32767.0,
        (true, "Uint16Array") => 1.0 / 65535.0,
        _ => 1.0,
    };
    e.values
        .iter()
        .map(|&v| (v * scale) as f32 as f64)
        .collect()
}

fn check(name: &str) {
    let Some(expected) = reference(name) else {
        return;
    };
    assert!(
        !expected.is_empty(),
        "{name}: the reference found no Draco primitives"
    );

    // --- the typed arrays, as DRACOLoader hands them over -----------------
    let decoded: Vec<(usize, usize, DracoPrimitive)> =
        GLTFLoader::draco_primitives(asset(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        decoded.iter().map(|(m, p, _)| (*m, *p)).collect::<Vec<_>>(),
        expected
            .iter()
            .map(|e| (e.mesh, e.primitive))
            .collect::<Vec<_>>(),
        "{name}: the Draco primitives"
    );

    let mut failures = Vec::new();
    for ((m, p, got), want) in decoded.iter().zip(&expected) {
        let at = format!("{name} mesh {m} primitive {p}");
        if got.index != want.index {
            failures.push(format!("{at}: the index differs"));
        }

        let got_names: Vec<&str> = got.attributes.iter().map(|a| a.name.as_str()).collect();
        let mut got_sorted = got_names.clone();
        got_sorted.sort_unstable();
        assert_eq!(
            got_sorted,
            want.attributes
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "{at}: the attribute names"
        );

        for attribute in &got.attributes {
            let e = &want.attributes[&attribute.name];
            let what = format!("{at} {}", attribute.name);
            assert_eq!(
                attribute.array.type_name(),
                e.type_name,
                "{what}: the array type"
            );
            assert_eq!(attribute.item_size, e.item_size, "{what}: itemSize");
            assert_eq!(attribute.normalized, e.normalized, "{what}: normalized");
            failures.extend(compare(
                &what,
                &e.values,
                &attribute.array.to_f64(),
                !is_float(&attribute.array),
            ));
        }
    }

    assert!(failures.is_empty(), "{name}:\n  {}", failures.join("\n  "));

    // --- the geometry the crate draws -------------------------------------
    let gltf = match GLTFLoader::load(asset(name)) {
        Ok(gltf) => gltf,
        // AVIF textures are not decoded (docs/nodes.md §30); the typed
        // arrays above are the Draco half of that asset.
        Err(e)
            if e.to_string()
                .contains("unknown required extension \"EXT_texture_avif") =>
        {
            eprintln!("{name}: geometry checked, the load itself stops at: {e}");
            return;
        }
        Err(e) => panic!("{name}: {e}"),
    };

    // `gltf.primitives` is in node order: each node with a mesh, that mesh's
    // primitives in order.
    let mut order = Vec::new();
    for node in gltf.json["nodes"].as_array().into_iter().flatten() {
        if let Some(mesh) = node["mesh"].as_u64() {
            let count = gltf.json["meshes"][mesh as usize]["primitives"]
                .as_array()
                .map_or(0, Vec::len);
            order.extend((0..count).map(|p| (mesh as usize, p)));
        }
    }
    assert_eq!(
        order.len(),
        gltf.primitives.len(),
        "{name}: primitives in node order"
    );

    let mut checked = 0;
    for ((m, p), primitive) in order.iter().zip(&gltf.primitives) {
        let Some(want) = expected.iter().find(|e| (e.mesh, e.primitive) == (*m, *p)) else {
            continue;
        };
        let at = format!("{name} mesh {m} primitive {p} (loaded)");
        let geometry = &primitive.geometry;

        match (&geometry.index, &want.index) {
            (Some(Index::U32(got)), Some(want)) => assert_eq!(got, want, "{at}: the index"),
            (None, None) => {}
            (got, _) => panic!("{at}: the index is {got:?}, three.js has a Uint32Array"),
        }

        for (attribute_name, e) in &want.attributes {
            let attribute = geometry
                .get_attribute(attribute_name)
                .unwrap_or_else(|| panic!("{at}: no {attribute_name}"));
            assert_eq!(
                attribute.item_size, e.item_size,
                "{at} {attribute_name}: itemSize"
            );
            let got: Vec<f64> = attribute.array().iter().map(|&v| v as f64).collect();
            if let Some(failure) =
                compare(&format!("{at} {attribute_name}"), &widened(e), &got, false)
            {
                panic!("{failure}");
            }
        }
        checked += 1;
    }
    assert!(checked > 0, "{name}: no Draco primitive is in the scene");
}

macro_rules! assets {
    ($($test:ident => $file:literal,)*) => {
        $(
            #[test]
            fn $test() {
                check($file);
            }
        )*
    };
}

assets! {
    duck => "duck.glb",
    gears => "gears.glb",
    iridescent_dish_with_olives => "IridescentDishWithOlives.glb",
    shader_ball_2 => "ShaderBall2.glb",
    littlest_tokyo => "LittlestTokyo.glb",
    carbon_frame_bike => "CarbonFrameBike.glb",
    bath_day => "bath_day.glb",
    pool => "pool.glb",
    forest_house => "AVIFTest/forest_house.glb",
    ferrari => "ferrari.glb",
    rolex => "rolex.glb",
    tennyson_bust => "tennyson-bust.glb",
    nemetona => "nemetona.glb",
    venice_mask => "venice_mask.glb",
    kira => "kira.glb",
}
