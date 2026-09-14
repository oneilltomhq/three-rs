//! Port of `three.js/src/objects/InstancedMesh.js` +
//! `src/core/InstancedBufferAttribute.js` (rung 2 subset).
//!
//! `InstancedMesh` extends `Mesh` in three.js; here it holds the `Mesh` payload
//! it extends, and the node's [`Payload`] variant keeps the two apart for the
//! renderer.

use std::rc::Rc;

use crate::core::{BufferGeometry, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Matrix4, Sphere};
use crate::objects::{Mesh, Payload};

/// `new InstancedBufferAttribute( new Float32Array( count * 16 ), 16 )`.
#[derive(Clone)]
pub struct InstancedBufferAttribute {
    pub array: Vec<f32>,
    pub item_size: usize,
}

impl InstancedBufferAttribute {
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self { array, item_size }
    }

    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}

#[derive(Clone)]
pub struct InstancedMesh {
    pub mesh: Mesh,
    /// `InstancedMesh.count` — the number of instances the renderer draws.
    pub count: usize,
    pub instance_matrix: InstancedBufferAttribute,
    /// `InstancedMesh.instanceColor`, `null` until `setColorAt()` is called.
    pub instance_color: Option<InstancedBufferAttribute>,
    /// `InstancedMesh.boundingSphere` — the object's *own* bounding sphere, in
    /// object space, over all of its instances.
    ///
    /// `Frustum.intersectsObject` prefers it to the geometry's whenever the
    /// object declares one (`object.boundingSphere !== undefined` in three.js),
    /// which is the only thing that keeps an instanced draw whose instances are
    /// spread out from being culled as a point at the object's origin.
    /// `None` is three.js' `null`: nothing has computed it yet, so the
    /// geometry's sphere is used instead.
    pub bounding_sphere: Option<Sphere>,
}

impl InstancedMesh {
    /// `new InstancedMesh( geometry, material, count )`, as a scene-graph [`Node`].
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        count: usize,
    ) -> Node {
        let mut object = Object3D {
            object_type: "InstancedMesh",
            ..Default::default()
        };
        let mut instanced = Self {
            mesh: Mesh {
                geometry,
                material: Some(material),
                morph_target_influences: Vec::new(),
            },
            count,
            instance_matrix: InstancedBufferAttribute::new(vec![0.0; count * 16], 16),
            instance_color: None,
            bounding_sphere: None,
        };
        // `for ( let i = 0; i < count; i ++ ) this.setMatrixAt( i, _identity );`
        // — the constructor's last step. A mesh whose instances are never placed
        // still draws at the origin, not collapsed to a zero matrix.
        let identity = Matrix4::identity();
        for i in 0..count {
            instanced.set_matrix_at(i, &identity);
        }
        object.payload = Payload::InstancedMesh(instanced);
        object.into_node()
    }

    /// `InstancedMesh.getMatrixAt( index, matrix )` —
    /// `matrix.fromArray( this.instanceMatrix.array, index * 16 )`.
    pub fn matrix_at(&self, index: usize) -> Matrix4 {
        let offset = index * 16;
        let mut matrix = Matrix4::identity();
        for (e, &a) in matrix
            .elements
            .iter_mut()
            .zip(&self.instance_matrix.array[offset..offset + 16])
        {
            *e = a as f64;
        }
        matrix
    }

    /// `InstancedMesh.computeBoundingSphere()` — the geometry's sphere pushed
    /// through every instance matrix, unioned. `count` instances, not the
    /// array's capacity, exactly as three.js does.
    ///
    /// A geometry with no position attribute has no sphere to spread, so this
    /// leaves [`InstancedMesh::bounding_sphere`] alone; three.js would throw.
    pub fn compute_bounding_sphere(&mut self) {
        let Some(geometry_sphere) = self.mesh.geometry.compute_bounding_sphere() else {
            return;
        };
        let mut bounding_sphere = Sphere::default();
        bounding_sphere.make_empty();

        for i in 0..self.count {
            let mut sphere = Sphere::new(geometry_sphere.center, geometry_sphere.radius);
            sphere.apply_matrix4(&self.matrix_at(i));
            bounding_sphere.union(&sphere);
        }

        self.bounding_sphere = Some(bounding_sphere);
    }

    /// `InstancedMesh.setMatrixAt( index, matrix )` —
    /// `matrix.toArray( this.instanceMatrix.array, index * 16 )`.
    pub fn set_matrix_at(&mut self, index: usize, matrix: &Matrix4) {
        let offset = index * 16;
        let elements = matrix.to_f32_array();
        self.instance_matrix.array[offset..offset + 16].copy_from_slice(&elements);
    }
}

/// The array is `count * item_size` floats — sixteen per instance for the
/// instance matrix. Debug prints its length, as the textures do.
impl std::fmt::Debug for InstancedBufferAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancedBufferAttribute")
            .field("array", &format_args!("{} floats", self.array.len()))
            .field("item_size", &self.item_size)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometries::plane_geometry;
    use crate::objects::Payload;

    /// Two instances ten units either side of the origin: the object's own
    /// sphere has to span both, where the geometry's spans one unit plane at
    /// the origin. This is the difference that decides whether a spread-out
    /// instanced draw survives the frustum cull.
    #[test]
    fn compute_bounding_sphere_spans_the_instances() {
        let node = InstancedMesh::new(
            Rc::new(plane_geometry(1.0, 1.0, 1, 1)),
            MeshBasicNodeMaterial::default(),
            2,
        );

        let half_diagonal = (0.5f64 * 0.5 + 0.5 * 0.5).sqrt();
        {
            let mut object = node.borrow_mut();
            let instanced = object
                .payload
                .instanced_mesh_mut()
                .expect("InstancedMesh::new makes an InstancedMesh payload");
            let mut left = Matrix4::identity();
            left.set_position(-10.0, 0.0, 0.0);
            let mut right = Matrix4::identity();
            right.set_position(10.0, 0.0, 0.0);
            instanced.set_matrix_at(0, &left);
            instanced.set_matrix_at(1, &right);

            // Before `computeBoundingSphere`, `boundingSphere` is null and the
            // geometry's sphere stands in — the unit plane at the origin.
            assert!(instanced.bounding_sphere.is_none());

            instanced.compute_bounding_sphere();
            let sphere = instanced
                .bounding_sphere
                .expect("compute_bounding_sphere sets it for a geometry with positions");
            assert!(sphere.center.x.abs() < 1e-12);
            assert!((sphere.radius - (10.0 + half_diagonal)).abs() < 1e-12);
        }

        // And the payload hands that sphere to the cull, not the geometry's.
        let object = node.borrow();
        let sphere = object
            .payload
            .bounding_sphere_in(&object.matrix_world)
            .expect("a plane has a bounding sphere");
        assert!((sphere.radius - (10.0 + half_diagonal)).abs() < 1e-12);

        // The same payload with no sphere of its own falls back to the
        // geometry's, as three.js does.
        let plain = InstancedMesh::new(
            Rc::new(plane_geometry(1.0, 1.0, 1, 1)),
            MeshBasicNodeMaterial::default(),
            2,
        );
        let plain = plain.borrow();
        let Payload::InstancedMesh(instanced) = &plain.payload else {
            panic!("InstancedMesh::new makes an InstancedMesh payload");
        };
        assert!(instanced.bounding_sphere.is_none());
        let sphere = plain
            .payload
            .bounding_sphere_in(&plain.matrix_world)
            .expect("a plane has a bounding sphere");
        assert!((sphere.radius - half_diagonal).abs() < 1e-12);
    }
}
