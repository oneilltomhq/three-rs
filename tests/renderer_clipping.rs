//! A steady frame of `webgpu_clipping` and `webgpu_clipping_stencil`, held to
//! three.js' own screenshot of each page at the e2e grader's tolerance.
//!
//! A stand-in for the two pages' e2e rungs (`tests/e2e/clipping.rs`) until
//! those are wired into the ladder; it goes when they are.

use std::path::{Path, PathBuf};

#[path = "../examples/webgpu_clipping.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_clipping;

#[path = "../examples/webgpu_clipping_stencil.rs"]
#[allow(dead_code)]
mod webgpu_clipping_stencil;

fn out_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/renderer_clipping")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes the canvas, grades it against `examples/screenshots/<name>.jpg`
/// and asserts it passes.
fn grade(name: &str, renderer: &mut three_rs::Renderer) {
    let out = out_dir(name);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    let actual = out.join("actual.png");
    three_rs::testing::write_png(actual.to_str().unwrap(), width, height, &pixels);

    let expected = three_rs::testing::three_js_dir()
        .join("examples/screenshots")
        .join(format!("{name}.jpg"));
    let result = three_rs::testing::compare(&actual, &expected, &out);
    println!(
        "{name}: {:.3}% different ({} of {} pixels), limit {}%; clip-distances: {}; images: {}",
        result.different_pixels,
        result.num_different_pixels,
        result.width * result.height,
        result.max_different_pixels,
        renderer
            .device()
            .features()
            .contains(wgpu::Features::CLIP_DISTANCES),
        out.display()
    );
    assert!(
        result.pass,
        "{name}: wrong in {:.3}% of pixels ({}); see {}",
        result.different_pixels,
        result.num_different_pixels,
        out.display()
    );
}

/// Both pages in one test: they share the GPU, and libtest would otherwise
/// run them on two threads at once.
#[test]
fn clipping_pages_match_three() {
    three_rs::testing::pin_time(Some(0.0));

    let mut app = webgpu_clipping::init();
    webgpu_clipping::animate(&mut app);
    webgpu_clipping::animate(&mut app);
    grade("webgpu_clipping", &mut app.renderer);
    drop(app);

    let mut app = webgpu_clipping_stencil::init();
    webgpu_clipping_stencil::animate(&mut app);
    webgpu_clipping_stencil::animate(&mut app);
    grade("webgpu_clipping_stencil", &mut app.renderer);
}
