//! #161's gates: `Node::Custom`, `context()` and `isolate()`.
//!
//! * **`context()`**, against three's dump of `webgpu_custom_fog_background`
//!   (`m10`, the composite quad): `rangeFogFactor( 2.7, 4 ).context( {
//!   getViewZ: () => scenePassViewZ } )`, spelled the same way in the port.
//! * **`isolate()` and `context()`**, against three's dump of
//!   `tools/dump-pages/isolate_context.html`, a one-material page written for
//!   this gate because no example calls `.isolate()` itself.
//! * **`CustomNode`**: `RGBShiftNode` defined here, outside the crate, builds
//!   the same WGSL as the crate's [`rgb_shift`].
//!
//! The fixtures under `tests/fixtures/nodes_custom/` are `tools/dump-webgpu.mjs`
//! output, copied verbatim. r187dev spells a single-assignment value `let
//! nodeConstN = …;` where the port declares a `var` and assigns `nodeVarN = …;`,
//! and three numbers uniforms across the whole page, so the comparison renames
//! every `nodeVarN`, `nodeConstN` and `nodeUniformN` to one sequence in order
//! of first appearance and drops the `let`. Everything else in the compared
//! lines is compared as text.

use std::cell::Cell;
use std::rc::Rc;

use three_rs::materials::{
    quad_vertex_node, render_output, setup, MeshBasicNodeMaterial, SetupContext,
};
use three_rs::math::Color;
use three_rs::nodes::display::rgb_shift;
use three_rs::nodes::tsl::{
    custom, float, isolate, range_fog_factor, range_fog_factor_with_view_z, texture_uv, time,
    uniform_value, uv, vec2_join, vec3_join, vec4_join,
};
use three_rs::nodes::{ContextValue, CustomNode, NodeBuilder, NodeProgram, NodeRef, Type};
use three_rs::textures::Texture;
use three_rs::ToneMapping;

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/nodes_custom")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn build(material: &MeshBasicNodeMaterial) -> NodeProgram {
    NodeBuilder::new().build(&setup(material, &SetupContext::default(), None))
}

/// The fragment entry point's statements, from `// code` to `return
/// output;`, one trimmed line each, blank lines dropped.
fn body(wgsl: &str) -> Vec<String> {
    let main = &wgsl[wgsl.find("@fragment").expect("a fragment entry point")..];
    let code = &main[main.find("// code").expect("a code section")..];
    let code = &code[..code.find("return output;").expect("a return")];
    code.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Rename `nodeVarN`, `nodeConstN` and `nodeUniformN` to `vK` / `uK` in
/// order of first appearance, and drop a leading `let `.
fn canonical(lines: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut uniforms: Vec<String> = Vec::new();
    lines
        .iter()
        .map(|line| {
            let line = line.strip_prefix("let ").unwrap_or(line);
            let mut out = String::new();
            let mut rest = line;
            loop {
                let next = ["nodeVar", "nodeConst", "nodeUniform"]
                    .iter()
                    .filter_map(|p| rest.find(p).map(|at| (at, *p)))
                    .min();
                let Some((at, prefix)) = next else {
                    out.push_str(rest);
                    break;
                };
                out.push_str(&rest[..at]);
                let after = &rest[at + prefix.len()..];
                let digits = after.chars().take_while(char::is_ascii_digit).count();
                let name = &rest[at..at + prefix.len() + digits];
                let table = if prefix == "nodeUniform" {
                    &mut uniforms
                } else {
                    &mut names
                };
                let index = match table.iter().position(|n| n == name) {
                    Some(i) => i,
                    None => {
                        table.push(name.to_string());
                        table.len() - 1
                    }
                };
                let tag = if prefix == "nodeUniform" { 'u' } else { 'v' };
                out.push_str(&format!("{tag}{index}"));
                rest = &after[digits..];
            }
            out
        })
        .collect()
}

/// The first `smoothstep( … )` call in `wgsl`'s fragment body, parentheses
/// balanced.
fn smoothstep_call(wgsl: &str) -> String {
    let code = body(wgsl).join("\n");
    let start = code.find("smoothstep(").expect("a smoothstep call");
    let mut depth = 0;
    for (i, c) in code[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return code[start..=start + i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced smoothstep call")
}

// ---------------------------------------------------------------------------
// context()
// ---------------------------------------------------------------------------

/// `webgpu_custom_fog_background`'s composite: `fogFactor.mix( scenePassTM,
/// fogColor )` under `renderOutput()`, with the fog factor as `make_fog` makes
/// it from the pass's view-z node.
fn fog_composite(make_fog: impl FnOnce(NodeRef) -> NodeRef) -> NodeProgram {
    let pass = three_rs::PassNode::new();
    pass.depth_texture().set_multisample(true);
    let fog_factor = make_fog(pass.view_z_node(three_rs::renderer::DEPTH_ATTACHMENT));
    let scene_pass_tm =
        three_rs::materials::tone_mapping_node(ToneMapping::AcesFilmic, float(1.0), pass.node());
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "RenderPipeline";
    material.fragment_node = Some(render_output(
        fog_factor.mix(scene_pass_tm, Color::from_hex(0x4080cc)),
        ToneMapping::None,
    ));
    material.vertex_node = Some(quad_vertex_node());
    build(&material)
}

#[test]
fn context_installs_get_view_z_as_three_does() {
    let program = fog_composite(|view_z| {
        range_fog_factor(float(2.7), float(4.0))
            .context(ContextValue::new().set("getViewZ", view_z))
    });
    let expected = fixture("webgpu_custom_fog_background_m10_fragment_RenderPipeline.wgsl");
    let ours = smoothstep_call(&program.fragment_wgsl);
    let theirs = smoothstep_call(&expected);
    assert_eq!(
        canonical(&[ours]),
        canonical(&[theirs]),
        "the fog factor must read the pass's view z, not positionView.z"
    );
}

#[test]
fn context_form_matches_the_argument_form() {
    let context = fog_composite(|view_z| {
        range_fog_factor(float(2.7), float(4.0))
            .context(ContextValue::new().set("getViewZ", view_z))
    });
    let argument =
        fog_composite(|view_z| range_fog_factor_with_view_z(float(2.7), float(4.0), view_z));
    assert_eq!(context.fragment_wgsl, argument.fragment_wgsl);
    assert_eq!(context.vertex_wgsl, argument.vertex_wgsl);
}

#[test]
fn outside_a_context_the_fog_factor_reads_position_view() {
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(vec3_join(vec![
        float(0.0),
        float(0.0),
        range_fog_factor(float(1.0), float(3.0)),
    ]));
    let wgsl = build(&material).fragment_wgsl;
    assert!(
        smoothstep_call(&wgsl).starts_with("smoothstep( 1.0, 3.0, ( - "),
        "{wgsl}"
    );
    assert!(
        !smoothstep_call(&wgsl).contains("object."),
        "no getViewZ uniform may leak in: {wgsl}"
    );
}

// ---------------------------------------------------------------------------
// isolate()
// ---------------------------------------------------------------------------

/// `tools/dump-pages/isolate_context.html`'s material.
fn isolate_context_material() -> MeshBasicNodeMaterial {
    let a = time().sin().mul(2.0);
    let b = time().cos().mul(3.0);
    let view_z = uniform_value(Type::F32, vec![-2.0]);
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(vec3_join(vec![
        isolate(a.clone()).add(a),
        b.add(b.mul(0.5).isolate()),
        range_fog_factor(1.0, 3.0).context(ContextValue::new().set("getViewZ", view_z)),
    ]));
    material
}

#[test]
fn isolate_and_context_match_three() {
    let program = build(&isolate_context_material());
    let expected = fixture("isolate_context_m01_fragment.wgsl");
    assert_eq!(
        canonical(&body(&program.fragment_wgsl)),
        canonical(&body(&expected)),
        "\n--- port ---\n{}\n--- three ---\n{}",
        program.fragment_wgsl,
        expected
    );
}

#[test]
fn without_isolate_the_shared_node_is_one_var() {
    // The control for the gate above: the same `a.add( a )` with no isolate
    // is one var and one `sin`, so the two `sin`s there are the isolate's.
    let a = time().sin().mul(2.0);
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(vec3_join(vec![a.clone().add(a), float(0.0), float(0.0)]));
    let wgsl = build(&material).fragment_wgsl;
    assert_eq!(body(&wgsl).join("\n").matches("sin(").count(), 1, "{wgsl}");

    let wgsl = build(&isolate_context_material()).fragment_wgsl;
    let code = body(&wgsl).join("\n");
    assert_eq!(code.matches("sin(").count(), 2, "{wgsl}");
    assert_eq!(code.matches("cos(").count(), 1, "{wgsl}");
}

// ---------------------------------------------------------------------------
// CustomNode
// ---------------------------------------------------------------------------

/// `RGBShiftNode` (`examples/jsm/tsl/display/RGBShiftNode.js`) as a user of
/// the crate would write it: a struct holding the node's inputs, and a
/// `setup` that composes TSL. The crate's own [`rgb_shift`] is kept; this is
/// the proof that the escape hatch is enough to write it outside.
struct RgbShiftNode {
    texture: Texture,
    amount: NodeRef,
    angle: NodeRef,
}

impl CustomNode for RgbShiftNode {
    fn type_name(&self) -> &'static str {
        "RGBShiftNode"
    }

    fn node_type(&self) -> Type {
        Type::Vec4
    }

    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        let uv_node = uv();
        let offset = vec2_join(vec![self.angle.cos(), self.angle.sin()]).mul(self.amount.clone());
        let cr = texture_uv(&self.texture, uv_node.clone().add(offset.clone()));
        let cga = texture_uv(&self.texture, uv_node.clone());
        let cb = texture_uv(&self.texture, uv_node.sub(offset));
        vec4_join(vec![cr.x(), cga.y(), cb.z(), cga.a()])
    }
}

fn quad(fragment: NodeRef) -> NodeProgram {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(render_output(fragment, ToneMapping::None));
    material.vertex_node = Some(quad_vertex_node());
    build(&material)
}

#[test]
fn custom_rgb_shift_matches_the_crate_node() {
    let texture = Texture::render_target(256, 256, wgpu::TextureFormat::Rgba16Float);
    let crate_node = quad(rgb_shift(&texture, float(0.001), float(0.0)));
    let user_node = quad(custom(RgbShiftNode {
        texture: texture.clone(),
        amount: float(0.001),
        angle: float(0.0),
    }));
    assert_eq!(user_node.fragment_wgsl, crate_node.fragment_wgsl);
    assert_eq!(user_node.vertex_wgsl, crate_node.vertex_wgsl);
}

/// Counts its `setup` calls, and returns `builder.context.scale` (or `1`)
/// times its input.
struct Scaled {
    input: NodeRef,
    setups: Rc<Cell<u32>>,
}

impl CustomNode for Scaled {
    fn type_name(&self) -> &'static str {
        "ScaledNode"
    }

    fn node_type(&self) -> Type {
        self.input.ty()
    }

    fn setup(&self, builder: &NodeBuilder) -> NodeRef {
        self.setups.set(self.setups.get() + 1);
        let scale = builder.context("scale").unwrap_or_else(|| float(1.0));
        self.input.mul(scale)
    }
}

#[test]
fn custom_setup_runs_once_per_build_and_reads_the_context() {
    let setups = Rc::new(Cell::new(0));
    let node = custom(Scaled {
        input: time(),
        setups: setups.clone(),
    });
    let scaled = node.context(ContextValue::new().set("scale", float(4.0)));
    let mut material = MeshBasicNodeMaterial::new();
    // Reached three times: twice through the context node, once bare. The
    // first reach sets it up (under the context), and the other two reuse
    // that expansion, as three's `nodeProperties.outputNode` is reused. The
    // context node is not cacheable and is walked once, so the custom node
    // itself is counted twice and its value is one var.
    material.color_node = Some(vec3_join(vec![scaled.clone(), scaled, node]));
    let wgsl = build(&material).fragment_wgsl;
    assert_eq!(setups.get(), 1);
    let code = body(&wgsl).join("\n");
    assert_eq!(code.matches("* 4.0").count(), 1, "{wgsl}");

    // A second build sets it up again: the expansion is per build.
    build(&material);
    assert_eq!(setups.get(), 2);
}
