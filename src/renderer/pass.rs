//! Port of `three.js/src/nodes/display/PassNode.js`.
//!
//! `pass( scene, camera )` is a `TempNode` that owns a `RenderTarget`, renders
//! the given scene into it once per frame from `updateBefore()`, and evaluates
//! to the target's colour texture. Its `setup()` returns a `PassTextureNode`,
//! which is itself a `TempNode`, so a pass contributes **two** vars to the
//! generated shader — `nodeVarN = textureSample( … ); nodeVarN+1 = nodeVarN;` —
//! and `.a` is taken on the outer one. `to_var( texture_uv( … ) )` reproduces
//! that exactly.
//!
//! `PassTextureNode` also calls `setUpdateMatrix( false )`, so a pass samples
//! the raw `uv()` varying with no texture matrix — unlike `texture( map )`,
//! which carries a `mat3x3` in the object uniform block.

use crate::cameras::PerspectiveCamera;
use crate::nodes::tsl::{texture_uv, to_var, uv};
use crate::nodes::NodeRef;
use crate::objects::Scene;
use crate::textures::{DepthTexture, TextureFilter, TextureType};

use super::render_target::{RenderTarget, RenderTargetOptions};
use super::Renderer;

/// `pass( scene, camera )`.
///
/// Ownership divergence from three.js, documented in `docs/postprocessing.md`:
/// three.js discovers the pass nodes of a frame by collecting the graph's
/// `updateBefore` nodes while the quad's material is built, and fires them from
/// inside the quad's own render. Rust's ownership rules make a node that holds
/// `&mut Scene` across a frame impractical, so the port keeps the node and calls
/// [`PassNode::render`] explicitly, immediately before
/// [`RenderPipeline::render`](super::RenderPipeline::render). The GPU sees the
/// same order: in three.js the nested renders use their own command encoders and
/// are submitted before the canvas pass they are nested inside.
pub struct PassNode {
    render_target: RenderTarget,
    node: NodeRef,
}

impl Default for PassNode {
    fn default() -> Self {
        Self::new()
    }
}

impl PassNode {
    /// `new PassNode( PassNode.COLOR, scene, camera )`: a 1×1 `RenderTarget`
    /// with `{ type: HalfFloatType }` plus a `DepthTexture` named `depth`,
    /// resized to the drawing buffer on the first frame.
    pub fn new() -> Self {
        let render_target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                // `PassNode.setup()` then overwrites this with
                // `renderer.getOutputBufferType()`, which is `HalfFloatType`.
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer: true,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: PassNode's render target is a HalfFloat colour type");
        render_target.set_depth_texture(DepthTexture::new());

        let node = to_var(None, texture_uv(&render_target.texture(), uv()));

        Self {
            render_target,
            node,
        }
    }

    /// `passNode.getTextureNode()` — the node to compose with.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `pass( … ).a`.
    pub fn a(&self) -> NodeRef {
        self.node.a()
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> crate::textures::Texture {
        self.render_target.texture()
    }

    /// `PassNode.updateBefore( frame )`: size the target to the drawing buffer,
    /// then `renderer.setRenderTarget( this.renderTarget ); renderer.render(
    /// this.scene, this.camera )` with the previous target restored after.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut PerspectiveCamera,
    ) {
        let (width, height) = renderer.drawing_buffer_size();
        self.render_target.set_size(width, height);
        // `PassNode.setup()`: `renderTarget.samples = renderer.samples`.
        self.render_target.set_samples(renderer.samples());

        let previous = renderer.render_target();
        renderer.set_render_target(Some(self.render_target.clone()));
        renderer.render(scene, camera);
        renderer.set_render_target(previous);
    }
}
