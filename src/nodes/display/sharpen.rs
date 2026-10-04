//! Port of `three.js/examples/jsm/tsl/display/SharpenNode.js` — AMD
//! FidelityFX FSR 1's robust contrast-adaptive sharpening (RCAS).
//!
//! A five-tap cross (the centre and its four edge neighbours, read with
//! `textureLoad` at integer texels) gives a negative lobe weight for the
//! ring, limited by the local contrast so the result cannot leave the range
//! of the ring and the centre; the output is the ring and the centre blended
//! by that weight. With `denoise` the lobe is scaled down where the centre
//! stands out from its ring (noise rather than an edge).
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME` that draws its
//! one quad into a half-float target sized to the drawing buffer, and hands
//! the graph `passTexture( this, target.texture )`. The port's
//! [`SharpenState`] implements [`NodeUpdate`] and is registered as the
//! updater of that texture, as [`TraaNode`](super::TraaNode)'s state is, so
//! any material that reads the result draws the quad first.
//!
//! `sharpness` is in stops: `con = exp2( -sharpness )` scales the lobe, so 0
//! is the strongest sharpening and every further unit halves it. Three's doc
//! comment calls 2 "no sharpening"; it is a quarter of the strength of 0.
//!
//! Not ported: `material.contextNode = context( builder.getSharedContext()
//! )`. The port builds the quad material in the constructor, in a
//! `NodeBuilder` pass of its own, as [`RttNode`](super::RttNode) does.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{TextureSource, Type};
use crate::nodes::tsl::{
    abs, block, boolean, exp2, float, floor, int, ivec2, max, texture_load, texture_size,
    texture_uv, to_const, to_var, uv, vec3, vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

use super::rtt::{convert_to_texture, RttNode};

/// `sharpen( node, sharpness = 0.2, denoise = false )`.
///
/// `node` is wrapped in [`convert_to_texture`], as three wraps it in
/// `convertToTexture()`; for a texture already in hand (a pass attachment,
/// which three's `convertToTexture()` passes through) use
/// [`SharpenNode::new`]. `sharpness` is three's `(number|Node<float>)`: a
/// number becomes a constant folded into the shader, as three's
/// `nodeObject( 0.2 )` does, and a float node — a
/// [`uniform_settable`](crate::nodes::tsl::uniform_settable) for a value
/// written between frames — is read as it is. `denoise` is three's
/// `nodeObject( false )`, a constant the shader branches on.
pub fn sharpen(node: NodeRef, sharpness: impl Into<NodeRef>, denoise: bool) -> SharpenNode {
    let input = convert_to_texture(node);
    SharpenNode::build(&input.texture(), sharpness.into(), denoise, Some(input))
}

/// `SharpenNode` — see the module docs. Keep it alive for as long as the
/// graph reads [`node`](Self::node): the renderer reaches it through its
/// texture, weakly.
pub struct SharpenNode(Rc<SharpenState>);

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct SharpenState {
    /// The `convertToTexture( node )` [`sharpen`] made, held so it keeps
    /// being drawn; `None` when the caller passed a texture.
    _input: Option<RttNode>,
    /// `this._renderTarget`.
    target: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: QuadMesh,
    /// `this._textureNode` — `passTexture( this, target.texture )`.
    node: NodeRef,
}

impl SharpenNode {
    /// `new SharpenNode( textureNode, sharpness, denoise )` over a texture
    /// already in hand, with the material `setup()` gives it. `sharpness` is
    /// a float node; see [`sharpen`].
    pub fn new(map: &Texture, sharpness: NodeRef, denoise: bool) -> Self {
        Self::build(map, sharpness, denoise, None)
    }

    fn build(map: &Texture, sharpness: NodeRef, denoise: bool, input: Option<RttNode>) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`.
        let target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer: false,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )
        .expect("three-rs: the sharpen target is a HalfFloat colour type");

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "Sharpen_RCAS";
        material.fragment_node = Some(rcas(map, sharpness, denoise, false));

        let node = to_var(None, texture_uv(&target.texture(), uv()));

        let state = Rc::new(SharpenState {
            _input: input,
            target,
            quad: QuadMesh::new(material),
            node,
        });
        register_texture_update(state.target.texture().id(), &state);
        Self(state)
    }

    /// `sharpenNode.getTextureNode()` — the sharpened frame.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._renderTarget.texture`.
    pub fn texture(&self) -> Texture {
        self.0.target.texture()
    }

    /// `SharpenNode.setSize( width, height )`. `updateBefore()` sizes the
    /// target to the drawing buffer every frame, so a caller rarely needs
    /// this.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.target.set_size(width, height);
    }

    /// The `Sharpen_RCAS` quad material, for `examples/dump_wgsl.rs` and
    /// the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.quad.material
    }
}

impl NodeUpdate for SharpenState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SharpenNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
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

/// `luma( s )` — `s.g + ( s.b + s.r ) * 0.5`, the approximate luminance
/// times two.
fn luma(s: &NodeRef) -> NodeRef {
    s.y().add(s.z().add(s.x()).mul(0.5))
}

/// `setup()`'s `rcas` `Fn()`, which has no layout and so inlines into
/// `main()`.
///
/// `FSR1Node`'s RCAS pass is the same `Fn` over its EASU target, except that
/// it converts the size first, `vec2( textureSize( textureLoad( easuTex ) )
/// )`: one `vec2<f32>` that both axes read, so a `let`, where `SharpenNode`'s
/// unconverted `uvec2` is spelled out at each read. `vec2_size` picks
/// `FSR1Node`'s form.
#[inline(never)]
pub(super) fn rcas(map: &Texture, sharpness: NodeRef, denoise: bool, vec2_size: bool) -> NodeRef {
    let target_uv = uv();
    // `textureSize( textureLoad( inputTex ) )`, at three's default level.
    let tex_size = || texture_size(TextureSource::Texture2D(map.clone()), int(0));
    // Three caches the converted size, read twice, in a `let`; the port's
    // conversion is not a cached node, so it is a `toConst()` here, which
    // is the same WGSL.
    let mut statements = Vec::new();
    let (size_x, size_y) = if vec2_size {
        let size = to_const(None, tex_size().to(Type::Vec2));
        statements.push(size.clone());
        (size.x(), size.y())
    } else {
        (tex_size().x(), tex_size().y())
    };

    let p = to_const(
        None,
        ivec2(
            floor(target_uv.x().mul(size_x)).to(Type::I32),
            floor(target_uv.y().mul(size_y)).to(Type::I32),
        ),
    );

    let e = texture_load(map, p.clone());
    let b = texture_load(map, p.add(ivec2(int(0), int(-1))));
    let d = texture_load(map, p.add(ivec2(int(-1), int(0))));
    let f = texture_load(map, p.add(ivec2(int(1), int(0))));
    let h = texture_load(map, p.add(ivec2(int(0), int(1))));

    // Approximate luminance (luma times 2).
    let b_l = luma(&b);
    let d_l = luma(&d);
    let e_l = luma(&e);
    let f_l = luma(&f);
    let h_l = luma(&h);

    // Sharpening amount from user parameter.
    let con = to_const(None, exp2(sharpness.negate()));

    // Min and max of ring.
    let mn4 = to_const(None, b.rgb().min(d.rgb()).min(f.rgb().min(h.rgb())));
    let mx4 = to_const(None, max(max(b.rgb(), d.rgb()), max(f.rgb(), h.rgb())));

    // Compute adaptive lobe weight. Limiters based on how much sharpening
    // the local contrast can tolerate.
    let rcas_limit = to_const(None, float(0.25 - 1.0 / 16.0));

    let hit_min = to_const(None, mn4.min(e.rgb()).div(mx4.mul(4.0)));
    let hit_max = to_const(
        None,
        vec3(1.0, 1.0, 1.0)
            .sub(max(mx4.clone(), e.rgb()))
            .div(mn4.mul(4.0).sub(4.0)),
    );
    let lobe_rgb = to_const(None, max(hit_min.negate(), hit_max.clone()));

    let lobe = to_const(
        None,
        max(
            rcas_limit.negate(),
            max(lobe_rgb.x(), max(lobe_rgb.y(), lobe_rgb.z())).min(float(0.0)),
        )
        .mul(con.clone()),
    );

    // Noise attenuation.
    let nz = to_const(
        None,
        b_l.add(d_l.clone())
            .add(f_l.clone())
            .add(h_l.clone())
            .mul(0.25)
            .sub(e_l.clone()),
    );
    let nz_range = to_const(
        None,
        max(
            max(b_l.clone(), d_l.clone()),
            max(e_l.clone(), max(f_l.clone(), h_l.clone())),
        )
        .sub(b_l.min(d_l).min(e_l.min(f_l.min(h_l)))),
    );
    let nz_factor = to_const(
        None,
        float(1.0).sub(
            abs(nz.clone())
                .div(max(nz_range.clone(), float(1.0 / 65536.0)))
                .saturate()
                .mul(0.5),
        ),
    );

    let effective_lobe = to_const(
        None,
        boolean(denoise)
            .equal(true)
            .select(lobe.mul(nz_factor.clone()), lobe.clone()),
    );

    // Resolve: weighted blend of cross neighbors and center.
    let result = to_const(
        None,
        b.rgb()
            .add(d.rgb())
            .add(f.rgb())
            .add(h.rgb())
            .mul(effective_lobe.clone())
            .add(e.rgb())
            .div(effective_lobe.mul(4.0).add(1.0)),
    );

    // Three's `toConst()`s join the `Fn`'s stack where they are made, so
    // they are declared in this order rather than at first read.
    statements.extend([
        p,
        con,
        mn4,
        mx4,
        rcas_limit,
        hit_min,
        hit_max,
        lobe_rgb,
        lobe,
        nz,
        nz_range,
        nz_factor,
        effective_lobe,
        result.clone(),
    ]);
    block(statements, vec4_join(vec![result, e.a()]))
}
