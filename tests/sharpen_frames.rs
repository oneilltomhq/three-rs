//! `sharpen()`, over frames. Its one page, `webgpu_postprocessing_ssr_denoise`,
//! is not graded yet; the RCAS shader is gated against three's dump in
//! `tests/nodes_display_wgsl.rs`, and what the frames do is checked here.
//!
//! The input is a soft vertical edge, grey 0.25 on the left rising to 0.75 on
//! the right over about six pixels (`smoothstep` across `uv.x`), drawn by
//! [`sharpen`]'s own `convertToTexture()` and read straight to the canvas
//! (`outputColorTransform = false`). Sharpness is a uniform written between
//! frames of one node. RCAS scales its lobe by `exp2( -sharpness )`, so 0 is
//! the strongest setting and a large value is none:
//!
//! * at sharpness 30 the lobe is ~1e-9 and the output is the input;
//! * at sharpness 0 the pixels at the foot of the edge get darker and those
//!   at its shoulder lighter — no pixel on the dark half gets lighter, none
//!   on the light half darker — and the flat regions away from the edge do
//!   not move;
//! * sharpness 1 moves the edge pixels, but less in total than 0;
//! * with `denoise` the lobe is attenuated where the centre stands out from
//!   its ring, so no pixel moves further than without it, the edge moves
//!   less in total, and the flat regions still hold (a NaN would not);
//! * after a resize the target follows the drawing buffer.
//!
//! The one-texel border is left out of every check: RCAS reads its cross
//! with unclamped `textureLoad`s, which off the texture return zero or any
//! texel, as WGSL allows.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use three_rs::nodes::display::sharpen;
use three_rs::nodes::tsl::{float, smoothstep, uniform_settable, uv, vec4_join};
use three_rs::nodes::{NodeRef, Type};
use three_rs::{RenderPipeline, Renderer, RendererParameters};

const SIZE: u32 = 32;

/// `vec4( v, v, v, 1 )` with `v = 0.25 + 0.5 · smoothstep( 0.4, 0.6, uv.x )`.
fn soft_edge() -> NodeRef {
    let v = float(0.25).add(smoothstep(0.4, 0.6, uv().x()).mul(0.5));
    vec4_join(vec![v.clone(), v.clone(), v, float(1.0)])
}

/// The red channel of every canvas pixel, rows top to bottom.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer, size: u32) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (size, size));
    pixels.chunks(4).map(|p| p[0]).collect()
}

/// The pixels off the one-texel border, as `( x, y, value )`.
fn interior(red: &[u8], size: u32) -> impl Iterator<Item = (u32, u32, u8)> + '_ {
    red.iter().enumerate().filter_map(move |(i, &r)| {
        let (x, y) = (i as u32 % size, i as u32 / size);
        (x > 0 && y > 0 && x < size - 1 && y < size - 1).then_some((x, y, r))
    })
}

/// Equal to `plain` within one step of eight-bit quantisation, off the
/// border, at every pixel for which `which( x )` holds.
fn assert_matches(red: &[u8], plain: &[u8], size: u32, which: impl Fn(u32) -> bool, what: &str) {
    for (x, y, r) in interior(red, size) {
        let p = plain[(y * size + x) as usize];
        if which(x) {
            assert!(r.abs_diff(p) <= 1, "{what}: ({x}, {y}) is {r}, want {p}");
        }
    }
}

/// Sum of `|red - plain|` off the border.
fn movement(red: &[u8], plain: &[u8], size: u32) -> u32 {
    interior(red, size)
        .map(|(x, y, r)| u32::from(r.abs_diff(plain[(y * size + x) as usize])))
        .sum()
}

/// Columns well away from the edge, whose whole cross is flat.
fn flat(x: u32) -> bool {
    x <= 9 || x >= 23
}

#[test]
fn sharpen_steepens_a_soft_edge() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let mut pipeline = RenderPipeline::new();
    pipeline.output_color_transform = false;

    pipeline.output_node = Some(soft_edge());
    let plain = frame(&mut pipeline, &mut renderer, SIZE);
    let (dark, light) = (
        plain[(16 * SIZE + 4) as usize],
        plain[(16 * SIZE + 27) as usize],
    );
    assert!(dark < 80 && light > 180, "the edge runs {dark} to {light}");
    let ramp = interior(&plain, SIZE)
        .filter(|&(_, _, r)| r > dark + 1 && r + 1 < light)
        .count();
    assert!(ramp >= 3 * 30, "only {ramp} pixels are on the soft edge");

    let (sharpness_node, sharpness) = uniform_settable(Type::F32, vec![30.0]);
    let sharpened = sharpen(soft_edge(), sharpness_node, false);
    pipeline.output_node = Some(sharpened.node());

    let none = frame(&mut pipeline, &mut renderer, SIZE);
    assert_matches(&none, &plain, SIZE, |_| true, "sharpness 30");

    sharpness.set(vec![0.0]);
    let strongest = frame(&mut pipeline, &mut renderer, SIZE);
    assert_matches(&strongest, &plain, SIZE, flat, "sharpness 0, flat");
    let mut darker = 0;
    let mut lighter = 0;
    for (x, y, r) in interior(&strongest, SIZE) {
        let p = plain[(y * SIZE + x) as usize];
        if p < 128 {
            assert!(r <= p + 1, "dark side ({x}, {y}) went from {p} up to {r}");
            darker += usize::from(r + 2 <= p);
        } else {
            assert!(
                r + 1 >= p,
                "light side ({x}, {y}) went from {p} down to {r}"
            );
            lighter += usize::from(r >= p + 2);
        }
    }
    assert!(
        darker >= 30 && lighter >= 30,
        "{darker} pixels darkened and {lighter} lightened; want a column of each"
    );

    sharpness.set(vec![1.0]);
    let weaker = frame(&mut pipeline, &mut renderer, SIZE);
    assert_matches(&weaker, &plain, SIZE, flat, "sharpness 1, flat");
    let (strong_total, weak_total) = (
        movement(&strongest, &plain, SIZE),
        movement(&weaker, &plain, SIZE),
    );
    assert!(
        weak_total > 0 && weak_total < strong_total,
        "sharpness 1 moved the edge by {weak_total}, sharpness 0 by {strong_total}"
    );

    // `denoise`, at the strongest sharpness.
    let denoised = sharpen(soft_edge(), 0.0, true);
    pipeline.output_node = Some(denoised.node());
    let quiet = frame(&mut pipeline, &mut renderer, SIZE);
    assert_matches(&quiet, &plain, SIZE, flat, "denoise, flat");
    for (x, y, r) in interior(&quiet, SIZE) {
        let i = (y * SIZE + x) as usize;
        assert!(
            r.abs_diff(plain[i]) <= strongest[i].abs_diff(plain[i]) + 1,
            "denoise moved ({x}, {y}) to {r}, further than {} from {}",
            strongest[i],
            plain[i]
        );
    }
    let quiet_total = movement(&quiet, &plain, SIZE);
    assert!(
        quiet_total > 0 && quiet_total < strong_total,
        "denoise moved the edge by {quiet_total}, plain RCAS by {strong_total}"
    );

    // A resize: the target follows the drawing buffer.
    let size = 48;
    renderer.set_size(f64::from(size), f64::from(size));
    pipeline.output_node = Some(soft_edge());
    let plain = frame(&mut pipeline, &mut renderer, size);
    sharpness.set(vec![30.0]);
    pipeline.output_node = Some(sharpened.node());
    let resized = frame(&mut pipeline, &mut renderer, size);
    assert_matches(&resized, &plain, size, |_| true, "sharpness 30 at 48 px");
}
