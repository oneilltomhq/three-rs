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
fn gtao_matches_three() {
    check("gtao", Region::Body);
}

#[test]
fn gtao_screen_position_from_clip_matches_three() {
    check("gtao", Region::Function("getScreenPositionFromClip"));
}

#[test]
fn godrays_matches_three() {
    check("godrays", Region::Body);
}

#[test]
fn bilateral_blur_horizontal_matches_three() {
    check("bilateral_blur_horizontal", Region::Body);
}

#[test]
fn bilateral_blur_vertical_matches_three() {
    check("bilateral_blur_vertical", Region::Body);
}

#[test]
fn depth_aware_blend_matches_three() {
    check("depth_aware_blend", Region::Body);
}

#[test]
fn rtt_matches_three() {
    check("rtt", Region::Body);
}

#[test]
fn lensflare_matches_three() {
    check("lensflare", Region::Body);
}

#[test]
fn lensflare_gaussian_blur_horizontal_matches_three() {
    check("lensflare_gaussian_blur_horizontal", Region::Body);
}

#[test]
fn lensflare_gaussian_blur_vertical_matches_three() {
    check("lensflare_gaussian_blur_vertical", Region::Body);
}

#[test]
fn lensflare_composite_matches_three() {
    check("lensflare_composite", Region::Body);
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

#[test]
fn ssr_matches_three() {
    check("ssr", Region::Body);
}

#[test]
fn ssr_copy_matches_three() {
    check("ssr_copy", Region::Body);
}

#[test]
fn ssr_blur_matches_three() {
    check("ssr_blur", Region::Body);
}

/// The page's `RTT`: `scenePassColor.add( ssrPass.rgb )`, which reads the
/// blur chain at the roughness-picked level.
#[test]
fn ssr_resolve_matches_three() {
    check("ssr_resolve", Region::Body);
}

#[test]
fn smaa_edges_matches_three() {
    check("smaa_edges", Region::Body);
}

#[test]
fn smaa_weights_matches_three() {
    check("smaa_weights", Region::Body);
}

#[test]
fn smaa_blend_matches_three() {
    check("smaa_blend", Region::Body);
}

#[test]
fn ssgi_matches_three() {
    check("ssgi", Region::Body);
}

/// `nodeVarN` / `nodeConstN` replaced by `_`: the names the two builders
/// pick for themselves, which differ (the port declares a `var` where three
/// writes a `let`).
fn anonymise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = ["nodeVar", "nodeConst"]
        .iter()
        .filter_map(|prefix| rest.find(prefix))
        .min()
    {
        let before_ok = rest[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !is_ident_char(c));
        let len = rest[at..]
            .find(|c: char| !is_ident_char(c))
            .unwrap_or(rest.len() - at);
        let name = &rest[at..at + len];
        out.push_str(&rest[..at]);
        if before_ok && is_generated_name(name) {
            out.push('_');
        } else {
            out.push_str(name);
        }
        rest = &rest[at + len..];
    }
    out.push_str(rest);
    out
}

/// The lines of a `struct name { … };` block, trimmed.
fn struct_members(wgsl: &str, name: &str) -> Vec<String> {
    let start = wgsl
        .find(&format!("struct {name} {{"))
        .unwrap_or_else(|| panic!("no struct {name}"));
    let block = &wgsl[start..];
    let block = &block[block.find('{').unwrap() + 1..block.find("};").unwrap()];
    block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_string())
        .collect()
}

/// What `fingerprint` leaves out, for the SSGI body: the swizzles, the
/// binary and unary operators and the integer literals as multisets, every
/// one-statement `if`/`else` (a lowered `select`) with its condition and both
/// branches in order, and each `.yx` with the expression it swizzles and the
/// blocks it sits in.
#[derive(Debug, PartialEq, Default)]
struct Shapes {
    swizzles: BTreeMap<String, usize>,
    operators: BTreeMap<String, usize>,
    integers: BTreeMap<String, usize>,
    selects: Vec<[String; 3]>,
    yx: Vec<(String, String)>,
}

/// The binary and unary operators three's printer writes between spaces.
const OPERATORS: [&str; 20] = [
    "+", "-", "*", "/", "%", "<", ">", "<=", ">=", "==", "!=", "&&", "||", "&", "|", "^", "<<",
    ">>", "!", "~",
];

fn shapes(wgsl: &str) -> Shapes {
    let text = region(wgsl, Region::Body);
    // The material's tail is not the SSGI node's: three's `Output` is an
    // `f32` (`max( … ).x`) where the port's is the `vec4` every material
    // declares, a known difference (docs/nodes.md §69).
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("Output ="))
        .collect();
    let mut shapes = Shapes::default();

    for line in &lines {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            // `.xyz` after a name or a `)`: a swizzle (`object.nodeUniformN`
            // and `output.mN` are member reads, not all-component letters).
            if c == '.' && i > 0 && (is_ident_char(chars[i - 1]) || chars[i - 1] == ')') {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && is_ident_char(chars[end]) {
                    end += 1;
                }
                let member: String = chars[start..end].iter().collect();
                let swizzle = (1..=4).contains(&member.len())
                    && (member.chars().all(|c| "xyzw".contains(c))
                        || member.chars().all(|c| "rgba".contains(c)));
                if swizzle {
                    *shapes.swizzles.entry(member).or_default() += 1;
                }
                i = end;
                continue;
            }
            // An integer literal: digits not part of a name or a float.
            if c.is_ascii_digit()
                && (i == 0 || !(is_ident_char(chars[i - 1]) || chars[i - 1] == '.'))
            {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i < chars.len() && chars[i] == 'u' {
                    i += 1;
                }
                let float = i < chars.len() && (chars[i] == '.' || chars[i] == 'e');
                if !float && (i == chars.len() || !is_ident_char(chars[i])) {
                    let literal: String = chars[start..i].iter().collect();
                    *shapes.integers.entry(literal).or_default() += 1;
                }
                continue;
            }
            i += 1;
        }
        // Both builders put a space either side of a binary operator and
        // after the `(` before a unary one.
        for token in line.split(' ') {
            if OPERATORS.contains(&token) {
                *shapes.operators.entry(token.to_string()).or_default() += 1;
            }
        }
    }

    // `if ( c ) {` / `a = x;` / `} else {` / `a = y;` / `}`: three's `If`
    // lowering of a `select`, so swapped branches show.
    for window in lines.windows(5) {
        if window[0].starts_with("if (")
            && window[2] == "} else {"
            && window[4] == "}"
            && window[1].contains(" = ")
            && window[3].contains(" = ")
        {
            let rhs = |line: &str| anonymise(line.split_once(" = ").unwrap().1);
            shapes
                .selects
                .push([anonymise(window[0]), rhs(window[1]), rhs(window[3])]);
        }
    }

    // Each `.yx`: what it swizzles, with a spilled variable (`a = clamp( … );
    // b = a.yx;`) resolved to the expression it holds, and the enclosing
    // block headers, so the swizzle cannot move to the other branch.
    let mut blocks: Vec<String> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('}') {
            blocks.pop();
        }
        if line.ends_with('{') {
            blocks.push(anonymise(line));
        }
        if let Some(rhs) = line
            .strip_suffix(".yx;")
            .and_then(|line| line.split_once(" = "))
            .map(|(_, rhs)| rhs)
        {
            let mut operand = rhs.trim_start_matches("let ").to_string();
            if is_generated_name(&operand) {
                let assignment = format!("{operand} = ");
                let held = lines[..index]
                    .iter()
                    .rev()
                    .find_map(|line| line.trim_start_matches("let ").strip_prefix(&assignment))
                    .unwrap_or_else(|| panic!("{operand} is never assigned"));
                operand = held.trim_end_matches(';').to_string();
            }
            shapes.yx.push((blocks.join(" / "), anonymise(&operand)));
        } else {
            assert!(
                !line.contains(".yx"),
                "a `.yx` inside an expression: {line}"
            );
        }
    }
    shapes
}

/// What [`check`]'s fingerprint does not see in the SSGI body: the
/// `OutputType` members' types, the uniform block's members, swizzles (the
/// horizon pair's `.yx`), operators, integer literals and the order of
/// select branches.
#[test]
fn ssgi_shapes_match_three() {
    let quads = materials::display_quads();
    let quad = quads.iter().find(|quad| quad.label == "ssgi").unwrap();
    let flow = setup(&quad.material, &SetupContext::default(), None);
    let ours = NodeBuilder::new().build(&flow).fragment_wgsl;
    let three = fixture(quad.fixture);

    // `outputStruct( aoField, giField )`: an `f32` and a `vec3`, not two
    // `vec4`s as an `mrt()` would make them.
    let members = struct_members(&ours, "OutputType");
    assert_eq!(
        members,
        ["@location( 0 ) m0 : f32", "@location( 1 ) m1 : vec3<f32>"],
        "\n{ours}"
    );
    assert_eq!(members, struct_members(&three, "OutputType"));
    assert_eq!(
        struct_members(&ours, "objectStruct"),
        struct_members(&three, "objectStruct"),
        "the uniform block's members and types"
    );

    let (ours_shapes, three_shapes) = (shapes(&ours), shapes(&three));
    // `directionIsRight.select( frontBackHorizon.yx, frontBackHorizon.xy )`,
    // once per `horizonSampling` call: in the `true` branch of `if ( true )`
    // and of `if ( false )`.
    assert_eq!(three_shapes.yx.len(), 2);
    assert!(three_shapes.yx[0].0.ends_with("if ( true ) {"));
    assert!(three_shapes.yx[1].0.ends_with("if ( false ) {"));
    assert!(three_shapes.yx[0].1.starts_with("clamp( "));
    // Not vacuous: the dump's lowered selects and its integer literals.
    assert_eq!(three_shapes.selects.len(), 16);
    assert_eq!(three_shapes.integers.get("4294967295u"), Some(&2));
    assert_eq!(ours_shapes, three_shapes, "\n{ours}");
}

#[test]
fn ssgi_spatial_offsets_matches_three() {
    check("ssgi", Region::Function("spatialOffsets"));
}

#[test]
fn ssgi_gtao_fast_acos_matches_three() {
    check("ssgi", Region::Function("GTAOFastAcos"));
}

#[test]
fn ssgi_composite_matches_three() {
    check("ssgi_composite", Region::Body);
}

#[test]
fn ssgi_traa_resolve_matches_three() {
    check("ssgi_traa", Region::Body);
}
