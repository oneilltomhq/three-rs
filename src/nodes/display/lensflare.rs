//! Port of `three.js/examples/jsm/tsl/display/LensflareNode.js` — ghosts of
//! the bright parts of an image, mirrored through the screen centre.
//!
//! `lensflare( node, params )` owns one render target, a quarter of the
//! drawing buffer by default. Its `updateBefore()` draws a quad whose fragment
//! samples the input `ghostSamples` times along the vector from the flipped
//! uv to the centre, keeps what is above `threshold`, tints it and fades it
//! toward the screen edge; [`LensflareNode::node`] samples the result.
//!
//! Faithful quirks, kept because three's shader has them:
//!
//! * `const result = vec4().toVar()` starts at `vec4( 0, 0, 0, 1 )`, and every
//!   ghost is a `vec3` added to it, which `NodeBuilder.format()` widens with a
//!   `1.0` alpha. The flare's alpha is therefore `1 + ghostSamples`, not one;
//!   only the colour reaches `webgpu_postprocessing_lensflare`'s output.
//! * `Loop( { end: int( ghostSamples ) } )` with the default `float( 4 )`
//!   writes `i < 4` into the loop header, so a constant `ghost_samples`
//!   becomes an `int` literal here too; any other node is cast.
//!
//! Like [`gaussian_blur`](super::gaussian_blur), the port takes the texture
//! three's `convertToTexture()` would produce, and the caller wraps a
//! non-texture input in [`rtt`](super::rtt) itself. Unlike it, the node
//! registers its own `updateBefore()` with the renderer, as
//! [`TraaNode`](super::TraaNode) does, so nothing has to call it.
//!
//! Not ported: `dispose()` (the target is dropped with the last handle) and
//! the shared `builder.getSharedContext()` the material is given, which the
//! port's per-material builds have no use for.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{Node, Type};
use crate::nodes::tsl::{
    block, distance, float, fract, int, loop_options, texture_uv, to_var, uv, vec2, vec3, vec4,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

/// `lensflare( node, params )`' `params` object. Every field but
/// [`down_sample_ratio`](Self::down_sample_ratio) is a node, so it can be a
/// uniform the application drives.
#[derive(Clone)]
#[non_exhaustive]
pub struct LensflareParams {
    /// `params.ghostTint` — `vec3( 1, 1, 1 )` by default.
    pub ghost_tint: NodeRef,
    /// `params.threshold` — `float( 0.5 )`: what is subtracted from every
    /// ghost before it is clamped at zero.
    pub threshold: NodeRef,
    /// `params.ghostSamples` — `float( 4 )`: the number of ghosts, the loop
    /// bound.
    pub ghost_samples: NodeRef,
    /// `params.ghostSpacing` — `float( 0.25 )`: the step along the ghost
    /// vector.
    pub ghost_spacing: NodeRef,
    /// `params.ghostAttenuationFactor` — `float( 25 )`: the exponent of the
    /// fade toward the screen edge.
    pub ghost_attenuation_factor: NodeRef,
    /// `params.downSampleRatio` — `4`: the target is the drawing buffer
    /// divided by this, rounded.
    pub down_sample_ratio: f64,
}

impl Default for LensflareParams {
    fn default() -> Self {
        Self {
            ghost_tint: vec3(1.0, 1.0, 1.0),
            threshold: float(0.5),
            ghost_samples: float(4.0),
            ghost_spacing: float(0.25),
            ghost_attenuation_factor: float(25.0),
            down_sample_ratio: 4.0,
        }
    }
}

/// `lensflare( node, params )` — see the module docs for why it takes a
/// texture.
pub fn lensflare(map: &Texture, params: LensflareParams) -> LensflareNode {
    LensflareNode::new(map, params)
}

/// `LensflareNode` — a handle; the state is shared with the renderer's
/// update-before registry.
pub struct LensflareNode(Rc<LensflareState>);

/// What a [`LensflareNode`] shares with the renderer.
pub(crate) struct LensflareState {
    /// `this._renderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: QuadMesh,
    /// `this.downSampleRatio`.
    down_sample_ratio: f64,
    /// `this._textureNode` — `passTexture( this, this._renderTarget.texture
    /// )`.
    node: NodeRef,
}

impl LensflareNode {
    /// `new LensflareNode( textureNode, params )`, with the material
    /// `setup()` gives it.
    pub fn new(map: &Texture, params: LensflareParams) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false } )` — the default
        // `UnsignedByteType`, so the flare is clamped to [0, 1] when it is
        // stored.
        let target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::UnsignedByte,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: the lensflare target is a colour type");

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "LensflareNode";
        material.fragment_node = Some(lensflare_fragment(map, &params));

        let node = to_var(None, texture_uv(&target.texture(), uv()));
        let state = Rc::new(LensflareState {
            target,
            quad: QuadMesh::new(material),
            down_sample_ratio: params.down_sample_ratio,
            node,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self(state)
    }

    /// `lensflareNode.getTextureNode()` — the flare, for the graph
    /// downstream.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._renderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.0.target.texture()
    }

    /// The quad material, for `examples/dump_wgsl.rs`.
    #[doc(hidden)]
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.quad.material
    }

    /// `LensflareNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }
}

/// `Math.round( size / downSampleRatio )`. Three does not clamp at one; a
/// zero-sized target is the caller's problem there and a validation error
/// here, so the port clamps.
fn down_sampled(size: u32, ratio: f64) -> u32 {
    ((size as f64 / ratio).round() as u32).max(1)
}

impl LensflareState {
    fn set_size(&self, width: u32, height: u32) {
        self.target.set_size(
            down_sampled(width, self.down_sample_ratio),
            down_sampled(height, self.down_sample_ratio),
        );
    }
}

impl NodeUpdate for LensflareState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `LensflareNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);

        // `RendererUtils.resetRendererState( renderer, _rendererState )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        renderer.set_render_target(Some(self.target.clone()));
        renderer.render_quad(&self.quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// The `lensflare` `Fn` of `LensflareNode.setup()`.
fn lensflare_fragment(map: &Texture, params: &LensflareParams) -> NodeRef {
    // Flip uvs so the lens flare pivots around the image centre.
    let tex_coord = to_var(None, uv().one_minus());
    // Ghosts are positioned along this vector.
    let ghost_vec = to_var(
        None,
        vec2(0.5, 0.5)
            .sub(tex_coord.clone())
            .mul(params.ghost_spacing.clone()),
    );
    // `vec4().toVar()` — `vec4( 0, 0, 0, 1 )`, see the module docs.
    let result = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));

    // `int( this.ghostSamplesNode )`.
    let end = match params.ghost_samples.node() {
        Node::Const { values, .. } => int(values[0] as i64),
        _ => params.ghost_samples.to(Type::I32),
    };
    let ghosts = loop_options("i", Type::I32, int(0), end, "<", |i| {
        // `fract()` wraps the coordinates around.
        let sample_uv = to_var(None, fract(tex_coord.add(ghost_vec.mul(i.to(Type::F32)))));
        // Reduce contributions from samples at the screen edge.
        let d = distance(sample_uv.clone(), vec2(0.5, 0.5));
        let weight = d.one_minus().pow(params.ghost_attenuation_factor.clone());
        let sample = to_var(None, texture_uv(map, sample_uv)).xyz();
        let sample = sample
            .sub(params.threshold.clone())
            .max(vec3(0.0, 0.0, 0.0))
            .mul(params.ghost_tint.clone());
        vec![result.add_assign(sample.mul(weight))]
    });

    block(vec![tex_coord, ghost_vec, result.clone(), ghosts], result)
}
