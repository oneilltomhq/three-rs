//! The WGSL gate for issue #166's two rungs: what the node system generates
//! for `webgpu_compute_texture`'s kernel and material and for
//! `webgpu_volume_perlin`'s raymarch — and since, for `webgpu_texturegrad`'s
//! gradient taps — against three.js' own dumps of the same
//! pages at 5f610f5 (`tests/fixtures/textures/`, verbatim output of
//! `tools/dump-webgpu.mjs`).
//!
//! [`canonical`] is the only thing between the two texts, and every rule in it
//! is a divergence listed in `docs/nodes.md` §8:
//!
//! - the banner line, and the compute template's `enable subgroups;` and
//!   `@builtin( subgroup_size )` parameter;
//! - the numbers in `nodeUniformN` / `nodeVarN` / `nodeVaryingN` /
//!   `nodeConstN`, renumbered on both sides by order of first appearance;
//! - lines that are empty or only whitespace. Three leaves an indented blank
//!   line after the last statement of every `If` body and an empty
//!   `// directives` block in a render stage; the port leaves neither.
//!
//! The render stages are compared from `// flow` to `DiffuseColor = …`, the
//! part the page's `colorNode` generates, plus the whole uniform block; the
//! rest is the material tail every rung already shares, whose one difference
//! here (three's `let nodeConstN` for the output value where the port has a
//! var) is §8's "Usage-promoted temps".

use std::collections::HashMap;

use three_rs::materials::{setup, SetupContext};
use three_rs::nodes::wgsl::TextureKind;
use three_rs::nodes::{BindingDesc, NodeBuilder};
use three_rs::Texture;

#[path = "../examples/webgpu_compute_texture.rs"]
#[allow(dead_code)]
mod compute_texture;
#[path = "../examples/webgpu_texturegrad.rs"]
#[allow(dead_code)]
mod texturegrad;
#[path = "../examples/webgpu_volume_perlin.rs"]
#[allow(dead_code)]
mod volume_perlin;

/// Apply the rules in the module comment.
fn canonical(wgsl: &str) -> String {
    let mut out: String = wgsl
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| *line != "// directives" && *line != "enable subgroups;")
        .map(|line| {
            if line.ends_with("- Node System") {
                "// Node System"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    out = out.replace(
        ",\n\t@builtin( subgroup_size ) subgroupSize : u32 ) {",
        " ) {",
    );

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

/// The lines of `wgsl` from the one starting with `from` up to and including
/// the first one after it starting with `to`.
fn section(wgsl: &str, from: &str, to: &str) -> String {
    let lines: Vec<&str> = wgsl.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with(from))
        .unwrap_or_else(|| panic!("no line starting {from:?}"));
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.trim_start().starts_with(to))
            .unwrap_or_else(|| panic!("no line starting {to:?} after {from:?}"));
    lines[start..=end].join("\n")
}

#[track_caller]
fn assert_same(generated: &str, three: &str, what: &str) {
    let (a, b) = (canonical(generated), canonical(three));
    if a == b {
        return;
    }
    let mut report = String::new();
    for (i, (left, right)) in a.lines().zip(b.lines()).enumerate() {
        if left != right {
            report.push_str(&format!(
                "line {}:\n  port : {left:?}\n  three: {right:?}\n",
                i + 1
            ));
        }
    }
    if a.lines().count() != b.lines().count() {
        report.push_str(&format!(
            "line counts differ: port {} three {}\n",
            a.lines().count(),
            b.lines().count()
        ));
    }
    panic!("{what}: generated WGSL differs from three.js 5f610f5\n{report}");
}

/// `computeTexture( { storageTexture } ).compute( width * height )` — the
/// whole module, `texture_storage_2d<rgba8unorm, write>` and `textureStore`
/// included.
#[test]
fn compute_texture_kernel_matches_three() {
    let storage = Texture::storage(compute_texture::WIDTH, compute_texture::HEIGHT);
    let program = NodeBuilder::new().build_compute(&compute_texture::compute_texture(&storage));
    assert_same(
        &program.wgsl,
        include_str!("fixtures/textures/compute_texture.compute.wgsl"),
        "compute_texture kernel",
    );
    // `ceil( 512 * 512 / 64 )`.
    assert_eq!(program.dispatch, [4096, 1, 1]);

    // The storage binding reaches the layout as a write-only 2D storage
    // texture of the texture's own format, with no sampler beside it.
    let bindings: Vec<_> = program.groups.iter().flatten().collect();
    assert!(!bindings
        .iter()
        .any(|b| matches!(b, BindingDesc::Sampler { .. })));
    let kinds: Vec<TextureKind> = bindings
        .iter()
        .filter_map(|b| match b {
            BindingDesc::Texture { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [TextureKind::Storage {
            format: wgpu::TextureFormat::Rgba8Unorm,
            access: three_rs::nodes::StorageAccess::WriteOnly,
            dim3: false,
        }]
    );
}

/// The plane's `texture( storageTexture )`: an ordinary sampled
/// `texture_2d<f32>` with a sampler — the same texture, a different binding
/// from the kernel's.
#[test]
fn compute_texture_material_matches_three() {
    let storage = Texture::storage(compute_texture::WIDTH, compute_texture::HEIGHT);
    let material = compute_texture::material(&storage);
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let three = include_str!("fixtures/textures/compute_texture.fragment.wgsl");

    assert_same(
        &section(&program.fragment_wgsl, "// uniforms", "// vars"),
        &section(three, "// uniforms", "// vars"),
        "compute_texture fragment uniforms",
    );
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor = "),
        &section(three, "// flow", "DiffuseColor = "),
        "compute_texture fragment flow",
    );
}

/// `opaqueRaymarchingTexture` on a `texture3D()`: the `hitBox` slab test, the
/// float-stepped `Loop`, the four-step bisection with `select`, the
/// `Texture3DNode.normal()` chain, `Break()`, and the `bool` uniform stored
/// as a `u32`.
#[test]
fn volume_perlin_fragment_matches_three() {
    let texture = volume_perlin::volume_texture();
    let material = volume_perlin::material(&texture);
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let three = include_str!("fixtures/textures/volume_perlin.fragment.wgsl");

    assert_same(
        &section(&program.fragment_wgsl, "// uniforms", "// vars"),
        &section(three, "// uniforms", "// vars"),
        "volume_perlin fragment uniforms",
    );
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor = "),
        &section(three, "// flow", "DiffuseColor = "),
        "volume_perlin fragment flow",
    );
}

/// `RaymarchingBox`'s two varyings: the camera in the box's local space and
/// the direction to the vertex from it. Three's dump puts the first through a
/// `let` before the varying write (§8, "Property-assignment temps"); the
/// values are the same expressions.
#[test]
fn volume_perlin_vertex_writes_the_ray_varyings() {
    let texture = volume_perlin::volume_texture();
    let material = volume_perlin::material(&texture);
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let three = include_str!("fixtures/textures/volume_perlin.vertex.wgsl");

    let origin = "( object.nodeUniform0 * vec4<f32>( render.cameraPosition, 1.0 ) ).xyz;";
    assert!(three.contains(origin));
    assert!(program
        .vertex_wgsl
        .contains(&format!("\tvaryings.nodeVarying0 = {origin}")));
    assert!(three.contains("( position - varyings.nodeVarying3 );"));
    assert!(program
        .vertex_wgsl
        .contains("\tvaryings.nodeVarying1 = ( position - varyings.nodeVarying0 );"));
}

/// `ImprovedNoise` through the page's `Uint8Array` fill: a checksum of the
/// whole 128³ volume, and three texels, computed by running the page's loop
/// under Node against three's `ImprovedNoise.js`. The checksum is
/// `( sum * 31 + byte ) >>> 0` over the bytes in order.
#[test]
fn volume_data_matches_three() {
    let data = volume_perlin::volume_data();
    assert_eq!(data.len(), 128 * 128 * 128);
    assert_eq!(data[0], 128);
    assert_eq!(data[12345], 143);
    assert_eq!(data[2_000_000], 119);
    let checksum = data.iter().fold(0u32, |sum, &b| {
        sum.wrapping_mul(31).wrapping_add(u32::from(b))
    });
    assert_eq!(checksum, 2_534_957_684);
}

/// Parse and validate a generated module the way wgpu will, so a binding or
/// builtin the generator gets wrong fails here rather than at pipeline
/// creation.
fn validate(wgsl: &str, what: &str) {
    use wgpu::naga;
    let module = naga::front::wgsl::parse_str(wgsl)
        .unwrap_or_else(|e| panic!("{what}: {}\n{wgsl}", e.emit_to_string(wgsl)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{what}: {e:?}\n{wgsl}"));
}

/// `Storage3DTexture`, which no graded rung reaches yet (three's
/// `webgpu_compute_texture_3d` also needs `CanvasTexture` and MaterialX noise,
/// see `docs/nodes.md` §31): a kernel that `textureStore`s into it through a
/// `vec3<u32>` coordinate, and a material that samples the same texture with
/// `texture3D( t, null, 0 )`. Both modules must validate, the kernel's binding
/// must be a write-only 3D storage texture of the texture's format, and the
/// material's a filterable `texture_3d<f32>` with a sampler.
#[test]
fn storage_3d_texture_is_written_and_sampled() {
    use three_rs::nodes::tsl::{
        float, instance_index, storage_texture_3d, texture_3d, texture_store, to_const, uint,
        vec3_join, vec4_join,
    };
    use three_rs::nodes::{ComputeFlow, StorageAccess, Type};
    use three_rs::Data3DTexture;

    let volume = Data3DTexture::storage(8, 8, 8);
    let storage = storage_texture_3d(&volume);
    let i = instance_index();
    let x = to_const(None, i.modulo(uint(8)));
    let y = to_const(None, i.div(uint(8)).modulo(uint(8)));
    let z = to_const(None, i.div(uint(64)));
    let value = vec4_join(vec![
        x.to(Type::F32).div(8.0),
        y.to(Type::F32).div(8.0),
        z.to(Type::F32).div(8.0),
        float(1.0),
    ]);
    let store = texture_store(&storage, vec3_join(vec![x, y, z]), value);
    let kernel = NodeBuilder::new().build_compute(&ComputeFlow {
        statements: vec![store],
        count: 512,
        workgroup_size: [64, 1, 1],
        name: None,
        on_init: None,
    });
    validate(&kernel.wgsl, "storage 3D kernel");
    assert!(kernel
        .wgsl
        .contains("texture_storage_3d<rgba8unorm, write>;"));
    assert!(kernel.wgsl.contains("vec3<u32>( "));
    assert_eq!(kernel.dispatch, [8, 1, 1]);
    let kinds: Vec<TextureKind> = kernel
        .groups
        .iter()
        .flatten()
        .filter_map(|b| match b {
            BindingDesc::Texture { kind, .. } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [TextureKind::Storage {
            format: wgpu::TextureFormat::Rgba8Unorm,
            access: StorageAccess::WriteOnly,
            dim3: true,
        }]
    );

    let mut material = three_rs::MeshBasicNodeMaterial::new();
    material.color_node =
        Some(texture_3d(&volume, float(0.0)).sample(three_rs::nodes::tsl::vec3s(0.5)));
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    validate(&program.fragment_wgsl, "storage 3D sampled fragment");
    validate(&program.vertex_wgsl, "storage 3D sampled vertex");
    assert!(program.fragment_wgsl.contains("texture_3d<f32>;"));
    assert!(program.fragment_wgsl.contains("textureSampleLevel( "));
}

/// `webgpu_texturegrad`'s `colorNode`: four `textureSampleGrad` taps whose
/// gradient is a var an `If` zeroes, the white seam, and the page's two
/// usage-promoted `let`s. The texture is a stand-in; only its being a
/// filterable 2-D texture reaches the WGSL.
#[test]
fn texturegrad_fragment_matches_three() {
    let map = Texture::new(4, 4, Some(vec![0; 4 * 4 * 4]));
    let mut material = three_rs::materials::MeshBasicNodeMaterial::new();
    material.color_node = Some(texturegrad::color_node(&map));
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    let three = include_str!("fixtures/textures/texturegrad.fragment.wgsl");

    assert_same(
        &section(&program.fragment_wgsl, "// uniforms", "// vars"),
        &section(three, "// uniforms", "// vars"),
        "texturegrad fragment uniforms",
    );
    assert_same(
        &section(&program.fragment_wgsl, "// flow", "DiffuseColor = "),
        &section(three, "// flow", "DiffuseColor = "),
        "texturegrad fragment flow",
    );
}
