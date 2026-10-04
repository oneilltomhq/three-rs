//! Port of `three.js/examples/jsm/tsl/display/DepthOfFieldNode.js` — bokeh
//! depth of field in nine full-screen draws.
//!
//! `dof( textureNode, viewZNode, focusDistance, focalLength, bokehScale )`
//! owns six render targets. Every frame it:
//!
//! 1. writes the circle of confusion, split into a near and a far field, into
//!    a two-attachment red `HalfFloat` target (`outputStruct( near, far )`);
//! 2. blurs the near field with a `gaussianBlur( …, 1, 2 )` into a
//!    half-resolution red target, so the near field's edge does not alias
//!    where it is blended over the background;
//! 3. gathers 64 Vogel-disc taps of the input around each pixel, scaled by
//!    the pixel's CoC, once for each field (`blur64`), then spreads each
//!    result with a 16-tap max filter (`blur16`);
//! 4. composites the far and then the near field over the input.
//!
//! The divergences are the ones `bloom.rs` and `traa.rs` record:
//!
//! * three's `updateBefore()` runs from inside the draw that reaches the
//!   node; the port's state implements [`NodeUpdate`] and is registered as
//!   the updater of the composite texture, so the first draw in a frame that
//!   samples [`DepthOfFieldNode::node`] runs the nine draws. It asks for the
//!   input's pass first, as `TRAANode`'s port does.
//! * three swaps `_CoCTextureNode.value` between the passes, so one
//!   `_blur64Material` serves the near and the far field. A texture is an
//!   identity in the port's graphs, so there are two blur64 materials, one per
//!   CoC texture, with the same WGSL. `_blur16Material` reads the same
//!   texture for both fields and stays one material drawn twice.
//! * the quad materials are built in [`DepthOfFieldNode::new`], not in a
//!   `setup()` under `context( builder.getSharedContext() )`; each quad is its
//!   own build anyway.
//! * the near-field Gaussian is the port's [`GaussianBlurNode`], built over
//!   the near-field texture rather than `_CoCTextureNode`, and run by this
//!   node's update rather than by its own.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    block, float, int, loop_n, max, mix, output_struct, property, smoothstep, step, texture,
    texture_uv, texture_with_uv, to_var, to_var_intent, uniform_array_vec2, uniform_settable, uv,
    vec2, vec3, vec4, vec4_join, UniformArray,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

use super::gaussian_blur::{GaussianBlurNode, GaussianBlurOptions};

/// `dof( node, viewZNode, focusDistance, focalLength, bokehScale )`.
///
/// Three.js takes the input as a texture node (`scenePass.getTextureNode()`);
/// the port takes its texture, as [`gaussian_blur`](super::gaussian_blur) and [`traa`](super::traa)
/// do. `view_z` is the scene's view-space depth (`scenePass.getViewZNode()`);
/// the other three are the `uniform()`s the page drives from its GUI:
/// `focus_distance` is the distance along the camera's look direction that is
/// sharp, `focal_length` how far from it a surface goes fully out of focus,
/// both in world units, and `bokeh_scale` an artistic size factor.
pub fn dof(
    map: &Texture,
    view_z: NodeRef,
    focus_distance: NodeRef,
    focal_length: NodeRef,
    bokeh_scale: NodeRef,
) -> DepthOfFieldNode {
    DepthOfFieldNode::new(map, view_z, focus_distance, focal_length, bokeh_scale)
}

/// `new DepthOfFieldNode( textureNode, viewZNode, focusDistanceNode,
/// focalLengthNode, bokehScaleNode )`.
pub struct DepthOfFieldNode(Rc<DofState>);

/// What `updateBefore()` needs: the targets, the quads and the size uniform.
struct DofState {
    /// `this.textureNode.value` — the input, which also sizes the effect.
    map: Texture,
    /// `this._invSize`.
    inv_size: SettableValue,
    /// `this._CoCRT` — near field in attachment 0, far field in 1.
    coc: RenderTarget,
    /// `this._CoCBlurredRT`.
    coc_blurred: RenderTarget,
    /// `this._blur64RT`.
    blur64: RenderTarget,
    /// `this._blur16NearRT`.
    blur16_near: RenderTarget,
    /// `this._blur16FarRT`.
    blur16_far: RenderTarget,
    /// `this._compositeRT`.
    composite: RenderTarget,
    /// `this._CoCBlurNode` — `gaussianBlur( this._CoCTextureNode, 1, 2 )` over
    /// the near field.
    coc_blur: GaussianBlurNode,
    /// `this._CoCMaterial`'s quad.
    coc_quad: QuadMesh,
    /// `this._CoCBlurredMaterial`'s quad.
    coc_blurred_quad: QuadMesh,
    /// `this._blur64Material` with `_CoCTextureNode` on the blurred near field.
    blur64_near_quad: QuadMesh,
    /// `this._blur64Material` with `_CoCTextureNode` on the far field.
    blur64_far_quad: QuadMesh,
    /// `this._blur16Material`'s quad, drawn once per field.
    blur16_quad: QuadMesh,
    /// `this._compositeMaterial`'s quad.
    composite_quad: QuadMesh,
    /// `this._textureNode` — `texture( this._compositeRT.texture )`.
    node: NodeRef,
}

/// `DepthOfFieldNode._generateKernels()`: Vogel's method — 80 points spread
/// evenly over the unit disc along the golden angle, every fifth one into the
/// 16-point kernel and the rest into the 64-point one. Returns
/// `( points16, points64 )`.
fn generate_kernels() -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    const GOLDEN_ANGLE: f64 = 2.39996323;
    const SAMPLES: u32 = 80;
    let mut points16 = Vec::with_capacity(16);
    let mut points64 = Vec::with_capacity(64);
    for i in 0..SAMPLES {
        let theta = i as f64 * GOLDEN_ANGLE;
        let r = (i as f64).sqrt() / (SAMPLES as f64).sqrt();
        let p = [r * theta.cos(), r * theta.sin()];
        if i % 5 == 0 {
            points16.push(p);
        } else {
            points64.push(p);
        }
    }
    (points16, points64)
}

/// `Math.round( size / 2 )` — the blur passes run at half resolution.
fn half_size(size: u32) -> u32 {
    (size as f64 / 2.0).round() as u32
}

/// `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`.
fn target() -> RenderTarget {
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
    .expect("three-rs: a depth-of-field target is a HalfFloat colour type")
}

/// The same with `format: RedFormat` and `count` attachments.
fn red_target(count: usize) -> RenderTarget {
    let target = target();
    target.set_red_format();
    target.set_count(count);
    target
}

fn quad(material: MeshBasicNodeMaterial) -> QuadMesh {
    QuadMesh::new(material)
}

impl DepthOfFieldNode {
    /// `new DepthOfFieldNode( … )`, with the materials `setup()` gives it.
    pub fn new(
        map: &Texture,
        view_z: NodeRef,
        focus_distance: NodeRef,
        focal_length: NodeRef,
        bokeh_scale: NodeRef,
    ) -> Self {
        let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);

        let coc = red_target(2);
        let coc_blurred = red_target(1);
        let blur64 = target();
        let blur16_near = target();
        let blur16_far = target();
        let composite = target();

        let coc_textures = coc.textures();
        let (near_field, far_field) = (&coc_textures[0], &coc_textures[1]);

        // `gaussianBlur( this._CoCTextureNode, 1, 2 )`, whose texture node
        // holds the near field when `_CoCBlurredMaterial` is drawn.
        // `directionNode` is the literal `1`, which `vec2()` folds into a
        // `vec2( 1, 1 )` constant. The input is a `texture()` node, so unlike
        // a pass texture its taps go through the uv matrix.
        let coc_blur = GaussianBlurNode::with_uv_matrix(
            near_field,
            Some(vec2(1.0, 1.0)),
            2,
            GaussianBlurOptions::default(),
        );

        let (points16, points64) = generate_kernels();
        let bokeh64 = uniform_array_vec2(&points64);
        let bokeh16 = uniform_array_vec2(&points16);

        let mut coc_material = MeshBasicNodeMaterial::new();
        coc_material.name = "DoF.CoC";
        let (color, output) = coc_node(view_z, focus_distance, focal_length);
        coc_material.color_node = Some(color);
        coc_material.output_node = Some(output);

        // `this._CoCBlurredMaterial.colorNode = this._CoCBlurNode`.
        let mut coc_blurred_material = MeshBasicNodeMaterial::new();
        coc_blurred_material.name = "DoF.CoCBlurred";
        coc_blurred_material.color_node = Some(coc_blur.node());

        let blur64_material = |coc_map: &Texture| {
            let mut material = MeshBasicNodeMaterial::new();
            material.name = "DoF.Blur64";
            material.fragment_node = Some(blur64_node(
                map,
                coc_map,
                &bokeh64,
                inv_size_node.clone(),
                bokeh_scale.clone(),
            ));
            quad(material)
        };
        let blur64_near_quad = blur64_material(&coc_blurred.texture());
        let blur64_far_quad = blur64_material(far_field);

        let mut blur16_material = MeshBasicNodeMaterial::new();
        blur16_material.name = "DoF.Blur16";
        blur16_material.fragment_node = Some(blur16_node(
            &blur64.texture(),
            &bokeh16,
            inv_size_node,
            bokeh_scale,
        ));

        let mut composite_material = MeshBasicNodeMaterial::new();
        composite_material.name = "DoF.Composite";
        composite_material.fragment_node = Some(composite_node(
            map,
            &blur16_near.texture(),
            &blur16_far.texture(),
        ));

        let node = texture(&composite.texture());

        let state = Rc::new(DofState {
            map: map.clone(),
            inv_size,
            coc,
            coc_blurred,
            blur64,
            blur16_near,
            blur16_far,
            composite,
            coc_blur,
            coc_quad: quad(coc_material),
            coc_blurred_quad: quad(coc_blurred_material),
            blur64_near_quad,
            blur64_far_quad,
            blur16_quad: quad(blur16_material),
            composite_quad: quad(composite_material),
            node,
        });
        crate::nodes::frame::register_texture_update(state.composite.texture().id(), &state);
        Self(state)
    }

    /// `dofNode.getTextureNode()` — the composited frame, for the graph
    /// downstream (usually the pipeline's `outputNode`).
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._compositeRT.texture`.
    pub fn texture(&self) -> Texture {
        self.0.composite.texture()
    }

    /// `DepthOfFieldNode.setSize( width, height )` — what `updateBefore()`
    /// calls with the input's size every frame.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }

    /// The quad materials in draw order — CoC, the near field's Gaussian
    /// (horizontal, vertical), CoC blur, blur64 near, blur64 far, blur16,
    /// composite — for `examples/dump_wgsl.rs` and the dump gate.
    #[doc(hidden)]
    pub fn quad_materials(&self) -> Vec<&MeshBasicNodeMaterial> {
        let s = &self.0;
        let gaussian = s.coc_blur.quad_materials();
        vec![
            &s.coc_quad.material,
            gaussian[0],
            gaussian[1],
            &s.coc_blurred_quad.material,
            &s.blur64_near_quad.material,
            &s.blur64_far_quad.material,
            &s.blur16_quad.material,
            &s.composite_quad.material,
        ]
    }
}

impl DofState {
    /// `DepthOfFieldNode.setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
        self.coc.set_size(width, height);
        self.composite.set_size(width, height);

        let (half_x, half_y) = (half_size(width), half_size(height));
        self.coc_blurred.set_size(half_x, half_y);
        self.blur64.set_size(half_x, half_y);
        self.blur16_near.set_size(half_x, half_y);
        self.blur16_far.set_size(half_x, half_y);
    }

    fn draw(renderer: &mut Renderer, target: &RenderTarget, quad: &QuadMesh) {
        renderer.set_render_target(Some(target.clone()));
        renderer.render_quad(quad);
    }
}

impl NodeUpdate for DofState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `DepthOfFieldNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The input's pass first, so `map` has its size and this frame's
        // pixels (see `traa.rs`).
        if let Some(pass) = crate::nodes::frame::texture_update(self.map.id()) {
            renderer.update_before_node(&pass);
        }

        // resize
        let (width, height) = self.map.size();
        self.set_size(width, height);

        // `_rendererState = RendererUtils.resetRendererState( renderer, … )`,
        // then `renderer.setClearColor( 0x000000, 0 )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.auto_clear = true;
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 0.0);

        // coc
        Self::draw(renderer, &self.coc, &self.coc_quad);

        // blur near field to avoid visible aliased edges when the near field
        // is blended with the background. Three's Gaussian runs from the
        // `updateBefore()` of the node the CoC-blur material reaches; the port
        // runs it here, just before that material is drawn.
        self.coc_blur.render(renderer);
        Self::draw(renderer, &self.coc_blurred, &self.coc_blurred_quad);

        // blur64 near, blur16 near
        Self::draw(renderer, &self.blur64, &self.blur64_near_quad);
        Self::draw(renderer, &self.blur16_near, &self.blur16_quad);

        // blur64 far, blur16 far
        Self::draw(renderer, &self.blur64, &self.blur64_far_quad);
        Self::draw(renderer, &self.blur16_far, &self.blur16_quad);

        // composite
        Self::draw(renderer, &self.composite, &self.composite_quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// The `CoC` `Fn` and the `outputStruct( nearField, farField )` it fills —
/// `_CoCMaterial`'s `colorNode` and `outputNode`.
///
/// `nearField` and `farField` are `property( 'float' )`s the colour node
/// assigns as a side effect; the colour itself is `float( 0 )`.
fn coc_node(view_z: NodeRef, focus_distance: NodeRef, focal_length: NodeRef) -> (NodeRef, NodeRef) {
    let near_field = property("DoFNearField", Type::F32);
    let far_field = property("DoFFarField", Type::F32);

    let signed_dist = view_z.negate().sub(focus_distance);
    let coc = smoothstep(float(0.0), focal_length, signed_dist.abs());
    let color = block(
        vec![
            near_field.assign(step(signed_dist.clone(), float(0.0)).mul(coc.clone())),
            far_field.assign(step(float(0.0), signed_dist).mul(coc)),
        ],
        float(0.0),
    );
    (color, output_struct(vec![near_field, far_field]))
}

/// The `blur64` `Fn`: 64 Vogel taps of the input, spread by the pixel's CoC,
/// averaged; the CoC rides along in alpha.
fn blur64_node(
    map: &Texture,
    coc_map: &Texture,
    bokeh64: &UniformArray,
    inv_size: NodeRef,
    bokeh_scale: NodeRef,
) -> NodeRef {
    let acc = to_var_intent(vec3(0.0, 0.0, 0.0));
    let uv_node = uv();
    // `this._CoCTextureNode.sample( uvNode ).r` — a red map's node already
    // is the `float`.
    let coc = texture_with_uv(coc_map, uv_node.clone());
    let sample_step = inv_size.mul(bokeh_scale).mul(coc.clone());

    let gather = loop_n("i", int(64), |i| {
        let s_uv = uv_node
            .clone()
            .add(sample_step.mul(bokeh64.element_xy(i.clone())));
        let tap = texture_uv(map, s_uv);
        vec![acc.add_assign(tap.xyz())]
    });

    // `const acc = vec3()`, assigned to: declared ahead of the loop.
    block(
        vec![acc.clone(), gather, acc.div_assign(float(64.0))],
        vec4_join(vec![acc, coc]),
    )
}

/// The `blur16` `Fn`: a 16-tap max filter over `blur64`'s result, which
/// fills the gaps between the 64 taps.
fn blur16_node(
    blur64: &Texture,
    bokeh16: &UniformArray,
    inv_size: NodeRef,
    bokeh_scale: NodeRef,
) -> NodeRef {
    let uv_node = uv();
    let col = to_var(None, texture_with_uv(blur64, uv_node.clone()));
    let max_val = col.xyz();
    let coc = col.w();
    let sample_step = inv_size.mul(bokeh_scale).mul(coc.clone());

    let spread = loop_n("i", int(16), |i| {
        let s_uv = uv_node
            .clone()
            .add(sample_step.mul(bokeh16.element_xy(i.clone())));
        let tap = texture_with_uv(blur64, s_uv);
        vec![max_val.assign(max(tap.xyz(), max_val.clone()))]
    });

    block(vec![col.clone(), spread], vec4_join(vec![max_val, coc]))
}

/// The `composite` `Fn`: the far field over the input, then the near field
/// over that, each blended by its own CoC.
fn composite_node(map: &Texture, blur16_near: &Texture, blur16_far: &Texture) -> NodeRef {
    let uv_node = uv();
    let near = texture_with_uv(blur16_near, uv_node.clone());
    let far = texture_with_uv(blur16_far, uv_node.clone());
    let beauty = texture_uv(map, uv_node);

    // Three's TODO: the bokeh scale is not applied to the blend factors, so a
    // partly out-of-focus object (CoC between 0 and 1) blurs less.
    let blend_near = near.w().min(float(0.5)).mul(float(2.0));
    let blend_far = far.w().min(float(0.5)).mul(float(2.0));

    let result = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    block(
        vec![
            result.clone(),
            result.xyz().assign(mix(beauty.xyz(), far.xyz(), blend_far)),
            result
                .xyz()
                .assign(mix(result.xyz(), near.xyz(), blend_near)),
        ],
        result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kernels_split_80_vogel_points_16_and_64() {
        let (p16, p64) = generate_kernels();
        assert_eq!((p16.len(), p64.len()), (16, 64));
        // i = 0 is the centre and goes to points16; i = 1 is the first of
        // points64, at r = sqrt( 1 / 80 ) along the golden angle.
        assert_eq!(p16[0], [0.0, 0.0]);
        let r = 1.0 / 80f64.sqrt();
        assert_eq!(p64[0], [r * 2.39996323f64.cos(), r * 2.39996323f64.sin()]);
        // i = 79, the last point, is on points64 at r = sqrt( 79 / 80 ).
        let last = p64[63];
        assert!(((last[0].hypot(last[1])) - (79f64 / 80.0).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn half_size_rounds() {
        assert_eq!(half_size(1024), 512);
        assert_eq!(half_size(5), 3);
        assert_eq!(half_size(1), 1);
    }
}
