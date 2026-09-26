//! The shadow filters, the `filterNode` / `shadowNode` hooks and the VSM
//! blur passes (#168), pinned against lines of three.js' own WGSL for
//! `webgpu_shadowmap_vsm` and `webgpu_shadowmap_pointlight` (dumped with
//! `tools/dump-webgpu.mjs`, see `docs/dumping.md` and `docs/nodes.md` §28).
//!
//! Every line asserted verbatim below is copied from three's dump; the rest
//! are structural checks on the filter choice. No GPU is needed.

use three_rs::lights::{
    vsm_pass_horizontal, LightKind, ShadowFilter, ShadowFilterFn, ShadowFilterMap, ShadowMapType,
};
use three_rs::materials::phong::{LightDesc, ShadowMap};
use three_rs::materials::{
    quad_vertex_node, setup, shadow_material_for, MeshBasicNodeMaterial, SetupContext, Side,
};
use three_rs::math::Color;
use three_rs::nodes::tsl::float;
use three_rs::nodes::NodeBuilder;
use three_rs::textures::{DepthTexture, Texture};

fn fragment(material: &MeshBasicNodeMaterial, ctx: SetupContext, components: u32) -> String {
    let flow = setup(material, &ctx, None);
    NodeBuilder::new()
        .with_output_components(components)
        .build(&flow)
        .fragment_wgsl
}

/// One spot light at index 0 with the given shadow.
fn spot(shadow_map: ShadowMap) -> SetupContext {
    SetupContext {
        lights: vec![LightDesc {
            index: 0,
            kind: LightKind::Spot,
            shadow_map: Some(shadow_map),
        }],
        ..SetupContext::default()
    }
}

fn lit(filter: ShadowFilter) -> String {
    let material = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    fragment(
        &material,
        spot(ShadowMap::Filtered {
            map: ShadowFilterMap::Depth(DepthTexture::new()),
            filter,
        }),
        4,
    )
}

#[test]
fn pcf_soft_resolves_to_pcf() {
    assert_eq!(ShadowMapType::default(), ShadowMapType::Pcf);
    assert_eq!(ShadowMapType::PcfSoft.resolved(), ShadowMapType::Pcf);
    assert_eq!(ShadowMapType::Vsm.resolved(), ShadowMapType::Vsm);
}

#[test]
fn the_filter_follows_the_type_unless_a_filter_node_is_set() {
    assert!(matches!(
        ShadowFilter::of(ShadowMapType::Basic, None),
        ShadowFilter::Basic
    ));
    assert!(matches!(
        ShadowFilter::of(ShadowMapType::PcfSoft, None),
        ShadowFilter::Pcf
    ));
    assert!(matches!(
        ShadowFilter::of(ShadowMapType::Vsm, None),
        ShadowFilter::Vsm
    ));
    let custom = ShadowFilterFn::new(|_| float(0.5));
    assert!(matches!(
        ShadowFilter::of(ShadowMapType::Vsm, Some(&custom)),
        ShadowFilter::Custom(_)
    ));
}

#[test]
fn basic_is_one_compare_and_pcf_is_five() {
    let basic = lit(ShadowFilter::Basic);
    assert_eq!(basic.matches("textureSampleCompare(").count(), 1, "{basic}");
    assert!(!basic.contains("vogelDiskSample"), "{basic}");

    let pcf = lit(ShadowFilter::Pcf);
    assert_eq!(pcf.matches("textureSampleCompare(").count(), 5, "{pcf}");
}

#[test]
fn a_filter_node_replaces_the_taps() {
    let custom = lit(ShadowFilter::Custom(ShadowFilterFn::new(|_| float(0.25))));
    assert!(!custom.contains("textureSampleCompare("), "{custom}");
    assert!(custom.contains("0.25"), "{custom}");
}

#[test]
fn a_shadow_node_replaces_the_shadow_node() {
    let material = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    let wgsl = fragment(&material, spot(ShadowMap::Node(float(0.75))), 4);
    assert!(!wgsl.contains("textureSample"), "{wgsl}");
    assert!(!wgsl.contains("shadowPositionWorld"), "{wgsl}");
    assert!(wgsl.contains("0.75"), "{wgsl}");
}

#[test]
fn vsm_reads_the_moments_as_a_vec2() {
    let moments = Texture::new(256, 256, None);
    moments.set_format(wgpu::TextureFormat::Rg16Float);
    let material = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    let wgsl = fragment(
        &material,
        spot(ShadowMap::Filtered {
            map: ShadowFilterMap::Moments(moments),
            filter: ShadowFilter::Vsm,
        }),
        4,
    );
    assert!(!wgsl.contains("textureSampleCompare("), "{wgsl}");
    // The Chebyshev bound, remapped against light bleeding.
    assert!(wgsl.contains(" - 0.3 ) / 0.65 ), 0.0, 1.0 )"), "{wgsl}");
    let sample = wgsl
        .lines()
        .find(|line| line.contains("textureSample("))
        .expect("VSMShadowFilter samples the moments");
    assert!(sample.trim_end().ends_with(").xy ).xy;"), "{sample}");
}

/// Three's `m04_fragment_fragment_VSMHorizontal.wgsl`, line 78.
#[test]
fn the_horizontal_blur_matches_three() {
    let vertical = Texture::new(256, 256, None);
    vertical.set_format(wgpu::TextureFormat::Rg16Float);
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(vsm_pass_horizontal(1, &vertical));
    material.vertex_node = Some(quad_vertex_node());
    let wgsl = fragment(&material, SetupContext::default(), 2);
    assert!(wgsl.contains("@location( 0 ) color: vec2<f32>"), "{wgsl}");
    assert!(
        wgsl.contains("var<private> nodeVar2 : vec2<f32>;"),
        "{wgsl}"
    );
    assert!(
        wgsl.contains(
            "nodeVar2 = textureSample( nodeUniform1, nodeUniform1_sampler, ( object.nodeUniform2 * vec3<f32>( ( ( fragCoord.xy + ( vec2<f32>( ( nodeVar0 + ( f32( i ) * nodeVar1 ) ), 0.0 ) * vec2<f32>( render.nodeUniform3 ) ) ) / render.nodeUniform4 ), 1.0 ) ).xy ).xy;"
        ),
        "{wgsl}"
    );
}

/// Three's `webgpu_shadowmap_pointlight` `m04_fragment_fragment_ShadowMaterial.wgsl`:
/// the override material inherits `alphaMap` and `alphaTest`.
#[test]
fn the_shadow_pass_inherits_alpha_map_and_alpha_test() {
    let bars = Texture::new(2, 2, Some(vec![0; 16]));
    let mut cage = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
    cage.side = Side::Double;
    cage.alpha_map = Some(bars);
    cage.alpha_test = 0.5;

    let shadow = shadow_material_for(&cage, ShadowMapType::Pcf);
    assert_eq!(shadow.side, Side::Double);
    let wgsl = fragment(&shadow, SetupContext::default(), 4);
    for line in [
        "DiffuseColor = vec4<f32>( 0.0, 0.0, 0.0, 1.0 );",
        "DiffuseColor.w = ( vec4<f32>( DiffuseColor.w ) * ( vec4<f32>( object.nodeUniform0 ) * nodeVar0 ) ).x;",
        "if ( ( DiffuseColor.w <= object.nodeUniform3 ) ) {",
    ] {
        assert!(wgsl.contains(line), "missing `{line}` in\n{wgsl}");
    }
    // `blending = NoBlending`: no opaque clamp after the test.
    assert!(!wgsl.contains("DiffuseColor.w = 1.0;"), "{wgsl}");
}

/// VSM keeps the material's own side for the shadow pass; the other types
/// flip it (`_shadowSide`).
#[test]
fn vsm_keeps_the_side() {
    let material = MeshBasicNodeMaterial::phong(Color::from_hex(0x999999));
    assert_eq!(
        shadow_material_for(&material, ShadowMapType::Pcf).side,
        Side::Back
    );
    assert_eq!(
        shadow_material_for(&material, ShadowMapType::Vsm).side,
        Side::Front
    );
}
