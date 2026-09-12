//! The rung harness: render an example headless, then hand the frame to
//! three.js' own `test/e2e/image.js` for the downscale and the comparison.
//!
//! Run with: `cargo test --test e2e -- --nocapture`

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../../examples/webgpu_depth_texture.rs"]
#[allow(dead_code)] // the example's own `main()` is unused here
mod webgpu_depth_texture;

#[path = "../../examples/webgpu_instance_mesh.rs"]
#[allow(dead_code)]
mod webgpu_instance_mesh;

#[path = "../../examples/webgpu_materials_basic.rs"]
#[allow(dead_code)]
mod webgpu_materials_basic;

#[path = "../../examples/webgpu_rtt.rs"]
#[allow(dead_code)]
mod webgpu_rtt;

#[path = "../../examples/webgpu_lights_phong.rs"]
#[allow(dead_code)]
mod webgpu_lights_phong;

fn three_js_dir() -> PathBuf {
    match std::env::var("THREE_JS_DIR") {
        Ok(dir) => PathBuf::from(dir),
        Err(_) => PathBuf::from(std::env::var("HOME").unwrap()).join("src/vendor/three.js"),
    }
}

fn out_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/e2e")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Comparison {
    width: u32,
    height: u32,
    num_different_pixels: u64,
    different_pixels: f64,
    max_different_pixels: f64,
    pass: bool,
}

/// Shells out to node so that `image.js`'s `scale()` and `compare()` run
/// unchanged — no second implementation of the comparator exists in this tree.
fn compare(name: &str, actual: &Path, out: &Path) -> Comparison {
    let three = three_js_dir();
    let expected = three
        .join("examples/screenshots")
        .join(format!("{name}.jpg"));

    assert!(
        expected.exists(),
        "reference screenshot missing: {}",
        expected.display()
    );

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/e2e/compare.mjs");

    let output = Command::new("node")
        .arg(&script)
        .arg(&three)
        .arg(actual)
        .arg(&expected)
        .arg(out)
        .output()
        .expect("failed to run node");

    assert!(
        output.status.success(),
        "comparator failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json = stdout.trim();
    println!("compare: {json}");

    // The payload is a flat JSON object of numbers and one bool.
    let get = |key: &str| -> f64 {
        let needle = format!("\"{key}\":");
        let start = json.find(&needle).unwrap() + needle.len();
        let rest = &json[start..];
        let end = rest.find([',', '}']).unwrap();
        rest[..end].trim().parse().unwrap()
    };

    Comparison {
        width: get("width") as u32,
        height: get("height") as u32,
        num_different_pixels: get("numDifferentPixels") as u64,
        different_pixels: get("differentPixels"),
        max_different_pixels: get("maxDifferentPixels"),
        pass: json.contains("\"pass\":true"),
    }
}

#[test]
fn webgpu_depth_texture() {
    let name = "webgpu_depth_texture";
    let out = out_dir(name);

    let mut app = webgpu_depth_texture::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_depth_texture::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
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
}

#[test]
fn webgpu_instance_mesh() {
    let name = "webgpu_instance_mesh";
    let out = out_dir(name);

    let mut app = webgpu_instance_mesh::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_instance_mesh::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
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
}

#[test]
fn webgpu_materials_basic() {
    let name = "webgpu_materials_basic";
    let out = out_dir(name);

    let mut app = webgpu_materials_basic::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_materials_basic::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
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
}

#[test]
fn webgpu_rtt() {
    let name = "webgpu_rtt";
    let out = out_dir(name);

    let mut app = webgpu_rtt::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_rtt::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
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
}

#[test]
fn webgpu_lights_phong() {
    let name = "webgpu_lights_phong";
    let out = out_dir(name);

    let mut app = webgpu_lights_phong::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_lights_phong::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
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
}
