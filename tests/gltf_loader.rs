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
