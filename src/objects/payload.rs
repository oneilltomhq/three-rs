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

use crate::lights::LightPayload;
use crate::objects::{InstancedBufferAttribute, InstancedMesh, Mesh};

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
    /// `PointLight extends Light extends Object3D`. The renderer reaches it
    /// through `RenderList.lights`, which `_projectObject()` fills from
    /// `object.is_light` — set alongside this variant.
    Light(LightPayload),
}

impl fmt::Debug for Payload {
    /// `Object3D` derives `Debug`; geometries, materials and node graphs do not,
    /// so a payload prints as its variant.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Payload::None => "None",
            Payload::Mesh(_) => "Mesh",
            Payload::InstancedMesh(_) => "InstancedMesh",
            Payload::Light(_) => "Light",
        };
        f.write_str(name)
    }
}

impl Payload {
    /// `object.isMesh` — true for `InstancedMesh` too, exactly as in three.js
    /// where `InstancedMesh extends Mesh`.
    pub fn is_mesh(&self) -> bool {
        matches!(self, Payload::Mesh(_) | Payload::InstancedMesh(_))
    }

    /// `object.isInstancedMesh`.
    pub fn is_instanced_mesh(&self) -> bool {
        matches!(self, Payload::InstancedMesh(_))
    }

    /// The `Mesh` half of whichever mesh-ish payload this is.
    pub fn mesh(&self) -> Option<&Mesh> {
        match self {
            Payload::Mesh(mesh) => Some(mesh),
            Payload::InstancedMesh(instanced) => Some(&instanced.mesh),
            _ => None,
        }
    }

    pub fn mesh_mut(&mut self) -> Option<&mut Mesh> {
        match self {
            Payload::Mesh(mesh) => Some(mesh),
            Payload::InstancedMesh(instanced) => Some(&mut instanced.mesh),
            _ => None,
        }
    }

    /// The light this node is, if it is one.
    pub fn light(&self) -> Option<&LightPayload> {
        match self {
            Payload::Light(light) => Some(light),
            _ => None,
        }
    }

    pub fn light_mut(&mut self) -> Option<&mut LightPayload> {
        match self {
            Payload::Light(light) => Some(light),
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

    /// The number of instances the renderer draws: `InstancedMesh.count`, else 1.
    pub fn count(&self) -> u32 {
        match self {
            Payload::InstancedMesh(instanced) => instanced.count as u32,
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
}
