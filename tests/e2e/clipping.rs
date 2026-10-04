//! The clipping rungs: `webgpu_clipping` (two nested `ClippingGroup`s, union
//! and intersection planes, alpha-to-coverage edges) and
//! `webgpu_clipping_stencil` (stencil-buffer caps on a three-plane cut).
//!
//! `main.rs` declares the two example modules beside every other rung's, so
//! `rung!` in `steady_frame_builds_nothing` can name them as well.

use super::{compare, gpu, out_dir, steady_frame, webgpu_clipping, webgpu_clipping_stencil};

#[test]
fn webgpu_clipping() {
    let name = "webgpu_clipping";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_clipping::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_clipping::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    let actual = out.join("actual.png");
    three_rs::testing::write_png(actual.to_str().unwrap(), width, height, &pixels);

    let result = compare(name, &actual, &out);

    println!(
        "{name}: {:.1}% different ({} of {} pixels, {}x{}), limit {}%",
        result.different_pixels,
        result.num_different_pixels,
        result.width * result.height,
        result.width,
        result.height,
        result.max_different_pixels
    );
    println!("images: {}", out.display());

    assert!(
        result.pass,
        "diff wrong in {:.1}% of pixels ({} pixels); see {}",
        result.different_pixels,
        result.num_different_pixels,
        out.display()
    );
    steady_frame(name, &mut app, webgpu_clipping::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_clipping_stencil() {
    let name = "webgpu_clipping_stencil";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_clipping_stencil::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_clipping_stencil::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    let actual = out.join("actual.png");
    three_rs::testing::write_png(actual.to_str().unwrap(), width, height, &pixels);

    let result = compare(name, &actual, &out);

    println!(
        "{name}: {:.1}% different ({} of {} pixels, {}x{}), limit {}%",
        result.different_pixels,
        result.num_different_pixels,
        result.width * result.height,
        result.width,
        result.height,
        result.max_different_pixels
    );
    println!("images: {}", out.display());

    assert!(
        result.pass,
        "diff wrong in {:.1}% of pixels ({} pixels); see {}",
        result.different_pixels,
        result.num_different_pixels,
        out.display()
    );
    steady_frame(name, &mut app, webgpu_clipping_stencil::animate, |app| {
        app.renderer.device()
    });
}
