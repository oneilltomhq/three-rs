//! `PMREMGenerator.fromScene` — the gate `webgpu_furnace_test`'s image cannot
//! give you.
//!
//! The image is a strong gate for the *material*: a white furnace makes an
//! energy error a visible band. It is a weak gate for the *generator*, because
//! the environment is one colour, so a permuted face or a flipped `up`
//! produces the same uniform cube. The face orientation is gated by
//! `webgpu_pmrem_scene`'s e2e rung, which reads each face back; this file gates
//! the prefilter itself: under a constant environment every level of the PMREM
//! cube has to *stay* that constant, through the GGX levels, the integration
//! levels and (with a `sigma`) the two blur passes — the white-furnace
//! identity one level below the one the image tests.

use three_rs::math::Color;
use three_rs::renderer::pmrem::{max_lod_for, PmremGenerator};
use three_rs::{Background, Renderer, RendererParameters, Scene};

/// `fromScene()`'s `_setSize( 256 )`.
const SIZE: u32 = 256;

/// `webgpu_furnace_test`'s `const COLOR = 0xcccccc`.
const COLOR: u32 = 0xcccccc;

/// Captures a solid-colour scene with `sigma` and checks every face of every
/// level.
///
/// The tolerance is **1% of the value**, justified by the storage rather than
/// by what was measured: both cubes are `rgba16float`, whose relative
/// precision near 0.6 is 2⁻¹¹ ≈ 0.049%, and each level is a normalised sum of
/// texels that were themselves rounded, a handful of times over. Every level's
/// worst case is printed, so a regression that eats the margin is visible
/// before it fails.
///
/// Nothing here comes from a reference image: the expected value is
/// `Color::from_hex( 0xcccccc )`, the page's own constant through three's own
/// sRGB → linear conversion.
fn solid_colour_pmrem_is_that_colour(sigma: f64) {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(COLOR)));

    let mut generator = PmremGenerator::new();
    let pmrem = generator
        .from_scene(&mut renderer, &mut scene, sigma, None)
        .unwrap();
    assert_eq!(pmrem.size(), (SIZE, SIZE));
    let max_lod = max_lod_for(SIZE);
    assert_eq!(pmrem.mip_level_count(), max_lod + 1, "maxLod + 1 levels");

    // `fromScene` puts the background back when it is done.
    assert!(
        matches!(scene.background, Some(Background::Color(_))),
        "the env scene keeps its background; three restores it after borrowing it"
    );

    let expected = Color::from_hex(COLOR).r as f32;
    let mut worst = 0.0f32;
    for lod in 0..=max_lod {
        let mut lod_worst = 0.0f32;
        for face in 0..6 {
            let (width, height, pixels) = renderer
                .read_cube_pixels_rgba16f(&pmrem, face, lod)
                .unwrap();
            assert_eq!((width, height), (SIZE >> lod, SIZE >> lod), "lod {lod}");
            for texel in pixels.as_chunks::<4>().0 {
                for &value in &texel[..3] {
                    lod_worst = lod_worst.max((value - expected).abs() / expected);
                }
            }
        }
        println!(
            "sigma {sigma}: LOD {lod} ({}² faces): worst {:.4}%",
            SIZE >> lod,
            lod_worst * 100.0
        );
        assert!(
            lod_worst <= 0.01,
            "LOD {lod} drifts {:.3}% off the furnace colour; a constant environment \
             convolved with a normalised kernel is that constant, so this is an \
             energy term, not a rounding error",
            lod_worst * 100.0
        );
        worst = worst.max(lod_worst);
    }
    println!("sigma {sigma}: worst {:.4}% off {expected}", worst * 100.0);
}

/// `fromScene( scene )` — `webgpu_furnace_test`'s call: capture, mips, then
/// `_applyPMREM`'s GGX and integration levels.
#[test]
fn a_solid_colour_scene_prefilters_to_that_colour() {
    solid_colour_pmrem_is_that_colour(0.0);
}

/// `fromScene( scene, 0.04 )` — every `RoomEnvironment` page: the capture is
/// blurred source → PMREM level 0 → source level 0 before the mips are
/// generated. A blur of a constant is the constant, so a blur that reads the
/// wrong target or the wrong level shows up as drift here.
#[test]
fn a_blurred_solid_colour_scene_prefilters_to_that_colour() {
    solid_colour_pmrem_is_that_colour(0.04);
}
