//! Port of `three.js/src/objects/Mesh.js` (rung 1 subset).

use std::rc::Rc;

use crate::core::{BufferGeometry, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Matrix4;
use crate::objects::Payload;

/// The state `Mesh` adds to `Object3D`. It lives in a node's
/// [`Payload`](crate::objects::Payload) rather than wrapping an `Object3D`,
/// because the renderer finds meshes by walking the tree — see
/// `docs/scene-graph.md`.
#[derive(Clone)]
pub struct Mesh {
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
    /// `Mesh.morphTargetInfluences` — one weight per `morphAttributes.position`
    /// entry, filled in by `updateMorphTargets()` from the constructor.
    pub morph_target_influences: Vec<f64>,
}

impl Mesh {
    /// `new Mesh( geometry, material )`, as a scene-graph [`Node`].
    ///
    /// The material is `impl Into<Option<MeshBasicNodeMaterial>>`, so a call
    /// passes the material by value as `Line::new` and `InstancedMesh::new` do,
    /// or `None` where three.js leaves `material` undefined and the renderer
    /// falls back to a default white `MeshBasicMaterial` — which is what
    /// `webgpu_postprocessing_masking` and `webgpu_depth_texture` rely on,
    /// together with `scene.overrideMaterial`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: impl Into<Option<MeshBasicNodeMaterial>>,
    ) -> Node {
        let mut object = Object3D {
            object_type: "Mesh",
            ..Default::default()
        };
        // `this.updateMorphTargets()`: `morphTargetInfluences` gets one 0 per
        // morph attribute of the first key in `geometry.morphAttributes`.
        let morph_target_influences = match geometry.morph_attributes().next() {
            Some((_, attributes)) => vec![0.0; attributes.len()],
            None => Vec::new(),
        };

        object.payload = Payload::Mesh(Self {
            geometry,
            material: material.into(),
            morph_target_influences,
        });
        object.into_node()
    }

    /// `Mesh.intersectsFrustum( frustum )` needs the world matrix, which lives on
    /// the node; this is the geometry half, i.e. the bounding sphere three.js
    /// lazily computes in `_projectObject`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<crate::math::Sphere> {
        let bounding_sphere = self.geometry.compute_bounding_sphere()?;
        let mut sphere = crate::math::Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(matrix_world);
        Some(sphere)
    }
}
