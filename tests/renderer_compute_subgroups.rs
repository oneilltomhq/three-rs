//! Subgroup functions on a real device (`docs/nodes.md` §84): every
//! invocation of a kernel writes `subgroupAdd( 1u )` and `subgroupSize`, and
//! the two must agree — the sum of a one from each invocation of a full
//! subgroup is its size.
//!
//! The renderer asks for `wgpu::Features::SUBGROUP` when the adapter has it
//! (`src/renderer/mod.rs` `SUBGROUP_FEATURES`); on one that does not, the
//! test says so and passes, as the renderer logs and skips such a kernel.
//!
//! Also here, because the subgroup kernels lean on it: a barrier kernel has
//! no bounds check, so its tail invocations run (`docs/nodes.md` §84.6).

use three_rs::nodes::tsl::{
    atomic_add, if_then, instance_index, instanced_array, subgroup_add, subgroup_size, uint,
    workgroup_barrier,
};
use three_rs::nodes::{ComputeFlow, Type};
use three_rs::{Renderer, RendererParameters};

#[test]
fn subgroup_add_of_ones_is_the_subgroup_size() {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    if !renderer.features().contains(wgpu::Features::SUBGROUP) {
        eprintln!(
            "renderer_compute_subgroups: skipped, the device has no wgpu::Features::SUBGROUP"
        );
        return;
    }

    // 256 invocations in workgroups of 128, the largest subgroup size the
    // test accepts, so every subgroup is full. The barrier is
    // `webgpu_compute_reduce`'s: it drops the bounds check, so the subgroup
    // calls are in uniform control flow.
    let sums = instanced_array(256, Type::U32);
    let sizes = instanced_array(256, Type::U32);
    let mut flow = ComputeFlow::new(
        vec![
            workgroup_barrier(),
            sums.element(instance_index()).assign(subgroup_add(uint(1))),
            sizes.element(instance_index()).assign(subgroup_size()),
        ],
        256,
    );
    flow.workgroup_size = [128, 1, 1];
    renderer.compute(&flow).unwrap();

    let sums = renderer.read_storage_buffer_u32(&sums).unwrap();
    let sizes = renderer.read_storage_buffer_u32(&sizes).unwrap();
    assert_eq!(sums.len(), 256);
    let size = sizes[0];
    assert!(
        size.is_power_of_two() && (1..=128).contains(&size),
        "subgroupSize {size}"
    );
    assert!(sizes.iter().all(|&s| s == size), "{sizes:?}");
    assert_eq!(sums, sizes, "subgroupAdd( 1u ) per invocation");
}

/// `BarrierNode.setup()` drops the bounds check, and the dispatch is still
/// `ceil( count / workgroupSize )`: `count` 100 in workgroups of 64 runs 128
/// invocations, and the 28 past `count` index past it. A kernel guards its
/// own accesses after the barrier.
#[test]
fn barrier_runs_the_tail() {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");

    let counter = instanced_array(1, Type::U32).to_atomic();
    // 128 elements, so a store past `count` would land and show.
    let out = instanced_array(128, Type::U32);
    let flow = ComputeFlow::new(
        vec![
            workgroup_barrier(),
            atomic_add(counter.element(uint(0)), uint(1)),
            if_then(
                instance_index().less_than(uint(100)),
                vec![out
                    .element(instance_index())
                    .assign(instance_index().add(uint(1)))],
            ),
        ],
        100,
    );
    assert_eq!(flow.workgroup_size, [64, 1, 1]);
    renderer.compute(&flow).unwrap();

    assert_eq!(
        renderer.read_storage_buffer_u32(&counter).unwrap(),
        vec![128],
        "every invocation of both workgroups runs"
    );
    let out = renderer.read_storage_buffer_u32(&out).unwrap();
    let expected: Vec<u32> = (0..128).map(|i| if i < 100 { i + 1 } else { 0 }).collect();
    assert_eq!(out, expected, "the guarded stores stay below count");
}
