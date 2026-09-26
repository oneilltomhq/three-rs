//! Port of `three.js/examples/jsm/tsl/display/GaussianBlurNode.js` — a
//! separable Gaussian blur in two full-screen passes.
//!
//! `gaussianBlur( node, directionNode, sigma, options )` owns two render
//! targets. [`GaussianBlurNode::render`] (three's `updateBefore`) draws the
//! horizontal pass from the input into `horizontal`, then the vertical pass
//! from `horizontal` into `vertical`, and [`GaussianBlurNode::node`] samples
//! `vertical` for the graph downstream.
//!
//! The ownership divergences are `BloomNode`'s (see `bloom.rs` and
//! `docs/postprocessing.md`): the application calls `render` before it draws
//! whatever reads [`GaussianBlurNode::node`], and the two quad materials are
//! built in the constructor instead of in a setup pass.

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{
    block, float, premultiply_alpha, texture_uv, to_var, uniform_settable, unpremultiply_alpha, uv,
    vec2,
};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{Texture, TextureFilter, TextureType};

/// `gaussianBlur( node, directionNode, sigma, options )`' `options` object.
#[derive(Clone, Copy, Debug)]
pub struct GaussianBlurOptions {
    /// `options.premultipliedAlpha` — blur premultiplied colours and
    /// un-premultiply the sum, so transparent texels do not bleed their
    /// (meaningless) colour into their neighbours.
    pub premultiplied_alpha: bool,
    /// `options.resolutionScale` — the size of the two targets relative to
    /// the input texture.
    pub resolution_scale: f64,
}

impl Default for GaussianBlurOptions {
    /// `options.premultipliedAlpha || false`, `options.resolutionScale || 1`.
    fn default() -> Self {
        Self {
            premultiplied_alpha: false,
            resolution_scale: 1.0,
        }
    }
}

/// `new GaussianBlurNode( textureNode, directionNode, sigma, options )`.
pub struct GaussianBlurNode {
    /// `this.textureNode.value` — the texture blurred.
    map: Texture,
    /// `this._horizontalRT`.
    horizontal: RenderTarget,
    /// `this._verticalRT`.
    vertical: RenderTarget,
    /// `this._hMaterial`'s quad.
    horizontal_quad: QuadMesh,
    /// `this._vMaterial`'s quad.
    vertical_quad: QuadMesh,
    /// `this._invSize`, shared by both passes.
    inv_size: SettableValue,
    /// `this._textureNode` — `passTexture( this, this._verticalRT.texture )`.
    node: NodeRef,
    /// `this.sigma`.
    sigma: u32,
    /// `this.resolutionScale`.
    resolution_scale: f64,
}

/// `gaussianBlur( node, directionNode, sigma, options )`.
///
/// Three.js takes a node and runs it through `convertToTexture()`; the port
/// takes the texture, as [`radial_blur`](super::radial_blur) does, and the
/// caller converts first (`convert_to_texture( node ).texture()`) — the
/// `RttNode` needs its own `render` call, so the caller has to hold it anyway.
///
/// `direction` is `directionNode`: `None` is three's `null`, which becomes
/// `vec2( 1 )`. `sigma` is a whole number of texels, as in every three.js
/// call site: it sets the kernel size `3 + 2σ`, a JS loop bound.
pub fn gaussian_blur(
    map: &Texture,
    direction: Option<NodeRef>,
    sigma: u32,
    options: GaussianBlurOptions,
) -> GaussianBlurNode {
    GaussianBlurNode::new(map, direction, sigma, options)
}

/// `GaussianBlurNode._getCoefficients( kernelRadius )`: `exp( -0.5 i² / σ² )`
/// for `σ = kernelRadius / 3`, normalised so the centre tap plus both sides of
/// every other tap sum to one.
fn coefficients(kernel_radius: u32) -> Vec<f64> {
    let sigma = kernel_radius as f64 / 3.0;
    let mut coefficients = vec![1.0];
    let mut sum = 1.0;
    for i in 1..kernel_radius {
        let i = i as f64;
        let w = (-0.5 * i * i / (sigma * sigma)).exp();
        coefficients.push(w);
        sum += 2.0 * w;
    }
    coefficients.into_iter().map(|c| c / sum).collect()
}

/// `GaussianBlurNode.setSize()`: `Math.max( Math.round( size * scale ), 1 )`.
/// Rounding, not the floor `BloomNode` and `RTTNode` use.
fn scaled_size(size: u32, resolution_scale: f64) -> u32 {
    ((size as f64 * resolution_scale).round() as u32).max(1)
}

/// The texture type a target has to take to match `map` — three's
/// `this._horizontalRT.texture.type = map.type`.
fn texture_type_of(map: &Texture) -> TextureType {
    match map.format() {
        wgpu::TextureFormat::Rgba16Float => TextureType::HalfFloat,
        _ => TextureType::UnsignedByte,
    }
}

impl GaussianBlurNode {
    pub fn new(
        map: &Texture,
        direction: Option<NodeRef>,
        sigma: u32,
        options: GaussianBlurOptions,
    ) -> Self {
        // `new RenderTarget( 1, 1, { depthBuffer: false } )`, whose texture
        // type `updateBefore()` then sets to the input's every frame. The
        // port's target fixes its colour format at construction, so the type
        // is taken from the input here; nothing in three.js changes a blurred
        // texture's type after the first frame.
        let target = || {
            RenderTarget::new_with_options(
                1,
                1,
                RenderTargetOptions {
                    texture_type: texture_type_of(map),
                    samples: 0,
                    depth_buffer: false,
                    min_filter: TextureFilter::Linear,
                    mag_filter: TextureFilter::Linear,
                },
            )
            .expect("three-rs: a gaussian blur target is a colour type")
        };
        let horizontal = target();
        let vertical = target();

        let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![0.0, 0.0]);
        // `vec2( this.directionNode || 1 )`.
        let direction = match direction {
            Some(direction) => direction.to(Type::Vec2),
            None => vec2(1.0, 1.0),
        };

        let blur = |map: &Texture, pass_direction: NodeRef, name: &'static str| {
            let mut material = MeshBasicNodeMaterial::new();
            material.name = name;
            material.fragment_node = Some(Self::blur(
                map,
                direction.clone(),
                pass_direction,
                inv_size_node.clone(),
                sigma,
                options.premultiplied_alpha,
            ));
            QuadMesh::new(material)
        };
        let horizontal_quad = blur(map, vec2(1.0, 0.0), "Gaussian_blur_horizontal");
        let vertical_quad = blur(
            &horizontal.texture(),
            vec2(0.0, 1.0),
            "Gaussian_blur_vertical",
        );

        // `passTexture( this, this._verticalRT.texture )`, with the input's
        // `uvNode` — `uv()` for every texture the port hands in — wrapped in
        // the var three's `TempNode` promotion gives a node read more than
        // once.
        let node = to_var(None, texture_uv(&vertical.texture(), uv()));

        Self {
            map: map.clone(),
            horizontal,
            vertical,
            horizontal_quad,
            vertical_quad,
            inv_size,
            node,
            sigma,
            resolution_scale: options.resolution_scale,
        }
    }

    /// The `blur` `Fn` of `GaussianBlurNode.setup()`, inlined: the kernel is
    /// unrolled by a JS `for` loop, so every tap is its own statement with
    /// its own literal weight.
    fn blur(
        map: &Texture,
        direction: NodeRef,
        pass_direction: NodeRef,
        inv_size: NodeRef,
        sigma: u32,
        premultiplied_alpha: bool,
    ) -> NodeRef {
        let uv_node = uv();
        let sample = |coord: NodeRef| {
            let texel = texture_uv(map, coord);
            if premultiplied_alpha {
                premultiply_alpha(texel)
            } else {
                texel
            }
        };

        let kernel_size = 3 + 2 * sigma;
        let coefficients = coefficients(kernel_size);

        // `const direction = directionNode.mul( passDirection )`, read by
        // every tap.
        let direction = direction.mul(pass_direction);

        let diffuse_sum = to_var(None, sample(uv_node.clone()).mul(float(coefficients[0])));
        let mut statements = vec![diffuse_sum.clone()];
        for (i, &w) in coefficients.iter().enumerate().skip(1) {
            let x = float(i as f64);
            // `vec2( direction.mul( invSize.mul( x ) ) ).toVar()` — the
            // `vec2()` is a no-op conversion of what already is one.
            let uv_offset = to_var(None, direction.clone().mul(inv_size.clone().mul(x)));
            let sample1 = sample(uv_node.clone().add(uv_offset.clone()));
            let sample2 = sample(uv_node.clone().sub(uv_offset));
            statements.push(diffuse_sum.add_assign(sample1.add(sample2).mul(float(w))));
        }

        let result = if premultiplied_alpha {
            unpremultiply_alpha(diffuse_sum)
        } else {
            diffuse_sum
        };
        block(statements, result)
    }

    /// `gaussianBlurNode.getTextureNode()` — the blurred texture, for the
    /// graph downstream.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `this._verticalRT.texture`.
    pub fn texture(&self) -> Texture {
        self.vertical.texture()
    }

    /// `this.sigma`.
    pub fn sigma(&self) -> u32 {
        self.sigma
    }

    /// The two quad materials, horizontal first — for `examples/dump_wgsl.rs`.
    pub fn quad_materials(&self) -> Vec<&MeshBasicNodeMaterial> {
        vec![&self.horizontal_quad.material, &self.vertical_quad.material]
    }

    /// `GaussianBlurNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        let width = scaled_size(width, self.resolution_scale);
        let height = scaled_size(height, self.resolution_scale);
        self.inv_size
            .set(vec![1.0 / width as f64, 1.0 / height as f64]);
        self.horizontal.set_size(width, height);
        self.vertical.set_size(width, height);
    }

    /// `GaussianBlurNode.updateBefore( frame )`: size the targets from the
    /// input (`map.image.width / height`), then the two passes, with the
    /// renderer's state reset around them.
    pub fn render(&self, renderer: &mut Renderer) {
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

        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `webgpu_procedural_texture`'s `gaussianBlur( …, 20 )`: a 43-tap
    /// kernel, whose weights `dump/m02` bakes as literals.
    #[test]
    fn sigma_20_matches_the_dumped_coefficients() {
        let c = coefficients(3 + 2 * 20);
        assert_eq!(c.len(), 43);
        assert_eq!(c[0], 0.027917486897724192);
        assert_eq!(c[1], 0.027849625383014897);
        assert_eq!(c[2], 0.027647028978021935);
        assert_eq!(c[42], 0.0003814108419989991);
    }

    #[test]
    fn set_size_rounds_and_clamps_to_one() {
        assert_eq!(scaled_size(512, 1.0), 512);
        assert_eq!(scaled_size(5, 0.5), 3);
        assert_eq!(scaled_size(1, 0.25), 1);
    }
}
