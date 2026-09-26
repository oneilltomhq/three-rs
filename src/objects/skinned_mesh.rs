//! Port of `three.js/src/objects/SkinnedMesh.js`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::core::{BufferGeometry, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Box3, Matrix4, Sphere, Vector3, Vector4};
use crate::objects::{Mesh, Payload, Skeleton};

/// `AttachedBindMode` / `DetachedBindMode` from `src/constants.js`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BindMode {
    /// `'attached'` — the bind matrix inverse tracks `matrixWorld`.
    #[default]
    Attached,
    /// `'detached'` — it tracks `bindMatrix`.
    Detached,
}

/// `class SkinnedMesh extends Mesh`.
///
/// The `Mesh` half is held rather than inherited, exactly as
/// [`InstancedMesh`](crate::objects::InstancedMesh) holds it, and the whole
/// struct is one node's [`Payload`] — so the renderer's tree walk finds a
/// skinned mesh wherever it hangs, and the bones (which are ordinary nodes in
/// the same tree) reach it through their shared [`Skeleton`].
#[derive(Clone)]
pub struct SkinnedMesh {
    /// The `Mesh` it extends: `geometry` and `material`.
    pub mesh: Mesh,
    /// `SkinnedMesh.skeleton`.
    pub skeleton: Option<Rc<RefCell<Skeleton>>>,
    /// `SkinnedMesh.bindMode`.
    pub bind_mode: BindMode,
    /// `SkinnedMesh.bindMatrix`.
    pub bind_matrix: Matrix4,
    /// `SkinnedMesh.bindMatrixInverse`.
    ///
    /// In the default [`BindMode::Attached`] this is `matrixWorld⁻¹`, recomputed
    /// **every frame** by `Object3D.updateMatrixWorld()` — see
    /// [`SkinnedMesh::update_bind_matrix_inverse`], which the tree walk calls
    /// for this payload the way three.js' `SkinnedMesh.updateMatrixWorld()`
    /// override does. A stale or identity value here scales the skin by the
    /// node's own scale and is silent.
    pub bind_matrix_inverse: Matrix4,
    /// `Mesh.boundingBox` (`SkinnedMesh.computeBoundingBox`).
    pub bounding_box: Option<Box3>,
    /// `Mesh.boundingSphere` (`SkinnedMesh.computeBoundingSphere`).
    pub bounding_sphere: Option<Sphere>,
    /// `Mesh.morphTargetInfluences`, which glTF morph animation writes into.
    /// Shared, because `SceneResolver` binds a `.morphTargetInfluences` track to
    /// it while the mesh is owned elsewhere.
    pub morph_target_influences: Rc<RefCell<Vec<f64>>>,
    /// `Mesh.morphTargetDictionary`.
    pub morph_target_dictionary: Vec<(String, usize)>,
}

impl SkinnedMesh {
    /// `new SkinnedMesh( geometry, material )`, as a scene-graph [`Node`].
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: impl Into<Option<MeshBasicNodeMaterial>>,
    ) -> Node {
        let mut object = Object3D {
            object_type: "SkinnedMesh",
            ..Default::default()
        };
        object.payload = Payload::SkinnedMesh(Box::new(Self::of(geometry, material.into())));
        object.into_node()
    }

    /// The payload on its own, for a caller that already has the node —
    /// `GLTFLoader`, which builds the node from the glTF node definition first.
    pub fn of(geometry: Rc<BufferGeometry>, material: Option<MeshBasicNodeMaterial>) -> Self {
        Self {
            mesh: Mesh {
                geometry,
                material,
                morph_target_influences: Vec::new(),
                line_segments: None,
                count: 1,
            },
            skeleton: None,
            bind_mode: BindMode::Attached,
            bind_matrix: Matrix4::identity(),
            bind_matrix_inverse: Matrix4::identity(),
            bounding_box: None,
            bounding_sphere: None,
            morph_target_influences: Rc::new(RefCell::new(Vec::new())),
            morph_target_dictionary: Vec::new(),
        }
    }

    /// `SkinnedMesh.geometry`.
    pub fn geometry(&self) -> &Rc<BufferGeometry> {
        &self.mesh.geometry
    }

    /// `SkinnedMesh.bind( skeleton, bindMatrix )` for a mesh that is already a
    /// node — the node is needed for three.js' `bindMatrix === undefined`
    /// branch, which updates the world matrix and takes it.
    pub fn bind(node: &Node, skeleton: Rc<RefCell<Skeleton>>, bind_matrix: Option<Matrix4>) {
        let bind_matrix = match bind_matrix {
            Some(matrix) => matrix,
            None => {
                node.update_matrix_world(true);
                skeleton.borrow_mut().calculate_inverses();
                node.borrow().matrix_world
            }
        };

        let mut object = node.borrow_mut();
        let skin = object
            .payload
            .skinned_mesh_mut()
            .expect("three-rs: SkinnedMesh::bind needs a SkinnedMesh node");
        skin.skeleton = Some(skeleton);
        skin.bind_matrix.copy(&bind_matrix);
        skin.bind_matrix_inverse.copy(&bind_matrix);
        skin.bind_matrix_inverse.invert();
    }

    /// The tail of `SkinnedMesh.updateMatrixWorld( force )`: everything after
    /// `super.updateMatrixWorld( force )`. The tree walk
    /// (`Object3D.updateMatrixWorld`) runs it for every `SkinnedMesh` payload it
    /// passes, which is what keeps `bindMatrixInverse` in step with a moving or
    /// scaled parent without the caller having to remember.
    pub fn update_bind_matrix_inverse(&mut self, matrix_world: &Matrix4) {
        match self.bind_mode {
            BindMode::Attached => {
                self.bind_matrix_inverse.copy(matrix_world);
                self.bind_matrix_inverse.invert();
            }
            BindMode::Detached => {
                self.bind_matrix_inverse.copy(&self.bind_matrix);
                self.bind_matrix_inverse.invert();
            }
        }
    }

    /// `SkinnedMesh.pose()`.
    pub fn pose(&mut self) {
        if let Some(skeleton) = &self.skeleton {
            skeleton.borrow_mut().pose();
        }
    }

    /// `SkinnedMesh.normalizeSkinWeights()`. Mutates the geometry, so it takes
    /// the geometry out of the `Rc` (three.js mutates in place; an `Rc` shared
    /// with the renderer cannot be).
    pub fn normalize_skin_weights(&mut self) {
        let mut geometry = (*self.mesh.geometry).clone();

        let Some(skin_weight) = geometry.get_attribute_mut("skinWeight") else {
            return;
        };

        let mut vector = Vector4::default();

        for i in 0..skin_weight.count() {
            vector.set(
                skin_weight.get_x(i),
                skin_weight.get_y(i),
                skin_weight.get_z(i),
                skin_weight.get_w(i),
            );

            let scale = 1.0 / vector.manhattan_length();

            if scale.is_finite() {
                vector.multiply_scalar(scale);
            } else {
                // fix gltf files with all zero weights
                vector.set(1.0, 0.0, 0.0, 0.0);
            }

            skin_weight.set_x(i, vector.x);
            skin_weight.set_y(i, vector.y);
            skin_weight.set_z(i, vector.z);
            skin_weight.set_w(i, vector.w);
        }

        self.mesh.geometry = Rc::new(geometry);
    }

    /// `SkinnedMesh.applyBoneTransform( index, vector )`.
    pub fn apply_bone_transform(&self, index: usize, vector: &mut Vector3) {
        let Some(skeleton) = &self.skeleton else {
            return;
        };
        let skeleton = skeleton.borrow();

        let (Some(skin_index), Some(skin_weight)) = (
            self.mesh.geometry.get_attribute("skinIndex"),
            self.mesh.geometry.get_attribute("skinWeight"),
        ) else {
            return;
        };

        let mut base_position = *vector;
        base_position.apply_matrix4(&self.bind_matrix);

        vector.set(0.0, 0.0, 0.0);

        for i in 0..4 {
            let weight = match i {
                0 => skin_weight.get_x(index),
                1 => skin_weight.get_y(index),
                2 => skin_weight.get_z(index),
                _ => skin_weight.get_w(index),
            };

            if weight == 0.0 {
                continue;
            }

            let bone_index = match i {
                0 => skin_index.get_x(index),
                1 => skin_index.get_y(index),
                2 => skin_index.get_z(index),
                _ => skin_index.get_w(index),
            } as usize;

            let mut offset_matrix = Matrix4::identity();
            offset_matrix.multiply_matrices(
                &skeleton.bones[bone_index].borrow().matrix_world,
                &skeleton.bone_inverses[bone_index],
            );

            let mut temp = base_position;
            temp.apply_matrix4(&offset_matrix);
            vector.add_scaled_vector(&temp, weight);
        }

        vector.apply_matrix4(&self.bind_matrix_inverse);
    }

    /// `SkinnedMesh.computeBoundingBox()`.
    pub fn compute_bounding_box(&mut self) {
        let Some(position) = self.mesh.geometry.position() else {
            return;
        };
        let count = position.count();

        let mut bounding_box = Box3::default();
        bounding_box.make_empty();

        for i in 0..count {
            let mut vertex = Vector3::new(position.get_x(i), position.get_y(i), position.get_z(i));
            self.apply_bone_transform(i, &mut vertex);
            bounding_box.expand_by_point(&vertex);
        }

        self.bounding_box = Some(bounding_box);
    }

    /// `SkinnedMesh.computeBoundingSphere()`.
    pub fn compute_bounding_sphere(&mut self) {
        let Some(position) = self.mesh.geometry.position() else {
            return;
        };
        let count = position.count();

        let mut bounding_sphere = Sphere::default();
        bounding_sphere.make_empty();

        for i in 0..count {
            let mut vertex = Vector3::new(position.get_x(i), position.get_y(i), position.get_z(i));
            self.apply_bone_transform(i, &mut vertex);
            bounding_sphere.expand_by_point(&vertex);
        }

        self.bounding_sphere = Some(bounding_sphere);
    }
}
