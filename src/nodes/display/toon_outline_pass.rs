//! Port of `three.js/src/nodes/display/ToonOutlinePassNode.js` — a scene pass
//! that draws every `MeshToonNodeMaterial` object twice: first with a
//! back-side material whose vertices are pushed out along the clip-space
//! normal, which leaves a dark rim around the silhouette, then as itself.
//!
//! Three subclasses `PassNode` and swaps the renderer's render-object function
//! around `super.updateBefore()`; the port wraps a
//! [`PassNode`](crate::renderer::PassNode) and sets the renderer's
//! `toon_outline` hook around [`ToonOutlinePassNode::render`] instead, which
//! the render loop reads per draw exactly where three calls the function. See
//! `docs/nodes.md` §42.

use std::rc::Rc;

use crate::cameras::RenderCamera;
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::math::Color;
use crate::nodes::tsl::{
    camera_projection_matrix, float, model_view_matrix, normal_local, position_local, vec3,
    vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::Scene;
use crate::renderer::{PassNode, Renderer};

/// `toonOutlinePass( scene, camera, color, thickness, alpha )`.
pub struct ToonOutlinePassNode {
    pass: PassNode,
    /// `_createMaterial()`'s result. Three builds one per source material
    /// (`_materialCache`), every one from the same three nodes, so they differ
    /// only in identity; the port keeps the one template and lets the render
    /// loop key each draw's copy on the source material instead.
    material: Rc<MeshBasicNodeMaterial>,
}

/// `toonOutlinePass( scene, camera, color = new Color( 0, 0, 0 ), thickness =
/// 0.003, alpha = 1 )` — the page's call, with every default. The scene and
/// camera are handed to [`ToonOutlinePassNode::render`] instead, as they are
/// for [`PassNode`].
pub fn toon_outline_pass() -> ToonOutlinePassNode {
    ToonOutlinePassNode::new(Color::new(0.0, 0.0, 0.0), float(0.003), float(1.0))
}

impl ToonOutlinePassNode {
    /// `new ToonOutlinePassNode( scene, camera, nodeObject( color ),
    /// nodeObject( thickness ), nodeObject( alpha ) )`.
    pub fn new(color: Color, thickness_node: NodeRef, alpha_node: NodeRef) -> Self {
        let color_node = vec3(color.r, color.g, color.b);
        Self {
            pass: PassNode::new(),
            material: Rc::new(Self::create_material(
                color_node,
                thickness_node,
                alpha_node,
            )),
        }
    }

    /// `ToonOutlinePassNode._createMaterial()`.
    ///
    /// `new NodeMaterial()` — not a `MeshBasicNodeMaterial`, so `lights` stays
    /// false and the fragment flow is the bare `DiffuseColor` one the dump
    /// shows (`m03`). The port's `Basic` kind with its default `lights =
    /// false` is exactly that flow.
    fn create_material(
        color_node: NodeRef,
        thickness_node: NodeRef,
        alpha_node: NodeRef,
    ) -> MeshBasicNodeMaterial {
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "Toon_Outline";
        material.side = Side::Back;

        // vertex node

        let outline_normal = normal_local().negate();
        let mvp = camera_projection_matrix().mul(model_view_matrix());

        // "TODO: support outline thickness ratio for each vertex"
        let ratio = float(1.0);
        let pos = mvp
            .clone()
            .mul(vec4_join(vec![position_local(), float(1.0)]));
        let pos2 = mvp.mul(vec4_join(vec![
            position_local().add(outline_normal),
            float(1.0),
        ]));
        // "subtract pos2 from pos because BackSide objectNormal is negative"
        let norm = pos.clone().sub(pos2).normalize();

        material.vertex_node = Some(
            pos.clone()
                .add(norm.mul(thickness_node).mul(pos.w()).mul(ratio)),
        );

        // color node

        material.color_node = Some(vec4_join(vec![color_node, alpha_node]));

        material
    }

    /// The node for the graph downstream — `RenderPipeline.outputNode`.
    pub fn node(&self) -> NodeRef {
        self.pass.node()
    }

    /// The outline material every toon draw is preceded by — what
    /// `_getOutlineMaterial()` returns, for inspecting its program
    /// (`examples/dump_wgsl.rs`).
    pub fn outline_material(&self) -> &MeshBasicNodeMaterial {
        &self.material
    }

    /// The wrapped `PassNode`.
    pub fn pass(&self) -> &PassNode {
        &self.pass
    }

    /// `ToonOutlinePassNode.updateBefore( frame )`: install the outline
    /// render-object function, run `PassNode.updateBefore()`, and put the
    /// previous function back.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut dyn RenderCamera,
    ) {
        let previous = renderer.toon_outline.replace(self.material.clone());
        self.pass.render_scene(renderer, scene, camera);
        renderer.toon_outline = previous;
    }
}
