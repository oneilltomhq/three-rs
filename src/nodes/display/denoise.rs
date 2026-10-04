//! Port of `three.js/examples/jsm/tsl/display/DenoiseNode.js` — a single
//! pass, 16-tap, Poisson-disk, edge-aware denoiser.
//!
//! Every tap of a disk of 16 offsets (two rings, radius growing linearly
//! with the index), rotated per pixel by a tiled simplex-noise texture, is
//! weighted by how close its luma, its view normal and its distance from the
//! centre's tangent plane are to the centre pixel's. Sky pixels (`depth >=
//! 1`) and pixels without a normal pass through.
//!
//! Three's node is a plain `Node`, not a `TempNode`: it draws nothing of its
//! own and inlines into whichever material reads it, which `denoise()` makes
//! an RTT quad by wrapping its input in `convertToTexture()`. Its
//! `updateBefore()` only copies the input's size into `_resolution`; the port
//! reads that size (and the camera's `projectionMatrixInverse`, which three
//! binds by reference) when the uniforms are uploaded, so there is no update
//! step at all.
//!
//! Not ported: nothing in the graph. `generateDefaultNoise()` seeds its
//! [`SimplexNoise`] from `Math.random`; the port has no global random
//! source, so it draws from [`DeterministicRandom`] — the sequence the e2e
//! harness gives three's pages — and the texture's contents are only as
//! reproducible as three's.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::addons::simplex_noise::SimplexNoise;
use crate::cameras::PerspectiveCamera;
use crate::nodes::node::{
    LiveValue, SettableValue, TextureSource, Type, UniformGroup, UniformSource,
};
use crate::nodes::tsl::{
    abs, block, dot, float, get_normal_from_depth, get_view_position, if_else, if_then, int,
    loop_options, luminance, mat2_join, max, pass_depth_texture_uv, pi, property, texture_sample,
    texture_size, texture_uv, to_const, to_var, uniform, uniform_array_vec3, uniform_settable, uv,
    vec2_join, vec3_join, vec4_join, UniformArray,
};
use crate::nodes::NodeRef;
use crate::testing::DeterministicRandom;
use crate::textures::{DepthTexture, MinFilter, Texture, TextureFilter, Wrapping};

use super::SampleFn;

/// `denoise( node, depthNode, normalNode, camera )`.
///
/// `input` is the texture three's `convertToTexture( node )` renders (the
/// port's [`convert_to_texture`](super::convert_to_texture)`( node
/// ).texture()`, or a pass attachment), `depth` the pass's depth attachment,
/// `normal` a sampler of the view normal (`None` reconstructs it from the
/// depth with `getNormalFromDepth`), and `camera` the pass's.
pub fn denoise(
    input: &Texture,
    depth: &DepthTexture,
    normal: Option<SampleFn>,
    camera: &Rc<RefCell<PerspectiveCamera>>,
) -> DenoiseNode {
    DenoiseNode::new(input, depth, normal, camera)
}

/// `DenoiseNode` — see the module docs.
pub struct DenoiseNode {
    /// `this.lumaPhi` — luma edge stopping: a tap whose luma differs by
    /// `lumaPhi` or more gets no weight. 5 by default.
    pub luma_phi: SettableValue,
    /// `this.depthPhi` — the view-space distance from the centre's tangent
    /// plane at which a tap's weight reaches zero. 5 by default.
    pub depth_phi: SettableValue,
    /// `this.normalPhi` — the exponent on the normals' cosine. 5 by default.
    pub normal_phi: SettableValue,
    /// `this.radius` — the disk's outer radius in pixels. 5 by default.
    pub radius: SettableValue,
    /// `this.index` — picks which noise channel rotates the disk
    /// (`noiseTexel[ index % 4 * 2π ]`). 0 by default.
    pub index: SettableValue,
    /// `this._noiseTexture`.
    noise: Texture,
    /// What `setup()` returns.
    node: NodeRef,
}

/// The uniforms `setup()`'s `Fn`s close over.
struct Uniforms {
    luma_phi: NodeRef,
    depth_phi: NodeRef,
    normal_phi: NodeRef,
    radius: NodeRef,
    index: NodeRef,
    resolution: NodeRef,
    sample_vectors: UniformArray,
    projection_inverse: NodeRef,
}

/// The samplers `setup()` builds over the node's inputs.
struct Inputs<'a> {
    input: &'a Texture,
    depth: &'a DepthTexture,
    normal: Option<SampleFn>,
    noise: &'a Texture,
    u: Uniforms,
}

impl Inputs<'_> {
    /// `sampleTexture( uv )` — `this.textureNode.sample( uv )`.
    fn sample_texture(&self, coord: NodeRef) -> NodeRef {
        texture_uv(self.input, coord)
    }

    /// `sampleDepth( uv )` — `this.depthNode.sample( uv ).x`.
    fn sample_depth(&self, coord: NodeRef) -> NodeRef {
        pass_depth_texture_uv(self.depth, coord)
    }

    /// `sampleNormal( uv )` — `this.normalNode.sample( uv ).rgb.normalize()`,
    /// or `getNormalFromDepth( uv, depthNode.value, projectionMatrixInverse
    /// )` without a normal node.
    fn sample_normal(&self, coord: NodeRef) -> NodeRef {
        match &self.normal {
            Some(normal) => normal(coord).rgb().normalize(),
            None => get_normal_from_depth(coord, self.depth, self.u.projection_inverse.clone()),
        }
    }
}

impl DenoiseNode {
    /// `new DenoiseNode( textureNode, depthNode, normalNode, camera )` with
    /// the graph `setup()` returns. See [`denoise`].
    pub fn new(
        input: &Texture,
        depth: &DepthTexture,
        normal: Option<SampleFn>,
        camera: &Rc<RefCell<PerspectiveCamera>>,
    ) -> Self {
        let (luma_phi, luma_phi_value) = uniform_settable(Type::F32, vec![5.0]);
        let (depth_phi, depth_phi_value) = uniform_settable(Type::F32, vec![5.0]);
        let (normal_phi, normal_phi_value) = uniform_settable(Type::F32, vec![5.0]);
        let (radius, radius_value) = uniform_settable(Type::F32, vec![5.0]);
        let (index, index_value) = uniform_settable(Type::F32, vec![0.0]);

        // `_resolution`, which `updateBefore()` sets to the input's size.
        let map = input.clone();
        let resolution = uniform(
            UniformSource::Live(LiveValue::new(move || {
                let (width, height) = map.size();
                vec![f64::from(width), f64::from(height)]
            })),
            Type::Vec2,
            UniformGroup::Object,
            None,
        );
        // `uniform( camera.projectionMatrixInverse )` — by reference.
        let camera: Weak<RefCell<PerspectiveCamera>> = Rc::downgrade(camera);
        let projection_inverse = uniform(
            UniformSource::Live(LiveValue::new(move || {
                let camera = camera
                    .upgrade()
                    .expect("three-rs: a DenoiseNode outlived its camera");
                let m = camera.borrow().projection_matrix_inverse.elements;
                m.to_vec()
            })),
            Type::Mat4,
            UniformGroup::Object,
            None,
        );
        let sample_vectors = uniform_array_vec3(&generate_denoise_samples(16, 2, 1.0));

        let noise = generate_default_noise(64);
        let inputs = Inputs {
            input,
            depth,
            normal,
            noise: &noise,
            u: Uniforms {
                luma_phi,
                depth_phi,
                normal_phi,
                radius,
                index,
                resolution,
                sample_vectors,
                projection_inverse,
            },
        };
        // `output = Fn( () => denoise( uvNode ) )`.
        let node = denoise_fn(&inputs, uv());
        Self {
            luma_phi: luma_phi_value,
            depth_phi: depth_phi_value,
            normal_phi: normal_phi_value,
            radius: radius_value,
            index: index_value,
            noise,
            node,
        }
    }

    /// The node `setup()` returns: the denoised colour at `uv()`.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `this._noiseTexture` — the 64×64 simplex-noise rotation texture.
    pub fn noise_texture(&self) -> &Texture {
        &self.noise
    }
}

/// `denoiseSample( center, viewNormal, viewPosition, sampleUv )`: one tap's
/// colour times its weight, and the weight. No layout, so it inlines.
#[inline(never)]
fn denoise_sample(
    s: &Inputs,
    center: &NodeRef,
    view_normal: &NodeRef,
    view_position: &NodeRef,
    sample_uv: NodeRef,
) -> (Vec<NodeRef>, NodeRef) {
    let u = &s.u;
    let texel = to_var(None, s.sample_texture(sample_uv.clone()));
    let depth = to_var(None, s.sample_depth(sample_uv.clone()));
    let normal = to_var(None, s.sample_normal(sample_uv.clone()));
    let neighbor_color = texel.rgb();
    let view_pos = to_var(
        None,
        get_view_position(sample_uv, depth.clone(), u.projection_inverse.clone()),
    );
    let normal_diff = to_var(None, dot(view_normal.clone(), normal.clone()));
    let normal_similarity = to_var(
        None,
        max(normal_diff.clone(), 0.0).pow(u.normal_phi.clone()),
    );
    let luma_diff = to_var(
        None,
        abs(luminance(neighbor_color.clone()).sub(luminance(center.clone()))),
    );
    let luma_similarity = to_var(
        None,
        max(float(1.0).sub(luma_diff.div(u.luma_phi.clone())), 0.0),
    );
    let depth_diff = to_var(
        None,
        abs(dot(
            view_position.sub(view_pos.clone()),
            view_normal.clone(),
        )),
    );
    let depth_similarity = max(float(1.0).sub(depth_diff.div(u.depth_phi.clone())), 0.0);
    let w = luma_similarity
        .mul(depth_similarity)
        .mul(normal_similarity.clone());
    let statements = vec![
        texel,
        depth,
        normal,
        view_pos,
        normal_diff,
        normal_similarity,
        luma_diff,
        luma_similarity,
        depth_diff,
    ];
    (
        statements,
        vec4_join(vec![neighbor_color.mul(w.clone()), w]),
    )
}

/// `denoise( uvNode )` — the `Fn()` whose layout three comments out, so it
/// inlines: pass the sky and normal-less pixels through, denoise the rest
/// into the `result` property.
#[inline(never)]
fn denoise_fn(s: &Inputs, uv_node: NodeRef) -> NodeRef {
    let depth = to_var(None, s.sample_depth(uv_node.clone()));
    let view_normal = to_var(None, s.sample_normal(uv_node.clone()));
    let texel = to_var(None, s.sample_texture(uv_node.clone()));
    let result = property("denoiseResult", Type::Vec4);
    let condition = depth
        .greater_than_equal(1.0)
        .or(dot(view_normal.clone(), view_normal.clone()).equal(0.0));
    let branch = if_else(
        condition,
        vec![result.assign(texel.clone())],
        denoise_else(s, &uv_node, &depth, &view_normal, &texel, &result),
    );
    block(vec![depth, view_normal, texel, branch], result)
}

/// `denoise()`'s `Else` branch: the noise rotation, the 16-tap loop and the
/// normalisation.
#[inline(never)]
fn denoise_else(
    s: &Inputs,
    uv_node: &NodeRef,
    depth: &NodeRef,
    view_normal: &NodeRef,
    texel: &NodeRef,
    result: &NodeRef,
) -> Vec<NodeRef> {
    let u = &s.u;
    let center = vec3_join(vec![texel.rgb()]);
    let view_position = to_const(
        None,
        get_view_position(uv_node.clone(), depth.clone(), u.projection_inverse.clone()),
    );

    let noise_resolution = texture_size(TextureSource::Texture2D(s.noise.clone()), int(0));
    let noise_uv = vec2_join(vec![uv_node.x(), uv_node.y().one_minus()])
        .mul(u.resolution.div(noise_resolution));
    let noise_texel = to_var(None, texture_sample(s.noise, noise_uv));
    let channel = || u.index.mod_(4.0).mul(2.0).mul(pi());
    let x = noise_texel.element_node(channel()).sin();
    let y = noise_texel.element_node(channel()).cos();
    let noise_vec = vec2_join(vec![x, y]);
    let rotation_matrix = mat2_join(vec![
        noise_vec.x(),
        noise_vec.y().negate(),
        noise_vec.x(),
        noise_vec.y(),
    ]);

    let total_weight = to_var(None, float(1.0));
    let denoised = to_var(None, vec3_join(vec![texel.rgb()]));

    let taps = loop_options("i", Type::I32, int(0), int(16), "<", |i| {
        let sample_dir = u.sample_vectors.element_xyz(i.clone());
        let offset = rotation_matrix
            .mul(
                sample_dir
                    .xy()
                    .mul(float(1.0).add(sample_dir.z().mul(u.radius.sub(1.0)))),
            )
            .div(u.resolution.clone());
        let sample_uv = uv_node.add(offset);
        let (mut statements, sample_result) =
            denoise_sample(s, &center, view_normal, &view_position, sample_uv);
        statements.push(denoised.add_assign(sample_result.xyz()));
        statements.push(total_weight.add_assign(sample_result.w()));
        statements
    });

    vec![
        view_position.clone(),
        noise_texel,
        total_weight.clone(),
        denoised.clone(),
        taps,
        if_then(
            total_weight.greater_than(float(0.0)),
            vec![denoised.div_assign(total_weight.clone())],
        ),
        result.assign(vec4_join(vec![denoised, texel.a()])),
    ]
}

/// `generateDenoiseSamples( numSamples, numRings, radiusExponent )`: the
/// disk's offsets as `( cos θ, sin θ, r )`, `θ = 2π · rings · i / n` and `r =
/// ( i / ( n - 1 ) )^exponent`.
pub fn generate_denoise_samples(
    num_samples: usize,
    num_rings: usize,
    radius_exponent: f64,
) -> Vec<[f64; 3]> {
    (0..num_samples)
        .map(|i| {
            let angle =
                2.0 * std::f64::consts::PI * num_rings as f64 * i as f64 / num_samples as f64;
            let radius = (i as f64 / (num_samples - 1) as f64).powf(radius_exponent);
            [angle.cos(), angle.sin(), radius]
        })
        .collect()
}

/// `generateDefaultNoise( size )`: a `size`×`size` RGBA8 `DataTexture`, each
/// channel `( noise * 0.5 + 0.5 ) * 255` of 2D simplex noise at `( x, y )`,
/// `( x + size, y )`, `( x, y + size )` and `( x + size, y + size )`, with
/// `RepeatWrapping`. The [`SimplexNoise`] draws from [`DeterministicRandom`]
/// where three's draws from `Math.random`.
pub fn generate_default_noise(size: u32) -> Texture {
    let mut random = DeterministicRandom::new();
    let simplex = SimplexNoise::new(|| random.next());
    let n = size as usize;
    let s = f64::from(size);
    let mut data = vec![0u8; n * n * 4];
    // `Uint8Array` stores truncate toward zero after `ToUint8`; the values
    // are already in [0, 255].
    let byte = |v: f64| ((v * 0.5 + 0.5) * 255.0) as u8;
    for i in 0..n {
        for j in 0..n {
            let (x, y) = (i as f64, j as f64);
            let at = (i * n + j) * 4;
            data[at] = byte(simplex.noise(x, y));
            data[at + 1] = byte(simplex.noise(x + s, y));
            data[at + 2] = byte(simplex.noise(x, y + s));
            data[at + 3] = byte(simplex.noise(x + s, y + s));
        }
    }
    // `new DataTexture( data, size, size )`: nearest filters, no mipmaps,
    // `flipY = false`; then `RepeatWrapping` both ways.
    let texture = Texture::new(size, size, Some(data));
    texture.set_flip_y(false);
    texture.set_generate_mipmaps(false);
    texture.set_min_filter(MinFilter::Nearest);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `generateDenoiseSamples( 16, 2, 1 )`: two turns of the circle, the
    /// radius from 0 to 1.
    #[test]
    fn denoise_samples_match_three() {
        let samples = generate_denoise_samples(16, 2, 1.0);
        assert_eq!(samples.len(), 16);
        assert_eq!(samples[0], [1.0, 0.0, 0.0]);
        let [x, y, r] = samples[2];
        assert!((x - (std::f64::consts::PI / 2.0).cos()).abs() < 1e-15);
        assert!((y - 1.0).abs() < 1e-15);
        assert!((r - 2.0 / 15.0).abs() < 1e-15);
        assert_eq!(samples[15][2], 1.0);
    }
}
