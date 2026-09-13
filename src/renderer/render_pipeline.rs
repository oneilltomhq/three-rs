//! Port of `three.js/src/renderers/common/RenderPipeline.js` (`PostProcessing`
//! is a deprecated subclass of it).
//!
//! A `RenderPipeline` is a `QuadMesh` whose material's `fragmentNode` is the
//! user's `outputNode` wrapped in `renderOutput( outputNode, toneMapping,
//! outputColorSpace )`. `render()` neutralises the renderer's tone mapping and
//! output colour space for the duration of the quad's draw, so
//! `Renderer.needsFrameBufferTarget` is false and the quad draws straight into
//! the canvas — the colour transform lives in the quad's own shader instead of
//! in a second output pass.

use crate::materials::{render_output, MeshBasicNodeMaterial, ToneMapping};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;

use super::Renderer;

pub struct RenderPipeline {
    /// `renderPipeline.outputNode`.
    pub output_node: Option<NodeRef>,
    /// `renderPipeline.outputColorTransform`, default true.
    pub output_color_transform: bool,
    quad_mesh: QuadMesh,
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderPipeline {
    pub fn new() -> Self {
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "RenderPipeline";
        Self {
            output_node: None,
            output_color_transform: true,
            quad_mesh: QuadMesh::new(material),
        }
    }

    /// `RenderPipeline.render()`.
    pub fn render(&mut self, renderer: &mut Renderer) {
        // `_updateContext()`.
        let output_node = self
            .output_node
            .clone()
            .expect("three-rs: RenderPipeline.outputNode is not set");

        self.quad_mesh.material.fragment_node = Some(if self.output_color_transform {
            // `renderer.toneMapping` is already `NoToneMapping` by the time
            // `RenderOutputNode.setup()` reads it: `PostProcessing.render()`
            // neutralises it around the whole quad render, compile included.
            render_output(output_node, ToneMapping::None)
        } else {
            output_node
        });

        // `renderer.toneMapping = NoToneMapping; renderer.outputColorSpace =
        // workingColorSpace;` — restored after the draw.
        let quad = &self.quad_mesh;
        renderer.with_neutral_output(|renderer| renderer.render_quad(quad));
    }
}
