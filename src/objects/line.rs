//! Ports of `three.js/src/objects/Line.js` and `three.js/src/objects/LineSegments.js`.
//!
//! `Line extends Object3D` — it is *not* a `Mesh`, and the renderer keeps them
//! apart: `WebGPUUtils.getPrimitiveTopology()` reads the object, not the
//! material, so the same `LineBasicNodeMaterial` draws a `line-strip` under a
//! `Line` and a `line-list` under a `LineSegments`.
//!
//! `LineLoop` is deliberately absent. `Renderer._projectObject()` errors on it
//! ("Objects of type THREE.LineLoop are not supported. Please use THREE.Line or
//! THREE.LineSegments."), so there is nothing to port.
//!
//! `computeLineDistances()` is not ported either: its only consumer is
//! `LineDashedMaterial` / `dashSize`, which no rung needs.

use std::rc::Rc;

use crate::core::{BufferGeometry, Intersection, Node, Object3D, Raycaster};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Matrix4, Ray, Sphere, Vector3};
use crate::objects::Payload;

/// The state `Line` adds to `Object3D`: `geometry` and `material`, exactly as
/// `Mesh` has them. It lives in a node's [`Payload`](crate::objects::Payload) —
/// see `docs/scene-graph.md`.
#[derive(Clone)]
pub struct Line {
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
    /// `LineSegments.isLineSegments`. The single bit that separates the two
    /// subclasses for everything this port does with them.
    pub is_line_segments: bool,
}

impl Line {
    /// `new Line( geometry, material )`.
    ///
    /// three.js defaults the material to `new LineBasicMaterial()`, which under
    /// `WebGPURenderer` is a `LineBasicNodeMaterial`; the port takes it by value
    /// because every call site on the ladder passes one.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Node {
        Self::node(geometry, material, false)
    }

    /// The geometry half of `Line.intersectsFrustum( frustum )`, i.e.
    /// `frustum.intersectsObject( line )`: the bounding sphere pushed through
    /// `matrixWorld`. Identical to `Mesh`', because `Frustum.intersectsObject()`
    /// only ever reads `geometry.boundingSphere`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<Sphere> {
        let bounding_sphere = self.geometry.compute_bounding_sphere()?;
        let mut sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(matrix_world);
        Some(sphere)
    }

    /// `Line.raycast( raycaster, intersects )` — every segment that passes
    /// within `raycaster.params.line.threshold` of the ray. `scale` is the
    /// node's local `scale`, which three.js divides the threshold by.
    ///
    /// `LineSegments` walks the vertices two at a time, `Line` one at a time.
    pub fn raycast(
        &self,
        matrix_world: &Matrix4,
        scale: &Vector3,
        object: &Node,
        raycaster: &Raycaster,
        intersects: &mut Vec<Intersection>,
    ) {
        let geometry = &self.geometry;
        let threshold = raycaster.params.line.threshold;
        let draw_range = geometry.draw_range;

        // Check the bounding sphere's distance to the ray.
        let Some(mut sphere) = self.bounding_sphere_in(matrix_world) else {
            return;
        };
        sphere.radius += threshold;
        if !raycaster.ray.intersects_sphere(&sphere) {
            return;
        }

        let mut inverse_matrix = *matrix_world;
        inverse_matrix.invert();
        let mut ray = raycaster.ray;
        ray.apply_matrix4(&inverse_matrix);

        let local_threshold = threshold / ((scale.x + scale.y + scale.z) / 3.0);
        let local_threshold_sq = local_threshold * local_threshold;

        let step = if self.is_line_segments { 2 } else { 1 };
        let Some(position) = geometry.position() else {
            return;
        };
        let end_of = |count: usize| match draw_range.count {
            Some(draw_count) => count.min(draw_range.start + draw_count),
            None => count,
        };

        let mut check = |a: usize, b: usize, i: usize| {
            let start = position.get_vector3(a);
            let end = position.get_vector3(b);
            if let Some(intersect) = check_intersection(
                matrix_world,
                object,
                raycaster,
                &ray,
                local_threshold_sq,
                &start,
                &end,
                i,
            ) {
                intersects.push(intersect);
            }
        };

        let start = draw_range.start;
        if let Some(index) = &geometry.index {
            let end = end_of(index.count());
            let mut i = start;
            while i + 1 < end {
                check(index.get_x(i), index.get_x(i + 1), i);
                i += step;
            }
        } else {
            let end = end_of(position.count());
            let mut i = start;
            while i + 1 < end {
                check(i, i + 1, i);
                i += step;
            }
        }
    }

    /// Rewrite the line's `position` attribute and mark it for re-upload —
    /// `positions` is the flat `[ x, y, z, x, y, z, ... ]` the attribute holds.
    ///
    /// The convenience form of `attribute.array_mut()` then
    /// [`set_needs_update`](crate::core::BufferAttribute::set_needs_update),
    /// which is what a consumer moving one wall of a diagram wants instead of
    /// rebuilding the scene (issue #47). The next render re-writes that one
    /// buffer; the geometry keeps its id and its other buffers.
    ///
    /// Panics if the geometry has no `position` attribute.
    pub fn set_positions(&self, positions: &[f32]) {
        let attribute = self
            .geometry
            .get_attribute("position")
            .expect("three-rs: the line geometry has a position attribute");

        let mut array = attribute.array_mut();
        array.clear();
        array.extend_from_slice(positions);
        drop(array);

        attribute.set_needs_update();
    }

    fn node(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        is_line_segments: bool,
    ) -> Node {
        let mut object = Object3D {
            object_type: if is_line_segments {
                "LineSegments"
            } else {
                "Line"
            },
            ..Default::default()
        };
        object.payload = Payload::Line(Self {
            geometry,
            material: Some(material),
            is_line_segments,
        });
        object.into_node()
    }
}

/// `class LineSegments extends Line` — a series of lines drawn between *pairs*
/// of vertices, i.e. `line-list` rather than `line-strip`. The only state it
/// adds is `isLineSegments`, so it is a constructor on [`Line`]'s payload
/// rather than a payload of its own.
pub struct LineSegments;

impl LineSegments {
    /// `new LineSegments( geometry, material )`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Node {
        Line::node(geometry, material, true)
    }
}

/// `Line.js`' `checkIntersection( object, raycaster, ray, thresholdSq, a, b, i )`,
/// with the two endpoints already read.
#[allow(clippy::too_many_arguments)]
fn check_intersection(
    matrix_world: &Matrix4,
    object: &Node,
    raycaster: &Raycaster,
    ray: &Ray,
    threshold_sq: f64,
    start: &Vector3,
    end: &Vector3,
    i: usize,
) -> Option<Intersection> {
    let mut point_on_ray = Vector3::ZERO;
    let mut point_on_segment = Vector3::ZERO;
    let dist_sq = ray.distance_sq_to_segment(
        start,
        end,
        Some(&mut point_on_ray),
        Some(&mut point_on_segment),
    );
    if dist_sq > threshold_sq {
        return None;
    }

    // Move back to world space for the distance calculation.
    point_on_ray.apply_matrix4(matrix_world);
    let distance = raycaster.ray.origin.distance_to(&point_on_ray);
    if distance < raycaster.near || distance > raycaster.far {
        return None;
    }

    // three.js reports the point on the *segment*, not on the ray.
    point_on_segment.apply_matrix4(matrix_world);
    let mut intersection = Intersection::new(distance, point_on_segment, object.clone());
    intersection.index = Some(i);
    Some(intersection)
}
