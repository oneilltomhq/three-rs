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
//! three only dumps inside a larger material, over the node's loop, and for
//! the OIT pass's lit materials over the tail of `main()` past the lighting —
//! is what decides the pixels:
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
    /// `main()` from the first statement containing the text to `return
    /// output;` — the tail of a lit material, past the lighting.
    From(&'static str),
    /// The one statement in `main()` containing the text.
    Statement(&'static str),
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
        Region::From(text) => {
            let start = body
                .find(text)
                .unwrap_or_else(|| panic!("no {text:?} in main()"));
            let start = body[..start].rfind('\n').map_or(0, |i| i + 1);
            body[start..].to_string()
        }
        Region::Statement(text) => body
            .lines()
            .find(|line| line.contains(text))
            .unwrap_or_else(|| panic!("no {text:?} in main()"))
            .to_string(),
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

    if matches!(which, Region::Body | Region::From(_)) {
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

/// webgpu_oit `m06`: `vec4( mix( accum.rgb / max( accum.a, 1e-5 ),
/// beauty.rgb, revealage.r ), beauty.a )` under the pipeline's output
/// transform — three taps, two of them through a `mat3x3` uv matrix.
#[test]
fn oit_composite_matches_three() {
    check("oit_composite", Region::Body);
}

/// webgpu_oit's two lights, in scene order: the `HemisphereLight`, then the
/// `DirectionalLight`.
fn oit_lights() -> Vec<LightDesc> {
    [LightKind::Hemisphere, LightKind::Directional]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| LightDesc {
            index,
            kind,
            shadow_map: None,
        })
        .collect()
}

/// One of webgpu_oit's `MeshStandardMaterial`s, built and fingerprinted
/// over `which` against three's module `fixture_name`.
///
/// The OIT gates take the tail of `main()`, from `outgoingLight` on: the
/// `Output` and the MRT members, which is everything the pass changes. The
/// lighting above it is the ordinary lit standard material, and differs from
/// the dump in the ways `docs/nodes.md` §8 lists (hoisted accumulator zeros,
/// named lighting temps, the indirect-diffuse block's position) — none of
/// them OIT's.
fn check_oit_material(
    material: &three_rs::MeshBasicNodeMaterial,
    mrt: Option<three_rs::materials::MrtContext>,
    fixture_name: &str,
    which: Region,
) {
    let ctx = SetupContext {
        lights: oit_lights(),
        mrt,
        ..SetupContext::default()
    };
    let program = NodeBuilder::new().build(&setup(material, &ctx, None));
    let ours = fingerprint(&program.fragment_wgsl, which);
    let three = fingerprint(&fixture(fixture_name), which);
    assert_eq!(ours, three, "\n{}", program.fragment_wgsl);
}

/// The material tail the OIT gates compare: see [`check_oit_material`].
const OIT_TAIL: Region = Region::From("outgoingLight =");

/// The OIT pass's MRT over a transparent material, as the renderer hands it
/// to the build: `accum` a `vec4`, `revealage` — an `r8unorm` attachment —
/// an `f32`.
fn oit_mrt() -> three_rs::materials::MrtContext {
    let oit = three_rs::nodes::display::oit_pass(
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::Scene::new())),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            45.0, 1.6, 0.1, 100.0,
        ))),
    );
    three_rs::materials::MrtContext {
        node: oit.mrt_node(),
        attachments: vec!["accum".to_string(), "revealage".to_string()],
        output_types: vec![three_rs::nodes::Type::Vec4, three_rs::nodes::Type::F32],
    }
}

fn oit_transparent(color: u32, roughness: f64) -> three_rs::MeshBasicNodeMaterial {
    let mut material =
        three_rs::MeshBasicNodeMaterial::standard(three_rs::Color::from_hex(color), roughness, 0.0);
    material.transparent = true;
    material.opacity = 0.5;
    material
}

/// webgpu_oit `m01`: the knot in the default pass — a plain lit standard
/// material, no MRT (`PassNode._mrt` is null).
#[test]
fn oit_default_pass_matches_three() {
    let knot =
        three_rs::MeshBasicNodeMaterial::standard(three_rs::Color::from_hex(0xffffff), 0.2, 0.0);
    check_oit_material(&knot, None, "webgpu_oit_m01_opaque.wgsl", OIT_TAIL);
}

/// webgpu_oit `m03`: a transparent sphere (or a plane's front half) in the
/// OIT pass — the `accum` / `revealage` MRT with equation (9)'s weight
/// over `positionView.z`.
#[test]
fn oit_accumulate_matches_three() {
    let sphere = oit_transparent(0xffc020, 0.3);
    check_oit_material(
        &sphere,
        Some(oit_mrt()),
        "webgpu_oit_m03_accumulate.wgsl",
        OIT_TAIL,
    );
}

/// webgpu_oit `m04`: a `DoubleSide` plane's back half in the OIT pass, whose
/// only difference from `m03` is the flipped geometry normal.
#[test]
fn oit_accumulate_back_side_matches_three() {
    let mut plane = oit_transparent(0xff4030, 0.5);
    plane.side = three_rs::materials::Side::Back;
    for which in [
        OIT_TAIL,
        Region::Statement("( normalViewGeometry * vec3<f32>( -1.0 ) )"),
    ] {
        check_oit_material(
            &plane,
            Some(oit_mrt()),
            "webgpu_oit_m04_accumulate_back.wgsl",
            which,
        );
    }
}
