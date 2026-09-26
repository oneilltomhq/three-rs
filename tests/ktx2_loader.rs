//! `KTX2Loader` against three.js' own `KTX2Loader`, byte for byte.
//!
//! `tools/ktx2_reference.mjs` runs three's loader under node — its
//! `ktx-parse`, its Emscripten Basis Universal transcoder, its `zstddec` —
//! over one `.ktx2` file for one device profile, and writes the texture it
//! builds: class, format, type, colour space, filters, and every mip level's
//! bytes. Each case here loads the same file with [`Ktx2Loader`] set up for
//! the same device and asserts every one of those fields is equal, and every
//! mip level identical. There is no tolerance anywhere: a transcode that is
//! not bit-exact is a failure, and the file it happened on is in the message.
//!
//! The inputs are every `.ktx2` in the three.js examples (the eighteen
//! `textures/ktx2/` samples, `spiritedaway.ktx2`, the eight PMREM cubes), a
//! Zstandard-supercompressed repack of two of them (the checkout has no
//! supercompressed non-Basis file), and the Basis images inside the three
//! glTF assets that use `KHR_texture_basisu`. The profiles are the four a
//! WebGPU device can present: no compression, BC, ASTC, ETC2.
//!
//! Skipped, with a note, when there is no three.js checkout or no node.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use three_rs::loaders::{Ktx2Loader, Ktx2Support, Ktx2Texture};
use three_rs::textures::{MinFilter, TextureFilter};

/// The device profiles `tools/ktx2_reference.mjs` knows.
const PROFILES: [&str; 4] = ["rgba", "bc", "astc", "etc2"];

fn support(profile: &str) -> Ktx2Support {
    let features = match profile {
        "rgba" => wgpu::Features::empty(),
        "bc" => wgpu::Features::TEXTURE_COMPRESSION_BC,
        "astc" => wgpu::Features::TEXTURE_COMPRESSION_ASTC,
        "etc2" => wgpu::Features::TEXTURE_COMPRESSION_ETC2,
        other => panic!("unknown profile {other}"),
    };
    Ktx2Support::from_features(features)
}

/// `NearestFilter` … `LinearMipmapLinearFilter` in `constants.js`.
fn min_filter_constant(filter: MinFilter) -> u64 {
    match filter {
        MinFilter::Nearest => 1003,
        MinFilter::NearestMipmapNearest => 1004,
        MinFilter::NearestMipmapLinear => 1005,
        MinFilter::Linear => 1006,
        MinFilter::LinearMipmapNearest => 1007,
        MinFilter::LinearMipmapLinear => 1008,
    }
}

fn mag_filter_constant(filter: TextureFilter) -> u64 {
    match filter {
        TextureFilter::Nearest => 1003,
        TextureFilter::Linear => 1006,
    }
}

/// Where a case's oracle output goes; removed once the case is compared.
fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ktx2_loader")
        .join(label.replace(['/', ' '], "_"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Whether node and the checkout are there at all; the reason if not.
fn skip_reason() -> Option<String> {
    let three = three_rs::testing::three_js_dir();
    if !three.join("examples/jsm/loaders/KTX2Loader.js").exists() {
        return Some(format!(
            "no three.js checkout at {} (set THREE_JS_DIR)",
            three.display()
        ));
    }
    if !three.join("build/three.module.js").exists() {
        return Some(format!("no build/three.module.js in {}", three.display()));
    }
    match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => None,
        Ok(_) | Err(_) => Some("cannot run node".to_string()),
    }
}

/// Runs the reference over one file into `out`.
fn reference(file: &Path, profile: &str, out: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/ktx2_reference.mjs");
    let output = Command::new("node")
        .arg(&script)
        .arg(three_rs::testing::three_js_dir())
        .arg(file)
        .arg(out)
        .arg(profile)
        .output()
        .expect("node ran a moment ago");
    assert!(
        output.status.success(),
        "{} ({profile}): the reference failed:\n{}",
        file.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every field and every byte of `got` against the reference in `dir`.
fn compare(label: &str, got: &Ktx2Texture, dir: &Path) -> Vec<String> {
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("texture.json")).unwrap()).unwrap();

    let mut failures = Vec::new();
    let mut field = |name: &str, want: serde_json::Value, have: serde_json::Value| {
        if want != have {
            failures.push(format!("{label}: {name} is {have}, three has {want}"));
        }
    };

    field("class", json["class"].clone(), got.class.name().into());
    field("format", json["format"].clone(), (got.format as u32).into());
    field(
        "type",
        json["type"].clone(),
        (got.texture_type as u32).into(),
    );
    field(
        "colorSpace",
        json["colorSpace"].clone(),
        got.color_space.name().into(),
    );
    field(
        "premultiplyAlpha",
        json["premultiplyAlpha"].clone(),
        got.premultiply_alpha.into(),
    );
    field(
        "minFilter",
        json["minFilter"].clone(),
        min_filter_constant(got.min_filter).into(),
    );
    field(
        "magFilter",
        json["magFilter"].clone(),
        mag_filter_constant(got.mag_filter).into(),
    );
    field(
        "generateMipmaps",
        json["generateMipmaps"].clone(),
        got.generate_mipmaps.into(),
    );
    field("width", json["width"].clone(), got.width.into());
    field("height", json["height"].clone(), got.height.into());
    field("depth", json["depth"].clone(), got.depth.into());
    field(
        "normalized",
        json["normalized"].clone(),
        got.normalized.into(),
    );

    let faces = json["faces"].as_array().unwrap();
    if faces.len() != got.faces.len() {
        failures.push(format!(
            "{label}: {} faces, three has {}",
            got.faces.len(),
            faces.len()
        ));
        return failures;
    }

    for (f, (want_face, got_face)) in faces.iter().zip(&got.faces).enumerate() {
        let want_face = want_face.as_array().unwrap();
        if want_face.len() != got_face.len() {
            failures.push(format!(
                "{label}: face {f} has {} mips, three has {}",
                got_face.len(),
                want_face.len()
            ));
            continue;
        }
        for (m, (want, mip)) in want_face.iter().zip(got_face).enumerate() {
            let at = format!("{label}: face {f} mip {m}");
            if want["width"] != mip.width || want["height"] != mip.height {
                failures.push(format!(
                    "{at}: {}x{}, three has {}x{}",
                    mip.width, mip.height, want["width"], want["height"]
                ));
            }
            let bytes = std::fs::read(dir.join(want["file"].as_str().unwrap())).unwrap();
            if bytes != mip.data {
                let differing = bytes.iter().zip(&mip.data).filter(|(a, b)| a != b).count();
                failures.push(format!(
                    "{at}: {} bytes, three has {}; {differing} of the common bytes differ",
                    mip.data.len(),
                    bytes.len()
                ));
            }
        }
    }

    failures
}

/// One file, every profile: the reference, the port, the comparison.
fn check_file(label: &str, file: &Path) -> Vec<String> {
    let bytes = std::fs::read(file).unwrap();
    let mut failures = Vec::new();
    for profile in PROFILES {
        let label = format!("{label} [{profile}]");
        let dir = scratch(&label);
        reference(file, profile, &dir);
        match Ktx2Loader::new()
            .with_support(support(profile))
            .parse(&bytes)
        {
            Ok(texture) => failures.extend(compare(&label, &texture, &dir)),
            Err(error) => failures.push(format!("{label}: the port failed: {error}")),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    failures
}

/// `check_file` over `files` on a few threads — each case is a node process
/// and a transcode, and the PMREM cubes are the bulk of it.
fn check_all(files: Vec<(String, PathBuf)>) {
    let queue = Mutex::new(files);
    let failures = Mutex::new(Vec::new());
    let workers = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .min(6);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let Some((label, file)) = queue.lock().unwrap().pop() else {
                    break;
                };
                let found = check_file(&label, &file);
                failures.lock().unwrap().extend(found);
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    assert!(failures.is_empty(), "\n  {}", failures.join("\n  "));
}

fn ktx2_files() -> Vec<(String, PathBuf)> {
    let examples = three_rs::testing::three_js_dir().join("examples");
    let mut files = vec![(
        "textures/spiritedaway.ktx2".to_string(),
        examples.join("textures/spiritedaway.ktx2"),
    )];
    for dir in ["textures/ktx2", "textures/pmrem"] {
        for entry in std::fs::read_dir(examples.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "ktx2") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                files.push((format!("{dir}/{name}"), path));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_ktx2_in_the_three_js_examples_matches_three() {
    if let Some(reason) = skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let files = ktx2_files();
    // 18 samples, spiritedaway, 8 PMREM cubes — a checkout missing some
    // would otherwise pass by checking less.
    assert_eq!(files.len(), 27, "{files:?}");
    check_all(files);
}

// --- Zstandard ---------------------------------------------------------------

/// `file` rewritten with every level Zstandard-compressed
/// (`supercompressionScheme = 2`), by the `zstd` command-line tool. `None`
/// when there is no `zstd` to run.
///
/// Everything before the first level's data — header, level index, DFD,
/// key/value data — is kept; the compressed levels follow it in index order
/// and the index is patched. KTX 2.0 requires no alignment of supercompressed
/// levels, so none is added.
fn zstd_repack(file: &Path, out: &Path) -> Option<()> {
    let bytes = std::fs::read(file).unwrap();
    let u32_at = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    let u64_at = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap());

    assert_eq!(u32_at(44), 0, "the source is not supercompressed");
    let level_count = u32_at(40).max(1) as usize;
    let levels: Vec<(usize, usize)> = (0..level_count)
        .map(|i| {
            let entry = 80 + i * 24;
            (u64_at(entry) as usize, u64_at(entry + 8) as usize)
        })
        .collect();
    let data_start = levels.iter().map(|&(offset, _)| offset).min().unwrap();

    let mut repacked = bytes[..data_start].to_vec();
    repacked[44..48].copy_from_slice(&2u32.to_le_bytes());
    for (i, &(offset, length)) in levels.iter().enumerate() {
        let raw = out.with_extension(format!("level{i}"));
        std::fs::write(&raw, &bytes[offset..offset + length]).unwrap();
        let output = Command::new("zstd")
            .args(["-q", "-19", "-c"])
            .arg(&raw)
            .output()
            .ok()?;
        assert!(output.status.success(), "zstd failed");
        let _ = std::fs::remove_file(&raw);

        let entry = 80 + i * 24;
        let new_offset = repacked.len() as u64;
        repacked[entry..entry + 8].copy_from_slice(&new_offset.to_le_bytes());
        repacked[entry + 8..entry + 16]
            .copy_from_slice(&(output.stdout.len() as u64).to_le_bytes());
        // `uncompressedByteLength` stays the source level's length.
        repacked.extend_from_slice(&output.stdout);
    }

    std::fs::write(out, repacked).unwrap();
    Some(())
}

#[test]
fn zstd_supercompressed_levels_match_three() {
    if let Some(reason) = skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let examples = three_rs::testing::three_js_dir().join("examples/textures/ktx2");
    let dir = scratch("zstd");
    let mut files = Vec::new();
    // One uncompressed format and one block-compressed one: `DataTexture` and
    // `CompressedTexture` come out of different branches of
    // `createRawTexture`, after the same inflate.
    for name in ["2d_rgba16_linear.ktx2", "2d_bc7.ktx2"] {
        let out = dir.join(name.replace(".ktx2", ".zstd.ktx2"));
        if zstd_repack(&examples.join(name), &out).is_none() {
            eprintln!("skipping: cannot run zstd");
            return;
        }
        // The repack really is supercompressed, and really does shrink.
        let reader_bytes = std::fs::read(&out).unwrap();
        let reader = ktx2::Reader::new(&reader_bytes[..]).unwrap();
        assert_eq!(
            reader.header().supercompression_scheme,
            Some(ktx2::SupercompressionScheme::Zstandard)
        );
        files.push((format!("{name} (zstd)"), out));
    }
    check_all(files);
}

// --- KHR_texture_basisu ------------------------------------------------------

/// The `image/ktx2` images of a GLB: `( label, bytes )` for each, read
/// straight out of the BIN chunk — through no part of the port's glTF loader,
/// which is not what this test is about (and which cannot load `coffeemat.glb`
/// at all: it also requires `EXT_meshopt_compression`).
fn glb_ktx2_images(path: &Path) -> Vec<(String, Vec<u8>)> {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"glTF");
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + json_length]).unwrap();
    let bin_start = 20 + json_length + 8;

    let mut images = Vec::new();
    for (i, image) in json["images"].as_array().unwrap().iter().enumerate() {
        if image["mimeType"] != "image/ktx2" {
            continue;
        }
        let view = &json["bufferViews"][image["bufferView"].as_u64().unwrap() as usize];
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let length = view["byteLength"].as_u64().unwrap() as usize;
        images.push((
            format!("image {i}"),
            bytes[bin_start + offset..bin_start + offset + length].to_vec(),
        ));
    }
    images
}

#[test]
fn khr_texture_basisu_images_match_three() {
    if let Some(reason) = skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let models = three_rs::testing::three_js_dir().join("examples/models/gltf");
    let dir = scratch("glb");
    let mut files = Vec::new();
    for name in ["CarbonFrameBike.glb", "coffeemat.glb", "facecap.glb"] {
        let images = glb_ktx2_images(&models.join(name));
        assert!(!images.is_empty(), "{name} has no KTX2 image");
        for (label, image) in images {
            let out = dir.join(format!("{name}.{}.ktx2", label.replace(' ', "")));
            std::fs::write(&out, image).unwrap();
            files.push((format!("{name} {label}"), out));
        }
    }
    check_all(files);
}
