//! Port of `three.js/src/scenes/Scene.js` (rung 2 subset).

use crate::core::{Node, Object3D, Object3DNode};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
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

/// `Scene extends Object3D`.
///
/// The `Object3D` half is a real scene-graph [`Node`], so the tree under a scene
/// is the tree the renderer walks: `Group`s, lights and their children all
/// nest, and `Renderer::render` collects drawables with
/// [`crate::renderer::project_object`]. The fields below are what `Scene` adds
/// to `Object3D`; they are not a `Payload` variant because nothing in the
/// renderer's traversal branches on them.
pub struct Scene {
    /// The scene root. `node.borrow().is_scene` is true.
    pub node: Node,
    pub background: Option<Background>,
    pub override_material: Option<MeshBasicNodeMaterial>,
}

impl Default for Scene {
    fn default() -> Self {
        let mut object = Object3D::default();
        object.object_type = "Scene";
        object.is_scene = true;

        Self {
            node: object.into_node(),
            background: None,
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
        self.node.borrow().matrix_world
    }

    /// `scene.background = value`.
    pub fn set_background(&mut self, background: impl Into<Background>) {
        self.background = Some(background.into());
    }

    /// `scene.add( object )`.
    pub fn add(&self, object: &Node) -> &Self {
        self.node.add(object);
        self
    }

    /// `scene.remove( object )`.
    pub fn remove(&self, object: &Node) -> &Self {
        self.node.remove(object);
        self
    }

    /// `scene.children`, cloned — the direct children only. The renderer does
    /// not use this; it walks the whole tree.
    pub fn children(&self) -> Vec<Node> {
        self.node.children()
    }

    /// `Object3D.updateMatrixWorld()` on the scene root, which is what
    /// `Renderer.render()` calls before projecting the scene.
    pub fn update_matrix_world(&self) {
        self.node.update_matrix_world(false);
    }
}
