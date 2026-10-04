//! Port of `three.js/examples/jsm/tsl/display/FSR1Node.js` — AMD FidelityFX
//! Super Resolution 1.0, a spatial upscaler in two full-screen passes.
//!
//! - **EASU** (edge-adaptive spatial upsampling) reads a 12-texel cross of
//!   the low-resolution input around each output pixel, finds the local edge
//!   direction and strength from the luma of its four bilinear quadrants, and
//!   shapes an approximate Lanczos2 kernel into an ellipse along that edge.
//!   The result is clamped to the four nearest texels so the negative lobe
//!   cannot ring.
//! - **RCAS** (robust contrast-adaptive sharpening) is `SharpenNode`'s pass
//!   over the EASU target: a five-tap cross with a negative lobe limited by
//!   the local contrast, attenuated in noisy areas when `denoise` is set.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME`: `updateBefore()`
//! sizes both half-float targets to the drawing buffer and draws the EASU
//! quad, then the RCAS quad, with a shared `QuadMesh`, and the graph reads
//! `passTexture( this, rcasRT.texture )`. The port's [`Fsr1State`] implements
//! [`NodeUpdate`] and is registered as the updater of the RCAS texture, as
//! [`SharpenNode`](super::SharpenNode)'s state is, so any material that reads
//! the result draws both quads first. The input size is whatever the input
//! texture is (a pass at `setResolutionScale( 0.5 )` is half the canvas);
//! the output is the drawing buffer.
//!
//! `sharpness` is in stops, as in `SharpenNode`: `con = exp2( -sharpness )`
//! scales the RCAS lobe, so 0 is the strongest sharpening.
//!
//! Not ported: `material.contextNode = context( builder.getSharedContext()
//! )`. The port builds both quad materials in the constructor, in a
//! `NodeBuilder` pass of their own, as [`RttNode`](super::RttNode) does.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{TextureSource, Type};
use crate::nodes::tsl::{
    abs, block, float, floor, fract, int, ivec2, max, sqrt, texture_load, texture_size, texture_uv,
    to_const, to_var, uv, vec2, vec2_join, vec4,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

use super::rtt::{convert_to_texture, RttNode};
use super::sharpen::rcas;

/// `fsr1( node, sharpness = 0.2, denoise = false )`.
///
/// `node` is wrapped in [`convert_to_texture`], as three wraps it in
/// `convertToTexture()`; for a texture already in hand (a pass attachment,
/// which three's `convertToTexture()` passes through) use [`Fsr1Node::new`].
/// `sharpness` is three's `(number|Node<float>)`: a number becomes a constant
/// folded into the RCAS shader, as three's `nodeObject( 0.2 )` does, and a
/// float node — a [`uniform_settable`](crate::nodes::tsl::uniform_settable)
/// for a value written between frames — is read as it is. `denoise` is
/// three's `nodeObject( false )`, a constant the shader branches on. Three's
/// defaults are [`Fsr1Node::DEFAULT_SHARPNESS`] and `false`.
pub fn fsr1(node: NodeRef, sharpness: impl Into<NodeRef>, denoise: bool) -> Fsr1Node {
    let input = convert_to_texture(node);
    Fsr1Node::build(&input.texture(), sharpness.into(), denoise, Some(input))
}

/// `FSR1Node` — see the module docs. Keep it alive for as long as the graph
/// reads [`node`](Self::node): the renderer reaches it through its texture,
/// weakly.
pub struct Fsr1Node(Rc<Fsr1State>);

/// The node's state, shared with the renderer's update-before registry.
pub(crate) struct Fsr1State {
    /// The `convertToTexture( node )` [`fsr1`] made, held so it keeps being
    /// drawn; `None` when the caller passed a texture.
    _input: Option<RttNode>,
    /// `this._easuRT`.
    easu_target: RenderTarget,
    /// `this._rcasRT`.
    rcas_target: RenderTarget,
    /// `_quadMesh` with `this._easuMaterial`.
    easu_quad: QuadMesh,
    /// `_quadMesh` with `this._rcasMaterial`.
    rcas_quad: QuadMesh,
    /// `this._textureNode` — `passTexture( this, rcasRT.texture )`.
    node: NodeRef,
}

/// `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`.
fn half_float_target() -> RenderTarget {
    RenderTarget::new_with_options(
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
    .expect("three-rs: the FSR 1 targets are a HalfFloat colour type")
}

impl Fsr1Node {
    /// Three's default `sharpness`, `0.2`.
    pub const DEFAULT_SHARPNESS: f64 = 0.2;

    /// `new FSR1Node( textureNode, sharpness, denoise )` over a texture
    /// already in hand, with the two materials `setup()` gives it.
    /// `sharpness` is a float node; see [`fsr1`].
    pub fn new(map: &Texture, sharpness: NodeRef, denoise: bool) -> Self {
        Self::build(map, sharpness, denoise, None)
    }

    fn build(map: &Texture, sharpness: NodeRef, denoise: bool, input: Option<RttNode>) -> Self {
        let easu_target = half_float_target();
        let rcas_target = half_float_target();

        let mut easu_material = MeshBasicNodeMaterial::new();
        easu_material.name = "FSR1_EASU";
        easu_material.fragment_node = Some(easu(map));

        let mut rcas_material = MeshBasicNodeMaterial::new();
        rcas_material.name = "FSR1_RCAS";
        rcas_material.fragment_node = Some(rcas(&easu_target.texture(), sharpness, denoise, true));

        let node = to_var(None, texture_uv(&rcas_target.texture(), uv()));

        let state = Rc::new(Fsr1State {
            _input: input,
            easu_target,
            rcas_target,
            easu_quad: QuadMesh::new(easu_material),
            rcas_quad: QuadMesh::new(rcas_material),
            node,
        });
        register_texture_update(state.rcas_target.texture().id(), &state);
        Self(state)
    }

    /// `fsr1Node.getTextureNode()` — the upscaled, sharpened frame.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._rcasRT.texture`, the result.
    pub fn texture(&self) -> Texture {
        self.0.rcas_target.texture()
    }

    /// `this._easuRT.texture`, the upscaled frame before sharpening.
    pub fn easu_texture(&self) -> Texture {
        self.0.easu_target.texture()
    }

    /// `FSR1Node.setSize( width, height )` — the output size of both passes.
    /// `updateBefore()` sizes them to the drawing buffer every frame, so a
    /// caller rarely needs this.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.easu_target.set_size(width, height);
        self.0.rcas_target.set_size(width, height);
    }

    /// The `FSR1_EASU` quad material, for `examples/dump_wgsl.rs` and the
    /// dump gate.
    #[doc(hidden)]
    pub fn easu_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.easu_quad.material
    }

    /// The `FSR1_RCAS` quad material, for the dump gate.
    #[doc(hidden)]
    pub fn rcas_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.rcas_quad.material
    }
}

impl NodeUpdate for Fsr1State {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `FSR1Node.updateBefore( frame )`.
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
        self.easu_target.set_size(width, height);
        self.rcas_target.set_size(width, height);

        // EASU pass
        renderer.set_render_target(Some(self.easu_target.clone()));
        renderer.render_quad(&self.easu_quad);

        // RCAS pass
        renderer.set_render_target(Some(self.rcas_target.clone()));
        renderer.render_quad(&self.rcas_quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// `luma( s )` in `easu` — `s.r * 0.5 + s.g + s.b * 0.5`, the approximate
/// luminance EASU finds edges with.
fn easu_luma(s: &NodeRef) -> NodeRef {
    s.x().mul(0.5).add(s.y()).add(s.z().mul(0.5))
}

/// `1.0 / 65536.0`, the floor under EASU's edge-length divisor.
const EDGE_EPSILON: f64 = 1.0 / 65536.0;

/// `_accumulateEdge( dir, len, w, aL, bL, cL, dL, eL )` — one bilinear
/// quadrant's edge direction and length, weighted by `w`, added into `dir`
/// and `len`. A plain arrow function in three, so its `toConst()`s and
/// `addAssign()`s join the caller's stack in this order.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn accumulate_edge(
    statements: &mut Vec<NodeRef>,
    dir: &NodeRef,
    len: &NodeRef,
    w: &NodeRef,
    luma: [&NodeRef; 5],
) {
    let [a_l, b_l, c_l, d_l, e_l] = luma;

    let dc = to_const(None, d_l.sub(c_l));
    let cb = to_const(None, c_l.sub(b_l));
    let dir_x = to_const(None, d_l.sub(b_l));
    let len_x = to_const(None, max(abs(dc.clone()), abs(cb.clone())));
    let s_len_x = to_const(
        None,
        abs(dir_x.clone())
            .div(max(len_x.clone(), float(EDGE_EPSILON)))
            .saturate(),
    );
    let x_dir = dir.x().add_assign(dir_x.mul(w));
    let x_len = len.add_assign(s_len_x.mul(&s_len_x).mul(w));

    let ec = to_const(None, e_l.sub(c_l));
    let ca = to_const(None, c_l.sub(a_l));
    let dir_y = to_const(None, e_l.sub(a_l));
    let len_y = to_const(None, max(abs(ec.clone()), abs(ca.clone())));
    let s_len_y = to_const(
        None,
        abs(dir_y.clone())
            .div(max(len_y.clone(), float(EDGE_EPSILON)))
            .saturate(),
    );
    let y_dir = dir.y().add_assign(dir_y.mul(w));
    let y_len = len.add_assign(s_len_y.mul(&s_len_y).mul(w));

    statements.extend([
        dc, cb, dir_x, len_x, s_len_x, x_dir, x_len, ec, ca, dir_y, len_y, s_len_y, y_dir, y_len,
    ]);
}

/// What `_accumulateTap` reads besides its own offset and colour: the
/// accumulators and the kernel shape `easu` computed once.
struct Kernel {
    /// `aC`, the weighted colour sum.
    a_c: NodeRef,
    /// `aW`, the weight sum.
    a_w: NodeRef,
    /// The normalised edge direction.
    dir: NodeRef,
    /// The anisotropic lengths.
    len2: NodeRef,
    /// The negative lobe.
    lob: NodeRef,
    /// `1 / lob`, the clip on the squared distance.
    clp: NodeRef,
}

/// `_accumulateTap( aC, aW, offset, dir, len2, lob, clp, color )` — one
/// tap's approximate Lanczos2 weight, accumulated. A plain arrow function,
/// so its `toConst()`s and `addAssign()`s join the caller's stack.
#[inline(never)]
fn accumulate_tap(statements: &mut Vec<NodeRef>, k: &Kernel, offset: NodeRef, color: &NodeRef) {
    let vx = to_const(
        None,
        offset.x().mul(k.dir.x()).add(offset.y().mul(k.dir.y())),
    );
    let vy = to_const(
        None,
        offset
            .x()
            .mul(k.dir.y())
            .negate()
            .add(offset.y().mul(k.dir.x())),
    );

    let sx = to_const(None, vx.mul(k.len2.x()));
    let sy = to_const(None, vy.mul(k.len2.y()));
    let d2 = to_const(None, sx.mul(&sx).add(sy.mul(&sy)).min(k.clp.clone()));

    let w_b = to_const(None, d2.mul(2.0 / 5.0).sub(1.0));
    let w_a = to_const(None, d2.mul(k.lob.clone()).sub(1.0));
    let w = to_const(
        None,
        w_b.mul(&w_b)
            .mul(25.0 / 16.0)
            .sub(25.0 / 16.0 - 1.0)
            .mul(w_a.mul(&w_a)),
    );

    let color_sum = k.a_c.add_assign(color.mul(&w));
    let weight_sum = k.a_w.add_assign(w.clone());

    statements.extend([vx, vy, sx, sy, d2, w_b, w_a, w, color_sum, weight_sum]);
}

/// `setup()`'s `easu` `Fn()`, which has no layout and so inlines into
/// `main()`.
#[inline(never)]
fn easu(map: &Texture) -> NodeRef {
    let target_uv = uv();
    // `vec2( textureSize( textureNode ) )`, at three's default level.
    let tex_size = texture_size(TextureSource::Texture2D(map.clone()), int(0)).to(Type::Vec2);

    let pp = to_const(None, target_uv.mul(tex_size).sub(0.5));
    let fp = to_const(None, floor(pp.clone()));
    let f = to_const(None, fract(pp.clone()));

    // Fetch exact texel values at integer coordinates (no filtering).
    let ifp = to_const(None, ivec2(fp.x().to(Type::I32), fp.y().to(Type::I32)));
    let tap = |dx: i64, dy: i64| texture_load(map, ifp.add(ivec2(int(dx), int(dy))));

    // 12-tap cross pattern:
    //       b c
    //     e f g h
    //     i j k l
    //       n o
    let s_b = tap(0, -1);
    let s_c = tap(1, -1);
    let s_e = tap(-1, 0);
    let s_f = tap(0, 0);
    let s_g = tap(1, 0);
    let s_h = tap(2, 0);
    let s_i = tap(-1, 1);
    let s_j = tap(0, 1);
    let s_k = tap(1, 1);
    let s_l = tap(2, 1);
    let s_n = tap(0, 2);
    let s_o = tap(1, 2);

    // Approximate luminance for edge detection.
    let b_l = easu_luma(&s_b);
    let c_l = easu_luma(&s_c);
    let e_l = easu_luma(&s_e);
    let f_l = easu_luma(&s_f);
    let g_l = easu_luma(&s_g);
    let h_l = easu_luma(&s_h);
    let i_l = easu_luma(&s_i);
    let j_l = easu_luma(&s_j);
    let k_l = easu_luma(&s_k);
    let l_l = easu_luma(&s_l);
    let n_l = easu_luma(&s_n);
    let o_l = easu_luma(&s_o);

    // Accumulate edge direction and length from 4 bilinear quadrants.
    let dir = to_var(None, vec2(0.0, 0.0));
    let len = to_var(None, float(0.0));

    let w0 = to_const(None, float(1.0).sub(f.x()).mul(float(1.0).sub(f.y())));
    let w1 = to_const(None, f.x().mul(float(1.0).sub(f.y())));
    let w2 = to_const(None, float(1.0).sub(f.x()).mul(f.y()));
    let w3 = to_const(None, f.x().mul(f.y()));

    let mut statements = vec![
        pp,
        fp,
        f.clone(),
        ifp,
        dir.clone(),
        len.clone(),
        w0.clone(),
        w1.clone(),
        w2.clone(),
        w3.clone(),
    ];

    accumulate_edge(
        &mut statements,
        &dir,
        &len,
        &w0,
        [&b_l, &e_l, &f_l, &g_l, &j_l],
    );
    accumulate_edge(
        &mut statements,
        &dir,
        &len,
        &w1,
        [&c_l, &f_l, &g_l, &h_l, &k_l],
    );
    accumulate_edge(
        &mut statements,
        &dir,
        &len,
        &w2,
        [&f_l, &i_l, &j_l, &k_l, &n_l],
    );
    accumulate_edge(
        &mut statements,
        &dir,
        &len,
        &w3,
        [&g_l, &j_l, &k_l, &l_l, &o_l],
    );

    let (kernel, kernel_statements) = easu_kernel(&dir, &len);
    statements.extend(kernel_statements);

    // Accumulate weighted taps.
    statements.extend([kernel.a_c.clone(), kernel.a_w.clone()]);
    for (x, y, color) in [
        (0.0, -1.0, &s_b),
        (1.0, -1.0, &s_c),
        (-1.0, 0.0, &s_e),
        (0.0, 0.0, &s_f),
        (1.0, 0.0, &s_g),
        (2.0, 0.0, &s_h),
        (-1.0, 1.0, &s_i),
        (0.0, 1.0, &s_j),
        (1.0, 1.0, &s_k),
        (2.0, 1.0, &s_l),
        (0.0, 2.0, &s_n),
        (1.0, 2.0, &s_o),
    ] {
        accumulate_tap(&mut statements, &kernel, vec2(x, y).sub(&f), color);
    }

    // Normalize.
    statements.push(kernel.a_c.div_assign(kernel.a_w.clone()));

    // Anti-ringing: clamp to min/max of the 4 nearest samples (f, g, j, k).
    let min4 = to_const(None, s_f.min(s_g.clone()).min(s_j.min(s_k.clone())));
    let max4 = to_const(None, max(max(s_f, s_g), max(s_j, s_k)));
    statements.extend([min4.clone(), max4.clone()]);

    block(statements, kernel.a_c.clamp(min4, max4))
}

/// The middle of `easu`: normalise the edge direction, shape the kernel by
/// the edge strength, and declare the tap accumulators (not yet in the
/// statements; the caller declares them where three does).
#[inline(never)]
fn easu_kernel(dir: &NodeRef, len: &NodeRef) -> (Kernel, Vec<NodeRef>) {
    // Normalize direction, defaulting to (1, 0) when gradient is negligible.
    let dir_sq = to_const(None, dir.x().mul(dir.x()).add(dir.y().mul(dir.y())));
    let zro = to_const(None, dir_sq.less_than(1.0 / 32768.0));
    let r_dir_len = to_const(
        None,
        float(1.0).div(sqrt(max(dir_sq.clone(), float(1.0 / 32768.0)))),
    );

    let normalize_x = dir.x().assign(zro.select(float(1.0), dir.x()));
    let normalize = dir.mul_assign(zro.select(float(1.0), r_dir_len.clone()));

    // Shape the kernel based on edge strength.
    let half = len.assign(len.mul(0.5));
    let square = len.mul_assign(len.clone());

    // Stretch factor: 1.0 for axis-aligned edges, sqrt(2) on diagonals.
    let stretch = to_const(
        None,
        dir.x()
            .mul(dir.x())
            .add(dir.y().mul(dir.y()))
            .div(max(abs(dir.x()), abs(dir.y()))),
    );

    // Anisotropic lengths: x stretches along edge, y shrinks perpendicular.
    let len2 = to_const(
        None,
        vec2_join(vec![
            float(1.0).add(stretch.sub(1.0).mul(len)),
            float(1.0).sub(len.mul(0.5)),
        ]),
    );

    // Negative lobe: strong on flat areas (0.5), reduced on edges (0.21).
    let lob = to_const(None, float(0.5).add(float(1.0 / 4.0 - 0.04 - 0.5).mul(len)));
    let clp = to_const(None, float(1.0).div(lob.clone()));

    let kernel = Kernel {
        a_c: to_var(None, vec4(0.0, 0.0, 0.0, 0.0)),
        a_w: to_var(None, float(0.0)),
        dir: dir.clone(),
        len2: len2.clone(),
        lob: lob.clone(),
        clp: clp.clone(),
    };
    let statements = vec![
        dir_sq,
        zro,
        r_dir_len,
        normalize_x,
        normalize,
        half,
        square,
        stretch,
        len2,
        lob,
        clp,
    ];
    (kernel, statements)
}
