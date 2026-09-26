//! Issue #167's GPU gates: indirect draw and dispatch, atomics and workgroup
//! memory, checked by reading back what the GPU did.
//!
//! `webgpu_struct_drawindirect`'s graded frame is its background (see the
//! example's module comment): the page draws before its kernels fill the
//! draw buffer. The image cannot see the feature, so this file does:
//!
//! - after one `render()`, the two kernels have written `[ 3, 100000, 0, 0,
//!   0 ]` — `time` is 0, so `max( pow( sin( 0 ) + 1, 4 ) * 100000, 100 )` —
//!   through the struct view, including the `atomicStore`;
//! - the first frame drew nothing (a zeroed argument buffer), and the second,
//!   drawn from what the kernels wrote, draws the triangles.
//!
//! The kernels below are small on purpose: each one's answer is known in
//! closed form, and each fails loudly if the feature it exercises is missing
//! (a lost barrier reads zeros, a non-atomic add loses increments, a direct
//! dispatch runs every invocation the bounds check allows).

use three_rs::core::IndirectStorageBufferAttribute;
use three_rs::nodes::tsl::{
    atomic_add, atomic_load, atomic_max, instance_index, instanced_array, invocation_local_index,
    uint, workgroup_array, workgroup_barrier, workgroup_id,
};
use three_rs::nodes::{ComputeFlow, Type};
use three_rs::{Renderer, RendererParameters};

#[path = "../examples/webgpu_struct_drawindirect.rs"]
#[allow(dead_code)]
mod example;

fn kernel(statements: Vec<three_rs::nodes::NodeRef>, count: usize) -> ComputeFlow {
    ComputeFlow {
        statements,
        count,
        workgroup_size: [64, 1, 1],
        name: None,
        on_init: None,
    }
}

/// The page's own frame loop, twice, with the draw buffer read between.
#[test]
fn the_kernels_write_the_draw_arguments_the_next_frame_draws() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = example::init();

    example::animate(&mut app);
    let (_, _, first) = app.renderer.read_canvas_pixels().unwrap();

    let args = app
        .renderer
        .read_indirect_buffer(&app.kernels.draw_buffer)
        .unwrap();
    assert_eq!(args, vec![3, example::INSTANCES as u32, 0, 0, 0]);

    // Frame one drew from the zeroed buffer: every pixel is the background.
    let background = first[..4].to_vec();
    assert!(
        first.chunks(4).all(|px| px == background.as_slice()),
        "frame one drew something from an all-zero indirect buffer"
    );

    example::animate(&mut app);
    let (_, _, second) = app.renderer.read_canvas_pixels().unwrap();
    let lit = second
        .chunks(4)
        .filter(|px| *px != background.as_slice())
        .count();
    // 100 000 small triangles inside a unit cube seen from ( 1, 1, 1 ) cover
    // a large part of the 800 x 500 frame.
    assert!(
        lit > 800 * 500 / 10,
        "frame two drew {lit} pixels from the kernels' arguments"
    );
}

#[test]
fn atomics_workgroup_memory_and_indirect_dispatch() {
    let mut renderer =
        Renderer::new(RendererParameters { antialias: false }).expect("a wgpu adapter and device");

    // 256 invocations, one `atomicAdd( counter, 1 )` each, and an
    // `atomicMax` of the index: without atomics the adds race and lose.
    let counters = instanced_array(2, Type::U32).to_atomic();
    let count = kernel(
        vec![
            atomic_add(counters.element(uint(0)), uint(1)),
            atomic_max(counters.element(uint(1)), instance_index()),
        ],
        256,
    );
    renderer.compute(&count).unwrap();
    let read = |renderer: &mut Renderer, array: &three_rs::nodes::tsl::StorageArray| {
        renderer.read_storage_buffer_u32(array).unwrap()
    };
    assert_eq!(read(&mut renderer, &counters), vec![256, 255]);

    // `atomicLoad` read as a value, into a plain array.
    let seen = instanced_array(64, Type::U32);
    renderer
        .compute(&kernel(
            vec![seen
                .element(instance_index())
                .assign(atomic_load(counters.element(uint(0))))],
            64,
        ))
        .unwrap();
    assert!(read(&mut renderer, &seen).iter().all(|&v| v == 256));

    // Workgroup memory and the barrier: each invocation writes its own slot
    // of a 64-element `workgroupArray`, waits, then reads the mirrored slot —
    // written by a *different* invocation, so without the barrier (or with
    // per-invocation memory) this reads zeros or garbage.
    let input = instanced_array(256, Type::U32);
    renderer
        .compute(&kernel(
            vec![input
                .element(instance_index())
                .assign(instance_index().mul(uint(3)).add(uint(1)))],
            256,
        ))
        .unwrap();
    let mirrored = instanced_array(256, Type::U32);
    let shared = workgroup_array(Type::U32, 64);
    renderer
        .compute(&kernel(
            vec![
                shared
                    .element(invocation_local_index())
                    .assign(input.element(instance_index())),
                workgroup_barrier(),
                mirrored
                    .element(instance_index())
                    .assign(shared.element(uint(63).sub(invocation_local_index()))),
            ],
            256,
        ))
        .unwrap();
    let want: Vec<u32> = (0..256u32)
        .map(|i| {
            let source = (i / 64) * 64 + (63 - i % 64);
            source * 3 + 1
        })
        .collect();
    assert_eq!(read(&mut renderer, &mirrored), want);

    // An atomic workgroup array: every invocation adds its input into one
    // shared counter; after the barrier each reads its workgroup's total.
    let totals = instanced_array(256, Type::U32);
    let sums = instanced_array(4, Type::U32);
    let shared_sum = workgroup_array(Type::U32, 1).to_atomic();
    renderer
        .compute(&kernel(
            vec![
                atomic_add(shared_sum.element(uint(0)), input.element(instance_index())),
                workgroup_barrier(),
                totals
                    .element(instance_index())
                    .assign(atomic_load(shared_sum.element(uint(0)))),
                sums.element(workgroup_id().x())
                    .assign(atomic_load(shared_sum.element(uint(0)))),
            ],
            256,
        ))
        .unwrap();
    let group_sum = |g: u32| (g * 64..g * 64 + 64).map(|i| i * 3 + 1).sum::<u32>();
    let totals = read(&mut renderer, &totals);
    for (i, total) in totals.iter().enumerate() {
        assert_eq!(*total, group_sum(i as u32 / 64), "invocation {i}");
    }
    assert_eq!(
        read(&mut renderer, &sums),
        (0..4).map(group_sum).collect::<Vec<_>>()
    );

    // `renderer.compute( node, indirectAttribute )`: the same 256-invocation
    // kernel dispatched with `[ 2, 1, 1 ]` workgroups read from the GPU runs
    // 128 invocations, not the `ceil( 256 / 64 ) = 4` workgroups a direct
    // dispatch would.
    let dispatch = IndirectStorageBufferAttribute::new(vec![2, 1, 1], 3);
    let tally = instanced_array(1, Type::U32).to_atomic();
    renderer
        .compute_indirect(
            &kernel(vec![atomic_add(tally.element(uint(0)), uint(1))], 256),
            &dispatch,
        )
        .unwrap();
    assert_eq!(read(&mut renderer, &tally), vec![128]);
}

#[path = "../examples/webgpu_particles.rs"]
#[allow(dead_code)]
mod particles;

/// `webgpu_particles`' graded frame is the grid alone: at time 0 every
/// sprite's life is at its end and its opacity zero (see the example). A
/// later time is where the feature shows, so this frame is taken there:
/// the fire, drawn through `drawIndexedIndirect` from the five words the page
/// wrote, adds orange to the frame, and the smoke darkens it.
#[test]
fn the_particles_draw_through_the_indirect_buffer() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = particles::init();
    particles::animate(&mut app);
    let (_, _, grid) = app.renderer.read_canvas_pixels().unwrap();

    // `[ indexCount, instanceCount, firstIndex, baseVertex, firstInstance ]`
    // for a one-quad plane, uploaded as the page wrote it.
    assert_eq!(
        app.renderer
            .read_indirect_buffer(&app.fire_indirect)
            .unwrap(),
        vec![6, particles::FIRE_COUNT as u32, 0, 0, 0]
    );

    three_rs::testing::pin_time(Some(1500.0));
    particles::animate(&mut app);
    let (_, _, later) = app.renderer.read_canvas_pixels().unwrap();

    let changed = grid
        .chunks(4)
        .zip(later.chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    let orange = later
        .chunks(4)
        .filter(|px| px[0] as i32 > px[2] as i32 + 60 && px[0] > 120)
        .count();
    let grid_orange = grid
        .chunks(4)
        .filter(|px| px[0] as i32 > px[2] as i32 + 60 && px[0] > 120)
        .count();
    println!("changed {changed}, orange {orange} (grid frame {grid_orange})");
    assert_eq!(grid_orange, 0, "the time-0 frame has a visible sprite");
    assert!(
        changed > 800 * 500 / 10,
        "the sprites changed {changed} pixels"
    );
    assert!(orange > 1000, "the fire lit {orange} pixels");
}
