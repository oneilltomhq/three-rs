//! Port of `three.js/src/extras/core/CurvePath.js`.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

use super::curve::{compute_lengths, Curve, CurveVector};
use super::line_curve::{LineCurve, LineVector};

/// A shared, immutable sub-curve of a [`CurvePath`].
///
/// three.js holds its curves by reference too, and `CurvePath.clone()`
/// deep-copies them; since a curve here is immutable once it is in a path,
/// sharing it between a path and its clone is the same thing.
pub type CurveRef<P> = Rc<dyn Curve<Point = P>>;

/// `CurvePath`: a sequence of connected curves, itself a [`Curve`].
///
/// # The two caches
///
/// `CurvePath.getPoint( t )` is built on `getLength()`, which sums every
/// sub-curve's length, and the inherited `getPointAt( u )` maps `u` through
/// `getLengths()`, which calls `getPoint` 201 times. Uncached that is
/// quadratic, so unlike the leaf curves (see [`super::curve`]) this keeps
/// both of three.js' caches, with three.js' own invalidation rules:
///
/// * `cacheLengths` (the per-curve running sums, `getCurveLengths()`) is
///   reused while it has one entry per curve, so adding a curve refreshes it;
/// * `cacheArcLengths` (`Curve.getLengths()`) is reused while it has
///   `divisions + 1` entries and `needsUpdate` is unset. Adding a curve does
///   **not** refresh it — in three.js either — so `get_point_at` after a later
///   `line_to` maps through the old path's arc lengths until
///   [`update_arc_lengths`](CurvePath::update_arc_lengths) is called. That is
///   reproduced on purpose: it is what three.js computes.
pub struct CurvePath<P: CurveVector> {
    /// The curves, in order.
    pub curves: Vec<CurveRef<P>>,
    /// Whether the path should automatically be closed by a line curve.
    pub auto_close: bool,
    /// `Curve.arcLengthDivisions`, default 200.
    pub arc_length_divisions: usize,
    /// `this.needsUpdate`.
    pub needs_update: Cell<bool>,
    cache_lengths: RefCell<Option<Vec<f64>>>,
    cache_arc_lengths: RefCell<Option<Vec<f64>>>,
}

impl<P: CurveVector> Default for CurvePath<P> {
    fn default() -> Self {
        Self {
            curves: Vec::new(),
            auto_close: false,
            arc_length_divisions: 200,
            needs_update: Cell::new(false),
            cache_lengths: RefCell::new(None),
            cache_arc_lengths: RefCell::new(None),
        }
    }
}

impl<P: CurveVector> Clone for CurvePath<P> {
    /// `CurvePath.clone()`: the curves, `autoClose` and `arcLengthDivisions`;
    /// the caches start empty, as a fresh three.js instance's do.
    fn clone(&self) -> Self {
        Self {
            curves: self.curves.clone(),
            auto_close: self.auto_close,
            arc_length_divisions: self.arc_length_divisions,
            ..Self::default()
        }
    }
}

impl<P: CurveVector> fmt::Debug for CurvePath<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CurvePath")
            .field(
                "curves",
                &self
                    .curves
                    .iter()
                    .map(|c| c.type_name())
                    .collect::<Vec<_>>(),
            )
            .field("auto_close", &self.auto_close)
            .finish()
    }
}

impl<P: CurveVector> CurvePath<P> {
    /// `new CurvePath()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `CurvePath.add( curve )`.
    pub fn add(&mut self, curve: impl Curve<Point = P> + 'static) {
        self.curves.push(Rc::new(curve));
    }

    /// `CurvePath.updateArcLengths()`.
    pub fn update_arc_lengths(&self) {
        self.needs_update.set(true);
        *self.cache_lengths.borrow_mut() = None;
        self.get_curve_lengths();
    }

    /// `CurvePath.getCurveLengths()`: the running sum of the sub-curves'
    /// lengths, cached while the curve count is unchanged.
    pub fn get_curve_lengths(&self) -> Vec<f64> {
        // Compute lengths and cache them
        // We cannot overwrite getLengths() because UtoT mapping uses it.
        // We use cache values if curves and cache array are same length

        if let Some(cache) = self.cache_lengths.borrow().as_ref() {
            if cache.len() == self.curves.len() {
                return cache.clone();
            }
        }

        // Get length of sub-curve
        // Push sums into cached array

        let mut lengths = Vec::new();
        let mut sums = 0.0;

        for curve in &self.curves {
            sums += curve.get_length();
            lengths.push(sums);
        }

        *self.cache_lengths.borrow_mut() = Some(lengths.clone());

        lengths
    }
}

impl<P: LineVector> CurvePath<P> {
    /// `CurvePath.closePath()`: adds a line curve back to the start if the
    /// path does not already end there.
    pub fn close_path(&mut self) -> &mut Self {
        // Add a line curve if start and end of lines are not connected
        let start_point = self.curves[0].get_point(0.0);
        let end_point = self.curves[self.curves.len() - 1].get_point(1.0);

        if !start_point.equals(&end_point) {
            self.curves
                .push(Rc::new(LineCurve::new(end_point, start_point)));
        }

        self
    }
}

impl<P: CurveVector> Curve for CurvePath<P> {
    type Point = P;

    fn type_name(&self) -> &'static str {
        "CurvePath"
    }

    fn arc_length_divisions(&self) -> usize {
        self.arc_length_divisions
    }

    /// `CurvePath.getPoint( t )`.
    ///
    /// # Panics
    ///
    /// Where three.js returns `null`: when `t * getLength()` lies past the last
    /// curve (`t > 1`, or an empty path).
    fn get_point(&self, t: f64) -> P {
        // To get accurate point with reference to
        // entire path distance at time t,
        // following has to be done:

        // 1. Length of each sub path have to be known
        // 2. Locate and identify type of curve
        // 3. Get t for the curve
        // 4. Return curve.getPointAt(t')

        let d = t * self.get_length();
        let curve_lengths = self.get_curve_lengths();

        // To think about boundaries points.

        for (i, &curve_length) in curve_lengths.iter().enumerate() {
            if curve_length >= d {
                let diff = curve_length - d;
                let curve = &self.curves[i];

                let segment_length = curve.get_length();
                let u = if segment_length == 0.0 {
                    0.0
                } else {
                    1.0 - diff / segment_length
                };

                return curve.get_point_at(u);
            }
        }

        panic!("three-rs: CurvePath::get_point({t}) lies past the end of the path (three.js returns null)");

        // loop where sum != 0, sum > d , sum+1 <d
    }

    /// `CurvePath.getLength()`.
    ///
    /// We cannot use the default THREE.Curve getPoint() with getLength()
    /// because in THREE.Curve, getLength() depends on getPoint() but in
    /// THREE.CurvePath getPoint() depends on getLength
    fn get_length(&self) -> f64 {
        let lens = self.get_curve_lengths();
        // `lens[ lens.length - 1 ]` is `undefined` (NaN downstream) for an
        // empty path.
        lens.last().copied().unwrap_or(f64::NAN)
    }

    /// `Curve.getLengths( divisions )` with its `cacheArcLengths` cache; see
    /// the type's doc.
    fn get_lengths(&self, divisions: usize) -> Vec<f64> {
        if let Some(cache) = self.cache_arc_lengths.borrow().as_ref() {
            if cache.len() == divisions + 1 && !self.needs_update.get() {
                return cache.clone();
            }
        }

        self.needs_update.set(false);

        let cache = compute_lengths(self, divisions);

        *self.cache_arc_lengths.borrow_mut() = Some(cache.clone());

        cache
    }

    /// `CurvePath.getSpacedPoints( divisions )`: `getPoint`, not `getPointAt`,
    /// at even `t`, plus the first point again when `autoClose` is set.
    fn get_spaced_points(&self, divisions: usize) -> Vec<P> {
        let mut points = Vec::new();

        for i in 0..=divisions {
            points.push(self.get_point(i as f64 / divisions as f64));
        }

        if self.auto_close {
            points.push(points[0]);
        }

        points
    }

    /// `CurvePath.getPoints( divisions )`: each curve sampled at its own
    /// resolution ([`Curve::curve_path_resolution`]), consecutive duplicates
    /// dropped.
    fn get_points(&self, divisions: usize) -> Vec<P> {
        let mut points: Vec<P> = Vec::new();
        let mut last: Option<P> = None;

        for curve in &self.curves {
            let resolution = curve.curve_path_resolution(divisions);

            let pts = curve.get_points(resolution);

            for point in pts {
                if last.is_some_and(|last| last.equals(&point)) {
                    continue; // ensures no consecutive points are duplicates
                }

                points.push(point);
                last = Some(point);
            }
        }

        if self.auto_close && points.len() > 1 && !points[points.len() - 1].equals(&points[0]) {
            points.push(points[0]);
        }

        points
    }
}

/// Implements [`Curve`] for a type that wraps a [`CurvePath`] (three.js'
/// `Path` and `Shape` subclasses), forwarding every method `CurvePath`
/// overrides and changing only `type`.
macro_rules! delegate_curve_path {
    ($ty:ty, $field:ident, $point:ty, $name:literal) => {
        impl $crate::extras::Curve for $ty {
            type Point = $point;

            fn type_name(&self) -> &'static str {
                $name
            }

            fn arc_length_divisions(&self) -> usize {
                $crate::extras::Curve::arc_length_divisions(&self.$field)
            }

            fn get_point(&self, t: f64) -> $point {
                $crate::extras::Curve::get_point(&self.$field, t)
            }

            fn get_length(&self) -> f64 {
                $crate::extras::Curve::get_length(&self.$field)
            }

            fn get_lengths(&self, divisions: usize) -> Vec<f64> {
                $crate::extras::Curve::get_lengths(&self.$field, divisions)
            }

            fn get_spaced_points(&self, divisions: usize) -> Vec<$point> {
                $crate::extras::Curve::get_spaced_points(&self.$field, divisions)
            }

            fn get_points(&self, divisions: usize) -> Vec<$point> {
                $crate::extras::Curve::get_points(&self.$field, divisions)
            }
        }
    };
}

pub(crate) use delegate_curve_path;
