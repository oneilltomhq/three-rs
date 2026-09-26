//! Port of `three.js/src/objects/Points.js`.
//!
//! `Points extends Object3D` — like `Line`, it is *not* a `Mesh`, and the
//! renderer keeps them apart because
//! `WebGPUUtils.getPrimitiveTopology( object, material )` reads the object:
//! `object.isPoints` is the first arm, and it is what turns the pipeline's
//! topology into `point-list`. A WebGPU point primitive is always one pixel,
//! which is why `webgpu_compute_points` draws 300 000 of them rather than
//! 300 000 sprites.

use std::rc::Rc;

use crate::core::{BufferGeometry, Intersection, Node, Object3D, Raycaster};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Matrix4, Sphere, Vector3};
use crate::objects::Payload;

/// The state `Points` adds to `Object3D`: `geometry` and `material`, plus the
/// non-standard `count` the compute examples set.
#[derive(Clone)]
pub struct Points {
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
    /// `mesh.count`, which is **not** a `Points` property in three.js. The
    /// renderer picks it up anyway: `RenderObject.getInstanceCount()`
    /// (`src/renderers/common/RenderObject.js:623-627`) is
    /// `instanced geometry ? … : object.count !== undefined ? max( 0, object.count ) : 1`,
    /// so `webgpu_compute_points`' `mesh.count = 300000` on a one-vertex
    /// geometry becomes `draw( 1, 300000, 0, 0 )`. Reproduced deliberately —
    /// see `docs/rung12-progress.md`.
    pub count: Option<usize>,
}

impl Points {
    /// `new Points( geometry, material )`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Node {
        let mut object = Object3D {
            object_type: "Points",
            ..Default::default()
        };
        object.payload = Payload::Points(Self {
            geometry,
            material: Some(material),
            count: None,
        });
        object.into_node()
    }

    /// The geometry half of `frustum.intersectsObject( points )` — identical to
    /// `Mesh`', because `Frustum.intersectsObject()` only reads
    /// `geometry.boundingSphere`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<Sphere> {
        let bounding_sphere = self.geometry.compute_bounding_sphere()?;
        let mut sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(matrix_world);
        Some(sphere)
    }

    /// `Points.raycast( raycaster, intersects )` — every point within
    /// `raycaster.params.points.threshold` of the ray. `scale` is the node's
    /// local `scale`, which three.js divides the threshold by.
    pub fn raycast(
        &self,
        matrix_world: &Matrix4,
        scale: &Vector3,
        object: &Node,
        raycaster: &Raycaster,
        intersects: &mut Vec<Intersection>,
    ) {
        let geometry = &self.geometry;
        let threshold = raycaster.params.points.threshold;
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

        let Some(position) = geometry.position() else {
            return;
        };
        let end_of = |count: usize| match draw_range.count {
            Some(draw_count) => count.min(draw_range.start + draw_count),
            None => count,
        };

        // `testPoint( point, index, localThresholdSq, matrixWorld, raycaster,
        // intersects, object )`.
        let mut test_point = |index: usize| {
            let point = position.get_vector3(index);
            let ray_point_distance_sq = ray.distance_sq_to_point(&point);
            if ray_point_distance_sq < local_threshold_sq {
                let mut intersect_point = ray.closest_point_to_point(&point);
                intersect_point.apply_matrix4(matrix_world);
                let distance = raycaster.ray.origin.distance_to(&intersect_point);
                if distance < raycaster.near || distance > raycaster.far {
                    return;
                }
                let mut intersection = Intersection::new(distance, intersect_point, object.clone());
                intersection.distance_to_ray = Some(ray_point_distance_sq.sqrt());
                intersection.index = Some(index);
                intersects.push(intersection);
            }
        };

        if let Some(index) = &geometry.index {
            for i in draw_range.start..end_of(index.count()) {
                test_point(index.get_x(i));
            }
        } else {
            for i in draw_range.start..end_of(position.count()) {
                test_point(i);
            }
        }
    }
}
