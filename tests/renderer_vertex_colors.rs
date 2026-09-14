//! `material.vertexColors` on a line material: one `LineSegments` carrying a
//! hue per vertex instead of one hue per draw call (issue #46).
//!
//! `MeshBasicNodeMaterial::line( colour )` puts the colour on the material, so
//! "one hue per branch" used to mean one `LineSegments` per hue — O(branches)
//! draw calls for a dendrogram. three.js answers that with
//! `LineBasicMaterial.vertexColors` and a `color` attribute, which
//! `NodeMaterial.setupDiffuseColor()` turns into
//! `colorNode = colorNode.mul( vertexColor() )`.
//!
//! The camera is the pixel camera of `renderer_lines.rs`: an
//! `OrthographicCamera( 0, W, H, 0, -1, 1 )` at the origin makes one world unit
//! one pixel, so a vertex at world `( x, y )` lands at framebuffer column `x`
//! and row `H - 1 - y` when both are on pixel centres. Both segments here are
//! horizontal and pinned to pixel centres, the one case where a hairline
//! rasteriser has no freedom, and each is sampled at its middle — never at an
//! endpoint, where the diamond-exit rule decides coverage.
//!
//! The colours are saturated primaries and the material colour is white, so
//! each channel is 0 or 1 through the diffuse multiply and stays 0 or 255
//! through the sRGB transfer, which has both as fixed points.

use std::rc::Rc;

use three_rs::core::{BufferAttribute, BufferGeometry};
use three_rs::{
    Color, LineSegments, MeshBasicNodeMaterial, OrthographicCamera, Renderer, RendererParameters,
    Scene,
};

const W: usize = 64;
const H: usize = 64;

/// The two segments: both span `x ∈ [ X0, X1 ]`, one at `Y_RED` and one at
/// `Y_GREEN`, all on pixel centres.
const X0: f64 = 10.5;
const X1: f64 = 40.5;
const Y_RED: f64 = 20.5;
const Y_GREEN: f64 = 40.5;

fn column_of(x: f64) -> usize {
    (x - 0.5).round() as usize
}

fn row_of(y: f64) -> usize {
    H - 1 - (y - 0.5).round() as usize
}

fn rgb(pixels: &[u8], column: usize, row: usize) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// `new BufferGeometry()` with a `position` and a `color` attribute — the
/// four vertices of two independent segments, and a colour for each.
fn two_segments(colors: Vec<f32>) -> Rc<BufferGeometry> {
    let (x0, x1) = (X0 as f32, X1 as f32);
    let (red, green) = (Y_RED as f32, Y_GREEN as f32);
    let positions = vec![
        x0, red, 0.0, //
        x1, red, 0.0, //
        x0, green, 0.0, //
        x1, green, 0.0,
    ];
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute("position", BufferAttribute::new(positions, 3));
    geometry.set_attribute("color", BufferAttribute::new(colors, 3));
    Rc::new(geometry)
}

/// Render one `LineSegments` on a black background and read the canvas back.
fn render(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Vec<u8> {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(&LineSegments::new(geometry, material));

    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0);
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer.render(&mut scene, &mut camera);

    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width as usize, height as usize), (W, H));
    pixels
}

/// The pixel at the middle of a segment at world height `y`.
fn middle(pixels: &[u8], y: f64) -> [u8; 3] {
    rgb(pixels, column_of((X0 + X1) / 2.0), row_of(y))
}

/// Two segments, one `LineSegments`, two hues.
#[test]
fn a_line_segments_carries_a_colour_per_vertex() {
    let geometry = two_segments(vec![
        1.0, 0.0, 0.0, // the red segment's two ends
        1.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, // the green segment's two ends
        0.0, 1.0, 0.0,
    ]);

    let mut material = MeshBasicNodeMaterial::line(Color::from_hex(0xffffff));
    material.vertex_colors = true;

    let pixels = render(geometry, material);
    assert_eq!(
        middle(&pixels, Y_RED),
        [255, 0, 0],
        "the first segment should be red"
    );
    assert_eq!(
        middle(&pixels, Y_GREEN),
        [0, 255, 0],
        "the second segment should be green"
    );
}

/// `vertexColors` is off by default, and off means the `color` attribute is not
/// read: the same geometry draws in the material's colour. This is what keeps
/// the change invisible to every existing line.
#[test]
fn without_the_flag_the_material_colour_wins() {
    let geometry = two_segments(vec![
        1.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, //
        0.0, 1.0, 0.0,
    ]);

    let material = MeshBasicNodeMaterial::line(Color::from_hex(0xffffff));
    assert!(!material.vertex_colors, "vertexColors defaults to false");

    let pixels = render(geometry, material);
    assert_eq!(middle(&pixels, Y_RED), [255, 255, 255]);
    assert_eq!(middle(&pixels, Y_GREEN), [255, 255, 255]);
}

/// The material's colour still multiplies the vertex colour, as
/// `colorNode.mul( vertexColor() )` says: a white-and-red geometry through a
/// red material is red and black, not white and red.
#[test]
fn the_material_colour_multiplies_the_vertex_colour() {
    let geometry = two_segments(vec![
        1.0, 1.0, 1.0, // white
        1.0, 1.0, 1.0, //
        0.0, 1.0, 0.0, // green
        0.0, 1.0, 0.0,
    ]);

    let mut material = MeshBasicNodeMaterial::line(Color::from_hex(0xff0000));
    material.vertex_colors = true;

    let pixels = render(geometry, material);
    assert_eq!(
        middle(&pixels, Y_RED),
        [255, 0, 0],
        "white vertices take the material's red"
    );
    assert_eq!(
        middle(&pixels, Y_GREEN),
        [0, 0, 0],
        "a green vertex through a red material has nothing left"
    );
}
