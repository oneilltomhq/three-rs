//! Every export of `MaterialXNodes.js`, against three.js r186's own WGSL.
//!
//! `tests/fixtures/materialx_library/page.html` calls each export once, inside
//! a `Fn()` with a layout named `t_<case>`, so that three.js emits every call
//! — inlined helpers included — as a `fn` of its own. The page's fragment
//! program, dumped, is `fragment-r186.wgsl`. This test builds the same
//! wrappers through the port and compares every `mx_*` and `t_*` `fn` line for
//! line (trailing whitespace normalised, as in `nodes_mx_noise.rs`).
//!
//! Regenerating the fixture: three's committed `build/` is newer than r186 and
//! caches shared temps as `let nodeConstN`, which the port does not follow, so
//! build the r186 tag into a scratch copy first:
//!
//! ```text
//! git -C $THREE archive r186 src utils package.json | tar -x -C $S/three-r186
//! ln -s $THREE/node_modules $S/three-r186/node_modules
//! (cd $S/three-r186 && npx rollup -c utils/build/rollup.config.js)
//! # $S/vendor-r186: a symlink per entry of $THREE, except build -> $S/three-r186/build
//! THREE_JS_DIR=$S/vendor-r186 node tools/dump-webgpu.mjs materialx_library \
//!     --page tests/fixtures/materialx_library/page.html --out target/dumps/materialx_library
//! cp target/dumps/materialx_library/fragment.wgsl tests/fixtures/materialx_library/fragment-r186.wgsl
//! ```

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use three_rs::nodes::materialx as mx;
use three_rs::nodes::node::{FnDef, Type};
use three_rs::nodes::tsl::{call, float, position_local, shader_fn, to_varying, uv, vec4};
use three_rs::nodes::{MaterialFlow, NodeBuilder, NodeRef};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/materialx_library/fragment-r186.wgsl"
);

/// Every `fn <name> ( … ) { … }` block in a WGSL source, keyed by name.
fn functions(src: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(rest) = lines[i].strip_prefix("fn ") {
            let name = rest.split([' ', '(']).next().unwrap_or("").to_string();
            let end = (i + 1..lines.len()).find(|&j| lines[j] == "}").unwrap_or(i);
            let body = lines[i..=end]
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

/// What each wrapper's arguments are built from in `page.html`: `p2` is
/// `uv()`, `p3` is `positionLocal` and `f` is `positionLocal.x`.
#[derive(Clone, Copy)]
enum Arg {
    P2,
    P3,
    F,
}

struct Case {
    name: &'static str,
    def: Rc<FnDef>,
    args: &'static [Arg],
}

fn ty(arg: Arg) -> Type {
    match arg {
        Arg::P2 => Type::Vec2,
        Arg::P3 => Type::Vec3,
        Arg::F => Type::F32,
    }
}

/// `t( name, type, inputs, body )` from `page.html`.
fn case(
    name: &'static str,
    ret: Type,
    args: &'static [Arg],
    body: impl Fn(&[NodeRef]) -> NodeRef + 'static,
) -> Case {
    let wgsl_name: &'static str = Box::leak(format!("t_{name}").into_boxed_str());
    const NAMES: [&str; 5] = ["a0", "a1", "a2", "a3", "a4"];
    let params = args
        .iter()
        .enumerate()
        .map(|(i, a)| (NAMES[i], ty(*a)))
        .collect();
    Case {
        name: wgsl_name,
        def: shader_fn(Some(wgsl_name), params, ret, body),
        args,
    }
}

use Arg::{F, P2, P3};

fn cases() -> Vec<Case> {
    let uv = uv;
    vec![
        // noise
        case("noise_float_2", Type::F32, &[P2], |a| {
            mx::mx_noise_float(a[0].clone(), 1.0, 0.0)
        }),
        case("noise_float_3", Type::F32, &[P3], |a| {
            mx::mx_noise_float(a[0].clone(), 1.0, 0.0)
        }),
        case("noise_vec3_2", Type::Vec3, &[P2], |a| {
            mx::mx_noise_vec3(a[0].clone(), 1.0, 0.0)
        }),
        case("noise_vec3_3", Type::Vec3, &[P3], |a| {
            mx::mx_noise_vec3(a[0].clone(), 2.0, 0.5)
        }),
        case("noise_vec4_2", Type::Vec4, &[P2], |a| {
            mx::mx_noise_vec4(a[0].clone(), 1.0, 0.0)
        }),
        case("noise_vec4_3", Type::Vec4, &[P3], |a| {
            mx::mx_noise_vec4(a[0].clone(), 1.0, 0.0)
        }),
        case("cell_noise_float_2", Type::F32, &[P2], |a| {
            mx::mx_cell_noise_float(a[0].clone())
        }),
        case("cell_noise_float_3", Type::F32, &[P3], |a| {
            mx::mx_cell_noise_float(a[0].clone())
        }),
        case("cell_noise_vec3_2", Type::Vec3, &[P2], |a| {
            mx::mx_cell_noise_vec3(a[0].clone())
        }),
        case("cell_noise_vec3_3", Type::Vec3, &[P3], |a| {
            mx::mx_cell_noise_vec3(a[0].clone())
        }),
        // fractal
        case("fractal_noise_float_2d", Type::F32, &[P2], |a| {
            mx::mx_fractal_noise_float_2d(a[0].clone(), 3, 2.0, 0.5, 1.0)
        }),
        case("fractal_noise_float", Type::F32, &[P3], |a| {
            mx::mx_fractal_noise_float(a[0].clone(), 3, 2.0, 0.5, 1.0)
        }),
        case("fractal_noise_vec2", Type::Vec2, &[P3], |a| {
            mx::mx_fractal_noise_vec2(a[0].clone(), 3, 2.0, 0.5, 1.0)
        }),
        case("fractal_noise_vec3", Type::Vec3, &[P3], |a| {
            mx::mx_fractal_noise_vec3(a[0].clone(), 3, 2.0, 0.5, 1.0)
        }),
        case("fractal_noise_vec4", Type::Vec4, &[P3], |a| {
            mx::mx_fractal_noise_vec4(a[0].clone(), 4, 2.5, 0.25, 2.0)
        }),
    ]
    .into_iter()
    .chain(more_cases(uv))
    .collect()
}

fn more_cases(_uv: fn() -> NodeRef) -> Vec<Case> {
    vec![
        // worley
        case("worley_noise_float_2", Type::F32, &[P2], |a| {
            mx::mx_worley_noise_float(a[0].clone(), 1.0, 0)
        }),
        case("worley_noise_float_3", Type::F32, &[P3], |a| {
            mx::mx_worley_noise_float(a[0].clone(), 0.5, 1)
        }),
        case("worley_noise_float_2d", Type::F32, &[P2], |a| {
            mx::mx_worley_noise_float_2d(a[0].clone(), 1.0, 0)
        }),
        case("worley_noise_float_3d", Type::F32, &[P3], |a| {
            mx::mx_worley_noise_float_3d(a[0].clone(), 1.0, 0)
        }),
        case("worley_noise_vec2_2", Type::Vec2, &[P2], |a| {
            mx::mx_worley_noise_vec2(a[0].clone(), 1.0)
        }),
        case("worley_noise_vec2_3", Type::Vec2, &[P3], |a| {
            mx::mx_worley_noise_vec2(a[0].clone(), 1.0)
        }),
        case("worley_noise_vec3_2", Type::Vec3, &[P2], |a| {
            mx::mx_worley_noise_vec3(a[0].clone(), 1.0, 1)
        }),
        case("worley_noise_vec3_3", Type::Vec3, &[P3], |a| {
            mx::mx_worley_noise_vec3(a[0].clone(), 1.0, 2)
        }),
        case("worley_noise_vec3_style_2", Type::Vec3, &[P2], |a| {
            mx::mx_worley_noise_vec3_style(a[0].clone(), 1.0, 0, 0)
        }),
        case("worley_noise_vec3_style_3", Type::Vec3, &[P3], |a| {
            mx::mx_worley_noise_vec3_style(a[0].clone(), 1.0, 1, 0)
        }),
    ]
}

/// The port's fragment program for every case, summed into the colour the
/// way `page.html` does.
fn generated(cases: &[Case]) -> String {
    let p3 = to_varying(Some("positionLocal"), position_local());
    let f = p3.x();
    let mut sum = float(0.0);
    for c in cases {
        let args = c
            .args
            .iter()
            .map(|a| match a {
                Arg::P2 => uv(),
                Arg::P3 => p3.clone(),
                Arg::F => f.clone(),
            })
            .collect();
        let value = call(&c.def, args);
        sum = sum.add(if value.ty() == Type::F32 {
            value
        } else {
            value.x()
        });
    }
    let flow = MaterialFlow {
        depth: None,
        pre_vertex_statements: vec![],
        fragment_statements: vec![],
        emit_output_property: true,
        output: vec4(0.0, 0.0, 0.0, 1.0).add(sum.mul(1e-3)),
        output_assign: None,
        output_node: None,
        mrt: None,
        vertex_statements: vec![],
        position: vec4(0.0, 0.0, 0.0, 1.0),
    };
    NodeBuilder::new().build(&flow).fragment_wgsl
}

#[test]
fn every_materialx_export_matches_r186() {
    let cases = cases();
    let wgsl = generated(&cases);
    // `uv()` is a varying the port numbers from its own counter; three's page
    // happens to make it `nodeVarying3`.
    let uv_name = wgsl.lines().find_map(|l| {
        let l = l.trim_start_matches(|c: char| c == '\t' || c == ' ');
        let rest = l.split("@location( ").nth(1)?;
        let name = rest.split(") ").nth(1)?.split(" :").next()?;
        (name.starts_with("nodeVarying") && l.contains("vec2<f32>")).then(|| name.to_string())
    });
    let wgsl = match uv_name {
        Some(name) => wgsl.replace(&name, "nodeVarying3"),
        None => wgsl,
    };
    let got = functions(&wgsl);
    let want = functions(&std::fs::read_to_string(FIXTURE).expect("fixture"));

    let mut names: Vec<&str> = cases.iter().map(|c| c.name).collect();
    // Every `mx_*` the port emitted, and every one three emitted for the cases
    // this test builds.
    let mut mx: BTreeMap<&str, ()> = BTreeMap::new();
    for name in got.keys().filter(|n| n.starts_with("mx_")) {
        mx.insert(name, ());
    }
    names.extend(mx.keys());

    let mut failures = Vec::new();
    for name in names {
        let Some(g) = got.get(name) else {
            failures.push(format!("three-rs emitted no `fn {name}`"));
            continue;
        };
        let Some(w) = want.get(name) else {
            failures.push(format!("three.js emitted no `fn {name}`"));
            continue;
        };
        if g != w {
            let (gl, wl): (Vec<&str>, Vec<&str>) = (g.lines().collect(), w.lines().collect());
            let at = (0..gl.len().max(wl.len()))
                .find(|&i| gl.get(i) != wl.get(i))
                .unwrap_or(0);
            failures.push(format!(
                "`fn {name}` differs at line {at}:\n  port:  {}\n  three: {}",
                gl.get(at).unwrap_or(&"<end>"),
                wl.get(at).unwrap_or(&"<end>")
            ));
        }
    }
    if std::env::var_os("MX_DUMP").is_some() {
        std::fs::write("target/mx_library_port.wgsl", &wgsl).unwrap();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
