//! The rung's second and decisive gate: what the compute passes actually wrote.
//!
//! The scout's finding about this example is that its graded image is a 2x2
//! block of lit pixels on an otherwise black 400x250 frame, so Three's own
//! comparator passes it at 0.0% whether the simulation runs or the frame is
//! black. Nothing about the compute stage can be gated on pixels. This reads
//! the storage buffers back instead and checks them against the same
//! arithmetic done on the CPU in `f32`.
//!
//! What each test catches:
//!
//! - **`onInit` ran, and ran first.** The particle buffer is created zeroed;
//!   only `precomputeShaderNode` ever puts anything in the velocity buffer. If
//!   the inner submit came second — or never — every particle stays at the
//!   origin and every pixel of the graded frame is still where it was.
//! - **`onInit` ran exactly once.** Running it again on frame two would
//!   overwrite the velocities the update kernel has been flipping.
//! - **The whole dispatch landed.** `ceil( 300000 / 64 ) = 4688` workgroups is
//!   300 032 invocations, 32 more than there are particles; the last real
//!   element is asserted, which a dispatch one workgroup short would miss.

use std::f32::consts::TAU;

use three_rs::nodes::ComputeFlow;

#[path = "../examples/webgpu_compute_points.rs"]
#[allow(dead_code)]
mod example;

use example::PARTICLE_COUNT;

/// `precomputeShaderNode`, done on the CPU in `f32` — the same operations in
/// the same order and the same precision as the kernel.
fn expected_velocity(i: usize) -> (f32, f32) {
    let angle = (i as f32 * 0.005) * TAU;
    let speed = i as f32 * 1e-8 + 1e-7;
    (angle.sin() * speed, angle.cos() * speed)
}

/// `sin` and `cos` are the only operations here whose result is allowed to
/// differ between this CPU and the GPU: WGSL leaves their accuracy to the
/// implementation (WGSL §Floating Point Accuracy gives `sin` 4096 ULP inside
/// ±pi and *no* bound outside it), and this kernel feeds them arguments up to
/// `300000 * 0.005 * TAU`, about 9424 radians, where an `f32` argument is
/// itself only good to ~5e-4 radians. Measured disagreement on this machine is
/// 1.3e-4 of the vector length; the bound below is an order of magnitude over
/// that and still an order of magnitude under anything a wrong kernel would
/// produce.
///
/// Everything else in both kernels is add, multiply, `clamp` and `length`, so
/// every other assertion in this file is exact.
const TRIG: f32 = 2e-3;

#[track_caller]
fn close(got: f32, want: f32, scale: f32, tolerance: f32, what: &str) {
    let tolerance = scale.abs() * tolerance;
    assert!(
        (got - want).abs() <= tolerance,
        "{what}: {got} != {want} (tolerance {tolerance})"
    );
}

/// One `renderer.compute( computeNode )`: `onInit` writes the velocities, then
/// the update kernel moves every particle by one velocity.
#[test]
fn one_frame_of_compute_moves_every_particle() {
    let mut app = example::init();
    app.renderer.compute(&app.particles.update).unwrap();

    let velocity = app
        .renderer
        .read_storage_buffer(&app.particles.velocity)
        .unwrap();
    let particle = app
        .renderer
        .read_storage_buffer(&app.particles.particle)
        .unwrap();
    assert_eq!(velocity.len(), PARTICLE_COUNT * 2);
    assert_eq!(particle.len(), PARTICLE_COUNT * 2);

    // Every hundredth particle plus the two ends — 300 000 × 2 assertions would
    // be a minute of formatting, and a wrong dispatch is never sparse.
    let sampled = (0..PARTICLE_COUNT)
        .step_by(97)
        .chain([PARTICLE_COUNT - 1, PARTICLE_COUNT - 33]);
    for i in sampled {
        let (vx, vy) = expected_velocity(i);
        let (gx, gy) = (velocity[i * 2], velocity[i * 2 + 1]);
        let speed = i as f32 * 1e-8 + 1e-7;

        // The exact half: `speed` is three arithmetic operations, and
        // ( sin, cos ) is a unit vector whatever the implementation, so the
        // length of the velocity is the speed to within rounding. This is what
        // catches a kernel that read the wrong index or the wrong field.
        close(
            gx.hypot(gy),
            speed,
            speed,
            1e-5,
            &format!("|velocity[{i}]|"),
        );

        // The inexact half: which way round the circle it points.
        close(gx, vx, speed, TRIG, &format!("velocity[{i}].x"));
        close(gy, vy, speed, TRIG, &format!("velocity[{i}].y"));

        // `position = particle + velocity` with `particle` still zero, and the
        // clamp to ±1 is inert at this magnitude, so the particle lands on its
        // velocity *bit for bit* — no tolerance. The pointer branch is dead:
        // `pointerVector` sits at ( -10, -10 ) and
        // `length( pointer - position ) <= 0.1` is false everywhere.
        assert_eq!(particle[i * 2], gx, "particle[{i}].x");
        assert_eq!(particle[i * 2 + 1], gy, "particle[{i}].y");
    }

    // Nothing is left at the origin: that is the failure the graded image
    // cannot see.
    let moved = (0..PARTICLE_COUNT)
        .filter(|i| particle[i * 2] != 0.0 || particle[i * 2 + 1] != 0.0)
        .count();
    assert_eq!(moved, PARTICLE_COUNT, "every particle moved");

    // `onInit` recurses through `compute()`, so three counts two calls here and
    // so does this.
    assert_eq!(app.renderer.info().compute.calls, 2);
}

/// Frame two: `onInit` must not run again, and the particles must keep moving
/// by the *same* velocity — `particle[i] == 2 * velocity[i]` after two frames.
#[test]
fn the_precompute_runs_once() {
    let mut app = example::init();
    app.renderer.compute(&app.particles.update).unwrap();
    // `build` is only cleared by `render()`, which this test never calls, so
    // the first frame's two programs and two pipelines are cleared by hand.
    app.renderer.info_mut().reset();
    app.renderer.compute(&app.particles.update).unwrap();

    let velocity = app
        .renderer
        .read_storage_buffer(&app.particles.velocity)
        .unwrap();
    let particle = app
        .renderer
        .read_storage_buffer(&app.particles.particle)
        .unwrap();

    for i in (0..PARTICLE_COUNT).step_by(97) {
        let (vx, vy) = expected_velocity(i);
        let speed = i as f32 * 1e-8 + 1e-7;
        // Still the precompute's velocity, not a second precompute's.
        close(
            velocity[i * 2],
            vx,
            speed,
            TRIG,
            &format!("velocity[{i}].x"),
        );
        close(
            velocity[i * 2 + 1],
            vy,
            speed,
            TRIG,
            &format!("velocity[{i}].y"),
        );

        // Two frames of `position + velocity` from zero is exactly `2 * v`:
        // doubling an `f32` is exact, so this is an equality, and it is the
        // assertion that fails if `onInit` ran again and moved the velocity.
        assert_eq!(particle[i * 2], velocity[i * 2] * 2.0, "particle[{i}].x");
        assert_eq!(
            particle[i * 2 + 1],
            velocity[i * 2 + 1] * 2.0,
            "particle[{i}].y"
        );
    }

    // Three calls, not four: one `onInit` and two updates.
    assert_eq!(app.renderer.info().compute.calls, 3);
    // And the second frame built nothing: no second program, no second
    // pipeline, and — the `onInit` question again from the other side — no
    // second precompute pipeline either.
    assert_eq!(app.renderer.info().build, Default::default());
}

/// The negative control the plan asks for: run the update kernel with no
/// `onInit` at all and the velocities stay zero, so every particle stays at the
/// origin — while the graded image, which only ever sees a 2x2 block of lit
/// pixels, would still be whatever it was. This is the gate proving the
/// readback can tell the two apart.
#[test]
fn without_the_precompute_nothing_moves() {
    let mut app = example::init();
    let update = ComputeFlow {
        on_init: None,
        ..clone_flow(&app.particles.update)
    };
    app.renderer.compute(&update).unwrap();

    let particle = app
        .renderer
        .read_storage_buffer(&app.particles.particle)
        .unwrap();
    assert!(
        particle.iter().all(|&v| v == 0.0),
        "with no onInit the velocities are zero and nothing moves"
    );
    assert_eq!(app.renderer.info().compute.calls, 1);
}

/// `ComputeFlow` is not `Clone` — it is the application's own graph, and
/// cloning one would quietly give it a second cache key. This rebuilds the
/// shallow parts for the negative control above.
fn clone_flow(flow: &ComputeFlow) -> ComputeFlow {
    ComputeFlow {
        statements: flow.statements.clone(),
        count: flow.count,
        workgroup_size: flow.workgroup_size,
        name: flow.name.clone(),
        on_init: None,
    }
}
