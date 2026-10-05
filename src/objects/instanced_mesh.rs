//! Port of `three.js/src/objects/InstancedMesh.js` +
//! `src/core/InstancedBufferAttribute.js` (rung 2 subset).
//!
//! `InstancedMesh` extends `Mesh` in three.js; here it holds the `Mesh` payload
//! it extends, and the node's [`Payload`] variant keeps the two apart for the
//! renderer.

use std::rc::Rc;

use crate::core::{AttributeId, BufferGeometry, Object3D, ObjectRef};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4, Sphere};
use crate::objects::{Mesh, Payload};

/// `new InstancedBufferAttribute( new Float32Array( count * 16 ), 16 )`.
///
/// The renderer keeps one GPU buffer per attribute, keyed on [`id`](Self::id),
/// and writes into it only when [`version`](Self::version) has moved since the
/// last write, as three.js' `Attributes.update()` does (issue #89).
///
/// Unlike three.js, a write through this type bumps the version itself:
/// [`array_mut`](Self::array_mut) always does, and `setMatrixAt()` /
/// `setColorAt()` do when the value they write differs from the one there.
/// three.js needs `attribute.needsUpdate = true` because its `Float32Array` is
/// mutated behind the attribute's back; here every write goes through the
/// attribute, so a second call would only add a stale-frame footgun, the same
/// reasoning as `Texture::set_data` (`docs/api.md`).
/// [`set_needs_update`](Self::set_needs_update) stays, for forcing a write.
///
/// `clone()` mints a fresh id, as a cloned `BufferAttribute` does, and shares
/// the array until one side writes to it.
#[derive(Clone)]
pub struct InstancedBufferAttribute {
    /// `BufferAttribute.id`, from the same never-reused counter.
    id: AttributeId,
    /// The `Float32Array` — `count * item_size` values. Behind an `Rc` so the
    /// renderer's per-frame snapshot of a draw is a pointer copy rather than a
    /// copy of the array (eight megabytes at 1 << 17 instance matrices).
    array: Rc<Vec<f32>>,
    /// `BufferAttribute.itemSize`.
    pub item_size: usize,
    /// `BufferAttribute.version`.
    version: u32,
}

impl InstancedBufferAttribute {
    /// `new InstancedBufferAttribute( array, itemSize )`.
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self {
            id: AttributeId::default(),
            array: Rc::new(array),
            item_size,
            version: 0,
        }
    }

    /// `BufferAttribute.id`. Never reused, and never shared with a clone.
    pub fn id(&self) -> usize {
        self.id.get()
    }

    /// `attribute.array`, for reading.
    pub fn array(&self) -> &[f32] {
        &self.array
    }

    /// `attribute.array`, for writing. Bumps the version, so the next render
    /// re-writes the GPU buffer: the caller is taken to have changed something.
    pub fn array_mut(&mut self) -> &mut [f32] {
        self.version += 1;
        Rc::make_mut(&mut self.array).as_mut_slice()
    }

    /// `BufferAttribute.version` — how many times the array has been changed
    /// or marked changed.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// `attribute.needsUpdate = true` — `version ++`, so the next render
    /// re-writes the GPU buffer even though nothing was written through
    /// [`array_mut`](Self::array_mut).
    pub fn set_needs_update(&mut self) {
        self.version += 1;
    }

    /// `BufferAttribute.count` — `array.length / itemSize`.
    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }

    /// `values` written at `offset`, and the version bumped only if that
    /// changed the array: an animation loop that re-sets the matrices it set
    /// last frame costs no upload.
    fn write(&mut self, offset: usize, values: &[f32]) {
        let range = offset..offset + values.len();
        if self.array[range.clone()] != *values {
            self.array_mut()[range].copy_from_slice(values);
        }
    }

    /// What the renderer reads of this attribute for one frame: the identity
    /// and version its GPU buffer is cached on, and the array to write.
    pub(crate) fn snapshot(&self) -> InstanceData {
        InstanceData {
            id: self.id(),
            version: self.version,
            array: self.array.clone(),
            item_size: self.item_size,
        }
    }
}

/// An [`InstancedBufferAttribute`] as one frame's draws see it. It keeps the
/// attribute's id, which a clone of the attribute would not, and holds the
/// array by `Rc` only until the frame's render list is dropped.
#[derive(Clone)]
pub(crate) struct InstanceData {
    pub id: usize,
    pub version: u32,
    pub array: Rc<Vec<f32>>,
    pub item_size: usize,
}

impl InstanceData {
    /// `BufferAttribute.count`.
    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}

/// `InstancedMesh extends Mesh`: one geometry and material, drawn `count`
/// times with a per-instance transform (and optionally colour).
#[derive(Clone)]
pub struct InstancedMesh {
    /// The `Mesh` half: geometry, material and morph state.
    pub mesh: Mesh,
    /// `InstancedMesh.count` — the number of instances the renderer draws.
    pub count: usize,
    /// `InstancedMesh.instanceMatrix`.
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
    /// `new InstancedMesh( geometry, material, count )`, as a scene-graph [`ObjectRef`].
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `ObjectRef`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        count: usize,
    ) -> ObjectRef {
        let mut object = Object3D {
            object_type: "InstancedMesh",
            ..Default::default()
        };
        let mut instanced = Self {
            mesh: Mesh {
                geometry,
                material: Some(material),
                materials: Vec::new(),
                morph_target_influences: Vec::new(),
                line_segments: None,
                count: None,
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
            .zip(&self.instance_matrix.array()[offset..offset + 16])
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

    /// `InstancedMesh.raycast( raycaster, intersects )` — the object's own
    /// bounding sphere first, then `Mesh.raycast()` once per instance with
    /// `matrixWorld × instanceMatrix`, each hit tagged with its `instanceId`.
    ///
    /// three.js raycasts each instance through a scratch `Mesh` that has no
    /// `morphTargetInfluences`, so morph targets are ignored here too.
    pub fn raycast(
        &mut self,
        matrix_world: &Matrix4,
        object: &crate::core::ObjectRef,
        raycaster: &crate::core::Raycaster,
        intersects: &mut Vec<crate::core::Intersection>,
    ) {
        // Test with the bounding sphere first.
        if self.bounding_sphere.is_none() {
            self.compute_bounding_sphere();
        }
        let Some(mut sphere) = self.bounding_sphere else {
            return;
        };
        sphere.apply_matrix4(matrix_world);
        if !raycaster.ray.intersects_sphere(&sphere) {
            return;
        }

        let geometry = &self.mesh.geometry;
        let Some(geometry_sphere) = geometry.compute_bounding_sphere() else {
            return;
        };
        let geometry_sphere = Sphere::new(geometry_sphere.center, geometry_sphere.radius);
        let side = self
            .mesh
            .material
            .as_ref()
            .map_or(crate::materials::Side::Front, |m| m.side);

        // Now test each instance.
        let mut instance_intersects = Vec::new();
        for instance_id in 0..self.count {
            // The world matrix of this instance; the mesh represents it alone.
            let mut instance_world_matrix = Matrix4::identity();
            instance_world_matrix.multiply_matrices(matrix_world, &self.matrix_at(instance_id));

            let target = crate::objects::mesh::MeshRaycast {
                geometry,
                side,
                matrix_world: instance_world_matrix,
                draw_start: geometry.draw_range.start,
                draw_count: geometry.draw_range.count,
                vertex_position: &|index| {
                    crate::objects::mesh::vertex_position(geometry, &[], index)
                },
            };
            target.raycast(
                &geometry_sphere,
                None,
                object,
                raycaster,
                &mut instance_intersects,
            );

            for mut intersect in instance_intersects.drain(..) {
                intersect.instance_id = Some(instance_id);
                intersects.push(intersect);
            }
        }
    }

    /// `InstancedMesh.setColorAt( index, color )`.
    ///
    /// The array is allocated on the first call and **filled with 1**, not 0,
    /// so an instance that is never given a colour keeps the material's own —
    /// `new Float32Array( count * 3 ).fill( 1 )` in three.js.
    ///
    /// The colour is stored in the working (linear-sRGB) space the `Color`
    /// already holds; three.js writes `color.toArray()`, which is the same.
    ///
    /// Bumps `instanceColor`'s version when the colour differs from the one
    /// there; see [`InstancedBufferAttribute`] for why no `needsUpdate`.
    pub fn set_color_at(&mut self, index: usize, color: &Color) {
        let colors = self.instance_color.get_or_insert_with(|| {
            InstancedBufferAttribute::new(vec![1.0; self.instance_matrix.count() * 3], 3)
        });
        colors.write(index * 3, &[color.r as f32, color.g as f32, color.b as f32]);
    }

    /// `InstancedMesh.setMatrixAt( index, matrix )` —
    /// `matrix.toArray( this.instanceMatrix.array, index * 16 )`.
    ///
    /// Bumps `instanceMatrix`'s version when the matrix differs from the one
    /// there; see [`InstancedBufferAttribute`] for why no `needsUpdate`.
    pub fn set_matrix_at(&mut self, index: usize, matrix: &Matrix4) {
        self.instance_matrix
            .write(index * 16, &matrix.to_f32_array());
    }
}

/// The array is `count * item_size` floats — sixteen per instance for the
/// instance matrix. Debug prints its length, as the textures do.
impl std::fmt::Debug for InstancedBufferAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstancedBufferAttribute")
            .field("id", &self.id())
            .field("array", &format_args!("{} floats", self.array.len()))
            .field("item_size", &self.item_size)
            .field("version", &self.version)
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
