//! `LightProbeGenerator.fromCubeTexture()` (`src/addons/lights.rs`) and the
//! irradiance a probe's coefficients give back.
//!
//! The first two tests check the projection against what it must be: a cube
//! of one colour `L` has radiance `L` in every direction, so only the constant
//! band survives, `c0 = L · Y00 · 4π`, and the irradiance it gives back is
//! `π · L` for every normal. The last two check the port against three.js
//! itself: `tools/light_probe_generator_reference.mjs` runs three's own
//! `LightProbeGenerator.js` under node on the same face bytes, which pins the
//! face table (which direction each texel of each face is) and the
//! sRGB-to-linear conversion. Those two skip, with a note, when there is no
//! three.js checkout or no node.

use std::f64::consts::PI;
use std::path::Path;
use std::process::Command;

use three_rs::addons::lights::LightProbeGenerator;
use three_rs::math::{ColorSpace, SphericalHarmonics3, Vector3};
use three_rs::textures::Image;
use three_rs::{CubeTexture, CubeTextureLoader};

/// `SphericalHarmonics3.getBasisAt()`'s band-0 constant, `1 / (2·sqrt(π))`.
const Y00: f64 = 0.282095;

/// The probe's coefficients, out of the `LightProbe` the generator returns.
fn coefficients(cube: &CubeTexture) -> SphericalHarmonics3 {
    let probe = LightProbeGenerator::from_cube_texture(cube).expect("an RGBA8 cube");
    let object = probe.borrow();
    let light = object.light().expect("a LightProbe");
    assert_eq!(light.light.intensity, 1.0, "fromCubeTexture's intensity");
    light.sh
}

/// Six `size`-texel faces of one RGBA8 colour.
fn uniform_cube(size: u32, rgba: [u8; 4]) -> CubeTexture {
    let face = rgba.repeat((size * size) as usize);
    CubeTexture::new(
        (0..6)
            .map(|_| Image::rgba8(size, size, face.clone()))
            .collect(),
    )
}

#[test]
fn uniform_cube_projects_to_the_constant_band_only() {
    let cube = uniform_cube(16, [128, 64, 255, 255]);
    let sh = coefficients(&cube);
    let radiance = [128.0 / 255.0, 64.0 / 255.0, 1.0];

    let c0 = sh.coefficients[0];
    for (channel, (got, l)) in [c0.x, c0.y, c0.z].into_iter().zip(radiance).enumerate() {
        let want = l * Y00 * 4.0 * PI;
        assert!(
            (got - want).abs() < 1e-9,
            "c0[{channel}] is {got}, L·Y00·4π is {want}"
        );
    }
    // The cube is symmetric under every permutation and sign flip of the axes,
    // so the higher bands cancel exactly, not just in the limit.
    for (index, c) in sh.coefficients.iter().enumerate().skip(1) {
        assert!(
            c.x.abs() < 1e-9 && c.y.abs() < 1e-9 && c.z.abs() < 1e-9,
            "c{index} is {c:?}, should vanish for a uniform cube"
        );
    }
}

#[test]
fn uniform_cube_gives_pi_l_irradiance() {
    let cube = uniform_cube(8, [200, 100, 50, 255]);
    let sh = coefficients(&cube);
    let radiance = [200.0 / 255.0, 100.0 / 255.0, 50.0 / 255.0];

    for normal in [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        *Vector3::new(1.0, 2.0, -3.0).normalize(),
    ] {
        let e = sh.get_irradiance_at(&normal);
        for (got, l) in [e.x, e.y, e.z].into_iter().zip(radiance) {
            // 0.282095 · 0.886227 · 4 = 0.99999989…: three's rounded constants.
            assert!(
                (got - PI * l).abs() < 1e-5,
                "irradiance at {normal:?} is {e:?}, π·L is {:?}",
                radiance.map(|l| PI * l)
            );
        }
    }
}

/// A cube whose every texel is different and whose faces are not symmetric,
/// so a swapped face, a flipped row or a mirrored column moves the answer.
fn patterned_cube(size: u32) -> CubeTexture {
    let images = (0..6u32)
        .map(|face| {
            let mut data = Vec::with_capacity((size * size * 4) as usize);
            for y in 0..size {
                for x in 0..size {
                    data.push((face * 40 + x * 255 / size) as u8);
                    data.push((255 - y * 255 / size) as u8);
                    data.push(((face * 97 + x * 13 + y * 7) % 256) as u8);
                    data.push(255);
                }
            }
            Image::rgba8(size, size, data)
        })
        .collect();
    CubeTexture::new(images)
}

/// three.js' own `fromCubeTexture()` on `cube`, or `None` (with a note) when
/// there is no checkout or no node to run it with.
fn reference(name: &str, cube: &CubeTexture) -> Option<Vec<f64>> {
    let three = three_rs::testing::three_js_dir();
    if !three
        .join("examples/jsm/lights/LightProbeGenerator.js")
        .exists()
    {
        eprintln!(
            "skipping: no three.js checkout at {} (set THREE_JS_DIR)",
            three.display()
        );
        return None;
    }

    let inner = cube.borrow();
    let size = inner.images[0].width;
    let faces: Vec<u8> = inner
        .images
        .iter()
        .flat_map(|image| image.data.clone())
        .collect();
    let color_space = match inner.color_space {
        ColorSpace::NoColorSpace => "",
        ColorSpace::Srgb => "srgb",
        ColorSpace::LinearSrgb => "srgb-linear",
        other => panic!("no reference for {other:?}"),
    };
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("light_probe_{name}.rgba"));
    std::fs::write(&path, faces).unwrap();

    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/light_probe_generator_reference.mjs");
    let output = match Command::new("node")
        .arg(&script)
        .arg(&three)
        .arg(&path)
        .arg(size.to_string())
        .arg(color_space)
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
    Some(
        serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
            .expect("the reference script's JSON array"),
    )
}

/// Asserts the port's 27 numbers are three's, to rounding.
fn assert_matches_three(name: &str, cube: &CubeTexture) {
    let Some(want) = reference(name, cube) else {
        return;
    };
    let got = coefficients(cube).to_array();
    assert_eq!(
        want.len(),
        27,
        "{name}: three.js gave {} numbers",
        want.len()
    );
    for (index, (want, got)) in want.iter().zip(got).enumerate() {
        assert!(
            (want - got).abs() <= 1e-9 * want.abs().max(1.0),
            "{name}: coefficient {} channel {} is {got}, three.js says {want}",
            index / 3,
            index % 3
        );
    }
}

#[test]
fn patterned_cube_matches_three_js() {
    let cube = patterned_cube(12);
    assert_matches_three("patterned_linear", &cube);
    cube.set_color_space(ColorSpace::Srgb);
    assert_matches_three("patterned_srgb", &cube);
}

/// The cube `webgpu_lightprobe` projects, decoded by the port's own loader.
#[test]
fn pisa_cube_matches_three_js() {
    let dir = three_rs::testing::three_js_dir().join("examples/textures/cube/pisa/");
    if !dir.exists() {
        eprintln!("skipping: no pisa cube at {}", dir.display());
        return;
    }
    let cube = CubeTextureLoader::new()
        .set_path(dir)
        .load(["px.png", "nx.png", "py.png", "ny.png", "pz.png", "nz.png"])
        .unwrap();
    assert_matches_three("pisa", &cube);
}
