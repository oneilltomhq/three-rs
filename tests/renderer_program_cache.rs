//! The compiled-program caches let go of what nothing names any more
//! (issue #237).
//!
//! `programs` is keyed by a content hash, so its only failure is growth: a
//! consumer that keeps editing a material's graph compiles one program per
//! distinct WGSL, and before #237 kept every one for the renderer's life. The
//! compute-program cache is keyed by the address of a kernel's statement
//! nodes and used to hold those nodes alive, so a caller that built a fresh
//! kernel every frame grew it by one program and one node graph per frame.
//!
//! Each test runs more frames than the sweep's grace window and checks that
//! the counts settle rather than climb, and that whatever survives or is
//! rebuilt still draws or computes the right thing.

use std::rc::Rc;
use std::sync::Mutex;

use three_rs::nodes::tsl::{instanced_array, uint, vec3, StorageArray};
use three_rs::nodes::{ComputeFlow, NodeRef, Type};
use three_rs::{
    plane_geometry, Mesh, MeshBasicNodeMaterial, ObjectRef, OrthographicCamera, Renderer,
    RendererParameters, Scene,
};

/// `CACHE_GRACE_FRAMES`: how many frames an unnamed program survives.
const GRACE: usize = 4;

/// The tests each build a device; one at a time, as `tests/e2e` does, so a
/// default multi-threaded `cargo test` does not contend for the adapter.
static GPU: Mutex<()> = Mutex::new(());

fn gpu() -> std::sync::MutexGuard<'static, ()> {
    GPU.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn renderer() -> Renderer {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(32.0, 32.0);
    renderer
}

/// `new OrthographicCamera( -1, 1, 1, -1, 0.1, 100 )` at `z = 10`: the 2x2
/// plane at the origin fills the canvas.
fn camera() -> OrthographicCamera {
    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 100.0);
    camera.object.position.z = 10.0;
    camera.update_matrix_world();
    camera
}

/// A 2x2 plane drawing `color`.
fn plane(color: NodeRef) -> ObjectRef {
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(color);
    Mesh::new(Rc::new(plane_geometry(2.0, 2.0, 1, 1)), material)
}

/// `mesh.material.colorNode = node; mesh.material.needsUpdate = true`.
fn set_color_node(mesh: &ObjectRef, node: NodeRef) {
    let mut object = mesh.borrow_mut();
    let material = object
        .mesh_mut()
        .and_then(|mesh| mesh.material.as_mut())
        .expect("three-rs: the plane has a material");
    material.color_node = Some(node);
    material.set_needs_update();
}

/// The canvas' centre pixel, RGB.
fn centre(renderer: &mut Renderer) -> [u8; 3] {
    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
    let at = ((h as usize / 2) * w as usize + w as usize / 2) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// `info.memory.programs` is the two compiled caches together.
#[track_caller]
fn assert_memory_counts(renderer: &Renderer) {
    let (programs, _, _, compute_pipelines) = renderer.program_cache_lens();
    assert_eq!(
        renderer.info().memory.programs,
        programs + compute_pipelines,
        "info.memory.programs counts the render programs and compute pipelines"
    );
}

/// One-invocation kernel `out[ 0 ] = out[ 0 ] + 1`, built from new nodes on
/// every call: the same WGSL every time, never the same nodes.
fn increment(out: &StorageArray) -> ComputeFlow {
    ComputeFlow::new(
        vec![out
            .element(uint(0))
            .assign(out.element(uint(0)).add(uint(1)))],
        1,
    )
}

/// A material whose graph is edited every frame — a new constant colour, so
/// new WGSL, each time — keeps the variants of the last few frames, not all of
/// them. Going back to a variant evicted long ago rebuilds it, and it draws.
#[test]
fn editing_a_material_graph_does_not_grow_the_program_cache() {
    const FRAMES: usize = 40;
    let _gpu = gpu();
    let mut renderer = renderer();
    let mut camera = camera();

    let mesh = plane(vec3(1.0, 0.0, 0.0));
    let mut scene = Scene::new();
    scene.add(&mesh);
    renderer.render(&mut scene, &mut camera);
    assert_eq!(centre(&mut renderer), [255, 0, 0]);
    let first = renderer.program_cache_lens();

    let mut lens = Vec::with_capacity(FRAMES);
    for frame in 1..=FRAMES {
        set_color_node(&mesh, vec3(0.0, frame as f64 / FRAMES as f64, 1.0));
        renderer.render(&mut scene, &mut camera);
        assert_eq!(
            renderer.info().build.programs_compiled,
            1,
            "frame {frame}: an edited graph is one build"
        );
        assert_memory_counts(&renderer);
        lens.push(renderer.program_cache_lens());
    }
    println!(
        "edits: (programs, pipelines, compute programs, compute pipelines) frame 0 {first:?}, \
         frame {} {:?}, frame {FRAMES} {:?}",
        2 * GRACE,
        lens[2 * GRACE - 1],
        lens[FRAMES - 1]
    );

    let (programs, pipelines, _, _) = lens[FRAMES - 1];
    assert_eq!(
        lens[FRAMES - 1],
        lens[2 * GRACE - 1],
        "the program caches were still growing after {FRAMES} edits"
    );
    // The frame's own variant, the ones still inside the grace window, and
    // the renderer's own (the output pass).
    assert!(
        programs <= first.0 + GRACE + 1 && pipelines <= first.1 + GRACE + 1,
        "{FRAMES} edits left {programs} programs and {pipelines} pipelines, \
         against {first:?} after one frame"
    );

    // The first variant has been out of the window for most of the run, so
    // this rebuilds it — the same cache key as before, with a new layout that
    // no cached bind group was made against.
    set_color_node(&mesh, vec3(1.0, 0.0, 0.0));
    renderer.render(&mut scene, &mut camera);
    assert_eq!(renderer.info().build.pipelines_built, 1);
    assert_eq!(
        centre(&mut renderer),
        [255, 0, 0],
        "the rebuilt first variant draws red"
    );
}

/// A material toggled between two graphs every frame recompiles neither: a
/// version bump re-runs the node builder, but the program it lands on is
/// still cached from the frame before last.
#[test]
fn a_material_toggled_between_two_graphs_keeps_both_programs() {
    let _gpu = gpu();
    let mut renderer = renderer();
    let mut camera = camera();

    let red = vec3(1.0, 0.0, 0.0);
    let blue = vec3(0.0, 0.0, 1.0);
    let mesh = plane(red.clone());
    let mut scene = Scene::new();
    scene.add(&mesh);
    renderer.render(&mut scene, &mut camera);
    set_color_node(&mesh, blue.clone());
    renderer.render(&mut scene, &mut camera);
    let settled = renderer.program_cache_lens();

    for frame in 0..4 * GRACE {
        let (node, want) = if frame % 2 == 0 {
            (red.clone(), [255, 0, 0])
        } else {
            (blue.clone(), [0, 0, 255])
        };
        set_color_node(&mesh, node);
        renderer.render(&mut scene, &mut camera);
        assert_eq!(
            renderer.info().build.pipelines_built,
            0,
            "toggle {frame}: the program and pipeline were evicted and rebuilt"
        );
        assert_eq!(renderer.program_cache_lens(), settled, "toggle {frame}");
        assert_eq!(centre(&mut renderer), want, "toggle {frame}");
    }
}

/// A render loop that builds its kernel afresh every frame — new nodes, the
/// same WGSL — and drops it: the built-program entry goes with the kernel,
/// and the compiled pipeline, still used every frame, is built once.
#[test]
fn a_fresh_kernel_every_frame_does_not_grow_the_compute_cache() {
    const FRAMES: usize = 30;
    let _gpu = gpu();
    let mut renderer = renderer();
    renderer.info_mut().auto_reset = false;
    let mut camera = camera();
    let mut empty = Scene::new();

    let out = instanced_array(1, Type::U32);
    let mut built_after_first = 0;
    for frame in 0..FRAMES {
        renderer.compute(&increment(&out)).unwrap();
        renderer.render(&mut empty, &mut camera);
        let (_, _, compute_programs, compute_pipelines) = renderer.program_cache_lens();
        assert_eq!(
            (compute_programs, compute_pipelines),
            (0, 1),
            "frame {frame}: the dropped kernel's entry should be swept, its pipeline kept"
        );
        assert_memory_counts(&renderer);
        if frame == 0 {
            built_after_first = renderer.info().build.pipelines_built;
        }
    }
    assert_eq!(
        renderer.info().build.pipelines_built,
        built_after_first,
        "a kernel rebuilt with the same WGSL every frame recompiled its pipeline"
    );
    assert_eq!(
        renderer.read_storage_buffer_u32(&out).unwrap(),
        vec![FRAMES as u32],
        "every frame's kernel ran"
    );
}

/// A new kernel *shape* every frame — the same buffer written with a new
/// constant — keeps the pipelines of the last few frames, not all of them.
#[test]
fn a_new_kernel_shape_every_frame_keeps_only_recent_pipelines() {
    const FRAMES: usize = 30;
    let _gpu = gpu();
    let mut renderer = renderer();
    let mut camera = camera();
    let mut empty = Scene::new();

    let out = instanced_array(1, Type::U32);
    let mut lens = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let kernel = ComputeFlow::new(vec![out.element(uint(0)).assign(uint(frame as u32 + 1))], 1);
        renderer.compute(&kernel).unwrap();
        drop(kernel);
        renderer.render(&mut empty, &mut camera);
        assert_memory_counts(&renderer);
        lens.push(renderer.program_cache_lens());
    }
    println!(
        "kernel shapes: (programs, pipelines, compute programs, compute pipelines) \
         frame {} {:?}, frame {FRAMES} {:?}",
        2 * GRACE,
        lens[2 * GRACE - 1],
        lens[FRAMES - 1]
    );
    let (_, _, compute_programs, compute_pipelines) = lens[FRAMES - 1];
    assert_eq!(compute_programs, 0);
    assert!(
        compute_pipelines <= GRACE + 1,
        "{FRAMES} kernel shapes left {compute_pipelines} compute pipelines"
    );
    assert_eq!(lens[FRAMES - 1], lens[2 * GRACE - 1]);
    assert_eq!(
        renderer.read_storage_buffer_u32(&out).unwrap(),
        vec![FRAMES as u32]
    );
}

/// A kernel kept alive keeps its pipeline however long it sits idle, so its
/// `onInit` does not run a second time; once it is dropped, both its entries
/// and its `onInit`'s go.
#[test]
fn an_idle_live_kernel_keeps_its_pipeline_and_does_not_reinit() {
    let _gpu = gpu();
    let mut renderer = renderer();
    let mut camera = camera();
    let mut empty = Scene::new();

    let out = instanced_array(1, Type::U32);
    let mut kernel = increment(&out);
    kernel.on_init = Some(Box::new(ComputeFlow::new(
        vec![out.element(uint(0)).assign(uint(100))],
        1,
    )));
    renderer.compute(&kernel).unwrap();
    assert_eq!(renderer.read_storage_buffer_u32(&out).unwrap(), vec![101]);

    for _ in 0..3 * GRACE {
        renderer.render(&mut empty, &mut camera);
    }
    let (_, _, compute_programs, compute_pipelines) = renderer.program_cache_lens();
    assert_eq!(
        (compute_programs, compute_pipelines),
        (2, 2),
        "the live kernel and its onInit keep their entries while idle"
    );

    renderer.compute(&kernel).unwrap();
    assert_eq!(
        renderer.read_storage_buffer_u32(&out).unwrap(),
        vec![102],
        "onInit ran again after the kernel sat idle"
    );

    drop(kernel);
    for _ in 0..GRACE + 2 {
        renderer.render(&mut empty, &mut camera);
    }
    let (_, _, compute_programs, compute_pipelines) = renderer.program_cache_lens();
    assert_eq!((compute_programs, compute_pipelines), (0, 0));
    assert_memory_counts(&renderer);
}

/// A caller that only dispatches and never renders never reaches the
/// per-frame sweep; a `compute()` that misses sweeps the dead entries itself.
#[test]
fn compute_without_render_does_not_grow_the_compute_cache() {
    const KERNELS: usize = 50;
    let _gpu = gpu();
    let mut renderer = renderer();

    let out = instanced_array(1, Type::U32);
    for _ in 0..KERNELS {
        renderer.compute(&increment(&out)).unwrap();
        let (_, _, compute_programs, compute_pipelines) = renderer.program_cache_lens();
        assert_eq!((compute_programs, compute_pipelines), (1, 1));
    }
    assert_eq!(
        renderer.read_storage_buffer_u32(&out).unwrap(),
        vec![KERNELS as u32]
    );
}
