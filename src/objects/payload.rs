//! What a scene-graph node *is*, beyond its transform.
//!
//! three.js gets this from subclassing: `Mesh extends Object3D` adds `geometry`
//! and `material`, and `Renderer._projectObject()` branches on `object.isMesh`.
//! Rust has no inheritance, so the subclass' extra state is a variant of
//! [`Payload`] held by [`crate::core::Object3D`], and `isMesh` is a `match`.
//!
//! This replaces the flat `Scene.children: Vec<Child>` list the renderer used to
//! iterate: a payload lives on the node itself, so a `Mesh` can sit anywhere in
//! the tree (under a `Group`, under a light) and the renderer's traversal finds
//! it. See `docs/scene-graph.md`.

use std::fmt;

use std::rc::Rc;

use crate::core::BufferGeometry;
use crate::lights::LightObject;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Matrix4, Sphere};
use crate::objects::{
    BatchedMesh, InstancedBufferAttribute, InstancedMesh, Line, Mesh, Points, SkinnedMesh, Sprite,
};

/// The subclass state of one [`crate::core::Object3D`].
///
/// `None` is a plain `Object3D`, a `Group`, a `Bone` — anything the renderer
/// walks through without drawing.
#[derive(Clone, Default)]
pub enum Payload {
    #[default]
    None,
    /// `Mesh.geometry` / `Mesh.material`.
    Mesh(Mesh),
    /// `InstancedMesh extends Mesh`.
    InstancedMesh(InstancedMesh),
    /// `SkinnedMesh extends Mesh` — the skeleton and the two bind matrices on
    /// top of the `Mesh`. `is_mesh()` is true for it, as `isMesh` is in
    /// three.js. Boxed because the two bind matrices and the bounding volumes
    /// make it half again the size of the next-largest variant, and every
    /// `Object3D` in the tree carries a `Payload`.
    SkinnedMesh(Box<SkinnedMesh>),
    /// `BatchedMesh extends Mesh` — boxed because it is much the largest
    /// variant (three data textures and two draw-list arrays).
    BatchedMesh(Box<BatchedMesh>),
    /// `Line extends Object3D` — and `LineSegments extends Line`, which is the
    /// `is_line_segments` flag inside. Not a `Mesh`: the renderer reads the
    /// object to pick `line-strip` / `line-list` over `triangle-list`.
    Line(Line),
    /// `Points extends Object3D` — like `Line`, not a `Mesh`: the renderer
    /// reads the object to pick `point-list` over `triangle-list`.
    Points(Points),
    /// `Sprite extends Object3D` — not a `Mesh` either: `_projectObject()`
    /// gives it an arm of its own, culled by `Frustum.intersectsSprite()` and
    /// sorted by its origin rather than by a geometry bounding sphere.
    Sprite(Sprite),
    /// `Light extends Object3D`, one variant per subclass. The renderer reaches
    /// it through `RenderList.lights`, which `_projectObject()` fills from
    /// `object.is_light` — set alongside this variant.
    Light(LightObject),
}

impl fmt::Debug for Payload {
    /// `Object3D` derives `Debug`; geometries, materials and node graphs do not,
    /// so a payload prints as its variant.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Payload::None => "None",
            Payload::Mesh(_) => "Mesh",
            Payload::InstancedMesh(_) => "InstancedMesh",
            Payload::SkinnedMesh(_) => "SkinnedMesh",
            Payload::BatchedMesh(_) => "BatchedMesh",
            Payload::Line(line) => {
                if line.is_line_segments {
                    "LineSegments"
                } else {
                    "Line"
                }
            }
            Payload::Points(_) => "Points",
            Payload::Sprite(_) => "Sprite",
            Payload::Light(_) => "Light",
        };
        f.write_str(name)
    }
}

impl Payload {
    /// `object.isMesh` — true for `InstancedMesh` too, exactly as in three.js
    /// where `InstancedMesh extends Mesh`.
    pub fn is_mesh(&self) -> bool {
        matches!(
            self,
            Payload::Mesh(_)
                | Payload::InstancedMesh(_)
                | Payload::SkinnedMesh(_)
                | Payload::BatchedMesh(_)
        )
    }

    /// `object.isSkinnedMesh`.
    pub fn is_skinned_mesh(&self) -> bool {
        matches!(self, Payload::SkinnedMesh(_))
    }

    /// The `SkinnedMesh` this node is, if it is one.
    pub fn skinned_mesh(&self) -> Option<&SkinnedMesh> {
        match self {
            Payload::SkinnedMesh(skin) => Some(skin),
            _ => None,
        }
    }

    pub fn skinned_mesh_mut(&mut self) -> Option<&mut SkinnedMesh> {
        match self {
            Payload::SkinnedMesh(skin) => Some(skin),
            _ => None,
        }
    }

    /// `object.isInstancedMesh`.
    pub fn is_instanced_mesh(&self) -> bool {
        matches!(self, Payload::InstancedMesh(_))
    }

    /// `object.isLine` — true for a `LineSegments` too, exactly as in three.js
    /// where `LineSegments extends Line`.
    pub fn is_line(&self) -> bool {
        matches!(self, Payload::Line(_))
    }

    /// `object.isLineSegments`.
    pub fn is_line_segments(&self) -> bool {
        matches!(self, Payload::Line(line) if line.is_line_segments)
    }

    /// `object.isPoints`.
    pub fn is_points(&self) -> bool {
        matches!(self, Payload::Points(_))
    }

    /// `object.isSprite`.
    pub fn is_sprite(&self) -> bool {
        matches!(self, Payload::Sprite(_))
    }

    /// The `Sprite` this node is, if it is one.
    pub fn sprite(&self) -> Option<&Sprite> {
        match self {
            Payload::Sprite(sprite) => Some(sprite),
            _ => None,
        }
    }

    pub fn sprite_mut(&mut self) -> Option<&mut Sprite> {
        match self {
            Payload::Sprite(sprite) => Some(sprite),
            _ => None,
        }
    }

    /// The `Points` this node is, if it is one.
    pub fn points(&self) -> Option<&Points> {
        match self {
            Payload::Points(points) => Some(points),
            _ => None,
        }
    }

    pub fn points_mut(&mut self) -> Option<&mut Points> {
        match self {
            Payload::Points(points) => Some(points),
            _ => None,
        }
    }

    /// The `Line` this node is, if it is one.
    pub fn line(&self) -> Option<&Line> {
        match self {
            Payload::Line(line) => Some(line),
            _ => None,
        }
    }

    pub fn line_mut(&mut self) -> Option<&mut Line> {
        match self {
            Payload::Line(line) => Some(line),
            _ => None,
        }
    }

    /// `object.geometry` for anything `_projectObject()`'s
    /// `isMesh || isLine || isPoints` arm draws, and for its `isSprite` arm.
    pub fn geometry(&self) -> Option<&Rc<BufferGeometry>> {
        match self {
            Payload::Mesh(mesh) => Some(&mesh.geometry),
            Payload::InstancedMesh(instanced) => Some(&instanced.mesh.geometry),
            Payload::SkinnedMesh(skin) => Some(&skin.mesh.geometry),
            Payload::BatchedMesh(batched) => Some(&batched.mesh.geometry),
            Payload::Line(line) => Some(&line.geometry),
            Payload::Points(points) => Some(&points.geometry),
            Payload::Sprite(sprite) => Some(&sprite.geometry),
            _ => None,
        }
    }

    /// `object.material` for the same set. `None` is three.js' "no material of
    /// its own", which `scene.overrideMaterial` (or `MeshBasicNodeMaterial`'s
    /// defaults) stands in for.
    pub fn material(&self) -> Option<&MeshBasicNodeMaterial> {
        match self {
            Payload::Mesh(mesh) => mesh.material.as_ref(),
            Payload::InstancedMesh(instanced) => instanced.mesh.material.as_ref(),
            Payload::SkinnedMesh(skin) => skin.mesh.material.as_ref(),
            Payload::BatchedMesh(batched) => batched.mesh.material.as_ref(),
            Payload::Line(line) => line.material.as_ref(),
            Payload::Points(points) => points.material.as_ref(),
            Payload::Sprite(sprite) => Some(&sprite.material),
            _ => None,
        }
    }

    /// `Frustum.intersectsObject( object )`' geometry half, for a mesh or a
    /// line.
    ///
    /// three.js' `intersectsObject` reads `object.boundingSphere` when the
    /// object has one and falls back to `geometry.boundingSphere` otherwise;
    /// [`InstancedMesh::bounding_sphere`] is that field, and it is the only way
    /// an instanced draw whose instances are spread out is not culled as a
    /// point at its own origin.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<Sphere> {
        match self {
            Payload::Line(line) => line.bounding_sphere_in(matrix_world),
            Payload::Points(points) => points.bounding_sphere_in(matrix_world),
            // `Sprite.intersectsFrustum( frustum )` is
            // `frustum.intersectsSprite( this )`, whose sphere is this.
            Payload::Sprite(sprite) => Some(sprite.bounding_sphere_in(matrix_world)),
            Payload::InstancedMesh(instanced) => match instanced.bounding_sphere {
                Some(bounding_sphere) => {
                    let mut sphere = bounding_sphere;
                    sphere.apply_matrix4(matrix_world);
                    Some(sphere)
                }
                None => instanced.mesh.bounding_sphere_in(matrix_world),
            },
            // `Frustum.intersectsObject` prefers the object's own
            // `boundingSphere` — `SkinnedMesh.computeBoundingSphere()` puts the
            // posed skin there — and falls back to the geometry's.
            Payload::SkinnedMesh(skin) => match skin.bounding_sphere {
                Some(bounding_sphere) => {
                    let mut sphere = bounding_sphere;
                    sphere.apply_matrix4(matrix_world);
                    Some(sphere)
                }
                None => skin.mesh.bounding_sphere_in(matrix_world),
            },
            _ => self.mesh()?.bounding_sphere_in(matrix_world),
        }
    }

    /// `Mesh.morphTargetInfluences`. A `Line` has the field in three.js too, but
    /// nothing on the ladder morphs one, so it is always empty here. A
    /// `SkinnedMesh` keeps its influences behind an `Rc<RefCell<…>>` — the
    /// animation binding writes into them from outside — so this returns a
    /// value rather than a borrow.
    pub fn morph_target_influences(&self) -> Vec<f64> {
        match self {
            Payload::SkinnedMesh(skin) => skin.morph_target_influences.borrow().clone(),
            _ => match self.mesh() {
                Some(mesh) => mesh.morph_target_influences.clone(),
                None => Vec::new(),
            },
        }
    }

    /// The `Mesh` half of whichever mesh-ish payload this is.
    pub fn mesh(&self) -> Option<&Mesh> {
        match self {
            Payload::Mesh(mesh) => Some(mesh),
            Payload::InstancedMesh(instanced) => Some(&instanced.mesh),
            Payload::SkinnedMesh(skin) => Some(&skin.mesh),
            Payload::BatchedMesh(batched) => Some(&batched.mesh),
            _ => None,
        }
    }

    pub fn mesh_mut(&mut self) -> Option<&mut Mesh> {
        match self {
            Payload::Mesh(mesh) => Some(mesh),
            Payload::InstancedMesh(instanced) => Some(&mut instanced.mesh),
            Payload::SkinnedMesh(skin) => Some(&mut skin.mesh),
            Payload::BatchedMesh(batched) => Some(&mut batched.mesh),
            _ => None,
        }
    }

    /// The light this node is, if it is one.
    pub fn light(&self) -> Option<&LightObject> {
        match self {
            Payload::Light(light) => Some(light),
            _ => None,
        }
    }

    pub fn light_mut(&mut self) -> Option<&mut LightObject> {
        match self {
            Payload::Light(light) => Some(light),
            _ => None,
        }
    }

    /// `object.isBatchedMesh`.
    pub fn is_batched_mesh(&self) -> bool {
        matches!(self, Payload::BatchedMesh(_))
    }

    pub fn batched_mesh(&self) -> Option<&BatchedMesh> {
        match self {
            Payload::BatchedMesh(batched) => Some(batched),
            _ => None,
        }
    }

    pub fn batched_mesh_mut(&mut self) -> Option<&mut BatchedMesh> {
        match self {
            Payload::BatchedMesh(batched) => Some(batched),
            _ => None,
        }
    }

    pub fn instanced_mesh(&self) -> Option<&InstancedMesh> {
        match self {
            Payload::InstancedMesh(instanced) => Some(instanced),
            _ => None,
        }
    }

    pub fn instanced_mesh_mut(&mut self) -> Option<&mut InstancedMesh> {
        match self {
            Payload::InstancedMesh(instanced) => Some(instanced),
            _ => None,
        }
    }

    /// `RenderObject.getInstanceCount()` (`RenderObject.js:617-631`): the
    /// instanced geometry's count, else `object.count` when the object defines
    /// one, else 1. `Points.count` is the second arm — see [`Points::count`].
    pub fn count(&self) -> u32 {
        match self {
            Payload::InstancedMesh(instanced) => instanced.count as u32,
            Payload::Points(points) => points.count.unwrap_or(1) as u32,
            // `RenderObject.getInstanceCount()`: an instanced geometry's
            // `instanceCount` wins over `object.count`. A `LineSegments2`'s
            // geometry is one, and its count is the number of segments.
            Payload::Mesh(mesh) => match &mesh.line_segments {
                Some(attributes) => attributes.instance_count() as u32,
                None => mesh.count as u32,
            },
            _ => 1,
        }
    }

    /// `InstancedMesh.instanceMatrix`, `None` for a plain mesh.
    pub fn instance_matrix(&self) -> Option<&InstancedBufferAttribute> {
        match self {
            Payload::InstancedMesh(instanced) => Some(&instanced.instance_matrix),
            _ => None,
        }
    }

    /// A `LineSegments2`'s instanced attributes, `None` for anything else.
    pub fn line_segments(&self) -> Option<&crate::nodes::lines::LineSegmentsAttributes> {
        match self {
            Payload::Mesh(mesh) => mesh.line_segments.as_ref(),
            _ => None,
        }
    }

    /// `InstancedMesh.instanceColor` — `None` until `setColorAt()` is called,
    /// and `None` for a plain mesh.
    pub fn instance_color(&self) -> Option<&InstancedBufferAttribute> {
        match self {
            Payload::InstancedMesh(instanced) => instanced.instance_color.as_ref(),
            _ => None,
        }
    }
}
