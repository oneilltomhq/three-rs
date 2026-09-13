//! The rung harness: render an example headless, then hand the frame to
//! three.js' own `test/e2e/image.js` for the downscale and the comparison.
//!
//! Run with: `cargo test --test e2e -- --nocapture`
//!
//! Each rung grades its first frame, exactly as three.js' harness does, and
//! then renders [`STEADY_FRAMES`] more of the same scene and times the last:
//! the performance ladder beside the pixel one (issue #57), so that a
//! per-frame cost the single graded frame cannot see fails here.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The rungs share one GPU. Run concurrently on the default test threads the
/// binary SIGSEGVs inside the Vulkan driver under device contention, so every
/// rung holds this for its whole render-and-compare. `--test-threads=1` is
/// then no longer required, only equivalent.
static GPU: Mutex<()> = Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Frames rendered after the graded one. The second frame is the first that
/// reuses every program, pipeline and upload the first built; the third and
/// fourth confirm that it stays that way. Only the last is held to the ceiling.
const STEADY_FRAMES: u32 = 3;

/// The ceiling on a steady frame, from the example's `animate()` to the GPU
/// finishing it. One number per build profile, because a debug build of the
/// examples and the renderer is five to seven times slower on the CPU and the
/// grader is normally run in debug (a plain `cargo test`).
///
/// How it was chosen, on this machine (Intel Iris Xe, Mesa 25.3.6, Fedora 43),
/// from the `--nocapture` output of this file:
///
/// - **debug**: the slowest rung is webgpu_materials_basic at ~111 ms (500
///   draws), then webgpu_depth_texture at ~63 ms; the other eight are under
///   35 ms. 600 ms is ~5x the slowest.
/// - **release**: the slowest is webgpu_materials_basic at ~16 ms (max ~27 ms
///   over 30 frames in `viewer --headless`); the other nine are under 11 ms.
///   100 ms is ~6x the slowest mean.
///
/// Both are wide enough that a busy machine, a driver hiccup or a slower GPU
/// does not fail the ladder, and both are low enough that the regression this
/// ladder exists for — 250 ms a frame in release from formatting a texture's
/// pixels into the program cache key, issue #55, several times that in debug —
/// cannot pass on any rung. Re-measure with `cargo test --test e2e --
/// --nocapture` (debug) or `cargo run --release --bin viewer -- <example>
/// --headless --frames 40` (release).
const STEADY_FRAME_CEILING: Duration = if cfg!(debug_assertions) {
    Duration::from_millis(600)
} else {
    Duration::from_millis(100)
};

/// Renders `STEADY_FRAMES` more frames through the example's own `animate()`,
/// waits for the GPU after each, prints their times, and asserts the last is
/// under [`STEADY_FRAME_CEILING`]. Called after the pixel comparison, so the
/// graded frame is untouched.
fn steady_frame<A>(name: &str, app: &mut A, animate: fn(&mut A), device: fn(&A) -> &wgpu::Device) {
    let mut times = Vec::with_capacity(STEADY_FRAMES as usize);
    for _ in 0..STEADY_FRAMES {
        let t0 = Instant::now();
        animate(app);
        device(app)
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        times.push(t0.elapsed());
    }

    let last = *times.last().unwrap();
    let listed: Vec<String> = times
        .iter()
        .map(|t| format!("{:.2}", t.as_secs_f64() * 1e3))
        .collect();
    println!(
        "{name}: steady frame {:.2} ms (frames 2..{}: {} ms; ceiling {} ms)",
        last.as_secs_f64() * 1e3,
        STEADY_FRAMES + 1,
        listed.join(", "),
        STEADY_FRAME_CEILING.as_millis()
    );

    assert!(
        last <= STEADY_FRAME_CEILING,
        "{name}: steady frame took {:.2} ms, over the {} ms ceiling — something is \
         being rebuilt, re-uploaded or re-formatted every frame",
        last.as_secs_f64() * 1e3,
        STEADY_FRAME_CEILING.as_millis()
    );
}

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

#[path = "../../examples/webgpu_postprocessing_masking.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_masking;

#[path = "../../examples/webgpu_lights_phong.rs"]
#[allow(dead_code)]
mod webgpu_lights_phong;

#[path = "../../examples/webgpu_morphtargets.rs"]
#[allow(dead_code)]
mod webgpu_morphtargets;

#[path = "../../examples/webgpu_tsl_galaxy.rs"]
#[allow(dead_code)]
mod webgpu_tsl_galaxy;
#[path = "../../examples/webgpu_shadowmap.rs"]
#[allow(dead_code)]
mod webgpu_shadowmap;
#[path = "../../examples/webgpu_lights_physical.rs"]
#[allow(dead_code)]
mod webgpu_lights_physical;

fn three_js_dir() -> PathBuf {
    three_rs::testing::three_js_dir()
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
    let _gpu = gpu();

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
    steady_frame(name, &mut app, webgpu_depth_texture::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_instance_mesh() {
    let name = "webgpu_instance_mesh";
    let out = out_dir(name);
    let _gpu = gpu();

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
    steady_frame(name, &mut app, webgpu_instance_mesh::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_materials_basic() {
    let name = "webgpu_materials_basic";
    let out = out_dir(name);
    let _gpu = gpu();

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
    steady_frame(name, &mut app, webgpu_materials_basic::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_rtt() {
    let name = "webgpu_rtt";
    let out = out_dir(name);
    let _gpu = gpu();

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
    steady_frame(name, &mut app, webgpu_rtt::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_postprocessing_masking() {
    let name = "webgpu_postprocessing_masking";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_masking::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_masking::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_postprocessing_masking::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_lights_phong() {
    let name = "webgpu_lights_phong";
    let out = out_dir(name);
    let _gpu = gpu();

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
    steady_frame(name, &mut app, webgpu_lights_phong::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_morphtargets() {
    let name = "webgpu_morphtargets";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_morphtargets::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_morphtargets::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_morphtargets::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_tsl_galaxy() {
    let name = "webgpu_tsl_galaxy";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_tsl_galaxy::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_tsl_galaxy::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_tsl_galaxy::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_shadowmap() {
    let name = "webgpu_shadowmap";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_shadowmap::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_shadowmap::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_shadowmap::animate, |app| app.renderer.device());
}

#[test]
fn webgpu_lights_physical() {
    let name = "webgpu_lights_physical";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_lights_physical::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_lights_physical::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_lights_physical::animate, |app| app.renderer.device());
}

/// Issue #56's "done when": a steady frame performs zero `NodeBuilder::build`
/// calls. Every graded rung is rendered three times; the first frame builds
/// its programs, and by the third nothing in the scene is new to the renderer
/// — the same steady-frame behaviour `WebGPURenderer` has, where
/// `RenderObjects.get()` finds every render object's cache key unchanged and
/// `NodeManager.getForRender()` never reaches the builder.
///
/// The count is `Renderer::program_builds()`, a cumulative counter that only a
/// cache miss touches. The second frame is printed but not asserted: it is
/// where a rung whose first frame renders into a target the second reads
/// (rtt, the pass nodes) would show a legitimate late build, and none does.
#[test]
fn steady_frame_builds_nothing() {
    let _gpu = gpu();

    macro_rules! rung {
        ($module:ident) => {{
            let mut app = $module::init();
            let builds: Vec<u64> = (0..3)
                .map(|_| {
                    let before = app.renderer.program_builds();
                    $module::animate(&mut app);
                    app.renderer.program_builds() - before
                })
                .collect();
            println!("{}: programs built per frame {:?}", stringify!($module), builds);
            assert!(
                builds[0] > 0,
                "{}: the first frame built nothing, so the counter is not wired",
                stringify!($module)
            );
            assert_eq!(
                builds[2],
                0,
                "{}: the third frame built {} programs; a steady frame must build none",
                stringify!($module),
                builds[2]
            );
        }};
    }

    rung!(webgpu_depth_texture);
    rung!(webgpu_instance_mesh);
    rung!(webgpu_materials_basic);
    rung!(webgpu_rtt);
    rung!(webgpu_postprocessing_masking);
    rung!(webgpu_lights_phong);
    rung!(webgpu_morphtargets);
    rung!(webgpu_tsl_galaxy);
    rung!(webgpu_shadowmap);
    rung!(webgpu_lights_physical);
}
