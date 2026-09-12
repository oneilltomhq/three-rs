//! Port of `three.js/src/math/Frustum.js`.

use super::{Box3, CoordinateSystem, Matrix4, Plane, Sphere, Vector3};
use crate::objects::Mesh;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Default for Frustum {
    /// `new Frustum()` — six default planes.
    fn default() -> Self {
        Self {
            planes: [Plane::default(); 6],
        }
    }
}

impl Frustum {
    pub const fn new(p0: Plane, p1: Plane, p2: Plane, p3: Plane, p4: Plane, p5: Plane) -> Self {
        Self {
            planes: [p0, p1, p2, p3, p4, p5],
        }
    }

    /// `Frustum.set()`.
    pub fn set(
        &mut self,
        p0: &Plane,
        p1: &Plane,
        p2: &Plane,
        p3: &Plane,
        p4: &Plane,
        p5: &Plane,
    ) -> &mut Self {
        let planes = &mut self.planes;

        planes[0].copy(p0);
        planes[1].copy(p1);
        planes[2].copy(p2);
        planes[3].copy(p3);
        planes[4].copy(p4);
        planes[5].copy(p5);

        self
    }

    /// `Frustum.copy()`.
    pub fn copy(&mut self, frustum: &Self) -> &mut Self {
        for i in 0..6 {
            let other = frustum.planes[i];
            self.planes[i].copy(&other);
        }

        self
    }

    /// `Frustum.setFromProjectionMatrix()`. three.js defaults are
    /// `coordinateSystem = WebGLCoordinateSystem` and `reversedDepth = false`.
    pub fn set_from_projection_matrix(
        &mut self,
        m: &Matrix4,
        coordinate_system: CoordinateSystem,
        reversed_depth: bool,
    ) -> &mut Self {
        let planes = &mut self.planes;
        let me = &m.elements;
        let (me0, me1, me2, me3) = (me[0], me[1], me[2], me[3]);
        let (me4, me5, me6, me7) = (me[4], me[5], me[6], me[7]);
        let (me8, me9, me10, me11) = (me[8], me[9], me[10], me[11]);
        let (me12, me13, me14, me15) = (me[12], me[13], me[14], me[15]);

        planes[0]
            .set_components(me3 - me0, me7 - me4, me11 - me8, me15 - me12)
            .normalize();
        planes[1]
            .set_components(me3 + me0, me7 + me4, me11 + me8, me15 + me12)
            .normalize();
        planes[2]
            .set_components(me3 + me1, me7 + me5, me11 + me9, me15 + me13)
            .normalize();
        planes[3]
            .set_components(me3 - me1, me7 - me5, me11 - me9, me15 - me13)
            .normalize();

        if reversed_depth {
            planes[4].set_components(me2, me6, me10, me14).normalize(); // far
            planes[5]
                .set_components(me3 - me2, me7 - me6, me11 - me10, me15 - me14)
                .normalize(); // near
        } else {
            planes[4]
                .set_components(me3 - me2, me7 - me6, me11 - me10, me15 - me14)
                .normalize(); // far

            match coordinate_system {
                CoordinateSystem::WebGL => {
                    planes[5]
                        .set_components(me3 + me2, me7 + me6, me11 + me10, me15 + me14)
                        .normalize(); // near
                }
                CoordinateSystem::WebGPU => {
                    planes[5].set_components(me2, me6, me10, me14).normalize(); // near
                }
            }
        }

        self
    }

    /// `Frustum.intersectsObject()`.
    ///
    /// Diverges from three.js in the degenerate case: a geometry with no
    /// `position` attribute has no bounding sphere at all here, where three.js
    /// would leave `geometry.boundingSphere` `null` and throw. This returns
    /// `false` (nothing to intersect) instead.
    pub fn intersects_object(&self, object: &Mesh) -> bool {
        let Some(bounding_sphere) = object.geometry.compute_bounding_sphere() else {
            return false;
        };

        let mut sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(&object.object.matrix_world);

        self.intersects_sphere(&sphere)
    }

    /// `Frustum.intersectsSphere()`.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        let planes = &self.planes;
        let center = &sphere.center;
        let neg_radius = -sphere.radius;

        for i in 0..6 {
            let distance = planes[i].distance_to_point(center);

            if distance < neg_radius {
                return false;
            }
        }

        true
    }

    /// `Frustum.intersectsBox()`.
    pub fn intersects_box(&self, box3: &Box3) -> bool {
        let planes = &self.planes;

        let mut vector = Vector3::ZERO;

        for i in 0..6 {
            let plane = &planes[i];

            // corner at max distance

            vector.x = if plane.normal.x > 0.0 {
                box3.max.x
            } else {
                box3.min.x
            };
            vector.y = if plane.normal.y > 0.0 {
                box3.max.y
            } else {
                box3.min.y
            };
            vector.z = if plane.normal.z > 0.0 {
                box3.max.z
            } else {
                box3.min.z
            };

            if plane.distance_to_point(&vector) < 0.0 {
                return false;
            }
        }

        true
    }

    /// `Frustum.containsPoint()`.
    pub fn contains_point(&self, point: &Vector3) -> bool {
        let planes = &self.planes;

        for i in 0..6 {
            if planes[i].distance_to_point(point) < 0.0 {
                return false;
            }
        }

        true
    }
}
