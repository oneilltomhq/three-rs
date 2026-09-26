//! Ports of `three.js/test/unit/src/extras/core/`: `Curve`, `CurvePath`,
//! `Path`, `Shape` and `ShapePath`, plus a few `ShapeUtils` checks three.js
//! has no unit file for.
//!
//! three.js' `Extending` tests (`instanceof`) become deref checks: `Path`
//! derefs to its `CurvePath` and `Shape` to its `Path`, which is how the port
//! models those two `extends`. `Curve` is a trait in the port, so `new
//! Curve()` and its `type` have no counterpart; the `Curve` base behaviour is
//! covered through the subclasses in `tests/extras_curves.rs`.

use three_rs::extras::shape_utils;
use three_rs::extras::{Curve, CurvePath, Path, Shape, ShapePath};
use three_rs::math::{Vector2, Vector3};

mod curve_path {
    use super::*;

    #[test]
    fn instancing() {
        let _ = CurvePath::<Vector2>::new();
        let _ = CurvePath::<Vector3>::new();
    }

    #[test]
    fn type_() {
        assert_eq!(CurvePath::<Vector2>::new().type_name(), "CurvePath");
    }
}

mod path {
    use super::*;

    #[test]
    fn extending() {
        let object = Path::new();
        let _: &CurvePath<Vector2> = &object;
    }

    #[test]
    fn instancing() {
        let _ = Path::new();
    }

    #[test]
    fn type_() {
        assert_eq!(Path::new().type_name(), "Path");
    }
}

mod shape {
    use super::*;

    #[test]
    fn extending() {
        let object = Shape::new();
        let _: &Path = &object;
    }

    #[test]
    fn instancing() {
        let _ = Shape::new();
    }

    #[test]
    fn type_() {
        assert_eq!(Shape::new().type_name(), "Shape");
    }
}

mod shape_path {
    use super::*;

    #[test]
    fn instancing() {
        let _ = ShapePath::new();
    }

    #[test]
    fn type_() {
        assert_eq!(ShapePath::new().type_name(), "ShapePath");
    }
}

mod shape_utils_checks {
    use super::*;

    fn square() -> Vec<Vector2> {
        vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ]
    }

    #[test]
    fn area_and_winding() {
        let ccw = square();
        assert_eq!(shape_utils::area(&ccw), 1.0);
        assert!(!shape_utils::is_clock_wise(&ccw));

        let mut cw = square();
        cw.reverse();
        assert_eq!(shape_utils::area(&cw), -1.0);
        assert!(shape_utils::is_clock_wise(&cw));

        assert_eq!(shape_utils::area(&[]), 0.0);
    }

    /// `triangulateShape` pops a closing point that repeats the first, in
    /// the caller's array, as three.js' does.
    #[test]
    fn triangulate_shape_drops_duplicate_end_points() {
        let mut contour = square();
        contour.push(Vector2::new(0.0, 0.0));
        let mut holes: Vec<Vec<Vector2>> = Vec::new();

        let faces = shape_utils::triangulate_shape(&mut contour, &mut holes);

        assert_eq!(contour, square());
        assert_eq!(faces.len(), 2);
        for face in &faces {
            assert!(face.iter().all(|&i| i < 4));
        }
    }
}
