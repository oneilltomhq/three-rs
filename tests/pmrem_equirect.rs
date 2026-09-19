//! The GPU half of `PMREMGenerator.fromEquirectangular` — the two gates the
//! graded image cannot separate.
//!
//! `spot1Lux.hdr` is the perfect oracle for both: a 1024×512 black image with
//! **one** bright texel at ( 597, 213 ). One lit texel in a black field means
//! a missing flip, an off-by-one flip, a row-stride bug, a permuted face or a
//! smeared blit each land somewhere provably wrong, where an environment map
//! with structure in it would just look like a different plausible sky.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use three_rs::loaders::HdrLoader;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{texture_uv, uv};
use three_rs::renderer::{RenderTarget, RenderTargetOptions};
use three_rs::testing::three_js_dir;
use three_rs::textures::MinFilter;
use three_rs::{QuadMesh, Renderer, RendererParameters, Texture, TextureFilter, TextureType};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 512;
/// `_setSize( 1024 / 4 )` ⇒ `lodMax = 8`, `cubeSize = 256`, atlas 768×1024.
const FACE: u32 = 256;
const ATLAS_WIDTH: u32 = 768;
const ATLAS_HEIGHT: u32 = 1024;
/// `_lodMeshes.length` — `lodMax - LOD_MIN + 1 + EXTRA_LODS`.
const LOD_COUNT: usize = 11;

fn spot1lux() -> Texture {
    HdrLoader::new()
        .load(three_js_dir().join("examples/textures/equirectangular/spot1Lux.hdr"))
        .expect("the vendor tree carries spot1Lux.hdr")
}

fn half_float_target(width: u32, height: u32) -> RenderTarget {
    RenderTarget::new_with_options(
        width,
        height,
        RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: false,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )
    .expect("HalfFloatType is a colour type")
}

/// The `( x, y )` and value of every texel whose rgb is not all zero, `y` down
/// from the top, inside `( x0, y0, w, h )` of a `width`-wide readback.
fn lit(pixels: &[f32], width: u32, rect: (u32, u32, u32, u32)) -> Vec<(u32, u32, [f32; 3])> {
    let (x0, y0, w, h) = rect;
    let mut out = Vec::new();
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let i = ((y * width + x) * 4) as usize;
            let rgb = [pixels[i], pixels[i + 1], pixels[i + 2]];
            if rgb.iter().any(|c| *c != 0.0) {
                out.push((x, y, rgb));
            }
        }
    }
    out
}

#[test]
fn the_equirect_source_and_the_atlas_it_becomes() {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();

    the_bright_texel_lands_on_the_flipped_row(&mut renderer);
    the_atlas_lights_one_face_and_every_lod(&mut renderer);
}

/// **The `flipY` gate.** `HDRLoader` sets `texData.flipY = true`, so the
/// decoded rows are uploaded bottom-up: the bright texel of row 213 must be
/// sampled from row `512 - 1 - 213 = 298`.
///
/// Three does this with two extra render passes — `WebGPUTextureUtils._flipY()`
/// borrows the mipmap blit pipeline and bounces the source through a scratch
/// texture — which is why its dump of this example has 25 passes where the
/// port has 23. The port reverses the rows on the CPU in `upload_texture_2d`
/// instead. A flip is an exact texel permutation, so the two are bit-identical
/// and this test is what says so: the texel is read back through a real
/// sample, at its own texel centre, with nearest filtering, and it has to be
/// the *only* lit one.
///
/// Listed as a divergence in `docs/nodes.md` §8.
fn the_bright_texel_lands_on_the_flipped_row(renderer: &mut Renderer) {
    let map = spot1lux();
    // Nearest, so each target texel is exactly one source texel and a
    // half-texel error cannot be interpolated away into something plausible.
    map.set_min_filter(MinFilter::Nearest);
    map.set_mag_filter(TextureFilter::Nearest);

    let target = half_float_target(WIDTH, HEIGHT);
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(texture_uv(&map, uv()));
    material.depth_test = false;
    material.depth_write = false;
    renderer.set_render_target(Some(target.clone()));
    renderer.render_quad(&QuadMesh::new(material));
    renderer.set_render_target(None);

    let (width, height, pixels) = renderer.read_target_pixels_rgba16f(&target).unwrap();
    assert_eq!((width, height), (WIDTH, HEIGHT));

    let lit = lit(&pixels, WIDTH, (0, 0, WIDTH, HEIGHT));
    assert_eq!(
        lit.len(),
        1,
        "spot1Lux.hdr has exactly one non-black texel; {} came back lit",
        lit.len()
    );
    let (x, y, rgb) = lit[0];
    assert_eq!(
        (x, y),
        (597, HEIGHT - 1 - 213),
        "the bright texel is at ( 597, 213 ) of the decoded rows and flipY = true \
         puts it on row {}",
        HEIGHT - 1 - 213
    );
    // Grey, and the half three's own `HDRLoader.parse` produces for it —
    // `tests/hdr_loader.rs` grades the halves against the oracle, so here it is
    // enough that the sample round-tripped the value rather than a filtered
    // blend of it with its black neighbours.
    assert_eq!(rgb[0], rgb[1]);
    assert_eq!(rgb[1], rgb[2]);
    assert_eq!(rgb[0], 26464.0);
}

/// **The atlas gate.** `fromEquirectangular` through the inherited GGX chain:
/// a 768×1024 atlas, one delta direction, so
///
/// * exactly one of the six mip-0 face tiles is lit and the other five are
///   black — a permuted `FACE_LIB`, a smeared blit or a viewport in the wrong
///   tile all break this, and none of them would stop the image rendering;
/// * every one of the eleven LOD tiles is lit, which is the whole prefilter
///   ladder having run through this entry point and not just the first pass.
///
/// This is `webgpu_pmrem_cubemap`'s gate re-run through the new entry point;
/// what it adds is that the equirect material really wrote the atlas, since a
/// shader that sampled the source at the wrong uv would light a different face
/// or none.
fn the_atlas_lights_one_face_and_every_lod(renderer: &mut Renderer) {
    let mut environment = PmremEnvironment::from_equirectangular(&spot1lux());
    environment.update(renderer).unwrap();
    let target = environment.target().expect("update() built the atlas");
    assert_eq!(target.size(), (ATLAS_WIDTH, ATLAS_HEIGHT));

    let (width, _, pixels) = renderer.read_target_pixels_rgba16f(target).unwrap();

    // Mip 0: the six faces, three across and two down, 256 each.
    let faces: Vec<usize> = (0..6)
        .map(|face| {
            let (col, row) = (face % 3, face / 3);
            lit(
                &pixels,
                width,
                (col as u32 * FACE, row as u32 * FACE, FACE, FACE),
            )
            .len()
        })
        .collect();
    assert_eq!(
        faces.iter().filter(|n| **n > 0).count(),
        1,
        "one direction lights one face of mip 0; lit texels per face were {faces:?}"
    );

    // The eleven LOD tiles, from the generator's own geometry: the prefilter
    // spreads the delta over the whole sphere, so every level is lit.
    let sizes: Vec<usize> = three_rs::renderer::pmrem::create_planes(8)
        .iter()
        .map(|mesh| mesh.size)
        .collect();
    assert_eq!(sizes.len(), LOD_COUNT);
    for (lod, size) in sizes.iter().enumerate() {
        let (x, y, size) = three_rs::renderer::pmrem::tile_rect(8, FACE as usize, *size, lod);
        let count = lit(
            &pixels,
            width,
            (x as u32, y as u32, 3 * size as u32, 2 * size as u32),
        )
        .len();
        assert!(
            count > 0,
            "LOD {lod} ({size}² faces at ( {x}, {y} )) is black, so its GGX step did not run"
        );
    }
}
