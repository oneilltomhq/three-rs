//! `oitPass()`, over frames: what the Weighted Blended OIT composite does
//! with draw order. The page's frame is graded by the `webgpu_oit` e2e rung
//! and its shaders are gated against three's dump in
//! `tests/nodes_display_wgsl.rs`; this checks the property the technique
//! exists for.
//!
//! Two quads, red in front and blue 0.5 behind it, overlapping in the middle,
//! both `MeshBasicNodeMaterial`s with `opacity` 0.5 over a black background,
//! and a small opaque green quad in front of the overlap. The camera is 3
//! from the red quad and 3.5 from the blue one, where equation (9)'s weight
//! is not clamped (its `3e3` upper bound only binds at `z = 0`): about
//! 2984.9 for red and 2972.1 for blue. The composite is their weighted
//! average, close to the equal one, and does not depend on draw order. The
//! expected bytes are derived from the weight, the `r8unorm` revealage and
//! the sRGB encode ([`Oit`]):
//!
//! * the overlap is `mix( ( w_r red + w_b blue ) / ( w_r + w_b ), black,
//!   revealage )` with revealage `0.5 * 0.5` — about `( 0.375, 0, 0.374 )`
//!   linear, 165 and 164–165 in sRGB;
//! * a quad on its own is `mix( colour, black, 0.5 )` — 0.5 linear, 187–188;
//! * where the opaque quad is, it is all there is: the transparent quads are
//!   depth-tested against the depth the default pass wrote. From the second
//!   frame on: on the first, the OIT target's first render clears the depth
//!   it shares, as three's renderer does for a target it has not rendered to
//!   yet — which is why the knot does not hide the planes in the page's
//!   graded first frame — so the test renders one frame of each pipeline
//!   before it looks.
//!
//! `renderOrder` forces the draw order, which is what the transparent sort
//! goes by first. The OIT frame is the same, byte for byte, in both orders,
//! and is the composite above even in the order that is wrong for the
//! painter's algorithm, while a plain `pass()` of the same scene gives a
//! different overlap in each order — so the order really did change.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::display::oit_pass;
use three_rs::{
    pass, Color, Mesh, MeshBasicNodeMaterial, Node, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

const SIZE: u32 = 64;

/// Row 32 runs through both transparent quads below the opaque one; column
/// 20 is the red quad alone, 32 the overlap, 42 the blue quad alone. Row 23
/// column 32 is the opaque quad.
const RED_ONLY: (u32, u32) = (20, 32);
const OVERLAP: (u32, u32) = (32, 32);
const BLUE_ONLY: (u32, u32) = (42, 32);
const OPAQUE: (u32, u32) = (32, 23);

fn render(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels
}

fn rgb(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let i = ((y * SIZE + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

fn assert_near(pixels: &[u8], at: (u32, u32), expected: [u8; 3], what: &str) {
    let got = rgb(pixels, at);
    assert!(
        got.iter()
            .zip(expected)
            .all(|(&g, e)| (g as i32 - e as i32).abs() <= 3),
        "{what} at {at:?}: {got:?}, expected about {expected:?}"
    );
}

/// The quads' `opacity`.
const ALPHA: f64 = 0.5;
/// `positionView.z` of the red and the blue quad: the camera is at `z = 3`.
const RED_Z: f64 = -3.0;
const BLUE_Z: f64 = -3.5;

/// `OITPassNode._getMRTNode()`'s default weight, equation (9): `alpha *
/// clamp( 0.03 / ( ( -z / 200 )^4 + 1e-5 ), 1e-2, 3e3 )`.
fn weight(alpha: f64, view_z: f64) -> f64 {
    alpha * (0.03 / ((-view_z / 200.0).powi(4) + 1e-5)).clamp(1e-2, 3e3)
}

/// A value stored into an 8-bit unorm channel.
fn unorm8(value: f64) -> f64 {
    (value * 255.0).round() / 255.0
}

/// `sRGBTransferOETF`, then the canvas's 8 bits.
fn srgb_byte(linear: f64) -> u8 {
    let encoded = if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

/// The OIT targets at one pixel, accumulated over the quads covering it.
/// `accum` is `rgba16float`, whose rounding is far below a byte here;
/// `revealage` is `r8unorm`, so each `Zero` / `OneMinusSrcColor` blend lands
/// on a multiple of 1 / 255.
struct Oit {
    accum: [f64; 4],
    revealage: f64,
}

impl Oit {
    /// Both cleared: `accum` to `( 0, 0, 0, 0 )`, `revealage` to 1.
    fn new() -> Self {
        Oit {
            accum: [0.0; 4],
            revealage: 1.0,
        }
    }

    /// One fragment: `accum += vec4( rgb * alpha, alpha ) * weight`,
    /// `revealage *= 1 - alpha`.
    fn draw(mut self, rgb: [f64; 3], alpha: f64, view_z: f64) -> Self {
        let w = weight(alpha, view_z);
        for (channel, value) in self.accum.iter_mut().zip([rgb[0], rgb[1], rgb[2], 1.0]) {
            *channel += value * alpha * w;
        }
        self.revealage = unorm8(self.revealage * (1.0 - alpha));
        self
    }

    /// The composite over `beauty`, as the canvas's bytes: `mix(
    /// accum.rgb / max( accum.a, 1e-5 ), beauty.rgb, revealage )`.
    fn over(&self, beauty: [f64; 3]) -> [u8; 3] {
        std::array::from_fn(|i| {
            let color = self.accum[i] / self.accum[3].max(1e-5);
            srgb_byte(color * (1.0 - self.revealage) + beauty[i] * self.revealage)
        })
    }
}

/// Normal blending of `rgb` at `alpha` over `dst`, as the plain pass does.
fn blend_over(dst: [f64; 3], rgb: [f64; 3], alpha: f64) -> [f64; 3] {
    std::array::from_fn(|i| rgb[i] * alpha + dst[i] * (1.0 - alpha))
}

fn bytes(linear: [f64; 3]) -> [u8; 3] {
    linear.map(srgb_byte)
}

fn quad(width: f64, height: f64, color: u32, opacity: Option<f64>) -> Node {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(color);
    if let Some(opacity) = opacity {
        material.transparent = true;
        material.opacity = opacity;
    }
    Mesh::new(Rc::new(plane_geometry(width, height, 1, 1)), material)
}

#[test]
fn oit_composite_does_not_depend_on_draw_order() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let red = quad(1.0, 1.6, 0xff0000, Some(ALPHA));
    red.borrow_mut().position.set(-0.25, 0.0, 0.0);
    scene.add(&red);

    let blue = quad(1.0, 1.6, 0x0000ff, Some(ALPHA));
    blue.borrow_mut().position.set(0.25, 0.0, -0.5);
    scene.add(&blue);

    let green = quad(0.4, 0.2, 0x00ff00, None);
    green.borrow_mut().position.set(0.0, 0.35, 0.3);
    scene.add(&green);

    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 3.0;

    // What each pixel should be, from the formulas above.
    const BLACK: [f64; 3] = [0.0, 0.0, 0.0];
    const RED: [f64; 3] = [1.0, 0.0, 0.0];
    const GREEN: [f64; 3] = [0.0, 1.0, 0.0];
    const BLUE: [f64; 3] = [0.0, 0.0, 1.0];
    let both = Oit::new().draw(RED, ALPHA, RED_Z).draw(BLUE, ALPHA, BLUE_Z);
    let overlap = both.over(BLACK);
    let red_only = Oit::new().draw(RED, ALPHA, RED_Z).over(BLACK);
    let blue_only = Oit::new().draw(BLUE, ALPHA, BLUE_Z).over(BLACK);
    // The first frame: both quads over the opaque green, its depth cleared.
    let first_frame_opaque = both.over(GREEN);
    let plain_red_hiding_blue = bytes(blend_over(BLACK, RED, ALPHA));
    let plain_red_over_blue = bytes(blend_over(blend_over(BLACK, BLUE, ALPHA), RED, ALPHA));

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let oit = oit_pass(scene.clone(), camera.clone());
    let mut oit_pipeline = RenderPipeline::new();
    oit_pipeline.output_node = Some(oit.node());

    let plain = pass(scene.clone(), camera.clone());
    let mut plain_pipeline = RenderPipeline::new();
    plain_pipeline.output_node = Some(plain.node());

    let set_order = |red_order: f64, blue_order: f64| {
        red.borrow_mut().render_order = red_order;
        blue.borrow_mut().render_order = blue_order;
    };

    let first = render(&mut oit_pipeline, &mut renderer);
    assert_near(
        &first,
        OPAQUE,
        first_frame_opaque,
        "the first frame, its depth cleared",
    );
    render(&mut plain_pipeline, &mut renderer);

    // Front to back: the wrong order for sorted blending.
    set_order(0.0, 1.0);
    let oit_wrong = render(&mut oit_pipeline, &mut renderer);
    let plain_wrong = render(&mut plain_pipeline, &mut renderer);

    // Back to front: the painter's order.
    set_order(1.0, 0.0);
    let oit_right = render(&mut oit_pipeline, &mut renderer);
    let plain_right = render(&mut plain_pipeline, &mut renderer);

    // The OIT result, in the wrong order.
    assert_near(&oit_wrong, OVERLAP, overlap, "the weighted average");
    assert_near(&oit_wrong, RED_ONLY, red_only, "the red quad alone");
    assert_near(&oit_wrong, BLUE_ONLY, blue_only, "the blue quad alone");
    assert_near(&oit_wrong, OPAQUE, [0, 255, 0], "the opaque quad");
    assert_eq!(rgb(&oit_wrong, (0, 0)), [0, 0, 0], "the background");

    // The same frame in the other order, every byte of it.
    assert!(
        oit_wrong == oit_right,
        "{} of {} OIT pixels differ between the two draw orders",
        oit_wrong
            .chunks(4)
            .zip(oit_right.chunks(4))
            .filter(|(a, b)| a != b)
            .count(),
        SIZE * SIZE
    );

    // Without OIT the order shows: front-to-back, the red quad's depth write
    // hides the blue one where they overlap; back-to-front, red is blended
    // over blue.
    assert_near(
        &plain_wrong,
        OVERLAP,
        plain_red_hiding_blue,
        "red hiding blue",
    );
    assert_near(&plain_right, OVERLAP, plain_red_over_blue, "red over blue");
    assert_near(&plain_right, OPAQUE, [0, 255, 0], "the opaque quad");
}
