//! Port of `three.js/src/extras/core/Shape.js`.

use std::ops::{Deref, DerefMut};

use super::curve::Curve;
use super::curve_path::delegate_curve_path;
use super::path::Path;
use crate::math::Vector2;

/// `Shape`: a [`Path`] with optional holes, the input to
/// [`shape_geometry`](crate::geometries::shape_geometry) and
/// [`extrude_geometry`](crate::geometries::extrude_geometry).
///
/// `Shape extends Path` in three.js; here a `Shape` holds its `Path` and
/// derefs to it, so the drawing API (`shape.move_to(..)`) reads the same.
/// `uuid` is not ported: it only serves `toJSON`.
#[derive(Clone, Debug, Default)]
pub struct Shape {
    /// The inherited `Path` state.
    pub path: Path,
    /// The paths that define the holes in the shape.
    pub holes: Vec<Path>,
}

impl Deref for Shape {
    type Target = Path;
    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl DerefMut for Shape {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.path
    }
}

delegate_curve_path!(Shape, path, Vector2, "Shape");

/// The return value of [`Shape::extract_points`], three.js'
/// `{ shape, holes }`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShapePoints {
    pub shape: Vec<Vector2>,
    pub holes: Vec<Vec<Vector2>>,
}

impl Shape {
    /// `new Shape()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `new Shape( points )`.
    pub fn from_points(points: &[Vector2]) -> Self {
        Self {
            path: Path::from_points(points),
            holes: Vec::new(),
        }
    }

    /// `Shape.getPointsHoles( divisions )`.
    pub fn get_points_holes(&self, divisions: usize) -> Vec<Vec<Vector2>> {
        self.holes
            .iter()
            .map(|hole| hole.get_points(divisions))
            .collect()
    }

    /// `Shape.extractPoints( divisions )`: the points of the outline and of
    /// every hole.
    pub fn extract_points(&self, divisions: usize) -> ShapePoints {
        ShapePoints {
            shape: self.get_points(divisions),
            holes: self.get_points_holes(divisions),
        }
    }
}
