//! Subgroup functions on a real device (`docs/nodes.md` §84): every
//! invocation of a kernel writes `subgroupAdd( 1u )` and `subgroupSize`, and
//! the two must agree — the sum of a one from each invocation of a full
//! subgroup is its size.
//!
//! The renderer asks for `wgpu::Features::SUBGROUP` when the adapter has it
//! (`src/renderer/mod.rs` `SUBGROUP_FEATURES`); on one that does not, the
//! test says so and passes, as the renderer logs and skips such a kernel.

use three_rs::nodes::tsl::{
    instance_index, instanced_array, subgroup_add, subgroup_size, uint, workgroup_barrier,
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

    // 256 invocations in workgroups of 64, every subgroup full. The barrier
    // is `webgpu_compute_reduce`'s: it drops the bounds check, so the
    // subgroup calls are in uniform control flow.
    let sums = instanced_array(256, Type::U32);
    let sizes = instanced_array(256, Type::U32);
    let flow = ComputeFlow::new(
        vec![
            workgroup_barrier(),
            sums.element(instance_index()).assign(subgroup_add(uint(1))),
            sizes.element(instance_index()).assign(subgroup_size()),
        ],
        256,
    );
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
