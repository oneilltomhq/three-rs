//! The stereo display passes, on the GPU. The rung grades the page against
//! three's screenshot and `tests/nodes_display_wgsl.rs` gates the two
//! composite quads' WGSL; this checks what each pass does with its two eyes.
//!
//! `StereoCamera` enables layer 1 on its left eye and layer 2 on its right,
//! as three's does. The scene is two planes that fill the view: a red one on
//! layer 1 only and a green one on layer 2 only, so each eye sees one colour.
//! Then:
//!
//! * `stereoPass` draws the left eye into the left half of the canvas and the
//!   right eye into the right half;
//! * `parallaxBarrierPass` takes the left eye where
//!   `mod( screenCoordinate.y, 2 ) > 1`, which with WebGPU's top-left
//!   framebuffer origin is every odd row from the top, and the right eye on
//!   the even rows;
//! * `anaglyphPass` writes `clamp( Mₗ·L + Mᵣ·R )` with the alpha of the more
//!   opaque eye, so each output channel is the matrices' mix of the two eyes.
//!   That is checked against `anaglyph_matrices()` (itself checked against
//!   three's numbers in `tests/cameras_stereo_camera.rs`) with each eye alone
//!   and with both, and again after a change of algorithm and colour mode.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::display::{
    anaglyph_matrices, anaglyph_pass, parallax_barrier_pass, stereo_pass, AnaglyphAlgorithm,
    AnaglyphColorMode,
};
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

/// A plane far larger than the view, seen by one eye only.
fn eye_plane(scene: &Scene, hex: u32, layer: u32) -> three_rs::ObjectRef {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(hex);
    let mesh = Mesh::new(Rc::new(plane_geometry(100.0, 100.0, 1, 1)), material);
    mesh.borrow_mut().layers.set(layer);
    scene.add(&mesh);
    mesh
}

/// Renders a frame and returns `(width, height, rgba8)`.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> (u32, u32, Vec<u8>) {
    pipeline.render(renderer);
    renderer.read_canvas_pixels().unwrap()
}

fn srgb(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c < 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// `clamp( Mₗ·l + Mᵣ·r, 0, 1 )`, sRGB-encoded as the pipeline's output
/// transform leaves it, in 8-bit levels.
fn anaglyph_expected(
    algorithm: AnaglyphAlgorithm,
    mode: AnaglyphColorMode,
    l: [f64; 3],
    r: [f64; 3],
) -> [f64; 3] {
    let (ml, mr) = anaglyph_matrices(algorithm, mode);
    let mut out = [0.0; 3];
    for (row, o) in out.iter_mut().enumerate() {
        let mut v = 0.0;
        for col in 0..3 {
            v += ml.elements[col * 3 + row] * l[col] + mr.elements[col * 3 + row] * r[col];
        }
        *o = srgb(v) * 255.0;
    }
    out
}

fn assert_every_pixel(
    pixels: &[u8],
    width: u32,
    what: &str,
    expect: impl Fn(u32, u32) -> [f64; 3],
) {
    for (i, p) in pixels.chunks(4).enumerate() {
        let (x, y) = (i as u32 % width, i as u32 / width);
        let want = expect(x, y);
        for c in 0..3 {
            assert!(
                (p[c] as f64 - want[c]).abs() <= 2.0,
                "{what}: pixel ({x}, {y}) is {:?}, expected {want:?}",
                &p[..3]
            );
        }
    }
}

const RED: [f64; 3] = [255.0, 0.0, 0.0];
const GREEN: [f64; 3] = [0.0, 255.0, 0.0];

#[test]
fn stereo_passes_place_each_eye() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 32.0);

    let scene = Scene::new();
    let left = eye_plane(&scene, 0xff0000, 1);
    let right = eye_plane(&scene, 0x00ff00, 2);

    let mut camera = PerspectiveCamera::new(60.0, 2.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 3.0;
    camera.update_projection_matrix();
    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let stereo = stereo_pass(scene.clone(), camera.clone());
    let anaglyph = anaglyph_pass(scene.clone(), camera.clone());
    let barrier = parallax_barrier_pass(scene, camera);
    let mut pipeline = RenderPipeline::new();

    // Side by side: the left eye's red on the left half, the right eye's
    // green on the right.
    pipeline.output_node = Some(stereo.node());
    let (width, height, pixels) = frame(&mut pipeline, &mut renderer);
    assert_eq!((width, height), (64, 32));
    assert_every_pixel(&pixels, width, "stereo", |x, _| {
        if x < width / 2 {
            RED
        } else {
            GREEN
        }
    });

    // Interleaved rows: odd rows from the top are the left eye. This takes
    // `read_canvas_pixels()`'s row 0 to be the top row, as
    // `tests/renderer_viewport.rs` and `tests/renderer_lines.rs` index it;
    // `viewport_and_scissor_measure_from_the_top_left` in the former fails
    // if it were the bottom one.
    pipeline.output_node = Some(barrier.node());
    let (width, _, pixels) = frame(&mut pipeline, &mut renderer);
    assert_every_pixel(&pixels, width, "parallax barrier", |_, y| {
        if y % 2 == 1 {
            RED
        } else {
            GREEN
        }
    });

    // Anaglyph, with the default Dubois red/cyan matrices: both eyes, then
    // each eye alone over the black clear colour.
    pipeline.output_node = Some(anaglyph.node());
    let (dubois, red_cyan) = (AnaglyphAlgorithm::Dubois, AnaglyphColorMode::RedCyan);
    let cases = [
        (true, true, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        (true, false, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        (false, true, [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (show_left, show_right, l, r) in cases {
        left.borrow_mut().visible = show_left;
        right.borrow_mut().visible = show_right;
        let (width, _, pixels) = frame(&mut pipeline, &mut renderer);
        let want = anaglyph_expected(dubois, red_cyan, l, r);
        assert_every_pixel(&pixels, width, "anaglyph", |_, _| want);
    }
    // The left eye alone lands in the red channel only, the right eye alone
    // in green and blue only: the channel separation the glasses rely on.
    let left_only = anaglyph_expected(dubois, red_cyan, [1.0; 3], [0.0; 3]);
    let right_only = anaglyph_expected(dubois, red_cyan, [0.0; 3], [1.0; 3]);
    assert!(left_only[0] > 250.0 && left_only[1] < 1.0 && left_only[2] < 1.0);
    assert!(right_only[0] < 1.0 && right_only[1] > 250.0 && right_only[2] > 250.0);

    // A new algorithm and colour mode take effect on the next frame.
    anaglyph.set_algorithm(AnaglyphAlgorithm::HalfColour);
    anaglyph.set_color_mode(AnaglyphColorMode::MagentaGreen);
    left.borrow_mut().visible = true;
    right.borrow_mut().visible = true;
    let (width, _, pixels) = frame(&mut pipeline, &mut renderer);
    let want = anaglyph_expected(
        AnaglyphAlgorithm::HalfColour,
        AnaglyphColorMode::MagentaGreen,
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    );
    assert_ne!(
        want,
        anaglyph_expected(dubois, red_cyan, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0])
    );
    assert_every_pixel(
        &pixels,
        width,
        "anaglyph half-colour magenta/green",
        |_, _| want,
    );
}
