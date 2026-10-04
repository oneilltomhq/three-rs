//! #144's dump gate: the fragment WGSL each ported display node generates,
//! against three.js r187dev's own dump of the page that uses it
//! (`tests/fixtures/nodes_display/`, copied verbatim from
//! `tools/dump-webgpu.mjs` output). One gate, the 3dlut page's smoke, is
//! over a vertex `main()` instead.
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
#[path = "../examples/webgpu_postprocessing_3dlut.rs"]
#[allow(dead_code)]
mod webgpu_postprocessing_3dlut;
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
    /// `// code` to `return varyings;` in the vertex `main()`.
    Vertex,
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
    if let Region::Vertex = which {
        let main = &wgsl[wgsl.find("@vertex").expect("a vertex entry point")..];
        let body = &main[main.find("// code").expect("a code section")..];
        return body[..body.find("return varyings;").expect("a return")].to_string();
    }
    let main = &wgsl[wgsl.find("@fragment").expect("a fragment entry point")..];
    let body = &main[main.find("// code").expect("a code section")..];
    let body = &body[..body.find("return output;").expect("a return")];
    match which {
        Region::Body => body.to_string(),
        Region::Function(_) | Region::Vertex => unreachable!(),
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

#[test]
fn anaglyph_matches_three() {
    check("anaglyph", Region::Body);
}

#[test]
fn parallax_barrier_matches_three() {
    check("parallax_barrier", Region::Body);
}

#[test]
fn lut_3d_matches_three() {
    check("lut_3d", Region::Body);
}

#[test]
fn outline_depth_matches_three() {
    check("outline_depth", Region::Body);
}

#[test]
fn outline_prepare_mask_matches_three() {
    check("outline_prepare_mask", Region::Body);
}

#[test]
fn outline_copy_matches_three() {
    check("outline_copy", Region::Body);
}

#[test]
fn outline_edge_detection_matches_three() {
    check("outline_edge_detection", Region::Body);
}

#[test]
fn outline_separable_blur_matches_three() {
    check("outline_separable_blur", Region::Body);
}

#[test]
fn outline_separable_blur2_matches_three() {
    check("outline_separable_blur2", Region::Body);
}

#[test]
fn outline_composite_matches_three() {
    check("outline_composite", Region::Body);
}

#[test]
fn outline_output_matches_three() {
    check("outline_output", Region::Body);
}

/// The one vertex-stage gate: webgpu_postprocessing_3dlut's smoke
/// (`m03`), whose `positionNode` twists and bends the plane by three plain
/// texture samples. Outside the fragment stage three's builder emits them as
/// `textureSampleLevel( …, 0 )`, and so must the port.
#[test]
fn lut_3d_smoke_vertex_matches_three() {
    let material = webgpu_postprocessing_3dlut::smoke_material();
    let program = NodeBuilder::new().build(&setup(&material, &SetupContext::default(), None));
    let ours = fingerprint(&program.vertex_wgsl, Region::Vertex);
    let three = fingerprint(
        &fixture("webgpu_postprocessing_3dlut_m03_smoke_vertex.wgsl"),
        Region::Vertex,
    );
    assert_eq!(
        ours.calls.get("textureSampleLevel"),
        Some(&3),
        "\n{}",
        program.vertex_wgsl
    );
    assert_eq!(ours, three, "\n{}", program.vertex_wgsl);
}

#[test]
fn sss_matches_three() {
    check("sss", Region::Body);
}

/// The page's ground in the scene pass: a Phong floor under a hemisphere
/// light and the shadow-casting directional light, in linear fog, with
/// `builtinShadowContext( sss.r, dirLight )` multiplied into the directional
/// light's colour after its shadow-map factor.
#[test]
fn sss_shadow_context_matches_three() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use three_rs::materials::phong::ShadowMap;
    use three_rs::nodes::display::sss;
    use three_rs::nodes::tsl::screen_uv;
    use three_rs::textures::DepthTexture;
    use three_rs::{Color, Fog, MeshPhongNodeMaterial, PerspectiveCamera, SceneFog};

    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0xcbcbcb));
    material.depth_write = false;
    let light = three_rs::DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    let sss_node = sss(
        &DepthTexture::new(),
        Rc::new(RefCell::new(PerspectiveCamera::new(45.0, 1.0, 0.1, 100.0))),
        &light,
    );
    let ctx = SetupContext {
        lights: vec![
            LightDesc {
                index: 0,
                kind: LightKind::Hemisphere,
                shadow_map: None,
            },
            LightDesc {
                index: 1,
                kind: LightKind::Directional,
                shadow_map: Some(ShadowMap::Context {
                    map: Box::new(ShadowMap::Planar(DepthTexture::new())),
                    shadow: sss_node.sample(screen_uv()).x(),
                }),
            },
        ],
        ..SetupContext::default()
    };
    let fog = SceneFog::Linear(Fog::new(Color::from_hex(0xa0a0a0), 10.0, 50.0)).node();
    let program = NodeBuilder::new().build(&setup(&material, &ctx, Some(&fog)));
    // The port's Phong flow emits each accumulator's zero twice in a row:
    // `irradiance`, `directDiffuse`, `directSpecular` and `indirectDiffuse =
    // vec3<f32>( 0.0, 0.0, 0.0 );`, once from the var's lazy initialiser and
    // once from the flow's explicit zero assign (`src/materials/
    // node_material.rs`). three's dump has each once. It happens on main too,
    // with or without the shadow context: issue #281. Only an adjacent repeat
    // of the same line is dropped, so a missing or extra light term still
    // shows.
    let mut lines: Vec<&str> = program.fragment_wgsl.lines().collect();
    lines.dedup();
    let ours = fingerprint(&lines.join("\n"), Region::Body);
    let three = fingerprint(
        &fixture("webgpu_postprocessing_sss_m12_ground.wgsl"),
        Region::Body,
    );
    assert_eq!(ours, three, "\n{}", program.fragment_wgsl);
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

/// `tools/dump-pages/ssr_stochastic.html` A: the stochastic path — GGX
/// sample, jittered march, edge fade toward the BRDF-sampled environment
/// and the miss fallback.
#[test]
fn ssr_stochastic_matches_three() {
    check("ssr_stochastic", Region::Body);
}

/// `computeScreenBorderFactor`, the stochastic path's edge fade, a `Fn`
/// with a layout.
#[test]
fn ssr_screen_border_factor_matches_three() {
    check(
        "ssr_stochastic",
        Region::Function("computeScreenBorderFactor"),
    );
}

/// B: the page's `envImportanceSampling` and `binaryRefine`, `stepExponent
/// = 3` and a history (multi-bounce reprojection).
#[test]
fn ssr_stochastic_refine_matches_three() {
    check("ssr_stochastic_refine", Region::Body);
}

/// C: the mirror path with `reflectNonMetals` — no metalness discard.
#[test]
fn ssr_reflect_non_metals_matches_three() {
    check("ssr_reflect_non_metals", Region::Body);
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
) -> String {
    let ctx = SetupContext {
        lights: oit_lights(),
        mrt,
        ..SetupContext::default()
    };
    let program = NodeBuilder::new().build(&setup(material, &ctx, None));
    let ours = fingerprint(&program.fragment_wgsl, which);
    let three = fingerprint(&fixture(fixture_name), which);
    assert_eq!(ours, three, "\n{}", program.fragment_wgsl);
    program.fragment_wgsl
}

/// The `revealage` member, line for line from three's `m03` / `m04`: the
/// fingerprint leaves out the output struct and what each member is assigned
/// to, and these two lines are where `getOutputType( 1 )` — `RedFormat` —
/// shows.
const OIT_REVEALAGE_LINES: [&str; 2] = ["\t@location( 1 ) m1 : f32,", "\toutput.m1 = Output.w;"];

/// Each of `lines` is a line of three's `fixture_name`, and of `ours`.
fn check_three_lines(ours: &str, fixture_name: &str, lines: &[&str]) {
    let three = fixture(fixture_name);
    for line in lines {
        assert!(
            three.lines().any(|l| l == *line),
            "{line:?} is not a line of {fixture_name}"
        );
        assert!(
            ours.lines().any(|l| l == *line),
            "{line:?} missing:\n{ours}"
        );
    }
}

/// The material tail the OIT gates compare: see [`check_oit_material`].
const OIT_TAIL: Region = Region::From("outgoingLight =");

/// The OIT pass's MRT over a transparent material, as the renderer hands it
/// to the build: resolved against the pass's own accumulation target, so
/// `accum` (`rgba16float`) is a `vec4` and `revealage` (`r8unorm`) an `f32`
/// by the renderer's attachment → type mapping.
fn oit_mrt() -> three_rs::materials::MrtContext {
    let oit = three_rs::nodes::display::oit_pass(
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::Scene::new())),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            45.0, 1.6, 0.1, 100.0,
        ))),
    );
    let mrt = oit.mrt_context();
    assert_eq!(mrt.attachments, ["accum", "revealage"]);
    assert_eq!(
        mrt.output_types,
        [three_rs::nodes::Type::Vec4, three_rs::nodes::Type::F32]
    );
    mrt
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
    let ours = check_oit_material(
        &sphere,
        Some(oit_mrt()),
        "webgpu_oit_m03_accumulate.wgsl",
        OIT_TAIL,
    );
    check_three_lines(
        &ours,
        "webgpu_oit_m03_accumulate.wgsl",
        &OIT_REVEALAGE_LINES,
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
        let ours = check_oit_material(
            &plane,
            Some(oit_mrt()),
            "webgpu_oit_m04_accumulate_back.wgsl",
            which,
        );
        check_three_lines(
            &ours,
            "webgpu_oit_m04_accumulate_back.wgsl",
            &OIT_REVEALAGE_LINES,
        );
    }
}

#[test]
fn retro_barrel_matches_three() {
    check("retro_barrel", Region::Body);
}

#[test]
fn retro_crt_matches_three() {
    check("retro_crt", Region::Body);
}

#[test]
fn bleach_bypass_matches_three() {
    check("bleach_bypass", Region::Body);
}

#[test]
fn sepia_matches_three() {
    check("sepia", Region::Body);
}

#[test]
fn film_matches_three() {
    check("film", Region::Body);
}

#[test]
fn film_no_intensity_matches_three() {
    check("film_no_intensity", Region::Body);
}

#[test]
fn temporal_reproject_seed_matches_three() {
    check("temporal_reproject_seed", Region::Body);
}

#[test]
fn temporal_reproject_resolve_matches_three() {
    check("temporal_reproject_resolve", Region::Body);
}

#[test]
fn temporal_reproject_resolve_specular_matches_three() {
    check("temporal_reproject_resolve_specular", Region::Body);
}

#[test]
fn temporal_reproject_layout_fns_match_three() {
    for name in [
        "beautyTexelFromScreen",
        "velocityToUVOffset",
        "clipToAABB",
        "projectWorldToUV",
    ] {
        check(
            "temporal_reproject_resolve_specular",
            Region::Function(name),
        );
    }
}

#[test]
fn specular_ggx_reflection_sample_matches_three() {
    check("specular_ggx_reflection_sample", Region::Body);
}

#[test]
fn specular_sample_ggx_vndf_matches_three() {
    check(
        "specular_ggx_reflection_sample",
        Region::Function("SampleGGXVNDF"),
    );
}

#[test]
fn specular_helpers_matches_three() {
    check("specular_helpers", Region::Body);
}

#[test]
fn specular_equirect_uv_to_dir_matches_three() {
    check("specular_helpers", Region::Function("equirectUvToDir"));
}

#[test]
fn specular_equirect_dir_pdf_matches_three() {
    check("specular_helpers", Region::Function("equirectDirPdf"));
}

#[test]
fn specular_mis_power_heuristic_matches_three() {
    check("specular_helpers", Region::Function("misPowerHeuristic"));
}

#[test]
fn specular_dominant_factor_matches_three() {
    check(
        "specular_helpers",
        Region::Function("getSpecularDominantFactor"),
    );
}

#[test]
fn analytic_noise_matches_three() {
    check("analytic_noise", Region::Body);
}

#[test]
fn env_sample_reflect_matches_three() {
    check("env_sample_reflect", Region::Body);
}

#[test]
fn env_sample_brdf_matches_three() {
    check("env_sample_brdf", Region::Body);
}

#[test]
fn env_sample_mis_matches_three() {
    check("env_sample_mis", Region::Body);
}

#[test]
fn env_sample_mis_equirect_dir_pdf_matches_three() {
    check("env_sample_mis", Region::Function("equirectDirPdf"));
}

#[test]
fn env_sample_mis_power_heuristic_matches_three() {
    check("env_sample_mis", Region::Function("misPowerHeuristic"));
}

#[test]
fn sharpen_rcas_matches_three() {
    check("sharpen_rcas", Region::Body);
}

#[test]
fn sharpen_rcas_denoise_matches_three() {
    check("sharpen_rcas_denoise", Region::Body);
}

#[test]
fn recurrent_denoise_diffuse_matches_three() {
    check("recurrent_denoise_diffuse", Region::Body);
}

#[test]
fn recurrent_denoise_specular_matches_three() {
    check("recurrent_denoise_specular", Region::Body);
}

#[test]
fn recurrent_denoise_diffuse_neighborhood_stats_matches_three() {
    check(
        "recurrent_denoise_diffuse",
        Region::Function("getNeighborhoodStats"),
    );
}

#[test]
fn recurrent_denoise_get_neighborhood_stats_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("getNeighborhoodStats"),
    );
}

#[test]
fn recurrent_denoise_karis_temporal_blend_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("karisTemporalBlend"),
    );
}

#[test]
fn recurrent_denoise_lobe_normal_falloff_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("lobeNormalFalloff"),
    );
}

#[test]
fn recurrent_denoise_vogel_disk_matches_three() {
    check("recurrent_denoise_specular", Region::Function("vogelDisk"));
}

#[test]
fn recurrent_denoise_diffuse_color_distance_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("diffuseColorDistance"),
    );
}

#[test]
fn recurrent_denoise_compute_hit_dist_factor_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("computeHitDistFactor"),
    );
}

#[test]
fn recurrent_denoise_specular_dominant_direction_matches_three() {
    check(
        "recurrent_denoise_specular",
        Region::Function("getSpecularDominantDirection"),
    );
}

#[test]
fn denoise_matches_three() {
    check("denoise", Region::Body);
}

#[test]
fn denoise_from_depth_matches_three() {
    check("denoise_from_depth", Region::Body);
}

#[test]
fn ssao_matches_three() {
    check("ssao", Region::Body);
}

#[test]
fn ssao_blur_matches_three() {
    check("ssao_blur", Region::Body);
}
