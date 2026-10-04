//! The WGSL gate for `ClippingNode`'s three modes, against three.js' own
//! dumps at 5f610f5 (verbatim output of `tools/dump-webgpu.mjs`):
//!
//! - `clipping()` and the software `clippingAlpha()` from
//!   `tests/fixtures/clipping_discard/`, a page of the port's own
//!   (`tools/dump-pages/clipping_discard.html`) that hides the
//!   `clip-distances` feature from three so its union plane is discarded in
//!   the fragment stage: one union plane and two intersection planes on a
//!   `MeshBasicNodeMaterial`, without (`m00`/`m01`) and with (`m02`/`m03`)
//!   `alphaToCoverage`;
//! - `hardwareClipping()` with `clippingAlpha()` from
//!   `tests/fixtures/webgpu_clipping/` — the example's knot, whose global
//!   plane goes to `@builtin( clip_distances )` and whose two intersection
//!   planes fade the alpha (`m00`/`m01`).
//!
//! The tests compare selected sections of the two texts, not whole shaders:
//! the fragment flow through the clipping statements, the fragment `// vars`
//! block, the `NodeBuffer` plane declarations of each stage, and in the
//! vertex stage the directives, the varyings, and the flow (or, for the knot,
//! the hardware-clipping loop). Within a section, [`canonical`] is the only
//! thing between them, after three's `VERTEX_` sub-build is undone in the
//! vertex flow:
//!
//! - lines that are empty or only whitespace, and the `// directives` header
//!   (its *contents* are compared);
//! - the numbers in `nodeUniformN` / `nodeVarN` / `nodeConstN` and
//!   `NodeBuffer_N`, renumbered on both sides by order of first appearance;
//! - the binding numbers of the render group: three numbers group 0 in the
//!   order the bindings are created, fragment stage first, so the fragment
//!   planes come before `render` and the vertex stage's hardware-clipping
//!   planes after it; the port's `render` struct is always binding 0. The
//!   layout and the shader come from the same descriptors (`docs/nodes.md`
//!   §8, the binding-index class).
//!
//! The knot is compared only where clipping writes: three's dump is lit by
//! the page's lights and the port's material here is not.

use std::collections::HashMap;
use std::rc::Rc;

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::builder::with_alpha_to_coverage_samples;
use three_rs::nodes::clipping::ClippingContext;
use three_rs::nodes::{NodeBuilder, NodeProgram};

const DISCARD_VERTEX: &str = include_str!("fixtures/clipping_discard/m00_vertex_vertex.wgsl");
const DISCARD_FRAGMENT: &str = include_str!("fixtures/clipping_discard/m01_fragment_fragment.wgsl");
const ALPHA_VERTEX: &str = include_str!("fixtures/clipping_discard/m02_vertex_vertex.wgsl");
const ALPHA_FRAGMENT: &str = include_str!("fixtures/clipping_discard/m03_fragment_fragment.wgsl");
const KNOT_VERTEX: &str = include_str!("fixtures/webgpu_clipping/m00_vertex_vertex.wgsl");
const KNOT_FRAGMENT: &str = include_str!("fixtures/webgpu_clipping/m01_fragment_fragment.wgsl");

/// Renumber every `{prefix}N` in `wgsl` by order of first appearance.
fn renumber(wgsl: &str, prefix: &str) -> String {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut result = String::with_capacity(wgsl.len());
    let mut rest = wgsl;
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
    result
}

/// Apply the rules in the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out: String = wgsl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| *line != "// directives")
        .map(|line| {
            if line.starts_with("@binding( ") && line.ends_with(" @group( 0 )") {
                "@binding( _ ) @group( 0 )"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in [
        "nodeUniform",
        "nodeVarying",
        "nodeVar",
        "nodeConst",
        "NodeBuffer_",
    ] {
        out = renumber(&out, prefix);
    }
    out
}

/// The lines of canonical `wgsl` from the one starting with `from` up to and
/// including the first one after it starting with `to`.
fn section(wgsl: &str, from: &str, to: &str) -> String {
    let wgsl = canonical(wgsl);
    let lines: Vec<&str> = wgsl.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with(from))
        .unwrap_or_else(|| panic!("no line starting {from:?} in\n{wgsl}"));
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.trim_start().starts_with(to))
            .unwrap_or_else(|| panic!("no line starting {to:?} after {from:?} in\n{wgsl}"));
    lines[start..=end].join("\n")
}

/// The fragment flow from `from` through the closing brace of
/// `clippingAlpha()`'s `diffuseColor.a.equal( 0 ).discard()`.
fn through_alpha_discard(wgsl: &str, from: &str) -> String {
    let block = section(wgsl, from, "if ( ( DiffuseColor.w == 0.0 ) )");
    format!("{block}\n\t\tdiscard;\n\t}}")
}

/// Every `struct NodeBuffer_N…` declaration of canonical `wgsl`, through
/// its `var<uniform>` line.
fn plane_buffers(wgsl: &str) -> String {
    let wgsl = canonical(wgsl);
    let lines: Vec<&str> = wgsl.lines().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(start) = lines[at..]
        .iter()
        .position(|l| l.starts_with("struct NodeBuffer_"))
    {
        let start = at + start;
        out.extend_from_slice(&lines[start..start + 5]);
        at = start + 5;
    }
    out.join("\n")
}

/// `canonical( three )` restricted to the same `section` call; panics with a
/// line-by-line report.
#[track_caller]
fn assert_same(port: &str, three: &str, what: &str) {
    if port == three {
        return;
    }
    let mut report = String::new();
    for (i, (left, right)) in port.lines().zip(three.lines()).enumerate() {
        if left != right {
            report.push_str(&format!(
                "line {}:\n  port : {left:?}\n  three: {right:?}\n",
                i + 1
            ));
        }
    }
    if port.lines().count() != three.lines().count() {
        report.push_str(&format!(
            "line counts differ: port {} three {}\n",
            port.lines().count(),
            three.lines().count()
        ));
    }
    panic!("{what}: generated WGSL differs from three.js 5f610f5\n{report}\n--- port ---\n{port}\n--- three ---\n{three}\n");
}

/// Three's `subBuild( node, 'VERTEX' )` of the position chain (§8, "`VERTEX_`
/// sub-builds"), undone on three's side as `tests/nodes_sky_wgsl.rs` does.
fn undo_vertex_sub_build(three: &str) -> String {
    let let_line = three
        .lines()
        .find(|l| l.trim_start().starts_with("let VERTEX_nodeConst"))
        .expect("three's projected-position temp");
    let (name, value) = let_line
        .trim_start()
        .trim_start_matches("let ")
        .split_once(" = ")
        .unwrap();
    three
        .lines()
        .filter(|l| *l != let_line)
        .map(|l| l.replace(name, value.trim_end_matches(';')))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("VERTEX_", "")
}

/// A setup context under a `ClippingContext` with `union` and
/// `intersection` planes, on a device with or without `clip-distances`.
fn clipped(union: usize, intersection: usize, hardware: bool) -> SetupContext {
    SetupContext {
        clipping: Some(Rc::new(ClippingContext::new(
            vec![[0.0; 4]; union],
            vec![[0.0; 4]; intersection],
            hardware,
        ))),
        ..SetupContext::default()
    }
}

/// `material` built under `context`, with `alphaToCoverage` on a
/// multisampled target when `a2c`.
fn build(material: &MeshBasicNodeMaterial, context: &SetupContext, a2c: bool) -> NodeProgram {
    with_alpha_to_coverage_samples(a2c, || {
        NodeBuilder::new().build(&setup(material, context, None))
    })
}

/// The discard page's two spheres: `MeshBasicNodeMaterial( { color } )`,
/// the second with `alphaToCoverage`.
fn sphere(alpha_to_coverage: bool) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(if alpha_to_coverage {
        0x40ff80
    } else {
        0xff8040
    });
    material.alpha_to_coverage = alpha_to_coverage;
    material
}

/// `clipping()`: the union loop's discard, then `clipped` over the
/// intersection planes — first in the fragment flow, before the diffuse
/// colour — and a vertex stage that clipping leaves alone.
#[test]
fn clipping_matches_three() {
    let program = build(&sphere(false), &clipped(1, 2, false), false);
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor.w = 1.0;"),
        &section(DISCARD_FRAGMENT, "// flow", "DiffuseColor.w = 1.0;"),
        "clipping() fragment flow",
    );
    assert_same(
        &section(&program.fragment_wgsl, "// vars", "var<private> Output"),
        &section(DISCARD_FRAGMENT, "// vars", "var<private> Output"),
        "clipping() vars",
    );
    assert_same(
        &plane_buffers(&program.fragment_wgsl),
        &plane_buffers(DISCARD_FRAGMENT),
        "clipping() plane buffers",
    );
    let three = undo_vertex_sub_build(DISCARD_VERTEX);
    assert_same(
        &section(&program.vertex_wgsl, "// varyings", "var<private> varyings"),
        &section(&three, "// varyings", "var<private> varyings"),
        "clipping() varyings",
    );
    assert_same(
        &section(&program.vertex_wgsl, "// flow", "return varyings;"),
        &section(&three, "// flow", "return varyings;"),
        "clipping() vertex flow",
    );
}

/// `clippingAlpha()` with the union plane in software: both loops fade
/// `clipOpacity`, after the diffuse colour, and a fully clipped fragment is
/// discarded.
#[test]
fn clipping_alpha_matches_three() {
    let program = build(&sphere(true), &clipped(1, 2, false), true);
    assert_same(
        &through_alpha_discard(&program.fragment_wgsl, "// flow"),
        &through_alpha_discard(ALPHA_FRAGMENT, "// flow"),
        "clippingAlpha() fragment flow",
    );
    assert_same(
        &section(&program.fragment_wgsl, "// vars", "var<private> Output"),
        &section(ALPHA_FRAGMENT, "// vars", "var<private> Output"),
        "clippingAlpha() vars",
    );
    assert_same(
        &plane_buffers(&program.fragment_wgsl),
        &plane_buffers(ALPHA_FRAGMENT),
        "clippingAlpha() plane buffers",
    );
    let three = undo_vertex_sub_build(ALPHA_VERTEX);
    assert_same(
        &section(&program.vertex_wgsl, "// flow", "return varyings;"),
        &section(&three, "// flow", "return varyings;"),
        "clippingAlpha() vertex flow",
    );
    // Without a multisampled target the same material discards.
    let program = build(&sphere(true), &clipped(1, 2, false), false);
    assert!(!program.fragment_wgsl.contains("clipOpacity"));
    assert!(program.fragment_wgsl.contains("clipped = true;"));
}

/// `hardwareClipping()`: the knot's one union plane is a clip distance
/// written after the varyings and before the projection, behind `enable
/// clip_distances;`; its two intersection planes take `clippingAlpha()`
/// after `EmissiveColor` with the union loop left out.
#[test]
fn hardware_clipping_matches_three() {
    let mut knot = MeshBasicNodeMaterial::phong(Color::from_hex(0x80ee10));
    knot.shininess = 0.0;
    knot.alpha_to_coverage = true;
    let program = build(&knot, &clipped(1, 2, true), true);

    let three = undo_vertex_sub_build(KNOT_VERTEX);
    assert_same(
        &section(&program.vertex_wgsl, "enable", "// structs"),
        &section(KNOT_VERTEX, "enable", "// structs"),
        "hardwareClipping() directive",
    );
    let member = "\t@builtin( clip_distances ) hw_clip_distances : array<f32, 1 >,\n\t@builtin( position ) builtinClipSpace : vec4<f32>";
    assert!(
        program.vertex_wgsl.contains(member),
        "{}",
        program.vertex_wgsl
    );
    assert!(KNOT_VERTEX.contains(member));
    assert_same(
        &section(
            &program.vertex_wgsl,
            "for ( var i",
            "v_modelViewProjection =",
        ),
        &section(&three, "for ( var i", "v_modelViewProjection ="),
        "hardwareClipping() vertex loop",
    );
    assert_same(
        &plane_buffers(&program.vertex_wgsl),
        &plane_buffers(KNOT_VERTEX),
        "hardwareClipping() plane buffer",
    );

    assert_same(
        &through_alpha_discard(&program.fragment_wgsl, "distanceToPlane = 0.0;"),
        &through_alpha_discard(KNOT_FRAGMENT, "distanceToPlane = 0.0;"),
        "clippingAlpha() fragment block",
    );
    for wgsl in [program.fragment_wgsl.as_str(), KNOT_FRAGMENT] {
        let before = section(wgsl, "EmissiveColor =", "distanceToPlane = 0.0;");
        assert_eq!(
            before.lines().count(),
            2,
            "EmissiveColor, then clippingAlpha()"
        );
    }
    assert_same(
        &plane_buffers(&program.fragment_wgsl),
        &plane_buffers(KNOT_FRAGMENT),
        "clippingAlpha() plane buffer",
    );
    assert!(!program.fragment_wgsl.contains("clip_distances"));
}

/// `setupHardwareClipping()`'s limits: more than eight union planes, or a
/// device without `clip-distances`, fall back to the fragment discard; with
/// no planes at all nothing is added.
#[test]
fn hardware_clipping_falls_back_to_discard() {
    let material = sphere(false);
    for context in [clipped(9, 0, true), clipped(2, 0, false)] {
        let program = build(&material, &context, false);
        assert!(!program.vertex_wgsl.contains("clip_distances"));
        assert!(program.fragment_wgsl.contains("discard;"));
    }
    let program = build(&material, &clipped(8, 0, true), false);
    assert!(program.vertex_wgsl.contains("enable clip_distances;"));
    assert!(program.vertex_wgsl.contains("array<f32, 8 >"));
    assert!(!program.fragment_wgsl.contains("discard;"));

    let plain = build(&material, &SetupContext::default(), false);
    let empty = build(&material, &clipped(0, 0, true), false);
    assert_eq!(plain.fragment_wgsl, empty.fragment_wgsl);
    assert_eq!(plain.vertex_wgsl, empty.vertex_wgsl);
}
