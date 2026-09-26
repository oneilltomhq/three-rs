//! `Line2NodeMaterial` with `worldUnits: true` and `alphaToCoverage` — the
//! material `webgpu_lines_fat_raycasting` draws — against three's own dump of
//! that page. No GPU: this reads the generated WGSL.
//!
//! The fixtures in `tests/fixtures/webgpu_lines_fat_raycasting/` are
//! `node tools/dump-webgpu.mjs webgpu_lines_fat_raycasting`'
//! `m00_vertex_vertex.wgsl` and `m01_fragment_fragment.wgsl`, verbatim.
//!
//! Not a whole-file diff: the vendored three.js is r187dev, whose generator
//! spells single-assignment temporaries as `let nodeConstN` where r186 (which
//! the port follows) hoists a `var<private> nodeVarN`, and adds the `VERTEX_`
//! sub-build names (`docs/nodes.md` §8). What is compared is what the world
//! units branch adds, with every temporary, uniform, varying and function
//! renumbered by first appearance on both sides:
//!
//! - the fragment `main` body in full — the `closestLineToLine` distance, the
//!   orthographic shortcut and the `fwidth` coverage ramp;
//! - the `closestLineToLine` function's arithmetic;
//! - every vertex-stage write to the `worldStart` / `worldEnd` / `worldPos`
//!   varying properties.

use std::collections::HashMap;

use three_rs::addons::lines::LineSegmentsGeometry;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::{NodeBuilder, NodeProgram};

const VERTEX: &str = include_str!("fixtures/webgpu_lines_fat_raycasting/line2.vert.wgsl");
const FRAGMENT: &str = include_str!("fixtures/webgpu_lines_fat_raycasting/line2.frag.wgsl");

/// `matLine` of the example: `linewidth: 1, worldUnits: true, vertexColors:
/// true, alphaToCoverage: true`.
fn program() -> NodeProgram {
    let mut geometry = LineSegmentsGeometry::new();
    geometry.set_positions(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    geometry.set_colors(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    let mut material = MeshBasicNodeMaterial::line2(Color::from_hex(0xffffff));
    material.linewidth = 1.0;
    material.world_units = true;
    material.vertex_colors = true;
    assert!(material.alpha_to_coverage, "three's default");
    let flow = setup(
        &material,
        &SetupContext {
            line_segments: Some(geometry.attributes()),
            ..SetupContext::default()
        },
        None,
    );
    NodeBuilder::new().build(&flow)
}

/// Fold `nodeConstN` / `nodeVarN` into one `tmpN` namespace, renumber every
/// counter by first appearance, drop `let`, and drop blank lines (r187 leaves a tab-only
/// line after each assignment inside an `If`).
fn canonical(lines: &[&str]) -> Vec<String> {
    let mut out = lines
        .iter()
        .map(|line| line.trim_end())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .replace("nodeVarying", "VARYING")
        // `tmp9999N` keeps a `nodeConstN` apart from the `nodeVarN` of the
        // same number until both are renumbered.
        .replace("nodeConst", "tmp9999")
        .replace("nodeVar", "tmp")
        .replace("VARYING", "nodeVarying")
        // r187 declares a single-assignment temporary where it is written.
        .replace("let ", "");
    for prefix in ["nodeUniform", "nodeVarying", "tmp", "fn"] {
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
    out.lines().map(str::to_string).collect()
}

/// The lines between `start` (exclusive) and the next line equal to `}`.
fn section<'a>(wgsl: &'a str, start: &str) -> Vec<&'a str> {
    wgsl.lines()
        .skip_while(|line| !line.starts_with(start))
        .skip(1)
        .take_while(|line| *line != "}")
        .collect()
}

#[track_caller]
fn assert_lines(port: Vec<String>, three: Vec<String>, what: &str) {
    if port == three {
        return;
    }
    let mut report = String::new();
    for (i, (a, b)) in port.iter().zip(&three).enumerate() {
        if a != b {
            report.push_str(&format!(
                "line {}:\n  port : {a:?}\n  three: {b:?}\n",
                i + 1
            ));
        }
    }
    report.push_str(&format!(
        "lines: port {} three {}\n",
        port.len(),
        three.len()
    ));
    panic!("{what} differs from three.js\n{report}");
}

#[test]
fn fragment_main_matches_three() {
    let program = program();
    assert_lines(
        canonical(&section(&program.fragment_wgsl, "fn main(")),
        canonical(&section(FRAGMENT, "fn main(")),
        "fragment main",
    );
}

/// `closestLineToLine`: three's body is `let`s, the port's is the r186
/// `var` + assignment form; the expressions must be the same.
#[test]
fn closest_line_to_line_matches_three() {
    let program = program();
    let body = |wgsl: &str| -> Vec<String> {
        canonical(&section(wgsl, "fn fn"))
            .into_iter()
            .filter(|line| !line.trim_start().starts_with("var "))
            .collect()
    };
    let three = &FRAGMENT[FRAGMENT.find("fn fn3").unwrap()..];
    let port =
        &program.fragment_wgsl[program.fragment_wgsl.find("( p1 : vec3<f32>").unwrap() - 7..];
    assert_lines(body(port), body(three), "closestLineToLine");
}

#[test]
fn vertex_world_writes_match_three() {
    let program = program();
    let writes = |wgsl: &str| -> Vec<String> {
        let lines: Vec<&str> = wgsl
            .lines()
            .filter(|line| line.trim_start().starts_with("varyings.world"))
            .collect();
        canonical(&lines)
    };
    let port = writes(&program.vertex_wgsl);
    assert_eq!(port.len(), 7, "{}", program.vertex_wgsl);
    assert_lines(
        port,
        writes(VERTEX),
        "worldStart / worldEnd / worldPos writes",
    );
}
