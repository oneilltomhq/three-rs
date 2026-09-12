//! Port of `three.js/src/core/BufferGeometry.js` (interleaved-free `f32`
//! attributes plus a `u16`/`u32` index).

use crate::math::{Matrix3, Matrix4, Vector3};

#[derive(Clone, Debug)]
pub struct BufferAttribute {
    pub array: Vec<f32>,
    pub item_size: usize,
}

impl BufferAttribute {
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self { array, item_size }
    }

    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }

    /// `BufferAttribute.getX/getY/getZ()` — the stored value is `f32`, widened
    /// the way JavaScript widens a `Float32Array` read to a number.
    pub fn get_x(&self, index: usize) -> f64 {
        self.array[index * self.item_size] as f64
    }

    pub fn get_y(&self, index: usize) -> f64 {
        self.array[index * self.item_size + 1] as f64
    }

    pub fn get_z(&self, index: usize) -> f64 {
        self.array[index * self.item_size + 2] as f64
    }

    pub fn get_w(&self, index: usize) -> f64 {
        self.array[index * self.item_size + 3] as f64
    }

    /// `BufferAttribute.setX/setY/setZ/setW()`.
    pub fn set_x(&mut self, index: usize, x: f64) -> &mut Self {
        self.array[index * self.item_size] = x as f32;
        self
    }

    pub fn set_y(&mut self, index: usize, y: f64) -> &mut Self {
        self.array[index * self.item_size + 1] = y as f32;
        self
    }

    pub fn set_z(&mut self, index: usize, z: f64) -> &mut Self {
        self.array[index * self.item_size + 2] = z as f32;
        self
    }

    pub fn set_w(&mut self, index: usize, w: f64) -> &mut Self {
        self.array[index * self.item_size + 3] = w as f32;
        self
    }

    /// `BufferAttribute.setXY()`.
    pub fn set_xy(&mut self, index: usize, x: f64, y: f64) -> &mut Self {
        let offset = index * self.item_size;
        self.array[offset] = x as f32;
        self.array[offset + 1] = y as f32;
        self
    }

    /// `BufferAttribute.setXYZW()`.
    pub fn set_xyzw(&mut self, index: usize, x: f64, y: f64, z: f64, w: f64) -> &mut Self {
        let offset = index * self.item_size;
        self.array[offset] = x as f32;
        self.array[offset + 1] = y as f32;
        self.array[offset + 2] = z as f32;
        self.array[offset + 3] = w as f32;
        self
    }

    /// `BufferAttribute.copyAt()` — copies one item from `attribute`.
    pub fn copy_at(&mut self, index1: usize, attribute: &Self, index2: usize) -> &mut Self {
        let index1 = index1 * self.item_size;
        let index2 = index2 * attribute.item_size;

        for i in 0..self.item_size {
            self.array[index1 + i] = attribute.array[index2 + i];
        }

        self
    }

    /// `BufferAttribute.copyArray()`.
    pub fn copy_array(&mut self, array: &[f32]) -> &mut Self {
        self.array.copy_from_slice(array);
        self
    }

    /// `BufferAttribute.set( value, offset )`.
    pub fn set(&mut self, value: &[f32], offset: usize) -> &mut Self {
        self.array[offset..offset + value.len()].copy_from_slice(value);
        self
    }

    /// `BufferAttribute.setXYZ()` — narrows to `f32` on the way in, which is
    /// where three.js loses precision too.
    pub fn set_xyz(&mut self, index: usize, x: f64, y: f64, z: f64) {
        let offset = index * self.item_size;
        self.array[offset] = x as f32;
        self.array[offset + 1] = y as f32;
        self.array[offset + 2] = z as f32;
    }

    /// `Vector3.fromBufferAttribute( attribute, index )`.
    pub fn get_vector3(&self, index: usize) -> Vector3 {
        Vector3::new(self.get_x(index), self.get_y(index), self.get_z(index))
    }

    /// `BufferAttribute.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &Matrix4) {
        for i in 0..self.count() {
            let mut v = self.get_vector3(i);
            v.apply_matrix4(m);
            self.set_xyz(i, v.x, v.y, v.z);
        }
    }

    /// `BufferAttribute.applyNormalMatrix()`.
    pub fn apply_normal_matrix(&mut self, m: &Matrix3) {
        for i in 0..self.count() {
            let mut v = self.get_vector3(i);
            v.apply_normal_matrix(m);
            self.set_xyz(i, v.x, v.y, v.z);
        }
    }
}

/// `Box3`, as far as `BufferGeometry.computeBoundingBox()` needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    pub min: Vector3,
    pub max: Vector3,
}

impl BoundingBox {
    /// `Box3.getCenter()`.
    pub fn center(&self) -> Vector3 {
        Vector3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }
}

/// `Sphere`, as far as `BufferGeometry.computeBoundingSphere()` needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingSphere {
    pub center: Vector3,
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub enum Index {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Index {
    pub fn count(&self) -> usize {
        match self {
            Index::U16(v) => v.len(),
            Index::U32(v) => v.len(),
        }
    }
}

/// A geometry with the named attributes three.js uses (`position`, `normal`,
/// `uv`). Rung 1 only consumes `position`, but the full set is generated so the
/// data matches three.js byte for byte.
#[derive(Clone, Debug, Default)]
pub struct BufferGeometry {
    pub position: Option<BufferAttribute>,
    pub normal: Option<BufferAttribute>,
    pub uv: Option<BufferAttribute>,
    pub index: Option<Index>,
}

impl BufferGeometry {
    pub fn new() -> Self {
        Self::default()
    }

    /// `BufferGeometry.setIndex( array )` — picks `Uint16` when it fits, the
    /// same rule three.js uses.
    pub fn set_index(&mut self, indices: &[u32]) {
        let max = indices.iter().copied().max().unwrap_or(0);
        self.index = Some(if max > 65535 {
            Index::U32(indices.to_vec())
        } else {
            Index::U16(indices.iter().map(|&i| i as u16).collect())
        });
    }

    /// `BufferGeometry.setIndex( bufferAttribute )` — keeps the array type the
    /// source declared, which is what `BufferGeometryLoader.parse()` does.
    pub fn set_index_attribute(&mut self, index: Index) {
        self.index = Some(index);
    }

    /// The centre of `BufferGeometry.computeBoundingSphere()`'s sphere, which is
    /// `Box3.setFromBufferAttribute( position ).getCenter()` — the only part of
    /// the bounding sphere the render-list sort reads.
    pub fn bounding_sphere_center(&self) -> Vector3 {
        match self.compute_bounding_box() {
            Some(box3) => box3.center(),
            None => Vector3::ZERO,
        }
    }

    /// `BufferGeometry.computeBoundingBox()` — `Box3.setFromBufferAttribute(
    /// position )`. `None` when there is no position attribute (three.js leaves
    /// `boundingBox` alone in that case).
    pub fn compute_bounding_box(&self) -> Option<BoundingBox> {
        let position = self.position.as_ref()?;

        let mut min = Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for i in 0..position.count() {
            let v = position.get_vector3(i);
            min.x = min.x.min(v.x);
            min.y = min.y.min(v.y);
            min.z = min.z.min(v.z);
            max.x = max.x.max(v.x);
            max.y = max.y.max(v.y);
            max.z = max.z.max(v.z);
        }

        Some(BoundingBox { min, max })
    }

    /// `BufferGeometry.computeBoundingSphere()`: the bounding box's centre, then
    /// the largest distance from it to any vertex (which beats the box's own
    /// sphere by up to sqrt(3)).
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        let position = self.position.as_ref()?;
        let center = self.compute_bounding_box()?.center();

        let mut max_radius_sq: f64 = 0.0;

        for i in 0..position.count() {
            let v = position.get_vector3(i);
            max_radius_sq = max_radius_sq.max(center.distance_to_squared(&v));
        }

        Some(BoundingSphere {
            center,
            radius: max_radius_sq.sqrt(),
        })
    }

    /// `BufferGeometry.computeVertexNormals()`.
    ///
    /// The accumulation runs through the `normal` attribute itself, so every
    /// partial sum is rounded to `f32` before the next triangle adds to it.
    pub fn compute_vertex_normals(&mut self) {
        let Some(position) = self.position.clone() else {
            return;
        };

        let needs_new = match &self.normal {
            Some(normal) => normal.count() != position.count(),
            None => true,
        };

        let mut normal = if needs_new {
            BufferAttribute::new(vec![0.0; position.count() * 3], 3)
        } else {
            let mut normal = self.normal.take().unwrap();
            for i in 0..normal.count() {
                normal.set_xyz(i, 0.0, 0.0, 0.0);
            }
            normal
        };

        let mut cb = Vector3::ZERO;
        let mut ab = Vector3::ZERO;

        // indexed elements
        if let Some(index) = &self.index {
            let get = |i: usize| -> usize {
                match index {
                    Index::U16(v) => v[i] as usize,
                    Index::U32(v) => v[i] as usize,
                }
            };

            let mut i = 0;
            while i < index.count() {
                let (v_a, v_b, v_c) = (get(i), get(i + 1), get(i + 2));

                let p_a = position.get_vector3(v_a);
                let p_b = position.get_vector3(v_b);
                let p_c = position.get_vector3(v_c);

                cb.sub_vectors(&p_c, &p_b);
                ab.sub_vectors(&p_a, &p_b);
                cb.cross(&ab);

                let mut n_a = normal.get_vector3(v_a);
                let mut n_b = normal.get_vector3(v_b);
                let mut n_c = normal.get_vector3(v_c);

                n_a.add(&cb);
                n_b.add(&cb);
                n_c.add(&cb);

                normal.set_xyz(v_a, n_a.x, n_a.y, n_a.z);
                normal.set_xyz(v_b, n_b.x, n_b.y, n_b.z);
                normal.set_xyz(v_c, n_c.x, n_c.y, n_c.z);

                i += 3;
            }
        } else {
            let mut i = 0;
            while i < position.count() {
                let p_a = position.get_vector3(i);
                let p_b = position.get_vector3(i + 1);
                let p_c = position.get_vector3(i + 2);

                cb.sub_vectors(&p_c, &p_b);
                ab.sub_vectors(&p_a, &p_b);
                cb.cross(&ab);

                normal.set_xyz(i, cb.x, cb.y, cb.z);
                normal.set_xyz(i + 1, cb.x, cb.y, cb.z);
                normal.set_xyz(i + 2, cb.x, cb.y, cb.z);

                i += 3;
            }
        }

        self.normal = Some(normal);

        self.normalize_normals();
    }

    /// `BufferGeometry.normalizeNormals()`.
    pub fn normalize_normals(&mut self) {
        let Some(normal) = self.normal.as_mut() else {
            return;
        };

        for i in 0..normal.count() {
            let mut v = normal.get_vector3(i);
            v.normalize();
            normal.set_xyz(i, v.x, v.y, v.z);
        }
    }

    /// `BufferGeometry.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix: &Matrix4) -> &mut Self {
        if let Some(position) = self.position.as_mut() {
            position.apply_matrix4(matrix);
        }

        if let Some(normal) = self.normal.as_mut() {
            let mut normal_matrix = Matrix3::identity();
            normal_matrix.get_normal_matrix(matrix);
            normal.apply_normal_matrix(&normal_matrix);
        }

        self
    }

    /// `BufferGeometry.scale()`.
    pub fn scale(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_scale(x, y, z);
        self.apply_matrix4(&m1)
    }
}
