//! `Renderer::set_viewport` / `set_scissor` / `auto_clear` / `clear_depth`,
//! gated by reading the framebuffer back.
//!
//! **This file exists for one number: the `y`.** three.js' unified `Renderer`
//! measures the viewport and the scissor from the **top-left** — its own
//! `setViewport` documentation says "the vertical coordinate for the upper left
//! corner" — and it is the WebGL backend that converts to GL's bottom-left
//! (`WebGLBackend.updateViewport()`: `state.viewport( x, renderContext.height -
//! height - y, … )`). `WebGPUBackend` passes the rectangle straight through,
//! because WebGPU's origin is already top-left, and so is wgpu's. A port that
//! "corrects" the origin therefore puts every inset view at the wrong end of the
//! frame — a render that looks entirely plausible and is wrong by thousands of
//! pixels. The rectangles below are chosen so that the flipped answer is
//! **disjoint** from the right one: nothing here can pass by accident.
//!
//! Nothing in this file comes from an image. Every expected pixel follows from
//! the rectangle that was set.

use std::rc::Rc;

use three_rs::{
    plane_geometry, Color, Mesh, MeshBasicNodeMaterial, Node, OrthographicCamera, RenderTarget,
    Renderer, RendererParameters, Scene,
};

const W: u32 = 64;
const H: u32 = 64;

/// The inset rectangle, in the renderer's (top-left) coordinates. Deliberately
/// off-centre in both axes and taller than it is wide, so that a flipped `y`
/// (`H - y - height` = `64 - 4 - 20` = 40) shares no row with it.
const VX: f64 = 8.0;
const VY: f64 = 4.0;
const VW: f64 = 16.0;
const VH: f64 = 20.0;

/// Where a flipped `y` would put the same rectangle.
const FLIPPED_VY: f64 = H as f64 - VY - VH;

/// `new OrthographicCamera( -1, 1, 1, -1, 0.1, 100 )` at `z = 10`: a 2x2 plane
/// at the origin covers the whole viewport, whatever the viewport is.
fn camera() -> OrthographicCamera {
    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 100.0);
    camera.object.position.z = 10.0;
    camera.update_matrix_world();
    camera
}

/// A 2x2 plane of one flat colour at `z`.
fn quad(color: u32, z: f64) -> Node {
    let mesh = Mesh::new(
        Rc::new(plane_geometry(2.0, 2.0, 1, 1)),
        MeshBasicNodeMaterial::line(Color::from_hex(color)),
    );
    mesh.borrow_mut().position.z = z;
    mesh
}

fn renderer() -> Renderer {
    let mut renderer =
        Renderer::new(RendererParameters { antialias: false }).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);
    renderer
}

/// The RGB of framebuffer pixel `( column, row )`, row 0 at the top — the same
/// orientation `page.screenshot()` hands the grader.
fn rgb(pixels: &[u8], column: u32, row: u32) -> [u8; 3] {
    let at = ((row * W + column) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

#[track_caller]
fn assert_rgb(pixels: &[u8], column: u32, row: u32, want: [u8; 3], what: &str) {
    assert_eq!(
        rgb(pixels, column, row),
        want,
        "{what}: pixel ( {column}, {row} )"
    );
}

/// Every pixel of a `VW` x `VH` box at `( x, y )`, and every pixel outside it,
/// against two colours.
#[track_caller]
fn assert_box(pixels: &[u8], x: f64, y: f64, inside: [u8; 3], outside: [u8; 3], what: &str) {
    let (x, y) = (x as u32, y as u32);
    let (w, h) = (VW as u32, VH as u32);

    for row in 0..H {
        for column in 0..W {
            let within = column >= x && column < x + w && row >= y && row < y + h;
            let want = if within { inside } else { outside };
            assert_rgb(pixels, column, row, want, what);
        }
    }
}

/// The whole point of the file: the box lands where the *renderer's* `y` says,
/// and not where a bottom-left origin would put it.
#[test]
fn viewport_and_scissor_measure_from_the_top_left() {
    let mut renderer = renderer();

    // Frame one, full-frame: clears the whole target black and blits the whole
    // target, so every pixel below is a pixel this test wrote.
    let mut black = Scene::new();
    black.set_background(Color::from_hex(0x000000));
    renderer.render(&mut black, &mut camera());

    // Frame two, the inset. `autoClear` off and no background, exactly as the
    // second `render()` of a picture-in-picture frame has it — a `Color`
    // background would force the clear back on (`Background.update()`'s
    // `forceClear`).
    let mut inset = Scene::new();
    inset.add(&quad(0xff0000, 0.0));

    renderer.auto_clear = false;
    renderer.set_scissor_test(true);
    renderer.set_scissor(VX, VY, VW, VH);
    renderer.set_viewport(VX, VY, VW, VH);
    renderer.render(&mut inset, &mut camera());

    let (w, h, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
    assert_eq!((w, h), (W, H));

    assert_box(
        &pixels,
        VX,
        VY,
        [255, 0, 0],
        [0, 0, 0],
        "the inset is at the rectangle that was set",
    );

    // Stated the other way round, so the failure message names the bug: the
    // flipped rectangle must be entirely background.
    for row in FLIPPED_VY as u32..(FLIPPED_VY + VH) as u32 {
        for column in VX as u32..(VX + VW) as u32 {
            assert_rgb(
                &pixels,
                column,
                row,
                [0, 0, 0],
                "the viewport y is NOT flipped into wgpu's coordinates",
            );
        }
    }
}

/// Without the scissor the *viewport* alone still confines the draw: a
/// full-screen quad is squeezed into the rectangle rather than clipped to it.
/// This is what separates the two settings — a port that implemented only the
/// scissor would pass the test above and fail this one, because the quad would
/// cover the frame.
#[test]
fn the_viewport_confines_a_draw_without_the_scissor() {
    let mut renderer = renderer();

    let mut black = Scene::new();
    black.set_background(Color::from_hex(0x000000));
    renderer.render(&mut black, &mut camera());

    let mut inset = Scene::new();
    inset.add(&quad(0x00ff00, 0.0));

    renderer.auto_clear = false;
    renderer.set_viewport(VX, VY, VW, VH);
    renderer.render(&mut inset, &mut camera());

    let (_, _, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
    assert_box(
        &pixels,
        VX,
        VY,
        [0, 255, 0],
        [0, 0, 0],
        "the viewport alone squeezes the quad into the rectangle",
    );
}

/// A render target carries its own viewport, which is the seam a target tiled
/// into several views (a PMREM chain, a shadow atlas) renders through: no
/// renderer state, no pixel ratio.
#[test]
fn a_render_target_renders_through_its_own_viewport() {
    let mut renderer = renderer();
    let target = RenderTarget::new(W, H);

    let mut black = Scene::new();
    black.set_background(Color::from_hex(0x000000));
    renderer.set_render_target(Some(target.clone()));
    renderer.render(&mut black, &mut camera());

    let mut inset = Scene::new();
    inset.add(&quad(0x0000ff, 0.0));

    // The renderer's own viewport is left at the full frame on purpose: a
    // render target must not read it.
    renderer.auto_clear = false;
    target.set_viewport(VX, VY, VW, VH);
    renderer.render(&mut inset, &mut camera());
    renderer.set_render_target(None);

    let (_, _, pixels) = renderer
        .read_target_pixels(&target)
        .expect("render target readback");
    assert_box(
        &pixels,
        VX,
        VY,
        [0, 0, 255],
        [0, 0, 0],
        "the render target's own viewport",
    );
}

/// `clear_depth()` between two views. Without it the second view's further
/// quad loses the depth test against the first and never appears — the silent
/// failure the fat-lines example's inset would show as a missing line.
#[test]
fn clear_depth_lets_a_further_quad_through() {
    let mut renderer = renderer();

    let mut near = Scene::new();
    near.set_background(Color::from_hex(0x000000));
    near.add(&quad(0xffffff, 0.0));
    renderer.render(&mut near, &mut camera());

    // A quad five units further from the camera, composited over the first.
    let mut far = Scene::new();
    far.add(&quad(0xff0000, -5.0));

    renderer.auto_clear = false;
    renderer.render(&mut far, &mut camera());

    let (_, _, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
    assert_rgb(
        &pixels,
        W / 2,
        H / 2,
        [255, 255, 255],
        "the further quad is depth-tested away while the depth buffer stands",
    );

    renderer.clear_depth();
    renderer.render(&mut far, &mut camera());

    let (_, _, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
    assert_rgb(
        &pixels,
        W / 2,
        H / 2,
        [255, 0, 0],
        "clear_depth() lets the further quad draw",
    );
}

/// `auto_clear = false` composites instead of clearing; with it on the second
/// render starts from the clear colour again. The pair pins the flag itself:
/// a renderer that ignored it would give the same answer twice.
#[test]
fn auto_clear_decides_whether_the_frame_is_kept() {
    for (auto_clear, want) in [(false, [255, 255, 255]), (true, [0, 0, 0])] {
        let mut renderer = renderer();

        let mut white = Scene::new();
        white.set_background(Color::from_hex(0x000000));
        white.add(&quad(0xffffff, 0.0));
        renderer.render(&mut white, &mut camera());

        // An empty scene with no background: with `auto_clear` on it clears to
        // the renderer's clear colour, with it off it leaves frame one alone.
        let mut empty = Scene::new();
        renderer.auto_clear = auto_clear;
        renderer.render(&mut empty, &mut camera());

        let (_, _, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
        assert_rgb(
            &pixels,
            W / 2,
            H / 2,
            want,
            &format!("auto_clear = {auto_clear}"),
        );
    }
}
