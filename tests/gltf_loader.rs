//! `GLTFLoader` against the assets `webgpu_skinning` loads (Michelle.glb) and
//! the Soldier.glb that the skinning examples share.
//!
//! Expected numbers come from three.js' own `GLTFLoader` in the vendor tree,
//! run under node (see `handoff`/docs/gltf-progress.md for the script).

use three_rs::loaders::GLTFLoader;

/// Three's own sample models, from the `THREE_JS_DIR` checkout.
fn models() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples/models/gltf")
}

fn names(gltf: &three_rs::loaders::Gltf) -> Vec<String> {
    let mut out = Vec::new();
    gltf.scene
        .traverse(&mut |node| out.push(node.borrow().name.clone()));
    out
}

#[test]
fn michelle_tree() {
    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();
    let names = names(&gltf);

    assert_eq!(names.len(), 68);
    assert_eq!(names[0], "Scene");
    assert_eq!(names[1], "Character");
    assert_eq!(names[2], "Ch03");
    assert_eq!(names[3], "mixamorigHips");
    assert_eq!(names[67], "mixamorigRightToe_End");

    assert_eq!(gltf.skins.len(), 1);
    assert_eq!(gltf.skins[0].borrow().bones.len(), 65);
    assert_eq!(gltf.skinned_meshes.len(), 1);
}

#[test]
fn michelle_geometry() {
    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();
    let node = gltf.skinned_meshes[0].borrow();
    let geometry = node.skinned_mesh().unwrap().geometry();

    assert_eq!(geometry.position().unwrap().count(), 16340);
    assert_eq!(geometry.index.as_ref().unwrap().count(), 84318);

    let expected = [
        0.057_498_686_015_605_927,
        1.457_242_131_233_215_3,
        0.066_856_533_288_955_69,
        0.045_773_558_318_614_96,
        1.458_376_407_623_291,
        0.072_215_639_054_774_94,
        0.045_767_586_678_266_525,
        1.456_706_166_267_395,
        0.070_081_681_013_107_3,
    ];
    for (i, expected) in expected.iter().enumerate() {
        let got = geometry.position().unwrap().array()[i] as f64;
        assert!((got - expected).abs() < 1e-6, "position[{i}]: {got}");
    }

    // `skinIndex` is unnormalized `Uint8`/`Uint16`, so it must come out exact
    let skin_index = geometry.get_attribute("skinIndex").unwrap();
    assert_eq!(
        skin_index.array()[0..8],
        [5.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0]
    );
}

#[test]
fn michelle_bone_inverses() {
    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();
    let skeleton = gltf.skins[0].borrow();

    let expected = [
        100.0,
        0.0,
        0.0,
        0.0,
        0.0,
        100.0,
        -0.000_016_292_065_993_184_224,
        0.0,
        0.0,
        0.000_016_292_065_993_184_224,
        100.0,
        0.0,
        0.0,
        -102.625_259_399_414_06,
        0.521_240_949_630_737_3,
        1.0,
    ];
    for (i, expected) in expected.iter().enumerate() {
        let got = skeleton.bone_inverses[0].elements[i];
        assert!((got - expected).abs() < 1e-6, "boneInverses[0][{i}]: {got}");
    }
}

#[test]
fn michelle_animations() {
    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();

    assert_eq!(gltf.animations.len(), 2);
    assert_eq!(gltf.animations[0].name, "SambaDance");
    assert_eq!(gltf.animations[1].name, "TPose");
    assert!((gltf.animations[0].duration - 18.233_333_587_646_484).abs() < 1e-6);
    assert!((gltf.animations[1].duration - 0.066_666_670_143_603_28).abs() < 1e-6);
    assert_eq!(gltf.animations[0].tracks.len(), 195);
    assert_eq!(gltf.animations[1].tracks.len(), 195);

    let track = &gltf.animations[0].tracks[0];
    assert_eq!(track.name, "mixamorigHips.position");
    let expected = [
        -0.117_928_624_153_137_21,
        -0.120_026_528_835_296_63,
        -98.860_733_032_226_56,
        -2.125_683_784_484_863_3,
    ];
    for (i, expected) in expected.iter().enumerate() {
        assert!((track.values[i] - expected).abs() < 1e-6, "values[{i}]");
    }
}

#[test]
fn soldier_tree() {
    let gltf = GLTFLoader::load(models().join("Soldier.glb")).unwrap();
    let names = names(&gltf);

    assert_eq!(names.len(), 69);
    assert_eq!(names[2], "vanguard_Mesh");
    assert_eq!(names[3], "vanguard_visor");

    assert_eq!(gltf.skins.len(), 2);
    assert_eq!(gltf.skins[0].borrow().bones.len(), 49);
    assert_eq!(gltf.skins[1].borrow().bones.len(), 2);
    assert_eq!(gltf.skinned_meshes.len(), 2);

    let clips: Vec<(&str, usize)> = gltf
        .animations
        .iter()
        .map(|clip| (clip.name.as_str(), clip.tracks.len()))
        .collect();
    assert_eq!(
        clips,
        [("Idle", 156), ("Run", 156), ("TPose", 156), ("Walk", 156)]
    );
    assert!((gltf.animations[0].duration - 1.966_666_698_455_810_5).abs() < 1e-6);

    let node = gltf.skinned_meshes[0].borrow();
    let geometry = node.skinned_mesh().unwrap().geometry();
    assert_eq!(geometry.position().unwrap().count(), 7325);
    assert_eq!(geometry.index.as_ref().unwrap().count(), 33558);
}

/// `mixer.clipAction( clip, root ).play(); mixer.update( 0 )` writes into the
/// tree: the expected bone world matrices are three's, at t = 0 of
/// `SambaDance` (the clip `webgpu_skinning` plays).
#[test]
fn michelle_mixer_at_zero() {
    use three_rs::animation::AnimationMixer;

    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);
    mixer.update(0.0);

    gltf.scene.update_matrix_world(true);

    let expected: [(&str, [f64; 16]); 2] = [
        (
            "mixamorigHips",
            [
                0.008_227_340_933_105_275,
                -0.000_188_572_412_805_589_21,
                -0.005_681_135_218_087_022,
                0.0,
                -0.000_241_645_235_677_088_98,
                0.009_973_857_810_013_35,
                -0.000_681_007_571_672_808_8,
                0.0,
                0.005_679_125_523_620_743,
                0.000_697_570_208_985_178_2,
                0.008_201_276_404_566_728,
                0.0,
                -0.001_179_286_215_172_270_5,
                0.988_607_274_750_171_1,
                -0.001_199_965_725_546_089_7,
                1.0,
            ],
        ),
        (
            "mixamorigLeftHand",
            [
                0.007_003_632_312_766_139,
                -0.005_556_839_674_743_623,
                -0.004_480_017_859_296_49,
                0.0,
                0.003_197_087_286_270_483,
                -0.003_169_483_333_125_039_6,
                0.008_929_329_623_035_23,
                0.0,
                -0.006_381_821_969_988_196,
                -0.007_686_080_423_159_218,
                -0.000_443_217_103_400_579_4,
                0.0,
                0.169_366_461_316_386_25,
                0.882_823_517_387_601_1,
                -0.060_322_049_899_743_96,
                1.0,
            ],
        ),
    ];

    for (name, expected) in expected {
        let bone = gltf.scene.get_object_by_name(name).unwrap();
        let got = bone.borrow().matrix_world.elements;

        for (i, expected) in expected.iter().enumerate() {
            assert!(
                (got[i] - expected).abs() < 1e-6,
                "{name}.matrixWorld[{i}]: {} != {expected}",
                got[i]
            );
        }
    }
}

/// What the skinning shader consumes: `Skeleton.update()`'s flattened
/// `boneMatrices`, and `SkinnedMesh.applyBoneTransform` / `computeBoundingBox`,
/// all at t = 0 of `SambaDance`, against three's.
#[test]
fn michelle_skinning_at_zero() {
    use three_rs::animation::AnimationMixer;
    use three_rs::math::Vector3;

    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);
    mixer.update(0.0);
    gltf.scene.update_matrix_world(true);

    // `SkinnedMesh.updateMatrixWorld` is where `bindMatrixInverse` comes from
    // in `AttachedBindMode`; the renderer calls it every frame.
    gltf.skinned_meshes[0].update_matrix_world(true);
    gltf.skins[0].borrow_mut().update();

    let expected: [f32; 32] = [
        0.822_734_1,
        -0.018_857_24,
        -0.568_113_5,
        0.0,
        -0.024_164_615,
        0.997_385_74,
        -0.068_100_89,
        0.0,
        0.567_912_5,
        0.069_757_186,
        0.820_127_6,
        0.0,
        0.026_579_812,
        -0.034_598_87,
        0.072_963_454,
        1.0,
        0.681_407_33,
        0.066_141_52,
        -0.728_909_7,
        0.0,
        0.012_672_049,
        0.994_692_86,
        0.102_104_954,
        0.0,
        0.731_794_66,
        -0.078_811_824,
        0.676_952_96,
        0.0,
        -0.012_397_086,
        -0.032_505_188,
        -0.112_112_06,
        1.0,
    ];

    let skeleton = gltf.skins[0].borrow();
    for (i, expected) in expected.iter().enumerate() {
        let got = skeleton.bone_matrices[i];
        assert!(
            (got - expected).abs() < 1e-6,
            "boneMatrices[{i}]: {got} != {expected}"
        );
    }
    drop(skeleton);

    // `applyBoneTransform( 0, v.fromBufferAttribute( position, 0 ) )`
    let node = gltf.skinned_meshes[0].borrow();
    let mesh = node.skinned_mesh().unwrap();
    let position = mesh.geometry().position().unwrap();
    let mut vertex = Vector3::new(position.get_x(0), position.get_y(0), position.get_z(0));
    mesh.apply_bone_transform(0, &mut vertex);

    let expected = [
        8.248_729_752_697_153,
        2.609_101_503_364_952_7,
        -143.658_634_049_021_8,
    ];
    for (i, expected) in expected.iter().enumerate() {
        let got = vertex.get_component(i);
        assert!(
            (got - expected).abs() < 1e-5,
            "applyBoneTransform[{i}]: {got}"
        );
    }
}

/// `GLTFParser.loadMaterial` for `Ch03_Body`: `KHR_materials_specular` and
/// `KHR_materials_ior` make it a `MeshPhysicalMaterial`, the four PNGs in the
/// BIN chunk decode to 512² RGBA, the ORM map reaches `metalnessMap` and
/// `roughnessMap` as *one* texture, and `normalScale.y` is negative because the
/// geometry has no `tangent` attribute (`useDerivativeTangents`).
#[test]
fn michelle_material() {
    use three_rs::materials::{MaterialKind, Side};
    use three_rs::textures::{ColorSpace, Wrapping};

    let gltf = GLTFLoader::load(models().join("Michelle.glb")).unwrap();

    let node = gltf.skinned_meshes[0].borrow();
    let mesh = node.skinned_mesh().unwrap();
    let material = mesh
        .mesh
        .material
        .as_ref()
        .expect("the body has a material");

    assert_eq!(material.kind, MaterialKind::Physical);
    assert_eq!(material.side, Side::Double);
    assert_eq!(material.metalness, 0.5);
    assert_eq!(material.roughness, 1.0);
    assert!((material.ior - 1.450_000_047_683_715_8).abs() < 1e-12);
    assert_eq!(material.specular_intensity, 1.0);
    assert_eq!(material.normal_scale.x, 1.0);
    assert_eq!(material.normal_scale.y, -1.0);

    let map = material.map.as_ref().expect("baseColorTexture");
    assert_eq!(map.size(), (512, 512));
    assert_eq!(map.data_len(), 512 * 512 * 4);
    assert_eq!(map.color_space(), ColorSpace::SRGB);
    assert!(!map.borrow().flip_y);
    assert_eq!(map.borrow().wrap_s, Wrapping::Repeat);
    assert_eq!(map.borrow().wrap_t, Wrapping::Repeat);
    // 512 -> 10 levels, which is what `WebGPUTextureUtils` builds.
    assert_eq!(map.mip_level_count(), 10);

    // One `Texture` object for both channels of the ORM map.
    let metalness = material.metalness_map.as_ref().expect("metalnessMap");
    let roughness = material.roughness_map.as_ref().expect("roughnessMap");
    assert_eq!(metalness.id(), roughness.id());
    assert_eq!(metalness.color_space(), ColorSpace::NoColorSpace);

    let normal = material.normal_map.as_ref().expect("normalMap");
    assert_eq!(normal.color_space(), ColorSpace::NoColorSpace);
    assert_eq!(normal.size(), (512, 512));

    let specular = material
        .specular_color_map
        .as_ref()
        .expect("specularColorTexture");
    assert_eq!(specular.color_space(), ColorSpace::SRGB);
}

/// An extension in `extensionsRequired` that the port does not read is an
/// error. three.js only warns (`'Unknown extension'`) and decodes nothing:
/// before Draco was ported, `IridescentDishWithOlives.glb` loaded
/// "successfully" that way into four zero-sized meshes. See the message of
/// the commit that added this check. The extension here is made up, so the
/// test outlives every real one being ported.
#[test]
fn unread_required_extension_is_an_error() {
    let json = br#"{
        "asset": { "version": "2.0" },
        "extensionsUsed": [ "EXT_not_ported_here" ],
        "extensionsRequired": [ "EXT_not_ported_here" ]
    }"#;
    let Err(error) = GLTFLoader::parse(json, std::path::PathBuf::from(".")) else {
        panic!("an asset requiring an unread extension must not load");
    };

    assert_eq!(
        error.to_string(),
        "THREE.GLTFLoader: unknown required extension \"EXT_not_ported_here\""
    );
}

/// Every texture reference on a material is a `GltfTextureRef`, including the
/// two the anisotropy rung left as bare indices: `anisotropyTexture` and
/// `clearcoatNormalTexture` carry `texCoord` and `KHR_texture_transform` like
/// any other map (`docs/nodes.md` §25.4). `AnisotropyBarnLamp.glb` writes both
/// as plain `{ index }`, so the texCoord/transform half is exercised against a
/// document built here.
#[test]
fn anisotropy_and_clearcoat_maps_are_texture_refs() {
    // The asset the `webgpu_loader_gltf_anisotropy` rung draws: bare indices,
    // no `texCoord`, no transform.
    let gltf = GLTFLoader::load(models().join("AnisotropyBarnLamp.glb")).unwrap();
    let metal = &gltf.materials[0];

    let anisotropy = metal
        .anisotropy_texture
        .as_ref()
        .expect("anisotropyTexture");
    assert_eq!(anisotropy.index, 3);
    assert_eq!(anisotropy.tex_coord, None);
    assert!(anisotropy.transform.is_none());

    let clearcoat = metal
        .clearcoat_normal_texture
        .as_ref()
        .expect("clearcoatNormalTexture");
    assert_eq!(clearcoat.index, 1);
    assert_eq!(clearcoat.tex_coord, None);
    assert!(clearcoat.transform.is_none());
    assert_eq!(metal.clearcoat_normal_scale, 1.0);

    // The same two references with the full `{ index, texCoord, extensions }`
    // shape the sheen rung's parser reads.
    let json = br#"{
        "asset": { "version": "2.0" },
        "scenes": [ { "nodes": [] } ],
        "scene": 0,
        "materials": [ {
            "extensions": {
                "KHR_materials_anisotropy": {
                    "anisotropyStrength": 0.5,
                    "anisotropyRotation": 1.25,
                    "anisotropyTexture": {
                        "index": 2,
                        "texCoord": 1,
                        "extensions": {
                            "KHR_texture_transform": {
                                "offset": [ 0.25, 0.5 ],
                                "scale": [ 2.0, 3.0 ],
                                "rotation": 0.5,
                                "texCoord": 1
                            }
                        }
                    }
                },
                "KHR_materials_clearcoat": {
                    "clearcoatFactor": 1.0,
                    "clearcoatNormalTexture": {
                        "index": 4,
                        "texCoord": 2,
                        "scale": 0.75,
                        "extensions": {
                            "KHR_texture_transform": { "offset": [ 1.0, 2.0 ] }
                        }
                    }
                }
            }
        } ]
    }"#;

    let gltf = GLTFLoader::parse(json, std::path::PathBuf::from(".")).unwrap();
    let material = &gltf.materials[0];

    let anisotropy = material
        .anisotropy_texture
        .as_ref()
        .expect("anisotropyTexture");
    assert_eq!(anisotropy.index, 2);
    assert_eq!(anisotropy.tex_coord, Some(1));
    let transform = anisotropy
        .transform
        .as_ref()
        .expect("KHR_texture_transform");
    assert_eq!(transform.offset, Some([0.25, 0.5]));
    assert_eq!(transform.scale, Some([2.0, 3.0]));
    assert_eq!(transform.rotation, Some(0.5));
    assert_eq!(transform.tex_coord, Some(1));

    let clearcoat = material
        .clearcoat_normal_texture
        .as_ref()
        .expect("clearcoatNormalTexture");
    assert_eq!(clearcoat.index, 4);
    assert_eq!(clearcoat.tex_coord, Some(2));
    let transform = clearcoat.transform.as_ref().expect("KHR_texture_transform");
    assert_eq!(transform.offset, Some([1.0, 2.0]));
    assert_eq!(transform.scale, None);
    assert_eq!(transform.rotation, None);
    // `scale` on the reference is the normal scale, not part of the transform.
    assert_eq!(material.clearcoat_normal_scale, 0.75);
}

/// `KHR_texture_basisu`: a texture whose extension names a KTX 2.0 image is
/// loaded through `KTX2Loader`, and the plain `source` (a PNG fallback, here
/// one that does not exist) is never read — `GLTFTextureBasisUExtension`
/// runs before the default `loadTexture`. None of three's own basisu assets
/// loads in the port yet (`CarbonFrameBike.glb` also needs Draco,
/// `facecap.glb` and `coffeemat.glb` meshopt), so the document is built here
/// around one of the `textures/ktx2/` samples. The transcode itself is
/// checked byte for byte against three in `tests/ktx2_loader.rs`.
#[test]
fn khr_texture_basisu_goes_through_ktx2_loader() {
    use three_rs::loaders::{Ktx2Loader, Ktx2Support};
    use three_rs::textures::{ColorSpace, MinFilter};

    let ktx2 = three_rs::testing::three_js_dir().join("examples/textures/ktx2");
    // One triangle: three `vec3<f32>`, base64.
    let positions: Vec<u8> = [0.0f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let json = format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": [ "KHR_texture_basisu" ],
        "extensionsRequired": [ "KHR_texture_basisu" ],
        "buffers": [ {{ "byteLength": 36, "uri": "data:application/octet-stream;base64,{}" }} ],
        "bufferViews": [ {{ "buffer": 0, "byteLength": 36 }} ],
        "accessors": [ {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3",
                          "min": [ 0, 0, 0 ], "max": [ 1, 1, 0 ] }} ],
        "images": [ {{ "uri": "2d_etc1s.ktx2" }}, {{ "uri": "no-such-fallback.png" }} ],
        "textures": [ {{ "source": 1, "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }} }} ],
        "materials": [ {{ "pbrMetallicRoughness": {{ "baseColorTexture": {{ "index": 0 }} }} }} ],
        "meshes": [ {{ "primitives": [ {{ "attributes": {{ "POSITION": 0 }}, "material": 0 }} ] }} ],
        "nodes": [ {{ "mesh": 0 }} ],
        "scenes": [ {{ "nodes": [ 0 ] }} ],
        "scene": 0
    }}"#,
        base64_encode(&positions)
    );

    let map_of = |gltf: &three_rs::loaders::Gltf| {
        let node = gltf.primitives[0].node.borrow();
        let material = node.payload.material().expect("a material");
        material.map.clone().expect("baseColorTexture")
    };

    // The default loader: no `setKTX2Loader`, so the RGBA fallback.
    let gltf = GLTFLoader::parse(json.as_bytes(), ktx2.clone()).unwrap();
    let map = map_of(&gltf);
    assert_eq!(map.size(), (40, 40));
    assert_eq!(map.format(), wgpu::TextureFormat::Rgba8UnormSrgb);
    assert_eq!(map.color_space(), ColorSpace::SRGB);
    assert_eq!(map.mip_level_count(), 6, "the KTX2 mip chain is kept");
    assert!(!map.borrow().flip_y);
    // The glTF sampler's defaults win over the KTX2 texture's filters.
    assert_eq!(map.borrow().min_filter, MinFilter::LinearMipmapLinear);

    // A loader that has seen a BC-capable device: BC7.
    let bc = Ktx2Loader::new().with_support(Ktx2Support {
        bptc: true,
        dxt: true,
        ..Default::default()
    });
    let gltf = GLTFLoader::parse_with_ktx2(json.as_bytes(), ktx2, &bc).unwrap();
    assert_eq!(
        map_of(&gltf).format(),
        wgpu::TextureFormat::Bc7RgbaUnormSrgb
    );
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
