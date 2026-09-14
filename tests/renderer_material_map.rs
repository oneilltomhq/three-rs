//! `material.map` reaches the diffuse colour on every lighting model, not just
//! the Standard one (issue #65).
//!
//! `NodeMaterial.setupDiffuseColor()` reads `materialColor`, and
//! `MaterialNode.COLOR` is the material's colour *times the map's texel*
//! whenever the material has a map — so an unlit `MeshBasicNodeMaterial { map }`
//! is textured on exactly the same terms as a `MeshStandardNodeMaterial` one.
//! The port used to multiply the texel in only on the Standard path, which made
//! `new MeshBasicMaterial( { map } )` — the commonest material in Three's
//! examples — render the flat colour with the texture silently dropped.
//!
//! The assertion is on the four texels, not on the flat colour: a fix that
//! merely got *some* texture in would still have to put the right texel under
//! each corner of the quad. The map is a 2x2 of four saturated primaries, the
//! material colour is white and the texture is in no colour space, so each
//! texel survives the diffuse multiply and the sRGB transfer bit for bit —
//! 0 and 255 are fixed points of the OETF.
//!
//! The camera is the pixel camera of `renderer_lines.rs`: an
//! `OrthographicCamera( 0, W, H, 0, -1, 1 )` at the origin makes one world unit
//! one pixel, so a `W/2`-wide quad centred at `( W/2, H/2 )` covers the middle
//! of the frame exactly and each of its quadrants is one texel.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::{
    Color, Mesh, MinFilter, OrthographicCamera, Renderer, RendererParameters, Scene, Texture,
    TextureFilter,
};

const W: usize = 64;
const H: usize = 64;
/// The quad's half-extent, in pixels.
const HALF: usize = 16;

/// The four texels of the map, `( u, v )` order within each row: the first row
/// of bytes is `v = 0` because `flipY` is turned off below.
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// A 2x2 checker of four distinguishable texels, sampled `Nearest` so each one
/// covers a quadrant of the quad with no interpolation and no mip selection.
fn checker() -> Texture {
    let mut data = Vec::new();
    // v = 0: red at u = 0, green at u = 1.
    data.extend_from_slice(&RED);
    data.extend_from_slice(&GREEN);
    // v = 1: blue at u = 0, white at u = 1.
    data.extend_from_slice(&BLUE);
    data.extend_from_slice(&WHITE);

    let texture = Texture::new(2, 2, Some(data));
    // `flipY = false` makes the first row of bytes `v = 0`, so the mapping from
    // the array above to uv space is the one the comments claim.
    texture.set_flip_y(false);
    texture.set_generate_mipmaps(false);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_min_filter(MinFilter::Nearest);
    texture
}

/// One quad filling the middle of the frame, drawn with the caller's material.
fn render(material: MeshBasicNodeMaterial) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let geometry = Rc::new(plane_geometry((HALF * 2) as f64, (HALF * 2) as f64, 1, 1));
    let mesh = Mesh::new(geometry, material);
    mesh.borrow_mut()
        .position
        .set((W / 2) as f64, (H / 2) as f64, 0.0);
    scene.add(&mesh);

    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0);
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer.render(&mut scene, &mut camera);

    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width as usize, height as usize), (W, H));
    pixels
}

fn rgb(pixels: &[u8], column: usize, row: usize) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The centre of one quadrant of the quad. `u` grows right; `v` grows *up*,
/// and the camera flips y, so `v = 1` is the upper half of the frame.
#[track_caller]
fn assert_texel(pixels: &[u8], u: usize, v: usize, texel: [u8; 4], what: &str) {
    let column = W / 2 - HALF / 2 + u * HALF;
    let row = H / 2 + HALF / 2 - v * HALF;
    let want = [texel[0], texel[1], texel[2]];
    assert_eq!(
        rgb(pixels, column, row),
        want,
        "{what}: the texel at u = {u}, v = {v} should be {want:?} at pixel ({column}, {row})"
    );
}

/// An unlit `MeshBasicNodeMaterial` with a `map` draws the map.
#[test]
fn a_basic_material_samples_its_map() {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.map = Some(checker());

    let pixels = render(material);
    assert_texel(&pixels, 0, 0, RED, "basic");
    assert_texel(&pixels, 1, 0, GREEN, "basic");
    assert_texel(&pixels, 0, 1, BLUE, "basic");
    assert_texel(&pixels, 1, 1, WHITE, "basic");
}

/// The same, on the Phong path with `lights = false` — the light-sphere
/// material of the Phong rung, whose outgoing light is `DiffuseColor.rgb`.
#[test]
fn an_unlit_phong_material_samples_its_map() {
    let mut material = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
    material.lights = false;
    material.map = Some(checker());

    let pixels = render(material);
    assert_texel(&pixels, 0, 0, RED, "phong");
    assert_texel(&pixels, 1, 0, GREEN, "phong");
    assert_texel(&pixels, 0, 1, BLUE, "phong");
    assert_texel(&pixels, 1, 1, WHITE, "phong");
}
