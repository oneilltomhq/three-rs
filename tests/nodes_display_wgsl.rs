//! #144's dump gate: the fragment WGSL each ported display node generates,
//! against three.js r187dev's own dump of the page that uses it
//! (`tests/fixtures/nodes_display/`, copied verbatim from
//! `tools/dump-webgpu.mjs` output).
//!
//! The comparison is structural, not byte-for-byte. r187dev spells a
//! single-use value `let nodeConstN = …;` where the port declares a
//! `nodeVarN` (see `docs/nodes.md`), and counters that have run for the whole
//! page number three's uniforms, so names and declaration style cannot be
//! compared. What is compared, over the `main()` body — or, for the two nodes
//! three only dumps inside a larger material, over the node's loop — is what
//! decides the pixels:
//!
//! - every built-in call, as a multiset (`textureSample` ×9, `dot` ×9, …),
//!   so a missed or duplicated tap or a different function shows;
//! - every float literal, as a multiset at f32 precision, so the kernels,
//!   coefficients and constants are the same numbers;
//! - the control flow: `if`, `else` and loop headers, with names removed;
//! - for whole-body quads, the texture and sampler binding types.

#[path = "display/materials.rs"]
mod materials;
#[path = "../examples/webgpu_refraction.rs"]
#[allow(dead_code)]
mod webgpu_refraction;

use std::collections::BTreeMap;

use three_rs::lights::LightKind;
use three_rs::materials::phong::LightDesc;
use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::NodeBuilder;

/// The part of a fragment module the fingerprint is taken over.
#[derive(Clone, Copy)]
enum Region {
    /// `// code` to `return output;` in `main()`.
    Body,
    /// The first loop in `main()` through the statement after it.
    Loop,
    /// The body of the named WGSL `fn`, through its `return` — for a node
    /// that is one `Fn()` with a layout, where `main()` is only the call.
    Function(&'static str),
}

fn region(wgsl: &str, which: Region) -> String {
    if let Region::Function(name) = which {
        let start = wgsl
            .find(&format!("fn {name} ("))
            .unwrap_or_else(|| panic!("no fn {name}"));
        let body = &wgsl[start..];
        let body = &body[body.find('{').expect("a body")..];
        let mut depth = 0i32;
        for (i, c) in body.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return body[..=i].to_string();
                    }
                }
                _ => {}
            }
        }
        panic!("fn {name} is not closed");
    }
    let main = &wgsl[wgsl.find("@fragment").expect("a fragment entry point")..];
    let body = &main[main.find("// code").expect("a code section")..];
    let body = &body[..body.find("return output;").expect("a return")];
    match which {
        Region::Body => body.to_string(),
        Region::Function(_) => unreachable!(),
        Region::Loop => {
            let start = body.find("for (").expect("a loop");
            let mut depth = 0i32;
            let mut end = start;
            for (i, c) in body[start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            // …and the division by the tap count that follows it.
            let after = &body[end..];
            let next = after.find(';').map(|i| i + 1).unwrap_or(0);
            body[start..end + next].to_string()
        }
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Names the two builders choose for themselves, which are not calls to
/// anything that affects the result.
fn is_generated_name(name: &str) -> bool {
    ["nodeVar", "nodeConst", "nodeUniform", "nodeVarying", "fn"]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit()))
        })
}

#[derive(Debug, PartialEq, Default)]
struct Fingerprint {
    calls: BTreeMap<String, usize>,
    floats: BTreeMap<String, usize>,
    flow: Vec<String>,
    bindings: Vec<String>,
}

fn fingerprint(wgsl: &str, which: Region) -> Fingerprint {
    let text = region(wgsl, which);
    let mut print = Fingerprint::default();

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let is_call = j < chars.len() && chars[j] == '(';
            let keyword = matches!(name.as_str(), "if" | "for" | "while");
            if is_call && !keyword && !is_generated_name(&name) {
                *print.calls.entry(name).or_default() += 1;
            }
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let literal: String = chars[start..i].iter().collect();
            let negative = start > 0 && chars[start - 1] == '-' && {
                // A unary minus: `( -1.0` or `, -0.01`, not `a - 1.0`.
                let before = chars[..start - 1].iter().rev().find(|c| **c != ' ');
                matches!(before, Some('(') | Some(','))
            };
            if literal.contains('.') {
                let value: f32 = literal.parse().expect("a float literal");
                let value = if negative { -value } else { value };
                *print.floats.entry(format!("{value:?}")).or_default() += 1;
            }
            continue;
        }
        i += 1;
    }

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("if (") || line.starts_with("} else") {
            print
                .flow
                .push(if line.starts_with("if") { "if" } else { "else" }.to_string());
        } else if line.starts_with("for (") {
            // Names out, shape in: `for ( var i : i32 = …; i <= …; i ++ )`
            // becomes `for var : i32 <= ++`.
            let ty = line
                .split(':')
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
                .unwrap_or("");
            let condition = ["<=", "<"]
                .into_iter()
                .find(|op| line.contains(&format!(" {op} ")))
                .unwrap_or("?");
            let step = if line.contains("++") { "++" } else { "+= 1." };
            print.flow.push(format!("for {ty} {condition} {step}"));
        }
    }

    if matches!(which, Region::Body) {
        let mut bindings: Vec<String> = wgsl
            .lines()
            .filter(|line| line.contains("var nodeUniform") || line.contains("var<uniform>"))
            .map(|line| {
                let ty = line
                    .rsplit(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_end_matches(';');
                if line.contains("var<uniform>") {
                    "uniform".to_string()
                } else {
                    ty.to_string()
                }
            })
            .collect();
        bindings.sort();
        print.bindings = bindings;
    }
    print
}

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/nodes_display/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn check(label: &str, which: Region) {
    let quads = materials::display_quads();
    let quad = quads
        .iter()
        .find(|quad| quad.label == label)
        .unwrap_or_else(|| panic!("no display quad {label}"));
    let flow = setup(&quad.material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let ours = fingerprint(&program.fragment_wgsl, which);
    let three = fingerprint(&fixture(quad.fixture), which);
    assert_eq!(
        ours, three,
        "{label}: the port's fingerprint (left) differs from three's {}\n\n{}",
        quad.fixture, program.fragment_wgsl
    );
}

#[test]
fn gaussian_blur_horizontal_matches_three() {
    check("gaussian_blur_horizontal", Region::Body);
}

#[test]
fn gaussian_blur_vertical_matches_three() {
    check("gaussian_blur_vertical", Region::Body);
}

#[test]
fn sobel_matches_three() {
    check("sobel", Region::Body);
}

#[test]
fn dot_screen_matches_three() {
    check("dot_screen", Region::Body);
}

#[test]
fn rgb_shift_matches_three() {
    check("rgb_shift", Region::Body);
}

#[test]
fn fxaa_matches_three() {
    check("fxaa", Region::Function("FxaaPixelShader"));
}

#[test]
fn after_image_matches_three() {
    check("after_image", Region::Body);
}

#[test]
fn pixelation_matches_three() {
    check("pixelation", Region::Body);
}

#[test]
fn box_blur_loops_match_three() {
    check("box_blur", Region::Loop);
}

/// Not a display node but the screen reads they share (#169): the
/// refractor's `backdropNode` — `viewportSharedTexture( viewportSafeUV(
/// screenUV + offset ) )`, with `viewportSafeUV`'s depth load and
/// `linearDepth` compare — blended in by `LightsNode` under the page's four
/// point lights.
#[test]
fn refraction_backdrop_matches_three() {
    let floor_normal = three_rs::Texture::new(2, 2, Some(vec![0; 16]));
    let material = webgpu_refraction::refractor_material(&floor_normal);
    let ctx = SetupContext {
        lights: (0..4)
            .map(|index| LightDesc {
                index,
                kind: LightKind::Point,
                shadow_map: None,
            })
            .collect(),
        ..SetupContext::default()
    };
    let program = NodeBuilder::new().build(&setup(&material, &ctx, None));
    let ours = fingerprint(&program.fragment_wgsl, Region::Body);
    let three = fingerprint(
        &fixture("webgpu_refraction_m06_refractor.wgsl"),
        Region::Body,
    );
    assert_eq!(ours, three, "\n{}", program.fragment_wgsl);
}

#[test]
fn hash_blur_loop_matches_three() {
    check("hash_blur", Region::Loop);
}

#[test]
fn motion_blur_matches_three() {
    check("motion_blur", Region::Body);
}

#[test]
fn traa_resolve_matches_three() {
    check("traa", Region::Body);
}

#[test]
fn traa_subpixel_correction_matches_three() {
    check("traa", Region::Function("subpixelCorrection"));
}

#[test]
fn traa_clip_aabb_matches_three() {
    check("traa", Region::Function("clipAABB"));
}

#[test]
fn traa_flicker_reduction_matches_three() {
    check("traa", Region::Function("flickerReduction"));
}

#[test]
fn dof_coc_matches_three() {
    check("dof_coc", Region::Body);
}

#[test]
fn dof_coc_gaussian_horizontal_matches_three() {
    check("dof_coc_gaussian_horizontal", Region::Body);
}

#[test]
fn dof_coc_gaussian_vertical_matches_three() {
    check("dof_coc_gaussian_vertical", Region::Body);
}

#[test]
fn dof_coc_blurred_matches_three() {
    check("dof_coc_blurred", Region::Body);
}

#[test]
fn dof_blur64_matches_three() {
    check("dof_blur64", Region::Body);
}

#[test]
fn dof_blur16_matches_three() {
    check("dof_blur16", Region::Body);
}

#[test]
fn dof_composite_matches_three() {
    check("dof_composite", Region::Body);
}

/// The fragment WGSL the port builds for the display quad `label`.
fn quad_wgsl(label: &str) -> String {
    let quads = materials::display_quads();
    let quad = quads
        .iter()
        .find(|quad| quad.label == label)
        .unwrap_or_else(|| panic!("no display quad {label}"));
    let flow = setup(&quad.material, &SetupContext::default(), None);
    NodeBuilder::new().build(&flow).fragment_wgsl
}

/// What the `dof_*` fingerprints cannot see: member types, swizzles and
/// constructors. The CoC pass's `outputStruct( near, far )` declares two
/// `f32` members, as three's `m04` does, and assigns each from a scalar.
/// No `dof_*` quad swizzles an already-scalar CoC read a second time.
#[test]
fn dof_coc_writes_two_f32_members() {
    let wgsl = quad_wgsl("dof_coc");
    for member in ["@location( 0 ) m0 : f32,", "@location( 1 ) m1 : f32,"] {
        assert!(wgsl.contains(member), "no `{member}` in\n{wgsl}");
    }
    assert!(!wgsl.contains("vec4<f32>,\n\t@location"), "{wgsl}");
    let body = region(&wgsl, Region::Body);
    for line in body
        .lines()
        .filter(|l| l.trim_start().starts_with("output.m"))
    {
        assert!(
            !line.contains("vec4<f32>(") && !line.trim_end().ends_with(".x;"),
            "a CoC member is not assigned a bare scalar: `{line}`\n{wgsl}"
        );
    }
    for label in [
        "dof_coc",
        "dof_coc_gaussian_horizontal",
        "dof_coc_gaussian_vertical",
        "dof_coc_blurred",
        "dof_blur64",
        "dof_blur16",
        "dof_composite",
    ] {
        let wgsl = quad_wgsl(label);
        assert!(
            !wgsl.contains(".x.x"),
            "{label}: a scalar read swizzled twice\n{wgsl}"
        );
        assert!(
            !wgsl.contains(").x ).x"),
            "{label}: a scalar read swizzled twice\n{wgsl}"
        );
    }
}

/// The composite starts from three's `vec4( 0, 0, 0, 1 )` and mixes the far
/// and then the near field into its `.xyz` (`m12`).
#[test]
fn dof_composite_builds_its_vec4() {
    let wgsl = quad_wgsl("dof_composite");
    let body = region(&wgsl, Region::Body);
    assert!(
        body.contains("vec4<f32>( 0.0, 0.0, 0.0, 1.0 )"),
        "no vec4( 0, 0, 0, 1 ) in the composite\n{wgsl}"
    );
    assert_eq!(body.matches("mix(").count(), 2, "{wgsl}");
    for line in body.lines().filter(|l| l.contains("mix(")) {
        assert_eq!(line.matches(".xyz").count(), 2, "`{line}`\n{wgsl}");
    }
}
