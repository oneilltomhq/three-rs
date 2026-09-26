//! `scene.fog`'s node and WGSL, without a GPU.
//!
//! three.js' `NodeManager.updateFog()` turns `scene.fog` into
//! `fog( color, rangeFogFactor( near, far ) )` or `fog( color,
//! densityFogFactor( density ) )`, where every parameter is a
//! `reference( …, sceneFog ).setGroup( renderGroup )` uniform. The port's
//! [`SceneFog::node`] builds the same graph; `docs/nodes.md` §28.
//!
//! These are the structural halves of the dump comparison in
//! `examples/dump_wgsl.rs` (`fog_standard_linear` / `fog_standard_exp2`,
//! against `tools/dump-pages/fog_standard_*.html`): the fog parameters are
//! members of the **render** group rather than constants, the fog statement is
//! three.js' `mix( Output.xyz, color, factor )` tail, and the node keeps its
//! identity so a new fog value is a uniform write, not a new program.

use three_rs::lights::LightKind;
use three_rs::materials::phong::LightDesc;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::tsl::FogNode;
use three_rs::nodes::{
    BindingDesc, NodeBuilder, NodeProgram as Program, UniformGroup, UniformSource,
};
use three_rs::{Fog, FogExp2, SceneFog};

/// `dump_wgsl`'s `fog_standard_*` program: a lit `MeshStandardNodeMaterial`
/// under one directional light, fogged by `fog`.
fn fogged_standard(fog: &FogNode) -> Program {
    let ctx = SetupContext {
        lights: vec![LightDesc {
            index: 0,
            kind: LightKind::Directional,
            shadow_map: None,
        }],
        ..SetupContext::default()
    };
    let material = MeshBasicNodeMaterial::standard(Color::from_hex(0xff8040), 0.5, 0.2);
    NodeBuilder::new().build(&setup(&material, &ctx, Some(fog)))
}

/// The members of the render group's uniform buffer, as `(name, source)`.
fn render_members(program: &Program) -> Vec<(String, UniformSource)> {
    program
        .groups
        .iter()
        .flatten()
        .filter_map(|binding| match binding {
            BindingDesc::Uniforms {
                group: UniformGroup::Render,
                members,
                ..
            } => Some(members.iter().map(|m| (m.name.clone(), m.source.clone()))),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The WGSL name of the render-group member fed from `source`.
fn member_for(program: &Program, source: UniformSource) -> String {
    render_members(program)
        .into_iter()
        .find(|(_, s)| *s == source)
        .unwrap_or_else(|| panic!("no render-group member reads {source:?}"))
        .0
}

/// The fragment line that assigns the fogged colour, trimmed.
fn fog_line(wgsl: &str) -> &str {
    wgsl.lines()
        .map(str::trim)
        .find(|line| line.contains("mix( Output.xyz"))
        .expect("the fragment has a fog statement")
}

#[test]
fn linear_fog_reads_render_group_uniforms() {
    let fog = SceneFog::from(Fog::new(Color::from_hex(0x4080cc), 2.0, 6.0));
    let program = fogged_standard(&fog.node());

    let color = member_for(&program, UniformSource::FogColor);
    let near = member_for(&program, UniformSource::FogNear);
    let far = member_for(&program, UniformSource::FogFar);

    // three.js r186's dump of `tools/dump-pages/fog_standard_linear.html`:
    // `nodeVar132 = vec4<f32>( mix( Output.xyz, render.nodeUniform14,
    // smoothstep( render.nodeUniform15, render.nodeUniform16, ( -
    // v_positionView.z ) ) ), Output.w );`
    let expected = format!(
        "= vec4<f32>( mix( Output.xyz, render.{color}, smoothstep( render.{near}, render.{far}, ( - v_positionView.z ) ) ), Output.w );"
    );
    let line = fog_line(&program.fragment_wgsl);
    assert!(line.ends_with(&expected), "fog line was `{line}`");

    // No fog parameter survives as a constant: 2.0 / 6.0 are nowhere in the
    // fragment, and the density uniform is not declared at all.
    assert!(!program.fragment_wgsl.contains("smoothstep( 2.0"));
    assert!(render_members(&program)
        .iter()
        .all(|(_, s)| *s != UniformSource::FogDensity));

    // The fogged colour is what the fragment writes out.
    let var = line.split(" =").next().unwrap();
    assert!(program.fragment_wgsl.contains(&format!("Output = {var};")));
}

#[test]
fn exp2_fog_reads_render_group_uniforms() {
    let fog = SceneFog::from(FogExp2::new(Color::from_hex(0x4080cc), 0.25));
    let program = fogged_standard(&fog.node());

    let color = member_for(&program, UniformSource::FogColor);
    let density = member_for(&program, UniformSource::FogDensity);

    // three.js r186: `viewZ` is a `toConst()`-ed `( - v_positionView.z )`, and
    // the factor is `1 - exp( -( density * density * viewZ * viewZ ) )`.
    let line = fog_line(&program.fragment_wgsl);
    let view_z = program
        .fragment_wgsl
        .lines()
        .map(str::trim)
        .find_map(|l| {
            l.strip_suffix(" = ( - v_positionView.z );")
                .map(|lhs| lhs.trim_start_matches("let ").to_string())
        })
        .expect("viewZ is bound once");
    let expected = format!(
        "= vec4<f32>( mix( Output.xyz, render.{color}, ( 1.0 - exp( ( - ( ( ( render.{density} * render.{density} ) * {view_z} ) * {view_z} ) ) ) ) ), Output.w );"
    );
    assert!(line.ends_with(&expected), "fog line was `{line}`");
    assert!(render_members(&program)
        .iter()
        .all(|(_, s)| *s != UniformSource::FogNear && *s != UniformSource::FogFar));
}

#[test]
fn fog_node_identity_is_stable_across_values() {
    // `getCacheNode( 'fog', sceneFog, … )`: the node — and with it the program
    // cache key — must not change when the scene's fog parameters do.
    let a = SceneFog::from(Fog::new(Color::from_hex(0x000000), 1.0, 10.0)).node();
    let b = SceneFog::from(Fog::new(Color::from_hex(0xffffff), 5.0, 50.0)).node();
    assert!(a.color.key() == b.color.key() && a.factor.key() == b.factor.key());

    let c = SceneFog::from(FogExp2::new(Color::from_hex(0x000000), 0.1)).node();
    let d = SceneFog::from(FogExp2::new(Color::from_hex(0xffffff), 0.9)).node();
    assert!(c.factor.key() == d.factor.key());

    // …and the two kinds are different programs.
    assert_ne!(a.factor.key(), c.factor.key());
}

#[test]
fn fog_node_takes_precedence_over_scene_fog_at_the_seam() {
    // `getFogNode()` is `scene.fogNode || sceneData.fogNode`. The renderer's
    // choice is one `or_else`; this pins the program side: a hand-built fog
    // node keeps its constants and pulls in no fog uniforms.
    use three_rs::nodes::tsl::{fog, range_fog_factor};
    let custom = fog(Color::from_hex(0xff00ff), range_fog_factor(12.0, 30.0));
    let program = fogged_standard(&custom);
    assert!(render_members(&program).iter().all(|(_, s)| !matches!(
        s,
        UniformSource::FogColor
            | UniformSource::FogNear
            | UniformSource::FogFar
            | UniformSource::FogDensity
    )));
}
