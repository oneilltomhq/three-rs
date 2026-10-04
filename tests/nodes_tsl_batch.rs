//! The thin-wrapper TSL tier (issue #141) against three.js' own WGSL.
//!
//! Every probe here is one material of `tests/fixtures/tsl_batch/probe.html`,
//! a page that sets each TSL function as a `MeshBasicNodeMaterial`'s
//! `fragmentNode`; `tests/fixtures/tsl_batch/<probe>.wgsl` is three's dumped
//! fragment shader for it (`tools/dump.mjs` over that page). Each test builds
//! the same node graph through the port and compares the `main` body.
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
use three_rs::nodes::tsl::*;
use three_rs::nodes::{NodeBuilder, NodeRef};

fn fragment(node: NodeRef) -> String {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(node);
    let flow = setup(&material, &SetupContext::default(), None);
    NodeBuilder::new().build(&flow).fragment_wgsl
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
/// Three's `Node.build()` now caches *any* cacheable node used more than once
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
    let ours = fragment(node);
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
fn transform_normal() {
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
    let text = body(wgsl).replace("let nodeConst", "nodeConst");
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
                out.push_str(&format!("v{k}"));
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

/// Asserts the port's `main` equals three's up to [`canonical`].
fn assert_canonical(name: &str, node: NodeRef) {
    let ours = fragment(node);
    assert_eq!(
        canonical(&ours),
        canonical(&fixture(name)),
        "{name}: main differs\n--- port ---\n{ours}"
    );
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
