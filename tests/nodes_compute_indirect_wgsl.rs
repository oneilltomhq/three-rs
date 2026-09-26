//! Issue #167's WGSL gates: the struct storage buffer and `atomicStore` of
//! `webgpu_struct_drawindirect` against three's own dump of the page, and the
//! workgroup-memory shapes of `webgpu_compute_reduce` against the lines three
//! writes for them.
//!
//! The two fixtures in `tests/fixtures/webgpu_struct_drawindirect/` are three's
//! modules verbatim (`tools/dump-webgpu.mjs`, three.js at 5f610f5). As in
//! `tests/nodes_compute_wgsl.rs`, [`canonical`] is the only thing between them
//! and the port's output, and each rule is a divergence in `docs/nodes.md` §8:
//! the banner, three's `enable subgroups;` / `subgroup_size` parameter, and the
//! numbers of names that come from counters three has been running for the
//! whole page.
//!
//! `webgpu_compute_reduce` is not a rung (see
//! `docs/webgpu_struct_drawindirect-progress.md`): every one of its kernels
//! that touches workgroup memory also uses subgroup builtins, which issue #167
//! leaves out. So its shapes are checked line by line here rather than as a
//! whole module, and the arithmetic in `tests/renderer_compute_indirect.rs`.

use std::collections::HashMap;

use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::tsl::{
    atomic_add, atomic_load, instance_index, instanced_array, invocation_local_index, uint,
    workgroup_array, workgroup_barrier, workgroup_id,
};
use three_rs::nodes::{BindingDesc, ComputeFlow, NodeBuilder, Type};

#[path = "../examples/webgpu_struct_drawindirect.rs"]
#[allow(dead_code)]
mod example;

/// Renumber `prefix<digits>` by order of first appearance and drop three's
/// subgroup scaffolding. See the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out = wgsl.replace(
        "// Three.js r187dev - Node System",
        "// three-rs - Node System",
    );
    out = out.replace("enable subgroups;\n", "");
    out = out.replace(
        ",\n\t@builtin( subgroup_size ) subgroupSize : u32 ) {",
        " ) {",
    );

    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in [
        "NodeBuffer_",
        "WorkgroupArray_",
        "nodeUniform",
        "nodeVarying",
        "nodeVar",
        "nodeConst",
    ] {
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
    panic!("{what}: generated WGSL differs from three.js\n{generated}\n{report}");
}

/// `computeInitDrawBuffer` — the struct declared under `// structs`, the
/// storage binding on one line, a plain member store and an `atomicStore`.
#[test]
fn init_draw_buffer_matches_three() {
    let kernels = example::draw_kernels();
    let program = NodeBuilder::new().build_compute(&kernels.init);
    assert_same(
        &program.wgsl,
        include_str!("fixtures/webgpu_struct_drawindirect/init_draw_buffer.compute.wgsl"),
        "computeInitDrawBuffer",
    );
    assert_eq!(program.dispatch, [1, 1, 1]);
}

/// `computeDrawBuffer` — `time` in the render group, the named
/// `var<private> instanceCount`, and the `u32( … )` a float needs on its way
/// into an `atomic< u32 >`.
#[test]
fn draw_buffer_matches_three() {
    let kernels = example::draw_kernels();
    let program = NodeBuilder::new().build_compute(&kernels.draw);
    assert_same(
        &program.wgsl,
        include_str!("fixtures/webgpu_struct_drawindirect/draw_buffer.compute.wgsl"),
        "computeDrawBuffer",
    );
    // `ceil( 100000 / 64 )`.
    assert_eq!(program.dispatch, [1563, 1, 1]);
}

/// Both kernels bind the attribute's one buffer: the storage binding carries
/// the `IndirectStorageBufferAttribute`'s id, which is what the renderer keys
/// the GPU buffer on and what `geometry.setIndirect()` draws from.
#[test]
fn both_kernels_bind_the_indirect_attribute() {
    let kernels = example::draw_kernels();
    let id = kernels.draw_buffer.id().get();
    for flow in [&kernels.init, &kernels.draw] {
        let program = NodeBuilder::new().build_compute(flow);
        let ids: Vec<usize> = program
            .groups
            .iter()
            .flatten()
            .filter_map(|desc| match desc {
                BindingDesc::Buffer { id, .. } => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(ids, vec![id]);
    }
}

/// The render half: not a whole-file diff, for the same reason
/// `tests/nodes_compute_wgsl.rs` gives for `PointsNodeMaterial` — three's
/// `VERTEX_` sub-build temps and declaration order are standing divergences
/// (§8). What is checked is what this page adds: five geometry attributes at
/// locations 0–4 in first-use order, the two `varyingProperty`s written in the
/// vertex flow and read as fragment parameters, and the fragment node's
/// `addAssign` on a swizzle of a var.
#[test]
fn render_material_reads_the_instanced_attributes() {
    let material = example::material();
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let vertex = &program.vertex_wgsl;
    let fragment = &program.fragment_wgsl;

    assert!(
        vertex.contains(
            "fn main( @location( 0 ) position : vec3<f32>,\n\
             \t@location( 1 ) offset : vec3<f32>,\n\
             \t@location( 2 ) orientationStart : vec4<f32>,\n\
             \t@location( 3 ) orientationEnd : vec4<f32>,\n\
             \t@location( 4 ) color : vec4<f32> ) -> VaryingsStruct {"
        ),
        "{vertex}"
    );
    assert!(
        vertex.contains(
            "\t@location( 0 ) vPosition : vec3<f32>,\n\t@location( 1 ) vColor : vec4<f32>,"
        ),
        "{vertex}"
    );
    assert!(vertex.contains("\tvaryings.vColor = color;\n"), "{vertex}");
    assert!(
        vertex.contains("\tpositionLocal = varyings.vPosition;\n"),
        "{vertex}"
    );
    assert!(
        fragment.contains(
            "fn main( @location( 0 ) vPosition : vec3<f32>,\n\t@location( 1 ) vColor : vec4<f32> ) -> OutputStruct {"
        ),
        "{fragment}"
    );
    let fragment = &canonical(fragment);
    assert!(
        fragment.contains("\tnodeVar0 = vColor;\n\tnodeVar0.x = ( nodeVar0.x + ( sin( ( ( vPosition.x * 10.0 ) + render.nodeUniform"),
        "{fragment}"
    );

    // With the attributes marked per-instance, only `position` steps per
    // vertex.
    let program = program.with_instanced_attributes(&[
        "offset".to_string(),
        "color".to_string(),
        "orientationStart".to_string(),
        "orientationEnd".to_string(),
    ]);
    let steps: Vec<bool> = program
        .vertex_buffers()
        .iter()
        .map(|desc| desc.instanced)
        .collect();
    assert_eq!(steps, vec![false, true, true, true, true]);
}

/// A kernel that sums a storage array one workgroup at a time: the shapes of
/// `webgpu_compute_reduce`'s workgroup kernels, without their subgroup calls.
fn workgroup_sum() -> (ComputeFlow, three_rs::nodes::tsl::StorageArray) {
    let input = instanced_array(256, Type::U32);
    let sums = instanced_array(4, Type::U32);
    let shared = workgroup_array(Type::U32, 64);
    let flow = ComputeFlow {
        statements: vec![
            shared
                .element(invocation_local_index())
                .assign(input.element(instance_index())),
            workgroup_barrier(),
            sums.element(workgroup_id().x())
                .assign(shared.element(uint(0))),
        ],
        count: 256,
        workgroup_size: [64, 1, 1],
        name: None,
        on_init: None,
    };
    (flow, input)
}

/// `var<workgroup>` under `// locals`, `local_invocation_index` first among
/// the entry point's parameters, and the barrier — each spelled exactly as
/// `webgpu_compute_reduce`'s dump (three's `m07`) spells it.
#[test]
fn workgroup_memory_matches_three_spelling() {
    let (flow, _) = workgroup_sum();
    let wgsl = NodeBuilder::new().build_compute(&flow).wgsl;
    assert!(
        wgsl.contains(
            "// locals\nvar<workgroup> WorkgroupArray_0: array< u32, 64 >;\n\n// structs"
        ),
        "{wgsl}"
    );
    assert!(
        wgsl.contains(
            "fn main( @builtin( local_invocation_index ) invocationLocalIndex : u32,\n\
             \t@builtin( global_invocation_id ) globalId : vec3<u32>,"
        ),
        "{wgsl}"
    );
    assert!(
        wgsl.contains("\tWorkgroupArray_0[ invocationLocalIndex ] = NodeBuffer_0.value[ instanceIndex ];\n\tworkgroupBarrier();\n"),
        "{wgsl}"
    );
    assert!(
        wgsl.contains("\tNodeBuffer_1.value[ workgroupId.x ] = WorkgroupArray_0[ 0u ];\n"),
        "{wgsl}"
    );
}

/// `instancedArray( n, 'uint' ).toAtomic()`: `array< atomic<u32> >` (no
/// spaces inside the `atomic< >`, unlike a struct member's), a bare
/// `atomicAdd` statement, and an `atomicLoad` whose value is read, which three
/// holds in a `let`.
#[test]
fn atomic_array_matches_three_spelling() {
    let counter = instanced_array(1, Type::U32).to_atomic();
    let out = instanced_array(64, Type::U32);
    let flow = ComputeFlow {
        statements: vec![
            atomic_add(counter.element(uint(0)), uint(1)),
            out.element(instance_index())
                .assign(atomic_load(counter.element(uint(0))).add(uint(0))),
        ],
        count: 64,
        workgroup_size: [64, 1, 1],
        name: None,
        on_init: None,
    };
    let wgsl = NodeBuilder::new().build_compute(&flow).wgsl;
    assert!(wgsl.contains("\tvalue : array< atomic<u32> >\n"), "{wgsl}");
    assert!(
        wgsl.contains("\tatomicAdd( &NodeBuffer_0.value[ 0u ], 1u );\n"),
        "{wgsl}"
    );
    assert!(
        wgsl.contains(
            "\tlet nodeConst0 = atomicLoad( &NodeBuffer_0.value[ 0u ] );\n\tNodeBuffer_1.value[ instanceIndex ] = ( nodeConst0 + 0u );\n"
        ),
        "{wgsl}"
    );
}

#[path = "../examples/webgpu_particles.rs"]
#[allow(dead_code)]
mod particles;

/// `webgpu_particles`' smoke sprite, against three's dump of the same page
/// (its `m03` / `m04`). Structural, for the reasons
/// [`render_material_reads_the_instanced_attributes`] gives; what is checked
/// is what the rung adds: the `range()` buffers are uniform arrays of 2000
/// `vec4`s read by `instanceIndex` in the vertex stage and by the flat
/// varying in the fragment stage, `lifeTime`'s `mod( 1 )` is the
/// `tsl_mod_float` helper, and `rotateUV()` is `rotate( uv - center ) +
/// center` with `RotateNode`'s `mat2x2`.
#[test]
fn smoke_sprite_matches_three_spelling() {
    let (smoke, _) = particles::materials(&three_rs::textures::Texture::new(1, 1, None));
    let ctx = SetupContext::default();
    let program = NodeBuilder::new().build(&setup(&smoke, &ctx, None));
    let vertex = canonical(&program.vertex_wgsl);
    let fragment = canonical(&program.fragment_wgsl);
    for wgsl in [&vertex, &fragment] {
        assert!(
            wgsl.contains(
                "fn tsl_mod_float( x : f32, y : f32 ) -> f32 { return x - y * floor( x / y ); }"
            ),
            "{wgsl}"
        );
        assert!(
            wgsl.contains("\tvalue : array< vec4<f32>, 2000 >\n"),
            "{wgsl}"
        );
    }
    // Three writes `positionNode`'s value into the varying the fragment
    // stage's colour reads; the port writes the var that holds it.
    let moved = vertex
        .find("\tpositionLocal = nodeVar")
        .expect("positionNode is assigned to positionLocal");
    let written = vertex
        .find("\tvaryings.positionLocal = positionLocal;\n")
        .unwrap_or_else(|| panic!("the varying carries the moved position\n{vertex}"));
    assert!(moved < written, "{vertex}");
    assert!(
        vertex.contains("@location( 1 ) @interpolate(flat, either) nodeVarying0 : u32"),
        "{vertex}"
    );
    assert!(
        vertex.contains(
            "( mat2x2<f32>( nodeVar6, nodeVar7, ( - nodeVar7 ), nodeVar6 ) * ( position.xy * "
        ),
        "{vertex}"
    );
    assert!(
        fragment.contains("NodeBuffer_0.value[ nodeVarying0 ].x"),
        "{fragment}"
    );
    assert!(
        fragment.contains(
            " * ( nodeVarying2 - vec2<f32>( 0.5, 0.5 ) ) ) + vec2<f32>( 0.5, 0.5 ) ) );\n"
        ),
        "{fragment}"
    );
    assert!(
        fragment.contains(
            "\tDiffuseColor.w = ( DiffuseColor.w * ( nodeVar6.w * ( 1.0 - nodeVar2 ) ) );\n"
        ),
        "{fragment}"
    );
}
