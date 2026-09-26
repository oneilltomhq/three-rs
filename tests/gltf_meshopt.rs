//! `EXT_meshopt_compression` against three.js' own `MeshoptDecoder`.
//!
//! `tools/meshopt_reference.mjs` runs `GLTFLoader` + `MeshoptDecoder` (the
//! WebAssembly build of meshoptimizer 1.1 three.js ships) under node. Two
//! kinds of test here:
//!
//! * One per asset in the three.js examples that uses the extension
//!   (`coffeemat.glb`, `facecap.glb`; a grep of `examples/models` for
//!   `EXT_meshopt_compression` finds no others). Each asserts
//!   - every compressed bufferView decodes to the same bytes;
//!   - every accessor, as [`GLTFLoader::accessors`] reads it, matches
//!     `loadAccessor`'s typed array: integers exactly, and floats and
//!     normalized values (which the port scales into range) to 1e-6;
//!   - every primitive [`GLTFLoader::parse`] builds has the attributes,
//!     morph attributes and index `loadGeometries` builds, to 1e-6.
//! * Sweeps of the codecs and filters over inputs the two assets do not
//!   reach: every 8-bit octahedral `x, y`, random 16-bit octahedral,
//!   quaternion and exponential words, the `INDICES` mode, which neither
//!   asset uses, and the widest stride. The stream is encoded here with
//!   `meshopt-rs`, decoded by three.js' decoder and by the port, and the
//!   bytes compared exactly.
//!
//! Skipped, with a note, when there is no three.js checkout or no node.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use meshopt_rs::index::buffer::{encode_index_buffer, encode_index_buffer_bound};
use meshopt_rs::index::sequence::{encode_index_sequence, encode_index_sequence_bound};
use meshopt_rs::index::IndexEncodingVersion;
use meshopt_rs::vertex::buffer::{encode_vertex_buffer, encode_vertex_buffer_bound};
use meshopt_rs::vertex::VertexEncodingVersion;
use serde_json::Value;
use three_rs::core::{BufferAttribute, Index};
use three_rs::loaders::meshopt::{decode_gltf_buffer, Filter, Mode};
use three_rs::loaders::GLTFLoader;

const TOLERANCE: f64 = 1e-6;

fn asset(name: &str) -> PathBuf {
    three_rs::testing::three_js_dir()
        .join("examples/models/gltf")
        .join(name)
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/meshopt_reference.mjs")
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

/// Runs the reference script with `args`; `None` (and a note) when node is
/// missing.
fn node(args: &[&std::ffi::OsStr]) -> Option<Vec<u8>> {
    let output = match Command::new("node")
        .arg(script())
        .arg(three_rs::testing::three_js_dir())
        .args(args)
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
        "the reference script failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(output.stdout)
}

fn three_js_present() -> bool {
    let three = three_rs::testing::three_js_dir();
    let decoder = three.join("examples/jsm/libs/meshopt_decoder.module.js");
    if !decoder.exists() {
        eprintln!(
            "skipping: no meshopt_decoder.module.js in the three.js checkout at {} (set THREE_JS_DIR)",
            three.display()
        );
    }
    decoder.exists()
}

// --- the assets ------------------------------------------------------------

/// A typed array as the reference printed it.
struct Typed {
    type_name: String,
    item_size: usize,
    normalized: bool,
    /// The elements, widened to `f64` (exact for every type).
    values: Vec<f64>,
}

impl Typed {
    fn from_json(value: &Value) -> Self {
        let type_name = value["type"].as_str().unwrap().to_string();
        let bytes = base64(value["data"].as_str().unwrap());
        macro_rules! read {
            ($t:ty) => {
                bytes
                    .chunks_exact(std::mem::size_of::<$t>())
                    .map(|b| <$t>::from_le_bytes(b.try_into().unwrap()) as f64)
                    .collect()
            };
        }
        let values = match type_name.as_str() {
            "Int8Array" => read!(i8),
            "Uint8Array" => read!(u8),
            "Int16Array" => read!(i16),
            "Uint16Array" => read!(u16),
            "Int32Array" => read!(i32),
            "Uint32Array" => read!(u32),
            "Float32Array" => read!(f32),
            other => panic!("unexpected typed array {other}"),
        };
        Self {
            type_name,
            item_size: value["itemSize"].as_u64().unwrap() as usize,
            normalized: value["normalized"].as_bool().unwrap(),
            values,
        }
    }

    /// Integers not scaled by `normalized` are compared exactly.
    fn exact(&self) -> bool {
        self.type_name != "Float32Array" && !self.normalized
    }

    /// What the port reads: `getNormalizedComponentScale` applied to a
    /// normalized array, as [`GLTFLoader::accessor`] applies it.
    fn widened(&self) -> Vec<f64> {
        let scale = match (self.normalized, self.type_name.as_str()) {
            (true, "Int8Array") => 1.0 / 127.0,
            (true, "Uint8Array") => 1.0 / 255.0,
            (true, "Int16Array") => 1.0 / 32767.0,
            (true, "Uint16Array") => 1.0 / 65535.0,
            _ => 1.0,
        };
        self.values.iter().map(|&v| v * scale).collect()
    }
}

/// Compares `got` with `want`: exactly, or to [`TOLERANCE`]. Says where and
/// by how much when they differ.
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

/// `path`'s GLB with `KHR_texture_basisu` dropped from
/// `extensionsRequired`, so that [`GLTFLoader::parse`] builds it. Both meshopt
/// assets use KTX2 textures, which are issue #172; the reference stubs
/// textures out too, and nothing geometric depends on them.
fn without_basisu(path: &Path) -> Vec<u8> {
    let glb = std::fs::read(path).unwrap();
    let json_length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let mut json: Value = serde_json::from_slice(&glb[20..20 + json_length]).unwrap();
    if let Some(required) = json["extensionsRequired"].as_array_mut() {
        required.retain(|name| name != "KHR_texture_basisu");
    }

    let mut chunk = serde_json::to_vec(&json).unwrap();
    while !chunk.len().is_multiple_of(4) {
        chunk.push(b' ');
    }
    let rest = &glb[20 + json_length..];

    let mut out = Vec::new();
    out.extend_from_slice(&glb[0..8]);
    out.extend_from_slice(&((12 + 8 + chunk.len() + rest.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&chunk);
    out.extend_from_slice(rest);
    out
}

fn attribute_values(attribute: &BufferAttribute) -> Vec<f64> {
    attribute.array().iter().map(|&v| v as f64).collect()
}

fn check(name: &str) {
    if !three_js_present() {
        return;
    }
    let path = asset(name);
    let Some(stdout) = node(&[path.as_os_str()]) else {
        return;
    };
    let reference: Value = serde_json::from_slice(&stdout).expect("the reference script's JSON");
    let mut failures = Vec::new();

    // --- the decoded bufferViews, byte for byte ---------------------------
    let want: Vec<(usize, Vec<u8>)> = reference["bufferViews"]
        .as_array()
        .unwrap()
        .iter()
        .map(|view| {
            (
                view["index"].as_u64().unwrap() as usize,
                base64(view["data"].as_str().unwrap()),
            )
        })
        .collect();
    assert!(!want.is_empty(), "{name}: no compressed bufferViews");
    let got = GLTFLoader::meshopt_buffer_views(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(
        got.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        want.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        "{name}: the compressed bufferViews"
    );
    for ((index, got), (_, want)) in got.iter().zip(&want) {
        if got != want {
            let differ = got.iter().zip(want).filter(|(a, b)| a != b).count();
            failures.push(format!(
                "{name} bufferView {index}: {differ} of {} bytes differ (lengths {} and {})",
                want.len(),
                want.len(),
                got.len()
            ));
        }
    }

    // --- every accessor ---------------------------------------------------
    let want: Vec<Typed> = reference["accessors"]
        .as_array()
        .unwrap()
        .iter()
        .map(Typed::from_json)
        .collect();
    let got = GLTFLoader::accessors(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(got.len(), want.len(), "{name}: the accessor count");
    for (index, ((values, item_size), want)) in got.iter().zip(&want).enumerate() {
        let what = format!("{name} accessor {index} ({})", want.type_name);
        assert_eq!(*item_size, want.item_size, "{what}: itemSize");
        failures.extend(compare(&what, &want.widened(), values, want.exact()));
    }

    // --- the geometry the crate builds ------------------------------------
    let gltf = GLTFLoader::parse(&without_basisu(&path), path.parent().unwrap().to_path_buf())
        .unwrap_or_else(|e| panic!("{name}: {e}"));

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
    assert_eq!(order.len(), gltf.primitives.len(), "{name}: primitives");

    let primitives = reference["primitives"].as_array().unwrap();
    for ((m, p), primitive) in order.iter().zip(&gltf.primitives) {
        let want = primitives
            .iter()
            .find(|e| e["mesh"] == *m && e["primitive"] == *p)
            .unwrap_or_else(|| panic!("{name}: the reference has no mesh {m} primitive {p}"));
        let at = format!("{name} mesh {m} primitive {p}");
        let geometry = &primitive.geometry;

        let index = (!want["index"].is_null()).then(|| Typed::from_json(&want["index"]));
        match (&geometry.index, &index) {
            (Some(Index::U16(got)), Some(want)) if want.type_name == "Uint16Array" => {
                let got: Vec<f64> = got.iter().map(|&i| i as f64).collect();
                failures.extend(compare(&format!("{at} index"), &want.values, &got, true));
            }
            (Some(Index::U32(got)), Some(want)) if want.type_name == "Uint32Array" => {
                let got: Vec<f64> = got.iter().map(|&i| i as f64).collect();
                failures.extend(compare(&format!("{at} index"), &want.values, &got, true));
            }
            (None, None) => {}
            (got, want) => panic!(
                "{at}: the index is {:?}, three.js has {:?}",
                got.as_ref().map(|_| "an index"),
                want.as_ref().map(|w| &w.type_name)
            ),
        }

        let names: Vec<&String> = want["attributes"].as_object().unwrap().keys().collect();
        for attribute_name in names {
            let e = Typed::from_json(&want["attributes"][attribute_name]);
            let attribute = geometry
                .get_attribute(attribute_name)
                .unwrap_or_else(|| panic!("{at}: no {attribute_name}"));
            assert_eq!(
                attribute.item_size, e.item_size,
                "{at} {attribute_name}: itemSize"
            );
            failures.extend(compare(
                &format!("{at} {attribute_name}"),
                &e.widened(),
                &attribute_values(attribute),
                false,
            ));
        }

        let got_morph: BTreeMap<&str, &[BufferAttribute]> = geometry.morph_attributes().collect();
        let want_morph = want["morphAttributes"].as_object().unwrap();
        assert_eq!(
            got_morph.keys().copied().collect::<Vec<_>>(),
            want_morph.keys().map(String::as_str).collect::<Vec<_>>(),
            "{at}: the morph attributes"
        );
        for (attribute_name, list) in want_morph {
            let list = list.as_array().unwrap();
            let got = got_morph[attribute_name.as_str()];
            assert_eq!(
                got.len(),
                list.len(),
                "{at} {attribute_name}: morph targets"
            );
            for (t, (got, want)) in got.iter().zip(list).enumerate() {
                let e = Typed::from_json(want);
                assert_eq!(got.item_size, e.item_size, "{at} {attribute_name}[{t}]");
                failures.extend(compare(
                    &format!("{at} morph {attribute_name}[{t}]"),
                    &e.widened(),
                    &attribute_values(got),
                    false,
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{name}:\n  {}", failures.join("\n  "));
}

#[test]
fn coffeemat() {
    check("coffeemat.glb");
}

#[test]
fn facecap() {
    check("facecap.glb");
}

// --- the sweeps --------------------------------------------------------------

/// xorshift64, so the sweeps are the same on every run.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn bytes(&mut self, count: usize) -> Vec<u8> {
        (0..count).map(|_| self.next() as u8).collect()
    }
}

/// `data` encoded with `meshopt_encodeVertexBuffer` at `STRIDE`.
fn encode_vertices<const STRIDE: usize>(data: &[u8]) -> Vec<u8> {
    let vertices = data.as_chunks::<STRIDE>().0;
    let mut buffer = vec![0; encode_vertex_buffer_bound(vertices.len(), STRIDE)];
    let length = encode_vertex_buffer(&mut buffer, vertices, VertexEncodingVersion::V0)
        .expect("the encoder's bound");
    buffer.truncate(length);
    buffer
}

/// Decodes `encoded` with three.js' `MeshoptDecoder` and with the port, and
/// asserts the bytes are equal.
fn sweep(what: &str, encoded: &[u8], count: usize, stride: usize, mode: Mode, filter: Filter) {
    if !three_js_present() {
        return;
    }
    let file = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("meshopt-{what}.bin"));
    std::fs::write(&file, encoded).unwrap();

    let mode_name = match mode {
        Mode::Attributes => "ATTRIBUTES",
        Mode::Triangles => "TRIANGLES",
        Mode::Indices => "INDICES",
    };
    let filter_name = match filter {
        Filter::None => "NONE",
        Filter::Octahedral => "OCTAHEDRAL",
        Filter::Quaternion => "QUATERNION",
        Filter::Exponential => "EXPONENTIAL",
    };
    let (count_arg, stride_arg) = (count.to_string(), stride.to_string());
    let Some(stdout) = node(&[
        "--decode".as_ref(),
        mode_name.as_ref(),
        filter_name.as_ref(),
        count_arg.as_ref(),
        stride_arg.as_ref(),
        file.as_os_str(),
    ]) else {
        return;
    };
    let want = base64(std::str::from_utf8(&stdout).unwrap());

    let got = decode_gltf_buffer(count, stride, encoded, mode, filter)
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_eq!(got.len(), want.len(), "{what}: length");
    let differ: Vec<usize> = (0..want.len() / stride)
        .filter(|&v| got[v * stride..(v + 1) * stride] != want[v * stride..(v + 1) * stride])
        .collect();
    assert!(
        differ.is_empty(),
        "{what}: {} of {} elements differ; the first is [{}]: three.js {:?}, the port {:?}",
        differ.len(),
        want.len() / stride,
        differ[0],
        &want[differ[0] * stride..(differ[0] + 1) * stride],
        &got[differ[0] * stride..(differ[0] + 1) * stride],
    );
}

/// Every signed 8-bit `x, y` pair, at the unit `z` of 8, 7 and 4 bits, and
/// random bytes besides.
#[test]
fn octahedral_8() {
    let mut random = Random(0x0c7a_4ed7_a100_0008);
    let mut data = Vec::new();
    for z in [127u8, 63, 7] {
        for x in 0..=255u8 {
            for y in 0..=255u8 {
                data.extend_from_slice(&[x, y, z, random.next() as u8]);
            }
        }
    }
    data.extend(random.bytes(4 * 65536));
    let count = data.len() / 4;
    sweep(
        "oct8",
        &encode_vertices::<4>(&data),
        count,
        4,
        Mode::Attributes,
        Filter::Octahedral,
    );
}

/// A grid of 16-bit `x, y` at the unit `z` of 16 and 12 bits, and random
/// words besides.
#[test]
fn octahedral_16() {
    let mut random = Random(0x0c7a_4ed7_a100_0016);
    let mut data = Vec::new();
    for z in [32767i16, 2047] {
        for x in (-32768..=32767i32).step_by(97) {
            for y in (-32768..=32767i32).step_by(331) {
                for c in [x as i16, y as i16, z, random.next() as i16] {
                    data.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
    }
    data.extend(random.bytes(8 * 65536));
    let count = data.len() / 8;
    sweep(
        "oct16",
        &encode_vertices::<8>(&data),
        count,
        8,
        Mode::Attributes,
        Filter::Octahedral,
    );
}

/// Quaternions as the encoder writes them, at every component width from 4
/// to 16 bits and every dropped component, and random words besides
/// (negative scales included: the filter is defined on every input).
#[test]
fn quaternion() {
    let mut random = Random(0x0a7e_4ed7_a100_0008);
    let mut data = Vec::new();
    for bits in 4..=16u32 {
        let scale = (1i32 << (bits - 1)) - 1;
        for _ in 0..4096 {
            let mut c = || (random.next() as i32).rem_euclid(2 * scale + 1) - scale;
            let (x, y, z) = (c(), c(), c());
            let qc = (random.next() & 3) as i32;
            // `( meshopt_quantizeSnorm( 1.f, bits ) & ~3 ) | qc`
            let w = (scale & !3) | qc;
            for v in [x, y, z, w] {
                data.extend_from_slice(&(v as i16).to_le_bytes());
            }
        }
    }
    data.extend(random.bytes(8 * 65536));
    let count = data.len() / 8;
    sweep(
        "quat",
        &encode_vertices::<8>(&data),
        count,
        8,
        Mode::Attributes,
        Filter::Quaternion,
    );
}

/// Random 24-bit mantissas at every exponent but -128 (where `-inf * 0` is
/// a NaN, whose bits are the platform's).
#[test]
fn exponential() {
    let mut random = Random(0x0e4b_4ed7_a100_0012);
    let mut data = Vec::new();
    for _ in 0..3 * 65536 {
        let word = loop {
            let word = random.next() as u32;
            if word >> 24 != 0x80 {
                break word;
            }
        };
        data.extend_from_slice(&word.to_le_bytes());
    }
    let count = data.len() / 12;
    sweep(
        "exp",
        &encode_vertices::<12>(&data),
        count,
        12,
        Mode::Attributes,
        Filter::Exponential,
    );
}

/// The widest stride the vertex codec takes, unfiltered.
#[test]
fn attributes_256() {
    let mut random = Random(0x0a77_4ed7_a100_0256);
    // Mostly smooth data, the way vertices are, with some noise in it.
    let data: Vec<u8> = (0..256 * 1000)
        .map(|i| ((i / 256) as u8).wrapping_add((random.next() % 5) as u8))
        .collect();
    sweep(
        "attributes256",
        &encode_vertices::<256>(&data),
        1000,
        256,
        Mode::Attributes,
        Filter::None,
    );
}

/// A strip-like triangle list and a random one, 16- and 32-bit.
fn triangles(random: &mut Random) -> Vec<u32> {
    let mut indices = Vec::new();
    for i in 0..20_000u32 {
        indices.extend_from_slice(&[i, i + 1, i + 2]);
    }
    for _ in 0..20_000 {
        indices.extend((0..3).map(|_| (random.next() % 60_000) as u32));
    }
    indices
}

#[test]
fn triangles_mode() {
    let mut random = Random(0x0714_4ed7_a100_0003);
    let indices = triangles(&mut random);
    let mut buffer = vec![0; encode_index_buffer_bound(indices.len(), 60_002)];
    let length = encode_index_buffer(&mut buffer, &indices, IndexEncodingVersion::V1)
        .expect("the encoder's bound");
    buffer.truncate(length);
    for stride in [2, 4] {
        sweep(
            &format!("triangles{stride}"),
            &buffer,
            indices.len(),
            stride,
            Mode::Triangles,
            Filter::None,
        );
    }
}

#[test]
fn indices_mode() {
    let mut random = Random(0x01d1_4ed7_a100_0001);
    let mut indices = triangles(&mut random);
    // `INDICES` takes any count, not only whole triangles.
    indices.push(59_999);
    let mut buffer = vec![0; encode_index_sequence_bound(indices.len(), 60_002)];
    let length = encode_index_sequence(&mut buffer, &indices, IndexEncodingVersion::V1);
    buffer.truncate(length);
    for stride in [2, 4] {
        sweep(
            &format!("indices{stride}"),
            &buffer,
            indices.len(),
            stride,
            Mode::Indices,
            Filter::None,
        );
    }
}
