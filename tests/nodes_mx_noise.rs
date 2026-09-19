//! The MaterialX noise chain, against three.js r186's own dumped WGSL.
//!
//! `tests/fixtures/rung7/m09_ShadowMaterial.frag-r186.wgsl` carries the twelve
//! `fn`s of the `mx_fractal_noise_float` chain and
//! `m11_phong_ground.frag-r186.wgsl` the five extra `fn`s of the `vec3` chain.
//! Both are Three's own dumps of `webgpu_shadowmap`'s shadow and ground
//! programs, kept whole so the surrounding program stays readable.
//! Each emitted `fn` is compared to the dump's, line by line, with trailing
//! whitespace normalised (the port tabs its blank lines; three.js does not).

use std::collections::HashMap;

use three_rs::nodes::materialx::{mx_fractal_noise_float, mx_fractal_noise_vec3};
use three_rs::nodes::tsl::{float, position_local, to_varying, vec4};
use three_rs::nodes::{MaterialFlow, NodeBuilder, NodeRef};

const DUMPS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/rung7");

/// Every `fn <name> ( … ) { … }` block in a WGSL source, keyed by name.
fn functions(src: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(rest) = line.strip_prefix("fn ") {
            let name = rest.split([' ', '(']).next().unwrap_or("").to_string();
            // The signature through the matching closing brace at column 0.
            let start = i;
            let mut end = i;
            for (j, l) in lines.iter().enumerate().skip(i + 1) {
                if *l == "}" {
                    end = j;
                    break;
                }
            }
            let body = lines[start..=end]
                .iter()
                .map(|l| l.trim_end())
                .collect::<Vec<_>>()
                .join("\n");
            out.insert(name, body);
            i = end + 1;
            continue;
        }
        i += 1;
    }
    out
}

fn fragment_wgsl(noise: NodeRef) -> String {
    // `positionLocal` reaches the fragment stage as a varying, exactly as the
    // dumped `main( @location( 0 ) positionLocal : vec3<f32> )` shows.
    let flow = MaterialFlow {
        pre_vertex_statements: vec![],
        fragment_statements: vec![three_rs::nodes::tsl::discard_if(
            noise.greater_than(float(0.0)),
        )],
        emit_output_property: true,
        output: vec4(0.0, 0.0, 0.0, 1.0),
        output_assign: None,
        output_node: None,
        mrt: None,
        vertex_statements: vec![],
        position: vec4(0.0, 0.0, 0.0, 1.0),
    };
    NodeBuilder::new().build(&flow).fragment_wgsl
}

fn position() -> NodeRef {
    to_varying(Some("positionLocal"), position_local())
}

fn dump(name: &str) -> HashMap<String, String> {
    let path = format!("{DUMPS}/{name}");
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    functions(&src)
}

fn compare(
    generated: &HashMap<String, String>,
    expected: &HashMap<String, String>,
    names: &[&str],
) {
    for name in names {
        let got = generated
            .get(*name)
            .unwrap_or_else(|| panic!("three-rs emitted no `fn {name}`"));
        let want = expected
            .get(*name)
            .unwrap_or_else(|| panic!("the dump has no `fn {name}`"));
        assert_eq!(got, want, "`fn {name}` differs from three.js r186");
    }
}

/// The twelve `fn`s of the `mx_fractal_noise_float` chain, in the order the dump
/// declares them.
const FLOAT_CHAIN: [&str; 12] = [
    "mx_floor",
    "mx_fade",
    "mx_rotl32",
    "mx_bjfinal",
    "mx_hash_int_2",
    "mx_select",
    "mx_negate_if",
    "mx_gradient_float_1",
    "mx_trilerp_0",
    "mx_gradient_scale3d_0",
    "mx_perlin_noise_float_1",
    "mx_fractal_noise_float",
];

/// The `vec3` chain's own `fn`s; it shares the other seven with the `float` one.
const VEC3_CHAIN: [&str; 5] = [
    "mx_hash_vec3_1",
    "mx_gradient_vec3_1",
    "mx_trilerp_1",
    "mx_gradient_scale3d_1",
    "mx_perlin_noise_vec3_1",
];

#[test]
fn fractal_noise_float_matches_r186() {
    let wgsl = fragment_wgsl(mx_fractal_noise_float(
        position().mul(float(0.1)),
        3,
        2.0,
        0.5,
        1.0,
    ));
    compare(
        &functions(&wgsl),
        &dump("m09_ShadowMaterial.frag-r186.wgsl"),
        &FLOAT_CHAIN,
    );
}

#[test]
fn fractal_noise_vec3_matches_r186() {
    let noise = mx_fractal_noise_vec3(position().mul(float(2.0)), 3, 2.0, 0.5, 1.0);
    let wgsl = fragment_wgsl(noise.x());
    let generated = functions(&wgsl);
    let expected = dump("m11_phong_ground.frag-r186.wgsl");
    compare(&generated, &expected, &VEC3_CHAIN);
    // The shared half of the chain is emitted identically in both dumps.
    compare(
        &generated,
        &expected,
        &[
            "mx_floor",
            "mx_fade",
            "mx_rotl32",
            "mx_bjfinal",
            "mx_hash_int_2",
            "mx_select",
            "mx_negate_if",
            "mx_gradient_float_1",
        ],
    );
}

#[test]
fn fractal_noise_float_call_site_matches_r186() {
    let wgsl = fragment_wgsl(mx_fractal_noise_float(
        position().mul(float(0.1)),
        3,
        2.0,
        0.5,
        1.0,
    ));
    assert!(
        wgsl.contains(
            "( ! ( ( mx_fractal_noise_float( ( positionLocal * vec3<f32>( 0.1 ) ), 3, 2.0, 0.5 ) \
             * 1.0 ) > 0.0 ) )"
        ),
        "call site differs: {wgsl}"
    );
}

#[test]
fn fractal_noise_vec3_call_site_matches_r186() {
    let noise = mx_fractal_noise_vec3(position().mul(float(2.0)), 3, 2.0, 0.5, 1.0);
    let wgsl = fragment_wgsl(noise.x());
    assert!(
        wgsl.contains(
            "( mx_fractal_noise_vec3( ( positionLocal * vec3<f32>( 2.0 ) ), 3, 2.0, 0.5 ) \
             * vec3<f32>( 1.0 ) )"
        ),
        "call site differs: {wgsl}"
    );
}
