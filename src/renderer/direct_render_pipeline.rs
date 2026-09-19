//! Port of `three.js/src/renderers/common/DirectRenderPipeline.js`.
//!
//! A [`RenderPipeline`](super::RenderPipeline) draws the scene into a render
//! target and then runs the output transform on a quad of its own. A
//! `DirectRenderPipeline` has no target and no quad: it installs a `getOutput`
//! hook on the renderer, and every material's fragment shader ends with the
//! transform inlined — `Output = …; Output = …;` then the pipeline's own
//! `outputNode`, `renderOutput( … )` and all. That is what
//! `webgpu_postprocessing_direct` grades, and its dump has no intermediate
//! colour texture anywhere.
//!
//! It costs a framebuffer and buys a change in blending, which is why it is a
//! separate class rather than the default: every material now writes
//! display-referred, tone-mapped colour, so anything that blends against the
//! framebuffer blends in the wrong space, and a transmissive material that
//! samples the framebuffer reads the wrong thing.

use std::cell::RefCell;

use crate::cameras::RenderCamera;
use crate::materials::{render_output, OutputContext, ToneMapping};
use crate::math::Color;
use crate::nodes::node::Type;
use crate::nodes::tsl::{output_property, uniform_value};
use crate::nodes::NodeRef;
use crate::objects::{Background, Scene};

use super::Renderer;

/// What `DirectRenderPipeline.context` was last built from: the output
/// node's identity, `outputColorTransform` and the renderer's tone mapping.
type ContextKey = (usize, bool, ToneMapping);

pub struct DirectRenderPipeline {
    /// `renderPipeline.outputNode`, defaulting to `output` — the material's
    /// own result, which makes the pipeline a no-op apart from where the
    /// transform happens.
    pub output_node: Option<NodeRef>,
    /// `renderPipeline.outputColorTransform`, default true.
    pub output_color_transform: bool,
    /// `_contextNode`, and what it was built from: the output node's identity,
    /// `outputColorTransform` and the renderer's tone mapping, exactly as
    /// [`RenderPipeline`](super::RenderPipeline) caches its quad's fragment
    /// node. Rebuilding it every frame would give every material a new program
    /// every frame.
    context: RefCell<Option<(ContextKey, OutputContext)>>,
    /// `_backgroundNodes` — one uniform node per solid background colour, so
    /// the substituted node keeps its identity across frames and the
    /// background quad's program stays a cache hit.
    background_nodes: RefCell<Vec<(Color, NodeRef)>>,
}

impl Default for DirectRenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectRenderPipeline {
    pub fn new() -> Self {
        Self {
            output_node: None,
            output_color_transform: true,
            context: RefCell::new(None),
            background_nodes: RefCell::new(Vec::new()),
        }
    }

    /// `DirectRenderPipeline.render( scene, camera )`.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut dyn RenderCamera,
    ) {
        // `_update()` reads the renderer's tone mapping *before* `render()`
        // neutralises it, so the transform the materials bake in is the one
        // the application set.
        let hook = self.context(renderer.tone_mapping);

        // `_getBackgroundNode( scene )`: a solid `Color` background is
        // rendered by clearing, which no shader sees — so it would miss the
        // transform every material now applies. Swapping in `uniform( color )`
        // turns it into the background quad's `colorNode`, and the quad's
        // fragment carries the same tail.
        let background = self
            .background_node(scene)
            .map(|node| scene.background.replace(Background::Node(node)));

        let previous = renderer.set_output_hook(Some(hook));
        // `renderer.toneMapping = NoToneMapping; renderer.outputColorSpace =
        // workingColorSpace;` — with both neutral there is no framebuffer
        // target and no output pass, which is the point.
        renderer.with_neutral_output(|renderer| renderer.render(scene, camera));
        renderer.set_output_hook(previous);

        if let Some(background) = background {
            scene.background = background;
        }
    }

    /// `_updateContext()`'s `getOutput` closure, memoised.
    fn context(&self, tone_mapping: ToneMapping) -> OutputContext {
        // `new DirectRenderPipeline( renderer, outputNode = output )`.
        let output_node = self.output_node.clone().unwrap_or_else(output_property);
        let built_for = (output_node.key(), self.output_color_transform, tone_mapping);
        let mut cell = self.context.borrow_mut();
        if let Some((key, context)) = cell.as_ref() {
            if *key == built_for {
                return context.clone();
            }
        }
        let node = if self.output_color_transform {
            render_output(output_node, tone_mapping)
        } else {
            output_node
        };
        let context = OutputContext { node };
        *cell = Some((built_for, context.clone()));
        context
    }

    /// `_getBackgroundNode( scene )`: `None` unless the background is a solid
    /// colour, and the same node every time for the same colour.
    fn background_node(&self, scene: &Scene) -> Option<NodeRef> {
        let color = match &scene.background {
            Some(Background::Color(color)) => *color,
            _ => return None,
        };
        let mut nodes = self.background_nodes.borrow_mut();
        if let Some((_, node)) = nodes.iter().find(|(c, _)| *c == color) {
            return Some(node.clone());
        }
        // `uniform( scene.background )` — a `Color` uniform, which is the vec3
        // `object.nodeUniform0` of the dump's `Background.material`, not a
        // baked constant.
        let node = uniform_value(Type::Vec3, vec![color.r, color.g, color.b]);
        nodes.push((color, node.clone()));
        Some(node)
    }
}
