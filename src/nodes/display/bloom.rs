//! Port of `three.js/examples/jsm/tsl/display/BloomNode.js` — the
//! UnrealBloomPass mip chain as a TSL effect node.
//!
//! `bloom( input )` owns eleven render targets and draws twelve full-screen
//! quads a frame: a luminosity high pass into `bright`, then five mip levels
//! blurred horizontally and vertically in turn, then one composite that adds
//! the five blurred mips back together into `h0`, which is the texture the
//! graph downstream samples.
//!
//! Three ownership divergences, all of them the same one `PassNode` and
//! `SsaaPassNode` already record in `docs/postprocessing.md`:
//!
//! * three.js fires `updateBefore()` from inside the `RenderPipeline` quad's
//!   render; the port has the application call [`BloomNode::render`] first.
//! * `BloomNode.setup()` builds the quad materials lazily, inside the outer
//!   build, so that `context( builder.getSharedContext() )` keeps their
//!   uniforms out of the outer graph's numbering. The port builds each quad
//!   material's fragment in its own `NodeBuilder` pass anyway, so the
//!   materials are built in [`BloomNode::new`] and there is no shared context.
//! * three.js swaps `separableBlurMaterial.colorTexture.value` between the
//!   horizontal and the vertical pass of a mip, so one material — and one
//!   pipeline — serves both. A `Texture` is an identity here and a material's
//!   graph names it, so the port has **two** blur materials per mip, one per
//!   direction. Their WGSL is the same text (that is what `dump_wgsl`'s five
//!   `bloom_separable_N` sections diff against `dump/m05..m09`); what differs
//!   is that the port builds ten programs where three.js builds five.

use std::rc::Rc;

use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::nodes::node::{FnDef, Lazy, SettableValue, Type};
use crate::nodes::tsl::{
    array_var, call, float, int, loop_n, luminance, mix, shader_fn, smoothstep, texture,
    texture_sample, texture_uv, to_var, uniform_array_vec3, uniform_settable, uniform_value, uv,
    vec4, vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;
use crate::renderer::{RenderTarget, RenderTargetOptions, Renderer};
use crate::textures::{TextureFilter, TextureType};

/// `this._nMips = 5`.
const N_MIPS: usize = 5;

/// `const kernelSizeArray = [ 6, 10, 14, 18, 22 ]` — the per-mip Gaussian
/// radii, widened in three.js PR #31528 to keep the blur strength while the
/// coefficients moved to bilinear taps.
const KERNEL_SIZES: [usize; N_MIPS] = [6, 10, 14, 18, 22];

/// The arguments `BloomNode.highPassFn` is called with.
pub struct HighPassInput {
    /// `this.inputNode` — what the effect is bloomed from.
    pub input: NodeRef,
    /// `this.threshold`.
    pub threshold: NodeRef,
    /// `this.smoothWidth`.
    pub smooth_width: NodeRef,
}

/// `luminosityHighPass` — `BloomNode`'s default high pass, and the only one
/// `webgpu_postprocessing_bloom_selective` uses.
///
/// `luminance( input.rgb )` is the **vec3** branch: the dot is against
/// `vec3( 0.2126, 0.7152, 0.0722 )` with no alpha coefficient, which is what
/// `dump/m03` has. A caller that hands `luminance()` a `vec4` gets three's
/// other branch, which the port does not have yet.
pub fn luminosity_high_pass(args: HighPassInput) -> NodeRef {
    let HighPassInput {
        input,
        threshold,
        smooth_width,
    } = args;
    let v = luminance(input.rgb());
    let alpha = smoothstep(threshold.clone(), threshold.add(smooth_width), v);
    mix(vec4(0.0, 0.0, 0.0, 0.0), input, alpha)
}

/// `lerpBloomFactor( factor, radius )` — `mix( factor, 1.2 - factor, radius )`,
/// with a layout, so it is a real `fn` in the generated WGSL (`dump/m11`'s
/// `fn4`) rather than an inlined expression.
fn lerp_bloom_factor(factor: NodeRef, radius: NodeRef) -> NodeRef {
    // One `FnDef` for all five calls, as three.js' one `Fn()` object is: a
    // `FnDef` is the function's identity in the builder, so five of them would
    // be five identical `fn`s in the module.
    thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
    let def = CELL.with(|cell| {
        cell.get(|| {
            shader_fn(
                None,
                vec![("factor", Type::F32), ("radius", Type::F32)],
                Type::F32,
                |args| {
                    let (factor, radius) = (args[0].clone(), args[1].clone());
                    let mirror_factor = float(1.2).sub(factor.clone());
                    mix(factor, mirror_factor, radius)
                },
            )
        })
    });
    call(&def, vec![factor, radius])
}

/// `_getSeparableBlurMaterial`'s CPU-side Gaussian: `kernelRadius`
/// coefficients `0.39894 * exp( -0.5 i² / σ² ) / σ` with `σ = kernelRadius / 3`,
/// merged pairwise into bilinear taps.
///
/// Returns `( centreWeight, offsets, weights )`. `kernelRadius` taps become
/// `kernelRadius / 2` bilinear ones — `[ 6, 10, 14, 18, 22 ]` → `[ 3, 5, 7, 9,
/// 11 ]`, which is the literal loop bound in each `Bloom_separable` module.
fn separable_blur_coefficients(kernel_radius: usize) -> (f64, Vec<f64>, Vec<f64>) {
    let sigma = kernel_radius as f64 / 3.0;
    let coefficients: Vec<f64> = (0..kernel_radius)
        .map(|i| {
            let i = i as f64;
            0.39894 * (-0.5 * i * i / (sigma * sigma)).exp() / sigma
        })
        .collect();

    let centre_weight = coefficients[0];
    let mut offsets = Vec::new();
    let mut weights = Vec::new();
    let mut i = 1;
    while i < kernel_radius {
        let wa = coefficients[i];
        let wb = if i + 1 < kernel_radius {
            coefficients[i + 1]
        } else {
            0.0
        };
        let w = wa + wb;
        offsets.push((i as f64 * wa + (i + 1) as f64 * wb) / w);
        weights.push(w);
        i += 2;
    }

    (centre_weight, offsets, weights)
}

/// `BloomNode.setSize()`'s mip chain: `Math.floor( size * resolutionScale )`,
/// then floor-halved once per mip.
///
/// **Floor, not rounding.** 800×500 at the default half scale is 400×250,
/// 200×125, 100×**62**, 50×**31**, 25×15 — `Math.floor( 125 / 2 )` is 62 and
/// rounding would give 63. The scout plan reads `Math.round( w / 2 )` off the
/// dumped sizes; the source is `Math.floor`, and the two disagree at exactly
/// the sizes this example uses.
fn mip_sizes(width: u32, height: u32, resolution_scale: f64) -> [(u32, u32); N_MIPS] {
    let mut resx = (width as f64 * resolution_scale).floor() as u32;
    let mut resy = (height as f64 * resolution_scale).floor() as u32;
    let mut sizes = [(0, 0); N_MIPS];
    for size in sizes.iter_mut() {
        *size = (resx, resy);
        resx /= 2;
        resy /= 2;
    }
    sizes
}

/// One mip level's pair of blur passes.
struct BlurMip {
    /// The horizontal quad: samples the level's input (`bright`, or the
    /// previous level's vertical target) and writes `h`.
    horizontal: QuadMesh,
    /// The vertical quad: samples `h` and writes `v`.
    vertical: QuadMesh,
    /// `separableBlurMaterial.invSize`, shared by both directions as it is in
    /// three.js — one uniform per mip, rewritten by [`BloomNode::set_size`].
    inv_size: SettableValue,
}

/// `bloom( inputNode, strength, radius, threshold )`.
pub struct BloomNode {
    /// `this._renderTargetBright`.
    bright: RenderTarget,
    /// `this._renderTargetsHorizontal` / `._renderTargetsVertical`.
    horizontal: [RenderTarget; N_MIPS],
    vertical: [RenderTarget; N_MIPS],
    /// `this._highPassFilterMaterial`'s quad.
    high_pass: QuadMesh,
    blur: Vec<BlurMip>,
    /// `this._compositeMaterial`'s quad.
    composite: QuadMesh,
    /// `this._textureOutput = passTexture( this, _renderTargetsHorizontal[ 0 ]
    /// .texture )`, wrapped in the var three's `TempNode` promotion gives it
    /// (`dump/m12`'s `nodeVar2 = nodeVar1`).
    node: NodeRef,
    /// `this.strength` / `.radius` / `.threshold` — the three knobs the page's
    /// GUI moves. `this.smoothWidth` is `uniform( 0.01 )` and nothing writes
    /// it, so it stays a plain value uniform.
    pub strength: SettableValue,
    pub radius: SettableValue,
    pub threshold: SettableValue,
    /// `this._resolutionScale`.
    resolution_scale: f64,
}

/// `bloom( node )` — `strength` 1, `radius` 0, `threshold` 0, the defaults
/// `webgpu_postprocessing_bloom_selective` takes.
pub fn bloom(input: NodeRef) -> BloomNode {
    BloomNode::new(input)
}

impl BloomNode {
    /// `new BloomNode( input )` with [`luminosity_high_pass`].
    pub fn new(input: NodeRef) -> Self {
        Self::with_high_pass(input, luminosity_high_pass)
    }

    /// `bloomNode.highPassFn = …`.
    ///
    /// The high pass is a constructor argument rather than a field because the
    /// port has no setup stage to rebuild the materials in: `webgpu_postprocessing_bloom`
    /// and `_anamorphic` both reuse `BloomNode` with their own high pass, and
    /// this is where they hand it over.
    pub fn with_high_pass(
        input: NodeRef,
        high_pass: impl FnOnce(HighPassInput) -> NodeRef,
    ) -> Self {
        let bright = Self::target("UnrealBloomPass.bright");
        let horizontal = std::array::from_fn(|_| Self::target("UnrealBloomPass.h"));
        let vertical: [RenderTarget; N_MIPS] =
            std::array::from_fn(|_| Self::target("UnrealBloomPass.v"));

        let (strength_node, strength) = uniform_settable(Type::F32, vec![1.0]);
        let (radius_node, radius) = uniform_settable(Type::F32, vec![0.0]);
        let (threshold_node, threshold) = uniform_settable(Type::F32, vec![0.0]);
        // `this.smoothWidth = uniform( 0.01 )`.
        let smooth_width = uniform_value(Type::F32, vec![0.01]);

        // 1. the luminosity high pass.
        let mut high_pass_material = MeshBasicNodeMaterial::new();
        high_pass_material.name = "Bloom_highPass";
        high_pass_material.fragment_node = Some(high_pass(HighPassInput {
            input,
            threshold: threshold_node,
            smooth_width,
        }));

        // 2. the five separable blurs, each a pair of quads.
        let mut blur = Vec::with_capacity(N_MIPS);
        for (i, &kernel_radius) in KERNEL_SIZES.iter().enumerate() {
            let (inv_size_node, inv_size) = uniform_settable(Type::Vec2, vec![1.0, 1.0]);
            let input_target = if i == 0 { &bright } else { &vertical[i - 1] };
            blur.push(BlurMip {
                horizontal: QuadMesh::new(Self::separable_blur_material(
                    kernel_radius,
                    &input_target.texture(),
                    inv_size_node.clone(),
                    [1.0, 0.0],
                )),
                vertical: QuadMesh::new(Self::separable_blur_material(
                    kernel_radius,
                    &horizontal[i].texture(),
                    inv_size_node,
                    [0.0, 1.0],
                )),
                inv_size,
            });
        }

        // 3. the composite.
        let mut composite_material = MeshBasicNodeMaterial::new();
        composite_material.name = "Bloom_comp";
        composite_material.fragment_node =
            Some(Self::composite_node(&vertical, radius_node, strength_node));

        let node = to_var(None, texture_uv(&horizontal[0].texture(), uv()));

        Self {
            bright,
            horizontal,
            vertical,
            high_pass: QuadMesh::new(high_pass_material),
            blur,
            composite: QuadMesh::new(composite_material),
            node,
            strength,
            radius,
            threshold,
            resolution_scale: 0.5,
        }
    }

    /// `new RenderTarget( 1, 1, { depthBuffer: false, type: HalfFloatType } )`
    /// with `generateMipmaps = false`, resized on the first frame.
    fn target(_name: &str) -> RenderTarget {
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
        .expect("three-rs: a bloom render target is a HalfFloat colour type")
    }

    /// `_getSeparableBlurMaterial( sharedContext, kernelRadius )`.
    fn separable_blur_material(
        kernel_radius: usize,
        color_texture: &crate::textures::Texture,
        inv_size: NodeRef,
        direction: [f64; 2],
    ) -> MeshBasicNodeMaterial {
        let (centre_weight, offsets, weights) = separable_blur_coefficients(kernel_radius);
        let gaussian_offsets = crate::nodes::tsl::const_array(offsets.clone());
        let gaussian_weights = crate::nodes::tsl::const_array(weights);
        let direction = uniform_value(Type::Vec2, direction.to_vec());

        let uv_node = uv();
        // `const diffuseSum = sampleTexel( uvNode ).rgb.mul( centerWeight
        // ).toVar()`.
        let diffuse_sum = to_var(
            None,
            texture_sample(color_texture, uv_node.clone())
                .rgb()
                .mul(float(centre_weight)),
        );

        let body = {
            let (diffuse_sum, uv_node, direction, inv_size) =
                (diffuse_sum.clone(), uv_node.clone(), direction, inv_size);
            let color_texture = color_texture.clone();
            move |i: &NodeRef| {
                let w = gaussian_weights.element_node(i.clone());
                let uv_offset = direction
                    .mul(inv_size)
                    .mul(gaussian_offsets.element_node(i.clone()));
                let sample1 =
                    texture_sample(&color_texture, uv_node.clone().add(uv_offset.clone())).rgb();
                let sample2 = texture_sample(&color_texture, uv_node.sub(uv_offset)).rgb();
                // The loop body is one statement: three.js' `w`, `uvOffset`,
                // `sample1` and `sample2` are plain `const`s, not `toVar()`s,
                // so they are pulled into the flow by the `addAssign` that
                // reads them — which is what puts `uvOffset`'s var, and the
                // uniforms it reaches, ahead of the first sample's.
                vec![diffuse_sum.add_assign(sample1.add(sample2).mul(w))]
            }
        };

        let mut material = MeshBasicNodeMaterial::new();
        material.name = "Bloom_separable";
        material.fragment_node = Some(crate::nodes::tsl::block(
            vec![
                diffuse_sum.clone(),
                loop_n("i", int(offsets.len() as i64), body),
            ],
            vec4_join(vec![diffuse_sum, float(1.0)]),
        ));
        material
    }

    /// `compositePass` — the five blurred mips, each scaled by its bloom
    /// factor and tint, summed and multiplied by `strength`.
    fn composite_node(
        vertical: &[RenderTarget; N_MIPS],
        radius: NodeRef,
        strength: NodeRef,
    ) -> NodeRef {
        // `const bloomFactors = array( [ 1.0, 0.8, 0.6, 0.4, 0.2 ] )` — read
        // five times, so it is a var.
        let bloom_factors = array_var(vec![1.0, 0.8, 0.6, 0.4, 0.2]);
        // `this.bloomTintColors` — five white `Vector3`s, which the page never
        // changes. A `uniformArray`, not five uniforms: one `array< vec4<f32>,
        // 5 >` uniform block (`dump/m11`'s `NodeBuffer_1297`).
        let tints = uniform_array_vec3(&[[1.0, 1.0, 1.0]; N_MIPS]);

        let mut sum: Option<NodeRef> = None;
        for (i, mip) in vertical.iter().enumerate() {
            let color = lerp_bloom_factor(bloom_factors.element(i), radius.clone())
                .mul(vec4_join(vec![tints.element(i), float(1.0)]))
                .mul(texture(&mip.texture()));
            sum = Some(match sum {
                Some(previous) => previous.add(color),
                None => color,
            });
        }

        sum.expect("three-rs: the bloom composite sums five mips")
            .mul(strength)
    }

    /// `bloomNode.getTextureNode()` — `h0`, for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// The twelve quad materials, in the order [`render`](Self::render) draws
    /// them: the high pass, then each mip's horizontal and vertical blur, then
    /// the composite. Only `examples/dump_wgsl.rs` wants them — it is the one
    /// caller that has to see a material three.js keeps private, because the
    /// generated WGSL of all thirteen modules is what the rung is graded on
    /// before a single pixel is compared.
    pub fn quad_materials(&self) -> Vec<&MeshBasicNodeMaterial> {
        let mut materials = vec![&self.high_pass.material];
        for mip in &self.blur {
            materials.push(&mip.horizontal.material);
            materials.push(&mip.vertical.material);
        }
        materials.push(&self.composite.material);
        materials
    }

    /// `bloomNode.setResolutionScale( scale )`.
    pub fn set_resolution_scale(&mut self, resolution_scale: f64) {
        self.resolution_scale = resolution_scale;
    }

    /// `bloomNode.getResolutionScale()`.
    pub fn resolution_scale(&self) -> f64 {
        self.resolution_scale
    }

    /// `BloomNode.setSize( width, height )`.
    pub fn set_size(&self, width: u32, height: u32) {
        let sizes = mip_sizes(width, height, self.resolution_scale);
        let (resx, resy) = sizes[0];
        self.bright.set_size(resx, resy);

        for (i, &(resx, resy)) in sizes.iter().enumerate() {
            self.horizontal[i].set_size(resx, resy);
            self.vertical[i].set_size(resx, resy);
            self.blur[i]
                .inv_size
                .set(vec![1.0 / resx as f64, 1.0 / resy as f64]);
        }
    }

    /// `BloomNode.updateBefore( frame )` — the twelve quad passes, in the
    /// order the dump's passes 2..13 have them.
    ///
    /// `resetRendererState()` is the three lines at the top: no MRT, an opaque
    /// black clear colour and `autoClear` on, so every one of the twelve
    /// passes clears its target before the quad covers it. All twelve are
    /// single-attachment and none has a depth attachment, because every bloom
    /// target is `depthBuffer: false`.
    pub fn render(&self, renderer: &mut Renderer) {
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
        self.set_size(width, height);

        // 1. extract bright areas
        renderer.set_render_target(Some(self.bright.clone()));
        renderer.render_quad(&self.high_pass);

        // 2. blur all the mips progressively
        for (i, mip) in self.blur.iter().enumerate() {
            renderer.set_render_target(Some(self.horizontal[i].clone()));
            renderer.render_quad(&mip.horizontal);

            renderer.set_render_target(Some(self.vertical[i].clone()));
            renderer.render_quad(&mip.vertical);
        }

        // 3. composite all the mips, back into `h0`
        renderer.set_render_target(Some(self.horizontal[0].clone()));
        renderer.render_quad(&self.composite);

        // `RendererUtils.restoreRendererState( renderer, _rendererState )`.
        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        renderer.set_clear_color(previous_clear_color, previous_clear_alpha);
        renderer.auto_clear = previous_auto_clear;
    }
}

#[cfg(test)]
/// The five `Bloom_separable` materials' coefficients, for the unit tests:
/// `( centreWeight, offsets, weights )` per mip.
fn kernel_coefficients() -> Vec<(f64, Vec<f64>, Vec<f64>)> {
    KERNEL_SIZES
        .iter()
        .map(|&radius| separable_blur_coefficients(radius))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `dump/m05..m09`'s baked arrays, read off the five `Bloom_separable`
    /// modules. The *code* computes them; this asserts three.js' own floats,
    /// down to the last bit of the `f64` the JS Gaussian produced.
    #[test]
    fn the_five_gaussian_kernels_match_the_dumped_coefficients() {
        let kernels = kernel_coefficients();
        assert_eq!(
            kernels.iter().map(|(_, o, _)| o.len()).collect::<Vec<_>>(),
            // `[ 6, 10, 14, 18, 22 ]` taps become `[ 3, 5, 7, 9, 11 ]`
            // bilinear ones — the literal loop bound of each module.
            vec![3, 5, 7, 9, 11]
        );

        // mip 0 — `dump/m05`
        assert_eq!(kernels[0].0, 0.19947);
        assert_eq!(kernels[0].1, vec![1.40733340004593, 3.294214972162989, 5.0]);
        assert_eq!(
            kernels[0].2,
            vec![
                0.2970163278514283,
                0.09175375661117716,
                0.008764100149861079
            ]
        );

        // mip 1 — `dump/m06`
        assert_eq!(kernels[1].0, 0.119682);
        assert_eq!(
            kernels[1].1,
            vec![
                1.4663011645670991,
                3.421894767115691,
                5.378716404808932,
                7.337378162829917,
                9.0
            ]
        );
        assert_eq!(
            kernels[1].2,
            vec![
                0.21438250006287293,
                0.13808060217496526,
                0.06253996870210721,
                0.019913324055006204,
                0.0031262625741366435
            ]
        );

        // mip 2 — `dump/m07`
        assert_eq!(kernels[2].0, 0.08548714285714286);
        assert_eq!(
            kernels[2].1,
            vec![
                1.4827874165827566,
                3.45990768708058,
                5.437195705962934,
                7.4147440341828155,
                9.392640964546707,
                11.370969190043434,
                13.0
            ]
        );
        assert_eq!(
            kernels[2].2,
            vec![
                0.16153278215047456,
                0.1287340360249976,
                0.08555930154584468,
                0.04742132242468507,
                0.02191797951110977,
                0.008447578271836165,
                0.001765199910796836
            ]
        );

        // mip 3 — `dump/m08`
        assert_eq!(kernels[3].0, 0.06649000000000001);
        assert_eq!(
            kernels[3].1,
            vec![
                1.4895848401126355,
                3.4757135713665743,
                5.4618796740944076,
                7.448104232731768,
                9.434407974610943,
                11.4208111469608,
                13.407333400045928,
                15.39399367783732,
                17.0
            ]
        );
        assert_eq!(
            kernels[3].2,
            vec![
                0.1284697562799138,
                0.11191824897278832,
                0.08731326755905255,
                0.06100111134675511,
                0.0381655709162591,
                0.02138356611366251,
                0.010729024104628605,
                0.004820686863785114,
                0.001201009762281257
            ]
        );

        // mip 4 — `dump/m09`
        assert_eq!(kernels[4].0, 0.0544009090909091);
        assert_eq!(
            kernels[4].1,
            vec![
                1.4930273115580075,
                3.483735079616612,
                5.474454081211128,
                7.465190698953718,
                9.455951266953607,
                11.446742053599085,
                13.437569244721434,
                15.42843892723576,
                17.419357073349026,
                19.410329525419996,
                21.0
            ]
        );
        assert_eq!(
            kernels[4].2,
            vec![
                0.10631235328115614,
                0.09691539802869314,
                0.08204439964112456,
                0.06449885004133733,
                0.04708705604762678,
                0.03192252271800665,
                0.020097333155365302,
                0.011749640609175892,
                0.006379034087935798,
                0.0032160931712181943,
                0.0009013823526604976
            ]
        );
    }

    /// The last tap of an odd kernel has no partner: `wb` is 0, the weight is
    /// `wa` alone and the offset is the whole integer `i` — which is why every
    /// offsets array ends on a round number (5, 9, 13, 17, 21).
    #[test]
    fn the_last_tap_of_each_kernel_is_unpaired() {
        for (radius, (_, offsets, _)) in KERNEL_SIZES.iter().zip(kernel_coefficients()) {
            assert_eq!(*offsets.last().unwrap(), (radius - 1) as f64);
        }
    }

    /// `dump/dump.json`'s eleven bloom textures: 400×250, 200×125, 100×62,
    /// 50×31, 25×15 at the default resolution scale. `Math.floor( 125 / 2 )`
    /// is 62 — rounding would give 63, and the halving is floored twice more
    /// after that.
    #[test]
    fn the_mip_chain_is_floor_halved() {
        assert_eq!(
            mip_sizes(800, 500, 0.5),
            [(400, 250), (200, 125), (100, 62), (50, 31), (25, 15)]
        );
    }

    /// `setResolutionScale()` really moves the chain — the whole of what the
    /// knob is for.
    #[test]
    fn the_resolution_scale_scales_the_chain() {
        assert_eq!(mip_sizes(800, 500, 1.0)[0], (800, 500));
        assert_eq!(mip_sizes(800, 500, 0.25)[0], (200, 125));
        // `Math.floor( 500 * 0.3 )` is 150, not 149.99999999999997's ceiling.
        assert_eq!(mip_sizes(800, 500, 0.3)[0], (240, 150));
    }
}
