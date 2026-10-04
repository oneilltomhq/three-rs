//! A map's filters and wrap modes are part of the program's cache key
//! (issue #276).
//!
//! On the WebGPU backend `RenderObject.getMaterialCacheKey()` folds every
//! texture-valued material property's `magFilter`, `minFilter`, `wrapS`,
//! `wrapT` and `wrapR` into the key, because they reach the generated code:
//! `NearestFilter` on both filters makes the map unfilterable, which binds it
//! `non-filtering` with no sampler and turns each tap into a `textureLoad`,
//! and that `textureLoad` wraps its coordinate through the
//! `tsl_coord_<s>S_<t>T_2d` function named after the wrap pair (#275). The
//! port keyed a material's programs on its id and version only, so a change
//! to either after the first draw kept drawing with the stale program.
//!
//! One quad, one map, three changes after the first draw:
//!
//! 1. `Linear` → `Nearest` on both filters: the texel boundary under the probe
//!    goes from a blend to one pure texel, and the frame builds one program,
//!    the unfilterable one, whose WGSL is new.
//! 2. `wrap_s` `ClampToEdge` → `Repeat` on that now-unfilterable map: the half
//!    of the quad past `u = 1` goes from the clamped edge texel to the tiled
//!    checker. Only a rebuild can do this, because the `textureLoad` path has
//!    no sampler to carry the wrap mode; the stale program clamps forever.
//! 3. Back to `Linear`: the blend returns. The stale (unfilterable) program
//!    would keep the pure texel, since `textureLoad` never interpolates.
//!
//! Each change frame builds exactly one program and the frame after it builds
//! none, so the sampler state costs a steady frame nothing.
//!
//! The set-up is `renderer_material_map.rs`' pixel camera: an
//! `OrthographicCamera( 0, W, H, 0, -1, 1 )` makes one world unit one pixel,
//! and the 32-pixel quad centred in the 64-pixel frame spans columns and rows
//! 16..48. The map is that test's 2x2 checker, with `repeat = ( 2, 2 )` so the
//! quad's uv runs 0..2 and the part past 1 shows what the wrap mode does.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::textures::Wrapping;
use three_rs::{
    Color, Mesh, MinFilter, OrthographicCamera, Renderer, RendererParameters, Scene, Texture,
    TextureFilter,
};

const W: usize = 64;
const H: usize = 64;
/// The quad's half-extent, in pixels.
const HALF: usize = 16;

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];

/// Column 24 is `u = 0.27`, `u' = 2u = 0.53` after the repeat: just past the
/// boundary between texel column 0 (red, centre 0.25) and column 1 (green,
/// centre 0.75), so linear filtering blends the two and nearest picks green.
/// Row 44 is `v' = 0.22`, below texel row 0's centre, where clamp-to-edge
/// keeps the tap on row 0 alone (red, green) whatever the filter.
const SEAM: (usize, usize) = (24, 44);
/// Column 36 is `u' = 1.28`: clamped, that is texel column 1 (green);
/// repeated, it is `fract = 0.28`, texel column 0 (red).
const PAST_ONE: (usize, usize) = (36, 44);

/// The 2x2 checker of `renderer_material_map.rs`: red, green on `v = 0`;
/// blue, white on `v = 1`. Linear filters and no mips to start with.
fn checker() -> Texture {
    let mut data = Vec::new();
    data.extend_from_slice(&[255, 0, 0, 255]);
    data.extend_from_slice(&[0, 255, 0, 255]);
    data.extend_from_slice(&[0, 0, 255, 255]);
    data.extend_from_slice(&[255, 255, 255, 255]);
    let texture = Texture::new(2, 2, Some(data));
    texture.set_flip_y(false);
    texture.set_generate_mipmaps(false);
    texture.set_mag_filter(TextureFilter::Linear);
    texture.set_min_filter(MinFilter::Linear);
    texture.set_repeat(2.0, 2.0);
    texture
}

fn rgb(pixels: &[u8], (column, row): (usize, usize)) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// A red-green blend: both channels well inside (0, 255), no blue.
#[track_caller]
fn assert_blend(texel: [u8; 3], what: &str) {
    assert!(
        (30..=225).contains(&texel[0]) && (30..=225).contains(&texel[1]) && texel[2] == 0,
        "{what}: expected a red-green blend at the seam, got {texel:?}"
    );
}

/// One frame, returning the canvas and the frame's program builds.
fn frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: &mut OrthographicCamera,
) -> (Vec<u8>, u64) {
    renderer.render(scene, camera);
    let builds = renderer.info().build.programs_compiled;
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width as usize, height as usize), (W, H));
    (pixels, builds)
}

/// The change frame builds one program, and the frame after it none.
#[track_caller]
fn change_frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: &mut OrthographicCamera,
    what: &str,
) -> Vec<u8> {
    let (pixels, builds) = frame(renderer, scene, camera);
    assert_eq!(builds, 1, "{what}: the change rebuilds the program");
    let (steady, builds) = frame(renderer, scene, camera);
    assert_eq!(
        builds, 0,
        "{what}: the frame after the change builds nothing"
    );
    assert_eq!(pixels, steady, "{what}: the steady frame draws the same");
    pixels
}

#[test]
fn sampler_state_changes_after_the_first_draw_rebuild_the_program() {
    let map = checker();
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.map = Some(map.clone());

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    let geometry = Rc::new(plane_geometry((HALF * 2) as f64, (HALF * 2) as f64, 1, 1));
    let mesh = Mesh::new(geometry, material);
    mesh.borrow_mut()
        .position
        .set((W / 2) as f64, (H / 2) as f64, 0.0);
    scene.add(&mesh);

    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0);
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);

    // The first draw: filterable, clamped.
    let (linear, builds) = frame(&mut renderer, &mut scene, &mut camera);
    assert!(builds >= 1, "the first frame builds the material");
    assert_blend(rgb(&linear, SEAM), "linear");
    assert_eq!(
        rgb(&linear, PAST_ONE),
        GREEN,
        "linear, clamped: the edge texel"
    );
    let programs_linear = renderer.info().memory.programs;

    // 1. Nearest on both filters: unfilterable, a new program.
    map.set_mag_filter(TextureFilter::Nearest);
    map.set_min_filter(MinFilter::Nearest);
    let nearest = change_frame(&mut renderer, &mut scene, &mut camera, "nearest");
    assert_eq!(rgb(&nearest, SEAM), GREEN, "nearest: one texel at the seam");
    assert_eq!(
        rgb(&nearest, PAST_ONE),
        GREEN,
        "nearest, clamped: the edge texel"
    );
    let programs_nearest = renderer.info().memory.programs;
    assert!(
        programs_nearest > programs_linear,
        "nearest: the unfilterable program is new WGSL \
         ({programs_linear} programs before, {programs_nearest} after)"
    );

    // 2. Repeat on s, on the unfilterable map: the `textureLoad`'s wrap
    //    function changes, so does the tiling.
    map.set_wrapping(Wrapping::Repeat, Wrapping::ClampToEdge);
    let repeated = change_frame(&mut renderer, &mut scene, &mut camera, "repeat");
    assert_eq!(rgb(&repeated, SEAM), GREEN, "repeat: the seam is unchanged");
    assert_eq!(
        rgb(&repeated, PAST_ONE),
        RED,
        "repeat: past u = 1 the checker tiles again"
    );
    let programs_repeat = renderer.info().memory.programs;
    assert!(
        programs_repeat > programs_nearest,
        "repeat: a new `tsl_coord_*` wrap function is new WGSL \
         ({programs_nearest} programs before, {programs_repeat} after)"
    );

    // 3. Back to linear: filterable again, the blend returns.
    map.set_mag_filter(TextureFilter::Linear);
    map.set_min_filter(MinFilter::Linear);
    let relinear = change_frame(&mut renderer, &mut scene, &mut camera, "linear again");
    assert_blend(rgb(&relinear, SEAM), "linear again");
    // `u' = 1.28` repeats to 0.28, just past texel column 0's centre: mostly
    // red with a little green filtered in. Clamped, it would be pure green.
    let past_one = rgb(&relinear, PAST_ONE);
    assert!(
        past_one[0] > 200 && past_one[1] < 120 && past_one[2] == 0,
        "linear again, repeated: the sampler tiles, so mostly red, got {past_one:?}"
    );
}
