//! Port of `three.js/src/math/Triangle.js`.

use crate::core::BufferAttribute;

use super::{Box3, Plane, Vector3, Vector4};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Triangle {
    pub a: Vector3,
    pub b: Vector3,
    pub c: Vector3,
}

impl Triangle {
    pub const fn new(a: Vector3, b: Vector3, c: Vector3) -> Self {
        Self { a, b, c }
    }

    /// `Triangle.getNormal()` (static).
    pub fn static_get_normal(a: &Vector3, b: &Vector3, c: &Vector3) -> Vector3 {
        let mut target = Vector3::default();
        let mut v0 = Vector3::default();

        target.sub_vectors(c, b);
        v0.sub_vectors(a, b);
        target.cross(&v0);

        let target_length_sq = target.length_sq();
        if target_length_sq > 0.0 {
            target.multiply_scalar(1.0 / target_length_sq.sqrt());
            return target;
        }

        target.set(0.0, 0.0, 0.0);
        target
    }

    /// `Triangle.getBarycoord()` (static). `None` where three.js returns `null`
    /// (collinear or singular triangle).
    pub fn static_get_barycoord(
        point: &Vector3,
        a: &Vector3,
        b: &Vector3,
        c: &Vector3,
    ) -> Option<Vector3> {
        let mut v0 = Vector3::default();
        let mut v1 = Vector3::default();
        let mut v2 = Vector3::default();

        v0.sub_vectors(c, a);
        v1.sub_vectors(b, a);
        v2.sub_vectors(point, a);

        let dot00 = v0.dot(&v0);
        let dot01 = v0.dot(&v1);
        let dot02 = v0.dot(&v2);
        let dot11 = v1.dot(&v1);
        let dot12 = v1.dot(&v2);

        let denom = dot00 * dot11 - dot01 * dot01;

        // collinear or singular triangle
        if denom == 0.0 {
            return None;
        }

        let inv_denom = 1.0 / denom;
        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

        // barycentric coordinates must always sum to 1
        let mut target = Vector3::default();
        target.set(1.0 - u - v, v, u);
        Some(target)
    }

    /// `Triangle.containsPoint()` (static).
    pub fn static_contains_point(point: &Vector3, a: &Vector3, b: &Vector3, c: &Vector3) -> bool {
        // if the triangle is degenerate then we can't contain a point
        let Some(v3) = Self::static_get_barycoord(point, a, b, c) else {
            return false;
        };

        (v3.x >= 0.0) && (v3.y >= 0.0) && ((v3.x + v3.y) <= 1.0)
    }

    /// `Triangle.getInterpolation()` (static). `None` where three.js returns
    /// `null` (degenerate triangle).
    #[allow(clippy::too_many_arguments)]
    pub fn static_get_interpolation(
        point: &Vector3,
        p1: &Vector3,
        p2: &Vector3,
        p3: &Vector3,
        v1: &Vector3,
        v2: &Vector3,
        v3: &Vector3,
    ) -> Option<Vector3> {
        let bary = Self::static_get_barycoord(point, p1, p2, p3)?;

        let mut target = Vector3::default();
        target.set_scalar(0.0);
        target.add_scaled_vector(v1, bary.x);
        target.add_scaled_vector(v2, bary.y);
        target.add_scaled_vector(v3, bary.z);

        Some(target)
    }

    /// `Triangle.getInterpolatedAttribute()` (static): the attribute's values
    /// at vertices `i1`, `i2`, `i3`, weighted by `barycoord`.
    ///
    /// three.js takes a `Vector2`/`Vector3`/`Vector4` target and reads as many
    /// components as the target has; the port returns a [`Vector4`] holding the
    /// first `min( itemSize, 4 )` components, the rest 0, and the caller takes
    /// the swizzle it wants (`Mesh` takes `xy` for `uv` and `xyz` for
    /// `normal`, which is what its `new Vector2()` / `new Vector3()` targets
    /// read).
    pub fn static_get_interpolated_attribute(
        attr: &BufferAttribute,
        i1: usize,
        i2: usize,
        i3: usize,
        barycoord: &Vector3,
    ) -> Vector4 {
        let item_size = attr.item_size.min(4);
        let array = attr.array();
        let read = |index: usize| {
            let mut v = Vector4::new(0.0, 0.0, 0.0, 0.0);
            for component in 0..item_size {
                v.set_component(component, array[index * attr.item_size + component] as f64);
            }
            v
        };
        let (v40, v41, v42) = (read(i1), read(i2), read(i3));

        let mut target = Vector4::new(0.0, 0.0, 0.0, 0.0);
        target.add_scaled_vector(&v40, barycoord.x);
        target.add_scaled_vector(&v41, barycoord.y);
        target.add_scaled_vector(&v42, barycoord.z);
        target
    }

    /// `Triangle.isFrontFacing()` (static).
    pub fn static_is_front_facing(
        a: &Vector3,
        b: &Vector3,
        c: &Vector3,
        direction: &Vector3,
    ) -> bool {
        let mut v0 = Vector3::default();
        let mut v1 = Vector3::default();

        v0.sub_vectors(c, b);
        v1.sub_vectors(a, b);

        // strictly front facing
        v0.cross(&v1).dot(direction) < 0.0
    }

    /// `Triangle.set()`.
    pub fn set(&mut self, a: &Vector3, b: &Vector3, c: &Vector3) -> &mut Self {
        self.a.copy(a);
        self.b.copy(b);
        self.c.copy(c);
        self
    }

    /// `Triangle.setFromPointsAndIndices()`.
    pub fn set_from_points_and_indices(
        &mut self,
        points: &[Vector3],
        i0: usize,
        i1: usize,
        i2: usize,
    ) -> &mut Self {
        self.a.copy(&points[i0]);
        self.b.copy(&points[i1]);
        self.c.copy(&points[i2]);
        self
    }

    /// `Triangle.setFromAttributeAndIndices()`.
    pub fn set_from_attribute_and_indices(
        &mut self,
        attribute: &BufferAttribute,
        i0: usize,
        i1: usize,
        i2: usize,
    ) -> &mut Self {
        self.a = attribute.get_vector3(i0);
        self.b = attribute.get_vector3(i1);
        self.c = attribute.get_vector3(i2);
        self
    }

    /// `Triangle.copy()`.
    pub fn copy(&mut self, triangle: &Self) -> &mut Self {
        self.a.copy(&triangle.a);
        self.b.copy(&triangle.b);
        self.c.copy(&triangle.c);
        self
    }

    /// `Triangle.getArea()`.
    pub fn get_area(&self) -> f64 {
        let mut v0 = Vector3::default();
        let mut v1 = Vector3::default();

        v0.sub_vectors(&self.c, &self.b);
        v1.sub_vectors(&self.a, &self.b);

        v0.cross(&v1).length() * 0.5
    }

    /// `Triangle.getMidpoint()`.
    pub fn get_midpoint(&self) -> Vector3 {
        let mut target = Vector3::default();
        target
            .add_vectors(&self.a, &self.b)
            .add(&self.c)
            .multiply_scalar(1.0 / 3.0);
        target
    }

    /// `Triangle.getNormal()`.
    pub fn get_normal(&self) -> Vector3 {
        Triangle::static_get_normal(&self.a, &self.b, &self.c)
    }

    /// `Triangle.getPlane()`.
    pub fn get_plane(&self) -> Plane {
        let mut target = Plane::default();
        target.set_from_coplanar_points(&self.a, &self.b, &self.c);
        target
    }

    /// `Triangle.getBarycoord()`.
    pub fn get_barycoord(&self, point: &Vector3) -> Option<Vector3> {
        Triangle::static_get_barycoord(point, &self.a, &self.b, &self.c)
    }

    /// `Triangle.getInterpolation()`.
    pub fn get_interpolation(
        &self,
        point: &Vector3,
        v1: &Vector3,
        v2: &Vector3,
        v3: &Vector3,
    ) -> Option<Vector3> {
        Triangle::static_get_interpolation(point, &self.a, &self.b, &self.c, v1, v2, v3)
    }

    /// `Triangle.containsPoint()`.
    pub fn contains_point(&self, point: &Vector3) -> bool {
        Triangle::static_contains_point(point, &self.a, &self.b, &self.c)
    }

    /// `Triangle.isFrontFacing()`.
    pub fn is_front_facing(&self, direction: &Vector3) -> bool {
        Triangle::static_is_front_facing(&self.a, &self.b, &self.c, direction)
    }

    /// `Triangle.intersectsBox()`.
    pub fn intersects_box(&self, box3: &Box3) -> bool {
        box3.intersects_triangle(self)
    }

    /// `Triangle.closestPointToPoint()`.
    pub fn closest_point_to_point(&self, p: &Vector3) -> Vector3 {
        let a = self.a;
        let b = self.b;
        let c = self.c;
        let mut target = Vector3::default();

        let mut vab = Vector3::default();
        let mut vac = Vector3::default();
        let mut vbc = Vector3::default();
        let mut vap = Vector3::default();
        let mut vbp = Vector3::default();
        let mut vcp = Vector3::default();

        // algorithm thanks to Real-Time Collision Detection by Christer Ericson,
        // published by Morgan Kaufmann Publishers, (c) 2005 Elsevier Inc.,
        // under the accompanying license; see chapter 5.1.5 for detailed explanation.
        // basically, we're distinguishing which of the voronoi regions of the triangle
        // the point lies in with the minimum amount of redundant computation.

        vab.sub_vectors(&b, &a);
        vac.sub_vectors(&c, &a);
        vap.sub_vectors(p, &a);
        let d1 = vab.dot(&vap);
        let d2 = vac.dot(&vap);
        if d1 <= 0.0 && d2 <= 0.0 {
            // vertex region of A; barycentric coords (1, 0, 0)
            target.copy(&a);
            return target;
        }

        vbp.sub_vectors(p, &b);
        let d3 = vab.dot(&vbp);
        let d4 = vac.dot(&vbp);
        if d3 >= 0.0 && d4 <= d3 {
            // vertex region of B; barycentric coords (0, 1, 0)
            target.copy(&b);
            return target;
        }

        let vc = d1 * d4 - d3 * d2;
        if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            let v = d1 / (d1 - d3);
            // edge region of AB; barycentric coords (1-v, v, 0)
            target.copy(&a).add_scaled_vector(&vab, v);
            return target;
        }

        vcp.sub_vectors(p, &c);
        let d5 = vab.dot(&vcp);
        let d6 = vac.dot(&vcp);
        if d6 >= 0.0 && d5 <= d6 {
            // vertex region of C; barycentric coords (0, 0, 1)
            target.copy(&c);
            return target;
        }

        let vb = d5 * d2 - d1 * d6;
        if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            let w = d2 / (d2 - d6);
            // edge region of AC; barycentric coords (1-w, 0, w)
            target.copy(&a).add_scaled_vector(&vac, w);
            return target;
        }

        let va = d3 * d6 - d5 * d4;
        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            vbc.sub_vectors(&c, &b);
            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            // edge region of BC; barycentric coords (0, 1-w, w)
            target.copy(&b).add_scaled_vector(&vbc, w); // edge region of BC
            return target;
        }

        // face region
        let denom = 1.0 / (va + vb + vc);
        // u = va * denom
        let v = vb * denom;
        let w = vc * denom;

        target
            .copy(&a)
            .add_scaled_vector(&vab, v)
            .add_scaled_vector(&vac, w);
        target
    }

    /// `Triangle.equals()`.
    pub fn equals(&self, triangle: &Self) -> bool {
        triangle.a.equals(&self.a) && triangle.b.equals(&self.b) && triangle.c.equals(&self.c)
    }
}
