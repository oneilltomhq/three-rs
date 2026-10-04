//! Port of `three.js/examples/jsm/tsl/display/SMAANode.js` — SMAA 1x Medium
//! with colour edge detection (iryoku's SMAA v2.8), the anti-aliasing pass
//! `webgpu_postprocessing_ssr` ends on.
//!
//! Three's node is a `Node` with `updateBeforeType = FRAME` that draws three
//! quads every frame, each into a half-float target the size of the drawing
//! buffer:
//!
//! 1. `SMAANode.edges`: luma-free colour edge detection with local contrast
//!    adaptation, discarding pixels without an edge;
//! 2. `SMAANode.weights`: the blending weights, from four searches along the
//!    edges (a fixed 8-step loop each) and the precomputed area texture;
//! 3. `SMAANode.blend`: the neighbourhood blend of the input by those
//!    weights.
//!
//! [`SmaaNode::node`] is the blend target sampled at `uv()`, three's
//! `passTexture( this, this._renderTargetBlend.texture )`.
//!
//! The area and search textures are the two base64 PNGs `SMAANode.js`
//! embeds. `smaa_area.png` (160×560 RGB) and `smaa_search.png` (66×33 grey)
//! next to this file are those payloads base64-decoded, byte for byte; they
//! are compiled in with `include_bytes!` and decoded to pixels at runtime,
//! on first use, then expanded to RGBA8 the way the browser's image decode
//! does. The search texture is
//! `NearestFilter` on both filters, so — as in three — it is unfilterable
//! and every tap is a clamped `textureLoad`.
//!
//! Three's `SMAASearchXLeft` … `SMAAArea` are layout-less `Fn`s, which
//! inline at their call sites; here they are Rust functions returning a
//! [`block`], with the same statements in the same order.

use std::path::Path;
use std::rc::Rc;

use crate::loaders::texture_loader::decode_png;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    abs, block, break_loop, discard, dot, float, if_else, if_then, int, loop_options, max, mix,
    sign, sqrt, step, texture_uv, texture_with_uv, to_const, to_var, to_varying, uniform_settable,
    uv, vec2, vec2_join, vec4, vec4_join,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{MinFilter, Texture, TextureFilter, TextureType};

/// `SMAA_THRESHOLD`.
const THRESHOLD: f64 = 0.1;
/// `SMAA_MAX_SEARCH_STEPS`.
const MAX_SEARCH_STEPS: i64 = 8;
/// `SMAA_AREATEX_MAX_DISTANCE`.
const AREATEX_MAX_DISTANCE: f64 = 16.0;
/// `SMAA_AREATEX_PIXEL_SIZE = vec2( 1 / 160, 1 / 560 )`.
const AREATEX_PIXEL_SIZE: (f64, f64) = (1.0 / 160.0, 1.0 / 560.0);
/// `SMAA_AREATEX_SUBTEX_SIZE = 1 / 7`.
const AREATEX_SUBTEX_SIZE: f64 = 1.0 / 7.0;

/// `_getAreaTexture()`, decoded.
const AREA_PNG: &[u8] = include_bytes!("smaa_area.png");
/// `_getSearchTexture()`, decoded.
const SEARCH_PNG: &[u8] = include_bytes!("smaa_search.png");

/// `smaa( node )` — SMAA 1x over a linear (pre-tone-mapping) input.
///
/// Three runs its argument through `convertToTexture()`; the port takes the
/// texture and the caller converts first, as with [`fxaa`](super::fxaa). If
/// the texture has a frame updater (a pass or an [`rtt`](super::rtt)), it is
/// run before the edges pass reads it.
pub fn smaa(input: &Texture) -> SmaaNode {
    SmaaNode::new(input)
}

/// `SMAANode`.
pub struct SmaaNode(Rc<SmaaState>);

/// What a [`SmaaNode`] shares with the frame's update list.
struct SmaaState {
    input: Texture,
    edges_target: RenderTarget,
    weights_target: RenderTarget,
    blend_target: RenderTarget,
    /// `this._invSize`.
    inv_size: SettableValue,
    edges_quad: QuadMesh,
    weights_quad: QuadMesh,
    blend_quad: QuadMesh,
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
    .expect("three-rs: an SMAA render target is a HalfFloat colour type")
}

/// An embedded PNG as a `new Texture()` with `generateMipmaps = false` and
/// `flipY = false`.
fn lookup_texture(name: &str, bytes: &[u8]) -> Texture {
    let image = decode_png(Path::new(name), bytes)
        .unwrap_or_else(|e| panic!("three-rs: SMAA's embedded {name} decodes: {e}"));
    let texture = Texture::new(image.width, image.height, Some(image.data));
    texture.set_generate_mipmaps(false);
    texture.set_flip_y(false);
    texture
}

/// A quad material for one of the three passes.
fn pass_quad(name: &'static str, fragment: NodeRef) -> QuadMesh {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = name;
    material.fragment_node = Some(fragment);
    QuadMesh::new(material)
}

/// The taps the three passes share.
struct Taps {
    input: Texture,
    edges: Texture,
    weights: Texture,
    area: Texture,
    search: Texture,
    inv_size: NodeRef,
}

impl Taps {
    /// `this.textureNode.sample( coord )`: the RTT's own uv, so no matrix.
    fn input(&self, coord: NodeRef) -> NodeRef {
        texture_uv(&self.input, coord)
    }

    /// `this._edgesTextureUniform.sample( coord )`.
    fn edges(&self, coord: NodeRef) -> NodeRef {
        texture_with_uv(&self.edges, coord)
    }

    /// `this._weightsTextureUniform.sample( coord )`.
    fn weights(&self, coord: NodeRef) -> NodeRef {
        texture_with_uv(&self.weights, coord)
    }

    /// `vec4( v.xy, v.xy )`.
    fn twice(v: &NodeRef) -> NodeRef {
        vec4_join(vec![v.xy(), v.xy()])
    }

    /// `vec4( uvNode.xy, uvNode.xy ).add( vec4( this._invSize.xy,
    /// this._invSize.xy ).mul( vec4( a, b, c, d ) ) ).toVertexStage()`.
    fn offset(&self, a: f64, b: f64, c: f64, d: f64) -> NodeRef {
        to_varying(
            None,
            Self::twice(&uv()).add(Self::twice(&self.inv_size).mul(vec4(a, b, c, d))),
        )
    }
}

impl SmaaNode {
    /// `new SMAANode( textureNode )`.
    pub fn new(input: &Texture) -> Self {
        let edges_target = half_float_target();
        let weights_target = half_float_target();
        let blend_target = half_float_target();

        let area = lookup_texture("smaa_area.png", AREA_PNG);
        area.set_min_filter(MinFilter::Linear);
        let search = lookup_texture("smaa_search.png", SEARCH_PNG);
        search.set_mag_filter(TextureFilter::Nearest);
        search.set_min_filter(MinFilter::Nearest);

        let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        let taps = Taps {
            input: input.clone(),
            edges: edges_target.texture(),
            weights: weights_target.texture(),
            area,
            search,
            inv_size: inv_size_node,
        };

        let state = Rc::new(SmaaState {
            input: input.clone(),
            edges_quad: pass_quad("SMAANode.edges", edge_detection(&taps)),
            weights_quad: pass_quad("SMAANode.weights", weights(&taps)),
            blend_quad: pass_quad("SMAANode.blend", blend(&taps)),
            edges_target,
            weights_target,
            blend_target,
            inv_size,
        });
        register_texture_update(state.blend_target.texture().id(), &state);
        Self(state)
    }

    /// `passTexture( this, this._renderTargetBlend.texture )` at `uv()`.
    pub fn node(&self) -> NodeRef {
        texture_uv(&self.0.blend_target.texture(), uv())
    }

    /// The edges pass's material, for the WGSL gate.
    #[doc(hidden)]
    pub fn edges_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.edges_quad.material
    }

    /// The weights pass's material, for the WGSL gate.
    #[doc(hidden)]
    pub fn weights_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.weights_quad.material
    }

    /// The blend pass's material, for the WGSL gate.
    #[doc(hidden)]
    pub fn blend_material(&self) -> &MeshBasicNodeMaterial {
        &self.0.blend_quad.material
    }
}

impl SmaaState {
    /// `SMAANode.setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
        self.edges_target.set_size(width, height);
        self.weights_target.set_size(width, height);
        self.blend_target.set_size(width, height);
    }
}

impl NodeUpdate for SmaaState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `SMAANode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The input first: three's `setup()` builds it before the node, so it
        // is earlier in the frame's update-before list.
        if let Some(input) = crate::nodes::frame::texture_update(self.input.id()) {
            renderer.update_before_node(&input);
        }

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`.
        let previous_target = renderer.render_target();
        let previous_level = renderer.active_mipmap_level();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);

        renderer.set_render_target(Some(self.edges_target.clone()));
        renderer.render_quad(&self.edges_quad);

        renderer.set_render_target(Some(self.weights_target.clone()));
        renderer.render_quad(&self.weights_quad);

        renderer.set_render_target(Some(self.blend_target.clone()));
        renderer.render_quad(&self.blend_quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target_level(previous_target, previous_level);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// `max( t.r, t.g, t.b )` of `t = abs( C.sub( other ) )`, which three spells
/// as a `let` because `t` is read three times.
#[inline(never)]
fn colour_delta(c: &NodeRef, other: &NodeRef) -> NodeRef {
    let t = to_const(None, abs(c.sub(other)));
    t.x().max(t.y()).max(t.z())
}

/// `SMAAEdgeDetection`.
#[inline(never)]
fn edge_detection(taps: &Taps) -> NodeRef {
    let v_offset0 = taps.offset(-1.0, 0.0, 0.0, -1.0);
    let v_offset1 = taps.offset(1.0, 0.0, 0.0, 1.0);
    let v_offset2 = taps.offset(-2.0, 0.0, 0.0, -2.0);

    let threshold = vec2(THRESHOLD, THRESHOLD);
    let mut statements = Vec::new();

    // `vec4()` is `( 0, 0, 0, 1 )`.
    let delta = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let c = to_var(None, taps.input(uv()).rgb());
    statements.extend([delta.clone(), c.clone()]);

    // Left and top.
    let c_left = to_var(None, taps.input(v_offset0.xy()).rgb());
    statements.push(c_left.clone());
    statements.push(delta.x().assign(colour_delta(&c, &c_left)));
    let c_top = to_var(None, taps.input(v_offset0.zw()).rgb());
    statements.push(c_top.clone());
    statements.push(delta.y().assign(colour_delta(&c, &c_top)));

    let edges = to_var(None, step(threshold, delta.xy()));
    statements.push(edges.clone());
    statements.push(if_then(
        dot(edges.clone(), vec2(1.0, 1.0)).equal(float(0.0)),
        vec![discard()],
    ));

    // Right and bottom.
    let c_right = to_var(None, taps.input(v_offset1.xy()).rgb());
    statements.push(c_right.clone());
    statements.push(delta.z().assign(colour_delta(&c, &c_right)));
    let c_bottom = to_var(None, taps.input(v_offset1.zw()).rgb());
    statements.push(c_bottom.clone());
    statements.push(delta.w().assign(colour_delta(&c, &c_bottom)));

    let max_delta = to_var(
        None,
        max(max(max(delta.x(), delta.y()), delta.z()), delta.w()),
    );
    statements.push(max_delta.clone());

    // Left-left and top-top.
    let c_left_left = to_var(None, taps.input(v_offset2.xy()).rgb());
    statements.push(c_left_left.clone());
    statements.push(delta.z().assign(colour_delta(&c, &c_left_left)));
    let c_top_top = to_var(None, taps.input(v_offset2.zw()).rgb());
    statements.push(c_top_top.clone());
    statements.push(delta.w().assign(colour_delta(&c, &c_top_top)));

    // `maxDelta = max( maxDelta, delta.z, delta.w )` rebinds the JS name, so
    // the final maximum is an expression, not an assignment.
    let max_delta = max(max(max_delta, delta.z()), delta.w());

    // Local contrast adaptation.
    statements.push(edges.mul_assign(step(float(0.5).mul(max_delta).to(Type::Vec2), delta.xy())));

    block(statements, vec4_join(vec![edges, float(0.0), float(0.0)]))
}

/// Which way an `SMAASearch*` walks.
#[derive(Clone, Copy)]
enum Search {
    XLeft,
    XRight,
    YUp,
    YDown,
}

impl Search {
    /// `e`'s start, `vec2( 0, 1 )` for horizontal searches and `vec2( 1, 0 )`
    /// for vertical ones.
    fn start(self) -> NodeRef {
        match self {
            Search::XLeft | Search::XRight => vec2(0.0, 1.0),
            Search::YUp | Search::YDown => vec2(1.0, 0.0),
        }
    }

    /// The searched axis of a vec2.
    fn axis(self, v: &NodeRef) -> NodeRef {
        match self {
            Search::XLeft | Search::XRight => v.x(),
            Search::YUp | Search::YDown => v.y(),
        }
    }
}

/// `SMAASearchLength( searchTex, e, bias, scale )`.
#[inline(never)]
fn search_length(taps: &Taps, e: NodeRef, bias: f64, scale: f64) -> NodeRef {
    let coord = to_var(None, e);
    let set = coord
        .x()
        .assign(float(bias).add(coord.x().mul(float(scale))));
    block(
        vec![coord.clone(), set],
        float(255.0)
            .to(Type::Vec4)
            .mul(texture_with_uv(&taps.search, coord))
            .x(),
    )
}

/// `SMAASearchXLeft` / `XRight` / `YUp` / `YDown( edgesTex, searchTex,
/// texcoord, end )`.
#[inline(never)]
fn search(taps: &Taps, which: Search, texcoord: NodeRef, end: NodeRef) -> NodeRef {
    let inv = &taps.inv_size;
    let e = to_var(None, which.start());
    let coord = to_var(None, texcoord);
    let mut statements = vec![e.clone(), coord.clone()];

    let step = match which {
        Search::XLeft => coord.sub_assign(vec2(2.0, 0.0).mul(inv)),
        Search::XRight => coord.add_assign(vec2(2.0, 0.0).mul(inv)),
        Search::YUp => coord.add_assign(vec2(0.0, -2.0).mul(inv)),
        Search::YDown => coord.sub_assign(vec2(0.0, -2.0).mul(inv)),
    };
    let reached_end = match which {
        Search::XLeft | Search::YUp => which.axis(&coord).less_than_equal(end),
        Search::XRight | Search::YDown => which.axis(&coord).greater_than_equal(end),
    };
    // The edge across the walk stops it: `e.g` for horizontal searches.
    let (along, across) = match which {
        Search::XLeft | Search::XRight => (e.y(), e.x()),
        Search::YUp | Search::YDown => (e.x(), e.y()),
    };
    let stop = reached_end.or(along
        .less_than_equal(float(0.8281))
        .or(across.not_equal(float(0.0))));
    statements.push(loop_options(
        "i",
        Type::I32,
        int(0),
        int(MAX_SEARCH_STEPS),
        "<",
        |_| {
            vec![
                e.assign(taps.edges(coord.clone()).xy()),
                step,
                if_then(stop, vec![break_loop()]),
            ]
        },
    ));

    let axis = which.axis(&coord);
    let inv_axis = which.axis(inv);
    // Undo the (-0.25, -0.125) offset, the search bias of one, and the length
    // the last step added.
    let backwards = matches!(which, Search::XLeft | Search::YUp);
    for amount in [
        float(0.25).mul(&inv_axis),
        inv_axis.clone(),
        float(2.0).mul(&inv_axis),
    ] {
        statements.push(if backwards {
            axis.add_assign(amount)
        } else {
            axis.sub_assign(amount)
        });
    }
    let (searched, bias) = match which {
        Search::XLeft => (e.clone(), 0.0),
        Search::XRight => (e.clone(), 0.5),
        Search::YUp => (e.swizzle("yx"), 0.0),
        Search::YDown => (e.swizzle("yx"), 0.5),
    };
    let length = inv_axis.mul(search_length(taps, searched, bias, 0.5));
    statements.push(if backwards {
        axis.sub_assign(length)
    } else {
        axis.add_assign(length)
    });

    block(statements, axis)
}

/// `SMAAArea( areaTex, dist, e1, e2, offset )`.
#[inline(never)]
fn area(taps: &Taps, dist: NodeRef, e1: NodeRef, e2: NodeRef, offset: NodeRef) -> NodeRef {
    let pixel_size = || vec2(AREATEX_PIXEL_SIZE.0, AREATEX_PIXEL_SIZE.1);
    // Rounding prevents precision errors of bilinear filtering; then the
    // scale and bias into texel space.
    let texcoord = float(AREATEX_MAX_DISTANCE)
        .mul(float(4.0).mul(vec2_join(vec![e1, e2])).round())
        .add(dist);
    let texcoord = to_var(
        None,
        pixel_size().mul(texcoord).add(float(0.5).mul(pixel_size())),
    );
    // The subpixel offset's row.
    let shift = texcoord
        .y()
        .add_assign(float(AREATEX_SUBTEX_SIZE).mul(offset));
    block(
        vec![texcoord.clone(), shift],
        texture_with_uv(&taps.area, texcoord).xy(),
    )
}

/// One `If( e.g.greaterThan( 0 ) )` (north) or `If( e.r.greaterThan( 0 ) )`
/// (west) block of `SMAAWeights`, writing `weights.rg` or `weights.ba`.
struct Direction {
    horizontal: bool,
    v_offset0: NodeRef,
    v_offset1: NodeRef,
    v_offset2: NodeRef,
    v_pixcoord: NodeRef,
    weights: NodeRef,
    subsample_indices: NodeRef,
}

#[inline(never)]
fn weights_direction(taps: &Taps, dir: &Direction) -> Vec<NodeRef> {
    let inv = &taps.inv_size;
    let (first, second, end_first, end_second, along) = if dir.horizontal {
        (
            (Search::XLeft, dir.v_offset0.xy()),
            (Search::XRight, dir.v_offset0.zw()),
            dir.v_offset2.x(),
            dir.v_offset2.y(),
            dir.v_offset1.y(),
        )
    } else {
        (
            (Search::YUp, dir.v_offset1.xy()),
            (Search::YDown, dir.v_offset1.zw()),
            dir.v_offset2.z(),
            dir.v_offset2.w(),
            dir.v_offset0.x(),
        )
    };
    // The searched component of a coordinate, and the other one.
    let searched = |v: &NodeRef| if dir.horizontal { v.x() } else { v.y() };
    let crossing = |v: &NodeRef| if dir.horizontal { v.y() } else { v.x() };
    let edge = |v: NodeRef| if dir.horizontal { v.x() } else { v.y() };

    let d = to_var(None, vec2(0.0, 0.0));
    let mut statements = vec![d.clone()];

    // The distance to the left (top).
    let coords_first = to_var(None, vec2(0.0, 0.0));
    statements.push(coords_first.clone());
    statements.push(searched(&coords_first).assign(search(taps, first.0, first.1, end_first)));
    statements.push(crossing(&coords_first).assign(along.clone()));
    statements.push(d.x().assign(searched(&coords_first)));

    // The crossing edges there, two at a time through bilinear filtering.
    let e1 = to_var(None, edge(taps.edges(coords_first)));
    statements.push(e1.clone());

    // The distance to the right (bottom).
    let coords_second = to_var(None, vec2(0.0, 0.0));
    statements.push(coords_second.clone());
    statements.push(searched(&coords_second).assign(search(taps, second.0, second.1, end_second)));
    statements.push(crossing(&coords_second).assign(along));
    statements.push(d.y().assign(searched(&coords_second)));

    // In pixels; the area texture is compressed quadratically.
    let inv_axis = if dir.horizontal { inv.x() } else { inv.y() };
    let pix_axis = if dir.horizontal {
        dir.v_pixcoord.x()
    } else {
        dir.v_pixcoord.y()
    };
    let d_pixels = d.div(inv_axis.to(Type::Vec2)).sub(pix_axis.to(Type::Vec2));
    let sqrt_d = sqrt(abs(d_pixels));

    let neighbour = if dir.horizontal {
        vec2(1.0, 0.0)
    } else {
        vec2(0.0, 1.0)
    };
    let e2 = to_var(
        None,
        edge(taps.edges(coords_second.add(neighbour.mul(inv)))),
    );
    statements.push(e2.clone());

    if dir.horizontal {
        statements.push(dir.weights.x().assign(e2.clone()));
        statements.push(dir.weights.xy().assign(area(
            taps,
            sqrt_d,
            e1,
            e2,
            dir.subsample_indices.y(),
        )));
    } else {
        statements.push(dir.weights.zw().assign(area(
            taps,
            sqrt_d,
            e1,
            e2,
            dir.subsample_indices.x(),
        )));
    }
    statements
}

/// `SMAAWeights`.
#[inline(never)]
fn weights(taps: &Taps) -> NodeRef {
    let inv = &taps.inv_size;
    let v_pixcoord = to_varying(None, uv().xy().div(inv));
    // Offsets for the searches (see `@PSEUDO_GATHER4`).
    let v_offset0 = taps.offset(-0.25, -0.125, 1.25, -0.125);
    let v_offset1 = taps.offset(-0.125, -0.25, -0.125, 1.25);
    // And the ends of the searches' loops.
    let v_offset2 = to_varying(
        None,
        vec4_join(vec![v_offset0.xz(), v_offset1.swizzle("yw")]).add(
            vec4(-2.0, 2.0, -2.0, 2.0)
                .mul(vec4_join(vec![inv.swizzle("xx"), inv.swizzle("yy")]))
                .mul(float(MAX_SEARCH_STEPS as f64).to(Type::Vec4)),
        ),
    );

    let weights = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let subsample_indices = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    let e = to_var(None, taps.edges(uv()).xy());
    let direction = |horizontal| Direction {
        horizontal,
        v_offset0: v_offset0.clone(),
        v_offset1: v_offset1.clone(),
        v_offset2: v_offset2.clone(),
        v_pixcoord: v_pixcoord.clone(),
        weights: weights.clone(),
        subsample_indices: subsample_indices.clone(),
    };

    let statements = vec![
        weights.clone(),
        subsample_indices.clone(),
        e.clone(),
        // Edge at north.
        if_then(
            e.y().greater_than(float(0.0)),
            weights_direction(taps, &direction(true)),
        ),
        // Edge at west.
        if_then(
            e.x().greater_than(float(0.0)),
            weights_direction(taps, &direction(false)),
        ),
    ];
    block(statements, weights)
}

/// `SMAABlend`.
#[inline(never)]
fn blend(taps: &Taps) -> NodeRef {
    let inv = &taps.inv_size;
    let v_offset1 = taps.offset(1.0, 0.0, 0.0, 1.0);

    let result = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let a = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let mut statements = vec![
        result.clone(),
        a.clone(),
        a.xz().assign(taps.weights(uv()).xz()),
        a.y().assign(taps.weights(v_offset1.zw()).y()),
        a.w().assign(taps.weights(v_offset1.xy()).w()),
    ];

    // Up to four lines can cross a pixel; favour the heaviest in each
    // direction, then go the way with the larger weight.
    let offset = to_var(None, vec2(0.0, 0.0));
    let horizontal = || offset.x().abs().greater_than(offset.y().abs());
    let c = to_var(None, taps.input(uv()));
    let texcoord = to_var(None, uv());
    let c_op = to_var(None, taps.input(texcoord.clone()));
    let s = to_var(
        None,
        horizontal().select(offset.x().abs(), offset.y().abs()),
    );
    let blended = vec![
        offset.clone(),
        offset
            .x()
            .assign(a.w().greater_than(a.z()).select(a.w(), a.z().negate())),
        offset
            .y()
            .assign(a.y().greater_than(a.x()).select(a.y(), a.x().negate())),
        if_else(
            horizontal(),
            vec![offset.y().assign(float(0.0))],
            vec![offset.x().assign(float(0.0))],
        ),
        c.clone(),
        texcoord.clone(),
        texcoord.add_assign(sign(offset.clone()).mul(inv)),
        c_op.clone(),
        s.clone(),
        result.assign(mix(c, c_op, s)),
    ];

    statements.push(if_else(
        dot(a.clone(), vec4(1.0, 1.0, 1.0, 1.0)).less_than(float(1e-5)),
        vec![result.assign(taps.input(uv()))],
        blended,
    ));
    block(statements, result)
}
