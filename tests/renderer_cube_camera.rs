//! `CubeCamera` and `LightProbeGenerator.fromCubeRenderTarget()`, against
//! `fromCubeTexture()` on the same environment.
//!
//! A `CubeCamera` at the origin of a scene whose only content is the pisa cube
//! as its background captures that cube again, so projecting the capture must
//! give the coefficients projecting the PNGs gives. They differ only by the
//! capture: the render target is RGBA8 in `NoColorSpace`, so the linear
//! radiance is rounded to 1/255 where the PNGs round the sRGB-encoded value,
//! and the background is sampled with linear filtering at a different
//! resolution (256² against the PNGs' own). That is a fraction of a percent
//! of `c0`; a face read back mirrored, swapped with its opposite or upside
//! down moves the odd bands by far more.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use three_rs::addons::lights::LightProbeGenerator;
use three_rs::math::SphericalHarmonics3;
use three_rs::textures::TextureType;
use three_rs::{
    CubeCamera, CubeRenderTarget, CubeTextureLoader, Renderer, RendererParameters, Scene,
};

fn sh(probe: &three_rs::ObjectRef) -> SphericalHarmonics3 {
    probe.borrow().light().expect("a LightProbe").sh
}

#[test]
fn cube_camera_capture_projects_like_the_cube_it_captured() {
    let dir = three_rs::testing::three_js_dir().join("examples/textures/cube/pisa/");
    if !dir.exists() {
        eprintln!("skipping: no pisa cube at {}", dir.display());
        return;
    }
    let cube_texture = CubeTextureLoader::new()
        .set_path(dir)
        .load(["px.png", "nx.png", "py.png", "ny.png", "pz.png", "nz.png"])
        .unwrap();
    let want = sh(&LightProbeGenerator::from_cube_texture(&cube_texture).unwrap());

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    let mut scene = Scene::new();
    scene.set_background(cube_texture);

    let mut cube_camera = CubeCamera::new(
        1.0,
        1000.0,
        CubeRenderTarget::new(256, TextureType::UnsignedByte).unwrap(),
    );
    cube_camera.update(&mut renderer, &mut scene);
    let got = sh(&LightProbeGenerator::from_cube_render_target(
        &mut renderer,
        &cube_camera.render_target.texture,
    )
    .unwrap());

    let scale = want.coefficients[0].x.abs();
    for (index, (want, got)) in want.coefficients.iter().zip(&got.coefficients).enumerate() {
        for (channel, (want, got)) in [(want.x, got.x), (want.y, got.y), (want.z, got.z)]
            .into_iter()
            .enumerate()
        {
            assert!(
                (want - got).abs() <= 0.01 * scale,
                "c{index}[{channel}]: the capture gives {got}, the PNGs {want} \
                 (c0.r is {scale})"
            );
        }
    }
}
