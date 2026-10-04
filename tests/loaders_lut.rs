//! `LutCubeLoader`, `Lut3dlLoader` and `LutImageLoader` against three.js' own
//! loaders.
//!
//! No GPU: all three are CPU work. `tests/lut/oracle.json` is written by
//! `tests/lut/gen.mjs`, which runs `LUTCubeLoader.parse` and
//! `LUT3dlLoader.parse` under node over the vendor `examples/luts/` tables
//! (both `UnsignedByteType` and `FloatType`) and over a few synthetic inputs
//! that exercise upstream's quirks, and runs `LUTImageLoader` in Chrome over
//! the three PNG strips with `flip` off and on. Each table is recorded as an
//! FNV-1a-64 over its bytes (a `Float32Array` over its little-endian bytes),
//! plus the first and last texels so a failure says where; the synthetic
//! cases record every value, floats as bit patterns so NaN survives JSON.
//!
//! The LUT files are read from the vendor tree rather than copied in.
//!
//! Regenerate with `node tests/lut/gen.mjs` after a vendor bump.

use serde_json::Value;
use three_rs::loaders::{Lut3dlLoader, LutCubeLoader, LutImageLoader};
use three_rs::testing::three_js_dir;
use three_rs::textures::Data3DTexture;
use three_rs::TextureType;

fn oracle() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/lut/oracle.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("the oracle is in the tree"))
        .expect("the oracle is JSON")
}

fn luts_dir() -> std::path::PathBuf {
    three_js_dir().join("examples/luts")
}

/// FNV-1a-64 over bytes, as `gen.mjs` computes it.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hex(value: &Value) -> u64 {
    u64::from_str_radix(value.as_str().expect("a hex string"), 16).expect("hex")
}

fn data(texture: &Data3DTexture) -> Vec<u8> {
    texture.borrow().data.clone().expect("a CPU table")
}

fn bytes(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|v| v.as_u64().expect("a byte") as u8)
        .collect()
}

fn f32_bits(texture: &Data3DTexture) -> Vec<u32> {
    data(texture)
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect()
}

fn words(value: &Value) -> Vec<u32> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|v| v.as_u64().expect("a u32") as u32)
        .collect()
}

fn float_loader_cube() -> LutCubeLoader {
    let mut loader = LutCubeLoader::new();
    loader
        .set_type(TextureType::Float)
        .expect("FloatType is legal");
    loader
}

fn float_loader_3dl() -> Lut3dlLoader {
    let mut loader = Lut3dlLoader::new();
    loader
        .set_type(TextureType::Float)
        .expect("FloatType is legal");
    loader
}

#[test]
fn cube_files_match_three() {
    let oracle = oracle();
    for entry in oracle["cube"].as_array().expect("cube entries") {
        let file = entry["file"].as_str().expect("file");
        let path = luts_dir().join(file);
        let lut = LutCubeLoader::new().load(&path).expect("the table parses");
        assert_eq!(
            lut.title.as_deref(),
            entry["title"].as_str(),
            "{file}: title"
        );
        assert_eq!(
            u64::from(lut.size),
            entry["size"].as_u64().unwrap(),
            "{file}: size"
        );
        let min = [lut.domain_min.x, lut.domain_min.y, lut.domain_min.z];
        let max = [lut.domain_max.x, lut.domain_max.y, lut.domain_max.z];
        let expect = |key: &str| -> Vec<f64> {
            entry[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        assert_eq!(min.to_vec(), expect("domainMin"), "{file}: domainMin");
        assert_eq!(max.to_vec(), expect("domainMax"), "{file}: domainMax");
        assert_eq!(lut.texture_3d.format(), wgpu::TextureFormat::Rgba8Unorm);

        let u8s = data(&lut.texture_3d);
        assert_eq!(
            &u8s[..16],
            &bytes(&entry["first"])[..],
            "{file}: first texels"
        );
        assert_eq!(
            &u8s[u8s.len() - 16..],
            &bytes(&entry["last"])[..],
            "{file}: last texels"
        );
        assert_eq!(
            fnv1a64(&u8s),
            hex(&entry["u8"]),
            "{file}: UnsignedByteType table"
        );

        let float = float_loader_cube().load(&path).expect("the table parses");
        assert_eq!(float.texture_3d.format(), wgpu::TextureFormat::Rgba32Float);
        assert_eq!(
            &f32_bits(&float.texture_3d)[..8],
            &words(&entry["f32First"])[..]
        );
        assert_eq!(
            fnv1a64(&data(&float.texture_3d)),
            hex(&entry["f32"]),
            "{file}: FloatType table"
        );
    }
}

#[test]
fn three_dl_files_match_three() {
    let oracle = oracle();
    for entry in oracle["threeDl"].as_array().expect("3dl entries") {
        let file = entry["file"].as_str().expect("file");
        let path = luts_dir().join(file);
        let lut = Lut3dlLoader::new().load(&path).expect("the table parses");
        assert_eq!(
            u64::from(lut.size),
            entry["size"].as_u64().unwrap(),
            "{file}: size"
        );
        let u8s = data(&lut.texture_3d);
        assert_eq!(
            &u8s[..16],
            &bytes(&entry["first"])[..],
            "{file}: first texels"
        );
        assert_eq!(
            &u8s[u8s.len() - 16..],
            &bytes(&entry["last"])[..],
            "{file}: last texels"
        );
        assert_eq!(
            fnv1a64(&u8s),
            hex(&entry["u8"]),
            "{file}: UnsignedByteType table"
        );

        let float = float_loader_3dl().load(&path).expect("the table parses");
        assert_eq!(
            &f32_bits(&float.texture_3d)[..8],
            &words(&entry["f32First"])[..]
        );
        assert_eq!(
            fnv1a64(&data(&float.texture_3d)),
            hex(&entry["f32"]),
            "{file}: FloatType table"
        );
    }
}

/// `{ "error": "..." }` in the oracle: the Rust error must carry the same
/// message, prefix and all.
fn assert_error<T: std::fmt::Debug>(
    label: &str,
    result: Result<T, three_rs::Error>,
    expected: &Value,
) {
    let message = expected.as_str().expect("an error message");
    match result {
        Err(error) => assert_eq!(error.to_string(), message, "{label}"),
        Ok(value) => panic!("{label}: three throws {message:?}, the port returned {value:?}"),
    }
}

#[test]
fn cube_quirks_match_three() {
    let oracle = oracle();
    for (name, case) in oracle["cubeSynthetic"].as_object().expect("cases") {
        let input = case["input"].as_str().expect("input");
        let label = format!("cube {name}");
        if let Some(error) = case["u8"].get("error") {
            assert_error(&label, LutCubeLoader::new().parse(input), error);
            assert_error(
                &label,
                float_loader_cube().parse(input),
                &case["f32"]["error"],
            );
            continue;
        }
        let lut = LutCubeLoader::new().parse(input).expect("parses");
        let expected = &case["u8"];
        assert_eq!(
            lut.title.as_deref(),
            expected["title"].as_str(),
            "{label}: title"
        );
        assert_eq!(
            u64::from(lut.size),
            expected["size"].as_u64().unwrap(),
            "{label}: size"
        );
        assert_eq!(
            [lut.domain_max.x, lut.domain_max.y, lut.domain_max.z].to_vec(),
            expected["domainMax"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect::<Vec<_>>(),
            "{label}: domainMax"
        );
        assert_eq!(
            data(&lut.texture_3d),
            bytes(&expected["data"]),
            "{label}: data"
        );
        let float = float_loader_cube().parse(input).expect("parses");
        assert_eq!(
            f32_bits(&float.texture_3d),
            words(&case["f32"]["bits"]),
            "{label}: float data"
        );
    }
}

#[test]
fn three_dl_quirks_match_three() {
    let oracle = oracle();
    for (name, case) in oracle["threeDlSynthetic"].as_object().expect("cases") {
        let input = case["input"].as_str().expect("input");
        let label = format!("3dl {name}");
        if let Some(error) = case["u8"].get("error") {
            assert_error(&label, Lut3dlLoader::new().parse(input), error);
            assert_error(
                &label,
                float_loader_3dl().parse(input),
                &case["f32"]["error"],
            );
            continue;
        }
        let lut = Lut3dlLoader::new().parse(input).expect("parses");
        assert_eq!(
            u64::from(lut.size),
            case["u8"]["size"].as_u64().unwrap(),
            "{label}: size"
        );
        assert_eq!(
            data(&lut.texture_3d),
            bytes(&case["u8"]["data"]),
            "{label}: data"
        );
        let float = float_loader_3dl().parse(input).expect("parses");
        assert_eq!(
            f32_bits(&float.texture_3d),
            words(&case["f32"]["bits"]),
            "{label}: float data"
        );
    }
}

#[test]
fn image_strips_match_three_in_chrome() {
    let oracle = oracle();
    for entry in oracle["image"].as_array().expect("image entries") {
        let file = entry["file"].as_str().expect("file");
        let flip = entry["flip"].as_bool().expect("flip");
        let label = format!("{file} flip={flip}");
        let loader = LutImageLoader { flip };
        let lut = loader.load(luts_dir().join(file)).expect("the strip loads");
        assert_eq!(
            u64::from(lut.size),
            entry["size"].as_u64().unwrap(),
            "{label}: size"
        );
        let texels = data(&lut.texture_3d);
        assert_eq!(
            texels.len() as u64,
            entry["bytes"].as_u64().unwrap(),
            "{label}: length"
        );
        for probe in entry["probes"].as_array().unwrap() {
            let t = probe["texel"].as_u64().unwrap() as usize;
            assert_eq!(
                &texels[t * 4..t * 4 + 4],
                &bytes(&probe["rgba"])[..],
                "{label}: texel {t}"
            );
        }
        assert_eq!(fnv1a64(&texels), hex(&entry["u8"]), "{label}: table");
    }
}

#[test]
fn set_type_refuses_what_three_does_not_document() {
    assert!(LutCubeLoader::new()
        .set_type(TextureType::HalfFloat)
        .is_err());
    assert!(Lut3dlLoader::new()
        .set_type(TextureType::UnsignedInt)
        .is_err());
}
