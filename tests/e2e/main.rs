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
    // Pin both clocks to zero for the rung, the way three.js' own harness
    // does: `test/e2e/deterministic-injection.js` assigns `Date.now = () => 0`
    // and `performance.now = () => 0` into the page before the example runs,
    // so that an example animating on a clock still grades the same frame
    // every time. The override is thread-local (`three_rs::testing::pin_time`)
    // and every rung takes this guard, so pinning here pins it on whichever
    // test thread the rung landed on, once, before any example code runs.
    let guard = GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    three_rs::testing::pin_time(Some(0.0));
    guard
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

#[path = "../../examples/webgpu_instance_uniform.rs"]
#[allow(dead_code)]
mod webgpu_instance_uniform;

#[path = "../../examples/webgpu_materials.rs"]
#[allow(dead_code)]
mod webgpu_materials;

#[path = "../../examples/webgpu_materials_basic.rs"]
#[allow(dead_code)]
mod webgpu_materials_basic;

#[path = "../../examples/webgpu_materials_cubemap_mipmaps.rs"]
#[allow(dead_code)]
mod webgpu_materials_cubemap_mipmaps;

#[path = "../../examples/webgpu_materials_envmaps.rs"]
#[allow(dead_code)]
mod webgpu_materials_envmaps;

#[path = "../../examples/webgpu_rtt.rs"]
#[allow(dead_code)]
mod webgpu_rtt;

#[path = "../../examples/webgpu_postprocessing_masking.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_masking;

#[path = "../../examples/webgpu_postprocessing_difference.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_difference;

#[path = "../../examples/webgpu_postprocessing_direct.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_direct;

#[path = "../../examples/webgpu_postprocessing_radial_blur.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_radial_blur;

#[path = "../../examples/webgpu_postprocessing_ssaa.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_ssaa;

#[path = "../../examples/webgpu_postprocessing_ca.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_ca;

#[path = "../../examples/webgpu_postprocessing_transition.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_transition;

#[path = "../../examples/webgpu_postprocessing_sobel.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_sobel;

#[path = "../../examples/webgpu_procedural_texture.rs"]
#[allow(dead_code)]
mod webgpu_procedural_texture;

#[path = "../../examples/webgpu_postprocessing_anamorphic.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_anamorphic;
#[path = "../../examples/webgpu_postprocessing_bloom.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_bloom;

#[path = "../../examples/webgpu_postprocessing_bloom_selective.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_bloom_selective;

#[path = "../../examples/webgpu_postprocessing_bloom_emissive.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_bloom_emissive;

#[path = "../../examples/webgpu_pmrem_cubemap.rs"]
#[allow(dead_code)]
mod webgpu_pmrem_cubemap;

#[path = "../../examples/webgpu_furnace_test.rs"]
#[allow(dead_code)]
mod webgpu_furnace_test;

#[path = "../../examples/webgpu_pmrem_test.rs"]
#[allow(dead_code)]
mod webgpu_pmrem_test;

#[path = "../../examples/webgpu_pmrem_scene.rs"]
#[allow(dead_code)]
mod webgpu_pmrem_scene;

#[path = "../../examples/webgpu_pmrem_equirectangular.rs"]
#[allow(dead_code)]
mod webgpu_pmrem_equirectangular;

#[path = "../../examples/webgpu_mrt.rs"]
#[allow(dead_code)]
mod webgpu_mrt;

#[path = "../../examples/webgpu_loader_gltf.rs"]
#[allow(dead_code)]
mod webgpu_loader_gltf;

#[path = "../../examples/webgpu_loader_gltf_sheen.rs"]
#[allow(dead_code)]
mod webgpu_loader_gltf_sheen;

#[path = "../../examples/webgpu_custom_fog_background.rs"]
#[allow(dead_code)]
mod webgpu_custom_fog_background;

#[path = "../../examples/webgpu_deferred.rs"]
#[allow(dead_code)]
mod webgpu_deferred;
#[path = "../../examples/webgpu_loader_gltf_anisotropy.rs"]
#[allow(dead_code)]
mod webgpu_loader_gltf_anisotropy;
#[path = "../../examples/webgpu_materials_texture_manualmipmap.rs"]
#[allow(dead_code)]
mod webgpu_materials_texture_manualmipmap;

#[path = "../../examples/webgpu_tsl_angular_slicing.rs"]
#[allow(dead_code)]
mod webgpu_tsl_angular_slicing;

#[path = "../../examples/webgpu_textures_2d-array_compressed.rs"]
#[allow(dead_code)]
mod webgpu_textures_2d_array_compressed;

#[path = "../../examples/webgpu_lights_phong.rs"]
#[allow(dead_code)]
mod webgpu_lights_phong;

#[path = "../../examples/webgpu_morphtargets.rs"]
#[allow(dead_code)]
mod webgpu_morphtargets;

#[path = "../../examples/webgpu_compute_points.rs"]
#[allow(dead_code)]
mod webgpu_compute_points;
#[path = "../../examples/webgpu_compute_texture.rs"]
#[allow(dead_code)]
mod webgpu_compute_texture;
#[path = "../../examples/webgpu_lights_physical.rs"]
#[allow(dead_code)]
mod webgpu_lights_physical;
#[path = "../../examples/webgpu_lines_fat.rs"]
#[allow(dead_code)]
mod webgpu_lines_fat;
#[path = "../../examples/webgpu_lines_fat_raycasting.rs"]
#[allow(dead_code)]
mod webgpu_lines_fat_raycasting;
#[path = "../../examples/webgpu_mesh_batch.rs"]
#[allow(dead_code)]
mod webgpu_mesh_batch;
#[path = "../../examples/webgpu_shadowmap.rs"]
#[allow(dead_code)]
mod webgpu_shadowmap;
#[path = "../../examples/webgpu_skinning.rs"]
#[allow(dead_code)]
mod webgpu_skinning;
#[path = "../../examples/webgpu_tsl_galaxy.rs"]
#[allow(dead_code)]
mod webgpu_tsl_galaxy;
#[path = "../../examples/webgpu_tsl_interoperability.rs"]
#[allow(dead_code)]
mod webgpu_tsl_interoperability;
#[path = "../../examples/webgpu_tsl_vfx_flames.rs"]
#[allow(dead_code)]
mod webgpu_tsl_vfx_flames;

#[path = "../../examples/webgpu_tsl_raging_sea.rs"]
#[allow(dead_code)]
mod webgpu_tsl_raging_sea;

#[path = "../../examples/webgpu_volume_perlin.rs"]
#[allow(dead_code)]
mod webgpu_volume_perlin;

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
    steady_frame(name, &mut app, webgpu_instance_mesh::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_instance_uniform() {
    let name = "webgpu_instance_uniform";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_instance_uniform::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_instance_uniform::animate(&mut app);

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

    // The point of the page: twelve meshes drawing from one node graph, with
    // the per-object `vec3` uniform the only difference between the draws.
    // Three programs and three pipelines — the grid, the teapot material and
    // the output pass — and twelve of the fourteen `NodeBuilder::build` runs
    // produce WGSL the cache already has.
    //
    // Three's page hands the *same* `Material` object to all twelve meshes and
    // so builds twice; a `MeshBasicNodeMaterial` is a value here and
    // `Material.clone()` gets a fresh `MaterialId` (`src/materials/mod.rs`), so
    // the port builds once per mesh and deduplicates on the generated WGSL. The
    // GPU sees the same three modules and three pipelines either way; the
    // difference is twelve first-frame builds, and nothing on a steady frame.
    let info = app.renderer.info();
    println!("{name}: info {info:?}");
    assert_eq!(
        (
            info.build.programs_compiled,
            info.build.pipelines_built,
            info.memory.programs
        ),
        (14, 3, 3),
        "twelve instance-uniform teapots must share one program and one pipeline"
    );

    steady_frame(name, &mut app, webgpu_instance_uniform::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_materials() {
    let name = "webgpu_materials";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_materials::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_materials::animate(&mut app);

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
    // Materials 28 and 29 of the page are the same expression twice: once
    // through an `Fn()` with a named input, once with the texture captured.
    // Three generates byte-identical WGSL for the two and they share one
    // program; so must the port. Nineteen programs are built — the grid, the
    // seventeen teapots and the output pass — and eighteen survive
    // deduplication.
    let info = app.renderer.info();
    println!("{name}: info {info:?}");
    assert_eq!(
        (info.build.programs_compiled, info.memory.programs),
        (19, 18),
        "the two desaturate materials must share one program"
    );

    steady_frame(name, &mut app, webgpu_materials::animate, |app| {
        app.renderer.device()
    });
}

/// `webgpu_materials_cubemap_mipmaps`: two spheres over the same nine cube
/// levels, one with the mips loaded from files and one with them generated by
/// the backend. The frame is the comparison, so a bug in either upload path
/// shows up as one sphere disagreeing with the other.
#[test]
fn webgpu_materials_cubemap_mipmaps() {
    let name = "webgpu_materials_cubemap_mipmaps";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_materials_cubemap_mipmaps::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_materials_cubemap_mipmaps::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_materials_cubemap_mipmaps::animate,
        |app| app.renderer.device(),
    );
}

/// `webgpu_materials_envmaps`: the cube-reflection default of the environment
/// mapping page — a `CubeTexture` background and one `MeshBasicMaterial`
/// sphere reflecting it. Both the GUI's `Type` and its `Refraction` toggle
/// are at their defaults for the first frame, so nothing equirectangular and
/// nothing refractive is in this image.
#[test]
fn webgpu_materials_envmaps() {
    let name = "webgpu_materials_envmaps";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_materials_envmaps::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_materials_envmaps::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_materials_envmaps::animate, |app| {
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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_masking::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_difference() {
    let name = "webgpu_postprocessing_difference";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_difference::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_difference::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_difference::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_direct() {
    let name = "webgpu_postprocessing_direct";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_direct::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_direct::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_direct::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_radial_blur() {
    let name = "webgpu_postprocessing_radial_blur";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_radial_blur::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_radial_blur::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_radial_blur::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_ssaa() {
    let name = "webgpu_postprocessing_ssaa";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_ssaa::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_ssaa::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_postprocessing_ssaa::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_postprocessing_ca() {
    let name = "webgpu_postprocessing_ca";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_ca::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_ca::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_postprocessing_ca::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_postprocessing_transition() {
    let name = "webgpu_postprocessing_transition";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_transition::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_transition::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_transition::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_sobel() {
    let name = "webgpu_postprocessing_sobel";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_sobel::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_sobel::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_sobel::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_procedural_texture() {
    let name = "webgpu_procedural_texture";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_procedural_texture::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_procedural_texture::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_procedural_texture::animate, |app| {
        app.renderer.device()
    });
}

/// The fifty spheres, against
/// `tests/fixtures/webgpu_postprocessing_bloom_selective/spheres_t0.json` —
/// three.js' own `Color` / `Vector3` run in node under the e2e harness's
/// seeded `Math.random` (the fixture's `oracle.mjs`).
///
/// Nine draws per sphere, and the third of each nine is the coin flip that
/// decides whether that sphere blooms. An off-by-one in the sequence would not
/// nudge the image, it would glow a different set of spheres — so this is
/// asserted before a single pixel is compared, and its failure message names
/// the sphere rather than a percentage.
fn assert_spheres(scene: &three_rs::Scene) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/webgpu_postprocessing_bloom_selective/spheres_t0.json");
    let oracle: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("the oracle fixture is readable"),
    )
    .expect("the oracle fixture is JSON");
    let expected = oracle["spheres"].as_array().expect("an array of spheres");

    let children = scene.children();
    assert_eq!(children.len(), expected.len(), "sphere count");

    for (i, (child, want)) in children.iter().zip(expected).enumerate() {
        let object = child.borrow();

        let position = [object.position.x, object.position.y, object.position.z];
        let want_position: Vec<f64> = want["position"]
            .as_array()
            .expect("position is an array")
            .iter()
            .map(|v| v.as_f64().expect("a number"))
            .collect();
        for (axis, (a, e)) in position.iter().zip(&want_position).enumerate() {
            // The same 1e-9 every rung's oracle uses: Rust's `f64::sin` and
            // V8's differ by up to an ulp, which the `* 10000` in the harness
            // PRNG turns into ~2e-12 of the fraction.
            assert!(
                (a - e).abs() <= 1e-9,
                "sphere {i} position[{axis}]: {a} vs {e}"
            );
        }

        let scale = want["scale"].as_f64().expect("a number");
        assert!(
            (object.scale.x - scale).abs() <= 1e-9,
            "sphere {i} scale: {} vs {scale}",
            object.scale.x
        );

        let material = object.material().expect("every sphere has a material");
        let color = material
            .color
            .get_hex(three_rs::math::ColorSpace::LinearSRGB);
        let want_color = want["color"].as_u64().expect("a number") as u32;
        assert_eq!(color, want_color, "sphere {i} color");

        // `material.mrtNode.get( 'bloomIntensity' ).value` — the uniform the
        // page's pointer handler would toggle, read back out of the graph.
        let node = material
            .mrt_node
            .as_ref()
            .expect("every sphere has an mrtNode")
            .get("bloomIntensity")
            .expect("the mrtNode has a bloomIntensity channel");
        let value = match &*node.0 {
            three_rs::nodes::Node::Uniform(uniform) => match &uniform.source {
                three_rs::nodes::UniformSource::Value(values) => values[0],
                other => panic!("sphere {i} bloomIntensity is {other:?}, not a value uniform"),
            },
            other => panic!("sphere {i} bloomIntensity is {other:?}, not a uniform"),
        };
        assert_eq!(
            value,
            want["bloomIntensity"].as_f64().expect("a number"),
            "sphere {i} bloomIntensity"
        );
    }
}

#[test]
fn webgpu_postprocessing_bloom_selective() {
    let name = "webgpu_postprocessing_bloom_selective";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_bloom_selective::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    // Before the GPU is asked for anything: the scene itself.
    assert_spheres(&app.scene);

    webgpu_postprocessing_bloom_selective::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_bloom_selective::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_anamorphic() {
    let name = "webgpu_postprocessing_anamorphic";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_anamorphic::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_anamorphic::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_anamorphic::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_bloom() {
    let name = "webgpu_postprocessing_bloom";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_bloom::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    // Before the GPU is asked for anything: the scene the loader built. The
    // numeric gate against three's own `GLTFLoader` parse is
    // `tests/gltf_primary_ion_drive.rs`; this is the one fact the *example*
    // owns, that the glTF scene is under the scene graph with its 20 nodes.
    assert_eq!(
        app.gltf_scene.borrow().children.len(),
        1,
        "the glTF scene's single child"
    );

    webgpu_postprocessing_bloom::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_bloom::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_postprocessing_bloom_emissive() {
    let name = "webgpu_postprocessing_bloom_emissive";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_postprocessing_bloom_emissive::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_postprocessing_bloom_emissive::animate(&mut app);

    // `emissiveTexture.type = UnsignedByteType`: attachment 1 is LDR where
    // attachment 0 is `rgba16float`, which is the whole reason the pipeline
    // carries a colour target per attachment.
    assert_eq!(
        app.scene_pass.texture_named("emissive").format(),
        wgpu::TextureFormat::Rgba8Unorm,
        "the emissive attachment is UnsignedByteType"
    );
    assert_eq!(
        app.scene_pass.texture().format(),
        wgpu::TextureFormat::Rgba16Float,
        "the colour attachment is HalfFloatType"
    );

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
    steady_frame(
        name,
        &mut app,
        webgpu_postprocessing_bloom_emissive::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_lines_fat() {
    let name = "webgpu_lines_fat";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_lines_fat::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_lines_fat::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_lines_fat::animate, |app| {
        app.renderer.device()
    });
}

/// The world-units fat line: `Line2NodeMaterial`'s `worldUnits` branch with
/// `alphaToCoverage` on a four-sample target, and a raycast from a pointer at
/// infinity that must hit nothing (the marker spheres stay hidden).
#[test]
fn webgpu_lines_fat_raycasting() {
    let name = "webgpu_lines_fat_raycasting";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_lines_fat_raycasting::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_lines_fat_raycasting::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));
    assert!(
        !app.sphere_inter.borrow().visible && !app.sphere_on_line.borrow().visible,
        "a ray from ( Infinity, Infinity ) is NaN and hits nothing"
    );

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
        webgpu_lines_fat_raycasting::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_pmrem_cubemap() {
    let name = "webgpu_pmrem_cubemap";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_pmrem_cubemap::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_pmrem_cubemap::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_pmrem_cubemap::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_pmrem_test() {
    let name = "webgpu_pmrem_test";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_pmrem_test::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_pmrem_test::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_pmrem_test::animate, |app| {
        app.renderer.device()
    });
}

/// `webgpu_pmrem_equirectangular`: the same PMREM machinery as
/// One UltraHDR equirectangular map as both `scene.background` (a 512² cube)
/// and `scene.environment` (a PMREM), with DamagedHelmet and its five maps in
/// front of it.
///
/// This is the gate on [`Scene::environment`](three_rs::Scene::environment):
/// the helmet's material carries no `envMap`, so every reflection in the frame
/// comes from the scene-level fallback that `NodeMaterial.setupEnvironment()`
/// reaches for. It is also the gate on `fitCameraToSelection` — the camera is
/// derived from the loaded model's bounding box, so a wrong `Box3` moves every
/// pixel.
#[test]
fn webgpu_loader_gltf() {
    let name = "webgpu_loader_gltf";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_loader_gltf::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_loader_gltf::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_loader_gltf::animate, |app| {
        app.renderer.device()
    });
}

/// `KHR_materials_sheen`: the sheen half of `PhysicalLightingModel`, lit by
/// nothing but the environment.
///
/// SheenChair's fabric is the ladder's first material with `sheen > 0`, so
/// this is the gate on `BRDF_Sheen`'s indirect half — `IBLSheenBRDF`'s
/// analytic fit applied to both `irradiance` and `iblIrradiance`, and the
/// energy compensation that takes what the sheen lobe reflected away from the
/// diffuse and specular terms underneath it. The page has no lights (its one
/// `DirectionalLight` is commented out upstream), so a sheen term that leaked
/// into the direct path would not show and a missing energy compensation
/// would brighten the whole chair.
///
/// It is also the gate on `KHR_texture_transform` (every map in the asset has
/// one, two of them rotated, which glTF composes `T * R * S` where three.js'
/// `Texture.updateMatrix` composes `T * S * R`) and on `Texture.channel` —
/// the occlusion maps are `TEXCOORD_1`, so this is the first fragment shader
/// that reads two uv sets. See `docs/nodes.md` §25.
#[test]
fn webgpu_loader_gltf_sheen() {
    let name = "webgpu_loader_gltf_sheen";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_loader_gltf_sheen::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_loader_gltf_sheen::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_loader_gltf_sheen::animate, |app| {
        app.renderer.device()
    });
}

/// The Anisotropy Barn Lamp: `KHR_materials_anisotropy` on the shade,
/// `KHR_materials_transmission` + `_volume` on the glass, an UltraHDR
/// equirect as both `scene.environment` and a blurred `scene.background`.
///
/// Two things carry the frame. The scene has no lights at all, so every lit
/// pixel comes from the PMREM — the anisotropic bent normal is what bends the
/// shade's reflection, and a wrong tangent frame shows up as a streak in the
/// wrong direction rather than as an error. And the glass is drawn in a second
/// pass that samples the renderer's mipped copy of the opaque frame, so a
/// missed copy leaves an opaque white bulb over roughly 2% of the image.
#[test]
fn webgpu_loader_gltf_anisotropy() {
    let name = "webgpu_loader_gltf_anisotropy";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_loader_gltf_anisotropy::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_loader_gltf_anisotropy::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_loader_gltf_anisotropy::animate,
        |app| app.renderer.device(),
    );
}

/// `scene.fog` (issue #140) and page-supplied mip levels. Two scenes under
/// `new THREE.Fog( 0x000000, 1500, 4000 )`, one per scissor half: the floor
/// fades into the black background through the render-group fog uniforms,
/// and its texture's eight hand-painted levels — one colour each — show which
/// mip the sampler picks, linear-filtered on the left and
/// `NearestMipmapNearest` on the right. A generated chain in place of the
/// page's levels turns the red / green / blue bands grey.
#[test]
fn webgpu_materials_texture_manualmipmap() {
    let name = "webgpu_materials_texture_manualmipmap";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_materials_texture_manualmipmap::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_materials_texture_manualmipmap::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_materials_texture_manualmipmap::animate,
        |app| app.renderer.device(),
    );
}

/// The gate on `Data3DTexture` and `texture3D` (issue #166): a 128³ `r8unorm`
/// volume of `ImprovedNoise`, raymarched by `RaymarchingBox` with a bisection
/// refinement and a central-difference normal. The iso-surface's shape is the
/// noise and the threshold; its colour is the normal and the position, so a
/// wrong texel order, filter or gradient moves most of the lit pixels.
#[test]
fn webgpu_volume_perlin() {
    let name = "webgpu_volume_perlin";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_volume_perlin::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_volume_perlin::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_volume_perlin::animate, |app| {
        app.renderer.device()
    });
}

/// The gate on storage textures (issue #166): a kernel `textureStore`s the
/// plasma into a 512² `StorageTexture` once, in `init()`, and the plane
/// samples it through the mip chain the renderer rebuilds after the store. A
/// store that never landed leaves the plane transparent black; mips left
/// stale by the store leave it black at the minified sample — either is most
/// of the plane's 250² pixels.
#[test]
fn webgpu_compute_texture() {
    let name = "webgpu_compute_texture";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_compute_texture::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_compute_texture::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_compute_texture::animate, |app| {
        app.renderer.device()
    });
}

/// A KTX2 `CompressedArrayTexture` (issue #172): the six-layer UASTC
/// `spiritedaway.ktx2`, transcoded by `Ktx2Loader` for this adapter — BC7
/// where the device has `TEXTURE_COMPRESSION_BC` — and sampled one layer at a
/// time through a `texture_2d_array<f32>` binding. The layer is `1`, because
/// the pinned clock gives the page's `depthStep` a zero delta. A wrong layer,
/// a flipped uv or a mis-strided block upload each change most of the plane.
#[test]
fn webgpu_textures_2d_array_compressed() {
    let name = "webgpu_textures_2d-array_compressed";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_textures_2d_array_compressed::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_textures_2d_array_compressed::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_textures_2d_array_compressed::animate,
        |app| app.renderer.device(),
    );
}

/// Issue #139's rung: `gears.glb` is three Draco-compressed meshes, and the
/// outer hull's `maskNode` cuts an angular wedge out of it — in the shadow
/// pass too — while its `outputNode` paints the exposed back faces a flat
/// colour. A decoder that got the Draco positions, normals or index wrong
/// moves the whole silhouette; a missing mask in the shadow pass leaves the
/// wedge's shadow on the plane.
#[test]
fn webgpu_tsl_angular_slicing() {
    let name = "webgpu_tsl_angular_slicing";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_tsl_angular_slicing::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_tsl_angular_slicing::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_tsl_angular_slicing::animate, |app| {
        app.renderer.device()
    });
}

/// The gate on `pass.getViewZNode()`: the depth attachment of a **4×MSAA**
/// pass, read back with `textureLoad( …, 0 )` through a
/// `texture_depth_multisampled_2d` binding and converted with
/// `perspectiveDepthToViewZ( depth, cameraNear, cameraFar )`.
///
/// Everything the frame shows rides on that one number. The scene is
/// `webgpu_loader_gltf`'s helmet with no background at all, so most of the
/// image is the *cleared* depth — 1.0, a view z of −`far` — which saturates
/// `smoothstep( 2.7, 4 )` and paints the fog colour. Getting the near/far
/// uniforms, the sample index or the multisample flag wrong changes the whole
/// picture rather than an edge. See `docs/nodes.md` §24.
#[test]
fn webgpu_custom_fog_background() {
    let name = "webgpu_custom_fog_background";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_custom_fog_background::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_custom_fog_background::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    // The `antialias: true` renderer makes the pass target multisampled, and
    // its depth attachment is what the composite binds.
    assert!(
        app.scene_pass.depth_texture().is_multisample(),
        "the pass's depth attachment is a 4x MSAA texture"
    );

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
        webgpu_custom_fog_background::animate,
        |app| app.renderer.device(),
    );
}

/// The gate on a deferred frame: a three-attachment G-buffer, a resolve quad
/// that runs the standard lighting flow over it through `overrideNodes()`, and
/// a transparent pass that shares the opaque pass's depth attachment without
/// clearing it.
///
/// Every switch this rung adds shows in the pixels rather than at an edge.
/// `lighting.enabled = false` on the G-buffer pass is the difference between
/// storing albedo and storing a lit colour; `opaque = false` on the transparent
/// pass is the difference between six lit planes and six planes over a second
/// copy of the skybox; the shared depth texture is the difference between
/// planes that hide behind the teapot and planes that draw through it. See
/// `docs/nodes.md` §27.
#[test]
fn webgpu_deferred() {
    let name = "webgpu_deferred";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_deferred::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_deferred::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    // `pass( scene, camera, { depthTexture: opaquePass.getTexture( 'depth' ) } )`
    // — one depth attachment, two passes. If the transparent pass ever grows
    // its own, the planes stop occluding against the teapot and the diff is
    // tiny but wrong.
    assert_eq!(
        app.transparent_pass.depth_texture().id(),
        app.opaque_pass.depth_texture().id(),
        "the transparent pass must share the opaque pass's depth attachment"
    );

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
    steady_frame(name, &mut app, webgpu_deferred::animate, |app| {
        app.renderer.device()
    });
}

/// The gate on multiple render targets as a *pass* property rather than a
/// material one, and on `WGSLNodeBuilder.isUnfilterable()`.
///
/// One draw of `webgpu_loader_gltf`'s scene fills four attachments; the
/// composite reads all four with `textureLoad` and no sampler, because
/// `pass( …, { minFilter: NearestFilter, magFilter: NearestFilter } )` makes
/// every one of them unfilterable. The skybox writes all four too — it is an
/// ordinary draw under the same MRT — which is what puts the environment in
/// the `normal` and `diffuse` bands. See `docs/nodes.md` §23.
#[test]
fn webgpu_mrt() {
    let name = "webgpu_mrt";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_mrt::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_mrt::animate(&mut app);

    // "optimize textures": attachment 0 stays HalfFloatType and the other
    // three are `UnsignedByteType`, so the pipeline needs a colour target per
    // attachment rather than one shared format.
    assert_eq!(
        app.scene_pass.texture().format(),
        wgpu::TextureFormat::Rgba16Float,
        "the colour attachment is HalfFloatType"
    );
    for extra in ["normal", "diffuse", "emissive"] {
        assert_eq!(
            app.scene_pass.texture_named(extra).format(),
            wgpu::TextureFormat::Rgba8Unorm,
            "the {extra} attachment is UnsignedByteType"
        );
    }

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
    steady_frame(name, &mut app, webgpu_mrt::animate, |app| {
        app.renderer.device()
    });
}

/// `webgpu_pmrem_test`, fed from an **UltraHDR** JPEG rather than a Radiance
/// `.hdr`, with a `backgroundNode` sampling the atlas at a fixed roughness of
/// 0.5.
///
/// The picture is entirely the loader's: the geometry is thirty spheres over
/// roughness x metalness with no lights, so every visible value comes out of
/// `UltraHdrLoader`'s gain-map reconstruction. `src/loaders/ultra_hdr_loader.rs`
/// carries the unit tests for the metadata parsers and the recovery formula;
/// this is the gate on the pixels they produce.
#[test]
fn webgpu_pmrem_equirectangular() {
    let name = "webgpu_pmrem_equirectangular";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_pmrem_equirectangular::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_pmrem_equirectangular::animate(&mut app);

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
    steady_frame(
        name,
        &mut app,
        webgpu_pmrem_equirectangular::animate,
        |app| app.renderer.device(),
    );
}

/// The white-furnace gate, which is the reason this example exists.
///
/// Under a **constant** environment an energy-conserving BSDF returns that
/// constant: every one of the 121 roughness x metalness spheres has to come
/// out the colour of the furnace, so the whole frame is one value and the
/// spheres are invisible. The identity is exact in the WGSL — for metalness 0
/// the diffuse term is `L * ( 1 - ( ssD + msD ) )` against a specular term of
/// `L * ( ssD + msD )`, and for metalness 1 with a white base colour
/// `Fss_ess = Ess` and `Fms * Ems = 1 - Ess` — so the only error is
/// discretisation: the 256-sample GGX prefilter, the half-float atlas and the
/// 16x16 DFG LUT.
///
/// The tolerance is therefore **one 8-bit sRGB step**, which at 0.8 sRGB is
/// about 0.4% of the linear value and is the floor the output pass quantises
/// to anyway; three's own frame is exactly uniform at `0xcccccc`, all 400 000
/// pixels. The expected byte comes from the page's own `COLOR` constant, not
/// from a reference image.
///
/// A wrong energy term does not fail this quietly: it shows up as a band
/// across a row or a column of the grid, which is a large multiple of one
/// step.
fn assert_white_furnace(width: u32, height: u32, pixels: &[u8]) {
    const TOLERANCE: i32 = 1;
    let expected = ((webgpu_furnace_test::COLOR >> 16) & 0xff) as i32;

    let mut lowest = 255i32;
    let mut highest = 0i32;
    let mut worst: Option<(u32, u32, [u8; 3])> = None;

    for y in 0..height {
        for x in 0..width {
            let at = ((y * width + x) * 4) as usize;
            let rgb = [pixels[at], pixels[at + 1], pixels[at + 2]];
            for channel in rgb {
                let value = channel as i32;
                lowest = lowest.min(value);
                highest = highest.max(value);
                if (value - expected).abs() > TOLERANCE && worst.is_none() {
                    worst = Some((x, y, rgb));
                }
            }
        }
    }

    println!(
        "webgpu_furnace_test: furnace channels {lowest}..={highest}, expected \
         {expected} +/- {TOLERANCE}"
    );

    assert!(
        worst.is_none(),
        "white furnace broken: pixel {:?} is {:?}, expected every channel within \
         {TOLERANCE} of {expected} (channels seen: {lowest}..={highest}). A \
         roughness or metalness row that does not return the environment is an \
         energy term, not a pixel difference",
        worst.map(|(x, y, _)| (x, y)),
        worst.map(|(_, _, rgb)| rgb),
    );
}

#[test]
fn webgpu_furnace_test() {
    let name = "webgpu_furnace_test";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_furnace_test::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_furnace_test::animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (800, 500));

    // The numeric gate first: it says *what* is wrong, where the image diff
    // only says that something is.
    assert_white_furnace(width, height, &pixels);

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
    steady_frame(name, &mut app, webgpu_furnace_test::animate, |app| {
        app.renderer.device()
    });
}

/// **The six faces, and the one example that can see them.**
///
/// `webgpu_furnace_test` reaches `fromScene` with a solid-colour environment,
/// so its PMREM is one value everywhere and a permuted face or a flipped `up`
/// are invisible in it (`docs/webgpu_furnace_test-progress.md`). This example
/// is the first one whose environment scene has *content*: a cube-texture
/// background and six coloured spheres. So the face table is observable, and
/// this is where it is held.
///
/// Two independent claims, neither of them from a reference image.
///
/// **1. Each layer of the PMREM is the matching layer of the background
/// cube.** Level 0 of a PMREM is `PMREM_ggx` at roughness 0, which is the
/// source cube read along each texel's own direction, and the source is the
/// scene captured by `CubeCamera`. Both the capture (the skybox) and any later
/// read go through the same `vec3( -d.x, d.yz )` cube lookup, and the WebGPU
/// face table with its `fov = -90` (which flips *both* screen axes, since
/// `width = aspect * height` goes negative too) is exactly the inverse of that
/// lookup. Working it through for layer 0:
///
/// ```text
/// px camera: up ( 0, -1, 0 ), lookAt ( -1, 0, 0 )
/// Object3D.lookAt:  z = ( 1, 0, 0 ), x = cross( up, z ) = ( 0, 0, 1 ),
///                   y = cross( z, x ) = ( 0, -1, 0 )
/// fov -90 flips both axes ⇒ screen ( a, b ) sees ray ( -1, b, -a )
/// the skybox reads vec3( 1, b, -a ) ⇒ +X face at u = ( 1 + a ) / 2,
/// v = ( 1 - b ) / 2 — screen pixel ( a, b ) itself
/// ```
///
/// So layer *i* holds `images[ i ]` upright, down-sampled from 1024² to 256².
/// A test that recomputed the port's own mapping would agree with itself; this
/// one holds the identity as a literal.
///
/// The comparison is over the 12 non-central blocks of a 4 × 4 grid of 64²
/// block means, which skips the sphere (angular radius `asin 0.2` = 11.5°, so
/// 26 px around the face centre) and absorbs the few pixels of drift that
/// come of the background being a *tessellated* 32 × 32 sphere rather than a
/// full-screen blit. Every one of the 6 images × 8 dihedral orientations is
/// scored, and the expected image in the identity orientation has to be the
/// best by a wide margin — so this gate fails on a permuted face and on a
/// flipped or rolled `up`.
///
/// **2. Each layer holds the right sphere.** The six `MeshBasicMaterial`
/// spheres sit one per axis at distance 1, so the centre of layer *i* is the
/// colour of the sphere face *i*'s camera looks at — px at **-x**, nx at +x,
/// py at +y, ny at -y, pz at +z, nz at -z — and the six colours are distinct.
/// That pins the direction table on its own, independently of the background.
fn assert_face_tiles(app: &mut webgpu_pmrem_scene::App) {
    /// `_setSize( 256 )`.
    const FACE: usize = 256;
    /// The Park3Med faces are 1024², four times the 256² face.
    const SIDE: usize = 1024;
    /// `CubeTexture.images` is in three's order px, nx, py, ny, pz, nz.
    const NAMES: [&str; 6] = ["px", "nx", "py", "ny", "pz", "nz"];
    /// Layer *i* of the PMREM ⇒ that index into `images`. Derived above.
    const TILE_IMAGE: [usize; 6] = [0, 1, 2, 3, 4, 5];
    /// The sphere each face looks at. `SPHERES` is in the page's own order,
    /// so this is the page's own colours, not new constants.
    const TILE_SPHERE: [usize; 6] = [3, 2, 5, 4, 1, 0];

    let pmrem = app.environment.texture().clone();

    // The sRGB EOTF the *hardware* applies to an `Rgba8UnormSrgb` cube face
    // before it filters it — the IEC 61966-2-1 curve, not three's rational
    // approximation of it, so this side of the comparison owes the port
    // nothing.
    fn to_linear(value: u8) -> f32 {
        let value = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    // The 4 × 4 grid of block means, for the PMREM face and for each image.
    let block_means = |read: &dyn Fn(usize, usize) -> [f32; 3], size: usize| {
        let step = size / 4;
        let mut means = [[[0.0f32; 3]; 4]; 4];
        for (by, row) in means.iter_mut().enumerate() {
            for (bx, mean) in row.iter_mut().enumerate() {
                let mut sum = [0.0f64; 3];
                for y in by * step..(by + 1) * step {
                    for x in bx * step..(bx + 1) * step {
                        let rgb = read(x, y);
                        for c in 0..3 {
                            sum[c] += rgb[c] as f64;
                        }
                    }
                }
                let n = (step * step) as f64;
                *mean = [
                    (sum[0] / n) as f32,
                    (sum[1] / n) as f32,
                    (sum[2] / n) as f32,
                ];
            }
        }
        means
    };

    /// One of the eight symmetries of the square, as a block-index map.
    type Dihedral = (&'static str, fn(usize, usize) -> (usize, usize));

    // The eight symmetries of the square, as block-index maps.
    let dihedral: [Dihedral; 8] = [
        ("identity", |x, y| (x, y)),
        ("flipX", |x, y| (3 - x, y)),
        ("flipY", |x, y| (x, 3 - y)),
        ("rot180", |x, y| (3 - x, 3 - y)),
        ("transpose", |x, y| (y, x)),
        ("rot90", |x, y| (3 - y, x)),
        ("rot270", |x, y| (y, 3 - x)),
        ("antitranspose", |x, y| (3 - y, 3 - x)),
    ];

    let images: Vec<(usize, usize, Vec<u8>)> = {
        let cube = app.cube.borrow();
        cube.images
            .iter()
            .map(|image| {
                (
                    image.width as usize,
                    image.height as usize,
                    image.data.clone(),
                )
            })
            .collect()
    };
    let image_means: Vec<[[[f32; 3]; 4]; 4]> = images
        .iter()
        .map(|(width, _, data)| {
            assert_eq!(*width, SIDE, "the Park3Med faces are 1024²");
            block_means(
                &|x, y| {
                    let at = (y * width + x) * 4;
                    [
                        to_linear(data[at]),
                        to_linear(data[at + 1]),
                        to_linear(data[at + 2]),
                    ]
                },
                SIDE,
            )
        })
        .collect();

    for face in 0..6 {
        let (face_width, face_height, atlas) = app
            .renderer
            .read_cube_pixels_rgba16f(&pmrem, face as u32, 0)
            .unwrap();
        assert_eq!((face_width, face_height), (FACE as u32, FACE as u32));

        let tile_means = block_means(
            &|x, y| {
                let at = (y * FACE + x) * 4;
                [atlas[at], atlas[at + 1], atlas[at + 2]]
            },
            FACE,
        );

        // Score every image in every orientation over the 12 blocks the
        // sphere does not touch.
        let mut scores: Vec<(f32, usize, &str)> = Vec::new();
        for (image, means) in image_means.iter().enumerate() {
            for (name, map) in dihedral {
                let mut error = 0.0f32;
                for (by, row) in tile_means.iter().enumerate() {
                    for (bx, mean) in row.iter().enumerate() {
                        if (1..=2).contains(&bx) && (1..=2).contains(&by) {
                            continue;
                        }
                        let (ix, iy) = map(bx, by);
                        for c in 0..3 {
                            error += (mean[c] - means[iy][ix][c]).abs();
                        }
                    }
                }
                scores.push((error / (12.0 * 3.0), image, name));
            }
        }
        scores.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let (best, image, orientation) = scores[0];
        let runner_up = scores[1];
        println!(
            "face {face}: best {}.jpg {orientation} at {best:.4}, \
             then {}.jpg {} at {:.4}",
            NAMES[image], NAMES[runner_up.1], runner_up.2, runner_up.0
        );
        assert_eq!(
            (image, orientation),
            (TILE_IMAGE[face], "identity"),
            "face {face} is {}.jpg {orientation}, not {}.jpg upright — a permuted \
             face or a flipped `up`",
            NAMES[image],
            NAMES[TILE_IMAGE[face]]
        );
        assert!(
            best < 0.002,
            "face {face} matches {}.jpg upright but only to {best:.4} in linear light",
            NAMES[image]
        );
        assert!(
            runner_up.0 > best * 5.0,
            "face {face}: {}.jpg upright ({best:.4}) is not clearly better than {}.jpg {} \
             ({:.4}), so this gate is not discriminating",
            NAMES[image],
            NAMES[runner_up.1],
            runner_up.2,
            runner_up.0
        );

        // The sphere in the middle of the tile.
        let (hex, _) = webgpu_pmrem_scene::SPHERES[TILE_SPHERE[face]];
        let expected = three_rs::Color::from_hex(hex);
        let expected = [expected.r as f32, expected.g as f32, expected.b as f32];
        let mut centre = [0.0f64; 3];
        for y in FACE / 2 - 4..FACE / 2 + 4 {
            for x in FACE / 2 - 4..FACE / 2 + 4 {
                let at = (y * FACE + x) * 4;
                for c in 0..3 {
                    centre[c] += atlas[at + c] as f64;
                }
            }
        }
        let centre = [
            (centre[0] / 64.0) as f32,
            (centre[1] / 64.0) as f32,
            (centre[2] / 64.0) as f32,
        ];
        println!(
            "face {face}: centre {centre:?}, sphere {:#08x} {expected:?}",
            hex
        );
        for c in 0..3 {
            assert!(
                (centre[c] - expected[c]).abs() < 0.01,
                "face {face} looks at the {:#08x} sphere, but the middle of its layer is \
                 {centre:?}",
                hex
            );
        }
    }
}

#[test]
fn webgpu_pmrem_scene() {
    let name = "webgpu_pmrem_scene";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_pmrem_scene::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    // The generator gate first, before a pixel of the frame is looked at:
    // this is the one example on the ladder whose environment scene is not a
    // solid colour, so it is the one whose atlas can say whether the six face
    // viewports really hold six different images in the right places.
    assert_face_tiles(&mut app);

    webgpu_pmrem_scene::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_pmrem_scene::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_lights_phong() {
    let name = "webgpu_lights_phong";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_lights_phong::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_lights_phong::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_morphtargets::animate, |app| {
        app.renderer.device()
    });
}

/// The rung whose pixels prove the least.
///
/// Three's own frame is black apart from the particle cloud, which at 400x250
/// — the size `image.js` downscales to — is a couple of lit pixels near the
/// centre. Everything the compute stage does is upstream of a frame that
/// Three's comparator would pass at 0.0% if the particles never moved. So this
/// test is here because the brief requires every rung to be graded, and the
/// rung's real gates are `tests/nodes_compute_wgsl.rs` (what the kernels
/// compile to) and `tests/renderer_compute_points.rs` (what they wrote). See
/// docs/rung12-progress.md.
#[test]
fn webgpu_compute_points() {
    let name = "webgpu_compute_points";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_compute_points::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_compute_points::animate(&mut app);

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

    // Not a pixel assertion — a liveness one, and the only thing this frame
    // can honestly say. The compute stage put 300 000 particles somewhere;
    // if the frame is entirely black, nothing was drawn at all, and the
    // comparison above would still have passed.
    let lit = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 0)
        .count();
    assert!(lit > 0, "{name}: the frame is completely black");
    println!("{name}: {lit} lit pixels at {width}x{height}");

    steady_frame(name, &mut app, webgpu_compute_points::animate, |app| {
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
    steady_frame(name, &mut app, webgpu_tsl_galaxy::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_tsl_vfx_flames() {
    let name = "webgpu_tsl_vfx_flames";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_tsl_vfx_flames::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_tsl_vfx_flames::animate(&mut app);
    println!("{name}: info {:?}", app.renderer.info());

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
    steady_frame(name, &mut app, webgpu_tsl_vfx_flames::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_tsl_raging_sea() {
    let name = "webgpu_tsl_raging_sea";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_tsl_raging_sea::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_tsl_raging_sea::animate(&mut app);

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
    // The draw calls and triangles the README's graded table records.
    let info = app.renderer.info();
    println!("{name}: info {info:?}");

    steady_frame(name, &mut app, webgpu_tsl_raging_sea::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_tsl_interoperability() {
    let name = "webgpu_tsl_interoperability";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_tsl_interoperability::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_tsl_interoperability::animate(&mut app);

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

    // `outputColorSpace = LinearSRGBColorSpace` is the working space, so
    // `needsFrameBufferTarget` is false: two programs and two pipelines, the
    // two quads, and no colour-transform pass behind them.
    let info = app.renderer.info();
    println!("{name}: info {info:?}");
    assert_eq!(
        (
            info.render.calls,
            info.build.pipelines_built,
            info.memory.programs
        ),
        (2, 2, 2),
        "a linear output space draws the scene straight into the canvas"
    );

    steady_frame(
        name,
        &mut app,
        webgpu_tsl_interoperability::animate,
        |app| app.renderer.device(),
    );
}

#[test]
fn webgpu_mesh_batch() {
    let name = "webgpu_mesh_batch";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_mesh_batch::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_mesh_batch::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_mesh_batch::animate, |app| {
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
    steady_frame(name, &mut app, webgpu_lights_physical::animate, |app| {
        app.renderer.device()
    });
}

#[test]
fn webgpu_skinning() {
    let name = "webgpu_skinning";
    let out = out_dir(name);
    let _gpu = gpu();

    let mut app = webgpu_skinning::init();
    println!("adapter: {:?}", app.renderer.adapter_info());

    webgpu_skinning::animate(&mut app);

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
    steady_frame(name, &mut app, webgpu_skinning::animate, |app| {
        app.renderer.device()
    });
}

/// Issue #56's "done when", as issue #67's counts: a steady frame builds and
/// uploads *nothing*. Every graded rung is rendered three times; the first
/// frame builds its programs and uploads its geometries and textures, and by
/// the second nothing in the scene is new to the renderer — the same
/// steady-frame behaviour `WebGPURenderer` has, where `RenderObjects.get()`
/// finds every render object's cache key unchanged and
/// `NodeManager.getForRender()` never reaches the builder.
///
/// Frames two and three are held to `BuildCounts::default()` — every field at
/// exactly zero, an equality rather than a ceiling, which is the whole reason
/// the count ladder exists beside the time one: the counts are deterministic
/// where a time is not, and a re-upload of a live geometry that a generous
/// time ceiling would never notice is one number here.
///
/// A rung's frame is not always one `render()`: rtt and the depth texture
/// render into a target and then draw a full-screen quad, and postprocessing
/// masking is three `PassNode`s and a `RenderPipeline`. `testing::strip` turns
/// `info.auto_reset` off and resets once per *frame*, the way three.js'
/// postprocessing does, so the counts below are the whole frame's — and it
/// keeps each frame's pixels too, so `target/e2e/<rung>/steady-strip.png` is
/// the three frames side by side with their counts under them (issue #68).
#[test]
fn steady_frame_builds_nothing() {
    let _gpu = gpu();

    macro_rules! rung {
        ($module:ident) => {
            rung!($module, 0)
        };
        ($module:ident, $textures:expr) => {
            rung!($module, $textures, 1)
        };
        // `$steady_from`: the first frame index held to zero. 1 for every
        // rung but one; see `webgpu_postprocessing_difference` below.
        ($module:ident, $textures:expr, $steady_from:expr) => {{
            let mut app = $module::init();

            // `[ "", "" ]`: nothing is done to the scene between the three
            // frames, which is what makes frames two and three steady.
            let strip = three_rs::testing::strip(
                &mut app,
                |app| &mut app.renderer,
                &mut |app: &mut $module::App| $module::animate(app),
                &mut [
                    ("", &mut |_: &mut $module::App| {}),
                    ("", &mut |_: &mut $module::App| {}),
                ],
            )
            .unwrap();

            for (index, frame) in strip.frames.iter().enumerate() {
                println!(
                    "{}: frame {} — {}",
                    stringify!($module),
                    index + 1,
                    frame.info
                );
            }

            assert!(
                strip.frames[0].info.build.total() > 0,
                "{}: the first frame built nothing, so the counters are not wired",
                stringify!($module)
            );
            assert!(
                strip.frames[2].info.render.calls > 0,
                "{}: the third frame drew nothing, so it is not a frame",
                stringify!($module)
            );
            strip.assert_steady_uploading($steady_from.., $textures);

            let png = out_dir(stringify!($module)).join("steady-strip.png");
            strip.write_png(png.to_str().expect("three-rs: the strip path is UTF-8"));
            println!("{}: strip {}", stringify!($module), png.display());
        }};
    }

    rung!(webgpu_depth_texture);
    rung!(webgpu_instance_mesh);
    rung!(webgpu_instance_uniform);
    rung!(webgpu_materials);
    rung!(webgpu_materials_basic);
    rung!(webgpu_materials_envmaps);
    rung!(webgpu_materials_cubemap_mipmaps);
    rung!(webgpu_rtt);
    rung!(webgpu_postprocessing_masking);
    // A two-frame cycle: `PassNode.toggleTexture()` swaps the current and
    // previous textures' GPU allocations before every render, so frame two
    // is the first to see each texture handle over its *other* allocation,
    // and makes the one view per handle and the one bind group that pairing
    // needs (issue #137). Frame three is back on frame one's pairing and
    // must create nothing; so must every frame after.
    rung!(webgpu_postprocessing_difference, 0, 2);
    rung!(webgpu_postprocessing_direct);
    rung!(webgpu_postprocessing_radial_blur);
    rung!(webgpu_postprocessing_ssaa);
    rung!(webgpu_postprocessing_ca);
    rung!(webgpu_postprocessing_bloom_selective);
    rung!(webgpu_postprocessing_anamorphic);
    rung!(webgpu_postprocessing_bloom);
    // The cube background and the PMREM environment are both built in
    // `init()`, before the first frame, so the steady frames build nothing for
    // them either.
    rung!(webgpu_postprocessing_bloom_emissive);
    rung!(webgpu_lights_phong);
    rung!(webgpu_morphtargets);
    rung!(webgpu_tsl_galaxy);
    rung!(webgpu_tsl_interoperability);
    rung!(webgpu_tsl_vfx_flames);
    rung!(webgpu_tsl_raging_sea);
    rung!(webgpu_shadowmap);
    rung!(webgpu_lights_physical);
    // The PMREM is built once, before the first frame; `update` is idempotent,
    // so the steady frames neither render nor upload anything for it.
    rung!(webgpu_pmrem_cubemap);
    rung!(webgpu_pmrem_test);
    // `fromScene` builds the PMREM in `init()`, so `update` is a no-op here
    // too and the steady frames are the scene pass and the output blit.
    rung!(webgpu_furnace_test);
    rung!(webgpu_pmrem_scene);
    rung!(webgpu_skinning);
    // The batch rewrites its indirect texture every `onBeforeRender()` and its
    // matrices texture every `animateMeshes()`; three.js uploads the same two.
    rung!(webgpu_mesh_batch, 2);
    rung!(webgpu_compute_points);
    rung!(webgpu_lines_fat);
    rung!(webgpu_lines_fat_raycasting);
    rung!(webgpu_loader_gltf);
    rung!(webgpu_loader_gltf_sheen);
    rung!(webgpu_mrt);
    rung!(webgpu_custom_fog_background);
    rung!(webgpu_deferred);
    rung!(webgpu_loader_gltf_anisotropy);
    rung!(webgpu_materials_texture_manualmipmap);
    rung!(webgpu_postprocessing_transition);
    rung!(webgpu_postprocessing_sobel);
    rung!(webgpu_procedural_texture);
    rung!(webgpu_volume_perlin);
    rung!(webgpu_compute_texture);
    rung!(webgpu_textures_2d_array_compressed);
    rung!(webgpu_tsl_angular_slicing);
}

// ---------------------------------------------------------------------------
// issue #58: cache identity and eviction
// ---------------------------------------------------------------------------

/// Issue #58 as counts (issue #67): what a scene mutation costs, and what it
/// gives back.
///
/// The cache tests below reach into the renderer's cache lengths, which is the
/// only way to see an entry that was never evicted. `info` says the same thing
/// in the vocabulary a consumer has: swapping one mesh's geometry uploads
/// **exactly one** geometry and builds nothing else, and dropping the mesh
/// takes the resident count back down. A renderer that re-uploaded a live
/// geometry every frame, or that never freed a dead one, moves one of these
/// numbers; neither shows up in a frame time.
///
/// The two steps are issue #68's `[ "replace geometry", "drop geometry" ]`, so
/// the record is a strip: three frames of the cube, each with its counts under
/// it, in `target/e2e/geometry_mutation/strip.png`.
#[test]
fn a_scene_mutation_uploads_exactly_what_changed() {
    use std::rc::Rc;
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::{
        box_geometry, BuildCounts, Mesh, Node, PerspectiveCamera, Renderer, RendererParameters,
        Scene,
    };

    /// The smallest thing `testing::strip` renders: a renderer, a scene, a
    /// camera, and the handle the steps mutate.
    struct App {
        renderer: Renderer,
        scene: Scene,
        camera: PerspectiveCamera,
        mesh: Option<Node>,
    }

    let _gpu = gpu();

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let mesh = Mesh::new(
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        MeshBasicNodeMaterial::new(),
    );
    let scene = Scene::new();
    scene.add(&mesh);

    let mut app = App {
        renderer,
        scene,
        camera,
        mesh: Some(mesh),
    };

    let strip = three_rs::testing::strip(
        &mut app,
        |app| &mut app.renderer,
        &mut |app: &mut App| app.renderer.render(&mut app.scene, &mut app.camera),
        &mut [
            // Replace the one geometry, keeping the mesh and its material.
            ("replace geometry", &mut |app: &mut App| {
                app.mesh
                    .as_ref()
                    .expect("three-rs: the mesh is still here")
                    .borrow_mut()
                    .mesh_mut()
                    .expect("three-rs: the mesh is a mesh")
                    .geometry = Rc::new(box_geometry(2.0, 2.0, 2.0, 1, 1, 1));
            }),
            // And drop it, which is the only signal the renderer gets.
            ("drop geometry", &mut |app: &mut App| {
                let mesh = app.mesh.take().expect("three-rs: the mesh is still here");
                app.scene.remove(&mesh);
            }),
        ],
    )
    .unwrap();

    for (index, frame) in strip.frames.iter().enumerate() {
        println!(
            "mutation: frame {} ({}) — {}",
            index + 1,
            frame.label,
            frame.info
        );
    }

    let png = out_dir("geometry_mutation").join("strip.png");
    strip.write_png(png.to_str().expect("three-rs: the strip path is UTF-8"));
    println!("mutation: strip {}", png.display());

    let [first, replaced, dropped] = match strip.frames.as_slice() {
        [first, replaced, dropped] => [&first.info, &replaced.info, &dropped.info],
        frames => panic!("three-rs: expected three frames, got {}", frames.len()),
    };

    // One upload, and nothing else: the resident count holds because the
    // geometry that was replaced is swept in the same render that uploads its
    // replacement.
    assert_eq!(
        replaced.build.geometries_uploaded, 1,
        "replacing one geometry should upload exactly one"
    );
    assert_eq!(
        replaced.build.programs_compiled, 0,
        "the material did not change, so nothing should have been built"
    );
    assert_eq!(
        replaced.memory.geometries, first.memory.geometries,
        "the replaced geometry should have been swept as the new one arrived"
    );

    // And the resident count goes back down by one. This is the #58 regression
    // — nothing was ever removed from the cache — as a number.
    assert_eq!(
        dropped.memory.geometries,
        first.memory.geometries - 1,
        "the dropped geometry should have left the cache"
    );
    assert_eq!(
        dropped.build,
        BuildCounts::default(),
        "dropping an object builds nothing"
    );
}

/// Issue #50: the readback off a `RenderTarget` is the readback off the canvas.
///
/// `read_canvas_pixels()` was the only readback, and it reads the canvas, which
/// is right only for as long as `present()` is a blit of that canvas. Both now
/// go through one copy-to-buffer-and-map path, so the way to check the new one
/// is to render the same scene twice — once to a target, once to the canvas —
/// and read each back its own way. Same pixels, or the shared path is not
/// shared.
#[test]
fn a_render_target_reads_back_the_same_pixels_as_the_canvas() {
    use std::rc::Rc;
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::{
        box_geometry, Mesh, PerspectiveCamera, RenderTarget, Renderer, RendererParameters, Scene,
    };

    let _gpu = gpu();

    // `antialias: false` so the canvas is single-sample, as a default
    // `RenderTarget` is; an MSAA canvas against a single-sample target would be
    // comparing the resolve, not the readback.
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);

    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let mesh = Mesh::new(
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        MeshBasicNodeMaterial::new(),
    );
    let mut scene = Scene::new();
    scene.add(&mesh);

    renderer.render(&mut scene, &mut camera);
    let (canvas_width, canvas_height, canvas) = renderer.read_canvas_pixels().unwrap();

    let target = RenderTarget::new(canvas_width, canvas_height);
    renderer.set_render_target(Some(target.clone()));
    renderer.render(&mut scene, &mut camera);
    renderer.set_render_target(None);

    let (width, height, pixels) = renderer.read_target_pixels(&target).unwrap();

    assert_eq!(
        (width, height),
        (canvas_width, canvas_height),
        "the target is the canvas' size"
    );
    assert_eq!(
        pixels.len(),
        (width * height * 4) as usize,
        "tightly packed RGBA8, the row padding stripped"
    );
    assert!(
        pixels.iter().any(|byte| *byte != 0),
        "the target was drawn to, so it cannot read back as all zeroes"
    );

    let differing = canvas
        .iter()
        .zip(pixels.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(
        differing, 0,
        "the same scene read off the target and off the canvas: {differing} bytes differ"
    );
}

/// Issue #47: a vertex moved in place, and the one buffer write it costs.
///
/// `attribute.array_mut()` then `set_needs_update()` is three.js'
/// `attribute.needsUpdate = true`, and the renderer's answer to it has to be
/// exactly one `queue.write_buffer` — not a new geometry, not a re-upload of
/// the attributes that did not change. The sibling test above is the same
/// scene with the geometry *replaced*; this is what the cheaper route costs.
#[test]
fn a_mutated_attribute_rewrites_one_buffer() {
    use std::rc::Rc;
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::{box_geometry, Mesh, PerspectiveCamera, Renderer, RendererParameters, Scene};

    let _gpu = gpu();

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    let mesh = Mesh::new(geometry.clone(), MeshBasicNodeMaterial::new());
    let mut scene = Scene::new();
    scene.add(&mesh);

    renderer.render(&mut scene, &mut camera);
    let (width, height, before) = renderer.read_canvas_pixels().unwrap();

    // Slide every vertex a long way to the left, through the `Rc` the mesh is
    // holding — no new geometry, no new mesh, nothing removed from the scene.
    let position = geometry
        .get_attribute("position")
        .expect("three-rs: the box has a position attribute");
    let uploaded = position.version();
    {
        let mut array = position.array_mut();
        for x in array.iter_mut().step_by(3) {
            *x -= 1.5;
        }
    }
    position.set_needs_update();
    assert_eq!(
        position.version(),
        uploaded + 1,
        "set_needs_update() bumps the attribute's version once"
    );

    renderer.render(&mut scene, &mut camera);
    let info = renderer.info().clone();
    println!("mutated attribute: {info}");

    assert_eq!(
        info.build.buffers_written, 1,
        "one moved attribute is one buffer write"
    );
    assert_eq!(
        info.build.geometries_uploaded, 0,
        "the geometry kept its id, so nothing was uploaded"
    );
    assert_eq!(
        info.build.programs_compiled, 0,
        "moving a vertex does not touch the material"
    );

    let (after_width, after_height, after) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (after_width, after_height));

    let moved = before
        .iter()
        .zip(after.iter())
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        moved > 0,
        "the box moved, so the frame must have changed; {moved} bytes differ"
    );

    // A third render changes nothing: the version now matches what was
    // uploaded, so the steady frame is a steady frame again.
    renderer.render(&mut scene, &mut camera);
    assert_eq!(
        renderer.info().build,
        three_rs::BuildCounts::default(),
        "a frame after the re-upload builds nothing"
    );
}

/// Issue #137: a draw's uniform buffer and bind group are kept across frames,
/// so a steady frame creates neither — and the cache must not freeze what is
/// in them. Changing a material's colour, and then moving the mesh, has to
/// reach the pixels through the same buffer and the same bind group, with no
/// creation at all.
#[test]
fn a_mutated_uniform_reaches_the_pixels_through_the_kept_buffer() {
    use std::rc::Rc;
    use three_rs::materials::MeshBasicNodeMaterial;
    use three_rs::{
        box_geometry, BuildCounts, Color, Mesh, PerspectiveCamera, Renderer, RendererParameters,
        Scene,
    };

    let _gpu = gpu();

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xff0000);
    let mesh = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    let mut scene = Scene::new();
    scene.add(&mesh);

    // The centre texel, RGBA.
    let centre = |renderer: &mut Renderer| {
        let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
        let at = ((height / 2 * width + width / 2) * 4) as usize;
        [pixels[at], pixels[at + 1], pixels[at + 2]]
    };

    // Two frames: the first builds, the second is steady.
    renderer.render(&mut scene, &mut camera);
    renderer.render(&mut scene, &mut camera);
    assert_eq!(
        renderer.info().build,
        BuildCounts::default(),
        "a steady frame creates no buffer, view, sampler or bind group"
    );
    let red = centre(&mut renderer);
    assert!(
        red[0] > 200 && red[1] < 50 && red[2] < 50,
        "the box starts red, got {red:?}"
    );

    // `material.color = …` with no `needsUpdate`: the program is unchanged,
    // only the uniform's bytes move.
    mesh.borrow_mut()
        .mesh_mut()
        .expect("three-rs: the mesh is a mesh")
        .material
        .as_mut()
        .expect("three-rs: the mesh has a material")
        .color = Color::from_hex(0x0000ff);
    renderer.render(&mut scene, &mut camera);
    let info = renderer.info().clone();
    println!("mutated uniform: {info}");
    assert_eq!(
        info.build,
        BuildCounts::default(),
        "a new colour is a buffer write, not a new buffer or bind group"
    );
    let blue = centre(&mut renderer);
    assert!(
        blue[0] < 50 && blue[1] < 50 && blue[2] > 200,
        "the colour change must reach the pixels, got {blue:?} (was {red:?})"
    );

    // And the object's half of the same group: move the box off the centre.
    mesh.borrow_mut().position.x = 3.0;
    renderer.render(&mut scene, &mut camera);
    assert_eq!(
        renderer.info().build,
        BuildCounts::default(),
        "a moved object is a buffer write, not a new buffer or bind group"
    );
    let moved = centre(&mut renderer);
    assert_ne!(
        moved, blue,
        "the box moved off the centre, so the centre must have changed"
    );
}

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

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
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
    let mesh = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    let mut scene = Scene::new();
    scene.add(&mesh);
    renderer.render(&mut scene, &mut camera);

    // and it drew: the plane is white against a cleared canvas.
    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
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

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(64.0, 64.0);
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.z = 5.0;

    let mut sizes = Vec::new();
    let mut bindings = Vec::new();
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
        bindings.push(renderer.binding_cache_lens());
    }

    // Issue #137's caches: each frame's mesh is a new object with a new
    // material, so its draw buffers and bind groups are new entries every
    // frame, and must age out at the same rate they arrive.
    println!(
        "churn: binding caches (draw buffers, views, bind groups) frame 10 {:?}, frame {FRAMES} {:?}",
        bindings[9],
        bindings[FRAMES - 1]
    );
    let (settled, last) = (bindings[9], bindings[FRAMES - 1]);
    assert!(
        last.0 <= settled.0 && last.1 <= settled.1 && last.2 <= settled.2,
        "the binding caches were still growing at frame {FRAMES}: {settled:?} at frame 10, \
         {last:?} at the end"
    );

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
