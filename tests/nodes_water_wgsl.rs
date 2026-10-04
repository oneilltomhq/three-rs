//! The WGSL gate for `webgpu_ocean`: what the node system generates for
//! `WaterMesh`'s material against three.js' own dumps of the page at 5f610f5
//! (`tests/fixtures/webgpu_ocean/`, verbatim `m09` / `m10` output of
//! `tools/dump-webgpu.mjs`).
//!
//! [`canonical`] is the only thing between the two texts, and every rule in it
//! is a divergence listed in `docs/nodes.md` §8:
//!
//! - lines that are empty or only whitespace, and the empty `// directives`
//!   block;
//! - the numbers in `nodeUniformN` / `nodeVarN` / `nodeVaryingN` /
//!   `nodeConstN`, renumbered on both sides by order of first appearance;
//! - the port's parenthesised `( 1.0 - nodeVarN.x )` in the mirror's
//!   `screenUV.flipX()`, which three writes bare (§55.3, `FlipNode`) —
//!   [`unparenthesise_flip_x`].
//!
//! The vertex stage is compared from `// flow` to the projected position's
//! assignment, after [`undo_vertex_sub_build`]; the fragment from `// flow`
//! to the opacity multiply, `DiffuseColor.w = …`, and its two uniform structs
//! by membership ([`members`]). The material tail after it is the output
//! clamp, which three writes `let nodeConstN = max( … )` and the port keeps
//! as a `nodeVarN` (§8, "Usage-promoted temps"); it is shared by every rung
//! and is checked here only for being present.

use std::collections::HashMap;
use std::rc::Rc;

use three_rs::addons::objects::{WaterMesh, WaterMeshOptions};
use three_rs::geometries::plane_geometry;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::nodes::NodeBuilder;
use three_rs::Texture;

const THREE_VERTEX: &str = include_str!("fixtures/webgpu_ocean/water.vertex.wgsl");
const THREE_FRAGMENT: &str = include_str!("fixtures/webgpu_ocean/water.fragment.wgsl");

/// Apply the rules in the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out: String = wgsl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| *line != "// directives")
        .collect::<Vec<_>>()
        .join("\n");

    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in ["nodeUniform", "nodeVarying", "nodeVar", "nodeConst"] {
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

/// `vec2<f32>( ( 1.0 - X.x ), X.y )` → `vec2<f32>( 1.0 - X.x, X.y )`.
fn unparenthesise_flip_x(wgsl: &str) -> String {
    const OPEN: &str = "vec2<f32>( ( 1.0 - ";
    let mut out = String::with_capacity(wgsl.len());
    let mut rest = wgsl;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        out.push_str("vec2<f32>( 1.0 - ");
        rest = &rest[at + OPEN.len()..];
        let close = rest.find(".x ), ").expect("the flipped x component");
        out.push_str(&rest[..close]);
        out.push_str(".x, ");
        rest = &rest[close + ".x ), ".len()..];
    }
    out.push_str(rest);
    out
}

/// The lines of `wgsl` from the one starting with `from` up to and including
/// the first one after it starting with `to`.
fn section(wgsl: &str, from: &str, to: &str) -> String {
    let lines: Vec<&str> = wgsl.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with(from))
        .unwrap_or_else(|| panic!("no line starting {from:?}"));
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.trim_start().starts_with(to))
            .unwrap_or_else(|| panic!("no line starting {to:?} after {from:?} in\n{wgsl}"));
    lines[start..=end].join("\n")
}

#[track_caller]
fn assert_same(generated: &str, three: &str, what: &str) {
    let (a, b) = (
        canonical(&unparenthesise_flip_x(generated)),
        canonical(three),
    );
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
    panic!("{what}: generated WGSL differs from three.js 5f610f5\n{report}\n--- port ---\n{a}\n");
}

fn material() -> MeshBasicNodeMaterial {
    // The map's contents never reach the WGSL; only its being a filterable
    // 2-D texture does.
    let normals = Texture::new(4, 4, Some(vec![0; 64]));
    let water = WaterMesh::new(
        Rc::new(plane_geometry(10000.0, 10000.0, 1, 1)),
        WaterMeshOptions::new(normals),
    );
    let node = water.mesh.borrow();
    node.mesh().unwrap().material.clone().unwrap()
}

fn program() -> three_rs::nodes::NodeProgram {
    let material = material();
    let flow = setup(&material, &SetupContext::default(), None);
    NodeBuilder::new().build(&flow)
}

/// Three's `subBuild( node, 'VERTEX' )` of the position chain (§8, "`VERTEX_`
/// sub-builds"): one more name for the projected position and a `let` before
/// the assignment. Undone on three's side, so the rest of the stage compares
/// line for line.
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

/// The vertex stage: `positionWorld` as a varying and the projected
/// position, nothing of the water's own.
#[test]
fn water_vertex_matches_three() {
    let program = program();
    let three = undo_vertex_sub_build(THREE_VERTEX);
    assert_same(
        &section(&program.vertex_wgsl, "// flow", "v_modelViewProjection = "),
        &section(&three, "// flow", "v_modelViewProjection = "),
        "water vertex flow",
    );
}

/// The members of the struct `name` in `wgsl`, sorted: three orders them by
/// when each uniform object was created and the port by generation (§8,
/// "Render-struct member order"), so only the membership is compared.
fn members(wgsl: &str, name: &str) -> Vec<String> {
    let start = wgsl
        .find(&format!("struct {name} {{"))
        .unwrap_or_else(|| panic!("no struct {name}"));
    let body = &wgsl[start..];
    let body = &body[body.find('{').unwrap() + 1..body.find('}').unwrap()];
    let mut out: Vec<String> = body
        .split(',')
        .map(|m| {
            let (member, ty) = m.trim().split_once(" : ").unwrap();
            // `nodeUniformN` numbers follow the order, so only the type of a
            // numbered member is kept.
            if member.starts_with("nodeUniform") {
                format!("nodeUniform : {ty}")
            } else {
                format!("{member} : {ty}")
            }
        })
        .collect();
    out.sort();
    out
}

/// `colorNode` and `opacityNode`: the four scrolling normal-map taps sharing
/// one uv matrix, the variadic `mul( 1.5, 1.0, 1.5 )`, the mirror tap at
/// `screenUV.flipX()` plus the distortion, the Fresnel mix, and the alpha.
#[test]
fn water_fragment_matches_three() {
    let program = program();
    for group in ["renderStruct", "objectStruct"] {
        assert_eq!(
            members(&program.fragment_wgsl, group),
            members(THREE_FRAGMENT, group),
            "{group}"
        );
    }
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor.w = "),
        &section(THREE_FRAGMENT, "// flow", "DiffuseColor.w = "),
        "water fragment flow",
    );
    // `transparent = true`: no `DiffuseColor.w = 1.0`, so the alpha the
    // opacity node produced reaches the clamp and the blend.
    let clamp = "= max( vec4<f32>( DiffuseColor.xyz, DiffuseColor.w ), vec4<f32>( 0.0 ) );";
    for wgsl in [&program.fragment_wgsl[..], THREE_FRAGMENT] {
        let tail = &wgsl[wgsl.find("DiffuseColor.w = (").unwrap()..];
        assert!(!tail.contains("DiffuseColor.w = 1.0;"), "{tail}");
        assert!(tail.contains(clamp), "{tail}");
    }
}
