//! The WGSL gate for `webgpu_sky`: what the node system generates for
//! `SkyMesh`'s `vertexNode` and `colorNode` against three.js' own dumps of
//! the page at 5f610f5 (`tests/fixtures/webgpu_sky/`, verbatim output of
//! `tools/dump-webgpu.mjs`).
//!
//! [`canonical`] is the only thing between the two texts, and every rule in it
//! is a divergence listed in `docs/nodes.md` §8:
//!
//! - lines that are empty or only whitespace, and the empty `// directives`
//!   block;
//! - the numbers in `nodeUniformN` / `nodeVarN` / `nodeVaryingN` /
//!   `nodeConstN`, renumbered on both sides by order of first appearance;
//! - the varyings' names: `SkyMesh` leaves its four `varyingProperty()`s
//!   unnamed, so three numbers them, and the port's `varyingProperty` takes a
//!   name. [`rename_varyings`] maps the port's names (upstream's JS
//!   identifiers) to three's numbers before the comparison.
//!
//! The vertex stage is compared from `// flow` to the `.z = .w` line that
//! pins the box to the far plane, after [`undo_vertex_sub_build`]; the
//! fragment from `// flow` to `DiffuseColor = …`, and its two uniform structs
//! by membership ([`members`]).

use std::collections::HashMap;

use three_rs::addons::objects::SkyMesh;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::nodes::NodeBuilder;

const THREE_VERTEX: &str = include_str!("fixtures/webgpu_sky/sky.vertex.wgsl");
const THREE_FRAGMENT: &str = include_str!("fixtures/webgpu_sky/sky.fragment.wgsl");

/// The varyings three numbered, in the port's names.
fn rename_varyings(wgsl: &str) -> String {
    wgsl.replace("vSunDirection", "nodeVarying7")
        .replace("vSunE", "nodeVarying5")
        .replace("vBetaR", "nodeVarying6")
        .replace("vBetaM", "nodeVarying8")
}

/// Apply the rules in the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out: String = rename_varyings(wgsl)
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
    panic!("{what}: generated WGSL differs from three.js 5f610f5\n{report}\n--- port ---\n{a}\n");
}

fn material() -> MeshBasicNodeMaterial {
    let sky = SkyMesh::new();
    let node = sky.mesh.borrow();
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

/// `vertexNode`: the four varyings and `position.z = position.w`.
#[test]
fn sky_vertex_matches_three() {
    let program = program();
    let three = undo_vertex_sub_build(THREE_VERTEX);
    assert_same(
        &section(&program.vertex_wgsl, "// flow", "v_modelViewProjection.z"),
        &section(&three, "// flow", "v_modelViewProjection.z"),
        "sky vertex flow",
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

/// `colorNode`: the scattering, the sun disc behind a `bool` uniform, the
/// `Lin` intent var, and the cloud `If` with its inlined `fbm` loop.
#[test]
fn sky_fragment_matches_three() {
    let program = program();
    for group in ["renderStruct", "objectStruct"] {
        assert_eq!(
            members(&program.fragment_wgsl, group),
            members(THREE_FRAGMENT, group),
            "{group}"
        );
    }
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor = "),
        &section(THREE_FRAGMENT, "// flow", "DiffuseColor = "),
        "sky fragment flow",
    );
}
