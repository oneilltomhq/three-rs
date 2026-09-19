//! `HdrLoader` and `HdrCubeTextureLoader` against three.js' own decoder.
//!
//! No GPU: the whole RGBE path is CPU work, so it can be graded long before a
//! pixel exists. `tests/hdr/oracle.json` is written by `tests/hdr/gen.mjs`,
//! which runs `HDRLoader.parse` from the vendor checkout under node over the
//! six pisa faces `webgpu_pmrem_cubemap` loads, and records for each face its
//! dimensions, an FNV-1a-64 over every byte of the resulting `Uint16Array`, the
//! first and last four texels and twelve fixed probes. A hash over all 262 144
//! halves is the bit-for-bit assertion the plan (§5.1) asks for, in 13 KB
//! rather than 6 MB; the probes are there so a failure says *where*.
//!
//! Nothing here is derived from a reference image: the oracle is the output of
//! three.js' parser on files in the vendor tree, and the `.hdr` files
//! themselves are read from that tree rather than copied in.
//!
//! Regenerate with `node tests/hdr/gen.mjs` after a vendor bump.

use three_rs::extras::{from_half_float, to_half_float};
use three_rs::loaders::{HdrCubeTextureLoader, HdrData, HdrLoader};
use three_rs::testing::three_js_dir;
use three_rs::TextureType;

const FACES: [&str; 6] = ["px.hdr", "nx.hdr", "py.hdr", "ny.hdr", "pz.hdr", "nz.hdr"];

fn pisa_dir() -> std::path::PathBuf {
    three_js_dir().join("examples/textures/cube/pisaHDR")
}

fn oracle() -> serde_json::Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/hdr/oracle.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("the oracle is in the tree"))
        .expect("the oracle is JSON")
}

/// FNV-1a-64 over the little-endian bytes of the halves, as `gen.mjs` computes
/// it.
fn fnv1a64(data: &[u16]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for half in data {
        for byte in half.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

fn u16s(value: &serde_json::Value) -> Vec<u16> {
    value
        .as_array()
        .expect("an array of halves")
        .iter()
        .map(|v| v.as_u64().expect("a half") as u16)
        .collect()
}

/// Every one of the six faces decodes to exactly the halves three.js'
/// `HDRLoader.parse` produces.
#[test]
fn the_six_pisa_faces_decode_bit_for_bit() {
    let oracle = oracle();
    let loader = HdrLoader::new();

    for (index, name) in FACES.iter().enumerate() {
        let expected = &oracle["faces"][index];
        assert_eq!(expected["file"], *name, "the oracle's face order");

        let bytes = std::fs::read(pisa_dir().join(name)).expect("the vendor tree has the face");
        let parsed = loader.parse(&bytes).expect("a well-formed .hdr");

        assert_eq!(parsed.width as u64, expected["width"].as_u64().unwrap());
        assert_eq!(parsed.height as u64, expected["height"].as_u64().unwrap());
        assert_eq!(parsed.gamma, expected["gamma"].as_f64().unwrap());
        assert_eq!(parsed.exposure, expected["exposure"].as_f64().unwrap());

        let HdrData::HalfFloat(data) = &parsed.data else {
            panic!("HDRLoader defaults to HalfFloatType");
        };
        assert_eq!(
            data.len() / 4,
            expected["texels"].as_u64().unwrap() as usize
        );

        // The probes first: they name the texel that moved.
        for probe in expected["probes"].as_array().unwrap() {
            let texel = probe["texel"].as_u64().unwrap() as usize;
            let rgba = u16s(&probe["rgba"]);
            assert_eq!(
                &data[texel * 4..texel * 4 + 4],
                &rgba[..],
                "{name} texel {texel}"
            );
        }
        assert_eq!(&data[..16], &u16s(&expected["first"])[..], "{name} head");
        assert_eq!(
            &data[data.len() - 16..],
            &u16s(&expected["last"])[..],
            "{name} tail"
        );

        // …then the hash, which covers every byte the probes do not.
        assert_eq!(
            fnv1a64(data),
            expected["fnv1a64"].as_str().unwrap(),
            "{name}: the decode differs somewhere outside the probes"
        );
    }
}

/// The `FloatType` branch of the same parse, as a second read of the RGBE
/// exponent scale that does not go through the half conversion at all.
#[test]
fn the_float_path_matches_too() {
    let oracle = oracle();
    let mut loader = HdrLoader::new();
    loader.set_data_type(TextureType::Float).unwrap();

    let bytes = std::fs::read(pisa_dir().join("px.hdr")).unwrap();
    let HdrData::Float(data) = loader.parse(&bytes).unwrap().data else {
        panic!("the loader was set to FloatType");
    };

    let expected: Vec<f32> = oracle["px_float_first"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    assert_eq!(&data[..16], &expected[..]);
}

/// `DataUtils.toHalfFloat` / `fromHalfFloat`, value for value, including the
/// out-of-range clamp and the truncating rounding that the RGBE decode inherits.
#[test]
fn the_half_float_conversion_matches_data_utils() {
    let oracle = oracle();

    for entry in oracle["half"].as_array().unwrap() {
        let bits = entry["bits"].as_u64().unwrap() as u32;
        let value = f32::from_bits(bits);
        let expected = entry["half"].as_u64().unwrap() as u16;
        assert_eq!(
            to_half_float(value as f64),
            expected,
            "toHalfFloat({value:e}) [{bits:#010x}]"
        );
    }

    for entry in oracle["half_back"]
        .as_array()
        .unwrap()
        .iter()
        .chain(oracle["half_back_extra"].as_array().unwrap())
    {
        let half = entry["half"].as_u64().unwrap() as u16;
        let expected = entry["bits"].as_u64().unwrap() as u32;
        assert_eq!(
            from_half_float(half).to_bits(),
            expected,
            "fromHalfFloat({half:#06x})"
        );
    }
}

/// `HDRCubeTextureLoader` puts face *i* of the url array at cube layer *i*,
/// with the scanlines in the order the decoder produced them.
///
/// Both halves of that are silent when wrong: a permuted face order rotates the
/// environment and a vertical flip mirrors it, and either still renders a
/// plausible reflection. The assertion is per face against the decoder's own
/// output, so it fails on a swap and on a flip.
#[test]
fn the_cube_loader_keeps_the_face_order_and_the_row_order() {
    let cube = HdrCubeTextureLoader::new()
        .set_path(pisa_dir())
        .load(FACES)
        .expect("the vendor tree has the six pisa faces");

    assert_eq!(cube.texture_type(), TextureType::HalfFloat);
    assert_eq!(cube.gpu_format(), wgpu::TextureFormat::Rgba16Float);
    // `HDRCubeTextureLoader` sets `generateMipmaps = false`, so the cube has
    // one mip and PMREM builds its own pyramid in the cubeUV atlas instead.
    assert_eq!(cube.mip_level_count(), 1);
    assert_eq!(cube.size(), (256, 256));

    let loader = HdrLoader::new();
    let inner = cube.borrow();
    assert!(
        !inner.flip_y,
        "CubeTexture.flipY is false and HDRCubeTextureLoader's DataTexture does \
         not re-apply HDRLoader's texData.flipY"
    );
    assert_eq!(inner.images.len(), 6);

    for (index, name) in FACES.iter().enumerate() {
        let bytes = std::fs::read(pisa_dir().join(name)).unwrap();
        let HdrData::HalfFloat(data) = loader.parse(&bytes).unwrap().data else {
            unreachable!()
        };
        let expected: Vec<u8> = data.iter().flat_map(|h| h.to_le_bytes()).collect();

        let image = &inner.images[index];
        assert_eq!((image.width, image.height), (256, 256), "{name}");
        assert_eq!(
            image.data.len(),
            256 * 256 * 8,
            "{name}: four halves a texel, eight bytes"
        );
        assert_eq!(image.data, expected, "{name} is not at layer {index}");
    }
}

/// The face at layer 0 is `px.hdr` and not, say, `nx.hdr` — the one assertion
/// above that would still pass if `FACES` itself were permuted.
#[test]
fn layer_zero_is_positive_x() {
    let oracle = oracle();
    let cube = HdrCubeTextureLoader::new()
        .set_path(pisa_dir())
        .load(FACES)
        .unwrap();

    let inner = cube.borrow();
    let first: Vec<u16> = inner.images[0].data[..32]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();

    assert_eq!(oracle["faces"][0]["file"], "px.hdr");
    assert_eq!(first, u16s(&oracle["faces"][0]["first"]));
}

/// `spot1Lux.hdr` — the equirect environment `webgpu_pmrem_test` loads —
/// decodes bit for bit, and `HdrLoader::load()` applies the four properties
/// `DataTextureLoader.load()` copies off `texData`.
///
/// The decisive one is **`flipY = true`**. It is the only property of this
/// texture whose absence renders a perfectly plausible wrong image: the
/// environment mirrored top-to-bottom, every sphere still a reasonable shiny
/// sphere. The row it actually lands on is asserted on the GPU in
/// `tests/pmrem_equirect.rs`; here the flag itself is pinned, together with
/// the single bright texel's position *before* the flip, so a failure says
/// which of the two halves moved.
#[test]
fn spot1lux_decodes_and_the_data_texture_carries_flip_y() {
    let oracle = oracle();
    let spot = &oracle["spot1lux"];
    let path = three_js_dir().join("examples/textures/equirectangular/spot1Lux.hdr");

    let loader = HdrLoader::new();
    let bytes = std::fs::read(&path).unwrap();
    let parsed = loader.parse(&bytes).unwrap();
    let HdrData::HalfFloat(data) = &parsed.data else {
        unreachable!("HalfFloatType is the default")
    };

    assert_eq!(parsed.width as u64, spot["width"].as_u64().unwrap());
    assert_eq!(parsed.height as u64, spot["height"].as_u64().unwrap());
    assert_eq!(fnv1a64(data), spot["fnv1a64"].as_str().unwrap());

    // Exactly one non-black texel, at ( 597, 213 ) of the decoded rows.
    let bright: Vec<(u32, u32, [u16; 4])> = data
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, t): &(usize, &[u16; 4])| t[0] != 0 || t[1] != 0 || t[2] != 0)
        .map(|(i, t)| {
            (
                i as u32 % parsed.width,
                i as u32 / parsed.width,
                [t[0], t[1], t[2], t[3]],
            )
        })
        .collect();
    let expected: Vec<(u32, u32, [u16; 4])> = spot["bright"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| {
            let rgba = u16s(&b["rgba"]);
            (
                b["x"].as_u64().unwrap() as u32,
                b["y"].as_u64().unwrap() as u32,
                [rgba[0], rgba[1], rgba[2], rgba[3]],
            )
        })
        .collect();
    assert_eq!(bright, expected);

    // `DataTextureLoader.load()`'s property copy.
    assert!(spot["flipY"].as_bool().unwrap(), "three's texData.flipY");
    let texture = loader.load(&path).unwrap();
    let inner = texture.borrow();
    assert!(inner.flip_y, "HDRLoader sets texData.flipY = true");
    assert!(!inner.generate_mipmaps);
    assert_eq!(inner.min_filter, three_rs::textures::MinFilter::Linear);
    assert_eq!(inner.mag_filter, three_rs::TextureFilter::Linear);
    assert_eq!(texture.format(), wgpu::TextureFormat::Rgba16Float);
    assert_eq!(texture.size(), (1024, 512));
}
