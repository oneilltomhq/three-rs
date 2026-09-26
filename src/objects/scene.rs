//! Port of `three.js/src/scenes/Scene.js` (rung 2 subset).

use crate::core::{Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Matrix4};
use crate::nodes::tsl::FogNode;
use crate::textures::CubeTexture;

/// `Scene.background`. three.js accepts a `Color`, a `Texture` or a
/// `CubeTexture`; `Background.update()` branches on which one it is — a colour
/// becomes the clear value, anything else becomes the skybox mesh.
#[derive(Clone, Debug)]
pub enum Background {
    Color(Color),
    CubeTexture(CubeTexture),
    /// `scene.backgroundNode = …` — `Background.update()`'s
    /// `background.isNode` branch, which draws the skybox sphere with
    /// `vec4( node ).mul( backgroundIntensity )` instead of clearing. Held in
    /// the same slot because `nodes.getBackgroundNode( scene ) ||
    /// scene.background` makes the node win over a plain background.
    ///
    /// Any node, not just a `color()`: `webgpu_skinning` uses
    /// `screenUV.y.mix( color( 0x66bbff ), color( 0x4466ff ) )`.
    Node(crate::nodes::NodeRef),
    /// `scene.background = <the texture of a generated PMREM>`.
    ///
    /// Upstream this is the PMREM cube render target's texture, flagged
    /// `isPMREMTexture`; `NodeManager.getBackgroundNode()` turns it into
    /// `pmremTexture( background )` and `Background.update()` wraps that in
    /// the node branch's context — `getUV` is `backgroundRotation.mul(
    /// normalWorldGeometry )` and `getTextureLevel` is `backgroundBlurriness`.
    /// The port carries the handle rather than the texture, because the
    /// `maxLod` uniform the read needs travels with it; the renderer builds the
    /// node, so an application writes the one line the page does.
    Pmrem(crate::materials::environment::PmremHandle),
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
    /// `scene.backgroundBlurriness` — the roughness the background's PMREM is
    /// read at, in `[ 0, 1 ]`. Three reads it through `backgroundBlurriness`,
    /// the render-group uniform `BackgroundNode` puts on `getTextureLevel`, so
    /// changing it is a uniform write and not a new program.
    pub background_blurriness: f64,
    /// `scene.environment` — the default environment map for every material in
    /// the scene that does not carry one of its own.
    ///
    /// Upstream this is a plain `Texture` and
    /// `NodeManager.updateEnvironment( scene )` turns it into a
    /// `scene.environmentNode`, which `NodeMaterial.setupEnvironment()` falls
    /// back to when neither `builder.context.environment` nor the material's
    /// own `envNode` is set. The port carries the generated PMREM's handle for
    /// the reason [`Background::Pmrem`] does: the `maxLod` uniform the read needs
    /// travels with the texture.
    ///
    /// Assigning it after a material has drawn once needs
    /// [`MeshBasicNodeMaterial::set_needs_update`] on that material, exactly as
    /// three.js needs `material.needsUpdate = true`: it changes the program,
    /// not a uniform.
    ///
    /// [`MeshBasicNodeMaterial::set_needs_update`]: crate::materials::MeshBasicNodeMaterial::set_needs_update
    pub environment: Option<crate::materials::environment::PmremHandle>,
    /// `scene.fog` — a classic [`Fog`](super::Fog) or
    /// [`FogExp2`](super::FogExp2). The renderer turns it into a fog node
    /// whose parameters are render-group uniforms (`NodeManager.updateFog()`),
    /// so changing a value is a uniform write; changing the *kind* is a new
    /// program. [`Scene::fog_node`] wins when both are set, as in three.js.
    pub fog: Option<super::SceneFog>,
    /// `scene.fogNode`. Read by `NodeMaterial`'s output flow on every material
    /// in the scene (rung 5).
    pub fog_node: Option<FogNode>,
    pub override_material: Option<MeshBasicNodeMaterial>,
}

impl Default for Scene {
    fn default() -> Self {
        let object = Object3D {
            object_type: "Scene",
            is_scene: true,
            ..Default::default()
        };

        Self {
            node: object.into_node(),
            background: None,
            background_blurriness: 0.0,
            environment: None,
            fog: None,
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
