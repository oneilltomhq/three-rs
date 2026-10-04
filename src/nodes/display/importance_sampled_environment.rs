//! Port of `examples/jsm/tsl/display/ImportanceSampledEnvironment.js`: HDR
//! environment lookups for screen-space effects, with luminance importance
//! sampling (CDF tables + MIS) for the rays SSR could not resolve on screen.
//!
//! [`EnvMapCdfGenerator`] is three's `EnvMapCDFGenerator`: it converts an
//! equirectangular environment map to a half-float copy (`preprocessEnvMap`)
//! and, for importance sampling, builds the marginal and conditional
//! inverse-CDF tables into `r16float` `DataTexture`s on the CPU.
//! [`ImportanceSampledEnvironment`] owns one, plus the `totalSum`, `size` and
//! `intensity` uniforms, and builds the three lookups three exposes:
//! [`sample_reflect`](ImportanceSampledEnvironment::sample_reflect),
//! [`sample_environment_brdf`](ImportanceSampledEnvironment::sample_environment_brdf)
//! and [`sample_environment_mis`](ImportanceSampledEnvironment::sample_environment_mis).
//!
//! The WGSL of all three is gated against three's dump of
//! `tools/dump-pages/specular_helpers.html` in `tests/nodes_display_wgsl.rs`,
//! and the CPU tables by this module's unit test (`docs/nodes.md` §85).
//!
//! **Divergence, texture identity.** Three's `updateFrom()` builds new
//! textures every time and points the existing texture nodes at them
//! (`this._mapNode.value = map`). A texture node here holds its [`Texture`]
//! handle and has no `value` to reassign, so a later `update_from` with an
//! environment of the same size writes the new pixels into the textures the
//! graphs already sample; one of a different size allocates new textures,
//! which only graphs built after it read.

use crate::extras::{from_half_float, to_half_float};
use crate::nodes::node::{NodeRef, SettableValue, Type};
use crate::nodes::tsl::{
    d_gtr, dot, equirect_dir_pdf, equirect_uv, f_schlick, float, geometry_term, if_then, luminance,
    max, mis_power_heuristic, smith_g, texture_level, texture_with_uv, to_var, uniform_settable,
    vec2_join, vec4_join,
};
use crate::textures::{MinFilter, Texture, TextureFilter, Wrapping};

/// `colorToLuminance( r, g, b )` — the Rec. 709 weights the CDF uses.
fn color_to_luminance(r: f64, g: f64, b: f64) -> f64 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// `binarySearchFindClosestIndexOf( array, targetValue, offset, count )` —
/// the first index in `array[ offset .. offset + count ]` whose value is not
/// below `target_value` (the last one if none is), relative to `offset`. The
/// values are `Float32Array` entries compared against a JS number, hence the
/// widening.
fn binary_search_find_closest_index_of(
    array: &[f32],
    target_value: f64,
    offset: usize,
    count: usize,
) -> usize {
    let mut lower = offset;
    let mut upper = offset + count - 1;
    while lower < upper {
        let mid = (lower + upper) >> 1;
        if (array[mid] as f64) < target_value {
            lower = mid + 1;
        } else {
            upper = mid;
        }
    }
    lower - offset
}

/// `preprocessEnvMap( envMap )` — a clone of the map whose data is RGBA
/// half floats with no Y flip: float data is converted as is, normalized
/// 8-bit data divided by 255, and a `flipY` map has its rows reversed.
fn preprocess_env_map(env_map: &Texture) -> Texture {
    let map = env_map.clone_texture();
    let (width, _) = env_map.size();
    let (format, flip_y, bytes) = {
        let inner = env_map.borrow();
        let bytes = inner.data.clone().expect(
            "three-rs: ImportanceSampledEnvironment needs an environment map with CPU \
             pixel data (a DataTexture or a loaded HDR), not a render target",
        );
        (inner.format, inner.flip_y, bytes)
    };
    let mut halves: Vec<u16> = match format {
        wgpu::TextureFormat::Rgba16Float => bytemuck::pod_collect_to_vec(&bytes),
        wgpu::TextureFormat::Rgba32Float => bytemuck::pod_collect_to_vec::<u8, f32>(&bytes)
            .into_iter()
            .map(|v| to_half_float(v as f64))
            .collect(),
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => bytes
            .iter()
            .map(|&v| to_half_float(v as f64 / 255.0))
            .collect(),
        other => panic!(
            "three-rs: ImportanceSampledEnvironment reads RGBA float, half-float or 8-bit \
             environment maps, not {other:?}"
        ),
    };
    if flip_y {
        let stride = 4 * width as usize;
        let flipped: Vec<u16> = halves
            .chunks_exact(stride)
            .rev()
            .flatten()
            .copied()
            .collect();
        halves = flipped;
    }
    {
        let mut inner = map.borrow_mut_inner();
        inner.format = wgpu::TextureFormat::Rgba16Float;
        inner.flip_y = false;
        inner.data = Some(bytemuck::cast_slice(&halves).to_vec());
        inner.version += 1;
    }
    map
}

/// The CPU half of `EnvMapCDFGenerator.updateFrom()`: the half-float bit
/// patterns of the marginal (`height` texels) and conditional (`width ×
/// height`) inverse-CDF tables, and the summed luminance.
#[derive(Debug, PartialEq)]
struct CdfTables {
    marginal: Vec<u16>,
    conditional: Vec<u16>,
    total_sum: f64,
}

/// The table arithmetic of `EnvMapCDFGenerator.updateFrom()` over RGBA half
/// floats, `Float32Array` storage included. Three also fills the two PDF
/// arrays, which nothing reads; they are left out.
fn cdf_tables(width: usize, height: usize, data: &[u16]) -> CdfTables {
    let mut cdf_conditional = vec![0.0f32; width * height];
    let mut cdf_marginal = vec![0.0f32; height];
    let mut total_sum = 0.0f64;
    let mut cumulative_weight_marginal = 0.0f64;

    for y in 0..height {
        let mut cumulative_row_weight = 0.0f64;
        for x in 0..width {
            let i = y * width + x;
            let r = from_half_float(data[4 * i]) as f64;
            let g = from_half_float(data[4 * i + 1]) as f64;
            let b = from_half_float(data[4 * i + 2]) as f64;
            let weight = color_to_luminance(r, g, b);
            cumulative_row_weight += weight;
            total_sum += weight;
            cdf_conditional[i] = cumulative_row_weight as f32;
        }
        if cumulative_row_weight != 0.0 {
            for value in &mut cdf_conditional[y * width..(y + 1) * width] {
                *value = (*value as f64 / cumulative_row_weight) as f32;
            }
        }
        cumulative_weight_marginal += cumulative_row_weight;
        cdf_marginal[y] = cumulative_weight_marginal as f32;
    }
    if cumulative_weight_marginal != 0.0 {
        for value in &mut cdf_marginal {
            *value = (*value as f64 / cumulative_weight_marginal) as f32;
        }
    }

    let marginal = (0..height)
        .map(|i| {
            let dist = (i + 1) as f64 / height as f64;
            let row = binary_search_find_closest_index_of(&cdf_marginal, dist, 0, height);
            to_half_float((row as f64 + 0.5) / height as f64)
        })
        .collect();
    let mut conditional = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let dist = (x + 1) as f64 / width as f64;
            let col = binary_search_find_closest_index_of(&cdf_conditional, dist, y * width, width);
            conditional.push(to_half_float((col as f64 + 0.5) / width as f64));
        }
    }
    CdfTables {
        marginal,
        conditional,
        total_sum,
    }
}

/// One CDF table as three builds it: a `RedFormat` + `HalfFloatType`
/// `DataTexture` with `LinearFilter` on both sides, clamped on both axes and
/// no mipmaps.
fn cdf_texture(width: u32, height: u32, data: &[u16]) -> Texture {
    let texture = Texture::data_r16float(width, height, data);
    texture.set_min_filter(MinFilter::Linear);
    texture.set_mag_filter(TextureFilter::Linear);
    texture.set_wrapping(Wrapping::ClampToEdge, Wrapping::ClampToEdge);
    texture.set_generate_mipmaps(false);
    texture
}

/// Puts `fresh` in `slot`: written into the texture already there when it has
/// the same size and format (so graphs that sample it see the new pixels, as
/// three's `node.value = texture` makes them), or in place of it otherwise.
fn install(slot: &mut Option<Texture>, fresh: Texture) {
    if let Some(current) = slot {
        if current.size() == fresh.size() && current.format() == fresh.format() {
            let source = fresh.borrow();
            let mut target = current.borrow_mut_inner();
            target.data = source.data.clone();
            target.color_space = source.color_space;
            target.flip_y = source.flip_y;
            target.generate_mipmaps = source.generate_mipmaps;
            target.wrap_s = source.wrap_s;
            target.wrap_t = source.wrap_t;
            target.mag_filter = source.mag_filter;
            target.min_filter = source.min_filter;
            target.anisotropy = source.anisotropy;
            target.version += 1;
            return;
        }
    }
    *slot = Some(fresh);
}

/// `EnvMapCDFGenerator` — the preprocessed environment map and its marginal
/// and conditional inverse-CDF textures for luminance importance sampling.
///
/// The marginal table is `height × 1` and holds, for each `( i + 1 ) / height`
/// quantile, the centre `v` of the row the marginal CDF reaches it in; row `y`
/// of the `width × height` conditional table holds the same per column within
/// row `y`. Both are `r16float` with linear filtering and clamped edges; the
/// map is RGBA half float, repeating in `s` and clamped in `t`. Three's code
/// weights each texel by its luminance alone — there is no `sin θ` term.
#[derive(Default)]
pub struct EnvMapCdfGenerator {
    map: Option<Texture>,
    marginal_weights: Option<Texture>,
    conditional_weights: Option<Texture>,
    total_sum: f64,
}

impl EnvMapCdfGenerator {
    /// `new EnvMapCDFGenerator()` — no map and no tables yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// `updateFrom( hdr )` — preprocesses the map and builds both CDF tables
    /// and the luminance total from it.
    pub fn update_from(&mut self, hdr: &Texture) {
        self.update_map_only(hdr);
        let map = self.map.as_ref().expect("the map was set just above");
        let (width, height) = map.size();
        let halves: Vec<u16> = bytemuck::pod_collect_to_vec(
            map.borrow()
                .data
                .as_ref()
                .expect("a preprocessed map has data"),
        );
        let tables = cdf_tables(width as usize, height as usize, &halves);
        install(
            &mut self.marginal_weights,
            cdf_texture(height, 1, &tables.marginal),
        );
        install(
            &mut self.conditional_weights,
            cdf_texture(width, height, &tables.conditional),
        );
        self.total_sum = tables.total_sum;
    }

    /// `updateMapOnly( hdr )` — the preprocessed map alone (repeating in `s`,
    /// clamped in `t`), with the luminance total reset to 0.
    pub fn update_map_only(&mut self, hdr: &Texture) {
        let map = preprocess_env_map(hdr);
        map.set_wrapping(Wrapping::Repeat, Wrapping::ClampToEdge);
        install(&mut self.map, map);
        self.total_sum = 0.0;
    }

    /// `generator.map` — the preprocessed RGBA half-float environment.
    pub fn map(&self) -> Option<&Texture> {
        self.map.as_ref()
    }

    /// `generator.marginalWeights` — the `height × 1` marginal table.
    pub fn marginal_weights(&self) -> Option<&Texture> {
        self.marginal_weights.as_ref()
    }

    /// `generator.conditionalWeights` — the `width × height` conditional table.
    pub fn conditional_weights(&self) -> Option<&Texture> {
        self.conditional_weights.as_ref()
    }

    /// `generator.totalSum` — the summed luminance of every texel.
    pub fn total_sum(&self) -> f64 {
        self.total_sum
    }

    /// `dispose()` — drops the map and the tables. A texture's GPU copy goes
    /// with its last handle, so graphs that still sample one keep it alive.
    pub fn dispose(&mut self) {
        self.marginal_weights = None;
        self.conditional_weights = None;
        self.map = None;
    }
}

/// The lobe [`ImportanceSampledEnvironment::sample_environment_brdf`] and
/// [`sample_environment_mis`](ImportanceSampledEnvironment::sample_environment_mis)
/// evaluate — the object three passes them.
#[derive(Clone)]
pub struct EnvironmentLobe {
    /// `cameraWorldMatrix` — the `mat4` uniform taking view space to world.
    pub camera_world_matrix: NodeRef,
    /// `viewReflectDir` — the view-space GGX-sampled reflected ray.
    pub view_reflect_dir: NodeRef,
    /// `N` — the view-space shading normal.
    pub n: NodeRef,
    /// `V` — the view-space direction to the camera.
    pub v: NodeRef,
    /// `alpha` — the GGX roughness α.
    pub alpha: NodeRef,
    /// `f0` — the Fresnel reflectance at normal incidence (`vec3`).
    pub f0: NodeRef,
}

/// `ImportanceSampledEnvironment` — a preprocessed HDR environment (CDF
/// textures and uniforms) and the TSL lookups SSR's environment fallback
/// uses. See the module docs.
pub struct ImportanceSampledEnvironment {
    importance_sampling: bool,
    cdf: EnvMapCdfGenerator,
    total_sum_node: NodeRef,
    total_sum: SettableValue,
    size_node: NodeRef,
    size: SettableValue,
    intensity_node: NodeRef,
    intensity: SettableValue,
}

impl ImportanceSampledEnvironment {
    /// `new ImportanceSampledEnvironment( importanceSampling )` — with
    /// `importance_sampling`, [`update_from`](Self::update_from) also builds
    /// the luminance CDF tables
    /// [`sample_environment_mis`](Self::sample_environment_mis) reads.
    pub fn new(importance_sampling: bool) -> Self {
        let (total_sum_node, total_sum) = uniform_settable(Type::F32, vec![0.0]);
        let (size_node, size) = uniform_settable(Type::Vec2, vec![1.0, 1.0]);
        let (intensity_node, intensity) = uniform_settable(Type::F32, vec![1.0]);
        Self {
            importance_sampling,
            cdf: EnvMapCdfGenerator::new(),
            total_sum_node,
            total_sum,
            size_node,
            size,
            intensity_node,
            intensity,
        }
    }

    /// `updateFrom( hdr )` — takes an equirectangular HDR environment map
    /// (RGBA float, half float or 8-bit, with CPU pixel data).
    pub fn update_from(&mut self, hdr: &Texture) {
        if self.importance_sampling {
            self.cdf.update_from(hdr);
            self.total_sum.set(vec![self.cdf.total_sum()]);
        } else {
            self.cdf.update_map_only(hdr);
        }
        let (width, height) = self.cdf.map().expect("update_from sets the map").size();
        self.size.set(vec![width as f64, height as f64]);
    }

    /// `clear()` — drops the environment and resets the uniforms; the
    /// lookups need another [`update_from`](Self::update_from) before they
    /// can be built again.
    pub fn clear(&mut self) {
        self.dispose();
        self.cdf = EnvMapCdfGenerator::new();
        self.total_sum.set(vec![0.0]);
        self.size.set(vec![1.0, 1.0]);
    }

    /// `this.intensity.value` — the radiance scale every lookup applies. 1 by
    /// default.
    pub fn intensity(&self) -> &SettableValue {
        &self.intensity
    }

    /// The CDF generator, for its textures and luminance total.
    pub fn generator(&self) -> &EnvMapCdfGenerator {
        &self.cdf
    }

    /// `dispose()`.
    pub fn dispose(&mut self) {
        self.cdf.dispose();
    }

    fn map(&self) -> &Texture {
        self.cdf.map().expect(
            "three-rs: ImportanceSampledEnvironment has no environment; call update_from first",
        )
    }

    /// `texture( this._mapNode, uv ).level( 0 ).rgb` — explicit LOD 0, since
    /// the equirect seam and poles would pull derivative-driven mip selection
    /// to the coarsest level.
    fn map_level_0(&self, uv: NodeRef) -> NodeRef {
        texture_level(self.map(), uv, float(0.0)).rgb()
    }

    /// `sampleReflect( { cameraWorldMatrix, viewReflectDir, sampleWeight } )` —
    /// the environment along the reflected direction, no MIS.
    /// `sample_weight` defaults to `float( 1 )`.
    pub fn sample_reflect(
        &self,
        camera_world_matrix: &NodeRef,
        view_reflect_dir: NodeRef,
        sample_weight: Option<NodeRef>,
    ) -> NodeRef {
        let sample_weight = sample_weight.unwrap_or_else(|| float(1.0));
        let world_reflect_dir = camera_world_matrix
            .mul(vec4_join(vec![view_reflect_dir, float(0.0)]))
            .xyz()
            .normalize();
        let env_uv = equirect_uv(world_reflect_dir);
        self.map_level_0(env_uv)
            .mul(self.intensity_node.clone())
            .mul(sample_weight)
    }

    /// `sampleEnvironmentBRDF( lobe )` — the environment for a screen-space
    /// miss along the BRDF-sampled ray only, weighted by `F · G2 / G1( N·V )`.
    #[inline(never)]
    pub fn sample_environment_brdf(&self, lobe: &EnvironmentLobe) -> NodeRef {
        let m = &lobe.camera_world_matrix;
        let world_normal = to_var(
            None,
            m.mul(vec4_join(vec![lobe.n.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let world_v = to_var(
            None,
            m.mul(vec4_join(vec![lobe.v.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let n_dot_v = to_var(
            None,
            max(float(0.0), dot(world_normal.clone(), world_v.clone())),
        );
        let l1 = to_var(
            None,
            m.mul(vec4_join(vec![lobe.view_reflect_dir.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let brdf_env_color = self.map_level_0(equirect_uv(l1.clone()));
        let h1 = to_var(None, world_v.add(l1.clone()).normalize());
        let n_dot_l1 = to_var(None, max(float(0.0), dot(world_normal, l1)));
        let v_dot_h1 = to_var(None, max(float(0.0), dot(world_v, h1)));
        let w1 = brdf_weight(lobe, &n_dot_l1, &n_dot_v, &v_dot_h1);
        brdf_env_color.mul(w1).mul(self.intensity_node.clone())
    }

    /// `sampleEnvironmentMIS( { …lobe, Xi2 } )` — the environment for a
    /// screen-space miss, estimated with multiple importance sampling between
    /// the BRDF-sampled ray and a direction drawn from the luminance CDF with
    /// `xi2.zw`. The CDF sample is skipped for near-mirror lobes (`alpha ≤
    /// 0.01`). Needs `importance_sampling`.
    #[inline(never)]
    pub fn sample_environment_mis(&self, lobe: &EnvironmentLobe, xi2: NodeRef) -> NodeRef {
        let marginal = self.cdf.marginal_weights().expect(
            "three-rs: sample_environment_mis needs ImportanceSampledEnvironment::new( true ) \
             and an update_from",
        );
        let conditional = self
            .cdf
            .conditional_weights()
            .expect("the conditional table is built with the marginal one");
        let env_w = self.size_node.x();
        let env_h = self.size_node.y();
        let total_sum = self.total_sum_node.clone();
        let alpha = lobe.alpha.clone();
        let m = &lobe.camera_world_matrix;

        let world_normal = to_var(
            None,
            m.mul(vec4_join(vec![lobe.n.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let world_v = to_var(
            None,
            m.mul(vec4_join(vec![lobe.v.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let n_dot_v = to_var(
            None,
            max(float(0.0), dot(world_normal.clone(), world_v.clone())),
        );

        // MIS sample 1: the BRDF / reflected-ray direction.
        let l1 = to_var(
            None,
            m.mul(vec4_join(vec![lobe.view_reflect_dir.clone(), float(0.0)]))
                .xyz()
                .normalize(),
        );
        let brdf_env_color = self.map_level_0(equirect_uv(l1.clone()));
        let h1 = to_var(None, world_v.add(l1.clone()).normalize());
        let n_dot_l1 = to_var(None, max(float(0.0), dot(world_normal.clone(), l1.clone())));
        let n_dot_h1 = to_var(None, max(float(0.0), dot(world_normal.clone(), h1.clone())));
        let v_dot_h1 = to_var(None, max(float(0.0), dot(world_v.clone(), h1)));

        let pdf_brdf1 = d_gtr(alpha.clone(), n_dot_h1, float(2.0))
            .mul(smith_g(n_dot_v.clone(), alpha.clone()))
            .div(max(float(1e-6), float(4.0).mul(n_dot_v.clone())))
            .max(float(1e-8));
        let pdf_env1 = env_w
            .mul(env_h.clone())
            .mul(luminance(brdf_env_color.clone()).div(total_sum.clone()))
            .mul(equirect_dir_pdf(l1))
            .max(float(1e-8));
        let w1 = mis_power_heuristic(pdf_brdf1, pdf_env1);
        let weight1 = brdf_weight(lobe, &n_dot_l1, &n_dot_v, &v_dot_h1);
        let result = to_var(None, brdf_env_color.mul(weight1).mul(w1));

        // MIS sample 2: the env-luminance CDF direction.
        let r_env = vec2_join(vec![xi2.z(), xi2.w()]);
        let v_cdf = texture_with_uv(marginal, vec2_join(vec![r_env.x(), float(0.0)])).x();
        let u_cdf = texture_with_uv(conditional, vec2_join(vec![r_env.y(), v_cdf.clone()])).x();
        let is_env_uv = vec2_join(vec![u_cdf, v_cdf]);
        // Three passes the `vec2` UV to `equirectUV()`, which reads `.z` off
        // it, and then uses the `vec2` result as a direction; the port keeps
        // both, so the widening `vec3( v, 0.0 )`s are three's.
        let env_dir_ws = equirect_uv(is_env_uv.clone());
        let env_half = world_v.add(env_dir_ws.clone()).normalize();
        let env_n_dot_l = max(float(0.0), dot(world_normal.clone(), env_dir_ws.clone()));
        let env_n_dot_h = max(float(0.0), dot(world_normal, env_half.clone()));
        let env_v_dot_h = max(float(0.0), dot(world_v, env_half));

        let inner = self.mis_env_sample(
            lobe,
            &result,
            &n_dot_v,
            is_env_uv,
            env_dir_ws,
            env_n_dot_l.clone(),
            env_n_dot_h,
            env_v_dot_h,
        );
        let cdf_branch = if_then(
            alpha.greater_than(float(0.01)),
            vec![if_then(env_n_dot_l.greater_than(float(0.001)), inner)],
        );

        crate::nodes::tsl::block(
            vec![result.clone(), cdf_branch],
            result.mul(self.intensity_node.clone()),
        )
    }

    /// The body of `sampleEnvironmentMIS()`'s inner `If( envNdotL > 0.001 )`:
    /// the CDF sample's contribution, added to `result`.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    fn mis_env_sample(
        &self,
        lobe: &EnvironmentLobe,
        result: &NodeRef,
        n_dot_v: &NodeRef,
        is_env_uv: NodeRef,
        env_dir_ws: NodeRef,
        env_n_dot_l: NodeRef,
        env_n_dot_h: NodeRef,
        env_v_dot_h: NodeRef,
    ) -> Vec<NodeRef> {
        let alpha = lobe.alpha.clone();
        let env_w = self.size_node.x();
        let env_h = self.size_node.y();
        // The GGX D both the BRDF pdf and the specular BRDF use.
        let d = to_var(None, d_gtr(alpha.clone(), env_n_dot_h, float(2.0)));
        let sampled_color = self.map_level_0(is_env_uv);
        let pdf_env2 = env_w
            .mul(env_h)
            .mul(luminance(sampled_color.clone()).div(self.total_sum_node.clone()))
            .mul(equirect_dir_pdf(env_dir_ws))
            .max(float(1e-8));
        let pdf_brdf2 = d
            .mul(smith_g(n_dot_v.clone(), alpha.clone()))
            .div(max(float(1e-6), float(4.0).mul(n_dot_v.clone())))
            .max(float(1e-8));
        let w2 = mis_power_heuristic(pdf_env2.clone(), pdf_brdf2);
        let env_brdf_spec = d
            .mul(geometry_term(env_n_dot_l.clone(), n_dot_v.clone(), alpha))
            .div(max(
                float(1e-6),
                float(4.0).mul(env_n_dot_l.clone()).mul(n_dot_v.clone()),
            ));
        let env_fresnel_weight = f_schlick(lobe.f0.clone(), env_v_dot_h);
        vec![
            d.clone(),
            result.add_assign(
                sampled_color
                    .mul(env_brdf_spec)
                    .mul(env_fresnel_weight)
                    .mul(env_n_dot_l)
                    .div(pdf_env2)
                    .mul(w2),
            ),
        ]
    }
}

/// `F_Schlick( f0, VdotH1 ).mul( GeometryTerm( NdotL1, NdotV, alpha ) ).div(
/// SmithG( NdotV, alpha ).max( 1e-4 ) )` — the BRDF sample's Monte-Carlo
/// weight `F · G1( N·L )`, with GGX D cancelled analytically.
fn brdf_weight(
    lobe: &EnvironmentLobe,
    n_dot_l1: &NodeRef,
    n_dot_v: &NodeRef,
    v_dot_h1: &NodeRef,
) -> NodeRef {
    f_schlick(lobe.f0.clone(), v_dot_h1.clone())
        .mul(geometry_term(
            n_dot_l1.clone(),
            n_dot_v.clone(),
            lobe.alpha.clone(),
        ))
        .div(smith_g(n_dot_v.clone(), lobe.alpha.clone()).max(float(1e-4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 4×2 RGBA float environment from per-texel `( r, g, b )`.
    fn env(texels: &[[f32; 3]; 8]) -> Texture {
        let data: Vec<f32> = texels
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 1.0])
            .collect();
        Texture::data_rgba32float(4, 2, &data)
    }

    fn halves(texture: &Texture) -> Vec<f32> {
        let data = texture.borrow().data.clone().unwrap();
        bytemuck::pod_collect_to_vec::<u8, u16>(&data)
            .into_iter()
            .map(from_half_float)
            .collect()
    }

    // Luminance is 0.2126 r + 0.7152 g + 0.0722 b; a grey texel ( v, v, v )
    // weighs v (to within an ulp), and every value below is exact in half
    // precision, so the preprocessing changes nothing.
    //
    // Row 0: grey 1, 2, 3, 4      weights 1, 2, 3, 4      row sum 10
    //        conditional CDF      0.1, 0.3, 0.6, 1.0
    // Row 1: black, black, grey 5, pure green 4
    //                             weights 0, 0, 5, 0.7152·4 = 2.8608
    //                             row sum 7.8608
    //        conditional CDF      0, 0, 5/7.8608 = 0.63607, 1.0
    // Marginal: row sums 10, 7.8608, total 17.8608
    //        marginal CDF         10/17.8608 = 0.55988, 1.0
    //
    // Conditional table, row y, column x: the first column whose CDF is at
    // least ( x + 1 ) / 4, as ( col + 0.5 ) / 4:
    //   row 0: 0.25 → col 1 (0.3), 0.5 → col 2 (0.6), 0.75 → col 3, 1.0 → col 3
    //          = 0.375, 0.625, 0.875, 0.875
    //   row 1: 0.25 → col 2 (0.636), 0.5 → col 2, 0.75 → col 3, 1.0 → col 3
    //          = 0.625, 0.625, 0.875, 0.875
    // Marginal table, texel i: the first row whose CDF is at least
    // ( i + 1 ) / 2, as ( row + 0.5 ) / 2:
    //   0.5 → row 0 (0.55988), 1.0 → row 1  = 0.25, 0.75
    const TEXELS: [[f32; 3]; 8] = [
        [1.0, 1.0, 1.0],
        [2.0, 2.0, 2.0],
        [3.0, 3.0, 3.0],
        [4.0, 4.0, 4.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [5.0, 5.0, 5.0],
        [0.0, 4.0, 0.0],
    ];

    #[test]
    fn cdf_tables_match_three() {
        let mut generator = EnvMapCdfGenerator::new();
        generator.update_from(&env(&TEXELS));

        let marginal = generator.marginal_weights().unwrap();
        let conditional = generator.conditional_weights().unwrap();
        assert_eq!(halves(marginal), [0.25, 0.75]);
        assert_eq!(
            halves(conditional),
            [0.375, 0.625, 0.875, 0.875, 0.625, 0.625, 0.875, 0.875]
        );
        assert!((generator.total_sum() - 17.8608).abs() < 1e-9);

        // `new DataTexture( data, height, 1 )` and `( data, width, height )`,
        // `HalfFloatType` + `RedFormat`, linear, clamped, no mipmaps.
        assert_eq!(marginal.size(), (2, 1));
        assert_eq!(conditional.size(), (4, 2));
        for table in [marginal, conditional] {
            let inner = table.borrow();
            assert_eq!(inner.format, wgpu::TextureFormat::R16Float);
            assert_eq!(inner.min_filter, MinFilter::Linear);
            assert_eq!(inner.mag_filter, TextureFilter::Linear);
            assert_eq!(inner.wrap_s, Wrapping::ClampToEdge);
            assert_eq!(inner.wrap_t, Wrapping::ClampToEdge);
            assert!(!inner.generate_mipmaps && !inner.flip_y);
        }

        // The map: RGBA half floats, repeating in s, clamped in t.
        let map = generator.map().unwrap();
        let inner = map.borrow();
        assert_eq!(inner.format, wgpu::TextureFormat::Rgba16Float);
        assert_eq!(
            (inner.wrap_s, inner.wrap_t),
            (Wrapping::Repeat, Wrapping::ClampToEdge)
        );
        drop(inner);
        assert_eq!(
            &halves(map)[24..32],
            [5.0, 5.0, 5.0, 1.0, 0.0, 4.0, 0.0, 1.0]
        );
    }

    #[test]
    fn flip_y_reverses_the_rows_first() {
        // With `flipY` the rows swap before anything is summed: row 0 is now
        // the 7.8608 row and row 1 the 10 row, so the marginal CDF is
        // 7.8608 / 17.8608 = 0.44012, 1.0 and both quantiles land in row 1:
        // 0.75, 0.75. The conditional rows swap with them.
        let hdr = env(&TEXELS);
        hdr.set_flip_y(true);
        let mut generator = EnvMapCdfGenerator::new();
        generator.update_from(&hdr);
        assert_eq!(halves(generator.marginal_weights().unwrap()), [0.75, 0.75]);
        assert_eq!(
            halves(generator.conditional_weights().unwrap()),
            [0.625, 0.625, 0.875, 0.875, 0.375, 0.625, 0.875, 0.875]
        );
        assert!(!generator.map().unwrap().borrow().flip_y);
    }

    #[test]
    fn a_second_update_of_the_same_size_keeps_the_textures() {
        let mut generator = EnvMapCdfGenerator::new();
        generator.update_from(&env(&TEXELS));
        let ids = (
            generator.map().unwrap().id(),
            generator.marginal_weights().unwrap().id(),
        );
        let mut flipped = TEXELS;
        flipped.rotate_left(4);
        generator.update_from(&env(&flipped));
        assert_eq!(
            ids,
            (
                generator.map().unwrap().id(),
                generator.marginal_weights().unwrap().id()
            )
        );
        assert_eq!(halves(generator.marginal_weights().unwrap()), [0.75, 0.75]);
    }
}
