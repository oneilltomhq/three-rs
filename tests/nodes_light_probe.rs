//! `LightProbeNode`'s WGSL, pinned against three.js' own dump of
//! `webgpu_lightprobe` (`tools/dump-webgpu.mjs`, see `docs/dumping.md`), and
//! where in a lighting model's fragment the probe's irradiance lands.
//!
//! The line asserted verbatim below is copied from three's dump, with the
//! buffer's numeric suffix (a node id) taken out. No GPU is needed.

use three_rs::lights::LightKind;
use three_rs::materials::phong::LightDesc;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Color;
use three_rs::nodes::NodeBuilder;

fn fragment(material: &MeshBasicNodeMaterial, kinds: &[LightKind]) -> String {
    let ctx = SetupContext {
        lights: kinds
            .iter()
            .enumerate()
            .map(|(index, &kind)| LightDesc {
                index,
                kind,
                shadow_map: None,
            })
            .collect(),
        ..SetupContext::default()
    };
    let flow = setup(material, &ctx, None);
    NodeBuilder::new()
        .with_output_components(4)
        .build(&flow)
        .fragment_wgsl
}

/// Replaces every `NodeBuffer_<digits>` with `NodeBuffer_N`: the digits are a
/// node id, which differs between three and the port.
fn normalise(wgsl: &str) -> String {
    let mut out = String::with_capacity(wgsl.len());
    let mut rest = wgsl;
    while let Some(at) = rest.find("NodeBuffer_") {
        let (head, tail) = rest.split_at(at + "NodeBuffer_".len());
        out.push_str(head);
        let digits = tail.len() - tail.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        out.push('N');
        rest = &tail[digits..];
    }
    out.push_str(rest);
    out
}

fn lines(wgsl: &str) -> Vec<String> {
    normalise(wgsl)
        .lines()
        .map(|l| l.trim().to_owned())
        .collect()
}

/// three r187's `getShIrradianceAt` as `LightProbeNode` emits it, from
/// `webgpu_lightprobe`'s `MeshStandardNodeMaterial` fragment.
const THREE_SH_LINE: &str = "irradiance = ( irradiance + ( ( ( ( ( ( ( ( ( NodeBuffer_N.value[ 0u ].xyz * vec3<f32>( 0.886227 ) ) + ( ( NodeBuffer_N.value[ 1u ].xyz * vec3<f32>( 1.023328 ) ) * vec3<f32>( normalWorld.y ) ) ) + ( ( NodeBuffer_N.value[ 2u ].xyz * vec3<f32>( 1.023328 ) ) * vec3<f32>( normalWorld.z ) ) ) + ( ( NodeBuffer_N.value[ 3u ].xyz * vec3<f32>( 1.023328 ) ) * vec3<f32>( normalWorld.x ) ) ) + ( ( ( NodeBuffer_N.value[ 4u ].xyz * vec3<f32>( 0.858086 ) ) * vec3<f32>( normalWorld.x ) ) * vec3<f32>( normalWorld.y ) ) ) + ( ( ( NodeBuffer_N.value[ 5u ].xyz * vec3<f32>( 0.858086 ) ) * vec3<f32>( normalWorld.y ) ) * vec3<f32>( normalWorld.z ) ) ) + ( NodeBuffer_N.value[ 6u ].xyz * vec3<f32>( ( ( ( normalWorld.z * normalWorld.z ) * 0.743125 ) - 0.247708 ) ) ) ) + ( ( ( NodeBuffer_N.value[ 7u ].xyz * vec3<f32>( 0.858086 ) ) * vec3<f32>( normalWorld.x ) ) * vec3<f32>( normalWorld.z ) ) ) + ( ( NodeBuffer_N.value[ 8u ].xyz * vec3<f32>( 0.429043 ) ) * vec3<f32>( ( ( normalWorld.x * normalWorld.x ) - ( normalWorld.y * normalWorld.y ) ) ) ) ) );";

const ZERO: &str = "irradiance = vec3<f32>( 0.0, 0.0, 0.0 );";

fn position(lines: &[String], wanted: &str) -> usize {
    lines
        .iter()
        .position(|l| l == wanted)
        .unwrap_or_else(|| panic!("no line `{wanted}` in\n{}", lines.join("\n")))
}

/// The irradiance lines after the last zero must hold every add, so no add
/// is wiped by a later zero.
fn assert_no_zero_after(lines: &[String], add: usize) {
    let late_zero = lines[add..].iter().any(|l| l == ZERO);
    assert!(
        !late_zero,
        "irradiance zeroed after an add:\n{}",
        lines.join("\n")
    );
}

#[test]
fn standard_probe_line_is_three_js_verbatim() {
    let m = MeshBasicNodeMaterial::standard(Color::from_hex(0xffffff), 0.0, 0.0);
    let l = lines(&fragment(&m, &[LightKind::Probe, LightKind::Directional]));
    let add = position(&l, THREE_SH_LINE);
    let normal = position(&l, "normalWorld = normalize( ( vec4<f32>( normalView, 0.0 ) * render.cameraViewMatrix ).xyz );");
    let zero = position(&l, ZERO);
    assert!(
        zero < normal && normal < add,
        "order zero, normalWorld, add as in three"
    );
    assert_no_zero_after(&l, add);
}

#[test]
fn phong_probe_without_ambient_keeps_its_irradiance() {
    let m = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
    let l = lines(&fragment(&m, &[LightKind::Probe]));
    let add = position(&l, THREE_SH_LINE);
    assert!(position(&l, ZERO) < add);
    assert_no_zero_after(&l, add);
}

/// Regression: Phong with a hemisphere light and no ambient light used to
/// zero `irradiance` after the hemisphere's add, so the light did nothing.
#[test]
fn phong_hemisphere_without_ambient_keeps_its_irradiance() {
    let m = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
    let l = lines(&fragment(&m, &[LightKind::Hemisphere]));
    let add = l
        .iter()
        .position(|l| l.starts_with("irradiance = ( irradiance + mix("))
        .expect("hemisphere add");
    assert!(position(&l, ZERO) < add);
    assert_no_zero_after(&l, add);
}

/// Two probes read two different buffers, each added once.
#[test]
fn two_probes_add_twice() {
    let m = MeshBasicNodeMaterial::standard(Color::from_hex(0xffffff), 0.0, 0.0);
    let l = lines(&fragment(&m, &[LightKind::Probe, LightKind::Probe]));
    assert_eq!(l.iter().filter(|l| *l == THREE_SH_LINE).count(), 2);
    let raw = fragment(&m, &[LightKind::Probe, LightKind::Probe]);
    let mut ids: Vec<&str> = raw
        .match_indices("NodeBuffer_")
        .map(|(i, _)| {
            let t = &raw[i + "NodeBuffer_".len()..];
            &t[..t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len()]
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 2, "one SH buffer per probe");
}
