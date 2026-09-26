//! Port of `three.js/src/extras/core/Curve.js`.
//!
//! All arithmetic is `f64`, matching JavaScript number semantics, and every
//! expression keeps three.js' own order and grouping so the rounding matches
//! bit for bit.
//!
//! three.js' `Curve` is an abstract base class whose subclasses override
//! `getPoint`; here it is a trait with `get_point` required and everything
//! else default-implemented on top of it. three.js' curves return a `Vector2`
//! or a `Vector3` depending on the subclass, and the base class branches on
//! `pt1.isVector2` where it has to build one; here that is the associated
//! [`Curve::Point`] type, bounded by [`CurveVector`].
//!
//! **Deliberate divergence: no arc length cache on the leaf curves.**
//! `Curve.getLengths()` memoises its result into `this.cacheArcLengths` and
//! invalidates it when `needsUpdate` is set. The methods here take `&self`, so
//! a leaf curve (a line, a Bézier, an ellipse, a spline) recomputes on every
//! call instead. The computation is pure, so the numbers are identical — only
//! the cost differs. [`CurvePath`](super::CurvePath) is the exception: its
//! `getPoint` is itself built on `getLength`, so recomputing there is
//! quadratic, and it keeps both of three.js' caches, with three.js'
//! invalidation rules (see its doc).

use crate::math::math_utils::clamp;
use crate::math::{Matrix4, Vector2, Vector3};

/// The return value of [`Curve::compute_frenet_frames`], three.js'
/// `{ tangents, normals, binormals }`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrenetFrames {
    pub tangents: Vec<Vector3>,
    pub normals: Vec<Vector3>,
    pub binormals: Vec<Vector3>,
}

/// What a curve's points are: `Vector2` for the 2D curves (`LineCurve`,
/// `EllipseCurve`, `Path`, `Shape`, ...), `Vector3` for the 3D ones. The
/// handful of vector operations the base class uses generically.
pub trait CurveVector: Copy + Default + PartialEq + std::fmt::Debug + 'static {
    /// `v.distanceTo( w )`.
    fn distance_to(&self, v: &Self) -> f64;
    /// `v.equals( w )`.
    fn equals(&self, v: &Self) -> bool;
    /// `new VectorN().copy( pt2 ).sub( pt1 ).normalize()` — `Curve.getTangent`'s
    /// body.
    fn tangent(pt1: &Self, pt2: &Self) -> Self;
    /// The point as a `Vector3`, `z = 0` for a `Vector2`. Only
    /// [`Curve::compute_frenet_frames`] uses it; three.js copies a `Vector2`
    /// tangent into a `Vector3` there, which leaves `z` `undefined` (`NaN`
    /// downstream), so frames of a 2D curve are meaningless in three.js and
    /// merely planar here.
    fn to_vector3(&self) -> Vector3;
}

impl CurveVector for Vector2 {
    fn distance_to(&self, v: &Self) -> f64 {
        Vector2::distance_to(self, v)
    }
    fn equals(&self, v: &Self) -> bool {
        Vector2::equals(self, v)
    }
    fn tangent(pt1: &Self, pt2: &Self) -> Self {
        let mut tangent = Vector2::default();
        tangent.copy(pt2).sub(pt1).normalize();
        tangent
    }
    fn to_vector3(&self) -> Vector3 {
        Vector3::new(self.x, self.y, 0.0)
    }
}

impl CurveVector for Vector3 {
    fn distance_to(&self, v: &Self) -> f64 {
        Vector3::distance_to(self, v)
    }
    fn equals(&self, v: &Self) -> bool {
        Vector3::equals(self, v)
    }
    fn tangent(pt1: &Self, pt2: &Self) -> Self {
        let mut tangent = Vector3::default();
        tangent.copy(pt2).sub(pt1).normalize();
        tangent
    }
    fn to_vector3(&self) -> Vector3 {
        *self
    }
}

/// `Curve.getLengths( divisions )`'s computation, without the cache, so that
/// an implementor that does cache ([`CurvePath`](super::CurvePath)) can
/// override [`Curve::get_lengths`] and still call it.
pub fn compute_lengths<C: Curve + ?Sized>(curve: &C, divisions: usize) -> Vec<f64> {
    let mut cache = Vec::new();
    let mut last = curve.get_point(0.0);
    let mut sum = 0.0;

    cache.push(0.0);

    for p in 1..=divisions {
        let current = curve.get_point(p as f64 / divisions as f64);
        sum += current.distance_to(&last);
        cache.push(sum);
        last = current;
    }

    cache
}

/// `Curve`. Implementors provide [`Curve::get_point`]; the rest is three.js'
/// own generic machinery built on it.
pub trait Curve {
    /// `Vector2` or `Vector3`.
    type Point: CurveVector;

    /// `Curve.getPoint( t, optionalTarget )`. `t` is an interpolation factor in
    /// `[0,1]`.
    fn get_point(&self, t: f64) -> Self::Point;

    /// `Curve.type`: `'LineCurve'`, `'EllipseCurve'`, `'Path'`, ...
    fn type_name(&self) -> &'static str;

    /// How many divisions `CurvePath.getPoints( divisions )` asks this curve
    /// for. three.js writes it as a chain of brand checks in `CurvePath`:
    /// `isEllipseCurve` doubles it, `isLineCurve` / `isLineCurve3` use 1 and
    /// `isSplineCurve` multiplies by its point count; every other curve uses
    /// `divisions` as given. Here each of those subclasses overrides this.
    fn curve_path_resolution(&self, divisions: usize) -> usize {
        divisions
    }

    /// `extrudePath.isCatmullRomCurve3 ? extrudePath.closed : false` — the one
    /// place three.js reads a subclass' `closed` generically
    /// (`ExtrudeGeometry`). Only [`CatmullRomCurve3`](super::CatmullRomCurve3)
    /// overrides it.
    fn is_closed_catmull_rom(&self) -> bool {
        false
    }

    /// `Curve.arcLengthDivisions`, default 200.
    fn arc_length_divisions(&self) -> usize {
        200
    }

    /// `Curve.getPointAt( u, optionalTarget )`.
    fn get_point_at(&self, u: f64) -> Self::Point {
        let t = self.get_u_to_t_mapping(u, None);
        self.get_point(t)
    }

    /// `Curve.getPoints( divisions )`; returns `divisions + 1` points.
    fn get_points(&self, divisions: usize) -> Vec<Self::Point> {
        let mut points = Vec::new();

        for d in 0..=divisions {
            points.push(self.get_point(d as f64 / divisions as f64));
        }

        points
    }

    /// `Curve.getSpacedPoints( divisions )`; returns `divisions + 1` points.
    fn get_spaced_points(&self, divisions: usize) -> Vec<Self::Point> {
        let mut points = Vec::new();

        for d in 0..=divisions {
            points.push(self.get_point_at(d as f64 / divisions as f64));
        }

        points
    }

    /// `Curve.getLength()`.
    fn get_length(&self) -> f64 {
        let lengths = self.get_lengths(self.arc_length_divisions());
        lengths[lengths.len() - 1]
    }

    /// `Curve.getLengths( divisions )`. See the module doc: no cache.
    fn get_lengths(&self, divisions: usize) -> Vec<f64> {
        compute_lengths(self, divisions)
    }

    /// `Curve.getUtoTmapping( u, distance )`, statement for statement.
    fn get_u_to_t_mapping(&self, u: f64, distance: Option<f64>) -> f64 {
        let arc_lengths = self.get_lengths(self.arc_length_divisions());

        let il = arc_lengths.len();

        // The targeted u distance value to get. three.js tests `if ( distance )`,
        // which is falsy for `0` as well as for `null`, so a distance of exactly
        // zero falls through to the `u`-scaled branch; the QUnit test
        // `getUtoTmapping( 0, 0 )` depends on that.
        let target_arc_length = match distance {
            Some(distance) if distance != 0.0 => distance,
            _ => u * arc_lengths[il - 1],
        };

        // binary search for the index with largest value smaller than target u distance

        // `low`/`high` are signed because `high` reaches -1 when the target is
        // below `arcLengths[ 0 ]`, exactly as the JS indices do. three.js hoists
        // `i` out of the loop, but `i = high` below overwrites it before it is
        // read again, so here it is loop-local.
        let (mut low, mut high): (isize, isize) = (0, il as isize - 1);

        while low <= high {
            // less likely to overflow, though probably not issue here, JS doesn't
            // really have integers, all numbers are floats. `high - low` is >= 0
            // inside the loop, so truncating division is `Math.floor` here.
            let i = low + (high - low) / 2;

            let comparison = arc_lengths[i as usize] - target_arc_length;

            if comparison < 0.0 {
                low = i + 1;
            } else if comparison > 0.0 {
                high = i - 1;
            } else {
                high = i;
                break;

                // DONE
            }
        }

        let i = high;

        assert!(
            i >= 0,
            "three-rs: Curve::get_u_to_t_mapping: target arc length {target_arc_length} lies before the start of the curve"
        );
        let i = i as usize;

        if arc_lengths[i] == target_arc_length {
            return i as f64 / (il - 1) as f64;
        }

        // we could get finer grain at lengths, or use simple interpolation between two points

        assert!(
            i + 1 < il,
            "three-rs: Curve::get_u_to_t_mapping: target arc length {target_arc_length} lies past the end of the curve"
        );

        let length_before = arc_lengths[i];
        let length_after = arc_lengths[i + 1];

        let segment_length = length_after - length_before;

        // determine where we are between the 'before' and 'after' points

        let segment_fraction = (target_arc_length - length_before) / segment_length;

        // add that fractional amount to t

        (i as f64 + segment_fraction) / (il - 1) as f64
    }

    /// `Curve.getTangent( t, optionalTarget )`.
    fn get_tangent(&self, t: f64) -> Self::Point {
        let delta = 0.0001;
        let mut t1 = t - delta;
        let mut t2 = t + delta;

        // Capping in case of danger

        if t1 < 0.0 {
            t1 = 0.0;
        }
        if t2 > 1.0 {
            t2 = 1.0;
        }

        let pt1 = self.get_point(t1);
        let pt2 = self.get_point(t2);

        Self::Point::tangent(&pt1, &pt2)
    }

    /// `Curve.getTangentAt( u, optionalTarget )`.
    fn get_tangent_at(&self, u: f64) -> Self::Point {
        let t = self.get_u_to_t_mapping(u, None);
        self.get_tangent(t)
    }

    /// `Curve.computeFrenetFrames( segments, closed )`.
    ///
    /// See <http://www.cs.indiana.edu/pub/techreports/TR425.pdf>.
    fn compute_frenet_frames(&self, segments: usize, closed: bool) -> FrenetFrames {
        let mut normal = Vector3::default();

        let mut tangents: Vec<Vector3> = Vec::new();
        let mut normals: Vec<Vector3> = Vec::new();
        let mut binormals: Vec<Vector3> = Vec::new();

        let mut vec = Vector3::default();
        let mut mat = Matrix4::default();

        // compute the tangent vectors for each segment on the curve

        for i in 0..=segments {
            let u = i as f64 / segments as f64;

            tangents.push(self.get_tangent_at(u).to_vector3());
        }

        // select an initial normal vector perpendicular to the first tangent vector,
        // and in the direction of the minimum tangent xyz component

        normals.push(Vector3::default());
        binormals.push(Vector3::default());
        let mut min = f64::MAX;
        let tx = tangents[0].x.abs();
        let ty = tangents[0].y.abs();
        let tz = tangents[0].z.abs();

        if tx <= min {
            min = tx;
            normal.set(1.0, 0.0, 0.0);
        }

        if ty <= min {
            min = ty;
            normal.set(0.0, 1.0, 0.0);
        }

        if tz <= min {
            normal.set(0.0, 0.0, 1.0);
        }

        vec.cross_vectors(&tangents[0], &normal).normalize();

        let n0 = *Vector3::default().cross_vectors(&tangents[0], &vec);
        normals[0] = n0;
        binormals[0] = *Vector3::default().cross_vectors(&tangents[0], &normals[0]);

        // compute the slowly-varying normal and binormal vectors for each segment on the curve

        for i in 1..=segments {
            normals.push(normals[i - 1]);

            binormals.push(binormals[i - 1]);

            vec.cross_vectors(&tangents[i - 1], &tangents[i]);

            if vec.length() > f64::EPSILON {
                vec.normalize();

                // clamp for floating pt errors
                let theta = clamp(tangents[i - 1].dot(&tangents[i]), -1.0, 1.0).acos();

                normals[i].apply_matrix4(mat.make_rotation_axis(&vec, theta));
            }

            binormals[i] = *Vector3::default().cross_vectors(&tangents[i], &normals[i]);
        }

        // if the curve is closed, postprocess the vectors so the first and last normal vectors are the same

        if closed {
            let mut theta = clamp(normals[0].dot(&normals[segments]), -1.0, 1.0).acos();
            theta /= segments as f64;

            if tangents[0].dot(vec.cross_vectors(&normals[0], &normals[segments])) > 0.0 {
                theta = -theta;
            }

            for i in 1..=segments {
                // twist a little...
                normals[i].apply_matrix4(mat.make_rotation_axis(&tangents[i], theta * i as f64));
                binormals[i] = *Vector3::default().cross_vectors(&tangents[i], &normals[i]);
            }
        }

        FrenetFrames {
            tangents,
            normals,
            binormals,
        }
    }
}
