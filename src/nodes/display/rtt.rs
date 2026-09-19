//! Port of `three.js/src/nodes/utils/RTTNode.js` — `rtt()` and
//! `convertToTexture()`.
//!
//! An `RTTNode` is a texture node whose texture is a render target it owns and
//! fills itself: the node it was handed becomes a full-screen quad's
//! `fragmentNode`, the quad is drawn into the target once a frame, and
//! everything downstream samples the result. It is what turns a value that
//! would otherwise be recomputed per tap — `webgpu_postprocessing_anamorphic`'s
//! bright pass, read eighty times by one horizontal blur — into a single texture
//! read.
//!
//! Two shapes worth knowing before reading the code:
//!
//! * **It is a `TextureNode`, not a pass.** `super( renderTarget.texture, uv()
//!   )` hands the base class a non-null uv node, so `setUpdateMatrix( uvNode
//!   === null )` leaves the uv matrix **off**: an `rtt()` tap carries no
//!   `mat3x3` uniform, unlike [`texture_sample`](crate::nodes::tsl::texture_sample).
//!   That is why [`RttNode::sample`] is built on
//!   [`texture_uv`](crate::nodes::tsl::texture_uv).
//! * **The target keeps a depth buffer.** `new RenderTarget( w, h, { type:
//!   HalfFloatType, ...options } )` — `depthBuffer` is not in the options an
//!   `rtt()` caller passes, so it defaults to `true`, unlike every
//!   [`BloomNode`](super::BloomNode) target. Three's anamorphic dump has the
//!   matching `depth24plus` beside the `rgba16float`.
//!
//! The one ownership divergence is the same one `PassNode`, `SsaaPassNode` and
//! `BloomNode` record in `docs/postprocessing.md`: three.js fires
//! `updateBefore()` from inside the render that samples the texture, so the
//! RTT pass is *recorded* after the pass that reads it and *submitted* before
//! it. The port has the application call [`RttNode::render`] explicitly,
//! before the reader's own render, which gives the GPU the same submission
//! order.

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::tsl::{texture_uv, uv};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType, Wrapping};

/// `rtt( node, width, height, options )`.
pub struct RttNode {
    /// `this.renderTarget`.
    render_target: RenderTarget,
    /// `this._quadMesh`.
    quad: QuadMesh,
    /// `this.width` / `this.height` — `None` is three's `null`, the
    /// `autoResize` case that follows the drawing buffer.
    size: Option<(u32, u32)>,
    /// `this._resolutionScale`.
    resolution_scale: f64,
}

/// `rtt( node )` — a half-float target the size of the drawing buffer.
pub fn rtt(node: NodeRef) -> RttNode {
    RttNode::new(node)
}

/// `convertToTexture( node )` — three.js' "make sure this is a texture" helper.
///
/// Upstream it returns its argument untouched when the node is already a
/// texture or sample node, and `passNode.getTextureNode()` for a pass. The port
/// has those two cases in the type system — a
/// [`PassNode`](crate::renderer::PassNode) hands out its own texture node and a
/// `NodeRef` that is already a texture is already a texture — so the only case
/// left is the one that actually builds something, and this is a spelling of
/// [`rtt`] that says why the caller wants it.
pub fn convert_to_texture(node: NodeRef) -> RttNode {
    rtt(node)
}

impl RttNode {
    /// `new RTTNode( node )`.
    pub fn new(node: NodeRef) -> Self {
        let render_target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                // `{ type: HalfFloatType, ...options }`.
                texture_type: TextureType::HalfFloat,
                samples: 0,
                // Not `false`: `RenderTarget`'s own default, which `rtt()`
                // never overrides.
                depth_buffer: true,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: an rtt() render target is a HalfFloat colour type");

        // `RTTNode.setup()`: `material.fragmentNode = this.node; material.name
        // = 'RTT'`. The port builds the quad material here rather than in a
        // setup pass, as `BloomNode` does and for the same reason — each quad
        // material's fragment is built in its own `NodeBuilder` pass anyway,
        // so there is no shared context to keep it out of.
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "RTT";
        material.fragment_node = Some(node);

        Self {
            render_target,
            quad: QuadMesh::new(material),
            size: None,
            resolution_scale: 1.0,
        }
    }

    /// `new RTTNode( node, width, height )` — a fixed-size target, which turns
    /// `autoResize` off.
    pub fn with_size(node: NodeRef, width: u32, height: u32) -> Self {
        let mut node = Self::new(node);
        node.size = Some((width, height));
        node.set_size(width, height);
        node
    }

    /// `rtt( node, null, null, { wrapS, wrapT } )` — the sampler's address
    /// modes.
    ///
    /// `webgpu_postprocessing_anamorphic` passes `MirroredRepeatWrapping` on
    /// both axes, and it matters: its high pass reads `uv.x ± 4i / width` with
    /// `i` up to ±40, i.e. up to 0.2 **outside** `[0,1]`, so the streak either
    /// folds back on itself (mirrored) or smears the edge column (clamped).
    /// Three's dump has two samplers where a clamped target would have shared
    /// one, since the sampler cache keys on exactly this pair.
    pub fn set_wrapping(&self, wrap_s: Wrapping, wrap_t: Wrapping) {
        self.render_target.texture().set_wrapping(wrap_s, wrap_t);
    }

    /// `rttNode.setResolutionScale( scale )`.
    pub fn set_resolution_scale(&mut self, resolution_scale: f64) {
        self.resolution_scale = resolution_scale;
        if let Some((width, height)) = self.size {
            self.set_size(width, height);
        }
    }

    /// `rttNode.getResolutionScale()`.
    pub fn resolution_scale(&self) -> f64 {
        self.resolution_scale
    }

    /// `RTTNode.autoResize` — true while no explicit size was given.
    pub fn auto_resize(&self) -> bool {
        self.size.is_none()
    }

    /// `RTTNode.setSize( width, height )` — `Math.floor`, like every other
    /// resolution scale in the node system.
    pub fn set_size(&self, width: u32, height: u32) {
        self.render_target.set_size(
            (width as f64 * self.resolution_scale).floor() as u32,
            (height as f64 * self.resolution_scale).floor() as u32,
        );
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.render_target.texture()
    }

    /// The node itself: the target's texture sampled at `uv()`, which is the
    /// `super( renderTarget.texture, uv() )` an `RTTNode` *is*.
    pub fn node(&self) -> NodeRef {
        texture_uv(&self.render_target.texture(), uv())
    }

    /// `rttNode.sample( uvNode )` — a tap at an explicit uv.
    ///
    /// No uv matrix: `TextureNode.clone()` keeps the non-null uv node the
    /// `RTTNode` constructor passed, so `updateMatrix` stays false and the tap
    /// is a bare `textureSample( tex, sampler, uv )`.
    pub fn sample(&self, coord: NodeRef) -> NodeRef {
        texture_uv(&self.render_target.texture(), coord)
    }

    /// The quad's material, for `examples/dump_wgsl.rs` — the one caller that
    /// has to see a material three.js keeps private, because the generated
    /// WGSL is what the rung is graded on before a pixel is compared.
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.quad.material
    }

    /// `RTTNode.updateBefore( frame )`: resize if `autoResize`, then draw the
    /// quad into the target with the renderer's state reset around it.
    pub fn render(&self, renderer: &mut Renderer) {
        if self.auto_resize() {
            let (width, height) = renderer.drawing_buffer_size();
            self.set_size(width, height);
        }

        // `resetRendererState( renderer, this._rendererState )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        renderer.set_render_target(Some(self.render_target.clone()));
        renderer.render_quad(&self.quad);

        // `restoreRendererState( renderer, this._rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::tsl::vec4;

    /// `autoResize` follows the drawing buffer; an explicit size does not, and
    /// the resolution scale floors.
    #[test]
    fn the_target_size_is_floored_and_scaled() {
        let node = RttNode::new(vec4(0.0, 0.0, 0.0, 0.0));
        assert!(node.auto_resize());
        node.set_size(800, 500);
        assert_eq!(node.texture().size(), (800, 500));

        let mut node = RttNode::with_size(vec4(0.0, 0.0, 0.0, 0.0), 800, 500);
        assert!(!node.auto_resize());
        // `Math.floor( 500 * 0.3 )` is 150, not 149.99999999999997's ceiling.
        node.set_resolution_scale(0.3);
        assert_eq!(node.texture().size(), (240, 150));
    }

    /// The wrapping reaches the texture the sampler is built from.
    #[test]
    fn the_wrapping_reaches_the_texture() {
        let node = RttNode::new(vec4(0.0, 0.0, 0.0, 0.0));
        node.set_wrapping(Wrapping::MirroredRepeat, Wrapping::MirroredRepeat);
        let texture = node.texture();
        let inner = texture.borrow();
        assert_eq!(inner.wrap_s, Wrapping::MirroredRepeat);
        assert_eq!(inner.wrap_t, Wrapping::MirroredRepeat);
    }
}
