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
//!
//! Before/after hooks ([`RenderPipeline::on_before_render`],
//! [`RenderPipeline::on_after_render`]) are three's `OnBeforeRenderPipeline` /
//! `OnAfterRenderPipeline` events: closures run around the quad's draw, which
//! is where `TRAANode` jitters the camera and clears the jitter again.

use crate::materials::{render_output, MeshBasicNodeMaterial, ToneMapping};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;

use super::Renderer;

/// A render-pipeline hook: three's `OnBeforeRenderPipeline( callback )` /
/// `OnAfterRenderPipeline( callback )` callback. It is handed the renderer; a
/// hook that jitters a camera captures that camera itself (three's TRAA
/// closes over `this`, which holds its camera).
type RenderPipelineHook = Box<dyn FnMut(&mut Renderer)>;

pub struct RenderPipeline {
    /// `renderPipeline.outputNode`.
    pub output_node: Option<NodeRef>,
    /// `renderPipeline.outputColorTransform`, default true.
    pub output_color_transform: bool,
    quad_mesh: QuadMesh,
    /// What the quad material's `fragmentNode` was last built from: the
    /// `outputNode`'s identity, `outputColorTransform` and the renderer's
    /// tone mapping (`_update()` compares `this._toneMapping !==
    /// this.renderer.toneMapping` and sets `needsUpdate`). `_updateContext()`
    /// assigns the node every render; here the assignment — a fresh
    /// `renderOutput( … )` graph — happens only when either changed, with
    /// `needsUpdate` set alongside, so a steady frame's program is a cache
    /// hit rather than a rebuild.
    built_for: Option<(usize, bool, ToneMapping)>,
    /// `_contextData.onBeforePipelineCallbacks`.
    before_render: Vec<RenderPipelineHook>,
    /// `_contextData.onAfterPipelineCallbacks`.
    after_render: Vec<RenderPipelineHook>,
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
            built_for: None,
            before_render: Vec::new(),
            after_render: Vec::new(),
        }
    }

    /// `OnBeforeRenderPipeline( callback )`: run `hook` at the start of every
    /// [`render`](Self::render), after the output node is (re)assigned and
    /// before the renderer's tone mapping is neutralised. Hooks run in the
    /// order they were added.
    ///
    /// three collects these from `EventNode`s while the pipeline's quad
    /// material builds, into a context that `_updateContext()` recreates, so a
    /// new `outputNode` drops the old hooks and its own nodes register fresh
    /// ones. The port has no builder context to collect into: the node that
    /// needs a hook (TRAA, issue #165) adds it here when it is built, and it
    /// stays for the pipeline's life.
    pub fn on_before_render(&mut self, hook: RenderPipelineHook) {
        self.before_render.push(hook);
    }

    /// `OnAfterRenderPipeline( callback )`: run `hook` at the end of every
    /// [`render`](Self::render), after the renderer's tone mapping and output
    /// colour space are restored. Hooks run in the order they were added.
    pub fn on_after_render(&mut self, hook: RenderPipelineHook) {
        self.after_render.push(hook);
    }

    /// `RenderPipeline.render()`.
    pub fn render(&mut self, renderer: &mut Renderer) {
        // `_updateContext()`.
        let output_node = self
            .output_node
            .clone()
            .expect("three-rs: RenderPipeline.outputNode is not set");

        // `_update()` reads the renderer's tone mapping *before*
        // `render()` neutralises it, so the transform the quad bakes in is
        // the one the application set — that is how the radial-blur example's
        // `NeutralToneMapping` reaches the post-processing shader.
        let tone_mapping = renderer.tone_mapping;
        let built_for = (output_node.key(), self.output_color_transform, tone_mapping);
        if self.built_for != Some(built_for) {
            self.quad_mesh.material.fragment_node = Some(if self.output_color_transform {
                render_output(output_node, tone_mapping)
            } else {
                output_node
            });
            self.quad_mesh.material.set_needs_update();
            self.built_for = Some(built_for);
        }

        for hook in &mut self.before_render {
            hook(renderer);
        }

        // `renderer.toneMapping = NoToneMapping; renderer.outputColorSpace =
        // workingColorSpace;` — restored after the draw.
        let quad = &self.quad_mesh;
        renderer.with_neutral_output(|renderer| renderer.render_quad(quad));

        for hook in &mut self.after_render {
            hook(renderer);
        }
    }
}
