//! Oracle for the shape stack — `Path` / `Shape` / `ShapePath`, `Earcut`,
//! `ShapeGeometry`, `ExtrudeGeometry` and `TubeGeometry` — against three.js
//! itself.
//!
//! `tools/geometry_reference.mjs` builds each scenario with the three.js
//! checkout's own source under node and prints the result; each test here
//! builds the same scenario with the port and compares. Geometry attributes
//! must agree to 1e-6 and the index, groups and Earcut's triangles exactly
//! (triangle order included — it decides the vertex order of every lid).
//!
//! Skipped, with a note, when there is no three.js checkout or no node; the
//! same condition the rung harness skips on.

use std::path::Path as FsPath;
use std::process::Command;
use std::rc::Rc;
use std::sync::OnceLock;

use serde_json::Value;
use three_rs::core::{BufferGeometry, Index};
use three_rs::extras::{earcut, CatmullRomCurve3, Curve, FillRule, Path, Shape, ShapePath};
use three_rs::geometries::{
    extrude_geometry, extrude_geometry_default_shape, shape_geometry, shape_geometry_default_shape,
    shape_geometry_multi, tube_geometry, tube_geometry_default_path, ExtrudeGeometryOptions,
};
use three_rs::math::{Vector2, Vector3};

const TOLERANCE: f64 = 1e-6;

/// Runs the reference script once for every test, or `None` to skip.
fn reference() -> Option<&'static Value> {
    static REFERENCE: OnceLock<Option<Value>> = OnceLock::new();
    REFERENCE
        .get_or_init(|| {
            let three = three_rs::testing::three_js_dir();
            if !three.join("src/extras/Earcut.js").exists() {
                eprintln!(
                    "skipping: no three.js checkout at {} (set THREE_JS_DIR)",
                    three.display()
                );
                return None;
            }

            let script =
                FsPath::new(env!("CARGO_MANIFEST_DIR")).join("tools/geometry_reference.mjs");
            let output = match Command::new("node").arg(&script).arg(&three).output() {
                Ok(output) => output,
                Err(error) => {
                    eprintln!("skipping: cannot run node ({error})");
                    return None;
                }
            };
            assert!(
                output.status.success(),
                "the reference script failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            Some(
                serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim())
                    .expect("the reference script's JSON"),
            )
        })
        .as_ref()
}

fn numbers(value: &Value, what: &str) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{what}: not an array"))
        .iter()
        .map(|v| v.as_f64().unwrap_or_else(|| panic!("{what}: not a number")))
        .collect()
}

#[track_caller]
fn assert_close(actual: &[f64], expected: &[f64], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: length");
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - e).abs() <= TOLERANCE,
            "{what}[{i}]: {a} != {e} (off by {})",
            (a - e).abs()
        );
    }
}

fn flat2(points: &[Vector2]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.x, p.y]).collect()
}

fn flat3(points: &[Vector3]) -> Vec<f64> {
    points.iter().flat_map(|p| [p.x, p.y, p.z]).collect()
}

fn flat_nested(value: &Value, what: &str) -> Vec<f64> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{what}: not an array"))
        .iter()
        .flat_map(|p| numbers(p, what))
        .collect()
}

#[track_caller]
fn assert_geometry(geometry: &BufferGeometry, expected: &Value, what: &str) {
    for name in ["position", "normal", "uv"] {
        let attribute = geometry.get_attribute(name);
        match (&expected[name], attribute) {
            (Value::Null, None) => {}
            (Value::Null, Some(_)) => panic!("{what}: port has `{name}`, three.js does not"),
            (_, None) => panic!("{what}: three.js has `{name}`, the port does not"),
            (values, Some(attribute)) => {
                let actual: Vec<f64> = attribute.array().iter().map(|&v| v as f64).collect();
                assert_close(&actual, &numbers(values, what), &format!("{what}.{name}"));
            }
        }
    }

    let index: Option<Vec<u64>> = geometry.index.as_ref().map(|index| match index {
        Index::U16(v) => v.iter().map(|&i| i as u64).collect(),
        Index::U32(v) => v.iter().map(|&i| i as u64).collect(),
    });
    let expected_index: Option<Vec<u64>> = expected["index"]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_u64().unwrap()).collect());
    assert_eq!(index, expected_index, "{what}.index");

    let groups: Vec<[u64; 3]> = geometry
        .groups
        .iter()
        .map(|g| [g.start as u64, g.count as u64, g.material_index as u64])
        .collect();
    let expected_groups: Vec<[u64; 3]> = expected["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            let g = g.as_array().unwrap();
            [
                g[0].as_u64().unwrap(),
                g[1].as_u64().unwrap(),
                g[2].as_u64().unwrap(),
            ]
        })
        .collect();
    assert_eq!(groups, expected_groups, "{what}.groups");
}

/// webgl_geometry_shapes' heart, in canvas units.
fn heart_shape() -> Shape {
    let (x, y) = (0.0, 0.0);
    let mut shape = Shape::new();
    shape
        .move_to(x + 25.0, y + 25.0)
        .bezier_curve_to(x + 25.0, y + 25.0, x + 20.0, y, x, y)
        .bezier_curve_to(x - 30.0, y, x - 30.0, y + 35.0, x - 30.0, y + 35.0)
        .bezier_curve_to(x - 30.0, y + 55.0, x - 10.0, y + 77.0, x + 25.0, y + 95.0)
        .bezier_curve_to(x + 60.0, y + 77.0, x + 80.0, y + 55.0, x + 80.0, y + 35.0)
        .bezier_curve_to(x + 80.0, y + 35.0, x + 80.0, y, x + 50.0, y)
        .bezier_curve_to(x + 35.0, y, x + 25.0, y + 25.0, x + 25.0, y + 25.0);
    shape
}

/// webgpu_instance_path's heart.
fn instance_path_heart() -> Path {
    let (x, y) = (0.0, 0.0);
    let mut path = Path::new();
    path.move_to(x - 2.5, y - 2.5)
        .bezier_curve_to(x - 2.5, y - 2.5, x - 2.0, y, x, y)
        .bezier_curve_to(x + 3.0, y, x + 3.0, y - 3.5, x + 3.0, y - 3.5)
        .bezier_curve_to(x + 3.0, y - 5.5, x + 1.0, y - 7.7, x - 2.5, y - 9.5)
        .bezier_curve_to(x - 6.0, y - 7.7, x - 8.0, y - 5.5, x - 8.0, y - 3.5)
        .bezier_curve_to(x - 8.0, y - 3.5, x - 8.0, y, x - 5.0, y)
        .bezier_curve_to(x - 3.5, y, x - 2.5, y - 2.5, x - 2.5, y - 2.5);
    path
}

fn holed_shape() -> Shape {
    let mut shape = Shape::new();
    shape
        .move_to(-4.0, -4.0)
        .line_to(4.0, -4.0)
        .quadratic_curve_to(5.0, 0.0, 4.0, 4.0)
        .spline_thru(&[Vector2::new(0.0, 5.0), Vector2::new(-4.0, 4.0)])
        .line_to(-4.0, -4.0);

    let mut round = Path::new();
    round.absarc(-1.5, 0.0, 1.2, 0.0, std::f64::consts::PI * 2.0, true);
    let mut triangle = Path::new();
    triangle
        .move_to(1.0, -2.0)
        .line_to(3.0, -2.0)
        .line_to(2.0, 1.0)
        .line_to(1.0, -2.0);
    let mut ellipse = Path::new();
    ellipse.absellipse(
        1.0,
        2.5,
        1.0,
        0.5,
        0.0,
        std::f64::consts::PI * 2.0,
        false,
        0.3,
    );

    shape.holes.extend([round, triangle, ellipse]);
    shape
}

fn knot_curve() -> CatmullRomCurve3 {
    CatmullRomCurve3::new(vec![
        Vector3::new(-10.0, 0.0, 10.0),
        Vector3::new(-5.0, 5.0, 5.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(5.0, -5.0, 5.0),
        Vector3::new(10.0, 0.0, 10.0),
    ])
}

#[test]
fn path_points_and_length() {
    let Some(reference) = reference() else { return };

    let heart = instance_path_heart();
    let points: Vec<Vector2> = (0..1000)
        .step_by(37)
        .map(|i| heart.get_point_at(i as f64 / 1000.0))
        .collect();
    assert_close(
        &flat2(&points),
        &flat_nested(&reference["instance_path_points"], "instance_path_points"),
        "instance_path_points",
    );
    assert_close(
        &[heart.get_length()],
        &[reference["instance_path_length"].as_f64().unwrap()],
        "instance_path_length",
    );

    let holed = holed_shape();
    let extracted = holed.extract_points(12);
    let expected = &reference["holed_extract"];
    assert_close(
        &flat2(&extracted.shape),
        &flat_nested(&expected["shape"], "holed shape"),
        "holed_extract.shape",
    );
    let holes = expected["holes"].as_array().unwrap();
    assert_eq!(extracted.holes.len(), holes.len(), "hole count");
    for (i, (hole, expected)) in extracted.holes.iter().zip(holes).enumerate() {
        assert_close(
            &flat2(hole),
            &flat_nested(expected, "hole"),
            &format!("holed_extract.holes[{i}]"),
        );
    }

    assert_close(
        &flat2(&holed.get_spaced_points(40)),
        &flat_nested(&reference["holed_spaced"], "holed_spaced"),
        "holed_spaced",
    );
}

#[test]
fn frenet_frames() {
    let Some(reference) = reference() else { return };

    let frames = knot_curve().compute_frenet_frames(16, false);
    let expected = &reference["knot_frames"];
    assert_close(
        &flat3(&frames.tangents),
        &flat_nested(&expected["tangents"], "tangents"),
        "tangents",
    );
    assert_close(
        &flat3(&frames.normals),
        &flat_nested(&expected["normals"], "normals"),
        "normals",
    );
    assert_close(
        &flat3(&frames.binormals),
        &flat_nested(&expected["binormals"], "binormals"),
        "binormals",
    );
}

#[test]
fn shape_path_to_shapes() {
    let Some(reference) = reference() else { return };

    let mut sp = ShapePath::new();
    sp.move_to(0.0, 0.0)
        .line_to(10.0, 0.0)
        .line_to(10.0, 10.0)
        .line_to(0.0, 10.0)
        .line_to(0.0, 0.0);
    sp.move_to(2.0, 2.0)
        .line_to(2.0, 8.0)
        .line_to(8.0, 8.0)
        .line_to(8.0, 2.0)
        .line_to(2.0, 2.0);
    sp.move_to(20.0, 0.0)
        .quadratic_curve_to(25.0, 10.0, 30.0, 0.0)
        .line_to(20.0, 0.0);

    for (rule, name) in [
        (FillRule::NonZero, "nonzero"),
        (FillRule::EvenOdd, "evenodd"),
    ] {
        sp.fill_rule = rule;
        let shapes = sp.to_shapes();
        let expected = reference["shape_path"][name].as_array().unwrap();
        assert_eq!(shapes.len(), expected.len(), "{name}: shape count");
        for (i, (shape, expected)) in shapes.iter().zip(expected).enumerate() {
            let points = shape.extract_points(4);
            assert_close(
                &flat2(&points.shape),
                &flat_nested(&expected["shape"], "shape"),
                &format!("{name}[{i}].shape"),
            );
            let holes = expected["holes"].as_array().unwrap();
            assert_eq!(points.holes.len(), holes.len(), "{name}[{i}]: hole count");
            for (h, (hole, expected)) in points.holes.iter().zip(holes).enumerate() {
                assert_close(
                    &flat2(hole),
                    &flat_nested(expected, "hole"),
                    &format!("{name}[{i}].holes[{h}]"),
                );
            }
        }
    }
}

#[test]
fn earcut_triangle_order() {
    let Some(reference) = reference() else { return };

    for name in ["star", "gear"] {
        let case = &reference["earcut"][name];
        let data = numbers(&case["data"], name);
        let holes: Vec<usize> = numbers(&case["holes"], name)
            .into_iter()
            .map(|h| h as usize)
            .collect();
        let expected: Vec<usize> = numbers(&case["triangles"], name)
            .into_iter()
            .map(|i| i as usize)
            .collect();
        assert_eq!(
            earcut::triangulate(&data, &holes, 2),
            expected,
            "{name}: triangles"
        );
    }
}

#[test]
fn shape_geometries() {
    let Some(reference) = reference() else { return };

    assert_geometry(
        &shape_geometry(&shape_geometry_default_shape(), 12),
        &reference["shape_default"],
        "shape_default",
    );
    assert_geometry(
        &shape_geometry(&heart_shape(), 30),
        &reference["shape_heart_30"],
        "shape_heart_30",
    );
    assert_geometry(
        &shape_geometry_multi(&[heart_shape(), holed_shape()], 6),
        &reference["shape_multi"],
        "shape_multi",
    );
}

#[test]
fn extrude_geometries() {
    let Some(reference) = reference() else { return };

    assert_geometry(
        &extrude_geometry(
            &[extrude_geometry_default_shape()],
            &ExtrudeGeometryOptions::default(),
        ),
        &reference["extrude_default"],
        "extrude_default",
    );

    // The heart ExtrudeGeometry the issue names.
    assert_geometry(
        &extrude_geometry(
            &[heart_shape()],
            &ExtrudeGeometryOptions {
                depth: 8.0,
                bevel_enabled: true,
                bevel_segments: 2,
                steps: 2,
                bevel_size: Some(1.0),
                bevel_thickness: 1.0,
                ..Default::default()
            },
        ),
        &reference["extrude_heart"],
        "extrude_heart",
    );

    assert_geometry(
        &extrude_geometry(
            &[holed_shape()],
            &ExtrudeGeometryOptions {
                curve_segments: 8,
                depth: 2.0,
                steps: 3,
                bevel_offset: 0.05,
                ..Default::default()
            },
        ),
        &reference["extrude_holed"],
        "extrude_holed",
    );

    assert_geometry(
        &extrude_geometry(
            &[heart_shape(), holed_shape()],
            &ExtrudeGeometryOptions {
                curve_segments: 5,
                bevel_enabled: false,
                ..Default::default()
            },
        ),
        &reference["extrude_flat"],
        "extrude_flat",
    );

    assert_geometry(
        &extrude_geometry(
            &[holed_shape()],
            &ExtrudeGeometryOptions {
                curve_segments: 4,
                steps: 20,
                extrude_path: Some(Rc::new(knot_curve())),
                ..Default::default()
            },
        ),
        &reference["extrude_path"],
        "extrude_path",
    );

    let mut closed = knot_curve();
    closed.closed = true;
    assert_geometry(
        &extrude_geometry(
            &[Shape::from_points(&[
                Vector2::new(0.0, 1.0),
                Vector2::new(-1.0, -1.0),
                Vector2::new(1.0, -1.0),
            ])],
            &ExtrudeGeometryOptions {
                steps: 30,
                extrude_path: Some(Rc::new(closed)),
                ..Default::default()
            },
        ),
        &reference["extrude_path_closed"],
        "extrude_path_closed",
    );
}

#[test]
fn tube_geometries() {
    let Some(reference) = reference() else { return };

    let (geometry, _) = tube_geometry(&tube_geometry_default_path(), 64, 1.0, 8, false);
    assert_geometry(&geometry, &reference["tube_default"], "tube_default");

    let (geometry, _) = tube_geometry(&knot_curve(), 40, 0.5, 6, false);
    assert_geometry(&geometry, &reference["tube_knot"], "tube_knot");

    let mut closed = knot_curve();
    closed.closed = true;
    let (geometry, _) = tube_geometry(&closed, 40, 0.5, 6, true);
    assert_geometry(
        &geometry,
        &reference["tube_knot_closed"],
        "tube_knot_closed",
    );
}

/// `webgpu_modifier_curve`'s text: `FontLoader` + `TextGeometry` with the
/// page's parameters, then `geometry.rotateX( Math.PI )`; and the defaults
/// `TextGeometry` fills in, with a newline and a glyph the font lacks.
#[test]
fn text_geometries() {
    use three_rs::addons::text_geometry::{text_geometry, TextGeometryOptions};
    use three_rs::loaders::FontLoader;

    let Some(reference) = reference() else { return };

    let font = FontLoader::new()
        .load(
            three_rs::testing::three_js_dir()
                .join("examples/fonts/helvetiker_regular.typeface.json"),
        )
        .unwrap();

    let mut parameters = TextGeometryOptions::new();
    parameters.size = 0.2;
    parameters.extrude.depth = 0.05;
    parameters.extrude.curve_segments = 12;
    parameters.extrude.bevel_enabled = true;
    parameters.extrude.bevel_thickness = 0.02;
    parameters.extrude.bevel_size = Some(0.01);
    parameters.extrude.bevel_offset = 0.0;
    parameters.extrude.bevel_segments = 5;
    let mut geometry = text_geometry("Hello three.js!", &font, &parameters);
    geometry.rotate_x(std::f64::consts::PI);
    assert_geometry(
        &geometry,
        &reference["text_modifier_curve"],
        "text_modifier_curve",
    );

    let mut parameters = TextGeometryOptions::new();
    parameters.extrude.curve_segments = 3;
    let geometry = text_geometry("a\nb\u{2603}", &font, &parameters);
    assert_geometry(&geometry, &reference["text_defaults"], "text_defaults");
}

/// `webgpu_modifier_curve`'s `Flow`: the half-float spline texture
/// `updateCurve` writes from the page's closed centripetal curve, bit for
/// bit, and the `spineLength` it sets.
#[test]
fn flow_spline_texture() {
    use three_rs::addons::curve_modifier_gpu::Flow;
    use three_rs::materials::MeshBasicNodeMaterial;

    let Some(reference) = reference() else { return };
    let expected = &reference["flow_modifier_curve"];

    let mut curve = CatmullRomCurve3::new(vec![
        Vector3::new(1.0, 0.0, -1.0),
        Vector3::new(1.0, 0.0, 1.0),
        Vector3::new(-1.0, 0.0, 1.0),
        Vector3::new(-1.0, 0.0, -1.0),
    ]);
    curve.closed = true;

    assert_close(
        &flat3(&curve.get_points(50)),
        &numbers(&expected["points"], "points"),
        "flow points",
    );

    let mut flow = Flow::new(
        Rc::new(BufferGeometry::new()),
        &MeshBasicNodeMaterial::new(),
        1,
    );
    flow.update_curve(0, &curve);

    assert_close(
        &flow.uniforms.spine_length.get(),
        &[expected["spineLength"].as_f64().unwrap()],
        "spineLength",
    );
    let data: Vec<u16> = numbers(&expected["data"], "data")
        .into_iter()
        .map(|v| v as u16)
        .collect();
    assert_eq!(data.len(), flow.spline_data().len());
    let mismatches: Vec<usize> = (0..data.len())
        .filter(|&i| data[i] != flow.spline_data()[i])
        .collect();
    assert!(
        mismatches.is_empty(),
        "{} of {} halves differ, first at {:?}",
        mismatches.len(),
        data.len(),
        mismatches.first()
    );
}
