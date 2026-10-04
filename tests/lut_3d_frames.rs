//! `lut3D()` on the GPU, over frames. `webgpu_postprocessing_3dlut` grades
//! one table at full intensity on one frame; this checks what the rung
//! cannot see, with two `size = 2` tables parsed by `LutCubeLoader`, an
//! identity and an inversion, over a gradient (`vec4( uv, 0.5, 1 )`) read
//! straight to the canvas (`outputColorTransform = false`):
//!
//! * the inversion maps every colour `c` to `1 - c`: with the half-texel
//!   pull-in, a two-texel table is sampled exactly between its texel
//!   centres, so the trilinear blend is the linear map itself;
//! * the identity gives back the input, and intensity 0 gives back the input
//!   whatever the table;
//! * handing the pipeline a new node built from another table — how the
//!   port's page swaps tables, where three assigns `lutNode.value` — switches
//!   the output on the next frame, and switching back restores the first.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use three_rs::loaders::LutCubeLoader;
use three_rs::nodes::display::lut_3d;
use three_rs::nodes::tsl::{float, texture_3d_sampled, uv, vec4_join};
use three_rs::nodes::NodeRef;
use three_rs::textures::Data3DTexture;
use three_rs::{RenderPipeline, Renderer, RendererParameters};

const SIZE: u32 = 32;

/// A `size = 2` `.cube` table whose texel at `( r, g, b )` is `f( r, g, b )`,
/// rows red fastest as the format specifies.
fn table(f: impl Fn(u32, u32, u32) -> [u32; 3]) -> Data3DTexture {
    let mut text = String::from("TITLE \"test\"\nLUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                let [x, y, z] = f(r, g, b);
                text.push_str(&format!("{x} {y} {z}\n"));
            }
        }
    }
    let lut = LutCubeLoader::new().parse(&text).unwrap();
    assert_eq!(lut.size, 2);
    lut.texture_3d
}

/// `vec4( uv.x, uv.y, 0.5, 1 )`, or its inverse.
fn gradient(inverted: bool) -> NodeRef {
    let (x, y) = (uv().x(), uv().y());
    let (x, y, z) = if inverted {
        (float(1.0).sub(x), float(1.0).sub(y), float(0.5))
    } else {
        (x, y, float(0.5))
    };
    vec4_join(vec![x, y, z, float(1.0)])
}

fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer, output: NodeRef) -> Vec<u8> {
    pipeline.output_node = Some(output);
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels
}

/// Equal to within one step of eight-bit quantisation.
fn assert_close(actual: &[u8], expected: &[u8], what: &str) {
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        let (pixel, channel) = (i / 4, i % 4);
        assert!(
            a.abs_diff(*e) <= 1,
            "{what}: pixel ({}, {}) channel {channel} is {a}, want {e}",
            pixel as u32 % SIZE,
            pixel as u32 / SIZE
        );
    }
}

#[test]
fn lut_3d_maps_swaps_and_blends() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let mut pipeline = RenderPipeline::new();
    pipeline.output_color_transform = false;

    let identity = table(|r, g, b| [r, g, b]);
    let inversion = table(|r, g, b| [1 - r, 1 - g, 1 - b]);

    let plain = frame(&mut pipeline, &mut renderer, gradient(false));
    let plain_inverted = frame(&mut pipeline, &mut renderer, gradient(true));
    assert_ne!(plain, plain_inverted, "the gradient is not symmetric");

    let through = |texture: &Data3DTexture, intensity: f64| {
        lut_3d(
            gradient(false),
            &texture_3d_sampled(texture),
            2.0,
            float(intensity),
        )
        .node()
    };

    let inverted = frame(&mut pipeline, &mut renderer, through(&inversion, 1.0));
    assert_close(&inverted, &plain_inverted, "inversion table");

    // A new node over another table, on the same pipeline.
    let identical = frame(&mut pipeline, &mut renderer, through(&identity, 1.0));
    assert_close(&identical, &plain, "identity table");

    // And back.
    let again = frame(&mut pipeline, &mut renderer, through(&inversion, 1.0));
    assert_eq!(again, inverted, "swapping back changed the frame");

    // `mix( base, lut, 0 )` is the base.
    let off = frame(&mut pipeline, &mut renderer, through(&inversion, 0.0));
    assert_close(&off, &plain, "intensity 0");
}
