//! The WebP decode against Chromium's, byte for byte, on every WebP image in
//! the three.js examples (issue #179).
//!
//! three.js has no WebP decoder of its own: `GLTFLoader` hands the image's
//! bytes to the browser's `createImageBitmap` and the renderer uploads the
//! bitmap with `copyExternalImageToTexture`. `tools/image_reference.mjs` does
//! exactly those two calls in the headless Chrome three.js' own `npm ci`
//! downloads, and writes the texture's texels. Here the same bytes go through
//! [`TextureLoader::from_bytes`], and every texel has to be equal. There is no
//! tolerance: `image-webp` and libwebp agree exactly on lossy (VP8, with its
//! fancy chroma upsampling), lossy with alpha (`ALPH`) and lossless (VP8L).
//!
//! The inputs are found, not listed: every `.glb` under `examples/models`
//! whose `extensionsUsed` names `EXT_texture_webp`, and every image one of its
//! textures reaches through the extension. At 5f610f5 that is 64 images in
//! eight files, five of which also need Draco and so cannot go through
//! `GLTFLoader` on this branch; their images are cut out of the GLB here
//! instead. The three that need nothing else (BoomBox, dungeon_warkarma,
//! steampunk_camera) are also loaded whole, and every texture their materials
//! end up with is checked against the reference for the image
//! `EXT_texture_webp.source` names.
//!
//! Skipped with a note when there is no three.js checkout, no node or no
//! `npm ci` in the checkout — unless `CI` is set, where a skip would be a
//! silent pass and is a failure instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use three_rs::loaders::{GLTFLoader, TextureLoader};
use three_rs::textures::Texture;

/// One WebP image cut out of a glTF asset.
struct Input {
    /// `<asset>#<image index>`.
    label: String,
    asset: PathBuf,
    image: usize,
    bytes: Vec<u8>,
}

/// What the reference wrote for one input.
struct Reference {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn skip_reason() -> Option<String> {
    let three = three_rs::testing::three_js_dir();
    if !three.join("examples/models/gltf").exists() {
        return Some(format!(
            "no three.js checkout at {} (set THREE_JS_DIR)",
            three.display()
        ));
    }
    if !three.join("node_modules/puppeteer").exists() {
        return Some(format!(
            "no node_modules/puppeteer in {} (run `npm ci` there)",
            three.display()
        ));
    }
    match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => None,
        Ok(_) | Err(_) => Some("cannot run node".to_string()),
    }
}

fn skip(reason: &str) -> bool {
    if std::env::var_os("CI").is_some() {
        panic!("the WebP oracle cannot run: {reason}");
    }
    eprintln!("skipped: {reason}");
    true
}

/// The JSON and BIN chunks of a GLB.
fn glb_chunks(data: &[u8]) -> (Value, &[u8]) {
    assert_eq!(&data[0..4], b"glTF");
    let json_length = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
    let json = serde_json::from_slice(&data[20..20 + json_length]).unwrap();
    let bin = data.get(20 + json_length + 8..).unwrap_or_default();
    (json, bin)
}

/// Every `.glb` under `dir`, sorted, so the case numbering is stable.
fn glbs(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            glbs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "glb") {
            out.push(path);
        }
    }
}

fn uses_webp(json: &Value) -> bool {
    json["extensionsUsed"]
        .as_array()
        .is_some_and(|used| used.iter().any(|name| name == "EXT_texture_webp"))
}

/// The images `GLTFTextureWebPExtension.loadTexture` would decode.
fn webp_images(json: &Value) -> Vec<usize> {
    let mut images: Vec<usize> = json["textures"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|texture| texture.pointer("/extensions/EXT_texture_webp/source"))
        .map(|source| source.as_u64().unwrap() as usize)
        .collect();
    images.sort_unstable();
    images.dedup();
    images
}

fn inputs() -> Vec<Input> {
    let models = three_rs::testing::three_js_dir().join("examples/models");
    let mut files = Vec::new();
    glbs(&models, &mut files);

    let mut inputs = Vec::new();
    for asset in files {
        let data = std::fs::read(&asset).unwrap();
        let (json, bin) = glb_chunks(&data);
        if !uses_webp(&json) {
            continue;
        }

        for image in webp_images(&json) {
            let def = &json["images"][image];
            assert_eq!(def["mimeType"], "image/webp", "{}#{image}", asset.display());
            let view = &json["bufferViews"][def["bufferView"]
                .as_u64()
                .unwrap_or_else(|| panic!("{}#{image} is not in a bufferView", asset.display()))
                as usize];
            assert_eq!(view["buffer"].as_u64().unwrap_or(0), 0);
            let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
            let length = view["byteLength"].as_u64().unwrap() as usize;

            let label = asset.strip_prefix(&models).unwrap().display().to_string();
            inputs.push(Input {
                label: format!("{label}#{image}"),
                asset: asset.clone(),
                image,
                bytes: bin[offset..offset + length].to_vec(),
            });
        }
    }
    inputs
}

/// Runs the reference over every input; one Chrome for all of them.
fn references(inputs: &[Input]) -> Vec<Result<Reference, String>> {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("loaders_webp");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("in")).unwrap();

    let list: Vec<Value> = inputs
        .iter()
        .enumerate()
        .map(|(i, input)| {
            let file = dir.join("in").join(i.to_string());
            std::fs::write(&file, &input.bytes).unwrap();
            serde_json::json!({ "file": file, "mime": "image/webp" })
        })
        .collect();
    let list_file = dir.join("list.json");
    std::fs::write(&list_file, serde_json::to_string(&list).unwrap()).unwrap();

    let out = dir.join("out");
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/image_reference.mjs");
    let output = Command::new("node")
        .arg(&script)
        .arg(three_rs::testing::three_js_dir())
        .arg(&list_file)
        .arg(&out)
        .output()
        .expect("node ran a moment ago");
    assert!(
        output.status.success(),
        "the reference failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let results = (0..inputs.len())
        .map(|i| {
            if let Ok(error) = std::fs::read_to_string(out.join(format!("{i}.error"))) {
                return Err(error);
            }
            let size: Value = serde_json::from_str(
                &std::fs::read_to_string(out.join(format!("{i}.json"))).unwrap(),
            )
            .unwrap();
            Ok(Reference {
                width: size["width"].as_u64().unwrap() as u32,
                height: size["height"].as_u64().unwrap() as u32,
                rgba: std::fs::read(out.join(format!("{i}.rgba"))).unwrap(),
            })
        })
        .collect();

    let _ = std::fs::remove_dir_all(&dir);
    results
}

/// `None` when `texture` holds exactly `reference`'s texels, else what differs.
fn compare(texture: &Texture, reference: &Reference) -> Option<String> {
    let (width, height) = texture.size();
    if (width, height) != (reference.width, reference.height) {
        return Some(format!(
            "{width}x{height}, Chrome decodes {}x{}",
            reference.width, reference.height
        ));
    }

    let inner = texture.borrow();
    let data = inner.data.as_deref().unwrap_or_default();
    if data == reference.rgba.as_slice() {
        return None;
    }
    if data.len() != reference.rgba.len() {
        return Some(format!(
            "{} bytes, Chrome has {}",
            data.len(),
            reference.rgba.len()
        ));
    }

    let mut texels = 0;
    let mut largest = 0;
    let mut first = None;
    for (i, (have, want)) in data.chunks(4).zip(reference.rgba.chunks(4)).enumerate() {
        if have != want {
            texels += 1;
            first.get_or_insert((i, have.to_vec(), want.to_vec()));
            for (a, b) in have.iter().zip(want) {
                largest = largest.max(a.abs_diff(*b));
            }
        }
    }
    let (i, have, want) = first.unwrap();
    Some(format!(
        "{texels} of {} texels differ, by up to {largest}; the first is ({}, {}): {have:?}, Chrome has {want:?}",
        width * height,
        i as u32 % width,
        i as u32 / width,
    ))
}

#[test]
fn every_webp_image_in_the_examples_decodes_as_chrome_does() {
    if let Some(reason) = skip_reason() {
        if skip(&reason) {
            return;
        }
    }

    let inputs = inputs();
    // A floor, so a checkout that has lost its models is not a pass: the eight
    // assets of 5f610f5 hold 64.
    assert!(
        inputs.len() >= 64,
        "only {} WebP images found under examples/models",
        inputs.len()
    );
    let references = references(&inputs);

    // The texture each (asset, image) decoded to, for the loaded check below.
    let mut decoded: BTreeMap<(PathBuf, usize), usize> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut lossless = 0;
    let mut alpha = 0;

    for (i, (input, reference)) in inputs.iter().zip(&references).enumerate() {
        decoded.insert((input.asset.clone(), input.image), i);
        let reference = match reference {
            Ok(reference) => reference,
            Err(error) => {
                failures.push(format!(
                    "{}: Chrome does not decode it: {error}",
                    input.label
                ));
                continue;
            }
        };

        // `VP8L` at offset 12 is a lossless image; `VP8X` with the alpha flag
        // is a lossy one with an `ALPH` plane. Counted only to say in the
        // output which paths were exercised.
        if &input.bytes[12..16] == b"VP8L" {
            lossless += 1;
        }
        if &input.bytes[12..16] == b"VP8X" && input.bytes[20] & 0x10 != 0 {
            alpha += 1;
        }

        match TextureLoader::new().from_bytes(&input.bytes, Some("image/webp")) {
            Ok(texture) => {
                if let Some(difference) = compare(&texture, reference) {
                    failures.push(format!("{}: {difference}", input.label));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", input.label)),
        }
    }

    eprintln!(
        "{} WebP images, {lossless} lossless, {alpha} lossy with alpha",
        inputs.len()
    );

    // The same images again through `GLTFLoader`, for the assets it can load
    // on its own: every texture a material ends up with is the one
    // `EXT_texture_webp.source` names, decoded exactly.
    let mut loaded = 0;
    let assets: Vec<PathBuf> = {
        let mut assets: Vec<_> = inputs.iter().map(|input| input.asset.clone()).collect();
        assets.dedup();
        assets
    };
    for asset in assets {
        let data = std::fs::read(&asset).unwrap();
        let (json, _) = glb_chunks(&data);
        let required: Vec<&str> = json["extensionsRequired"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if required.iter().any(|name| *name != "EXT_texture_webp") {
            continue;
        }

        let gltf =
            GLTFLoader::load(&asset).unwrap_or_else(|error| panic!("{}: {error}", asset.display()));
        for primitive in &gltf.primitives {
            let node = primitive.node.borrow();
            let Some(material) = node.mesh().and_then(|mesh| mesh.material.as_ref()) else {
                continue;
            };
            let def = &gltf.materials[primitive.material.unwrap()];
            let slots = [
                (&material.map, &def.base_color_texture),
                (&material.normal_map, &def.normal_texture),
                (&material.emissive_map, &def.emissive_texture),
                (&material.ao_map, &def.occlusion_texture),
                (&material.metalness_map, &def.metallic_roughness_texture),
            ];
            for (texture, reference) in slots {
                let (Some(texture), Some(reference)) = (texture, reference) else {
                    continue;
                };
                // A texture without the extension (steampunk_camera has JPEG
                // and PNG ones beside its WebP ones) is not this test's.
                if json
                    .pointer(&format!(
                        "/textures/{}/extensions/EXT_texture_webp",
                        reference.index
                    ))
                    .is_none()
                {
                    continue;
                }
                let image = gltf.textures[reference.index].source.unwrap();
                let Some(&case) = decoded.get(&(asset.clone(), image)) else {
                    failures.push(format!(
                        "{}: texture {} samples image {image}, which is not the WebP one",
                        asset.display(),
                        reference.index
                    ));
                    continue;
                };
                if let Ok(want) = &references[case] {
                    if let Some(difference) = compare(texture, want) {
                        failures.push(format!("{} (loaded): {difference}", inputs[case].label));
                    }
                }
                loaded += 1;
            }
        }
    }
    eprintln!("{loaded} material texture slots checked through GLTFLoader");
    assert!(loaded > 0, "no asset loaded through GLTFLoader");

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
