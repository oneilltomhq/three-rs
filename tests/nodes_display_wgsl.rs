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
//!
//! One normalisation, for `hashBlur` only: three dumps it over
//! `viewportSharedTexture()`, which the port does not have, so its taps are a
//! nearest `textureLoad` where the port's are a `textureSample`. There both
//! spellings are reduced to `tap( uv )` before the fingerprint is taken, so
//! the uv — which is where the node's hash and circle are — is still compared.

#[path = "display/materials.rs"]
mod materials;

use std::collections::BTreeMap;

use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::NodeBuilder;

/// The part of a fragment module the fingerprint is taken over.
#[derive(Clone, Copy)]
enum Region {
    /// `// code` to `return output;` in `main()`.
    Body,
    /// The first loop in `main()` through the statement after it.
    Loop,
    /// [`Region::Loop`] with every texture tap reduced to `tap( uv )`.
    LoopAnyTap,
}

fn region(wgsl: &str, which: Region) -> String {
    let main = &wgsl[wgsl.find("@fragment").expect("a fragment entry point")..];
    let body = &main[main.find("// code").expect("a code section")..];
    let body = &body[..body.find("return output;").expect("a return")];
    match which {
        Region::Body => body.to_string(),
        Region::LoopAnyTap => any_tap(&region(wgsl, Region::Loop)),
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

/// The argument list of the call whose `(` is at `open`: the top-level
/// arguments, and the index just past the closing `)`.
fn call_args(text: &str, open: usize) -> (Vec<String>, usize) {
    let mut depth = 0;
    let mut args = Vec::new();
    let mut start = open + 1;
    for (i, c) in text[open..].char_indices() {
        let at = open + i;
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    args.push(text[start..at].trim().to_string());
                    return (args, at + 1);
                }
            }
            ',' if depth == 1 => {
                args.push(text[start..at].trim().to_string());
                start = at + 1;
            }
            _ => {}
        }
    }
    panic!("unbalanced call at {open}");
}

/// `textureSample( t, s, uv )` and three's nearest `textureLoad( t,
/// vec2<u32>( clamp( floor( tsl_coord_clampS_clampT_2d( uv ) * … ) ) ), u32( 0 )
/// )` both become `tap( uv )`, and the dimensions vars the load declares go.
fn any_tap(text: &str) -> String {
    let text: String = text
        .lines()
        .filter(|line| !line.contains("= textureDimensions("))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = String::new();
    let mut rest = text.as_str();
    loop {
        let sample = rest.find("textureSample(");
        let load = rest.find("textureLoad(");
        let (at, is_load) = match (sample, load) {
            (Some(s), Some(l)) if l < s => (l, true),
            (Some(s), _) => (s, false),
            (None, Some(l)) => (l, true),
            (None, None) => break,
        };
        out.push_str(&rest[..at]);
        let open = at + rest[at..].find('(').unwrap();
        let (args, end) = call_args(rest, open);
        let uv = if is_load {
            let coord = &args[1];
            let wrap = coord
                .find("tsl_coord_clampS_clampT_2d(")
                .expect("a nearest load's wrapped coordinate");
            let wrap_open = wrap + coord[wrap..].find('(').unwrap();
            call_args(coord, wrap_open).0.remove(0)
        } else {
            args[2].clone()
        };
        out.push_str(&format!("tap( {} )", any_tap(&uv)));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
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

#[test]
fn hash_blur_loop_matches_three() {
    check("hash_blur", Region::LoopAnyTap);
}
