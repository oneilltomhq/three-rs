//! Port of `three.js/examples/jsm/tsl/display/BilateralBlurNode.js` — an
//! edge-preserving, separable blur in two full-screen passes.
//!
//! Each tap is weighted twice: by its distance from the centre (an
//! un-normalised Gaussian, `0.39894 · exp( -i² / 2σ² ) / σ`) and by how far its
//! luminance is from the centre's (`exp( -Δ² / 2σc² )`), and the sum is divided
//! by the total weight. `webgpu_postprocessing_godrays` blurs its ray march
//! with it, so the rays soften without bleeding over the pillars' silhouettes.
//!
//! Divergences from three.js, none of which reaches the generated WGSL:
//!
//! * **Two materials, not one.** Three builds one material and swaps its
//!   `textureNode.value` between the passes, writing `_passDirection` before
//!   each. The port's texture nodes are immutable, so it builds the material
//!   twice — once over the input, once over the horizontal target — each with
//!   its own `passDirection` uniform fixed at `( 1, 0 )` or `( 0, 1 )`. Both
//!   programs are the one three builds.
//! * **The input renders first.** Three sizes the targets from
//!   `map.image.width` before either pass draws; the port runs the input's own
//!   `updateBefore()` first (as [`TraaNode`](super::TraaNode) does for its
//!   pass) so that size is this frame's even on the first frame, when the
//!   input's target would otherwise still be 1×1.
//! * **The input is a texture**, as for [`gaussian_blur`](super::gaussian_blur):
//!   the caller does three's `convertToTexture()`.
//! * **`sigma` is a whole number.** Three's `sigma` is any number, and the
//!   kernel is `sigma * 2 + 3` taps, so a fractional sigma gives a fractional
//!   loop bound. The port takes a `u32`; the one page that uses the blur
//!   takes the default, 4.
//!
//! Not ported: `dispose()`, the texture-type copy `updateBefore()` does
//! every frame (`_horizontalRT.texture.type = map.type`), which the port does
//! once, at construction, as `GaussianBlurNode` does, and the shared
//! `builder.getSharedContext()` the material is given, which the port's
//! per-material builds have no use for.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    abs, block, exp, float, luminance, max, texture_uv, to_const, to_var, uniform_settable, uv,
    vec2,
};
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

/// `bilateralBlur( node, directionNode, sigma, sigmaColor )`.
///
/// `direction` is `directionNode` (`None` is three's `null`, i.e. `vec2( 1 )`),
/// `sigma` the spatial sigma in texels (three's default is 4) and
/// `sigma_color` the luminance sigma (default 0.1).
pub fn bilateral_blur(
    map: &Texture,
    direction: Option<NodeRef>,
    sigma: u32,
    sigma_color: f64,
) -> BilateralBlurNode {
    BilateralBlurNode::new(map, direction, sigma, sigma_color)
}

/// `BilateralBlurNode` — a handle; the state is shared with the renderer's
/// update-before registry.
pub struct BilateralBlurNode(Rc<BilateralBlurState>);

/// What a [`BilateralBlurNode`] shares with the renderer.
pub(crate) struct BilateralBlurState {
    /// `this.textureNode.value`.
    map: Texture,
    /// `this._horizontalRT`.
    horizontal: RenderTarget,
    /// `this._verticalRT`.
    vertical: RenderTarget,
    /// The material over the input, `passDirection = ( 1, 0 )`.
    horizontal_quad: QuadMesh,
    /// The material over `horizontal`, `passDirection = ( 0, 1 )`.
    vertical_quad: QuadMesh,
    /// `this._invSize`, shared by both passes.
    inv_size: SettableValue,
    /// `this.resolutionScale`.
    resolution_scale: f64,
    /// `this._textureNode` — `passTexture( this, this._verticalRT.texture )`.
    node: NodeRef,
}

/// `BilateralBlurNode._getSpatialCoefficients( kernelRadius )` — not
/// normalised; the shader divides by the weight it accumulates instead.
fn spatial_coefficients(kernel_radius: u32) -> Vec<f64> {
    let sigma = kernel_radius as f64 / 3.0;
    (0..kernel_radius)
        .map(|i| {
            let i = i as f64;
            0.39894 * (-0.5 * i * i / (sigma * sigma)).exp() / sigma
        })
        .collect()
}

/// `BilateralBlurNode.setSize()`: `Math.max( Math.round( size * scale ), 1 )`.
fn scaled_size(size: u32, resolution_scale: f64) -> u32 {
    ((size as f64 * resolution_scale).round() as u32).max(1)
}

impl BilateralBlurNode {
    /// `new BilateralBlurNode( textureNode, directionNode, sigma, sigmaColor )`,
    /// with the material `setup()` gives it.
    pub fn new(map: &Texture, direction: Option<NodeRef>, sigma: u32, sigma_color: f64) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false } )`, typed like the
        // input (see the module docs).
        let texture_type = match map.format() {
            wgpu::TextureFormat::Rgba16Float => TextureType::HalfFloat,
            _ => TextureType::UnsignedByte,
        };
        let target = || {
            RenderTarget::new_with_options(
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
            .expect("three-rs: a bilateral blur target is a colour type")
        };
        let horizontal = target();
        let vertical = target();

        let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        // `vec2( this.directionNode || 1 )`.
        let direction = match direction {
            Some(direction) => direction.to(Type::Vec2),
            None => vec2(1.0, 1.0),
        };

        let quad = |map: &Texture, pass_direction: [f64; 2]| {
            // `this._passDirection` — one per material here.
            let (pass_direction, _) = uniform_settable(Type::Vec2, pass_direction.to_vec());
            let mut material = MeshBasicNodeMaterial::new();
            material.name = "Bilateral_blur";
            material.fragment_node = Some(blur(
                map,
                direction.clone(),
                pass_direction,
                inv_size_node.clone(),
                sigma,
                sigma_color,
            ));
            QuadMesh::new(material)
        };
        let horizontal_quad = quad(map, [1.0, 0.0]);
        let vertical_quad = quad(&horizontal.texture(), [0.0, 1.0]);

        // `passTexture( this, this._verticalRT.texture )` with the input's
        // `uvNode`, `uv()` for every texture the port takes.
        let node = to_var(None, texture_uv(&vertical.texture(), uv()));

        let state = Rc::new(BilateralBlurState {
            map: map.clone(),
            horizontal,
            vertical,
            horizontal_quad,
            vertical_quad,
            inv_size,
            resolution_scale: 1.0,
            node,
        });
        register_texture_update(state.vertical.texture().id(), &state);
        Self(state)
    }

    /// `bilateralBlurNode.getTextureNode()` — the blurred texture.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// `this._verticalRT.texture`.
    pub fn texture(&self) -> Texture {
        self.0.vertical.texture()
    }

    /// The two quad materials, horizontal first — for `examples/dump_wgsl.rs`.
    #[doc(hidden)]
    pub fn quad_materials(&self) -> Vec<&MeshBasicNodeMaterial> {
        vec![
            &self.0.horizontal_quad.material,
            &self.0.vertical_quad.material,
        ]
    }

    /// `BilateralBlurNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        self.0.set_size(width, height);
    }
}

impl BilateralBlurState {
    fn set_size(&self, width: u32, height: u32) {
        let width = scaled_size(width, self.resolution_scale);
        let height = scaled_size(height, self.resolution_scale);
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
        self.horizontal.set_size(width, height);
        self.vertical.set_size(width, height);
    }
}

impl NodeUpdate for BilateralBlurState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `BilateralBlurNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        // The input first — see the module docs.
        if let Some(input) = crate::nodes::frame::texture_update(self.map.id()) {
            renderer.update_before_node(&input);
        }

        // `RendererUtils.resetRendererState( renderer, _rendererState )`.
        let previous_target = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();
        renderer.set_mrt(None);
        renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 1.0);
        renderer.auto_clear = true;

        let (width, height) = self.map.size();
        self.set_size(width, height);

        renderer.set_render_target(Some(self.horizontal.clone()));
        renderer.render_quad(&self.horizontal_quad);

        renderer.set_render_target(Some(self.vertical.clone()));
        renderer.render_quad(&self.vertical_quad);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
        true
    }
}

/// The `blur` `Fn` of `BilateralBlurNode.setup()`, unrolled by its JS `for`
/// loop: every tap has its own literal spatial weight.
fn blur(
    map: &Texture,
    direction: NodeRef,
    pass_direction: NodeRef,
    inv_size: NodeRef,
    sigma: u32,
    sigma_color: f64,
) -> NodeRef {
    let uv_node = uv();
    let kernel_size = sigma * 2 + 3;
    let spatial = spatial_coefficients(kernel_size);
    let direction = direction.mul(pass_direction);

    // Sample the centre pixel.
    let center_color = to_var(None, texture_uv(map, uv_node.clone()));
    let center_luminance = to_var(None, luminance(center_color.xyz()));

    // Accumulate weighted samples.
    let weight_sum = to_var(None, float(spatial[0]));
    let color_sum = to_var(None, center_color.mul(float(spatial[0])));

    // `-0.5 / sigmaColor²`.
    let color_sigma_factor = to_const(None, float(-0.5).div(float(sigma_color * sigma_color)));

    let mut statements = vec![
        center_color.clone(),
        center_luminance.clone(),
        weight_sum.clone(),
        color_sum.clone(),
        color_sigma_factor.clone(),
    ];
    for (i, &w) in spatial.iter().enumerate().skip(1) {
        let x = float(i as f64);
        let spatial_weight = float(w);
        let uv_offset = to_var(None, direction.clone().mul(inv_size.clone().mul(x)));

        // Sample in both directions.
        let sample1 = to_var(
            None,
            texture_uv(map, uv_node.clone().add(uv_offset.clone())),
        );
        let sample2 = to_var(None, texture_uv(map, uv_node.clone().sub(uv_offset)));

        // The luminance difference, for edge detection.
        let diff1 = abs(luminance(sample1.xyz()).sub(center_luminance.clone()));
        let diff2 = abs(luminance(sample2.xyz()).sub(center_luminance.clone()));

        // The colour weights: a Gaussian of the difference.
        let color_weight1 = to_var(
            None,
            exp(diff1.clone().mul(diff1).mul(color_sigma_factor.clone())),
        );
        let color_weight2 = to_var(
            None,
            exp(diff2.clone().mul(diff2).mul(color_sigma_factor.clone())),
        );

        // The bilateral weight: spatial times colour.
        let bilateral_weight1 = spatial_weight.clone().mul(color_weight1.clone());
        let bilateral_weight2 = spatial_weight.mul(color_weight2.clone());

        statements.push(color_weight1);
        statements.push(color_weight2);
        statements.push(color_sum.add_assign(sample1.mul(bilateral_weight1.clone())));
        statements.push(color_sum.add_assign(sample2.mul(bilateral_weight2.clone())));
        statements.push(weight_sum.add_assign(bilateral_weight1));
        statements.push(weight_sum.add_assign(bilateral_weight2));
    }

    // Normalise by the total weight.
    block(statements, color_sum.div(max(weight_sum, float(0.0001))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `webgpu_postprocessing_godrays`' `bilateralBlur( … )` — σ = 4, an
    /// 11-tap kernel whose weights `dumps/godrays/m11` bakes as literals.
    #[test]
    fn sigma_4_matches_the_dumped_coefficients() {
        let c = spatial_coefficients(4 * 2 + 3);
        assert_eq!(c.len(), 11);
        assert_eq!(c[0], 0.1088018181818182);
        assert_eq!(c[1], 0.10482978744722705);
        assert_eq!(c[10], 0.002639315969304918);
    }

    #[test]
    fn set_size_rounds_and_clamps_to_one() {
        assert_eq!(scaled_size(400, 1.0), 400);
        assert_eq!(scaled_size(0, 1.0), 1);
    }
}
