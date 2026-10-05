//! The thin-wrapper TSL tier (issue #141) against three.js' own WGSL.
//!
//! Every probe here is one material of `tests/fixtures/tsl_batch/probe.html`,
//! a page that sets each TSL function as a `MeshBasicNodeMaterial`'s
//! `fragmentNode`; `tests/fixtures/tsl_batch/<probe>.wgsl` is three's dumped
//! fragment shader for it (`tools/dump-webgpu.mjs` over that page). Each test
//! builds the same node graph through the port and compares the `main` body.
//! `<probe>.vertex.wgsl` is the same dump's vertex shader, kept for the
//! probes whose vertex stage the port shapes itself (`tangentWorld`'s varying,
//! `clipSpace`).
//!
//! The comparison normalises the one thing that legitimately differs between
//! the two builders — the varying's number, which depends on how many
//! varyings the vertex stage allocated first. Where the port writes a shared
//! intermediate as a `var<private> nodeVarN` and three as a `let nodeConstN`
//! (the general §8 divergence in `docs/nodes.md`), the test pins the
//! expression three emits rather than the whole body, and says so.
//!
//! No GPU: this reads the generated WGSL.

use three_rs::materials::{setup, MeshBasicNodeMaterial, SetupContext};
use three_rs::math::Matrix2;
use three_rs::nodes::builder::VertexBufferSource;
use three_rs::nodes::tsl::*;
use three_rs::nodes::{ComputeFlow, ComputeProgram, NodeBuilder, NodeProgram, NodeRef, Type};

/// The program for a `MeshBasicNodeMaterial` whose `fragmentNode` is `node`,
/// set up for a geometry with or without a `tangent` attribute.
fn program_for(node: NodeRef, has_tangent_attribute: bool) -> NodeProgram {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(node);
    let ctx = SetupContext {
        has_tangent_attribute,
        ..SetupContext::default()
    };
    let flow = setup(&material, &ctx, None);
    NodeBuilder::new().build(&flow)
}

/// The fragment shader on the probe page's default plane, which has no
/// `tangent` attribute.
fn fragment(node: NodeRef) -> String {
    program_for(node, false).fragment_wgsl
}

/// The fragment shader on the page's `tangentPlane` (`computeTangents()`).
fn fragment_with_tangents(node: NodeRef) -> String {
    program_for(node, true).fragment_wgsl
}

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/tsl_batch/{name}.wgsl",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Renames every `nodeVaryingN` to `nodeVarying` and collapses whitespace.
fn normalise(wgsl: &str) -> String {
    let mut out = String::with_capacity(wgsl.len());
    let mut rest = wgsl;
    while let Some(at) = rest.find("nodeVarying") {
        out.push_str(&rest[..at]);
        out.push_str("nodeVarying");
        rest = rest[at + "nodeVarying".len()..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The statements of `main` from `// flow` to `return output;`, normalised.
fn body(wgsl: &str) -> String {
    let start = wgsl.find("// flow").expect("no // flow");
    let end = wgsl[start..].find("return output;").expect("no return") + start;
    normalise(&wgsl[start..end])
        .replace("// flow ", "")
        .replace("// code ", "")
        .replace("// result ", "")
}

/// The `// codes` section, normalised.
fn codes(wgsl: &str) -> String {
    let start = wgsl.find("// codes").expect("no // codes");
    let end = wgsl[start..].find("@fragment").expect("no @fragment") + start;
    normalise(&wgsl[start..end])
}

/// Three's normalised body with `let {name} = X;` removed and every use of
/// `name` replaced by `X`.
///
/// Three's `ObjectRef.build()` now caches *any* cacheable node used more than once
/// in a `let nodeConstN`; the port promotes only operator, math and join nodes
/// (`docs/nodes.md` §8), so a `ConvertNode` read three times is written out
/// three times here. Same value.
fn inline_let(theirs: &str, name: &str) -> String {
    let decl = format!("let {name} = ");
    let at = theirs.find(&decl).expect("no such let");
    let value_start = at + decl.len();
    let value_end = theirs[value_start..].find("; ").unwrap() + value_start;
    let value = theirs[value_start..value_end].to_string();
    let without = format!("{}{}", &theirs[..at], &theirs[value_end + 2..]);
    without.replace(name, &value)
}

/// Asserts the port's `main` body equals three's.
fn assert_body(name: &str, node: NodeRef) {
    assert_body_of(name, fragment(node));
}

/// [`assert_body`] for a shader already built.
fn assert_body_of(name: &str, ours: String) {
    let theirs = fixture(name);
    assert_eq!(
        body(&ours),
        body(&theirs),
        "{name}: main differs\n--- port ---\n{ours}"
    );
}

/// Asserts each of three's expressions appears in the port's `main`.
fn assert_contains(name: &str, node: NodeRef, expressions: &[&str]) -> String {
    let ours = fragment(node);
    let theirs = body(&fixture(name));
    let got = body(&ours);
    for e in expressions {
        let e = normalise(e);
        assert!(
            theirs.contains(&e),
            "{name}: the test's expectation is not in three's dump: {e}"
        );
        assert!(
            got.contains(&e),
            "{name}: missing `{e}`\n--- port ---\n{ours}"
        );
    }
    ours
}

fn x() -> NodeRef {
    uv().x()
}
fn y() -> NodeRef {
    uv().y()
}

#[test]
fn constructors_color() {
    assert_body(
        "constructors_color",
        vec4_join(vec![
            color(0xff8000).add(color_rgb(0.25, 0.5, 0.75)),
            float(1.0),
        ]),
    );
}

#[test]
fn constructors_int_vectors() {
    assert_body(
        "constructors_int_vectors",
        vec4_join(vec![
            uvec3(1u32, 2u32, 3u32)
                .to_vec3()
                .add(ivec3(-1, 2, -3).to_vec3())
                .add(x().mul(10.0).to_uvec3().to_vec3()),
            ivec4(1, 2, 3, 4).w().to_float(),
        ]),
    );
}

#[test]
fn constructors_uvec_join() {
    assert_body(
        "constructors_uvec_join",
        vec4_join(vec![
            uvec2(x().mul(4.0), y().mul(4.0)).to_vec2(),
            uvec4(1u32, 2u32, 3u32, 4u32).z().to_float(),
            float(1.0),
        ]),
    );
}

#[test]
fn constructors_bool_vectors() {
    assert_body(
        "constructors_bool_vectors",
        vec4_join(vec![
            bvec2(x().greater_than(0.5), true).to_vec2(),
            bvec3(true, false, y().less_than(0.5)).xy().to_vec2(),
        ])
        .add(bool(x().greater_than(y())).to_float())
        .add(bvec4(true, false, true, false).to_vec4()),
    );
}

#[test]
fn constructors_matrices() {
    let mut m = Matrix2::default();
    m.set(1.0, 2.0, 3.0, 4.0);
    assert_body(
        "constructors_matrices",
        vec4_join(vec![
            model_world_matrix()
                .to_mat3()
                .mul(vec3_join(vec![uv(), float(1.0)])),
            float(1.0),
        ])
        .add(vec4_join(vec![mat2(m).mul(uv()), float(0.0), float(0.0)])),
    );
}

#[test]
fn constants() {
    assert_body(
        "constants",
        vec4_join(vec![
            x().mul(pi()),
            x().mul(pi2()),
            x().mul(half_pi()),
            x().add(epsilon()).min(infinity()),
        ]),
    );
}

#[test]
fn trig() {
    assert_body(
        "trig",
        vec4_join(vec![
            atan(x()).add(acos(x())).add(asin(x())).add(tan(x())),
            sinh(x()).add(cosh(x())).add(tanh(x())),
            asinh(x()).add(acosh(x().add(1.0))).add(atanh(x().mul(0.5))),
            round(x())
                .add(trunc(y()))
                .add(degrees(x()))
                .add(radians(y())),
        ]),
    );
}

#[test]
fn trig_methods() {
    // three's probe passes five components to `vec4()`, which drops the last.
    assert_body("trig_methods", vec4_join(vec![uv().atan(), uv().tan()]));
}

#[test]
fn matrices() {
    let m2 = mat2_join(vec![x(), y(), float(0.5), float(1.0)]);
    let m3 = model_world_matrix().to_mat3();
    let ours = fragment(
        vec4_join(vec![
            inverse(m2).mul(uv()),
            determinant(m3.clone()),
            float(1.0),
        ])
        .add(vec4_join(vec![
            transpose(m3.clone()).mul(vec3_join(vec![uv(), float(1.0)])),
            float(1.0),
        ]))
        .add(inverse(model_world_matrix()).mul(vec4_join(vec![uv(), float(0.0), float(1.0)])))
        .add(vec4_join(vec![
            inverse(m3).mul(vec3_join(vec![uv(), float(1.0)])),
            float(1.0),
        ])),
    );
    let theirs = fixture("matrices");
    // The polyfills are three's verbatim, in the order first used.
    assert_eq!(codes(&ours), codes(&theirs), "--- port ---\n{ours}");
    // The shared `mat3( modelWorldMatrix )` conversion is `let nodeConst0`
    // in three and written out at each use here (see `inline_let`).
    assert_eq!(
        body(&ours),
        inline_let(&body(&theirs), "nodeConst0"),
        "--- port ---\n{ours}"
    );
}

#[test]
fn powers() {
    assert_body(
        "powers",
        vec4_join(vec![
            pow2(x()),
            pow3(y()),
            pow4(x()),
            difference(x(), y()).add(cbrt(x())).add(length_sq(uv())),
        ]),
    );
}

#[test]
fn face_forward_builtin() {
    assert_body(
        "face_forward",
        vec4_join(vec![
            face_forward(
                vec3_join(vec![uv(), float(1.0)]),
                vec3(0.0, 0.0, 1.0),
                vec3_join(vec![y(), x(), float(0.5)]),
            ),
            float(1.0),
        ]),
    );
}

#[test]
fn shaping() {
    assert_body(
        "shaping",
        vec4_join(vec![
            pcurve(x(), 2.0, 3.0),
            gain(x(), 2.0),
            parabola(y(), 2.0),
            sinc(x(), float(3.0)),
        ]),
    );
}

#[test]
fn bits() {
    let i = x().mul(100.0).to_int();
    let u = y().mul(100.0).to_uint();
    let ours = fragment(vec4_join(vec![
        xor(x().greater_than(0.5), y().greater_than(0.5)).to_float(),
        bit_not(i).to_float(),
        count_one_bits(u.clone())
            .add(count_trailing_zeros(u.clone()))
            .add(count_leading_zeros(u))
            .to_float(),
        float(1.0),
    ]));
    let theirs = fixture("bits");
    // The shared `uint( y * 100 )` conversion: see `inline_let`.
    assert_eq!(
        body(&ours),
        inline_let(&body(&theirs), "nodeConst0"),
        "--- port ---\n{ours}"
    );
    assert_eq!(codes(&ours), codes(&theirs), "tsl_xor polyfill");
}

#[test]
fn bitcast() {
    assert_body(
        "bitcast",
        vec4_join(vec![
            float_bits_to_int(x()).to_float(),
            float_bits_to_uint(y()).to_float(),
            int_bits_to_float(x().mul(10.0).to_int()),
            uint_bits_to_float(y().mul(10.0).to_uint()),
        ]),
    );
}

#[test]
fn increment_decrement() {
    let i = to_var(None, x().mul(10.0).to_int());
    let a = increment(&i);
    let b = decrement(&i);
    let c = increment_before(&i);
    let d = decrement_before(&i);
    assert_body(
        "increment",
        vec4_join(vec![a.to_float(), b.to_float(), c.to_float(), d.to_float()]),
    );
}

#[test]
fn hash_and_rand() {
    let ours = assert_contains(
        "hash_rand",
        vec4_join(vec![hash(x().mul(100.0)), rand(uv()), float(0.0), float(1.0)]),
        &["fract( ( sin( tsl_mod_float( dot( nodeVarying, vec2<f32>( 12.9898, 78.233 ) ), 3.141592653589793 ) ) * 43758.5453 ) )"],
    );
    // three's `state` / `word` are `let nodeConst0/1`; here `nodeVar0/1`.
    let got = body(&ours);
    for e in [
        "nodeVar0 = ( ( u32( ( nodeVarying.x * 100.0 ) ) * 747796405u ) + 2891336453u );",
        "nodeVar1 = ( ( ( nodeVar0 >> ( ( nodeVar0 >> 28u ) + 4u ) ) ^ nodeVar0 ) * 277803737u );",
        "( f32( ( ( nodeVar1 >> 22u ) ^ nodeVar1 ) ) * 2.3283064365386963e-10 )",
    ] {
        assert!(got.contains(e), "missing `{e}`\n{ours}");
    }
}

#[test]
fn tri_noise() {
    let ours = fragment(vec4_join(vec![
        tri_noise_3d(vec3_join(vec![uv(), float(0.5)]), 1.0, time()).to_vec3(),
        float(1.0),
    ]));
    let theirs = fixture("tri_noise_3d");
    assert_eq!(codes(&ours), codes(&theirs), "--- port ---\n{ours}");
    assert_eq!(body(&ours), body(&theirs), "--- port ---\n{ours}");
}

#[test]
fn oscillators() {
    assert_body(
        "oscillators",
        vec4_join(vec![
            osc_triangle(time()),
            osc_square(time()),
            osc_sawtooth(x()),
            osc_sine(time()),
        ]),
    );
}

#[test]
fn uv_utils() {
    let ours = fragment(vec4_join(vec![
        rotate_uv(uv(), time()),
        spherize_uv(uv(), 10.0),
    ]));
    // `rotate()`'s cos / sin and `spherizeUV`'s delta / delta² are `let
    // nodeConstN` in three and `nodeVarN` here (docs/nodes.md §8).
    let theirs = body(&fixture("uv_utils"))
        .replace("let nodeConst", "nodeVar")
        .replace("nodeConst", "nodeVar");
    assert_eq!(body(&ours), theirs, "--- port ---\n{ours}");
}

#[test]
fn remap_forms() {
    assert_body(
        "remap",
        vec4_join(vec![
            x().remap(0.2, 0.8, 0.0, 1.0),
            x().remap_clamp(0.2, 0.8, 1.0, 2.0),
            remap(y(), 0.0, 1.0, 2.0, 3.0),
            remap_clamp(y(), 0.1, 0.9, 0.0, 1.0),
        ]),
    );
}

#[test]
fn timer() {
    let ours = fragment(vec4_join(vec![
        delta_time(),
        frame_id().to_float(),
        time(),
        float(1.0),
    ]));
    assert_eq!(body(&ours), body(&fixture("timer")), "--- port ---\n{ours}");
    // `frameId` is a `u32` member of the render group.
    assert!(normalise(&ours).contains("nodeUniform1 : u32"), "{ours}");
}

#[test]
fn mod_forms() {
    let ours = fragment(
        vec4_join(vec![
            uv().mod_(0.5),
            x().mod_(0.25),
            x().mul(10.0).to_int().mod_(3).to_float(),
        ])
        .add(vec4_join(vec![
            vec3_join(vec![uv(), float(1.0)]).mod_(vec3(0.5, 0.5, 0.5)),
            float(1.0),
        ]))
        .add(vec4_join(vec![uv(), float(0.0), float(1.0)]).mod_(1.0)),
    );
    let theirs = fixture("mod");
    assert_eq!(body(&ours), body(&theirs), "--- port ---\n{ours}");
    assert_eq!(codes(&ours), codes(&theirs), "--- port ---\n{ours}");
}

#[test]
fn element_forms() {
    assert_body(
        "element_forms",
        vec4_join(vec![
            x().smoothstep(0.2, 0.8),
            y().step(0.5),
            smoothstep(0.0, 1.0, uv()),
        ]),
    );
}

#[test]
fn pack_float() {
    let uv4 = || vec4_join(vec![uv(), float(0.0), float(1.0)]);
    assert_body(
        "pack_float",
        vec4_join(vec![
            pack_snorm_2x16(uv()).to_float(),
            pack_unorm_2x16(uv()).to_float(),
            pack_half_2x16(uv()).to_float(),
            pack_snorm_4x8(uv4())
                .to_float()
                .add(pack_unorm_4x8(uv4()).to_float()),
        ]),
    );
}

#[test]
fn unpack_float() {
    assert_body(
        "unpack_float",
        vec4_join(vec![
            unpack_snorm_2x16(x().mul(1000.0).to_uint()),
            unpack_unorm_2x16(y().mul(1000.0).to_uint()),
        ])
        .add(vec4_join(vec![
            unpack_half_2x16(x().mul(100.0).to_uint()),
            float(0.0),
            float(1.0),
        ]))
        .add(unpack_snorm_4x8(y().mul(100.0).to_uint()))
        .add(unpack_unorm_4x8(x().mul(10.0).to_uint())),
    );
}

#[test]
fn pack_4x8() {
    assert_body(
        "pack_4x8",
        vec4_join(vec![
            pack_4x_i8(ivec4(x().mul(10.0).to_int(), -2.0, 3.0, -4.0)).to_float(),
            pack_4x_u8(uvec4(y().mul(10.0).to_uint(), 2.0, 3.0, 4.0)).to_float(),
            pack_4x_i8_clamp(ivec4(x().mul(300.0).to_int(), -200.0, 3.0, 4.0)).to_float(),
            pack_4x_u8_clamp(uvec4(y().mul(300.0).to_uint(), 2.0, 3.0, 4.0)).to_float(),
        ]),
    );
}

#[test]
fn unpack_4x8() {
    assert_body(
        "unpack_4x8",
        unpack_4x_i8(x().mul(1000.0).to_uint())
            .to_vec4()
            .add(unpack_4x_u8(y().mul(1000.0).to_uint()).to_vec4())
            .add(vec4_join(vec![
                dot_4u8_packed(x().mul(100.0).to_uint(), y().mul(100.0).to_uint()).to_float(),
                dot_4i8_packed(x().mul(50.0).to_uint(), y().mul(50.0).to_uint()).to_float(),
                float(0.0),
                float(1.0),
            ])),
    );
}

#[test]
fn all_any() {
    assert_body(
        "all_any",
        vec4_join(vec![
            all(bvec2(x().greater_than(0.5), y().greater_than(0.5))).to_float(),
            any(bvec3(x().less_than(0.25), y().less_than(0.25), false)).to_float(),
            uv().greater_than(vec2(0.5, 0.5)).all().to_float(),
            uv().less_than(vec2(0.5, 0.5)).any().to_float(),
        ]),
    );
}

#[test]
fn transform_normal_by_view_matrices() {
    assert_body(
        "transform_normal",
        vec4_join(vec![
            transform_normal_by_view_matrix(
                vec3_join(vec![uv(), float(1.0)]),
                camera_view_matrix(),
            ),
            float(1.0),
        ])
        .add(vec4_join(vec![
            vec3_join(vec![y(), x(), float(1.0)])
                .transform_normal_by_inverse_view_matrix(camera_view_matrix()),
            float(0.0),
        ])),
    );
}

#[test]
fn deprecated_aliases() {
    assert_body(
        "deprecated_aliases",
        vec4_join(vec![
            faceforward(
                vec3_join(vec![uv(), float(1.0)]),
                vec3(0.0, 0.0, 1.0),
                vec3_join(vec![y(), x(), float(0.5)]),
            ),
            inversesqrt(x().add(1.0)),
        ]),
    );
}

/// `body` with the §8 let-vs-var divergence (`docs/nodes.md`) normalised
/// away: `let nodeConstN = X;` becomes `nodeConstN = X;`, and every
/// `nodeConstN` / `nodeVarN` is renamed `vK` in order of first appearance.
/// Three numbers its `let`s and `var`s separately; the port writes both as
/// `var`s and numbers them together, so the names differ while the
/// statements, their order and every expression match.
fn canonical(wgsl: &str) -> String {
    rename_locals(&body(wgsl).replace("let nodeConst", "nodeConst"), "v")
}

/// Every `nodeConstN` / `nodeVarN` in `text` renamed `{prefix}K` in order of
/// first appearance: [`canonical`]'s renaming.
fn rename_locals(text: &str, prefix: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let name_len = ["nodeConst", "nodeVar"].iter().find_map(|p| {
            let digits = rest
                .strip_prefix(p)?
                .bytes()
                .take_while(u8::is_ascii_digit)
                .count();
            (digits > 0).then_some(p.len() + digits)
        });
        match name_len {
            Some(len) => {
                let name = &rest[..len];
                let k = names.iter().position(|n| n == name).unwrap_or_else(|| {
                    names.push(name.to_string());
                    names.len() - 1
                });
                out.push_str(&format!("{prefix}{k}"));
                i += len;
            }
            None => {
                let c = rest.chars().next().unwrap();
                out.push(c);
                i += c.len_utf8();
            }
        }
    }
    out
}

/// [`codes`] up to the §8 let-vs-var divergence, function by function: each
/// `let nodeConstN = X;` becomes `nodeConstN = X;`, each hoisted `var nodeVarN :
/// T;` declaration is dropped, and the locals of each `fn` are renamed as
/// [`canonical`] renames `main`'s. Three numbers its `let`s and `var`s
/// separately and the port writes a shared intermediate as a hoisted `var`, so
/// the names and declarations differ while the statements, their order and
/// every expression match.
fn canonical_codes(wgsl: &str) -> String {
    let text = codes(wgsl).replace("let nodeConst", "nodeConst");
    let mut kept = Vec::new();
    for statement in text.split_inclusive("; ") {
        let declaration = statement
            .rsplit_once("var nodeVar")
            .is_some_and(|(_, rest)| !rest.contains('='));
        if declaration {
            // Keep whatever precedes the declaration in the same chunk (a
            // `fn` header or a `{`).
            kept.push(statement[..statement.rfind("var nodeVar").unwrap()].to_string());
        } else {
            kept.push(statement.to_string());
        }
    }
    let text = kept.concat();
    // `local`, not `v`: a `fn`'s own parameters can be called `v1`.
    text.split(" fn ")
        .map(|chunk| rename_locals(chunk, "local"))
        .collect::<Vec<_>>()
        .join(" fn ")
}

/// Asserts the port's `main` equals three's up to [`canonical`].
fn assert_canonical(name: &str, node: NodeRef) {
    let ours = fragment(node);
    assert_eq!(
        canonical(&ours),
        canonical(&fixture(name)),
        "{name}: main differs\n--- port ---\n{ours}"
    );
}

/// `nodeUniformN` renamed `uK` in order of first appearance.
///
/// Three numbers its unnamed uniforms across both stages, so a fragment's
/// object uniforms start wherever the vertex stage's left off (and skip the
/// vertex-only ones); the port's numbering differs while every uniform, its
/// group and its type match.
fn renumber_uniforms(text: &str) -> String {
    let mut names: Vec<String> = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("nodeUniform") {
        out.push_str(&rest[..at]);
        let tail = &rest[at + "nodeUniform".len()..];
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        let name = &rest[at..at + "nodeUniform".len() + digits];
        let k = names.iter().position(|n| n == name).unwrap_or_else(|| {
            names.push(name.to_string());
            names.len() - 1
        });
        out.push_str(&format!("u{k}"));
        rest = &tail[digits..];
    }
    out.push_str(rest);
    out
}

/// The `var<uniform>` struct members of `wgsl`, as `member -> (buffer, type)`,
/// and its bare `var name : type;` bindings (textures and samplers), with an
/// empty buffer.
fn uniform_declarations(wgsl: &str) -> std::collections::HashMap<String, (String, String)> {
    let mut members = std::collections::HashMap::new();
    for line in wgsl.lines() {
        let line = line.trim();
        let binding = if line.starts_with('@') {
            line.find(" var ").map(|at| &line[at + 1..])
        } else {
            Some(line)
        };
        if let Some((name, ty)) = binding
            .and_then(|binding| binding.strip_prefix("var "))
            .and_then(|binding| binding.trim_end_matches(';').split_once(':'))
        {
            members.insert(
                name.trim().to_string(),
                (String::new(), ty.trim().to_string()),
            );
            continue;
        }
        let Some(declaration) = line.strip_prefix("var<uniform> ") else {
            continue;
        };
        let (buffer, ty) = declaration
            .trim_end_matches(';')
            .split_once(':')
            .expect("var<uniform> name : type;");
        let open = format!("struct {} {{", ty.trim());
        let start = wgsl.find(&open).unwrap_or_else(|| panic!("no `{open}`")) + open.len();
        let end = wgsl[start..].find("};").expect("unterminated struct") + start;
        for member in wgsl[start..end].lines() {
            let member = member.trim().trim_end_matches(',');
            if let Some((name, member_ty)) = member.split_once(':') {
                members.insert(
                    name.trim().to_string(),
                    (buffer.trim().to_string(), member_ty.trim().to_string()),
                );
            }
        }
    }
    members
}

/// The buffer and declared type of each unnamed uniform `main` reads, in
/// order of first use: the order [`renumber_uniforms`] numbers them in, so
/// entry `K` is uniform `uK`. The type is `None` when the shader reads a
/// uniform it does not declare.
fn used_uniforms(wgsl: &str) -> Vec<(String, Option<String>)> {
    let declarations = uniform_declarations(wgsl);
    let text = canonical(wgsl);
    let mut seen: Vec<String> = Vec::new();
    let mut used = Vec::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("nodeUniform") {
        let tail = &rest[at + "nodeUniform".len()..];
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        let name = rest[at..at + "nodeUniform".len() + digits].to_string();
        if !seen.contains(&name) {
            let buffer = rest[..at]
                .strip_suffix('.')
                .and_then(|before| before.rsplit(|c: char| !c.is_alphanumeric()).next())
                .unwrap_or("")
                .to_string();
            let ty = declarations.get(&name).map(|(_, ty)| ty.clone());
            used.push((buffer, ty));
            seen.push(name);
        }
        rest = &tail[digits..];
    }
    used
}

/// [`assert_canonical`] with [`renumber_uniforms`] on both sides, plus each
/// renumbered uniform's buffer and type, which the renaming hides.
///
/// Three's fragment dumps do not always declare what `main` reads (the
/// `transform_normal_matrix` dump reads `object.nodeUniform0` with no object
/// struct) and can declare members `main` never reads, so the types are
/// compared uniform by uniform where three declares one, and the port must
/// declare every uniform it reads.
fn assert_renumbered(name: &str, node: NodeRef, theirs: &str) {
    let ours = fragment(node);
    assert_eq!(
        renumber_uniforms(&canonical(&ours)),
        renumber_uniforms(&canonical(theirs)),
        "{name}: main differs\n--- port ---\n{ours}"
    );
    assert_uniform_types(name, &ours, theirs);
}

/// Each uniform `main` reads, in first-use order, in the same buffer as
/// three's and of three's type where three declares it; the port must declare
/// every uniform it reads. [`renumber_uniforms`] hides all of this.
fn assert_uniform_types(name: &str, ours: &str, theirs: &str) {
    let (our_uniforms, their_uniforms) = (used_uniforms(ours), used_uniforms(theirs));
    assert_eq!(our_uniforms.len(), their_uniforms.len(), "{name}");
    for (k, (ours_k, theirs_k)) in our_uniforms.iter().zip(&their_uniforms).enumerate() {
        assert_eq!(
            ours_k.0, theirs_k.0,
            "{name}: u{k}'s buffer\n--- port ---\n{ours}"
        );
        let our_ty = ours_k
            .1
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: u{k} is read but not declared\n{ours}"));
        if let Some(their_ty) = &theirs_k.1 {
            assert_eq!(
                our_ty, their_ty,
                "{name}: u{k}'s type\n--- port ---\n{ours}"
            );
        }
    }
}

/// [`codes`] with the port's fn-local `var nodeVarN : T; nodeVarN = X;`
/// rewritten as three's `let nodeConstN = X;`, and every later `nodeVarN`
/// read as `nodeConstN` — the §8 let-vs-var divergence inside a `fn`.
fn codes_as_lets(wgsl: &str) -> String {
    let mut text = codes(wgsl);
    while let Some(at) = text.find("var nodeVar") {
        let name_end = text[at + 4..].find(' ').unwrap() + at + 4;
        let name = text[at + 4..name_end].to_string();
        let decl_end = text[at..].find("; ").unwrap() + at + 2;
        text.replace_range(at..decl_end, "");
        let k = &name["nodeVar".len()..];
        let assign = format!("{name} = ");
        let first = text[at..].find(&assign).unwrap() + at;
        text.replace_range(first..first + assign.len(), &format!("let nodeConst{k} = "));
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(i) = rest.find(&name) {
            let after = &rest[i + name.len()..];
            out.push_str(&rest[..i]);
            if after.starts_with(|c: char| c.is_ascii_digit()) {
                out.push_str(&name);
            } else {
                out.push_str(&format!("nodeConst{k}"));
            }
            rest = after;
        }
        out.push_str(rest);
        text = out;
    }
    text
}

fn filterable_map() -> three_rs::textures::Texture {
    three_rs::textures::Texture::new(16, 16, Some(vec![0; 4 * 16 * 16]))
}

#[test]
fn equirect_direction_matches() {
    // Three's `phi`, `cosPhi` and `theta` are `let`s; the port's are `var`s.
    assert_canonical(
        "equirect_direction",
        vec4_join(vec![equirect_direction(uv()), float(1.0)])
            .add(vec4_join(vec![equirect_direction(uv()), float(0.0)])),
    );
}

#[test]
fn matcap_uv_matches() {
    // The `x` axis is three's `let nodeConst0`, a `var` here.
    assert_canonical(
        "matcap_uv",
        vec4_join(vec![matcap_uv(), float(0.0), float(1.0)]),
    );
}

#[test]
fn max_mip_level_matches() {
    let map = filterable_map();
    let wgsl = fragment(vec4_join(vec![
        max_mip_level(&map),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert_eq!(body(&wgsl), body(&fixture("max_mip_level")), "{wgsl}");
    // A uniform, not a texture: no binding is declared for the map.
    assert!(!wgsl.contains("texture_2d"), "{wgsl}");
}

#[test]
fn spritesheet_uv_matches() {
    // `frameNum` is three's `let nodeConstN`, a `var` here.
    assert_canonical(
        "spritesheet_uv",
        vec4_join(vec![
            spritesheet_uv(vec2(6.0, 4.0), uv(), time()),
            spritesheet_uv(vec2(3.0, 3.0), uv(), float(0.0)),
        ]),
    );
}

#[test]
fn triplanar_textures_matches() {
    let map = filterable_map();
    assert_canonical(
        "triplanar_textures",
        triplanar_textures(
            &map,
            None,
            None,
            float(2.0),
            position_world(),
            normal_world(),
        ),
    );
}

#[test]
fn texture_bicubic_matches() {
    let map = filterable_map();
    assert_canonical("texture_bicubic", texture_bicubic(&map, uv(), float(0.5)));
}

#[test]
fn texture_3d_load_and_level() {
    let volume = three_rs::textures::Data3DTexture::new(
        vec![0; 8 * 8 * 8],
        8,
        8,
        8,
        wgpu::TextureFormat::R8Unorm,
    );
    volume.set_min_filter(three_rs::textures::MinFilter::Linear);
    volume.set_mag_filter(three_rs::textures::TextureFilter::Linear);
    let coord = join(
        three_rs::nodes::Type::IVec3,
        vec![
            x().mul(8.0).to_int(),
            y().mul(8.0).to_int(),
            float(2.0).to_int(),
        ],
    );
    let wgsl = fragment(vec4_join(vec![
        texture_3d_load(&volume, coord).x(),
        texture_3d_level(&volume, vec3_join(vec![uv(), float(0.5)]), float(1.0)).x(),
        float(0.0),
        float(1.0),
    ]));
    // Three caches the level tap's `vec3( uv, 0.5 )` in a `let`; the port
    // writes the join out at its one use (§8).
    let theirs = inline_let(&body(&fixture("texture_3d")), "nodeConst0");
    assert_eq!(body(&wgsl), theirs, "{wgsl}");
    // One binding and one sampler serve both taps.
    assert_eq!(wgsl.matches("texture_3d<f32>").count(), 1, "{wgsl}");
    assert_eq!(wgsl.matches(": sampler").count(), 1, "{wgsl}");
}

#[test]
fn bitangent_geometry_matches() {
    // The probe draws `tangentPlane`, which has tangents.
    assert_body_of(
        "bitangent_geometry",
        fragment_with_tangents(vec4_join(vec![bitangent_geometry(), float(1.0)])),
    );
}

#[test]
fn bitangent_local_matches() {
    // The probe draws `tangentPlane`, which has tangents.
    assert_body_of(
        "bitangent_local",
        fragment_with_tangents(vec4_join(vec![bitangent_local(), float(1.0)])),
    );
}

#[test]
fn bitangent_world_matches() {
    // The probe draws `tangentPlane`, which has tangents.
    assert_body_of(
        "bitangent_world",
        fragment_with_tangents(vec4_join(vec![
            bitangent_world().add(tangent_world()),
            float(1.0),
        ])),
    );
}

#[test]
fn tangent_world_matches() {
    assert_body(
        "tangent_world_frame",
        vec4_join(vec![tangent_world(), float(1.0)]),
    );
}

#[test]
fn parallax_matches() {
    // Three splats the shared `scale` var for `tangentViewFrame` but writes it
    // bare in `bitangentViewFrame`; the port splats both. `vec3 * f32` and
    // `vec3 * vec3( f32 )` are the same value.
    let theirs = fixture("parallax").replace(
        "bitangentViewFrame = ( nodeConst5 * nodeVar0 );",
        "bitangentViewFrame = ( nodeConst5 * vec3<f32>( nodeVar0 ) );",
    );
    assert_renumbered(
        "parallax",
        vec4_join(vec![
            parallax_uv(uv(), float(0.1)).xy(),
            parallax_direction().z(),
            float(1.0),
        ]),
        &theirs,
    );
}

#[test]
fn camera_near_far_and_normal_matrix_match() {
    assert_body(
        "camera_near_far",
        vec4_join(vec![
            camera_near(),
            camera_far(),
            camera_normal_matrix()
                .mul(vec3_join(vec![uv(), float(1.0)]))
                .xy(),
        ]),
    );
}

#[test]
fn model_scopes_match() {
    assert_body(
        "model_scopes",
        vec4_join(vec![
            model_direction()
                .add(model_position())
                .add(model_scale())
                .add(model_view_position()),
            model_radius(),
        ]),
    );
}

#[test]
fn object_scopes_match() {
    let target = three_rs::core::Object3D::new_node();
    target.borrow_mut().position.set(1.0, 2.0, 3.0);
    assert_body(
        "object_scopes",
        vec4_join(vec![
            object_direction(&target)
                .add(object_position(&target))
                .add(object_scale(&target))
                .add(object_view_position(&target)),
            object_radius(&target).add(1.0),
        ]),
    );
}

#[test]
fn model_view_precision_matches() {
    let local = || vec4_join(vec![position_local(), float(1.0)]);
    assert_renumbered(
        "model_view_precision",
        vec4_join(vec![
            mediump_model_view_matrix()
                .mul(local())
                .xyz()
                .add(highp_model_view_matrix().mul(local()).xyz())
                .add(highp_model_normal_view_matrix().mul(normal_local())),
            float(1.0),
        ]),
        &fixture("model_view_precision"),
    );
}

#[test]
fn transform_normal_matches() {
    assert_renumbered(
        "transform_normal_matrix",
        vec4_join(vec![
            transform_normal(vec3_join(vec![uv(), float(1.0)]), model_world_matrix())
                .add(transform_normal(
                    vec3_join(vec![y(), x(), float(1.0)]),
                    camera_view_matrix(),
                ))
                .add(vec3_join(vec![x(), float(1.0), y()]).transform_normal(model_normal_matrix())),
            float(1.0),
        ]),
        &fixture("transform_normal_matrix"),
    );
}

#[test]
fn transform_normal_to_view_matches() {
    assert_renumbered(
        "transform_normal_to_view",
        vec4_join(vec![
            transform_normal_to_view(vec3_join(vec![uv(), float(1.0)])),
            float(1.0),
        ]),
        &fixture("transform_normal_to_view"),
    );
}

#[test]
fn reflect_refract_match() {
    assert_renumbered(
        "reflect_refract",
        vec4_join(vec![
            reflect_view().add(refract_view()).add(refract_vector()),
            float(1.0),
        ]),
        &fixture("reflect_refract"),
    );
}

#[test]
fn clip_space_matches() {
    assert_body("clip_space", clip_space().div(clip_space().w()));
}

// --- display: depth conversions, blend modes, colour grading ---

#[test]
fn depth_conversions_match() {
    let view_z = || position_view().z();
    assert_body(
        "depth_conversions",
        vec4_join(vec![
            view_z_to_reversed_orthographic_depth(view_z(), camera_near(), camera_far()),
            orthographic_depth_to_view_z(x(), camera_near(), camera_far()),
            view_z_to_reversed_perspective_depth(view_z(), camera_near(), camera_far()),
            float(1.0),
        ]),
    );
}

#[test]
fn logarithmic_depth_matches() {
    assert_canonical(
        "logarithmic_depth",
        vec4_join(vec![
            view_z_to_logarithmic_depth(position_view().z(), camera_near(), camera_far()),
            logarithmic_depth_to_view_z(x(), camera_near(), camera_far()),
            float(0.0),
            float(1.0),
        ]),
    );
}

#[test]
fn blend_modes_match() {
    let a = vec3_join(vec![uv(), float(0.5)]);
    let b = vec3_join(vec![y(), float(0.25), x()]);
    let ours = fragment(
        vec4_join(vec![
            blend_burn(a.clone(), b.clone())
                .add(blend_dodge(a.clone(), b.clone()))
                .add(blend_screen(a.clone(), b.clone())),
            float(1.0),
        ])
        .add(blend_color(
            vec4_join(vec![a, x()]),
            vec4_join(vec![b, y()]),
        )),
    );
    let theirs = fixture("blend_modes");
    assert_eq!(canonical(&ours), canonical(&theirs), "--- port ---\n{ours}");
    // `fn`-local `var` read back as three's `let`: `docs/nodes.md` §8.
    assert_eq!(codes_as_lets(&ours), codes(&theirs), "--- port ---\n{ours}");
}

#[test]
fn vibrance_matches() {
    assert_canonical(
        "vibrance",
        vec4_join(vec![
            vibrance(vec3_join(vec![uv(), float(0.5)]), x())
                .add(vibrance(vec3_join(vec![y(), x(), float(0.25)]), float(0.0))),
            float(1.0),
        ]),
    );
}

#[test]
fn cdl_matches() {
    let rec709 = || vec3(0.2126, 0.7152, 0.0722);
    assert_canonical(
        "cdl",
        cdl(
            vec4_join(vec![uv(), float(0.5), float(1.0)]),
            vec3(1.1, 1.0, 0.9),
            vec3(0.1, 0.1, 0.1),
            vec3(1.2, 1.2, 1.2),
            float(0.9),
            rec709(),
        )
        .add(cdl(
            vec4_join(vec![y(), x(), float(0.25), float(1.0)]),
            vec3(1.0, 1.0, 1.0),
            vec3(0.0, 0.0, 0.0),
            vec3(1.0, 1.0, 1.0),
            float(1.0),
            rec709(),
        )),
    );
}

#[test]
fn cineon_tone_mapping_matches() {
    let ours = fragment(vec4_join(vec![
        cineon_tone_mapping(vec3_join(vec![uv(), float(0.5)]), float(1.2)),
        float(1.0),
    ]));
    let theirs = fixture("cineon_tone_mapping");
    assert_eq!(body(&ours), body(&theirs), "--- port ---\n{ours}");
    // `fn`-local `var` read back as three's `let`: `docs/nodes.md` §8.
    assert_eq!(codes_as_lets(&ours), codes(&theirs), "--- port ---\n{ours}");
}

/// `ToneMappingNode` in `CineonToneMapping` mode: `vec4(
/// cineonToneMapping( color.rgb, exposure ), color.a )`. The `fn` is the one
/// three emits; the call's argument is `color.rgb` of a `vec4`, so only the
/// call is pinned in `main`.
#[test]
fn cineon_tone_mapping_node_matches() {
    let color = vec4_join(vec![uv(), float(0.5), float(1.0)]);
    let ours = fragment(three_rs::materials::tone_mapping_node(
        three_rs::materials::ToneMapping::Cineon,
        float(1.2),
        color,
    ));
    let theirs = fixture("cineon_tone_mapping");
    // Only the call is pinned in `main`: `docs/nodes.md` §68.8.
    assert!(
        body(&ours).contains("cineonToneMapping( "),
        "--- port ---\n{ours}"
    );
    // `fn`-local `var` read back as three's `let`: `docs/nodes.md` §8.
    assert_eq!(codes_as_lets(&ours), codes(&theirs), "--- port ---\n{ours}");
}

// --- display: face direction, screen and viewport ---

#[test]
fn direction_to_face_direction_matches() {
    use three_rs::materials::Side;
    let mut material = MeshBasicNodeMaterial::new();
    material.side = Side::Double;
    material.fragment_node = Some(vec4_join(vec![
        direction_to_face_direction(vec3_join(vec![uv(), float(1.0)]), Side::Double),
        float(1.0),
    ]));
    let flow = setup(&material, &SetupContext::default(), None);
    let ours = NodeBuilder::new().build(&flow).fragment_wgsl;
    assert_eq!(
        body(&ours),
        body(&fixture("direction_to_face_direction")),
        "--- port ---\n{ours}"
    );
}

/// `directionToFaceDirection()` on a front- and a back-sided material: the
/// vector as is, and the vector times `-1`. The side is passed, not read
/// from the material (`docs/nodes.md` §68.3).
#[test]
fn direction_to_face_direction_single_sided_matches() {
    use three_rs::materials::Side;
    for (side, name) in [
        (Side::Front, "direction_to_face_direction_front"),
        (Side::Back, "direction_to_face_direction_back"),
    ] {
        let mut material = MeshBasicNodeMaterial::new();
        material.side = side;
        material.fragment_node = Some(vec4_join(vec![
            direction_to_face_direction(vec3_join(vec![uv(), float(1.0)]), side),
            float(1.0),
        ]));
        let flow = setup(&material, &SetupContext::default(), None);
        let ours = NodeBuilder::new().build(&flow).fragment_wgsl;
        assert_eq!(
            body(&ours),
            body(&fixture(name)),
            "{name}\n--- port ---\n{ours}"
        );
    }
}

#[test]
fn screen_position_matches() {
    assert_canonical(
        "screen_position",
        vec4_join(vec![
            get_screen_position(position_view(), camera_projection_matrix()),
            float(0.0),
            float(1.0),
        ]),
    );
}

#[test]
fn normal_from_depth_matches() {
    let depth = three_rs::DepthTexture::new();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_renumbered(
        "normal_from_depth",
        vec4_join(vec![
            get_normal_from_depth(uv(), &depth, camera_projection_matrix_inverse()),
            float(1.0),
        ]),
        &fixture("normal_from_depth"),
    );
}

#[test]
fn viewport_coords_match() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_renumbered(
        "viewport_coords",
        vec4_join(vec![
            viewport_uv(),
            viewport_coordinate().div(screen_size()),
        ]),
        &fixture("viewport_coords"),
    );
}

// --- lighting ---

#[test]
fn light_projection_uv_matches() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_renumbered(
        "light_projection_uv",
        vec4_join(vec![
            light_projection_uv(0, position_world()).add(light_projection_uv(
                0,
                vec4_join(vec![position_view(), float(1.0)]),
            )),
            float(1.0),
        ]),
        &fixture("light_projection_uv"),
    );
}

#[test]
fn direct_point_light_matches() {
    let (light_direction, light_color) = direct_point_light(
        vec3(1.0, 0.5, 0.25),
        vec3_join(vec![uv(), float(1.0)]),
        x(),
        float(2.0),
    );
    assert_canonical(
        "direct_point_light",
        vec4_join(vec![light_direction.add(light_color), float(1.0)]),
    );
}

#[test]
fn parallax_correct_normal_matches() {
    assert_canonical(
        "parallax_correct_normal",
        vec4_join(vec![
            get_parallax_correct_normal(
                normal_world(),
                vec3(200.0, 100.0, 100.0),
                vec3(0.0, -50.0, 0.0),
            ),
            float(1.0),
        ]),
    );
}

// --- material scopes ---

/// [`fragment`] with `node` built from, and set on, a material `configure`
/// has set up — the `MaterialNode` scopes read the material's maps.
fn material_fragment(
    configure: impl FnOnce(&mut MeshBasicNodeMaterial),
    node: impl FnOnce(&MeshBasicNodeMaterial) -> NodeRef,
) -> String {
    let mut material = MeshBasicNodeMaterial::new();
    configure(&mut material);
    material.fragment_node = Some(node(&material));
    let flow = setup(&material, &SetupContext::default(), None);
    NodeBuilder::new().build(&flow).fragment_wgsl
}

/// [`assert_renumbered`] over [`material_fragment`], three's dump first
/// passed through `fix`.
fn assert_material(
    name: &str,
    configure: impl FnOnce(&mut MeshBasicNodeMaterial),
    node: impl FnOnce(&MeshBasicNodeMaterial) -> NodeRef,
    fix: impl FnOnce(String) -> String,
) {
    let ours = material_fragment(configure, node);
    let theirs = fix(fixture(name));
    assert_eq!(
        renumber_uniforms(&canonical(&ours)),
        renumber_uniforms(&canonical(&theirs)),
        "{name}: main differs\n--- port ---\n{ours}"
    );
    assert_uniform_types(name, &ours, &theirs);
}

#[test]
fn material_defaults_match() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_defaults",
        |_| {},
        |m| {
            vec4_join(vec![
                material_normal(m)
                    .add(material_clearcoat_normal(m))
                    .add(material_light_map(m)),
                material_ao(m).add(material_specular_strength(m)),
            ])
        },
        |theirs| theirs,
    );
}

#[test]
fn material_maps_match() {
    let map = filterable_map();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_maps",
        |m| {
            m.ao_map = Some(map.clone());
            m.specular_map = Some(map.clone());
            m.light_map = Some(map.clone());
        },
        |m| {
            vec4_join(vec![
                material_light_map(m),
                material_ao(m).add(material_specular_strength(m)),
            ])
        },
        // Three gives each `texture( map )` its own `uniform( map.matrix )`;
        // the port shares one per map (`transformed_uv`). Same value.
        // `docs/nodes.md` §41, "One uv-matrix uniform per texture".
        |theirs| {
            theirs
                .replace("object.nodeUniform3 *", "object.nodeUniform1 *")
                .replace("object.nodeUniform5 *", "object.nodeUniform1 *")
        },
    );
}

#[test]
fn material_normal_maps_match() {
    let map = filterable_map();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_normal_maps",
        |m| {
            m.normal_map = Some(map.clone());
            m.clearcoat_normal_map = Some(map.clone());
        },
        |m| {
            vec4_join(vec![
                material_normal(m).add(material_clearcoat_normal(m)),
                float(1.0),
            ])
        },
        // The shared uv matrix, as in `material_maps_match` (`docs/nodes.md`
        // §41, "One uv-matrix uniform per texture"), and the splat
        // `parallax_matches` describes (`vec3 * f32` against `vec3 * vec3( f32 )`,
        // `docs/nodes.md` §68.8).
        |theirs| {
            theirs
                .replace("object.nodeUniform6 *", "object.nodeUniform4 *")
                .replace(
                    "bitangentViewFrame = ( nodeConst5 * nodeVar0 );",
                    "bitangentViewFrame = ( nodeConst5 * vec3<f32>( nodeVar0 ) );",
                )
        },
    );
}

/// `materialNormal`'s normal-map arm alone, `normalScale` read as the
/// `normalScale` uniform.
#[test]
fn material_normal_map_matches() {
    let map = filterable_map();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_normal_map",
        |m| m.normal_map = Some(map.clone()),
        |m| vec4_join(vec![material_normal(m), float(1.0)]),
        // The splat `parallax_matches` describes (`vec3 * f32` against
        // `vec3 * vec3( f32 )`, `docs/nodes.md` §68.8).
        |theirs| {
            theirs.replace(
                "bitangentViewFrame = ( nodeConst5 * nodeVar0 );",
                "bitangentViewFrame = ( nodeConst5 * vec3<f32>( nodeVar0 ) );",
            )
        },
    );
}

/// `materialNormal`'s bump-map arm: `bumpMap( bump.r, bumpScale )`.
#[test]
fn material_normal_bump_matches() {
    let map = filterable_map();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_normal_bump",
        |m| {
            m.bump_map = Some(map.clone());
            m.bump_scale = 2.0;
        },
        |m| vec4_join(vec![material_normal(m), float(1.0)]),
        // Three gives the twice-read `Hll = bump.r` a `let`; the builder does
        // not promote a swizzle, so the port reads `.x` at each use
        // (`docs/nodes.md` §8, "`toConst` on the shadow filter", and §68.8).
        |theirs| {
            theirs
                .replace("\tlet nodeConst3 = nodeVar1.x;\n", "")
                .replace("nodeConst3", "nodeVar1.x")
        },
    );
}

/// `materialNormal` and `materialClearcoatNormal` with no maps on a
/// flat-shaded material: `normalView` is `normalFlat`.
#[test]
fn material_normal_flat_matches() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_normal_flat",
        |m| m.flat_shading = true,
        |m| {
            vec4_join(vec![
                material_normal(m).add(material_clearcoat_normal(m)),
                float(1.0),
            ])
        },
        |theirs| theirs,
    );
}

/// The two maps only an accessor reads are warned about, not failed on.
#[test]
fn accessor_only_maps_pass_check_supported() {
    let map = filterable_map();
    let mut material = MeshBasicNodeMaterial::new();
    material.light_map = Some(map.clone());
    material.specular_map = Some(map);
    assert!(material.unsupported_fields().is_empty());
    assert!(material.check_supported().is_ok());
}

#[test]
fn material_point_size_matches() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_point_size",
        |m| m.size = 2.0,
        |_| {
            vec4_join(vec![
                material_point_size(),
                float(0.5),
                float(0.25),
                float(1.0),
            ])
        },
        |theirs| theirs,
    );
}

#[test]
fn point_width_matches() {
    let ours = fragment(vec4_join(vec![
        point_width(),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    let theirs = fixture("point_width");
    assert_eq!(body(&ours), body(&theirs), "--- port ---\n{ours}");
    assert!(ours.contains("var<private> pointWidth : f32;"), "{ours}");
}

// --- passes ---

#[test]
fn depth_pass_matches() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let scene: three_rs::SceneRef = Rc::new(RefCell::new(three_rs::Scene::new()));
    let camera: three_rs::CameraRef = Rc::new(RefCell::new(three_rs::PerspectiveCamera::new(
        50.0, 1.0, 0.1, 100.0,
    )));
    let depth = three_rs::depth_pass(scene, camera);
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_renumbered(
        "depth_pass",
        vec4_join(vec![
            depth.node().to(three_rs::nodes::Type::Vec3),
            float(1.0),
        ]),
        &fixture("depth_pass"),
    );
}

#[test]
fn get_texture_index_finds_attachments() {
    // `getTextureIndex()` is a CPU helper: no shader to compare. Three's
    // `textures` are `[ output, normal, emissive ]` named as below.
    use three_rs::nodes::get_texture_index;
    let names = ["output", "normal", "emissive"];
    assert_eq!(get_texture_index(&names, "output"), Some(0));
    assert_eq!(get_texture_index(&names, "emissive"), Some(2));
    assert_eq!(get_texture_index(&names, "depth"), None);
}

/// The statements of a vertex `main`, from `// flow` to its `return`,
/// normalised.
fn vertex_body(wgsl: &str) -> String {
    let start = wgsl.find("// flow").expect("no // flow");
    let end = wgsl[start..].find("return ").expect("no return") + start;
    normalise(&wgsl[start..end])
}

/// Asserts each statement is in three's vertex dump `<name>.vertex.wgsl` and
/// in the port's vertex `main`.
fn assert_vertex_contains(name: &str, ours: &str, statements: &[&str]) {
    let theirs = vertex_body(&fixture(&format!("{name}.vertex")));
    let got = vertex_body(ours);
    for statement in statements {
        let statement = normalise(statement);
        assert!(
            theirs.contains(&statement),
            "{name}: the test's expectation is not in three's vertex dump: {statement}"
        );
        assert!(
            got.contains(&statement),
            "{name}: vertex main is missing `{statement}`\n--- port ---\n{ours}"
        );
    }
}

#[test]
fn tangent_world_vertex_on_a_plane_without_tangents() {
    // The probe's default plane has no `tangent` attribute. Three's
    // `AttributeNode` then warns and writes `vec4()`'s default in its place,
    // and the vertex stage declares no input for it (three's dump has only
    // `position`); a `tangent` slot here would fail the draw for want of a
    // vertex buffer.
    let program = program_for(vec4_join(vec![tangent_world(), float(1.0)]), false);
    assert!(
        program.attributes.iter().all(|slot| slot.name != "tangent"),
        "a tangent-less geometry must not get a `tangent` slot: {:?}",
        program
            .attributes
            .iter()
            .map(|slot| &slot.name)
            .collect::<Vec<_>>()
    );
    assert!(
        !program.vertex_wgsl.contains("tangent : vec4<f32>"),
        "{}",
        program.vertex_wgsl
    );
    assert_vertex_contains(
        "tangent_world_frame",
        &program.vertex_wgsl,
        &[
            "tangentLocal = vec4<f32>( 0.0, 0.0, 0.0, 1.0 ).xyz;",
            "v_tangentView = ( modelViewMatrix * vec4<f32>( tangentLocal, 0.0 ) ).xyz;",
            "VERTEX_tangentView = normalize( v_tangentView );",
            "varyings.v_tangentWorld = normalize( ( render.cameraWorldMatrix * vec4<f32>( VERTEX_tangentView, 0.0 ) ).xyz );",
        ],
    );

    // With tangents, the same graph reads the attribute.
    let program = program_for(vec4_join(vec![tangent_world(), float(1.0)]), true);
    assert!(
        program.attributes.iter().any(|slot| slot.name == "tangent"),
        "{}",
        program.vertex_wgsl
    );
    assert!(
        vertex_body(&program.vertex_wgsl).contains("tangentLocal = tangent.xyz;"),
        "{}",
        program.vertex_wgsl
    );
}

#[test]
fn clip_space_vertex_writes_the_varying() {
    let program = program_for(clip_space().div(clip_space().w()), false);
    // Three's vertex output is the `VERTEX_`-prefixed var; the port's is not
    // (`docs/nodes.md` §67.1), so only the assignment's shape is compared.
    let theirs = vertex_body(&fixture("clip_space.vertex"));
    assert!(
        theirs.contains("varyings.v_clipSpace = VERTEX_v_modelViewProjection;"),
        "{theirs}"
    );
    let got = vertex_body(&program.vertex_wgsl);
    assert!(
        got.contains("varyings.v_clipSpace = v_modelViewProjection;"),
        "{}",
        program.vertex_wgsl
    );
    assert!(
        program
            .vertex_wgsl
            .contains("@location( 0 ) v_clipSpace : vec4<f32>"),
        "{}",
        program.vertex_wgsl
    );
}

#[test]
fn clip_space_outside_the_fragment_stage_is_zero() {
    // Three's `Fn` warns and returns `vec4()` when built in the vertex stage.
    let mut material = MeshBasicNodeMaterial::new();
    material.position_node = Some(position_local().add(clip_space().xyz()));
    let flow = setup(&material, &SetupContext::default(), None);
    let program = NodeBuilder::new().build(&flow);
    assert!(
        !program.vertex_wgsl.contains("v_clipSpace"),
        "{}",
        program.vertex_wgsl
    );
    assert!(
        program
            .vertex_wgsl
            .contains("vec4<f32>( 0.0, 0.0, 0.0, 0.0 ).xyz"),
        "{}",
        program.vertex_wgsl
    );
}

// ---------------------------------------------------------------------------
// sweep 4: utils
// ---------------------------------------------------------------------------

#[test]
fn bypass_matches() {
    // Both forms: the free function and the method. Each call is a void
    // `expression()`, which three adds to the flow as a line of its own.
    assert_body(
        "bypass",
        vec4_join(vec![
            bypass(x().mul(2.0), expression("let first = 1.0", Type::Void)),
            y().bypass(expression("let second = 2.0", Type::Void)),
            float(0.0),
            float(1.0),
        ]),
    );
}

#[test]
fn bypass_runs_its_call_once() {
    // A second read is the cached output, not a second line.
    let b = x().bypass(expression("let once = 1.0", Type::Void));
    let wgsl = fragment(vec4_join(vec![b.clone(), b, float(0.0), float(1.0)]));
    assert_eq!(wgsl.matches("let once = 1.0;").count(), 1, "{wgsl}");
}

#[test]
fn uniform_flow_matches() {
    // Three's unassigned `var<private> nodeVar0 : f32;` is in the port too.
    assert_body(
        "uniform_flow",
        vec4_join(vec![
            uniform_flow(x().greater_than(0.5).select(x().mul(2.0), y())),
            float(0.0),
            float(0.0),
            float(1.0),
        ]),
    );
    let ours = fragment(vec4_join(vec![
        x().greater_than(0.5)
            .select(x().mul(2.0), y())
            .uniform_flow(),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert!(ours.contains("var<private> nodeVar0 : f32;"), "{ours}");
    assert!(
        fixture("uniform_flow").contains("var<private> nodeVar0 : f32;"),
        "three's dump declares the unused var"
    );
}

#[test]
fn select_outside_uniform_flow_is_still_an_if() {
    let ours = fragment(vec4_join(vec![
        x().greater_than(0.5).select(x().mul(2.0), y()),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert!(ours.contains("if ( ( nodeVarying"), "{ours}");
    assert!(!ours.contains("select("), "{ours}");
}

#[test]
#[allow(deprecated)]
fn set_name_matches() {
    // Three numbers its third, unnamed, uniform `nodeUniform2` (the counter
    // runs over named uniforms too); the port's is `nodeUniform0`.
    let ours = fragment(vec4_join(vec![
        set_name(uniform_value(Type::F32, vec![0.5]), "myValue"),
        label(uniform_value(Type::F32, vec![0.25]), "otherValue"),
        uniform_value(Type::F32, vec![0.75]),
        float(1.0),
    ]));
    let theirs = fixture("set_name");
    assert_eq!(
        renumber_uniforms(&body(&ours)),
        renumber_uniforms(&body(&theirs)),
        "--- port ---\n{ours}"
    );
    for member in ["myValue : f32,", "otherValue : f32,"] {
        assert!(theirs.contains(member));
        assert!(ours.contains(member), "{member}\n{ours}");
    }
}

#[test]
fn set_name_names_only_the_first_uniform() {
    // `delete builder.context.nodeName`: the second uniform under the same
    // context is numbered as usual.
    let ours = fragment(vec4_join(vec![
        uniform_value(Type::F32, vec![0.5])
            .add(uniform_value(Type::F32, vec![0.25]))
            .set_name("first"),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert!(ours.contains("object.first"), "{ours}");
    assert!(ours.contains("object.nodeUniform0"), "{ours}");
    // Under the free function, a context, a uniform with a name of its own
    // keeps it: `this.name || builder.context.nodeName`.
    let named = uniform(
        three_rs::nodes::UniformSource::Value(vec![0.5]),
        Type::F32,
        three_rs::nodes::UniformGroup::Object,
        Some("own"),
    );
    let ours = fragment(vec4_join(vec![
        set_name(named, "ignored"),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert!(ours.contains("object.own"), "{ours}");
    assert!(!ours.contains("ignored"), "{ours}");
}

#[test]
#[allow(deprecated)]
fn set_name_method_renames_a_uniform_in_place() {
    // Three's `UniformNode.setName()` sets `this.name` and returns the node,
    // so `uniform( … ).setName( 'own' ).setName( 'second' )` is `second`, and
    // an earlier reference to the node sees the rename too.
    let original = uniform(
        three_rs::nodes::UniformSource::Value(vec![0.5]),
        Type::F32,
        three_rs::nodes::UniformGroup::Render,
        Some("own"),
    );
    let renamed = original.set_name("first").label("second");
    let ours = fragment(vec4_join(vec![original, renamed, float(0.0), float(1.0)]));
    assert!(ours.contains("render.second"), "{ours}");
    assert_eq!(ours.matches("second : f32,").count(), 1, "{ours}");
    for stale in ["own : f32", "first : f32", "render.own", "render.first"] {
        assert!(!ours.contains(stale), "{stale}\n{ours}");
    }
}

#[test]
fn unpack_rgb_to_normal_matches() {
    // Three's deprecated `colorToDirection` returns `unpackRGBToNormal( node )`,
    // so its dump is this name's dump too.
    assert_body(
        "color_direction",
        vec4_join(vec![
            unpack_rgb_to_normal(vec3_join(vec![uv(), float(0.5)]))
                .add(pack_normal_to_rgb(vec3_join(vec![y(), x(), float(1.0)]))),
            float(1.0),
        ]),
    );
}

#[test]
fn vertex_stage_matches() {
    assert_body(
        "vertex_stage",
        vec4_join(vec![vertex_stage(position_local().mul(2.0)), float(1.0)]),
    );
}

#[test]
fn unpack_normal_matches() {
    // Three's shared `xy` is a `let nodeConst0`; the port's a `var`.
    assert_canonical(
        "unpack_normal",
        vec4_join(vec![unpack_normal(uv().sub(0.5)), float(1.0)]),
    );
}

#[test]
#[allow(deprecated)]
fn color_direction_matches() {
    assert_body(
        "color_direction",
        vec4_join(vec![
            color_to_direction(vec3_join(vec![uv(), float(0.5)]))
                .add(direction_to_color(vec3_join(vec![y(), x(), float(1.0)]))),
            float(1.0),
        ]),
    );
}

#[test]
fn expression_matches() {
    assert_body(
        "expression",
        vec4_join(vec![
            expression("sin( 1.0 )", Type::F32).add(x()),
            expression("vec2<f32>( 0.25, 0.5 )", Type::Vec2),
            float(1.0),
        ]),
    );
}

#[test]
fn debug_matches_and_reports() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let seen: Rc<RefCell<Vec<(String, String)>>> = Rc::default();
    let log = seen.clone();
    let callback = three_rs::nodes::DebugCallback::new(move |info| {
        log.borrow_mut()
            .push((info.stage.to_string(), info.snippet.to_string()));
    });
    assert_body(
        "debug",
        vec4_join(vec![
            debug(x().mul(2.0), None),
            y().add(1.0).debug(Some(callback)),
            float(0.0),
            float(1.0),
        ]),
    );
    let seen = seen.borrow();
    assert!(
        seen.iter().any(|(stage, snippet)| stage == "fragment"
            && normalise(snippet) == "( nodeVarying.y + 1.0 )"),
        "{seen:?}"
    );
}

#[test]
fn sample_matches() {
    let s = sample(|coord| vec4_join(vec![coord.mul(2.0), float(0.0), float(1.0)]));
    assert_body("sample", s.node().add(s.sample(vec2_join(vec![y(), x()]))));
}

#[test]
fn wgsl_matches() {
    let helper = wgsl(
        "fn helperTwice( a : f32 ) -> f32 { return a * 2.0; }",
        vec![],
    );
    let def = wgsl_fn(
        "fn useHelper( a : f32 ) -> f32 { return helperTwice( a ); }",
        vec![helper],
    );
    let node = vec4_join(vec![
        call_wgsl(&def, vec![("a", x())]),
        float(0.0),
        float(0.0),
        float(1.0),
    ]);
    let ours = fragment(node.clone());
    assert_eq!(codes(&ours), codes(&fixture("wgsl")), "{ours}");
    assert_body("wgsl", node);
}

#[test]
fn event_nodes_emit_nothing() {
    assert_body(
        "event_nodes",
        vec4_join(vec![
            x().bypass(on_object_update(|_| {}))
                .bypass(on_before_frame_update(|_| {})),
            float(0.0),
            float(0.0),
            float(1.0),
        ]),
    );
}

// ---------------------------------------------------------------------------
// sweep 5: the lighting and material batch (`docs/nodes.md` §78)
// ---------------------------------------------------------------------------

#[test]
fn material_anisotropy_matches() {
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_anisotropy",
        |_| {},
        |m| vec4_join(vec![material_anisotropy(m), float(0.0), float(1.0)]),
        |theirs| theirs,
    );
}

#[test]
fn material_anisotropy_map_matches() {
    let map = filterable_map();
    // Unnamed uniforms renumbered in first-use order: `docs/nodes.md` §67.5.
    assert_material(
        "material_anisotropy_map",
        |m| m.anisotropy_map = Some(map.clone()),
        |m| vec4_join(vec![material_anisotropy(m), float(0.0), float(1.0)]),
        |theirs| theirs,
    );
}

#[test]
fn anisotropic_ggx_matches() {
    let node = vec4_join(vec![
        d_ggx_anisotropic(x(), y(), x().mul(0.5), y().mul(0.25), x().mul(y())),
        v_ggx_smith_correlated_anisotropic(
            x(),
            y(),
            float(0.5),
            float(0.25),
            x().mul(0.5),
            y().mul(0.5),
            x(),
            y(),
        ),
        float(0.0),
        float(1.0),
    ]);
    let ours = fragment(node.clone());
    let theirs = fixture("anisotropic_ggx");
    assert_eq!(codes_as_lets(&ours), codes(&theirs), "--- port ---\n{ours}");
    assert_body("anisotropic_ggx", node);
}

#[test]
fn schlick_to_f0_matches() {
    let node = vec4_join(vec![
        schlick_to_f0(vec3_join(vec![uv(), float(0.5)]), float(1.0), x()),
        float(1.0),
    ]);
    let ours = fragment(node.clone());
    let theirs = fixture("schlick_to_f0");
    assert_eq!(codes_as_lets(&ours), codes(&theirs), "--- port ---\n{ours}");
    assert_body("schlick_to_f0", node);
}

#[test]
fn ltc_matches() {
    let n = vec3(0.0, 0.0, 1.0);
    let v = vec3_join(vec![uv(), float(1.0)]).normalize();
    let p = vec3_join(vec![uv(), float(0.0)]);
    let corners = [
        vec3(-1.0, -1.0, 2.0),
        vec3(1.0, -1.0, 2.0),
        vec3(1.0, 1.0, 2.0),
        vec3(-1.0, 1.0, 2.0),
    ];
    let [p0, p1, p2, p3] = corners;
    let m_inv = mat3_join(vec![model_world_matrix()]);
    let node = vec4_join(vec![
        ltc_evaluate(
            n.clone(),
            v.clone(),
            p.clone(),
            m_inv,
            p0.clone(),
            p1.clone(),
            p2.clone(),
            p3.clone(),
        )
        .add(ltc_evaluate_volume(p, p0, p1, p2, p3)),
        float(1.0),
    ])
    .add(vec4_join(vec![ltc_uv(n, v, x()), float(0.0), float(0.0)]));
    let ours = fragment(node.clone());
    let theirs = fixture("ltc");
    assert_eq!(
        canonical_codes(&ours),
        canonical_codes(&theirs),
        "--- port ---\n{ours}"
    );
    assert_renumbered("ltc", node, &theirs);
}

// Sweep 6 (`docs/nodes.md` §84): the compute, storage and subgroup probes.
//
// Each is one `computeProbe( name, build )` of the probe page,
// `Fn( build )().compute( 64 ).setName( name )`, and
// `tests/fixtures/tsl_batch/<name>.compute.wgsl` is three's dumped module
// for it. Unlike the fragment probes these compare the whole module, through
// [`canonical_compute`].

fn compute_fixture(name: &str) -> String {
    fixture(&format!("{name}.compute"))
}

/// The only differences between a kernel of the port's and three's dump of
/// it, each a divergence in `docs/nodes.md` §8 / §84: the banner, the numbers
/// of names that come from counters three ran for the whole page, and — for a
/// kernel with no subgroup node — the `enable subgroups;` line and
/// `@builtin( subgroup_size )` parameter three writes into every kernel on a
/// device that has the feature (`strip_subgroups`).
fn canonical_compute(wgsl: &str, strip_subgroups: bool) -> String {
    let mut out = wgsl.replace(
        "// Three.js r187dev - Node System",
        "// three-rs - Node System",
    );
    if strip_subgroups {
        out = out.replace("enable subgroups;\n", "");
        out = out.replace(
            ",\n\t@builtin( subgroup_size ) subgroupSize : u32 ) {",
            " ) {",
        );
    }
    // Longest prefix first: `nodeVarying` must not be renumbered as `nodeVar`.
    for prefix in [
        "NodeBuffer_",
        "WorkgroupArray_",
        "nodeUniform",
        "nodeVarying",
        "nodeVar",
        "nodeConst",
    ] {
        let mut seen: std::collections::HashMap<String, usize> = Default::default();
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

/// naga's verdict on a module the port generated, with the subgroup
/// capability on. naga 30 does not implement the `enable subgroups;`
/// directive, so the renderer drops that line before handing it over
/// (`src/renderer/programs.rs` `wgsl_source`); the same is done here.
#[track_caller]
fn validate(wgsl: &str, what: &str) {
    use wgpu::naga;
    let source = wgsl.replacen("enable subgroups;\n", "", 1);
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|e| panic!("{what}: {}\n{source}", e.emit_to_string(&source)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::SUBGROUP,
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("{what}: {e:?}\n{source}"));
}

/// `Fn( () => statements )().compute( 64 ).setName( name )`.
fn kernel(name: &str, statements: Vec<NodeRef>) -> ComputeProgram {
    let mut flow = ComputeFlow::new(statements, 64);
    flow.name = Some(name.to_string());
    NodeBuilder::new().build_compute(&flow)
}

/// Asserts the port's kernel is three's module, that it does or does not use
/// subgroups, and that naga accepts it.
#[track_caller]
fn assert_kernel(name: &str, statements: Vec<NodeRef>, subgroups: bool) -> ComputeProgram {
    let program = assert_kernel_text(name, statements, subgroups);
    validate(&program.wgsl, name);
    program
}

/// [`assert_kernel`] without naga, for the kernels whose three spelling naga
/// 30 refuses (see each caller).
#[track_caller]
fn assert_kernel_text(name: &str, statements: Vec<NodeRef>, subgroups: bool) -> ComputeProgram {
    let program = kernel(name, statements);
    assert_eq!(program.subgroups, subgroups, "{name}: subgroups");
    let ours = canonical_compute(&program.wgsl, false);
    let theirs = canonical_compute(&compute_fixture(name), !subgroups);
    if ours != theirs {
        let mut report = String::new();
        for (i, (left, right)) in ours.lines().zip(theirs.lines()).enumerate() {
            if left != right {
                report.push_str(&format!(
                    "line {}:\n  port : {left:?}\n  three: {right:?}\n",
                    i + 1
                ));
            }
        }
        if ours.lines().count() != theirs.lines().count() {
            report.push_str(&format!(
                "line counts differ: port {} three {}\n",
                ours.lines().count(),
                theirs.lines().count()
            ));
        }
        panic!(
            "{name}: kernel differs from three's\n{}\n{report}",
            program.wgsl
        );
    }
    program
}

/// The page's `v()`: `float( instanceIndex )`, a fresh node per call.
fn v() -> NodeRef {
    instance_index().to(Type::F32)
}

/// `subgroupAdd`, the two scans of it, the same three of `subgroupMul`, and
/// `subgroupMin` / `subgroupMax`, behind a `workgroupBarrier()`.
#[test]
fn subgroup_arith_matches_three() {
    let out = instanced_array(64, Type::F32);
    assert_kernel(
        "subgroup_arith",
        vec![
            workgroup_barrier(),
            out.element(instance_index()).assign(
                subgroup_add(v())
                    .add(subgroup_inclusive_add(v()))
                    .add(subgroup_exclusive_add(v()))
                    .add(subgroup_mul(v()))
                    .add(subgroup_inclusive_mul(v()))
                    .add(subgroup_exclusive_mul(v()))
                    .add(subgroup_min(v()))
                    .add(subgroup_max(v())),
            ),
        ],
        true,
    );
}

/// The bitwise reductions on a `uint`, the two votes on a comparison
/// (three's JS-number operand turns `instanceIndex` into an `f32`),
/// `subgroupElect()` as a `bool` and `subgroupBallot()` as a `vec4<u32>`.
///
/// naga 30 reserves `subgroupElect` but its WGSL front end has no lowering
/// for it (`naga-30.0.1/src/front/wgsl/lower/mod.rs`, no arm beside
/// `subgroupBallot`'s), so the module is checked against three's text and
/// naga sees it without the `subgroupElect()` term.
#[test]
fn subgroup_bits_matches_three() {
    let elect_free = instanced_array(64, Type::U32);
    validate(
        &kernel(
            "subgroup_bits",
            vec![
                workgroup_barrier(),
                elect_free.element(instance_index()).assign(
                    subgroup_and(instance_index())
                        .add(subgroup_or(instance_index()))
                        .add(subgroup_xor(instance_index()))
                        .add(subgroup_all(instance_index().less_than(32.0)).to(Type::U32))
                        .add(subgroup_any(instance_index().equal(3.0)).to(Type::U32))
                        .add(subgroup_ballot(instance_index().greater_than(7.0)).x()),
                ),
            ],
        )
        .wgsl,
        "subgroup_bits without subgroupElect",
    );
    let out = instanced_array(64, Type::U32);
    assert_kernel_text(
        "subgroup_bits",
        vec![
            workgroup_barrier(),
            out.element(instance_index()).assign(
                subgroup_and(instance_index())
                    .add(subgroup_or(instance_index()))
                    .add(subgroup_xor(instance_index()))
                    .add(subgroup_all(instance_index().less_than(32.0)).to(Type::U32))
                    .add(subgroup_any(instance_index().equal(3.0)).to(Type::U32))
                    .add(subgroup_elect().to(Type::U32))
                    .add(subgroup_ballot(instance_index().greater_than(7.0)).x()),
            ),
        ],
        true,
    );
}

/// The broadcasts and shuffles: a JS-number lane id is built as an `int`,
/// `subgroupShuffle`'s type is its id's on a tie (`getInputType()` prefers
/// `b`), so the `float` value goes in as `i32( … )`, and the masks and deltas
/// are `uint`s.
///
/// WGSL takes an `i32` or a `u32` lane id; naga 30's validator only a `u32`
/// (`naga-30.0.1/src/valid/function.rs` `validate_subgroup_gather`), so
/// three's spelling is checked as text and naga sees the same calls with
/// `uint` ids.
#[test]
fn subgroup_shuffle_matches_three() {
    let with_uint_ids = instanced_array(64, Type::F32);
    let program = kernel(
        "subgroup_shuffle",
        vec![
            workgroup_barrier(),
            with_uint_ids.element(instance_index()).assign(
                subgroup_broadcast_first(v())
                    .add(subgroup_broadcast(v(), uint(3)).to(Type::F32))
                    .add(subgroup_shuffle(v(), instance_index().bit_xor(uint(1))).to(Type::F32))
                    .add(subgroup_shuffle_xor(v(), 2.0))
                    .add(subgroup_shuffle_up(v(), 1.0))
                    .add(subgroup_shuffle_down(v(), 1.0)),
            ),
        ],
    );
    assert!(
        program
            .wgsl
            .contains("subgroupBroadcast( u32( f32( instanceIndex ) ), 3u )"),
        "{}",
        program.wgsl
    );
    validate(&program.wgsl, "subgroup_shuffle with uint ids");
    let out = instanced_array(64, Type::F32);
    assert_kernel_text(
        "subgroup_shuffle",
        vec![
            workgroup_barrier(),
            out.element(instance_index()).assign(
                subgroup_broadcast_first(v())
                    .add(subgroup_broadcast(v(), 3.0))
                    .add(subgroup_shuffle(
                        v(),
                        instance_index().bit_xor(1.0).to(Type::I32),
                    ))
                    .add(subgroup_shuffle_xor(v(), 2.0))
                    .add(subgroup_shuffle_up(v(), 1.0))
                    .add(subgroup_shuffle_down(v(), 1.0)),
            ),
        ],
        true,
    );
}

/// `quadSwapX` / `quadSwapY` / `quadSwapDiagonal` in a kernel.
#[test]
fn subgroup_quad_matches_three() {
    let out = instanced_array(64, Type::F32);
    assert_kernel(
        "subgroup_quad",
        vec![
            workgroup_barrier(),
            out.element(instance_index()).assign(
                quad_swap_x(v())
                    .add(quad_swap_y(v()))
                    .add(quad_swap_diagonal(v())),
            ),
        ],
        true,
    );
}

/// `subgroupSize`, `subgroupIndex`, `invocationSubgroupIndex` and
/// `invocationLocalIndex`: the three index builtins become leading
/// parameters in the order the flow reads them, `subgroup_size` the last one,
/// and with no barrier the bounds check stays.
#[test]
fn subgroup_builtins_matches_three() {
    let out = instanced_array(64, Type::U32);
    assert_kernel(
        "subgroup_builtins",
        vec![out.element(instance_index()).assign(
            subgroup_size()
                .add(subgroup_index())
                .add(invocation_subgroup_index())
                .add(invocation_local_index()),
        )],
        true,
    );
}

/// `storageElement( buffer, index )` is `buffer.element( index )`.
#[test]
fn storage_element_matches_three() {
    let out = instanced_array(64, Type::F32);
    assert_kernel(
        "storage_element",
        vec![storage_element(&out, instance_index()).assign(v().mul(2.0))],
        false,
    );
}

/// `atomicFunc( method, pointer, value )`, the statement form.
#[test]
fn atomic_func_matches_three() {
    let counters = instanced_array(2, Type::U32).to_atomic();
    assert_kernel(
        "atomic_func",
        vec![
            atomic_func("atomicAdd", counters.element(uint(0)), Some(uint(1))),
            atomic_func(
                "atomicMax",
                counters.element(uint(1)),
                Some(instance_index()),
            ),
        ],
        false,
    );
}

/// The page's `Storage3DTexture( 4, 4, 4 )` store coordinate.
fn volume_coord() -> NodeRef {
    let i = instance_index;
    vec3_join(vec![
        i().modulo(uint(4)),
        i().div(uint(4)).modulo(uint(4)),
        i().div(uint(16)),
    ])
}

/// `textureBarrier()` after a `textureStore`: like every barrier it turns
/// off the bounds check and moves the vars into `main`
/// (`BarrierNode.setup()`), so there is no count uniform.
#[test]
fn texture_barrier_matches_three() {
    let volume = three_rs::Data3DTexture::storage(4, 4, 4);
    let storage = storage_texture_3d(&volume);
    let program = assert_kernel(
        "texture_barrier",
        vec![
            texture_store(&storage, volume_coord(), vec4(1.0, 0.0, 0.0, 1.0)),
            texture_barrier(),
        ],
        false,
    );
    assert!(!program.wgsl.contains("return;"), "{}", program.wgsl);
}

/// `textureStore( storageTexture3D( t ), uvec3, value )`.
#[test]
fn storage_texture_3d_matches_three() {
    let volume = three_rs::Data3DTexture::storage(4, 4, 4);
    let storage = storage_texture_3d(&volume);
    assert_kernel(
        "storage_texture_3d",
        vec![texture_store(
            &storage,
            volume_coord(),
            vec4_join(vec![v().div(64.0), float(0.0), float(0.0), float(1.0)]),
        )],
        false,
    );
}

/// `attributeArray( 64, 'vec3' )` written by a kernel: the same storage
/// buffer `instancedArray` declares.
#[test]
fn attribute_array_compute_matches_three() {
    let out = attribute_array(64, Type::Vec3);
    assert_kernel(
        "attribute_array_compute",
        vec![out
            .element(instance_index())
            .assign(vec3_join(vec![v(), float(0.0), float(1.0)]))],
        false,
    );
}

/// `attributeArray( 4, 'vec3' ).toAttribute()` read by a material: three's
/// fragment and vertex `main`, and the vertex buffer stepping per *vertex* —
/// the one thing that separates it from `instancedArray`. Three's pipeline
/// for this probe (`renderPipeline_attribute_array` in the dump's
/// `dump.json`) has `stepMode: 'vertex'` and `arrayStride: 16` on that buffer.
#[test]
fn attribute_array_steps_per_vertex() {
    let node = vec4_join(vec![
        attribute_array(4, Type::Vec3).to_attribute(),
        float(1.0),
    ]);
    let program = program_for(node, false);
    assert_body_of("attribute_array", program.fragment_wgsl.clone());
    let ours = &program.vertex_wgsl;
    let theirs = fixture("attribute_array.vertex");
    let (ours_n, theirs_n) = (normalise(ours), normalise(&theirs));
    assert!(
        theirs_n.contains("varyings.nodeVarying = nodeAttribute0;"),
        "the test's expectation is not in three's dump"
    );
    assert!(
        ours_n.contains("varyings.nodeVarying = nodeAttribute0;"),
        "{ours}"
    );
    // Three declares `position` at location 0 and the attribute at 1; the
    // port numbers attributes in first-use order (`docs/nodes.md` §8).
    assert!(ours_n.contains("nodeAttribute0 : vec3<f32>"), "{ours}");
    let storage_buffer = |program: &NodeProgram| {
        program
            .vertex_buffers()
            .into_iter()
            .find(|b| matches!(b.source, VertexBufferSource::Instance(_)))
            .expect("the storage buffer's vertex buffer")
    };
    let attribute = storage_buffer(&program);
    assert!(!attribute.instanced, "attributeArray steps per vertex");
    assert_eq!(attribute.array_stride, 16);

    // The same buffer through `instancedArray` steps per instance.
    let node = vec4_join(vec![
        instanced_array(4, Type::Vec3).to_attribute(),
        float(1.0),
    ]);
    assert!(storage_buffer(&program_for(node, false)).instanced);
}

/// Subgroup functions in a fragment shader: `enable subgroups;` under
/// `// directives`, between `// global` and `// structs` as three writes it,
/// and the calls on the `uv()` varying.
#[test]
fn subgroup_fragment_matches_three() {
    let node = vec4_join(vec![
        subgroup_add(x()),
        quad_swap_x(y()),
        quad_swap_diagonal(x()),
        float(1.0),
    ]);
    let program = program_for(node, false);
    // The renderer's feature guard reads this (`docs/nodes.md` §84.3).
    assert!(program.subgroups);
    assert!(!program_for(vec4_join(vec![x(), y(), x(), float(1.0)]), false).subgroups);
    let ours = program.fragment_wgsl;
    assert_body_of("subgroup_fragment", ours.clone());
    let header = "// global\ndiagnostic( off, derivative_uniformity );\n\n\n// directives\nenable subgroups;\n\n// structs\n";
    assert!(fixture("subgroup_fragment").contains(header));
    assert!(ours.contains(header), "{ours}");
    validate(&ours, "subgroup_fragment");
}

/// `quadBroadcast( e, id )` has no probe: three declares it with
/// `setParameterLength( 1 )`, so the lane id is dropped with a warning and
/// `generate()` then throws reading the missing input — three cannot build
/// it. The port takes both arguments and builds them as `subgroupBroadcast`
/// does: a JS-number id as an `int` (which naga 30 refuses, see
/// [`subgroup_shuffle_matches_three`]), a `uint` id as the value's type.
#[test]
fn quad_broadcast_validates() {
    let out = instanced_array(64, Type::F32);
    let program = kernel(
        "quad_broadcast",
        vec![
            workgroup_barrier(),
            out.element(instance_index())
                .assign(quad_broadcast(v(), 1.0)),
        ],
    );
    assert!(program.subgroups);
    assert!(
        program
            .wgsl
            .contains("quadBroadcast( f32( instanceIndex ), 1 )"),
        "{}",
        program.wgsl
    );
    let out = instanced_array(64, Type::U32);
    let program = kernel(
        "quad_broadcast",
        vec![
            workgroup_barrier(),
            out.element(instance_index())
                .assign(quad_broadcast(instance_index(), uint(1))),
        ],
    );
    assert!(
        program.wgsl.contains("quadBroadcast( instanceIndex, 1u )"),
        "{}",
        program.wgsl
    );
    validate(&program.wgsl, "quad_broadcast");
}

/// A kernel without subgroup nodes has neither the directive nor the
/// parameter, so it runs on an adapter without the feature.
#[test]
fn plain_kernel_does_not_enable_subgroups() {
    let out = instanced_array(64, Type::F32);
    let program = kernel("plain", vec![out.element(instance_index()).assign(v())]);
    assert!(!program.subgroups);
    assert!(!program.wgsl.contains("subgroup"), "{}", program.wgsl);
}

/// `subgroupSize` outside a kernel: `ComputeBuiltinNode` warns and writes
/// `generateConst( 'uint' )`.
#[test]
fn subgroup_size_outside_compute_is_zero() {
    let ours = fragment(vec4_join(vec![
        subgroup_size().to(Type::F32),
        float(0.0),
        float(0.0),
        float(1.0),
    ]));
    assert!(ours.contains("f32( 0u )"), "{ours}");
    assert!(!ours.contains("enable subgroups;"), "{ours}");
}
