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

#[path = "../../examples/webgpu_lights_physical.rs"]
#[allow(dead_code)]
mod webgpu_lights_physical;
#[path = "../../examples/webgpu_shadowmap.rs"]
#[allow(dead_code)]
mod webgpu_shadowmap;
#[path = "../../examples/webgpu_tsl_galaxy.rs"]
#[allow(dead_code)]
mod webgpu_tsl_galaxy;

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

use three_rs::testing::Comparison;

/// Three's reference screenshot for `name`, through the comparator in
/// `three_rs::testing`.
fn compare(name: &str, actual: &Path, out: &Path) -> Comparison {
    let expected = three_js_dir()
        .join("examples/screenshots")
        .join(format!("{name}.jpg"));
    let result = three_rs::testing::compare(actual, &expected, out);
    println!("compare: {result:?}");
    result
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
    steady_frame(name, &mut app, webgpu_depth_texture::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_instance_mesh::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_materials_basic::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_rtt::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_masking::animate,
        |app| app.renderer.device(),
    );
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
    steady_frame(name, &mut app, webgpu_lights_phong::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_morphtargets::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_tsl_galaxy::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_shadowmap::animate, |app| {
        app.renderer.device()
    });
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
    steady_frame(name, &mut app, webgpu_lights_physical::animate, |app| {
        app.renderer.device()
    });
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
            println!(
                "{}: programs built per frame {:?}",
                stringify!($module),
                builds
            );
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

// ---------------------------------------------------------------------------
// issue #58: cache identity and eviction
// ---------------------------------------------------------------------------

/// Issue #58's repro, and the two properties that between them make it
/// impossible rather than unlikely.
///
/// The bug: the geometry cache was keyed by `Rc::as_ptr( &geometry )` and never
/// evicted, so a geometry allocated at a dropped one's address was served the
/// dead one's GPU buffers — a panic ("the geometry has no uv attribute") when
/// the attribute sets differed, the wrong shape drawn silently when they did
/// not. The issue's repro won that race on allocation attempt 18.
///
/// Reproducing the *collision* on this branch is not possible, which is the
/// point: a cached entry holds a `Weak` on the geometry, and a `Weak` keeps the
/// block reserved, so for as long as a stale entry could be found its address
/// cannot be handed out — and by the time the sweep releases it, the entry it
/// would have collided with is gone. The two halves are asserted separately:
///
/// 1. **the address is not handed out** while the renderer still holds the
///    entry. This is the assertion that fails on `main`, within a couple of
///    dozen allocations;
/// 2. **the id is not handed out either**, ever — so even a collision the
///    allocator manufactured some other way could not find the wrong entry.
///
/// Then the repro's own scenario runs end to end: drop the line, render, draw a
/// textured plane, and check that it drew.
#[test]
fn a_dropped_geometry_does_not_lend_its_buffers_to_the_next_one() {
    use std::rc::Rc;
    use three_rs::core::BufferGeometry;
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::nodes::tsl::texture;
    use three_rs::{
        plane_geometry, Color, LineSegments, Mesh, PerspectiveCamera, Renderer, RendererParameters,
        Scene, Texture, Vector3,
    };

    /// Allocations the test is willing to make hunting for the freed address.
    /// The issue's repro found it on attempt 18.
    const ABA_TRIES: usize = 1000;

    let _gpu = gpu();

    // -- 2. ids are never reused, even when addresses are ------------------
    //
    // No renderer involved: drop an `Rc<BufferGeometry>` and allocate another
    // and the allocator hands the block straight back, which is the whole
    // premise of the bug. The id must not come back with it.
    let (freed, id_before) = {
        let g = Rc::new(BufferGeometry::new());
        (Rc::as_ptr(&g) as usize, g.id())
    };
    let again = Rc::new(BufferGeometry::new());
    if Rc::as_ptr(&again) as usize == freed {
        assert_ne!(
            again.id(),
            id_before,
            "two geometries at the same address must not share a cache key"
        );
    } else {
        println!(
            "a_dropped_geometry_does_not_lend_its_buffers_to_the_next_one: the \
             allocator did not hand the freed block straight back, so the id-reuse \
             half of this test did not run"
        );
    }
    drop(again);

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 3.0;

    // Frame 1: a line, position only. (The renderer holds a geometry or two of
    // its own — the full-screen quad, the background skybox — so the cache
    // counts here are relative, never absolute.)
    let line_ptr = {
        let mut g = BufferGeometry::new();
        g.set_from_points(&[Vector3::new(-1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)]);
        let geometry = Rc::new(g);
        let ptr = Rc::as_ptr(&geometry) as usize;
        let line = LineSegments::new(
            geometry,
            MeshBasicNodeMaterial::line(Color::from_hex(0xffffff)),
        );
        let mut scene = Scene::new();
        scene.add(&line);
        renderer.render(&mut scene, &mut camera);
        ptr
    };
    let with_line = renderer.geometry_cache_len();

    // -- 1. the freed address stays reserved while the entry lives ---------
    //
    // This is the repro's loop, and on `main` it finds the address: the line's
    // block is free the moment the line is dropped, and the renderer's entry
    // for it is still there to be found. Here the entry's `Weak` holds the
    // block, so no geometry can land on it.
    let mut probes = Vec::with_capacity(ABA_TRIES);
    for attempt in 1..=ABA_TRIES {
        let p = Rc::new(plane_geometry(1.0, 1.0, 1, 1));
        assert_ne!(
            Rc::as_ptr(&p) as usize,
            line_ptr,
            "allocation {attempt} landed on the dropped line geometry's address \
             {line_ptr:#x} while the renderer still holds its GPU buffers"
        );
        probes.push(p);
    }
    drop(probes);

    // Frame 2: nothing. The sweep drops the line's entry, and with it the
    // `Weak` that was reserving the block.
    let mut empty = Scene::new();
    renderer.render(&mut empty, &mut camera);
    assert_eq!(
        renderer.geometry_cache_len(),
        with_line - 1,
        "the dropped line geometry should have been swept out of the cache"
    );

    // Frame 3: the repro's textured plane, which has `uv` where the line had
    // none. On `main` this is where it panicked.
    let tex = Texture::new(2, 2, Some(vec![255u8; 16]));
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(texture(&tex));
    let mesh = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)));
    mesh.borrow_mut().mesh_mut().unwrap().material = Some(material);
    let mut scene = Scene::new();
    scene.add(&mesh);
    renderer.render(&mut scene, &mut camera);

    // and it drew: the plane is white against a cleared canvas.
    let (w, h, pixels) = renderer.read_canvas_pixels();
    let centre = ((h as usize / 2) * w as usize + w as usize / 2) * 4;
    assert_eq!(
        &pixels[centre..centre + 4],
        &[255, 255, 255, 255],
        "the textured plane should have drawn white at the centre of the frame"
    );
}

/// Nothing the consumer has dropped stays in the renderer's caches.
///
/// Fifty frames, each with a geometry and a material created for that frame and
/// dropped at the end of it — a layout that rebuilds, a per-frame tween, text
/// re-laid-out. Every one of the three maps must hold steady: the geometry
/// cache exactly, since an `Rc`'s strong count is exact, and the two by-use
/// caches within their grace window (`CACHE_GRACE_RENDERS`), which is why those
/// two are compared against a bound rather than against one.
#[test]
fn churning_geometry_and_materials_does_not_grow_the_caches() {
    use std::rc::Rc;
    use three_rs::materials::{instanced_range, MeshBasicNodeMaterial};
    use three_rs::{
        box_geometry, Color, InstancedMesh, PerspectiveCamera, Renderer, RendererParameters, Scene,
    };

    const FRAMES: usize = 50;
    /// A frame's entries, times the grace window, plus room for the renderer's
    /// own materials (the default, background and output ones). The point is
    /// that it does not climb with `FRAMES`.
    const BOUND: usize = 12;

    let _gpu = gpu();

    let mut renderer = Renderer::new(RendererParameters { antialias: false });
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let mut sizes = Vec::new();
    for frame in 0..FRAMES {
        // A new geometry and a new material every frame, both dropped with the
        // scene at the end of it. `instanced_range` is what puts an entry in
        // the third cache.
        let size = 1.0 + frame as f64 * 0.01;
        let geometry = Rc::new(box_geometry(size, size, size, 1, 1, 1));
        let mut material = MeshBasicNodeMaterial::new();
        material.color_node =
            Some(instanced_range(Color::from_hex(0x000000), Color::from_hex(0xffffff), 8).xyz());
        let mesh = InstancedMesh::new(geometry, material, 8);
        let mut scene = Scene::new();
        scene.add(&mesh);
        renderer.render(&mut scene, &mut camera);

        sizes.push((
            renderer.geometry_cache_len(),
            renderer.material_cache_len(),
            renderer.buffer_cache_len(),
        ));
    }

    println!(
        "churn: cache sizes (geometry, material, buffer) frame 1 {:?}, frame 10 {:?}, \
         frame {FRAMES} {:?}",
        sizes[0],
        sizes[9],
        sizes[FRAMES - 1]
    );

    let (geometries, materials, buffers) = sizes[FRAMES - 1];
    assert_eq!(
        geometries, sizes[0].0,
        "the frame's own geometry plus the renderer's should be all that is uploaded          after {FRAMES} frames, not {geometries}"
    );
    assert!(
        materials <= BOUND && buffers <= BOUND,
        "the by-use caches grew over {FRAMES} frames of churn: {materials} materials, \
         {buffers} buffers, both expected to settle under {BOUND}"
    );
    assert!(
        sizes[FRAMES - 1] <= sizes[9],
        "the caches were still growing at frame {FRAMES}: {:?} at frame 10, {:?} at the end",
        sizes[9],
        sizes[FRAMES - 1]
    );
}
