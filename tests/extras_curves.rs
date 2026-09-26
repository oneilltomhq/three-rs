//! Ports of `three.js/test/unit/src/extras/curves/`: `ArcCurve`,
//! `CubicBezierCurve`, `CubicBezierCurve3`, `EllipseCurve`, `LineCurve`,
//! `LineCurve3`, `QuadraticBezierCurve`, `QuadraticBezierCurve3` and
//! `SplineCurve` (`CatmullRomCurve3` has its own file,
//! `tests/extras_catmull_rom_curve3.rs`).
//!
//! Expectations are three.js' own; nothing here was recomputed from the Rust
//! implementation. `assert.deepEqual` is exact equality, `assert.numEqual`
//! is `qunit-utils.js`' `diff < 0.1`.
//!
//! `Extending` (`instanceof Curve`) and the `isXxx` brand tests are JS class
//! identity with nothing to assert in Rust: every type here implements
//! [`Curve`], which the compiler checks. `Instancing` and `type` are ported.

use three_rs::extras::{
    CubicBezierCurve, CubicBezierCurve3, Curve, EllipseCurve, LineCurve, LineCurve3,
    QuadraticBezierCurve, QuadraticBezierCurve3, SplineCurve,
};
use three_rs::math::{Vector2, Vector3};

/// `assert.numEqual`: `Math.abs( actual - expected ) < 0.1`.
#[track_caller]
fn num_equal(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 0.1,
        "{what}: {actual} should be equal to {expected}"
    );
}

#[track_caller]
fn num_equal2(actual: Vector2, expected: Vector2, what: &str) {
    num_equal(actual.x, expected.x, &format!("{what}.x"));
    num_equal(actual.y, expected.y, &format!("{what}.y"));
}

/// Checks x and y only, as the three.js tangent tests of the 3D curves do.
#[track_caller]
fn num_equal3_xy(actual: Vector3, expected: Vector3, what: &str) {
    num_equal(actual.x, expected.x, &format!("{what}.x"));
    num_equal(actual.y, expected.y, &format!("{what}.y"));
}

#[track_caller]
fn num_equal3(actual: Vector3, expected: Vector3, what: &str) {
    num_equal(actual.x, expected.x, &format!("{what}.x"));
    num_equal(actual.y, expected.y, &format!("{what}.y"));
    num_equal(actual.z, expected.z, &format!("{what}.z"));
}

#[track_caller]
fn num_equal_all(actual: &[f64], expected: &[f64], what: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{what}: correct number of segments"
    );
    for (i, (a, e)) in actual.iter().zip(expected).enumerate() {
        num_equal(*a, *e, &format!("{what}[{i}]"));
    }
}

fn v2(x: f64, y: f64) -> Vector2 {
    Vector2::new(x, y)
}

fn v3(x: f64, y: f64, z: f64) -> Vector3 {
    Vector3::new(x, y, z)
}

/// The shared `getUtoTmapping` start / end assertions.
#[track_caller]
fn assert_u_to_t_ends<C: Curve + ?Sized>(curve: &C) {
    assert_eq!(
        curve.get_u_to_t_mapping(0.0, Some(0.0)),
        0.0,
        "getUtoTmapping( 0, 0 ) is the starting point"
    );
    assert_eq!(
        curve.get_u_to_t_mapping(0.0, Some(curve.get_length())),
        1.0,
        "getUtoTmapping( 0, length ) is the ending point"
    );
}

mod arc_curve {
    use super::*;

    fn arc() -> EllipseCurve {
        // `new ArcCurve()`: every argument `undefined`, which EllipseCurve's
        // defaults fill in (0, 0, 1, 1, 0, 2π, false, 0).
        EllipseCurve::arc(0.0, 0.0, 1.0, 0.0, std::f64::consts::PI * 2.0, false)
    }

    #[test]
    fn instancing() {
        let _ = arc();
    }

    #[test]
    fn type_() {
        assert_eq!(arc().type_name(), "ArcCurve");
    }
}

mod cubic_bezier_curve {
    use super::*;

    fn curve() -> CubicBezierCurve<Vector2> {
        CubicBezierCurve::new(
            v2(-10.0, 0.0),
            v2(-5.0, 15.0),
            v2(20.0, 15.0),
            v2(10.0, 0.0),
        )
    }

    #[test]
    fn instancing() {
        let _ = CubicBezierCurve::<Vector2>::default();
    }

    #[test]
    fn type_() {
        assert_eq!(
            CubicBezierCurve::<Vector2>::default().type_name(),
            "CubicBezierCurve"
        );
    }

    #[test]
    fn simple_curve() {
        let curve = curve();
        let mut expected_points = vec![
            v2(-10.0, 0.0),
            v2(-3.359375, 8.4375),
            v2(5.625, 11.25),
            v2(11.796875, 8.4375),
            v2(10.0, 0.0),
        ];

        let points = curve.get_points(expected_points.len() - 1);
        assert_eq!(points, expected_points, "Correct points calculated");

        // symmetry
        let curve_rev = CubicBezierCurve::new(curve.v3, curve.v2, curve.v1, curve.v0);
        let points = curve_rev.get_points(expected_points.len() - 1);
        expected_points.reverse();
        assert_eq!(points, expected_points, "Reversed: Correct points curve");
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            36.64630888504102,
            "Correct length of curve",
        );

        let expected_lengths = [
            0.0,
            10.737285813492393,
            20.15159143794633,
            26.93408340370825,
            35.56079575637337,
        ];
        let lengths = curve.get_lengths(expected_lengths.len() - 1);
        num_equal_all(&lengths, &expected_lengths, "segment");
    }

    #[test]
    fn get_point_at() {
        let curve = curve();
        let expected_points = vec![
            v2(-10.0, 0.0),
            v2(-3.3188282598022596, 8.463722639089221),
            v2(3.4718554735926617, 11.07899406116314),
            v2(10.0, 0.0),
        ];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct points");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangents = [
            v2(0.316370061632252, 0.9486358543207215),
            v2(0.838961283088303, 0.5441911111721949),
            v2(1.0, 0.0),
            v2(0.47628313192245453, -0.8792919755383518),
            v2(-0.5546041767829665, -0.8321142992972107),
        ];
        for (i, (t, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal2(curve.get_tangent(*t), exp, &format!("getTangent #{i}"));
        }

        let expected_tangents = [
            v2(0.316370061632252, 0.9486358543207215),
            v2(0.7794223085548987, 0.6264988945935596),
            v2(0.988266153082452, 0.15274164681452052),
            v2(0.5004110404199416, -0.8657879593906534),
            v2(-0.5546041767829665, -0.8321142992972107),
        ];
        for (i, (u, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal2(curve.get_tangent_at(*u), exp, &format!("getTangentAt #{i}"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.5, Some(1.0)),
            0.02130029182257093,
            "getUtoTmapping( 0.5, 1 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v2(-10.0, 0.0),
            v2(-6.16826457740703, 6.17025727295411),
            v2(-0.058874033259857184, 10.1240558653185),
            v2(7.123523032625162, 11.154913869041575),
            v2(12.301846885754463, 6.808865855469985),
            v2(10.0, 0.0),
        ];
        assert_eq!(
            curve().get_spaced_points(5),
            expected_points,
            "Correct points calculated"
        );
    }
}

mod cubic_bezier_curve3 {
    use super::*;

    fn curve() -> CubicBezierCurve3 {
        CubicBezierCurve3::new(
            v3(-10.0, 0.0, 2.0),
            v3(-5.0, 15.0, 4.0),
            v3(20.0, 15.0, -5.0),
            v3(10.0, 0.0, 10.0),
        )
    }

    #[test]
    fn instancing() {
        let _ = CubicBezierCurve3::default();
    }

    #[test]
    fn type_() {
        assert_eq!(
            CubicBezierCurve3::default().type_name(),
            "CubicBezierCurve3"
        );
    }

    #[test]
    fn simple_curve() {
        let curve = curve();
        let mut expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(-3.359375, 8.4375, 1.984375),
            v3(5.625, 11.25, 1.125),
            v3(11.796875, 8.4375, 2.703125),
            v3(10.0, 0.0, 10.0),
        ];

        let points = curve.get_points(expected_points.len() - 1);
        assert_eq!(points, expected_points, "Correct points calculated");

        // symmetry
        let curve_rev = CubicBezierCurve3::new(curve.v3, curve.v2, curve.v1, curve.v0);
        let points = curve_rev.get_points(expected_points.len() - 1);
        expected_points.reverse();
        assert_eq!(points, expected_points, "Reversed: Correct points curve");
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            39.58103024989427,
            "Correct length of curve",
        );

        let expected_lengths = [
            0.0,
            10.73729718231036,
            20.19074500737662,
            27.154413277853756,
            38.453287150114214,
        ];
        let lengths = curve.get_lengths(expected_lengths.len() - 1);
        num_equal_all(&lengths, &expected_lengths, "segment");
    }

    #[test]
    fn get_point_at() {
        let curve = curve();
        let expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(-2.591880240484318, 8.908333501170798, 1.8953420625251136),
            v3(4.866251460832755, 11.22787914038507, 1.150832855206874),
            v3(10.0, 0.0, 10.0),
        ];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct points");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangents = [
            v3(0.3138715439944244, 0.9411440474105875, 0.12542940601858074),
            v3(0.8351825262580098, 0.54174002562179, -0.09480449605683638),
            v3(0.9997531780538501, 0.0, -0.02221672728433752),
            v3(0.40693407933981185, -0.7512629496079668, 0.5196235518317053),
            v3(
                -0.42632467075185815,
                -0.6396469221230213,
                0.6396085444448543,
            ),
        ];
        for (i, (t, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal3_xy(curve.get_tangent(*t), exp, &format!("getTangent #{i}"));
        }

        let expected_tangents = [
            v3(0.3138715439944244, 0.9411440474105875, 0.12542940601858074),
            v3(0.8016539573770751, 0.5918626760037707, -0.08396133262002324),
            v3(
                0.997337559412928,
                0.05740742907719314,
                -0.044968652092444425,
            ),
            v3(0.1389373097746809, -0.7882209938358005, 0.5995032016837588),
            v3(
                -0.42632467075185815,
                -0.6396469221230213,
                0.6396085444448543,
            ),
        ];
        for (i, (u, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal3_xy(curve.get_tangent_at(*u), exp, &format!("getTangentAt #{i}"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.5, Some(1.0)),
            0.021163245321323316,
            "getUtoTmapping( 0.5, 1 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(-5.756524515061918, 6.568020242700483, 2.22116711170301),
            v3(1.0003511895116906, 10.49656064587831, 1.4727101010850698),
            v3(8.767656412295171, 10.784286845278622, 1.2873599519775174),
            v3(12.306772513558396, 5.545103788071547, 4.909948454535794),
            v3(10.0, 0.0, 10.0),
        ];
        assert_eq!(
            curve().get_spaced_points(5),
            expected_points,
            "Correct points calculated"
        );
    }

    #[test]
    fn compute_frenet_frames() {
        let binormals = [
            v3(
                -0.9486358543207215,
                0.316370061632252,
                -6.938893903907228e-18,
            ),
            v3(
                -0.05491430765311864,
                0.9969838307670049,
                0.054842137122173326,
            ),
            v3(0.5944656510461876, 0.334836503700931, 0.7310917216844742),
        ];
        let normals = [
            v3(
                0.03968210891259515,
                0.11898683173537697,
                -0.9921025471723304,
            ),
            v3(
                -0.047981365124836806,
                0.05222670079466692,
                -0.9974819097732357,
            ),
            v3(
                0.6818048583242511,
                -0.6919077473246573,
                -0.23749906180354932,
            ),
        ];
        let tangents = [
            v3(0.3138715439944244, 0.9411440474105875, 0.12542940601858074),
            v3(
                0.9973375594129282,
                0.05740742907719315,
                -0.04496865209244443,
            ),
            v3(
                -0.42632467075185815,
                -0.6396469221230213,
                0.6396085444448543,
            ),
        ];

        let frames = curve().compute_frenet_frames(2, false);
        for j in 0..3 {
            num_equal3(
                frames.binormals[j],
                binormals[j],
                &format!("binormals[{j}]"),
            );
            num_equal3(frames.normals[j], normals[j], &format!("normals[{j}]"));
            num_equal3(frames.tangents[j], tangents[j], &format!("tangents[{j}]"));
        }
    }
}

mod ellipse_curve {
    use super::*;

    fn curve() -> EllipseCurve {
        EllipseCurve::new(
            0.0,
            0.0, // ax, aY
            10.0,
            10.0, // xRadius, yRadius
            0.0,
            2.0 * std::f64::consts::PI, // aStartAngle, aEndAngle
            false,                      // aClockwise
            0.0,                        // aRotation
        )
    }

    #[test]
    fn instancing() {
        let _ = EllipseCurve::default();
    }

    #[test]
    fn type_() {
        assert_eq!(EllipseCurve::default().type_name(), "EllipseCurve");
    }

    #[test]
    fn simple_curve() {
        let expected_points = [
            v2(10.0, 0.0),
            v2(0.0, 10.0),
            v2(-10.0, 0.0),
            v2(0.0, -10.0),
            v2(10.0, 0.0),
        ];
        let points = curve().get_points(expected_points.len() - 1);
        assert_eq!(
            points.len(),
            expected_points.len(),
            "Correct number of points"
        );
        for (i, (p, e)) in points.iter().zip(expected_points).enumerate() {
            num_equal2(*p, e, &format!("point[{i}]"));
        }
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            62.829269247282795,
            "Correct length of curve",
        );

        let expected_lengths = [
            0.0,
            11.755705045849462,
            23.51141009169892,
            35.26711513754839,
            47.02282018339785,
            58.77852522924731,
        ];
        num_equal_all(&curve.get_lengths(5), &expected_lengths, "segment");
    }

    #[test]
    fn get_point_get_point_at() {
        let curve = curve();
        for val in [0.0, 0.3, 0.5, 0.7, 1.0] {
            let expected_x = (val * std::f64::consts::PI * 2.0).cos() * 10.0;
            let expected_y = (val * std::f64::consts::PI * 2.0).sin() * 10.0;

            num_equal2(
                curve.get_point(val),
                v2(expected_x, expected_y),
                &format!("getPoint({val})"),
            );
            num_equal2(
                curve.get_point_at(val),
                v2(expected_x, expected_y),
                &format!("getPointAt({val})"),
            );
        }
    }

    #[test]
    fn get_tangent() {
        let curve = curve();
        let expected_tangents = [
            v2(-0.000314159260186071, 0.9999999506519786),
            v2(-1.0, 0.0),
            v2(0.0, -1.0),
            v2(1.0, 0.0),
            v2(0.00031415926018600165, 0.9999999506519784),
        ];
        for (i, (t, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal2(curve.get_tangent(*t), exp, &format!("getTangent #{i}"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.7, Some(1.0)),
            0.01591614882650014,
            "getUtoTmapping( 0.7, 1 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = [
            v2(10.0, 0.0),
            v2(3.0901699437494603, 9.51056516295154),
            v2(-8.090169943749492, 5.877852522924707),
            v2(-8.090169943749459, -5.877852522924751),
            v2(3.0901699437494807, -9.510565162951533),
            v2(10.0, -2.4492935982947065e-15),
        ];
        let points = curve().get_spaced_points(5);
        assert_eq!(
            points.len(),
            expected_points.len(),
            "Correct number of points"
        );
        for (i, (p, e)) in points.iter().zip(expected_points).enumerate() {
            num_equal2(*p, e, &format!("Point #{i}"));
        }
    }
}

mod line_curve {
    use super::*;

    fn points() -> [Vector2; 4] {
        [v2(0.0, 0.0), v2(10.0, 10.0), v2(-10.0, 10.0), v2(-8.0, 5.0)]
    }

    fn curve() -> LineCurve<Vector2> {
        let p = points();
        LineCurve::new(p[0], p[1])
    }

    #[test]
    fn instancing() {
        let _ = LineCurve::<Vector2>::default();
    }

    #[test]
    fn type_() {
        assert_eq!(LineCurve::<Vector2>::default().type_name(), "LineCurve");
    }

    #[test]
    fn get_point_at() {
        let p = points();
        let curve = LineCurve::new(p[0], p[3]);
        let expected_points = vec![v2(0.0, 0.0), v2(-2.4, 1.5), v2(-4.0, 2.5), v2(-8.0, 5.0)];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct points");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangent = 0.5f64.sqrt();

        let tangent = curve.get_tangent(0.0);
        num_equal(tangent.x, expected_tangent, "tangent.x correct");
        num_equal(tangent.y, expected_tangent, "tangent.y correct");

        let tangent = curve.get_tangent_at(0.0);
        num_equal(tangent.x, expected_tangent, "tangentAt.x correct");
        num_equal(tangent.y, expected_tangent, "tangentAt.y correct");
    }

    #[test]
    fn simple_curve() {
        let expected_points = vec![
            v2(0.0, 0.0),
            v2(2.0, 2.0),
            v2(4.0, 4.0),
            v2(6.0, 6.0),
            v2(8.0, 8.0),
            v2(10.0, 10.0),
        ];
        assert_eq!(
            curve().get_points(5),
            expected_points,
            "Correct points for first curve"
        );

        let p = points();
        let curve = LineCurve::new(p[1], p[2]);
        let expected_points = vec![
            v2(10.0, 10.0),
            v2(6.0, 10.0),
            v2(2.0, 10.0),
            v2(-2.0, 10.0),
            v2(-6.0, 10.0),
            v2(-10.0, 10.0),
        ];
        assert_eq!(
            curve.get_points(5),
            expected_points,
            "Correct points for second curve"
        );
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(curve.get_length(), 200f64.sqrt(), "Correct length of curve");

        let expected_lengths = [
            0.0,
            8f64.sqrt(),
            32f64.sqrt(),
            72f64.sqrt(),
            128f64.sqrt(),
            200f64.sqrt(),
        ];
        num_equal_all(&curve.get_lengths(5), &expected_lengths, "segment");
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.3, Some(0.0)),
            0.3,
            "getUtoTmapping( 0.3, 0 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v2(0.0, 0.0),
            v2(2.5, 2.5),
            v2(5.0, 5.0),
            v2(7.5, 7.5),
            v2(10.0, 10.0),
        ];
        assert_eq!(
            curve().get_spaced_points(4),
            expected_points,
            "Correct points calculated"
        );
    }
}

mod line_curve3 {
    use super::*;

    fn points() -> [Vector3; 4] {
        [
            v3(0.0, 0.0, 0.0),
            v3(10.0, 10.0, 10.0),
            v3(-10.0, 10.0, -10.0),
            v3(-8.0, 5.0, -7.0),
        ]
    }

    fn curve() -> LineCurve3 {
        let p = points();
        LineCurve3::new(p[0], p[1])
    }

    #[test]
    fn instancing() {
        let _ = LineCurve3::default();
    }

    #[test]
    fn type_() {
        assert_eq!(LineCurve3::default().type_name(), "LineCurve3");
    }

    #[test]
    fn get_point_at() {
        let p = points();
        let curve = LineCurve3::new(p[0], p[3]);
        let expected_points = vec![
            v3(0.0, 0.0, 0.0),
            v3(-2.4, 1.5, -2.1),
            v3(-4.0, 2.5, -3.5),
            v3(-8.0, 5.0, -7.0),
        ];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct getPointAt points");
    }

    #[test]
    fn simple_curve() {
        let expected_points = vec![
            v3(0.0, 0.0, 0.0),
            v3(2.0, 2.0, 2.0),
            v3(4.0, 4.0, 4.0),
            v3(6.0, 6.0, 6.0),
            v3(8.0, 8.0, 8.0),
            v3(10.0, 10.0, 10.0),
        ];
        assert_eq!(
            curve().get_points(5),
            expected_points,
            "Correct points for first curve"
        );

        let p = points();
        let curve = LineCurve3::new(p[1], p[2]);
        let expected_points = vec![
            v3(10.0, 10.0, 10.0),
            v3(6.0, 10.0, 6.0),
            v3(2.0, 10.0, 2.0),
            v3(-2.0, 10.0, -2.0),
            v3(-6.0, 10.0, -6.0),
            v3(-10.0, 10.0, -10.0),
        ];
        assert_eq!(
            curve.get_points(5),
            expected_points,
            "Correct points for second curve"
        );
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(curve.get_length(), 300f64.sqrt(), "Correct length of curve");

        let expected_lengths = [
            0.0,
            12f64.sqrt(),
            48f64.sqrt(),
            108f64.sqrt(),
            192f64.sqrt(),
            300f64.sqrt(),
        ];
        num_equal_all(&curve.get_lengths(5), &expected_lengths, "segment");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangent = (1.0f64 / 3.0).sqrt();
        let expected = v3(expected_tangent, expected_tangent, expected_tangent);

        num_equal3(curve.get_tangent(0.5), expected, "tangent");
        num_equal3(curve.get_tangent_at(0.5), expected, "tangentAt");
    }

    #[test]
    fn compute_frenet_frames() {
        let frames = curve().compute_frenet_frames(1, false);
        num_equal3(
            frames.binormals[0],
            v3(-0.5 * 2f64.sqrt(), 0.5 * 2f64.sqrt(), 0.0),
            "Frenet frames binormals",
        );
        num_equal3(
            frames.normals[0],
            v3(
                (1.0f64 / 6.0).sqrt(),
                (1.0f64 / 6.0).sqrt(),
                -(2.0f64 / 3.0).sqrt(),
            ),
            "Frenet frames normals",
        );
        num_equal3(
            frames.tangents[0],
            v3(
                (1.0f64 / 3.0).sqrt(),
                (1.0f64 / 3.0).sqrt(),
                (1.0f64 / 3.0).sqrt(),
            ),
            "Frenet frames tangents",
        );
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.7, Some(0.0)),
            0.7,
            "getUtoTmapping( 0.7, 0 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v3(0.0, 0.0, 0.0),
            v3(2.5, 2.5, 2.5),
            v3(5.0, 5.0, 5.0),
            v3(7.5, 7.5, 7.5),
            v3(10.0, 10.0, 10.0),
        ];
        assert_eq!(
            curve().get_spaced_points(4),
            expected_points,
            "Correct points calculated"
        );
    }
}

mod quadratic_bezier_curve {
    use super::*;

    fn curve() -> QuadraticBezierCurve<Vector2> {
        QuadraticBezierCurve::new(v2(-10.0, 0.0), v2(20.0, 15.0), v2(10.0, 0.0))
    }

    #[test]
    fn instancing() {
        let _ = QuadraticBezierCurve::<Vector2>::default();
    }

    #[test]
    fn type_() {
        assert_eq!(
            QuadraticBezierCurve::<Vector2>::default().type_name(),
            "QuadraticBezierCurve"
        );
    }

    #[test]
    fn simple_curve() {
        let curve = curve();
        let mut expected_points = vec![
            v2(-10.0, 0.0),
            v2(2.5, 5.625),
            v2(10.0, 7.5),
            v2(12.5, 5.625),
            v2(10.0, 0.0),
        ];
        assert_eq!(
            curve.get_points(expected_points.len() - 1),
            expected_points,
            "Correct points calculated"
        );

        // symmetry
        let curve_rev = QuadraticBezierCurve::new(curve.v2, curve.v1, curve.v0);
        expected_points.reverse();
        assert_eq!(
            curve_rev.get_points(expected_points.len() - 1),
            expected_points,
            "Reversed: Correct points curve"
        );
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            31.269026549416683,
            "Correct length of curve",
        );

        let expected_lengths = [
            0.0,
            13.707320124663317,
            21.43814317269643,
            24.56314317269643,
            30.718679298818998,
        ];
        let lengths = curve.get_lengths(expected_lengths.len() - 1);
        num_equal_all(&lengths, &expected_lengths, "segment");
    }

    #[test]
    fn get_point_at() {
        let curve = curve();
        let expected_points = vec![
            v2(-10.0, 0.0),
            v2(-1.5127849599387615, 3.993582003773624),
            v2(4.310076165722796, 6.269921971403917),
            v2(10.0, 0.0),
        ];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct points");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangents = [
            v2(0.89443315420562, 0.44720166888975904),
            v2(0.936329177569021, 0.3511234415884543),
            v2(1.0, 0.0),
            v2(-5.921189464667277e-13, -1.0),
            v2(-0.5546617882904897, -0.8320758983472577),
        ];
        for (i, (t, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal2(curve.get_tangent(*t), exp, &format!("getTangent #{i}"));
        }

        let expected_tangents = [
            v2(0.89443315420562, 0.44720166888975904),
            v2(0.9125211423360805, 0.40902954024086674),
            v2(0.9480289098765387, 0.3181842014278863),
            v2(0.7969127189169473, -0.6040944615111106),
            v2(-0.5546617882904897, -0.8320758983472577),
        ];
        for (i, (u, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal2(curve.get_tangent_at(*u), exp, &format!("getTangentAt #{i}"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.5, Some(1.0)),
            0.015073978276116116,
            "getUtoTmapping( 0.5, 1 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v2(-10.0, 0.0),
            v2(-4.366603655406173, 2.715408933540383),
            v2(1.3752241477827831, 5.191972084404416),
            v2(7.312990279153634, 7.136310044848586),
            v2(12.499856644824826, 5.653289188715387),
            v2(10.0, 0.0),
        ];
        assert_eq!(
            curve().get_spaced_points(5),
            expected_points,
            "Correct points calculated"
        );
    }
}

mod quadratic_bezier_curve3 {
    use super::*;

    fn curve() -> QuadraticBezierCurve3 {
        QuadraticBezierCurve3::new(
            v3(-10.0, 0.0, 2.0),
            v3(20.0, 15.0, -5.0),
            v3(10.0, 0.0, 10.0),
        )
    }

    #[test]
    fn instancing() {
        let _ = QuadraticBezierCurve3::default();
    }

    #[test]
    fn type_() {
        assert_eq!(
            QuadraticBezierCurve3::default().type_name(),
            "QuadraticBezierCurve3"
        );
    }

    #[test]
    fn simple_curve() {
        let curve = curve();
        let mut expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(2.5, 5.625, -0.125),
            v3(10.0, 7.5, 0.5),
            v3(12.5, 5.625, 3.875),
            v3(10.0, 0.0, 10.0),
        ];
        assert_eq!(
            curve.get_points(expected_points.len() - 1),
            expected_points,
            "Correct points calculated"
        );

        // symmetry
        let curve_rev = QuadraticBezierCurve3::new(curve.v2, curve.v1, curve.v0);
        expected_points.reverse();
        assert_eq!(
            curve_rev.get_points(expected_points.len() - 1),
            expected_points,
            "Reversed: Correct points curve"
        );
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            35.47294274967861,
            "Correct length of curve",
        );

        let expected_lengths = [
            0.0,
            13.871057998581074,
            21.62710402732536,
            26.226696400568883,
            34.91037361704809,
        ];
        let lengths = curve.get_lengths(expected_lengths.len() - 1);
        num_equal_all(&lengths, &expected_lengths, "segment");
    }

    #[test]
    fn get_point_at() {
        let curve = curve();
        let expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(-0.4981634504454243, 4.427089043881476, 0.19308849757196012),
            v3(6.149415812887238, 6.838853310980195, -0.20278120208668637),
            v3(10.0, 0.0, 10.0),
        ];
        let points: Vec<_> = [0.0, 0.3, 0.5, 1.0]
            .iter()
            .map(|&u| curve.get_point_at(u))
            .collect();
        assert_eq!(points, expected_points, "Correct points");
    }

    #[test]
    fn get_tangent_get_tangent_at() {
        let curve = curve();
        let expected_tangents = [
            v3(0.8755715084258769, 0.4377711603816079, -0.2042815331129452),
            v3(0.9340289249885844, 0.3502608468707904, -0.07005216937416067),
            v3(0.9284766908853163, 0.0, 0.37139067635396156),
            v3(
                -3.669031233375946e-13,
                -0.6196442885791218,
                0.7848827655333463,
            ),
            v3(-0.4263618889888853, -0.6396068005601663, 0.6396238584473043),
        ];
        for (i, (t, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal3_xy(curve.get_tangent(*t), exp, &format!("getTangent #{i}"));
        }

        let expected_tangents = [
            v3(0.8755715084258769, 0.4377711603816079, -0.2042815331129452),
            v3(0.9060725703490549, 0.3984742932857448, -0.14230507668907377),
            v3(0.9621604167456882, 0.2688562845452628, 0.044312872940942424),
            v3(
                0.016586454041780826,
                -0.6163270940470614,
                0.7873155674098058,
            ),
            v3(-0.4263618889888853, -0.6396068005601663, 0.6396238584473043),
        ];
        for (i, (u, exp)) in [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .zip(expected_tangents)
            .enumerate()
        {
            num_equal3_xy(curve.get_tangent_at(*u), exp, &format!("getTangentAt #{i}"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.5, Some(1.0)),
            0.014760890927167196,
            "getUtoTmapping( 0.5, 1 ) is correct",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = vec![
            v3(-10.0, 0.0, 2.0),
            v3(-3.712652983516992, 3.015179001762753, 0.6957120710270492),
            v3(2.7830973773262975, 5.730399338061483, -0.1452668772806931),
            v3(9.575825284074465, 7.48754187603603, 0.3461104039841496),
            v3(12.345199937734154, 4.575759904730531, 5.142117429101656),
            v3(10.0, 0.0, 10.0),
        ];
        assert_eq!(
            curve().get_spaced_points(5),
            expected_points,
            "Correct points calculated"
        );
    }

    #[test]
    fn compute_frenet_frames() {
        let binormals = [
            v3(-0.447201668889759, 0.8944331542056199, 0.0),
            v3(
                -0.2684231751110917,
                0.9631753839815436,
                -0.01556209353802903,
            ),
            v3(0.3459273556592433, 0.53807011680075, 0.7686447905324219),
        ];
        let normals = [
            v3(
                -0.18271617600817133,
                -0.09135504253146765,
                -0.9789121795283909,
            ),
            v3(
                0.046865035058597876,
                -0.003078628350883253,
                -0.9988964863970807,
            ),
            v3(
                0.8357929194629689,
                -0.5489842348221077,
                0.008155102228190641,
            ),
        ];
        let tangents = [
            v3(0.8755715084258767, 0.4377711603816078, -0.20428153311294514),
            v3(0.9621604167456884, 0.26885628454526284, 0.04431287294094243),
            v3(-0.4263618889888853, -0.6396068005601663, 0.6396238584473043),
        ];

        let frames = curve().compute_frenet_frames(2, false);
        for j in 0..3 {
            num_equal3(
                frames.binormals[j],
                binormals[j],
                &format!("binormals[{j}]"),
            );
            num_equal3(frames.normals[j], normals[j], &format!("normals[{j}]"));
            num_equal3(frames.tangents[j], tangents[j], &format!("tangents[{j}]"));
        }
    }
}

mod spline_curve {
    use super::*;

    fn curve() -> SplineCurve {
        SplineCurve::new(vec![
            v2(-10.0, 0.0),
            v2(-5.0, 5.0),
            v2(0.0, 0.0),
            v2(5.0, -5.0),
            v2(10.0, 0.0),
        ])
    }

    #[test]
    fn instancing() {
        let _ = SplineCurve::default();
    }

    #[test]
    fn type_() {
        assert_eq!(SplineCurve::default().type_name(), "SplineCurve");
    }

    #[test]
    fn simple_curve() {
        let curve = curve();
        let expected_points = [
            v2(-10.0, 0.0),
            v2(-6.08, 4.56),
            v2(-2.0, 2.48),
            v2(2.0, -2.48),
            v2(6.08, -4.56),
            v2(10.0, 0.0),
        ];
        let points = curve.get_points(5);
        assert_eq!(
            points.len(),
            expected_points.len(),
            "1st: Correct number of points"
        );
        for (i, (p, e)) in points.iter().zip(expected_points).enumerate() {
            num_equal2(*p, e, &format!("points[{i}]"));
        }

        let points = curve.get_points(4);
        assert_eq!(
            points, curve.points,
            "2nd: Returned points are identical to control points"
        );
    }

    #[test]
    fn get_length_get_lengths() {
        let curve = curve();
        num_equal(
            curve.get_length(),
            28.876950901868135,
            "Correct length of curve",
        );

        let expected_lengths = vec![
            0.0,
            50f64.sqrt(),
            200f64.sqrt(),
            450f64.sqrt(),
            800f64.sqrt(),
        ];
        assert_eq!(
            curve.get_lengths(4),
            expected_lengths,
            "Correct segment lengths"
        );
    }

    #[test]
    fn get_point_at() {
        let curve = curve();
        assert!(
            curve.get_point_at(0.0).equals(&curve.points[0]),
            "PointAt 0.0 correct"
        );
        assert!(
            curve.get_point_at(1.0).equals(&curve.points[4]),
            "PointAt 1.0 correct"
        );

        let point = curve.get_point_at(0.5);
        num_equal(point.x, 0.0, "PointAt 0.5 x correct");
        num_equal(point.y, 0.0, "PointAt 0.5 y correct");
    }

    #[test]
    fn get_tangent() {
        let curve = curve();
        let expected_tangent = [
            v2(0.7068243340243188, 0.7073891155729485),  // 0
            v2(0.7069654305325396, -0.7072481035902046), // 0.5
            v2(0.7068243340245123, 0.7073891155727552),  // 1
        ];
        for (i, (t, exp)) in [0.0, 0.5, 1.0].iter().zip(expected_tangent).enumerate() {
            num_equal2(curve.get_tangent(*t), exp, &format!("tangent[{i}]"));
        }
    }

    #[test]
    fn get_u_to_t_mapping() {
        let curve = curve();
        assert_u_to_t_ends(&curve);
        num_equal(
            curve.get_u_to_t_mapping(0.5, Some(0.0)),
            0.5,
            "getUtoTmapping( 0.5, 0 ) is the middle",
        );
    }

    #[test]
    fn get_spaced_points() {
        let expected_points = [
            v2(-10.0, 0.0),
            v2(-4.996509634683014, 4.999995128640857),
            v2(0.0, 0.0),
            v2(4.996509634683006, -4.999995128640857),
            v2(10.0, 0.0),
        ];
        let points = curve().get_spaced_points(4);
        assert_eq!(
            points.len(),
            expected_points.len(),
            "Correct number of points"
        );
        for (i, (p, e)) in points.iter().zip(expected_points).enumerate() {
            num_equal2(*p, e, &format!("points[{i}]"));
        }
    }
}
