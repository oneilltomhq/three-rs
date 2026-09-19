//! `PMREMGenerator.fromScene` — the two gates `webgpu_furnace_test`'s image
//! cannot give you.
//!
//! The image is a strong gate for the *material*: a white furnace makes an
//! energy error a visible band. It is a weak gate for the *generator*, because
//! the environment is one colour, so a permuted face, a flipped `up` or a
//! viewport in the wrong tile all produce the same uniform atlas. So the two
//! halves are gated separately:
//!
//! * the six cube-camera bases and the six viewport rectangles, against
//!   three's own `upSign` / `forwardSign` / `_setViewport` arithmetic, with no
//!   GPU at all;
//! * the atlas itself, which under a constant environment has to *stay*
//!   constant through all ten GGX steps — the white-furnace identity one level
//!   below the one the image tests.

use three_rs::math::{Color, Vector3};
use three_rs::renderer::pmrem::{face_camera, face_tile, PmremGenerator};
use three_rs::{Background, Renderer, RendererParameters, Scene};

/// `_setSize( 256 )` ⇒ `lodMax = 8`, atlas 768×1024.
const FACE: u32 = 256;
const ATLAS_WIDTH: u32 = 768;
const ATLAS_HEIGHT: u32 = 1024;
/// `_lodMeshes.length` — `lodMax - LOD_MIN + 1 + EXTRA_LODS`.
const LOD_COUNT: usize = 11;

/// `webgpu_furnace_test`'s `const COLOR = 0xcccccc`.
const COLOR: u32 = 0xcccccc;

/// **The six faces, with no GPU.** Three's `_sceneToCubeUV`:
///
/// ```js
/// const upSign = [ 1, 1, 1, 1, -1, 1 ];
/// const forwardSign = [ 1, -1, 1, -1, 1, -1 ];
/// // col 0: up = ( 0, upSign[i], 0 ),  lookAt( x + forwardSign[i], y, z )
/// // col 1: up = ( 0, 0, upSign[i] ),  lookAt( x, y + forwardSign[i], z )
/// // col 2: up = ( 0, upSign[i], 0 ),  lookAt( x, y, z + forwardSign[i] )
/// this._setViewport( target, col * size, i > 2 ? size : 0, size, size );
/// ```
///
/// Written out here as literals rather than recomputed, so the test fails if
/// the port's table drifts rather than agreeing with itself.
///
/// **That is the WebGPU generator**, `src/renderers/common/extras/`. r186 also
/// ships an older `src/extras/PMREMGenerator.js` for the WebGL renderer, whose
/// tables are `[ 1, -1, 1, 1, 1, 1 ]` / `[ 1, 1, 1, -1, -1, -1 ]` and which
/// draws the background box inside the face loop. Both spell a solid-colour
/// furnace identically, which is exactly why this test holds the right one by
/// hand rather than reading whichever file is open.
#[test]
fn the_six_cube_faces_are_threes() {
    let origin = Vector3::ZERO;
    let expected: [([f64; 3], [f64; 3]); 6] = [
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),  // +x
        ([0.0, 0.0, 1.0], [0.0, -1.0, 0.0]), // second column, up along +z
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),  // +z
        ([0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]), // -x
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0]), // second column, up along -z
        ([0.0, 1.0, 0.0], [0.0, 0.0, -1.0]), // -z
    ];

    for (face, (up, look_at)) in expected.iter().enumerate() {
        let (got_up, got_look_at) = face_camera(face, origin);
        assert_eq!(
            [got_up.x, got_up.y, got_up.z],
            *up,
            "face {face}: up vector"
        );
        assert_eq!(
            [got_look_at.x, got_look_at.y, got_look_at.z],
            *look_at,
            "face {face}: lookAt target"
        );
    }

    let size = FACE as usize;
    let expected_tiles = [
        (0, 0),
        (size, 0),
        (2 * size, 0),
        (0, size),
        (size, size),
        (2 * size, size),
    ];
    for (face, (x, y)) in expected_tiles.iter().enumerate() {
        assert_eq!(
            face_tile(size, face),
            (*x, *y, size, size),
            "face {face}: viewport tile"
        );
    }
}

/// **The atlas is the furnace.** `fromScene` over `webgpu_furnace_test`'s
/// environment scene — one `Color` background and nothing else — has to fill
/// every level of the atlas with that colour:
///
/// * level 0 is the background box, drawn once over the whole atlas through a
///   90° frustum from inside a unit cube;
/// * every GGX step after it convolves a constant with a normalised kernel,
///   which is the constant again. That is the white-furnace identity one level
///   below the material: if the prefilter loses or gains energy, it shows up
///   here, with no BSDF in the way to blame.
///
/// The tolerance is **1% of the value**, justified by the storage rather than
/// by what was measured: the atlas is `rgba16float`, whose relative precision
/// near 0.6 is 2⁻¹¹ ≈ 0.049%, and the ladder re-quantises through it once per
/// GGX step, so the error can only walk up by about that much per level over
/// eleven levels. It does exactly that — 0.0516% at LOD 0 rising to 0.3751%
/// at LOD 10, one ulp at a time — which is the signature of rounding and not
/// of a lost energy term, and leaves the limit about 2.7× clear. Every level's
/// worst case is printed, so a regression that eats the margin is visible
/// before it fails.
///
/// Nothing here comes from a reference image: the expected value is
/// `Color::from_hex( 0xcccccc )`, the page's own constant through three's own
/// sRGB → linear conversion.
#[test]
fn the_atlas_of_a_solid_colour_scene_is_that_colour() {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(COLOR)));

    let mut generator = PmremGenerator::new();
    let target = generator
        .from_scene(&mut renderer, &mut scene, None)
        .unwrap();
    assert_eq!(target.size(), (ATLAS_WIDTH, ATLAS_HEIGHT));

    // `_sceneToCubeUV` puts the background back when it is done.
    assert!(
        matches!(scene.background, Some(Background::Color(_))),
        "the env scene keeps its background; three restores it after borrowing it"
    );

    let expected = Color::from_hex(COLOR).r as f32;
    let (width, _, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();

    let sizes: Vec<usize> = three_rs::renderer::pmrem::create_planes(8)
        .iter()
        .map(|mesh| mesh.size)
        .collect();
    assert_eq!(sizes.len(), LOD_COUNT);

    let mut worst = 0.0f32;
    for (lod, size) in sizes.iter().enumerate() {
        let (x, y, size) = three_rs::renderer::pmrem::tile_rect(8, FACE as usize, *size, lod);
        let (x, y, w, h) = (x as u32, y as u32, 3 * size as u32, 2 * size as u32);

        let mut lod_worst = 0.0f32;
        for row in y..y + h {
            for column in x..x + w {
                let at = ((row * width + column) * 4) as usize;
                for channel in 0..3 {
                    let error = (pixels[at + channel] - expected).abs() / expected;
                    lod_worst = lod_worst.max(error);
                }
            }
        }
        println!(
            "LOD {lod} ({size}² faces at ( {x}, {y} )): worst {:.4}%",
            lod_worst * 100.0
        );
        assert!(
            lod_worst <= 0.01,
            "LOD {lod} drifts {:.3}% off the furnace colour; a constant environment \
             convolved with a normalised GGX kernel is that constant, so this is an \
             energy term, not a rounding error",
            lod_worst * 100.0
        );
        worst = worst.max(lod_worst);
    }
    println!("atlas: worst {:.4}% off {expected}", worst * 100.0);
}
