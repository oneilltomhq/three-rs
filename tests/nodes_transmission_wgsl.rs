//! The WGSL gate for `webgpu_materials_transmission`: what the node system
//! generates for the page's `transmission: 1` sphere against three.js' own
//! dumps at 5f610f5, verbatim output of `tools/dump-webgpu.mjs`:
//!
//! - `tests/fixtures/webgpu_materials_transmission/sphere_back.fragment.wgsl`
//!   — `m12`, the `BackSide` half of the transparent `DoubleSide` split;
//! - `tests/fixtures/webgpu_materials_transmission/sphere_front.fragment.wgsl`
//!   — `m14`, the `FrontSide` half.
//!
//! The two are one material: they differ only in the back half's flipped
//! normal and in the order of two `render` uniform members. Which viewport
//! texture each half samples (`viewportBackSideTexture` against
//! `viewportFrontSideTexture`) is a binding, not WGSL; that, and when each is
//! copied, is gated by the page's frame in `tests/e2e`.
//!
//! `main` is not compared whole: three dedups repeated reads into
//! `let nodeConstN = …` where the port keeps `nodeVarN = …` (`docs/nodes.md`
//! §8, "Usage-promoted temps"), reuses one `specularIntensity` uniform where
//! the port reads two, and lays `textureBicubicLevel`'s two levels out as one
//! `vec4` (`xy` for `floor( lod )`, `zw` for `ceil( lod )`) where the port
//! computes each level on its own — the same arithmetic per component
//! (`docs/nodes.md` §96). What is gated, after [`canonical`] (blank lines
//! dropped, `nodeUniformN` / `nodeVaryingN` / `nodeVarN` / `nodeConstN`
//! renumbered by first appearance):
//!
//! - the `// codes` block — `getVolumeTransmissionRay`, `volumeAttenuation`
//!   and `applyIorToRoughness`, the three `setLayout` helpers — verbatim
//!   ([`helper_functions_match_three`]);
//! - the right-hand sides of the lines that carry the page's features: the
//!   alpha map's sample and the two `DiffuseColor.w` writes, the
//!   `IOR` / `Transmission` / `Thickness` assignments, the refraction ray, the
//!   LOD, the PMREM's two cube reads and the final `totalDiffuse` mix
//!   ([`feature_lines_match_three`]), with three's `nodeConstN` read as a
//!   `nodeVarN` ([`rhs`]);
//! - `textureBicubicLevel`'s shape: eight `textureSampleLevel` taps of the
//!   viewport texture, four at `floor( lod )` and four at `ceil( lod )`, mixed
//!   by `fract( lod )` ([`bicubic_taps_match_three`]);
//! - the back half's normal flip ([`back_side_flips_the_normal`]).

use std::collections::HashMap;

use three_rs::materials::transmission::OpaqueFrame;
use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext, Side};
use three_rs::math::Color;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::NodeBuilder;
use three_rs::Texture;

const THREE_BACK: &str =
    include_str!("fixtures/webgpu_materials_transmission/sphere_back.fragment.wgsl");
const THREE_FRONT: &str =
    include_str!("fixtures/webgpu_materials_transmission/sphere_front.fragment.wgsl");

/// Blank lines and the empty `// directives` block dropped; the numbered node
/// names renumbered by order of first appearance.
fn canonical(wgsl: &str) -> String {
    let mut out: String = wgsl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| *line != "// directives")
        .collect::<Vec<_>>()
        .join("\n");

    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in ["nodeUniform", "nodeVarying", "nodeVar", "nodeConst"] {
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
    out
}

/// The page's sphere — `webgpu_materials_transmission.html`'s
/// `MeshPhysicalNodeMaterial` with `side` set to one half of the split, in a
/// transmission pass with an opaque copy and the PMREM environment.
fn sphere_fragment(side: Side) -> String {
    let mut material = MeshBasicNodeMaterial::physical(Color::new(1.0, 1.0, 1.0), 0.0, 0.0);
    material.ior = 1.5;
    material.transmission = 1.0;
    material.specular_intensity = 1.0;
    material.specular_color = Color::new(1.0, 1.0, 1.0);
    material.alpha_map = Some(Texture::new(2, 2, Some(vec![0; 16])));
    let env = PmremEnvironment::from_equirectangular(&Texture::new(4, 2, Some(vec![0; 32])));
    material.pmrem_env = Some(env.handle());
    material.side = side;
    material.transparent = true;
    let ctx = SetupContext {
        viewport_opaque_mip: Some(OpaqueFrame {
            texture: Texture::render_target(800, 500, wgpu::TextureFormat::Rgba16Float),
        }),
        ..SetupContext::default()
    };
    NodeBuilder::new()
        .build(&setup(&material, &ctx, None))
        .fragment_wgsl
}

/// Both halves, each beside three's dump of it.
fn halves() -> [(&'static str, String, &'static str); 2] {
    [
        ("back", sphere_fragment(Side::Back), THREE_BACK),
        ("front", sphere_fragment(Side::Front), THREE_FRONT),
    ]
}

/// From `// codes` up to the `@fragment` attribute.
fn codes(wgsl: &str) -> &str {
    let start = wgsl.find("// codes").expect("a `// codes` block");
    let end = start
        + wgsl[start..]
            .find("@fragment")
            .expect("an `@fragment` entry");
    &wgsl[start..end]
}

/// The right-hand side of a `main` line, `let` or not, renumbered on its own.
/// Three's `nodeConstN` temps read as the port's `nodeVarN` ones (§8,
/// "Usage-promoted temps"): `nodeConst3` becomes `nodeVar03`, a name no
/// `nodeVarN` can have, before the renumbering.
fn rhs(line: &str) -> String {
    let (_, value) = line.split_once(" = ").expect("an assignment");
    canonical(&value.replace("nodeConst", "nodeVar0"))
}

/// The left-hand name of an assignment, without `let `.
fn lhs(line: &str) -> &str {
    let (name, _) = line.trim_start().split_once(" = ").expect("an assignment");
    name.strip_prefix("let ").unwrap_or(name)
}

#[test]
fn helper_functions_match_three() {
    for (side, port, three) in halves() {
        assert!(codes(three).contains("fn applyIorToRoughness"), "{side}");
        assert_eq!(
            canonical(codes(&port)),
            canonical(codes(three)),
            "{side}: port:\n{}\nthree:\n{}",
            codes(&port),
            codes(three)
        );
    }
}

#[test]
fn feature_lines_match_three() {
    let wanted = |line: &str| {
        let line = line.trim_start();
        line.starts_with("DiffuseColor.w = ")
            || line.starts_with("IOR = ")
            || line.starts_with("Transmission = ")
            || line.starts_with("Thickness = ")
            || line.starts_with("totalDiffuse = ")
            || line.contains("getVolumeTransmissionRay(")
            || line.contains("log2( cameraViewport.z )")
            // The PMREM's cube reads: `textureSampleLevel( cube, flipped
            // direction, maxLod * r * ( 2 - r ) )`.
            || (line.contains("textureSampleLevel(") && line.contains("vec3<f32>( ( - "))
    };
    for (side, port, three) in halves() {
        let pick = |wgsl: &str| -> Vec<String> {
            wgsl.lines()
                .filter(|line| wanted(line) && !line.trim_start().starts_with("fn "))
                .map(rhs)
                .collect()
        };
        let (port_lines, three_lines) = (pick(&port), pick(three));
        assert_eq!(three_lines.len(), 10, "{side}: {three_lines:#?}");
        assert_eq!(port_lines, three_lines, "{side}");

        // The alpha map's sample is the line before the first `DiffuseColor.w`
        // write; three multiplies by the whole `vec4` and keeps `.x`, the
        // map's red channel (`docs/nodes.md` §96).
        let sample_of = |wgsl: &str| -> String {
            let lines: Vec<&str> = wgsl.lines().collect();
            let at = lines
                .iter()
                .position(|l| l.trim_start().starts_with("DiffuseColor.w = "))
                .expect("an opacity write");
            canonical(&format!("{}\n{}", lines[at - 1], lines[at]))
        };
        assert_eq!(sample_of(&port), sample_of(three), "{side}");
        assert!(sample_of(three).ends_with(" * nodeVar0 ) ).x;"), "{side}");
    }
}

#[test]
fn bicubic_taps_match_three() {
    for (side, port, three) in halves() {
        let shape = |wgsl: &str| -> (usize, usize, bool) {
            let lines: Vec<&str> = wgsl.lines().collect();
            let lod = lhs(lines
                .iter()
                .find(|l| l.contains("log2( cameraViewport.z )"))
                .expect("the LOD"))
            .to_string();
            let assigned = |rhs: String| -> String {
                lhs(lines
                    .iter()
                    .find(|l| l.trim_end().ends_with(&format!(" = {rhs};")))
                    .unwrap_or_else(|| panic!("{side}: no `{rhs}`")))
                .to_string()
            };
            let floor = assigned(format!("floor( {lod} )"));
            let ceil = assigned(format!("ceil( {lod} )"));
            let taps = |level: &str| {
                lines
                    .iter()
                    .filter(|l| {
                        l.contains("textureSampleLevel(") && l.ends_with(&format!(", {level} );"))
                    })
                    .count()
            };
            let mixed = lines
                .iter()
                .any(|l| l.contains(" = mix( ") && l.ends_with(&format!("fract( {lod} ) );")));
            (taps(&floor), taps(&ceil), mixed)
        };
        assert_eq!(shape(three), (4, 4, true), "{side}: three");
        assert_eq!(shape(&port), shape(three), "{side}");
    }
}

#[test]
fn back_side_flips_the_normal() {
    const FLIP: &str = " = ( normalViewGeometry * vec3<f32>( -1.0 ) );";
    let flips = |wgsl: &str| wgsl.lines().any(|l| l.ends_with(FLIP));
    assert!(flips(THREE_BACK) && !flips(THREE_FRONT));
    assert!(flips(&sphere_fragment(Side::Back)));
    assert!(!flips(&sphere_fragment(Side::Front)));
}
