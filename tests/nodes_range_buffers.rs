//! The four `range()` buffers `webgpu_tsl_galaxy` uploads, pinned against the
//! ones three.js actually writes for that page.
//!
//! Three fills a `RangeNode`'s buffer once, inside `RangeNode.setup()`, from the
//! page's `Math.random` — so *which* buffer is filled first is decided by the
//! order the node graph is set up in, and a single misplaced draw shifts every
//! later buffer by one. The port fills them lazily instead, in the order
//! `NodeProgram::vertex_buffers()` reports, which is attribute-allocation order,
//! which is generate order. This test drives that order through the real
//! `fill_range()` and checks the head of each buffer, so the two orders cannot
//! drift apart silently.
//!
//! The expected numbers are the heads of the four `Float32Array`s three.js
//! uploads for the page, read off the dumps in
//! `handoff/scouts/rung13/PLAN.md` §2.4. No GPU is needed.

use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::builder::VertexBufferSource;
use three_rs::nodes::node::BufferSource;
use three_rs::nodes::NodeBuilder;
use three_rs::renderer::fill_range;
use three_rs::testing::DeterministicRandom;

#[path = "../examples/webgpu_tsl_galaxy.rs"]
#[allow(dead_code)] // the example's own `main()` / `init()` are unused here
mod webgpu_tsl_galaxy;

/// `new Inspector()`'s draws, which the example applies before the frame.
const PRNG_OFFSET: usize = 5;

#[test]
fn galaxy_range_buffers() {
    let material = webgpu_tsl_galaxy::galaxy_material();
    let flow = setup(
        &material,
        &SetupContext {
            instance_count: Some(webgpu_tsl_galaxy::COUNT),
            instanced: true,
            light_count: 0,
        },
        None,
    );
    let program = NodeBuilder::new().build(&flow);

    let mut random = DeterministicRandom::new();
    random.skip(PRNG_OFFSET);

    // `Renderer::draw()` walks the descs in slot order and fills each one it has
    // not seen; the instance matrix comes from `InstancedMesh`, not from
    // `Math.random`, so it consumes nothing.
    let mut filled: Vec<([f64; 4], [f64; 4], [f32; 4])> = Vec::new();
    for desc in program.vertex_buffers() {
        let VertexBufferSource::Instance(buffer) = &desc.source else {
            continue;
        };
        let BufferSource::Range { min, max } = &buffer.source else {
            continue;
        };
        let data = fill_range(&mut random, *min, *max, buffer.count);
        filled.push((*min, *max, [data[0], data[1], data[2], data[3]]));
    }

    // `range( 0, branches )`, `range( 0, 1 )` (radiusRatio),
    // `range( vec3( -1 ), vec3( 1 ) )`, `range( 0, 1 )` (scaleNode) — in that
    // order, which is the order `NodeBuilder` reaches them in: the `positionNode`
    // chain first (branchAngle, then radiusRatio inside `angle`, then the offset),
    // and `scaleNode` last, because it is only reached through
    // `setupPositionView()` in the vertex flow.
    let expected: [([f64; 4], [f64; 4], [f32; 4]); 4] = [
        (
            [0.0; 4],
            [3.0; 4],
            [0.527_929_45, 1.989_817_1, 1.472_265, 2.935_939_2],
        ),
        (
            [0.0; 4],
            [1.0; 4],
            [0.781_111_16, 0.943_676_8, 0.369_732_4, 0.372_152_74],
        ),
        (
            [-1.0, -1.0, -1.0, 0.0],
            [1.0, 1.0, 1.0, 0.0],
            [0.231_912_13, 0.736_554_94, 0.071_700_86, 0.0],
        ),
        (
            [0.0; 4],
            [1.0; 4],
            [0.616_448_8, 0.782_27, 0.002_948_711_7, 0.231_343],
        ),
    ];

    assert_eq!(filled.len(), expected.len(), "four range() buffers");

    for (i, (want, got)) in expected.iter().zip(filled.iter()).enumerate() {
        assert_eq!(got.0, want.0, "buffer {i} min");
        assert_eq!(got.1, want.1, "buffer {i} max");
        for c in 0..4 {
            assert!(
                (got.2[c] - want.2[c]).abs() < 1e-6,
                "buffer {i} component {c}: {} != {}",
                got.2[c],
                want.2[c]
            );
        }
    }
}

/// `range( vec3( -1 ), vec3( 1 ) )`'s fourth component is min = max = 0, not 1:
/// `RangeNode.setup()` only fills `w` with 1 for a `Color`. The draw is still
/// consumed, which is why the next buffer starts where it does.
#[test]
fn vec3_range_has_zero_w() {
    let mut random = DeterministicRandom::new();
    let data = fill_range(&mut random, [-1.0, -1.0, -1.0, 0.0], [1.0, 1.0, 1.0, 0.0], 2);
    assert_eq!(data[3], 0.0);
    assert_eq!(data[7], 0.0);
    // Eight draws for two instances, even though two of them landed on a
    // constant: the sequence is exactly eight further on.
    let mut control = DeterministicRandom::new();
    control.skip(8);
    assert_eq!(random.next(), control.next());
}
