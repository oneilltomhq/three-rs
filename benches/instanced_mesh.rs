//! Frame time of one big `InstancedMesh`, the shape of a `BatchedText` sized to
//! `maxGlyphCount = 1 << 17` (issue #89).
//!
//! `cargo bench --bench instanced_mesh`. Needs a GPU adapter, as the
//! `gpu_only` tests do. Two runs of 50 frames each: one where nothing moves,
//! which should cost no buffer traffic at all, and one where a single instance
//! moves every frame, which costs one write of the instance matrix.
//!
//! The time is `render()` to the GPU finishing the frame, so it counts the
//! CPU-side buffer creation and the copy the issue measured.

use std::rc::Rc;
use std::time::{Duration, Instant};

use three_rs::geometries::plane_geometry;
use three_rs::math::Matrix4;
use three_rs::{
    Color, InstancedMesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Vector3,
};

const INSTANCES: usize = 1 << 17;
const FRAMES: usize = 50;
const SIDE: usize = 512;

fn main() {
    let mut camera = PerspectiveCamera::new(60.0, 1.6, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 10.0);
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::new(1.0, 1.0, 1.0);
    let mesh = InstancedMesh::new(
        Rc::new(plane_geometry(0.01, 0.01, 1, 1)),
        material,
        INSTANCES,
    );

    let mut matrix = Matrix4::default();
    for i in 0..INSTANCES {
        matrix.make_translation(place(i % SIDE), place(i / SIDE) * 0.5, 0.0);
        mesh.borrow_mut().set_matrix_at(i, &matrix);
    }
    scene.add(&mesh);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(1600.0, 1000.0);

    // Warm up: build the program and the pipeline, upload the geometry.
    for _ in 0..3 {
        frame(&mut renderer, &mut scene, &mut camera);
    }

    let still = (0..FRAMES)
        .map(|_| frame(&mut renderer, &mut scene, &mut camera))
        .sum::<Duration>();
    report("still", still);

    let moving = (0..FRAMES)
        .map(|f| {
            matrix.make_translation(place(f % SIDE), 4.0, 0.0);
            mesh.borrow_mut().set_matrix_at(0, &matrix);
            frame(&mut renderer, &mut scene, &mut camera)
        })
        .sum::<Duration>();
    report("one instance moving", moving);
}

/// A grid coordinate in -8..8.
fn place(i: usize) -> f64 {
    -8.0 + 16.0 * i as f64 / SIDE as f64
}

fn frame(renderer: &mut Renderer, scene: &mut Scene, camera: &mut PerspectiveCamera) -> Duration {
    let started = Instant::now();
    renderer.render(scene, camera);
    renderer
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    started.elapsed()
}

fn report(label: &str, total: Duration) {
    println!(
        "{INSTANCES} instances, {label}: {:.2} ms per frame over {FRAMES} frames",
        total.as_secs_f64() * 1000.0 / FRAMES as f64
    );
}
