//! `GLTFLoader` against the assets `webgpu_skinning` loads (Michelle.glb) and
//! the Soldier.glb that the skinning examples share.
//!
//! Expected numbers come from three.js' own `GLTFLoader` in the vendor tree,
//! run under node (see `handoff`/docs/gltf-progress.md for the script).

use three_rs::core::Object3DNode;
use three_rs::loaders::GLTFLoader;

const MODELS: &str = "/home/tom/src/vendor/three.js/examples/models/gltf";

fn names(gltf: &three_rs::loaders::Gltf) -> Vec<String> {
    let mut out = Vec::new();
    gltf.scene
        .traverse(&mut |node| out.push(node.borrow().name.clone()));
    out
}

#[test]
fn michelle_tree() {
    let gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();
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
    let gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();
    let geometry = &gltf.skinned_meshes[0].geometry;

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
        let got = geometry.position().unwrap().array[i] as f64;
        assert!((got - expected).abs() < 1e-6, "position[{i}]: {got}");
    }

    // `skinIndex` is unnormalized `Uint8`/`Uint16`, so it must come out exact
    let skin_index = geometry.get_attribute("skinIndex").unwrap();
    assert_eq!(skin_index.array[0..8], [5.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0]);
}

#[test]
fn michelle_bone_inverses() {
    let gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();
    let skeleton = gltf.skins[0].borrow();

    let expected = [
        100.0, 0.0, 0.0, 0.0, 0.0, 100.0, -0.000_016_292_065_993_184_224, 0.0, 0.0,
        0.000_016_292_065_993_184_224, 100.0, 0.0, 0.0, -102.625_259_399_414_06,
        0.521_240_949_630_737_3, 1.0,
    ];
    for (i, expected) in expected.iter().enumerate() {
        let got = skeleton.bone_inverses[0].elements[i];
        assert!((got - expected).abs() < 1e-6, "boneInverses[0][{i}]: {got}");
    }
}

#[test]
fn michelle_animations() {
    let gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();

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
    let gltf = GLTFLoader::load(format!("{MODELS}/Soldier.glb")).unwrap();
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

    let geometry = &gltf.skinned_meshes[0].geometry;
    assert_eq!(geometry.position().unwrap().count(), 7325);
    assert_eq!(geometry.index.as_ref().unwrap().count(), 33558);
}

/// `mixer.clipAction( clip, root ).play(); mixer.update( 0 )` writes into the
/// tree: the expected bone world matrices are three's, at t = 0 of
/// `SambaDance` (the clip `webgpu_skinning` plays).
#[test]
fn michelle_mixer_at_zero() {
    use three_rs::animation::AnimationMixer;

    let gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();

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

    let mut gltf = GLTFLoader::load(format!("{MODELS}/Michelle.glb")).unwrap();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);
    mixer.update(0.0);
    gltf.scene.update_matrix_world(true);

    // `SkinnedMesh.updateMatrixWorld` is where `bindMatrixInverse` comes from
    // in `AttachedBindMode`; the renderer calls it every frame.
    gltf.skinned_meshes[0].update_matrix_world(true);
    let mesh = &gltf.skinned_meshes[0];
    gltf.skins[0].borrow_mut().update();

    let expected: [f32; 32] = [
        0.822_734_117_507_934_6,
        -0.018_857_240_676_879_883,
        -0.568_113_505_840_301_5,
        0.0,
        -0.024_164_615_198_969_84,
        0.997_385_740_280_151_4,
        -0.068_100_892_007_350_92,
        0.0,
        0.567_912_518_978_118_9,
        0.069_757_185_876_369_48,
        0.820_127_606_391_906_7,
        0.0,
        0.026_579_812_169_075_012,
        -0.034_598_868_340_253_83,
        0.072_963_453_829_288_48,
        1.0,
        0.681_407_332_420_349_1,
        0.066_141_523_420_810_7,
        -0.728_909_671_306_610_1,
        0.0,
        0.012_672_048_993_408_68,
        0.994_692_862_033_844,
        0.102_104_954_421_520_23,
        0.0,
        0.731_794_655_323_028_6,
        -0.078_811_824_321_746_83,
        0.676_952_958_106_994_6,
        0.0,
        -0.012_397_086_247_801_78,
        -0.032_505_188_137_292_86,
        -0.112_112_060_189_247_13,
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
    let position = mesh.geometry.position().unwrap();
    let mut vertex = Vector3::new(position.get_x(0), position.get_y(0), position.get_z(0));
    mesh.apply_bone_transform(0, &mut vertex);

    let expected = [8.248_729_752_697_153, 2.609_101_503_364_952_7, -143.658_634_049_021_8];
    for (i, expected) in expected.iter().enumerate() {
        let got = vertex.get_component(i);
        assert!((got - expected).abs() < 1e-5, "applyBoneTransform[{i}]: {got}");
    }
}
