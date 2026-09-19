//! Port of `three.js/test/unit/src/extras/curves/CatmullRomCurve3.tests.js`.
//!
//! Expectations and epsilons are three.js' own; nothing here was recomputed
//! from the Rust implementation.
//!
//! The `Extending`, `Instancing`, `type` and `isCatmullRomCurve3` tests are not
//! ported: they are JS class-identity checks (`instanceof Curve`, the `type`
//! string, the `isCatmullRomCurve3` brand) that the Rust port expresses through
//! the type system and so have nothing to assert.

mod support;

use support::close;
use three_rs::extras::{CatmullRomCurve3, Curve, CurveType};
use three_rs::math::Vector3;

/// `qunit-utils.js` `assert.numEqual` compares with `diff < 0.1`.
const NUM_EQ: f64 = 0.1;

/// The module-level `positions` array the tests share.
fn positions() -> Vec<Vector3> {
    vec![
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-60.0, 20.0, 60.0),
        Vector3::new(-60.0, 120.0, 60.0),
        Vector3::new(60.0, 20.0, -60.0),
        Vector3::new(60.0, -100.0, -60.0),
    ]
}

/// `points.forEach( ... assert.numEqual( point.x, expectedPoints[ i ].x ) ... )`.
#[track_caller]
fn num_equal_points(points: &[Vector3], expected: &[Vector3]) {
    assert_eq!(points.len(), expected.len(), "correct number of points.");

    for (i, (point, exp)) in points.iter().zip(expected.iter()).enumerate() {
        close(point.x, exp.x, NUM_EQ, &format!("points[{i}].x"));
        close(point.y, exp.y, NUM_EQ, &format!("points[{i}].y"));
        close(point.z, exp.z, NUM_EQ, &format!("points[{i}].z"));
    }
}

// OTHERS
#[test]
fn catmullrom_check() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-60.0, -51.04, 60.0),
        Vector3::new(-60.0, -2.7199999999999998, 60.0),
        Vector3::new(-61.92, 44.48, 61.92),
        Vector3::new(-68.64, 95.36000000000001, 68.64),
        Vector3::new(-60.0, 120.0, 60.0),
        Vector3::new(-14.880000000000017, 95.36000000000001, 14.880000000000017),
        Vector3::new(41.75999999999997, 44.48000000000003, -41.75999999999997),
        Vector3::new(67.68, -2.720000000000023, -67.68),
        Vector3::new(65.75999999999999, -51.04000000000001, -65.75999999999999),
        Vector3::new(60.0, -100.0, -60.0),
    ];

    let points = curve.get_points(10);

    num_equal_points(&points, &expected_points);
}

#[test]
fn chordal_basic_check() {
    let mut curve = CatmullRomCurve3::new(positions());

    curve.curve_type = CurveType::Chordal;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-60.0, -52.0, 60.0),
        Vector3::new(-60.0, -4.0, 60.0),
        Vector3::new(-60.656435889910924, 41.62455386421379, 60.656435889910924),
        Vector3::new(-62.95396150459915, 87.31049238896205, 62.95396150459915),
        Vector3::new(-60.0, 120.0, 60.0),
        Vector3::new(-16.302568199486444, 114.1500463116312, 16.302568199486444),
        Vector3::new(42.998098664956586, 54.017050116427455, -42.998098664956586),
        Vector3::new(63.542500175682434, -1.137153397546383, -63.542500175682434),
        Vector3::new(62.65687513176183, -49.85286504815978, -62.65687513176183),
        Vector3::new(60.00000000000001, -100.0, -60.00000000000001),
    ];

    let points = curve.get_points(10);

    num_equal_points(&points, &expected_points);
}

#[test]
fn centripetal_basic_check() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::Centripetal;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-60.0, -51.47527724919028, 60.0),
        Vector3::new(-60.0, -3.300369665587032, 60.0),
        Vector3::new(-61.13836565863938, 42.86306307781241, 61.13836565863938),
        Vector3::new(-65.1226454638772, 90.69743905511538, 65.1226454638772),
        Vector3::new(-60.0, 120.0, 60.0),
        Vector3::new(-15.620412575504497, 103.10790870179872, 15.620412575504497),
        Vector3::new(42.384384731047874, 48.35477686933143, -42.384384731047874),
        Vector3::new(65.25545512241153, -1.646250966068339, -65.25545512241153),
        Vector3::new(63.94159134180865, -50.234688224551256, -63.94159134180865),
        Vector3::new(59.99999999999999, -100.0, -59.99999999999999),
    ];

    let points = curve.get_points(10);

    num_equal_points(&points, &expected_points);
}

#[test]
fn closed_catmullrom_basic_check() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;
    curve.closed = true;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-67.5, -46.25, 67.5),
        Vector3::new(-60.0, 20.0, 60.0),
        Vector3::new(-67.5, 83.75, 67.5),
        Vector3::new(-60.0, 120.0, 60.0),
        Vector3::new(0.0, 83.75, 0.0),
        Vector3::new(60.0, 20.0, -60.0),
        Vector3::new(75.0, -46.25, -75.0),
        Vector3::new(60.0, -100.0, -60.0),
        Vector3::new(0.0, -115.0, 0.0),
        Vector3::new(-60.0, -100.0, 60.0),
    ];

    let points = curve.get_points(10);

    num_equal_points(&points, &expected_points);
}

//
// curve.type = 'catmullrom'; only from here on
//
#[test]
fn get_length_get_lengths() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let length = curve.get_length();
    let expected_length = 551.549686276872;

    close(length, expected_length, NUM_EQ, "Correct length of curve");

    let expected_lengths = [0.0, 120.0, 220.0, 416.9771560359221, 536.9771560359221];
    let lengths = curve.get_lengths(expected_lengths.len() - 1);

    assert_eq!(
        lengths.len(),
        expected_lengths.len(),
        "Correct number of segments"
    );

    for (i, (segment, expected)) in lengths.iter().zip(expected_lengths.iter()).enumerate() {
        close(
            *segment,
            *expected,
            NUM_EQ,
            &format!("segment[{i}] correct"),
        );
    }
}

#[test]
fn get_point_at() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-64.84177333183106, 64.86956465359813, 64.84177333183106),
        Vector3::new(-28.288507045700854, 104.83101184518996, 28.288507045700854),
        Vector3::new(60.0, -100.0, -60.0),
    ];

    let points = [
        curve.get_point_at(0.0),
        curve.get_point_at(0.3),
        curve.get_point_at(0.5),
        curve.get_point_at(1.0),
    ];

    // `assert.deepEqual`: exact equality, not `numEqual`.
    assert_eq!(points, expected_points, "Correct points");
}

#[test]
fn get_tangent_get_tangent_at() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let expected_tangents = [
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(
            -0.0001090274561657922,
            0.9999999881130137,
            0.0001090274561657922,
        ),
        Vector3::new(
            0.7071067811865475,
            -2.0930381713877622e-13,
            -0.7071067811865475,
        ),
        Vector3::new(
            0.43189437062802816,
            -0.7917919583070032,
            -0.43189437062802816,
        ),
        Vector3::new(
            -0.00019991333100812723,
            -0.9999999600346592,
            0.00019991333100812723,
        ),
    ];

    let tangents = [
        curve.get_tangent(0.0),
        curve.get_tangent(0.25),
        curve.get_tangent(0.5),
        curve.get_tangent(0.75),
        curve.get_tangent(1.0),
    ];

    // three.js checks only x and y here.
    for (i, exp) in expected_tangents.iter().enumerate() {
        let tangent = tangents[i];

        close(
            tangent.x,
            exp.x,
            NUM_EQ,
            &format!("getTangent #{i}: x correct"),
        );
        close(
            tangent.y,
            exp.y,
            NUM_EQ,
            &format!("getTangent #{i}: y correct"),
        );
    }

    //

    let expected_tangents = [
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(
            -0.10709018822205997,
            0.9884651653817284,
            0.10709018822205997,
        ),
        Vector3::new(0.6396363672964268, -0.4262987629159402, -0.6396363672964268),
        Vector3::new(0.5077298411616501, -0.6960034603275557, -0.5077298411616501),
        Vector3::new(
            -0.00019991333100812723,
            -0.9999999600346592,
            0.00019991333100812723,
        ),
    ];

    let tangents = [
        curve.get_tangent_at(0.0),
        curve.get_tangent_at(0.25),
        curve.get_tangent_at(0.5),
        curve.get_tangent_at(0.75),
        curve.get_tangent_at(1.0),
    ];

    for (i, exp) in expected_tangents.iter().enumerate() {
        let tangent = tangents[i];

        close(
            tangent.x,
            exp.x,
            NUM_EQ,
            &format!("getTangentAt #{i}: x correct"),
        );
        close(
            tangent.y,
            exp.y,
            NUM_EQ,
            &format!("getTangentAt #{i}: y correct"),
        );
    }
}

#[test]
fn compute_frenet_frames() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let expected_binormals = [
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(-0.28685061854203, 0.6396363672964267, -0.7131493814579701),
        Vector3::new(
            -1.9982670528160395e-8,
            -0.0001999133310081272,
            -0.9999999800173295,
        ),
    ];
    let expected_normals = [
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(
            -0.7131493814579699,
            -0.6396363672964268,
            -0.2868506185420297,
        ),
        Vector3::new(
            -0.9999999800173294,
            0.00019991333100810582,
            -1.99826701852146e-8,
        ),
    ];
    let expected_tangents = [
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.6396363672964269, -0.4262987629159403, -0.6396363672964269),
        Vector3::new(
            -0.0001999133310081273,
            -0.9999999600346594,
            0.0001999133310081273,
        ),
    ];

    let frames = curve.compute_frenet_frames(2, false);

    for (i, (group, expected)) in [
        (&frames.binormals, &expected_binormals),
        (&frames.normals, &expected_normals),
        (&frames.tangents, &expected_tangents),
    ]
    .iter()
    .enumerate()
    {
        for (j, vec) in expected.iter().enumerate() {
            close(
                group[j].x,
                vec.x,
                NUM_EQ,
                &format!("Frenet frames [{i}, {j}].x correct"),
            );
            close(
                group[j].y,
                vec.y,
                NUM_EQ,
                &format!("Frenet frames [{i}, {j}].y correct"),
            );
            close(
                group[j].z,
                vec.z,
                NUM_EQ,
                &format!("Frenet frames [{i}, {j}].z correct"),
            );
        }
    }
}

#[test]
fn get_u_to_t_mapping() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let start = curve.get_u_to_t_mapping(0.0, Some(0.0));
    let end = curve.get_u_to_t_mapping(0.0, Some(curve.get_length()));
    let somewhere = curve.get_u_to_t_mapping(0.5, Some(500.0));

    let expected_somewhere = 0.8964116382083199;

    assert_eq!(start, 0.0, "getUtoTmapping( 0, 0 ) is the starting point");
    assert_eq!(end, 1.0, "getUtoTmapping( 0, length ) is the ending point");
    close(
        somewhere,
        expected_somewhere,
        NUM_EQ,
        "getUtoTmapping( 0.5, 500 ) is correct",
    );
}

#[test]
fn get_spaced_points() {
    let mut curve = CatmullRomCurve3::new(positions());
    curve.curve_type = CurveType::CatmullRom;

    let expected_points = [
        Vector3::new(-60.0, -100.0, 60.0),
        Vector3::new(-60.0, 10.311489426555056, 60.0),
        Vector3::new(-65.05889864636504, 117.99691802595966, 65.05889864636504),
        Vector3::new(6.054276900088592, 78.7153118386369, -6.054276900088592),
        Vector3::new(64.9991491385602, 8.386980812799566, -64.9991491385602),
        Vector3::new(60.0, -100.0, -60.0),
    ];

    // `getSpacedPoints()` with no argument: three.js' `divisions = 5`.
    let points = curve.get_spaced_points(5);

    assert_eq!(
        points.len(),
        expected_points.len(),
        "Correct number of points"
    );
    assert_eq!(
        points.as_slice(),
        expected_points.as_slice(),
        "Correct points calculated"
    );
}

#[test]
fn two_points() {
    let mut curve = CatmullRomCurve3::new(vec![
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
    ]);
    curve.curve_type = CurveType::CatmullRom;

    let expected_points = [
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(5.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 0.0),
    ];

    let points = curve.get_points(2);

    assert_eq!(
        points.len(),
        expected_points.len(),
        "Correct number of points"
    );
    assert_eq!(
        points.as_slice(),
        expected_points.as_slice(),
        "Correct points calculated"
    );
}
