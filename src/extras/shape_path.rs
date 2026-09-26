//! Port of `three.js/src/extras/core/ShapePath.js`.

use super::curve::Curve;
use super::path::Path;
use super::shape::Shape;
use super::shape_utils;
use crate::math::{Box2, Color, Vector2};

/// The SVG fill rule [`ShapePath::to_shapes`] resolves nesting with; three.js
/// reads it from `userData.style.fillRule`, a string, and falls back to
/// `'nonzero'` (with a warning) for anything else. An enum has no "anything
/// else", so the fallback has nothing to do here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FillRule {
    /// `'nonzero'`, the default.
    #[default]
    NonZero,
    /// `'evenodd'`.
    EvenOdd,
}

/// `ShapePath`: a set of sub-paths, drawn with a canvas-like API, that
/// [`to_shapes`](Self::to_shapes) resolves into [`Shape`]s with holes.
#[derive(Clone, Debug, Default)]
pub struct ShapePath {
    /// The color of the shape.
    pub color: Color,
    /// The paths.
    pub sub_paths: Vec<Path>,
    /// Index into `sub_paths` of the path being drawn — three.js'
    /// `currentPath`, a reference to the last `moveTo`'s path.
    current_path: Option<usize>,
    /// `userData.style.fillRule`.
    pub fill_rule: FillRule,
}

impl ShapePath {
    /// `new ShapePath()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `ShapePath.type`.
    pub fn type_name(&self) -> &'static str {
        "ShapePath"
    }

    /// three.js' `currentPath`. Panics where three.js would throw on `null`: a
    /// drawing call before the first `moveTo`.
    fn current_path(&mut self) -> &mut Path {
        let index = self
            .current_path
            .expect("three-rs: ShapePath: a drawing call before moveTo (three.js throws on the null currentPath)");
        &mut self.sub_paths[index]
    }

    /// `ShapePath.moveTo( x, y )`: starts a new sub-path.
    pub fn move_to(&mut self, x: f64, y: f64) -> &mut Self {
        let mut path = Path::new();
        path.move_to(x, y);
        self.sub_paths.push(path);
        self.current_path = Some(self.sub_paths.len() - 1);

        self
    }

    /// `ShapePath.lineTo( x, y )`.
    pub fn line_to(&mut self, x: f64, y: f64) -> &mut Self {
        self.current_path().line_to(x, y);

        self
    }

    /// `ShapePath.quadraticCurveTo( aCPx, aCPy, aX, aY )`.
    pub fn quadratic_curve_to(
        &mut self,
        a_c_px: f64,
        a_c_py: f64,
        a_x: f64,
        a_y: f64,
    ) -> &mut Self {
        self.current_path()
            .quadratic_curve_to(a_c_px, a_c_py, a_x, a_y);

        self
    }

    /// `ShapePath.bezierCurveTo( aCP1x, aCP1y, aCP2x, aCP2y, aX, aY )`.
    pub fn bezier_curve_to(
        &mut self,
        a_cp1x: f64,
        a_cp1y: f64,
        a_cp2x: f64,
        a_cp2y: f64,
        a_x: f64,
        a_y: f64,
    ) -> &mut Self {
        self.current_path()
            .bezier_curve_to(a_cp1x, a_cp1y, a_cp2x, a_cp2y, a_x, a_y);

        self
    }

    /// `ShapePath.splineThru( pts )`.
    pub fn spline_thru(&mut self, pts: &[Vector2]) -> &mut Self {
        self.current_path().spline_thru(pts);

        self
    }

    /// `ShapePath.toShapes()`: classifies every sub-path as an outer shape, a
    /// hole in one, or a redundant overlap, by winding number under
    /// [`fill_rule`](Self::fill_rule), and builds the shapes.
    pub fn to_shapes(&self) -> Vec<Shape> {
        // Point-in-polygon test using the even-odd ray-casting rule. Valid for
        // simple (non self-intersecting) polygons.

        fn point_in_polygon(p: &Vector2, polygon: &[Vector2]) -> bool {
            let mut inside = false;
            let n = polygon.len();

            let mut j = n.wrapping_sub(1);
            for i in 0..n {
                let a = &polygon[i];
                let b = &polygon[j];

                if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x
                {
                    inside = !inside;
                }

                j = i;
            }

            inside
        }

        // Returns a point guaranteed to be strictly inside the given simple
        // polygon. First tries the bounding-box center; if that falls outside
        // the polygon, casts a horizontal ray at the center's y and picks the
        // midpoint between the first two sorted intercepts.
        //
        // Port of paper.js' Path#getInteriorPoint()
        // https://github.com/paperjs/paper.js/blob/develop/src/path/PathItem.Boolean.js

        fn get_interior_point(polygon: &[Vector2], bounding_box: &Box2) -> Vector2 {
            let mut point = bounding_box.get_center();

            if point_in_polygon(&point, polygon) {
                return point;
            }

            let y = point.y;
            let mut intercepts: Vec<f64> = Vec::new();
            let n = polygon.len();

            for i in 0..n {
                let a = &polygon[i];
                let b = &polygon[(i + 1) % n];

                // Half-open crossing rule — counts each vertex exactly once and
                // skips horizontal edges.
                if (a.y > y) != (b.y > y) {
                    let x = a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
                    intercepts.push(x);
                }
            }

            if intercepts.len() > 1 {
                intercepts.sort_by(|a, b| js_compare(a - b));
                point.x = (intercepts[0] + intercepts[1]) / 2.0;
            }

            point
        }

        // Predicate that decides whether a winding number falls inside the fill
        // region, per the SVG fill-rule spec. Works for negative windings too,
        // because two's complement AND preserves odd/even.

        let fill_rule = self.fill_rule;
        let is_inside = |w: i64| match fill_rule {
            FillRule::NonZero => w != 0,
            FillRule::EvenOdd => (w & 1) != 0,
        };

        // Build an entry per usable subpath. Self-winding follows the standard
        // convention used by ShapeUtils: counter-clockwise (signed area > 0)
        // contributes +1 to the winding number at an interior point,
        // clockwise contributes -1.

        #[derive(Clone, Copy, PartialEq)]
        enum Role {
            Outer,
            Hole,
        }

        struct Entry<'a> {
            sub_path: &'a Path,
            points: Vec<Vector2>,
            bounding_box: Box2,
            interior_point: Vector2,
            abs_area: f64,
            winding: i64,
            container: Option<usize>,
            exclude: bool,
            role: Option<Role>,
        }

        let mut entries: Vec<Entry> = Vec::new();

        for sub_path in &self.sub_paths {
            // `subPath.getPoints()`: `CurvePath.getPoints`' default of 12.
            let points = sub_path.get_points(12);
            if points.len() < 3 {
                continue;
            }

            let area = shape_utils::area(&points);
            if area == 0.0 {
                continue;
            }

            let mut bounding_box = Box2::default();
            for point in &points {
                bounding_box.expand_by_point(point);
            }

            let interior_point = get_interior_point(&points, &bounding_box);

            entries.push(Entry {
                sub_path,
                points,
                bounding_box,
                interior_point,
                abs_area: area.abs(),
                winding: if area < 0.0 { -1 } else { 1 },
                container: None,
                exclude: false,
                role: None,
            });
        }

        // Sort by area descending. This guarantees that any subpath that could
        // contain `entries[i]` is located at a smaller index and has already
        // been processed when it's entries[i]'s turn. Port of paper.js'
        // reorientPaths() algorithm. (`Array.prototype.sort` is stable, as
        // `sort_by` is.)

        entries.sort_by(|a, b| js_compare(b.abs_area - a.abs_area));

        // Walk already-processed entries from closest-in-size to largest,
        // stopping at the innermost container. Accumulate the container's
        // cumulative winding into this entry's winding so that the final value
        // equals the winding number at this entry's interior point.
        //
        // A subpath only contributes to the fill boundary when crossing it
        // actually flips the "insideness" per the fill rule; otherwise it's a
        // redundant overlap and gets excluded to avoid double-counting.

        for i in 0..entries.len() {
            let mut container_winding = 0;

            for j in (0..i).rev() {
                let candidate = &entries[j];

                if !candidate
                    .bounding_box
                    .contains_box(&entries[i].bounding_box)
                {
                    continue;
                }
                if !point_in_polygon(&entries[i].interior_point, &candidate.points) {
                    continue;
                }

                let container = if candidate.exclude {
                    candidate.container
                } else {
                    Some(j)
                };
                container_winding = candidate.winding;

                let entry = &mut entries[i];
                entry.container = container;
                entry.winding += container_winding;
                break;
            }

            if is_inside(entries[i].winding) == is_inside(container_winding) {
                entries[i].exclude = true;
            }
        }

        // Classify retained entries. An entry is an outer shape if it has no
        // container or if its container is itself a hole (a solid nested inside
        // a hole becomes a new top-level shape); otherwise it's a hole in its
        // container. Entries were already sorted outermost-first, so each
        // container's role is known by the time we look at it.

        for i in 0..entries.len() {
            if entries[i].exclude {
                continue;
            }

            let role = match entries[i].container {
                None => Role::Outer,
                Some(c) if entries[c].role == Some(Role::Hole) => Role::Outer,
                Some(_) => Role::Hole,
            };
            entries[i].role = Some(role);
        }

        // Build Shapes for outers first, then attach holes to their container's
        // Shape. (`shape.curves = entry.subPath.curves` shares the curves;
        // sharing the `Rc`s is the same thing.)

        let mut shapes: Vec<Shape> = Vec::new();
        let mut shape_by_entry: Vec<Option<usize>> = vec![None; entries.len()];

        for (i, entry) in entries.iter().enumerate() {
            if entry.exclude || entry.role != Some(Role::Outer) {
                continue;
            }

            let mut shape = Shape::new();
            shape.curves = entry.sub_path.curves.clone();
            shapes.push(shape);
            shape_by_entry[i] = Some(shapes.len() - 1);
        }

        for entry in &entries {
            if entry.exclude || entry.role != Some(Role::Hole) {
                continue;
            }

            let Some(shape) = entry.container.and_then(|c| shape_by_entry[c]) else {
                continue;
            };

            let mut hole = Path::new();
            hole.curves = entry.sub_path.curves.clone();
            shapes[shape].holes.push(hole);
        }

        shapes
    }
}

/// A JS comparator's return value as an `Ordering`: negative is less,
/// positive greater, and zero or `NaN` equal.
fn js_compare(result: f64) -> std::cmp::Ordering {
    if result < 0.0 {
        std::cmp::Ordering::Less
    } else if result > 0.0 {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Equal
    }
}
