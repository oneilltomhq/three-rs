//! Port of `three.js/src/scenes/Scene.js` (rung 2 subset).

use crate::core::Object3D;
use crate::lights::PointLight;
use crate::nodes::tsl::FogNode;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::objects::{InstancedBufferAttribute, InstancedMesh, Mesh};
use crate::textures::CubeTexture;

/// `Scene.background`. three.js accepts a `Color`, a `Texture` or a
/// `CubeTexture`; `Background.update()` branches on which one it is — a colour
/// becomes the clear value, anything else becomes the skybox mesh.
#[derive(Clone, Debug)]
pub enum Background {
    Color(Color),
    CubeTexture(CubeTexture),
}

impl From<Color> for Background {
    fn from(color: Color) -> Self {
        Background::Color(color)
    }
}

impl From<CubeTexture> for Background {
    fn from(texture: CubeTexture) -> Self {
        Background::CubeTexture(texture)
    }
}

/// One entry of `Object3D.children`. `InstancedMesh` extends `Mesh` in
/// three.js; the renderer walks children through the accessors below and only
/// looks at `instance_matrix()` to decide whether to draw instanced.
pub enum Child {
    Mesh(Mesh),
    InstancedMesh(InstancedMesh),
}

impl From<Mesh> for Child {
    fn from(mesh: Mesh) -> Self {
        Child::Mesh(mesh)
    }
}

impl From<InstancedMesh> for Child {
    fn from(mesh: InstancedMesh) -> Self {
        Child::InstancedMesh(mesh)
    }
}

impl Child {
    pub fn mesh(&self) -> &Mesh {
        match self {
            Child::Mesh(mesh) => mesh,
            Child::InstancedMesh(instanced) => &instanced.mesh,
        }
    }

    pub fn mesh_mut(&mut self) -> &mut Mesh {
        match self {
            Child::Mesh(mesh) => mesh,
            Child::InstancedMesh(instanced) => &mut instanced.mesh,
        }
    }

    pub fn object(&self) -> &Object3D {
        &self.mesh().object
    }

    pub fn object_mut(&mut self) -> &mut Object3D {
        &mut self.mesh_mut().object
    }

    /// The number of instances to draw: `InstancedMesh.count`, else 1.
    pub fn count(&self) -> u32 {
        match self {
            Child::Mesh(_) => 1,
            Child::InstancedMesh(instanced) => instanced.count as u32,
        }
    }

    pub fn instance_matrix(&self) -> Option<&InstancedBufferAttribute> {
        match self {
            Child::Mesh(_) => None,
            Child::InstancedMesh(instanced) => Some(&instanced.instance_matrix),
        }
    }
}

/// `Scene extends Object3D`. The children list is still flat (`Child`) rather
/// than the `Object3DNode` tree, because `src/renderer` walks it directly — see
/// `docs/scene-graph.md`.
pub struct Scene {
    pub object: Object3D,
    pub children: Vec<Child>,
    /// `scene.add( light )`. Lights are kept apart from the drawable children
    /// because the renderer needs them as a list (the `LightsNode` uniforms)
    /// rather than as draw calls — but their own children *are* drawn, which is
    /// why `drawables()` exists.
    pub lights: Vec<PointLight>,
    pub background: Option<Background>,
    /// `scene.fogNode`. Read by `NodeMaterial`'s output flow on every material
    /// in the scene; not yet consumed by the builder (rung 5, in progress).
    pub fog_node: Option<FogNode>,
    pub override_material: Option<MeshBasicNodeMaterial>,
}

impl Default for Scene {
    fn default() -> Self {
        let mut object = Object3D::default();
        object.object_type = "Scene";
        object.is_scene = true;

        Self {
            object,
            children: Vec::new(),
            lights: Vec::new(),
            background: None,
            fog_node: None,
            override_material: None,
        }
    }
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Scene.matrixWorld`.
    pub fn matrix_world(&self) -> Matrix4 {
        self.object.matrix_world
    }

    /// `scene.background = value`.
    pub fn set_background(&mut self, background: impl Into<Background>) {
        self.background = Some(background.into());
    }

    pub fn add(&mut self, child: impl Into<Child>) {
        self.children.push(child.into());
    }

    /// `scene.add( light )`.
    pub fn add_light(&mut self, light: PointLight) {
        self.lights.push(light);
    }

    /// Every mesh the renderer draws, in `scene.add()` order, with the lights'
    /// own children after the scene's direct ones. `matrixWorld` is already
    /// composed through the light by `update_matrix_world`, so the renderer
    /// needs no parent pointer here.
    pub fn drawables(&self) -> Vec<&Child> {
        let mut out: Vec<&Child> = self.children.iter().collect();
        for light in &self.lights {
            out.extend(light.children.iter());
        }
        out
    }

    /// `Object3D.updateMatrixWorld()` on the scene root: the scene's own world
    /// matrix stays the identity and each child is composed then multiplied by it.
    pub fn update_matrix_world(&mut self) {
        // three.js' `Object3D.updateMatrixWorld( force )`: a scene whose own
        // world matrix changed forces every child's to be recomputed.
        let force = self.object.update_matrix_world_forced(None, false);

        let parent = self.object.matrix_world;
        for child in &mut self.children {
            child
                .object_mut()
                .update_matrix_world_forced(Some(&parent), force);
        }
        for light in &mut self.lights {
            let light_force = light
                .object_mut()
                .update_matrix_world_forced(Some(&parent), force);
            let light_world = light.object().matrix_world;
            for child in &mut light.children {
                child
                    .object_mut()
                    .update_matrix_world_forced(Some(&light_world), light_force);
            }
        }
    }
}
