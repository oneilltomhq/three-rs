//! The rung's first real gate: the WGSL the node system generates for
//! `webgpu_compute_points` against three.js r186's own dump of the same page.
//!
//! The graded image of this example is a 2x2 block of lit pixels on a black
//! 400x250 frame (the scout's finding): it passes at 0.0% whether the compute
//! stage works or does nothing at all. So the shader text is checked here, and
//! the simulation's arithmetic in `tests/renderer_compute_points.rs`.
//!
//! The dumps in `docs/rung12/` are three's, verbatim. [`canonical`] is the only
//! thing between them and the port's output, and every rule in it is a
//! divergence listed in `docs/nodes.md` §8:
//!
//! - the banner line;
//! - `enable subgroups;` and the `@builtin( subgroup_size )` parameter, which
//!   three emits for TSL subgroup functions this port does not have. `enable
//!   subgroups;` is not in the WGSL dialect wgpu implements, so keeping it
//!   would refuse to compile;
//! - the *numbers* in `NodeBuffer_N` / `nodeUniformN` / `nodeVarN` /
//!   `nodeVaryingN`. Three's come from counters that have already been running
//!   for the whole page; the port's restart at 0 per build. Renumbering both
//!   sides by order of first appearance keeps the check on the thing that
//!   matters — that the same value is referred to in the same places.
//!
//! Nothing else is normalised. Whitespace, statement order, literal spelling
//! and the storage access modes are compared as they are.

use std::collections::HashMap;

use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::{BindingDesc, NodeBuilder};

#[path = "../examples/webgpu_compute_points.rs"]
#[allow(dead_code)]
mod example;

/// Renumber `prefix<digits>` by order of first appearance, and apply the
/// banner / subgroup rules. See the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out = wgsl.replace(
        "// Three.js r186 - Node System",
        "// three-rs - Node System",
    );
    out = out.replace("enable subgroups;\n", "");
    out = out.replace(
        ",\n\t@builtin( subgroup_size ) subgroupSize : u32 ) {",
        " ) {",
    );

    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in ["NodeBuffer_", "nodeUniform", "nodeVarying", "nodeVar"] {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut result = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(at) = rest.find(prefix) {
            result.push_str(&rest[..at]);
            let after = &rest[at + prefix.len()..];
            let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                result.push_str(prefix);
                rest = after;
                continue;
            }
            let next = seen.len();
            let n = *seen.entry(digits.clone()).or_insert(next);
            result.push_str(&format!("{prefix}{n}"));
            rest = &after[digits.len()..];
        }
        result.push_str(rest);
        out = result;
    }
    out
}

#[track_caller]
fn assert_same(generated: &str, three: &str, what: &str) {
    let (a, b) = (canonical(generated), canonical(three));
    if a == b {
        return;
    }
    let mut report = String::new();
    for (i, (left, right)) in a.lines().zip(b.lines()).enumerate() {
        if left != right {
            report.push_str(&format!(
                "line {}:\n  port : {left:?}\n  three: {right:?}\n",
                i + 1
            ));
        }
    }
    if a.lines().count() != b.lines().count() {
        report.push_str(&format!(
            "line counts differ: port {} three {}\n",
            a.lines().count(),
            b.lines().count()
        ));
    }
    panic!("{what}: generated WGSL differs from three.js r186\n{report}");
}

/// `computeNode.onInit` — `precomputeShaderNode`.
#[test]
fn precompute_velocity_matches_three() {
    let particles = example::particles();
    let precompute = particles
        .update
        .on_init
        .as_ref()
        .expect("the update kernel carries the precompute as its onInit");
    let program = NodeBuilder::new().build_compute(precompute);
    assert_same(
        &program.wgsl,
        include_str!("../docs/rung12/precompute_velocity.compute.wgsl"),
        "precompute_velocity",
    );
    // `ceil( 300000 / 64 )`, three's own dispatch for this page.
    assert_eq!(program.dispatch, [4688, 1, 1]);
    assert_eq!(program.workgroup_size, [64, 1, 1]);
}

/// `computeNode` — `Update Particles`, the per-frame kernel.
#[test]
fn update_particles_matches_three() {
    let particles = example::particles();
    let program = NodeBuilder::new().build_compute(&particles.update);
    assert_same(
        &program.wgsl,
        include_str!("../docs/rung12/update_particles.compute.wgsl"),
        "update_particles",
    );
    assert_eq!(program.dispatch, [4688, 1, 1]);
}

/// The render half. Not a whole-file diff: `PointsNodeMaterial.vert-r186.wgsl`
/// and `.frag-r186.wgsl` differ from the port's output only by two divergences
/// that predate this rung and are already in `docs/nodes.md` §8 — three's
/// `VERTEX_` sub-build temps and the order the declarations come out in — plus
/// the binding-number divergence §8 gains here. `examples/dump_wgsl.rs` prints
/// both modules for the eyeball diff against the vendored dumps; what is
/// asserted here is what is *new*, and what would be a silently wrong frame:
///
/// - the storage buffer is `read`, not `read_write`, outside compute
///   (`WGSLNodeBuilder.getNodeAccess()`), and runtime-sized;
/// - one binding, visible to both stages, rather than three's two;
/// - `instanceIndex` reaches the fragment stage as a **flat** `u32` varying.
///   WGSL has no `instance_index` in a fragment entry point at all, so getting
///   this wrong does not compile; getting the `flat` wrong interpolates an
///   index and reads the wrong particle.
#[test]
fn points_material_reads_the_storage_buffer() {
    let material = example::material();
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);

    for (stage, wgsl) in [
        ("vertex", &program.vertex_wgsl),
        ("fragment", &program.fragment_wgsl),
    ] {
        assert!(
            wgsl.contains("\tvalue : array< vec2<f32> >\n"),
            "{stage}: the storage array is runtime-sized, with no element count"
        );
        assert!(
            wgsl.contains("var<storage, read> NodeBuffer_0 : NodeBuffer_0Struct;"),
            "{stage}: the storage buffer is read-only outside the compute stage"
        );
    }

    // `varyings.nodeVarying0 = instanceIndex;` in the vertex stage, read as a
    // flat parameter in the fragment stage.
    assert!(program
        .vertex_wgsl
        .contains("\tvaryings.nodeVarying0 = instanceIndex;\n"));
    assert!(program.fragment_wgsl.contains(
        "fn main( @location( 0 ) @interpolate(flat, either) nodeVarying0 : u32 ) -> OutputStruct {"
    ));
    assert!(
        program
            .fragment_wgsl
            .contains("NodeBuffer_0.value[ nodeVarying0 ]"),
        "the fragment stage indexes the buffer with the varying, not a builtin"
    );

    // One binding for the buffer, not three's two. Group 1 is the object group:
    // storage buffer at 0, the uniform struct at 1.
    let object_group = program.groups.last().expect("an object group");
    let buffers = object_group
        .iter()
        .filter(|desc| matches!(desc, BindingDesc::Buffer { .. }))
        .count();
    assert_eq!(buffers, 1, "one binding for one storage buffer");
    match &object_group[0] {
        BindingDesc::Buffer { visibility, .. } => {
            assert!(visibility.vertex && visibility.fragment && !visibility.compute);
        }
        other => panic!("expected the storage buffer at binding 0, got {other:?}"),
    }
}
