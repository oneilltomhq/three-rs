//! Port of `three.js/src/math/Ray.js`.

use super::{Box3, Matrix4, Plane, Sphere, Vector3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vector3,
    pub direction: Vector3,
}

impl Default for Ray {
    /// `new Ray()` — origin `(0,0,0)`, direction `(0,0,-1)`.
    fn default() -> Self {
        Self {
            origin: Vector3::new(0.0, 0.0, 0.0),
            direction: Vector3::new(0.0, 0.0, -1.0),
        }
    }
}

impl Ray {
    pub const fn new(origin: Vector3, direction: Vector3) -> Self {
        Self { origin, direction }
    }

    /// `Ray.set()`.
    pub fn set(&mut self, origin: &Vector3, direction: &Vector3) -> &mut Self {
        self.origin.copy(origin);
        self.direction.copy(direction);
        self
    }

    /// `Ray.copy()`.
    pub fn copy(&mut self, ray: &Self) -> &mut Self {
        self.origin.copy(&ray.origin);
        self.direction.copy(&ray.direction);
        self
    }

    /// `Ray.at()`.
    pub fn at(&self, t: f64) -> Vector3 {
        let mut target = Vector3::default();
        target.copy(&self.origin).add_scaled_vector(&self.direction, t);
        target
    }

    /// `Ray.lookAt()`.
    pub fn look_at(&mut self, v: &Vector3) -> &mut Self {
        let origin = self.origin;
        self.direction.copy(v).sub(&origin).normalize();
        self
    }

    /// `Ray.recast()`.
    pub fn recast(&mut self, t: f64) -> &mut Self {
        let vector = self.at(t);
        self.origin.copy(&vector);
        self
    }

    /// `Ray.closestPointToPoint()`.
    pub fn closest_point_to_point(&self, point: &Vector3) -> Vector3 {
        let mut target = Vector3::default();
        target.sub_vectors(point, &self.origin);

        let direction_distance = target.dot(&self.direction);

        if direction_distance < 0.0 {
            target.copy(&self.origin);
            return target;
        }

        target
            .copy(&self.origin)
            .add_scaled_vector(&self.direction, direction_distance);
        target
    }

    /// `Ray.distanceToPoint()`.
    pub fn distance_to_point(&self, point: &Vector3) -> f64 {
        self.distance_sq_to_point(point).sqrt()
    }

    /// `Ray.distanceSqToPoint()`.
    pub fn distance_sq_to_point(&self, point: &Vector3) -> f64 {
        let mut vector = Vector3::default();
        let direction_distance = vector.sub_vectors(point, &self.origin).dot(&self.direction);

        // point behind the ray

        if direction_distance < 0.0 {
            return self.origin.distance_to_squared(point);
        }

        vector
            .copy(&self.origin)
            .add_scaled_vector(&self.direction, direction_distance);

        vector.distance_to_squared(point)
    }

    /// `Ray.distanceSqToSegment()`.
    ///
    /// `optional_point_on_ray` / `optional_point_on_segment` are three.js'
    /// out-params: when given they receive the closest point on the ray and on
    /// the segment respectively.
    pub fn distance_sq_to_segment(
        &self,
        v0: &Vector3,
        v1: &Vector3,
        optional_point_on_ray: Option<&mut Vector3>,
        optional_point_on_segment: Option<&mut Vector3>,
    ) -> f64 {
        // from https://github.com/pmjoniak/GeometricTools/blob/master/GTEngine/Include/Mathematics/GteDistRaySegment.h
        // It returns the min distance between the ray and the segment
        // defined by v0 and v1
        // It can also set two optional targets :
        // - The closest point on the ray
        // - The closest point on the segment

        let mut seg_center = Vector3::default();
        let mut seg_dir = Vector3::default();
        let mut diff = Vector3::default();

        seg_center.copy(v0).add(v1).multiply_scalar(0.5);
        seg_dir.copy(v1).sub(v0).normalize();
        diff.copy(&self.origin).sub(&seg_center);

        let seg_extent = v0.distance_to(v1) * 0.5;
        let a01 = -self.direction.dot(&seg_dir);
        let b0 = diff.dot(&self.direction);
        let b1 = -diff.dot(&seg_dir);
        let c = diff.length_sq();
        let det = (1.0 - a01 * a01).abs();
        let mut s0;
        let mut s1;
        let sqr_dist;

        if det > 0.0 {
            // The ray and segment are not parallel.

            s0 = a01 * b1 - b0;
            s1 = a01 * b0 - b1;
            let ext_det = seg_extent * det;

            if s0 >= 0.0 {
                if s1 >= -ext_det {
                    if s1 <= ext_det {
                        // region 0
                        // Minimum at interior points of ray and segment.

                        let inv_det = 1.0 / det;
                        s0 *= inv_det;
                        s1 *= inv_det;
                        sqr_dist = s0 * (s0 + a01 * s1 + 2.0 * b0)
                            + s1 * (a01 * s0 + s1 + 2.0 * b1)
                            + c;
                    } else {
                        // region 1

                        s1 = seg_extent;
                        s0 = (-(a01 * s1 + b0)).max(0.0);
                        sqr_dist = -s0 * s0 + s1 * (s1 + 2.0 * b1) + c;
                    }
                } else {
                    // region 5

                    s1 = -seg_extent;
                    s0 = (-(a01 * s1 + b0)).max(0.0);
                    sqr_dist = -s0 * s0 + s1 * (s1 + 2.0 * b1) + c;
                }
            } else if s1 <= -ext_det {
                // region 4

                s0 = (-(-a01 * seg_extent + b0)).max(0.0);
                s1 = if s0 > 0.0 {
                    -seg_extent
                } else {
                    (-b1).max(-seg_extent).min(seg_extent)
                };
                sqr_dist = -s0 * s0 + s1 * (s1 + 2.0 * b1) + c;
            } else if s1 <= ext_det {
                // region 3

                s0 = 0.0;
                s1 = (-b1).max(-seg_extent).min(seg_extent);
                sqr_dist = s1 * (s1 + 2.0 * b1) + c;
            } else {
                // region 2

                s0 = (-(a01 * seg_extent + b0)).max(0.0);
                s1 = if s0 > 0.0 {
                    seg_extent
                } else {
                    (-b1).max(-seg_extent).min(seg_extent)
                };
                sqr_dist = -s0 * s0 + s1 * (s1 + 2.0 * b1) + c;
            }
        } else {
            // Ray and segment are parallel.

            s1 = if a01 > 0.0 { -seg_extent } else { seg_extent };
            s0 = (-(a01 * s1 + b0)).max(0.0);
            sqr_dist = -s0 * s0 + s1 * (s1 + 2.0 * b1) + c;
        }

        if let Some(point_on_ray) = optional_point_on_ray {
            point_on_ray
                .copy(&self.origin)
                .add_scaled_vector(&self.direction, s0);
        }

        if let Some(point_on_segment) = optional_point_on_segment {
            point_on_segment
                .copy(&seg_center)
                .add_scaled_vector(&seg_dir, s1);
        }

        sqr_dist
    }

    /// `Ray.intersectSphere()`.
    pub fn intersect_sphere(&self, sphere: &Sphere) -> Option<Vector3> {
        if sphere.radius < 0.0 {
            return None; // handle empty spheres, see #31187
        }

        let mut vector = Vector3::default();
        vector.sub_vectors(&sphere.center, &self.origin);
        let tca = vector.dot(&self.direction);
        let d2 = vector.dot(&vector) - tca * tca;
        let radius2 = sphere.radius * sphere.radius;

        if d2 > radius2 {
            return None;
        }

        let thc = (radius2 - d2).sqrt();

        // t0 = first intersect point - entrance on front of sphere
        let t0 = tca - thc;

        // t1 = second intersect point - exit point on back of sphere
        let t1 = tca + thc;

        // test to see if t1 is behind the ray - if so, return null
        if t1 < 0.0 {
            return None;
        }

        // test to see if t0 is behind the ray:
        // if it is, the ray is inside the sphere, so return the second exit point scaled by t1,
        // in order to always return an intersect point that is in front of the ray.
        if t0 < 0.0 {
            return Some(self.at(t1));
        }

        // else t0 is in front of the ray, so return the first collision point scaled by t0
        Some(self.at(t0))
    }

    /// `Ray.intersectsSphere()`.
    pub fn intersects_sphere(&self, sphere: &Sphere) -> bool {
        if sphere.radius < 0.0 {
            return false; // handle empty spheres, see #31187
        }

        self.distance_sq_to_point(&sphere.center) <= (sphere.radius * sphere.radius)
    }

    /// `Ray.distanceToPlane()`.
    pub fn distance_to_plane(&self, plane: &Plane) -> Option<f64> {
        let denominator = plane.normal.dot(&self.direction);

        if denominator == 0.0 {
            // line is coplanar, return origin
            if plane.distance_to_point(&self.origin) == 0.0 {
                return Some(0.0);
            }

            // Null is preferable to undefined since undefined means.... it is undefined

            return None;
        }

        let t = -(self.origin.dot(&plane.normal) + plane.constant) / denominator;

        // Return if the ray never intersects the plane

        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }

    /// `Ray.intersectPlane()`.
    pub fn intersect_plane(&self, plane: &Plane) -> Option<Vector3> {
        let t = self.distance_to_plane(plane)?;

        Some(self.at(t))
    }

    /// `Ray.intersectsPlane()`.
    pub fn intersects_plane(&self, plane: &Plane) -> bool {
        // check if the ray lies on the plane first

        let dist_to_point = plane.distance_to_point(&self.origin);

        if dist_to_point == 0.0 {
            return true;
        }

        let denominator = plane.normal.dot(&self.direction);

        if denominator * dist_to_point < 0.0 {
            return true;
        }

        // ray origin is behind the plane (and is pointing behind it)

        false
    }

    /// `Ray.intersectBox()`.
    pub fn intersect_box(&self, box3: &Box3) -> Option<Vector3> {
        let mut tmin;
        let mut tmax;
        let tymin;
        let tymax;
        let tzmin;
        let tzmax;

        let invdirx = 1.0 / self.direction.x;
        let invdiry = 1.0 / self.direction.y;
        let invdirz = 1.0 / self.direction.z;

        let origin = &self.origin;

        if invdirx >= 0.0 {
            tmin = (box3.min.x - origin.x) * invdirx;
            tmax = (box3.max.x - origin.x) * invdirx;
        } else {
            tmin = (box3.max.x - origin.x) * invdirx;
            tmax = (box3.min.x - origin.x) * invdirx;
        }

        if invdiry >= 0.0 {
            tymin = (box3.min.y - origin.y) * invdiry;
            tymax = (box3.max.y - origin.y) * invdiry;
        } else {
            tymin = (box3.max.y - origin.y) * invdiry;
            tymax = (box3.min.y - origin.y) * invdiry;
        }

        if (tmin > tymax) || (tymin > tmax) {
            return None;
        }

        if tymin > tmin || tmin.is_nan() {
            tmin = tymin;
        }

        if tymax < tmax || tmax.is_nan() {
            tmax = tymax;
        }

        if invdirz >= 0.0 {
            tzmin = (box3.min.z - origin.z) * invdirz;
            tzmax = (box3.max.z - origin.z) * invdirz;
        } else {
            tzmin = (box3.max.z - origin.z) * invdirz;
            tzmax = (box3.min.z - origin.z) * invdirz;
        }

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }

        if tzmin > tmin || tmin.is_nan() {
            tmin = tzmin;
        }

        if tzmax < tmax || tmax.is_nan() {
            tmax = tzmax;
        }

        //return point closest to the ray (positive side)

        if tmax < 0.0 {
            return None;
        }

        Some(self.at(if tmin >= 0.0 { tmin } else { tmax }))
    }

    /// `Ray.intersectsBox()`.
    pub fn intersects_box(&self, box3: &Box3) -> bool {
        self.intersect_box(box3).is_some()
    }

    /// `Ray.intersectTriangle()`.
    pub fn intersect_triangle(
        &self,
        a: &Vector3,
        b: &Vector3,
        c: &Vector3,
        backface_culling: bool,
    ) -> Option<Vector3> {
        // Watertight ray/triangle intersection. Reference: Woop, Benthin, Wald,
        // "Watertight Ray/Triangle Intersection", JCGT vol. 2 no. 1 (2013), Appendix A.
        // https://jcgt.org/published/0002/01/05/

        let origin = &self.origin;
        let direction = &self.direction;

        let dx = direction.x;
        let dy = direction.y;
        let dz = direction.z;

        // triangle vertices relative to the ray origin

        let (aox, aoy, aoz) = (a.x - origin.x, a.y - origin.y, a.z - origin.z);
        // `box` is a reserved word in Rust; three.js' `box`/`boy`/`boz` become `box_`/`boy`/`boz`.
        let (box_, boy, boz) = (b.x - origin.x, b.y - origin.y, b.z - origin.z);
        let (cox, coy, coz) = (c.x - origin.x, c.y - origin.y, c.z - origin.z);

        // Use the dimension where the ray direction is maximal as the projection
        // axis (kz) and read every component already permuted into (kx, ky, kz).
        // kx and ky are swapped when the direction's kz component is negative, to
        // preserve the winding order of triangles.

        let (adx, ady, adz) = (dx.abs(), dy.abs(), dz.abs());

        let dkx;
        let dky;
        let dkz;
        let akx;
        let aky;
        let akz;
        let bkx;
        let bky;
        let bkz;
        let ckx;
        let cky;
        let ckz;

        if adx >= ady && adx >= adz {
            dkz = dx;
            akz = aox;
            bkz = box_;
            ckz = cox;

            if dx >= 0.0 {
                dkx = dy;
                dky = dz;
                akx = aoy;
                aky = aoz;
                bkx = boy;
                bky = boz;
                ckx = coy;
                cky = coz;
            } else {
                dkx = dz;
                dky = dy;
                akx = aoz;
                aky = aoy;
                bkx = boz;
                bky = boy;
                ckx = coz;
                cky = coy;
            }
        } else if ady >= adz {
            dkz = dy;
            akz = aoy;
            bkz = boy;
            ckz = coy;

            if dy >= 0.0 {
                dkx = dz;
                dky = dx;
                akx = aoz;
                aky = aox;
                bkx = boz;
                bky = box_;
                ckx = coz;
                cky = cox;
            } else {
                dkx = dx;
                dky = dz;
                akx = aox;
                aky = aoz;
                bkx = box_;
                bky = boz;
                ckx = cox;
                cky = coz;
            }
        } else {
            dkz = dz;
            akz = aoz;
            bkz = boz;
            ckz = coz;

            if dz >= 0.0 {
                dkx = dx;
                dky = dy;
                akx = aox;
                aky = aoy;
                bkx = box_;
                bky = boy;
                ckx = cox;
                cky = coy;
            } else {
                dkx = dy;
                dky = dx;
                akx = aoy;
                aky = aox;
                bkx = boy;
                bky = box_;
                ckx = coy;
                cky = cox;
            }
        }

        // a zero direction has no maximal axis and cannot intersect

        if dkz == 0.0 {
            return None;
        }

        // shear constants that align the ray with the +kz axis

        let sx = dkx / dkz;
        let sy = dky / dkz;
        let sz = 1.0 / dkz;

        // sheared and scaled vertices

        let (ax, ay) = (akx - sx * akz, aky - sy * akz);
        let (bx, by) = (bkx - sx * bkz, bky - sy * bkz);
        let (cx, cy) = (ckx - sx * ckz, cky - sy * ckz);

        // scaled barycentric coordinates (signed edge functions); the shear makes a
        // shared edge evaluate identically for both adjacent triangles, so the ray
        // can never fall between them

        let u = cx * by - cy * bx;
        let v = ax * cy - ay * cx;
        let w = bx * ay - by * ax;

        if backface_culling {
            if u < 0.0 || v < 0.0 || w < 0.0 {
                return None;
            }
        } else if (u < 0.0 || v < 0.0 || w < 0.0) && (u > 0.0 || v > 0.0 || w > 0.0) {
            return None;
        }

        let det = u + v + w;

        // ray is co-planar with the triangle

        if det == 0.0 {
            return None;
        }

        // scaled hit distance; t = tScaled / det must lie in front of the origin

        let t_scaled = sz * (u * akz + v * bkz + w * ckz);

        if if det > 0.0 { t_scaled < 0.0 } else { t_scaled > 0.0 } {
            return None;
        }

        Some(self.at(t_scaled / det))
    }

    /// `Ray.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix4: &Matrix4) -> &mut Self {
        self.origin.apply_matrix4(matrix4);
        self.direction.transform_direction(matrix4);
        self
    }

    /// `Ray.equals()`.
    pub fn equals(&self, ray: &Self) -> bool {
        ray.origin.equals(&self.origin) && ray.direction.equals(&self.direction)
    }
}
