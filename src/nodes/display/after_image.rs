//! Port of `three.js/examples/jsm/tsl/display/AfterImageNode.js` — a
//! feedback trail: every frame keeps the brighter of the new input and the
//! previous output, the latter faded by `damp`.
//!
//! Three owns two render targets, `_compRT` and `_oldRT`, renders into the
//! first, and swaps the two JS objects after the render, re-pointing its two
//! texture nodes at them at the top of the next `updateBefore()`. A texture is
//! an identity in the port's graph, so the swap is `PassNode`'s previous-frame
//! machinery instead (`docs/nodes.md` §14): one target, whose attachment is
//! the "current" texture the output node and the render read and write, plus
//! a previous texture behind the same name, and
//! [`RenderTarget::toggle_texture`] exchanges their GPU allocations before
//! each render. The GPU sees three's two allocations alternating in three's
//! order.

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::tsl::{block, float, max, sign, texture_uv, to_const, to_var, uv};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer, OUTPUT_ATTACHMENT};
use crate::textures::{Texture, TextureFilter, TextureType};

/// `afterImage( node, damp )`.
pub struct AfterImageNode {
    /// `this._compRT`, with `this._oldRT`'s texture as its previous texture.
    target: RenderTarget,
    /// `this._materialComposed`'s quad.
    quad: QuadMesh,
    /// `this._textureNode` — `passTexture( this, this._compRT.texture )`.
    node: NodeRef,
}

/// `afterImage( node, damp )`. `map` is what three's `convertToTexture()`
/// would have made of the node; `damp` is `float( 0.96 )` in three when the
/// page passes none.
pub fn after_image(map: &Texture, damp: NodeRef) -> AfterImageNode {
    AfterImageNode::new(map, damp)
}

impl AfterImageNode {
    pub fn new(map: &Texture, damp: NodeRef) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false } )`, whose type
        // `updateBefore()` sets to the input's — see `GaussianBlurNode::new`.
        let texture_type = match map.format() {
            wgpu::TextureFormat::Rgba16Float => TextureType::HalfFloat,
            _ => TextureType::UnsignedByte,
        };
        let target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: an after-image target is a colour type");
        let old = target.add_previous_texture(OUTPUT_ATTACHMENT);

        // `textureNodeOld.uvNode = textureNode.uvNode || uv()`.
        let texel_old = to_var(None, texture_uv(&old, uv()));
        let texel_new = to_var(None, texture_uv(map, uv()));
        let threshold = to_const(None, float(0.1));

        // m acts as a mask. It's 1 if the previous pixel was "bright enough"
        // (above the threshold) and 0 if it wasn't.
        let m = max(sign(texel_old.clone().sub(threshold)), 0.0);

        // This is where the after-image fades: texelOld is multiplied by
        // `damp` where the mask is 1, and cleared where it is 0.
        let fragment = block(
            vec![
                texel_old.clone(),
                texel_new.clone(),
                texel_old.mul_assign(damp.mul(m)),
            ],
            max(texel_new, texel_old),
        );

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "AfterImage";
        material.fragment_node = Some(fragment);

        let node = to_var(None, texture_uv(&target.texture(), uv()));

        Self {
            target,
            quad: QuadMesh::new(material),
            node,
        }
    }

    /// `afterImageNode.getTextureNode()`.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// The `AfterImage` quad material, for `examples/dump_wgsl.rs`.
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.quad.material
    }

    /// `AfterImageNode.updateBefore( frame )`: size both textures to the
    /// drawing buffer, swap them, and composite into the current one.
    ///
    /// Three swaps *after* the render and re-points its nodes *before* the
    /// next one; the port's nodes never move, so the swap comes first — after
    /// it, the previous texture holds last frame's composite, which is what
    /// three's `_oldRT` holds at the same moment.
    pub fn render(&self, renderer: &mut Renderer) {
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        let (width, height) = renderer.drawing_buffer_size();
        self.target.set_size(width, height);
        self.target.toggle_texture(OUTPUT_ATTACHMENT);

        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(&self.quad);

        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
    }
}
