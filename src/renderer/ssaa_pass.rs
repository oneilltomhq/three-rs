//! Port of `three.js/examples/jsm/tsl/display/SSAAPassNode.js`.
//!
//! `ssaaPass( scene, camera )` is a [`PassNode`] subclass: it owns the same
//! accumulator render target, plus a clone of it for one sample, and renders
//! the scene once per jitter offset with `camera.setViewOffset()` moved by a
//! fraction of a pixel. Each sample is added into the accumulator by a
//! full-screen quad with additive blending and a per-sample weight, and the
//! accumulator's texture is what the graph downstream reads.
//!
//! Rust has no subclassing, so the port composes: an [`SsaaPassNode`] *has* a
//! `PassNode` and forwards [`node`](SsaaPassNode::node) /
//! [`texture`](SsaaPassNode::texture) to it. What `PassNode` contributes —
//! the `rgba16float` target, its own `depth`, and the
//! `to_var( texture_uv( … , uv() ) )` pair of vars a `PassTextureNode` emits —
//! is unchanged.
//!
//! The ownership divergence is the one `docs/postprocessing.md` already
//! records for `PassNode`: three.js fires `updateBefore()` from inside the
//! `RenderPipeline` quad's own render, the port has the application call
//! [`SsaaPassNode::render`] immediately before it.

use crate::cameras::PerspectiveCamera;
use crate::materials::{Blending, MeshBasicNodeMaterial};
use crate::math::Color;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::tsl::{texture, uniform_settable, unpremultiply_alpha};
use crate::nodes::NodeRef;
use crate::objects::{QuadMesh, Scene};

use super::pass::PassNode;
use super::render_target::RenderTarget;
use super::Renderer;

/// `_JitterVectors` — sample patterns on an assumed `[-8, 8)` integer grid,
/// scaled by 1/16 before they reach `setViewOffset`. Level `n` has `2^n`
/// samples; only level 3 is exercised by `webgpu_postprocessing_ssaa`, and all
/// six are here because `sampleLevel` is a public knob.
const JITTER_VECTORS: [&[[f64; 2]]; 6] = [
    &[[0.0, 0.0]],
    &[[4.0, 4.0], [-4.0, -4.0]],
    &[[-2.0, -6.0], [6.0, -2.0], [-6.0, 2.0], [2.0, 6.0]],
    &[
        [1.0, -3.0],
        [-1.0, 3.0],
        [5.0, 1.0],
        [-3.0, -5.0],
        [-5.0, 5.0],
        [-7.0, -1.0],
        [3.0, 7.0],
        [7.0, -7.0],
    ],
    &[
        [1.0, 1.0],
        [-1.0, -3.0],
        [-3.0, 2.0],
        [4.0, -1.0],
        [-5.0, -2.0],
        [2.0, 5.0],
        [5.0, 3.0],
        [3.0, -5.0],
        [-2.0, 6.0],
        [0.0, -7.0],
        [-4.0, -6.0],
        [-6.0, 4.0],
        [-8.0, 0.0],
        [7.0, -4.0],
        [6.0, 7.0],
        [-7.0, -8.0],
    ],
    &[
        [-4.0, -7.0],
        [-7.0, -5.0],
        [-3.0, -5.0],
        [-5.0, -4.0],
        [-1.0, -4.0],
        [-2.0, -2.0],
        [-6.0, -1.0],
        [-4.0, 0.0],
        [-7.0, 1.0],
        [-1.0, 2.0],
        [-6.0, 3.0],
        [-3.0, 3.0],
        [-7.0, 6.0],
        [-3.0, 6.0],
        [-5.0, 7.0],
        [-1.0, 7.0],
        [5.0, -7.0],
        [1.0, -6.0],
        [6.0, -5.0],
        [4.0, -4.0],
        [2.0, -3.0],
        [7.0, -2.0],
        [1.0, -1.0],
        [4.0, -1.0],
        [2.0, 1.0],
        [6.0, 2.0],
        [0.0, 4.0],
        [4.0, 4.0],
        [2.0, 5.0],
        [7.0, 5.0],
        [5.0, 6.0],
        [3.0, 7.0],
    ],
];

/// `1 / 16` — the comment in `SSAAPassNode.js` beside the literal.
const JITTER_SCALE: f64 = 0.0625;

/// `const roundingRange = 1 / 32`.
const ROUNDING_RANGE: f64 = 1.0 / 32.0;

/// `_JitterVectors[ i ]`, as the eight-entry table the jitter test reads.
pub(crate) fn jitter_vectors(sample_level: usize) -> &'static [[f64; 2]] {
    JITTER_VECTORS[sample_level.min(5)]
}

/// `sampleWeight.value` for sample `i` of `count`: `1 / count`, plus, when
/// `unbiased`, a per-sample offset uniformly spread over `1 / 32` so the
/// rounding errors of the accumulation cancel rather than pile up.
pub(crate) fn sample_weight(i: usize, count: usize, unbiased: bool) -> f64 {
    let base = 1.0 / count as f64;
    if unbiased {
        base + ROUNDING_RANGE * (-0.5 + (i as f64 + 0.5) / count as f64)
    } else {
        base
    }
}

/// `ssaaPass( scene, camera )`.
pub struct SsaaPassNode {
    pass: PassNode,
    /// `this._sampleRenderTarget = this.renderTarget.clone()` — one sample's
    /// scene render, before it is weighted and added in.
    sample_render_target: RenderTarget,
    /// `this._quadMesh`, whose material is built once in `setup()`.
    quad: QuadMesh,
    /// `this.sampleWeight` — one `f32` object uniform, written eight times a
    /// frame.
    sample_weight: SettableValue,
    /// `this.sampleLevel`: `2^n` samples. three.js' default is 4;
    /// `webgpu_postprocessing_ssaa` sets 3.
    pub sample_level: usize,
    /// `this.unbiased`.
    pub unbiased: bool,
}

impl Default for SsaaPassNode {
    fn default() -> Self {
        Self::new()
    }
}

impl SsaaPassNode {
    pub fn new() -> Self {
        // `super( PassNode.COLOR, scene, camera, { samples: 0 } )`: the
        // accumulator is the ordinary pass target, except that `{ samples: 0 }`
        // pins it single-sample instead of taking `renderer.samples` — so an
        // `antialias: true` renderer does not also MSAA every sample.
        let pass = PassNode::new();
        pass.render_target().set_samples(0);
        let sample_render_target = pass.render_target().clone_target();

        // `this.sampleWeight = uniform( 1 )`.
        let (sample_weight_node, sample_weight) = uniform_settable(Type::F32, vec![1.0]);

        // `setup()`'s quad material. `texture( this._sampleRenderTarget.texture )`
        // — not `passTexture` — so `setUpdateMatrix` stays on and the fragment
        // samples through a `mat3x3<f32>` texture matrix in the object uniform
        // block. That asymmetry is the whole of `docs/postprocessing.md`'s
        // `PassTextureNode` note, seen from the other side.
        let mut material = MeshBasicNodeMaterial::new();
        material.name = "SSAA";
        material.fragment_node = Some(unpremultiply_alpha(
            texture(&sample_render_target.texture()).mul(sample_weight_node),
        ));
        material.transparent = true;
        material.depth_test = false;
        material.depth_write = false;
        // `premultipliedAlpha: true` does two things: it picks the
        // `one / one / add` half of the blend table (`AdditiveBlending` with
        // premultiplied alpha is `set_blend( One, One, One, One )`), and it
        // wraps the fragment node in `premultiplyAlpha` — the dump's
        // `output.color = fn0( fn1( … ) )`.
        material.premultiplied_alpha = true;
        material.blending = Blending::Additive;

        Self {
            pass,
            sample_render_target,
            quad: QuadMesh::new(material),
            sample_weight,
            sample_level: 4,
            unbiased: true,
        }
    }

    /// `ssaaPass.getTextureNode()` — the accumulator, for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.pass.node()
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> crate::textures::Texture {
        self.pass.texture()
    }

    /// `SSAAPassNode.updateBefore( frame )`.
    ///
    /// Twenty-six GPU passes for `sampleLevel = 3`, in this order: for each of
    /// the eight jitter offsets, a clear of the sample target, the scene
    /// rendered into it, and the weighted accumulation quad — with the
    /// accumulator's own clear wedged **inside** iteration 0, between the
    /// first scene render and the first quad.
    ///
    /// `copyTextureToTexture( sampleRT.depthTexture, renderTarget.depthTexture )`
    /// is not ported; see `docs/postprocessing.md`.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut PerspectiveCamera,
    ) {
        let (width, height) = renderer.drawing_buffer_size();
        self.pass.render_target().set_size(width, height);
        self.sample_render_target.set_size(width, height);

        let previous_target = renderer.render_target();
        let previous_auto_clear = renderer.auto_clear;
        let previous_clear_color = renderer.clear_color();
        let previous_clear_alpha = renderer.clear_alpha();

        // `renderer.autoClear = false`: every pass below loads, and the only
        // things that clear are the explicit `clear()` calls.
        renderer.auto_clear = false;

        let jitter = jitter_vectors(self.sample_level);
        let count = jitter.len();

        // `const viewOffset = { … }` seeded from the target, then overwritten
        // by the camera's own view offset when it has one enabled. The page
        // calls `camera.setViewOffset( 800, 500, 0, 0, 800, 500 )` in
        // `init()`, so the base offset here is that enabled, zero one — and
        // the restore at the end is the `setViewOffset` branch, never
        // `clearViewOffset`.
        let original = camera.view.filter(|view| view.enabled);
        let (full_width, full_height, offset_x, offset_y, view_width, view_height) = match original
        {
            Some(view) => (
                view.full_width,
                view.full_height,
                view.offset_x,
                view.offset_y,
                view.width,
                view.height,
            ),
            None => (
                width as f64,
                height as f64,
                0.0,
                0.0,
                width as f64,
                height as f64,
            ),
        };

        for (i, offset) in jitter.iter().enumerate() {
            camera.set_view_offset(
                full_width,
                full_height,
                offset_x + offset[0] * JITTER_SCALE,
                offset_y + offset[1] * JITTER_SCALE,
                view_width,
                view_height,
            );

            self.sample_weight
                .set(vec![sample_weight(i, count, self.unbiased)]);

            renderer.set_render_target(Some(self.sample_render_target.clone()));
            renderer.clear(true, true);
            renderer.render(scene, camera);

            // accumulation

            renderer.set_render_target(Some(self.pass.render_target().clone()));

            if i == 0 {
                // `setClearColor( 0x000000, 0.0 ); clear(); setClearColor( back )`.
                // The alpha matters: the quad writes `( colour * w, w )` and
                // the weights sum to one, so the accumulator ends the frame at
                // alpha 1 — which is what `renderOutput`'s unpremultiply
                // divides by. Cleared to alpha 1 instead, every texel would
                // come out of the final quad at half brightness.
                renderer.set_clear_color(Color::new(0.0, 0.0, 0.0), 0.0);
                renderer.clear(true, true);
                renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
            }

            renderer.render_quad(&self.quad);
        }

        // restore
        camera.set_view_offset(
            full_width,
            full_height,
            offset_x,
            offset_y,
            view_width,
            view_height,
        );
        if original.is_none() {
            camera.clear_view_offset();
        }

        renderer.set_render_target(previous_target);
        renderer.auto_clear = previous_auto_clear;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eight weights of `sampleLevel = 3`, unbiased — the only
    /// floating-point content of the effect, and a transcription error in
    /// `uniformCenteredDistribution` would show up as a uniform brightness
    /// shift a JPEG comparison might still pass.
    #[test]
    fn the_eight_unbiased_sample_weights() {
        let weights: Vec<f64> = (0..8).map(|i| sample_weight(i, 8, true)).collect();
        assert_eq!(
            weights,
            // `0.125 + ( 1 / 32 ) * ( -0.5 + ( i + 0.5 ) / 8 )`: eight steps
            // of `1 / 256` centred on `1 / 8`. (The scout plan's table has
            // steps of `3 / 512` and sums to 1.0547; it is the formula that
            // is the oracle, and these weights sum to exactly one.)
            vec![
                0.111328125,
                0.115234375,
                0.119140625,
                0.123046875,
                0.126953125,
                0.130859375,
                0.134765625,
                0.138671875,
            ]
        );
        // They sum to exactly one, which is why the accumulator can be read
        // back as an ordinary premultiplied colour.
        assert_eq!(weights.iter().sum::<f64>(), 1.0);
    }

    /// Biased, every sample weighs the same.
    #[test]
    fn the_biased_weights_are_flat() {
        for i in 0..8 {
            assert_eq!(sample_weight(i, 8, false), 0.125);
        }
    }

    /// `_JitterVectors[ Math.max( 0, Math.min( this.sampleLevel, 5 ) ) ]`:
    /// level `n` has `2^n` samples, and level 3 is the page's.
    #[test]
    fn the_jitter_tables_are_powers_of_two() {
        for level in 0..=5 {
            assert_eq!(jitter_vectors(level).len(), 1 << level);
        }
        assert_eq!(jitter_vectors(9).len(), 32);
        assert_eq!(
            jitter_vectors(3),
            &[
                [1.0, -3.0],
                [-1.0, 3.0],
                [5.0, 1.0],
                [-3.0, -5.0],
                [-5.0, 5.0],
                [-7.0, -1.0],
                [3.0, 7.0],
                [7.0, -7.0],
            ]
        );
    }
}
