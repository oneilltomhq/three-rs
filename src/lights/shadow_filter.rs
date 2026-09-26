//! Ports of `three.js/src/nodes/lighting/ShadowFilterNode.js`
//! (`BasicShadowFilter`, `PCFShadowFilter`, `VSMShadowFilter`), the two VSM
//! blur passes of `ShadowNode.js` (`VSMPassVertical`, `VSMPassHorizontal`),
//! and `renderer.shadowMap.type`.
//!
//! The point-light filters (`BasicPointShadowFilter`, `PointShadowFilter`)
//! live beside `pointShadowFilter` in `point_shadow.rs`.

use std::rc::Rc;

use crate::nodes::tsl::{
    block, depth_texture_sample, float, frag_coord, if_then, int, interleaved_gradient_noise,
    loop_n, max, shadow_blur_samples, shadow_map_compare, shadow_map_size, shadow_radius, sqrt,
    step, texture_sample, to_var, vec2, vec2_join, vogel_disk_sample,
};
use crate::nodes::{NodeRef, Type};
use crate::textures::{CubeDepthTexture, DepthTexture, Texture};

/// `renderer.shadowMap.type` — `BasicShadowMap`, `PCFShadowMap`,
/// `PCFSoftShadowMap`, `VSMShadowMap`.
///
/// `PCFSoftShadowMap` is kept only so a page that asks for it ports
/// verbatim: `WebGPURenderer` has removed it, warns, and renders with
/// `PCFShadowMap` instead (`Renderer.render()` rewrites the type before the
/// frame), which is what [`ShadowMapType::resolved`] does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShadowMapType {
    Basic,
    #[default]
    Pcf,
    /// Deprecated in three.js' WebGPU renderer; resolves to [`Self::Pcf`].
    PcfSoft,
    Vsm,
}

impl ShadowMapType {
    /// The type the renderer actually uses: `PCFSoftShadowMap` is
    /// `PCFShadowMap`.
    pub fn resolved(self) -> Self {
        match self {
            ShadowMapType::PcfSoft => ShadowMapType::Pcf,
            other => other,
        }
    }
}

/// The texture a shadow filter reads — `inputs.depthTexture`, which is
/// whatever `ShadowNode.setupShadow()` hands the filter:
///
/// - the `ShadowDepthTexture` itself for `BasicShadowMap` / `PCFShadowMap`;
/// - the `VSMHorizontal` render target's RG moments for `VSMShadowMap`;
/// - the `CubeDepthTexture` for a point light (any type — VSM is not applied
///   to point lights).
#[derive(Clone, Debug)]
pub enum ShadowFilterMap {
    Depth(DepthTexture),
    Moments(Texture),
    Cube(CubeDepthTexture),
}

/// By identity, as a texture contributes its `uuid` to the cache key.
impl std::hash::Hash for ShadowFilterMap {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ShadowFilterMap::Depth(texture) => texture.id().hash(state),
            ShadowFilterMap::Moments(texture) => texture.id().hash(state),
            ShadowFilterMap::Cube(texture) => texture.id().hash(state),
        }
    }
}

/// The object three passes a shadow filter `Fn`:
/// `{ depthTexture, shadowCoord, shadow }` for a spot or directional light,
/// `{ depthTexture, bd3D, dp, shadow }` for a point light.
///
/// `shadow` is `index`: every `reference( …, shadow )` uniform the filter
/// reads is keyed on the light's index in the port ([`shadow_radius`],
/// [`shadow_map_size`], …).
pub struct ShadowFilterInputs {
    pub index: usize,
    pub map: ShadowFilterMap,
    /// `shadowCoord` (a `vec3`: uv and the biased depth) for a planar
    /// shadow; `bd3D`, the unit light-to-fragment direction, for a point
    /// light.
    pub shadow_coord: NodeRef,
    /// `dp`, the biased perspective depth a point filter compares against.
    /// `None` for a planar shadow.
    pub dp: Option<NodeRef>,
}

/// `shadow.filterNode` — a user shadow filter, used in place of the one
/// `renderer.shadowMap.type` picks (`ShadowNode.setupShadow()`'s
/// `shadow.filterNode || this.getShadowFilterFn( type )`).
///
/// Hashed by identity: the program depends on *which* function it is, the
/// way a JS `Fn` contributes its own node to the cache key.
#[derive(Clone)]
pub struct ShadowFilterFn(Rc<dyn Fn(&ShadowFilterInputs) -> NodeRef>);

impl ShadowFilterFn {
    pub fn new(filter: impl Fn(&ShadowFilterInputs) -> NodeRef + 'static) -> Self {
        Self(Rc::new(filter))
    }

    pub fn call(&self, inputs: &ShadowFilterInputs) -> NodeRef {
        (self.0)(inputs)
    }
}

impl std::fmt::Debug for ShadowFilterFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShadowFilterFn")
    }
}

impl std::hash::Hash for ShadowFilterFn {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as *const () as usize).hash(state);
    }
}

/// Which filter a shadow is read through: the renderer's type, or the
/// light's own `shadow.filterNode`.
#[derive(Clone, Debug, Hash)]
pub enum ShadowFilter {
    Basic,
    Pcf,
    Vsm,
    Custom(ShadowFilterFn),
}

impl ShadowFilter {
    /// `shadow.filterNode || getShadowFilterFn( type )`.
    pub fn of(shadow_type: ShadowMapType, filter_node: Option<&ShadowFilterFn>) -> Self {
        match (filter_node, shadow_type.resolved()) {
            (Some(filter), _) => ShadowFilter::Custom(filter.clone()),
            (None, ShadowMapType::Basic) => ShadowFilter::Basic,
            (None, ShadowMapType::Vsm) => ShadowFilter::Vsm,
            (None, _) => ShadowFilter::Pcf,
        }
    }

    /// Run the filter — `filterFn( inputs )`. For a point light, VSM falls
    /// back to `PointShadowFilter` (`PointShadowNode.getShadowFilterFn()`).
    pub fn apply(&self, inputs: &ShadowFilterInputs) -> NodeRef {
        match (self, &inputs.map) {
            (ShadowFilter::Custom(filter), _) => filter.call(inputs),
            (ShadowFilter::Basic, ShadowFilterMap::Cube(_)) => {
                crate::lights::point_shadow::basic_point_shadow_filter(inputs)
            }
            (_, ShadowFilterMap::Cube(_)) => {
                crate::lights::point_shadow::point_shadow_filter(inputs)
            }
            (ShadowFilter::Basic, _) => basic_shadow_filter(inputs),
            (ShadowFilter::Pcf, _) => pcf_shadow_filter(inputs),
            (ShadowFilter::Vsm, _) => vsm_shadow_filter(inputs),
        }
    }
}

fn depth_map(inputs: &ShadowFilterInputs, filter: &str) -> DepthTexture {
    match &inputs.map {
        ShadowFilterMap::Depth(map) => map.clone(),
        other => panic!("three-rs: {filter} reads a depth texture, got {other:?}"),
    }
}

/// `BasicShadowFilter` — one hardware comparison at the shadow coordinate,
/// on a `NearestFilter` comparison sampler: an unfiltered `[0, 1]` shadow.
///
/// `texture( depthTexture, shadowCoord.xy )` has an explicit uv, so no uv
/// matrix is involved.
pub fn basic_shadow_filter(inputs: &ShadowFilterInputs) -> NodeRef {
    let map = depth_map(inputs, "BasicShadowFilter");
    let coord = inputs.shadow_coord.clone();
    shadow_map_compare(&map, coord.clone().xy(), coord.z())
}

/// `PCFShadowFilter` — five Vogel-disk taps rotated by interleaved gradient
/// noise, each one a hardware comparison sample (so 20 effective taps).
pub fn pcf_shadow_filter(inputs: &ShadowFilterInputs) -> NodeRef {
    let map = depth_map(inputs, "PCFShadowFilter");
    let (index, coord) = (inputs.index, inputs.shadow_coord.clone());
    let texel_size = vec2(1.0, 1.0).div(shadow_map_size(index));
    let radius_scaled = shadow_radius(index).mul(texel_size.x());
    // `6.28318530718` mirrors three.js's `PCFShadowFilter` literal (an approximation
    // of `TAU`, not the exact constant); keeping the same literal keeps this
    // pixel-identical to three.js's output.
    #[allow(clippy::approx_constant)]
    let phi = interleaved_gradient_noise(frag_coord().xy()).mul(float(6.28318530718));

    let mut sum: Option<NodeRef> = None;
    for i in 0..5 {
        let offset = vogel_disk_sample(int(i), int(5), phi.clone()).mul(radius_scaled.clone());
        let tap = shadow_map_compare(&map, coord.clone().xy().add(offset), coord.clone().z());
        sum = Some(match sum {
            Some(acc) => acc.add(tap),
            None => tap,
        });
    }
    sum.expect("three-rs: the five-tap loop always sets sum")
        .mul(float(1.0 / 5.0))
}

/// `VSMShadowFilter` — Chebyshev's upper bound on the probability that the
/// fragment is lit, from the blurred depth mean and standard deviation the
/// two VSM passes left in the `VSMHorizontal` target, with the light-bleeding
/// remap `clamp( ( p - 0.3 ) / 0.65 )`.
///
/// `texture( depthTexture ).sample( shadowCoord.xy )` on an `RGFormat`
/// texture is a `vec2` node in three (`.rg`), so the tap is a `vec2` var read
/// twice; the port's sample is a `vec4`, and the `.xy` it narrows through is
/// the same swizzle three's dump carries.
pub fn vsm_shadow_filter(inputs: &ShadowFilterInputs) -> NodeRef {
    let moments = match &inputs.map {
        ShadowFilterMap::Moments(texture) => texture.clone(),
        other => panic!("three-rs: VSMShadowFilter reads the VSM moments, got {other:?}"),
    };
    let coord = inputs.shadow_coord.clone();

    // The moments target is `RGFormat`, so the tap is already a `vec2` node
    // and `.rg` collapses into it (`NodeUtils.getTextureType`).
    let distribution = texture_sample(&moments, coord.clone().xy()).xy();
    let mean = distribution.clone().x();
    let variance = max(
        float(0.0000001),
        distribution.clone().y().mul(distribution.y()),
    );

    // `renderer.reversedDepthBuffer` is off: `step( shadowCoord.z, mean )`.
    let hard_shadow = step(coord.clone().z(), mean.clone());

    let output = to_var(None, float(1.0));

    // Distance from mean; Chebyshev's inequality; the bleeding remap.
    let d = coord.z().sub(mean);
    let p_max = variance.clone().div(variance.add(d.clone().mul(d)));
    let p_max = p_max.sub(0.3).div(0.65).clamp(0.0, 1.0);

    block(
        vec![
            output.clone(),
            if_then(
                hard_shadow.clone().not_equal(float(1.0)),
                vec![output.assign(max(hard_shadow, p_max))],
            ),
        ],
        output,
    )
}

/// `VSMPassVertical` — the first VSM blur: `blurSamples` depth taps down a
/// column of the shadow map, `radius` texels apart at most, reduced to the
/// mean and standard deviation the second pass blurs again.
pub fn vsm_pass_vertical(index: usize, depth: &DepthTexture) -> NodeRef {
    vsm_pass(index, "meanVertical", "squareMeanVertical", |offset| {
        let uv = frag_coord()
            .xy()
            .add(vec2_join(vec![float(0.0), offset]).mul(shadow_radius(index)))
            .div(shadow_map_size(index));
        // `depth = depth.x` on a `float` node is the node itself.
        let depth = depth_texture_sample(depth, uv);
        (depth.clone(), depth.clone().mul(depth))
    })
}

/// `VSMPassHorizontal` — the second VSM blur: `blurSamples` taps of the
/// first pass's `( mean, stdDev )` along a row, recombined into a mean and a
/// standard deviation (`E[x²] = σ² + μ²`).
pub fn vsm_pass_horizontal(index: usize, vertical: &Texture) -> NodeRef {
    vsm_pass(index, "meanHorizontal", "squareMeanHorizontal", |offset| {
        let uv = frag_coord()
            .xy()
            .add(vec2_join(vec![offset, float(0.0)]).mul(shadow_radius(index)))
            .div(shadow_map_size(index));
        // An `RGFormat` texture is a `vec2` node in three
        // (`NodeUtils.getTextureType`), so the tap's own var is a `vec2`.
        let distribution = texture_sample(vertical, uv).xy();
        (
            distribution.clone().x(),
            distribution
                .clone()
                .y()
                .mul(distribution.clone().y())
                .add(distribution.clone().x().mul(distribution.x())),
        )
    })
}

/// The shared body of the two passes: the running sums, the `Loop` over
/// `blurSamples` and the final `vec2( mean, stdDev )`. `tap( uvOffset )`
/// gives the loop body's two addends.
fn vsm_pass(
    index: usize,
    mean_name: &'static str,
    square_mean_name: &'static str,
    tap: impl Fn(NodeRef) -> (NodeRef, NodeRef),
) -> NodeRef {
    let samples = shadow_blur_samples(index);
    let mean = to_var(Some(mean_name), float(0.0));
    let squared_mean = to_var(Some(square_mean_name), float(0.0));

    let uv_stride = samples
        .clone()
        .less_than_equal(float(1.0))
        .select(float(0.0), float(2.0).div(samples.clone().sub(1.0)));
    let uv_start = samples
        .clone()
        .less_than_equal(float(1.0))
        .select(float(0.0), float(-1.0));

    let body = {
        let (mean, squared_mean) = (mean.clone(), squared_mean.clone());
        move |i: &NodeRef| {
            let uv_offset = uv_start.add(i.clone().to(Type::F32).mul(uv_stride));
            let (value, square) = tap(uv_offset);
            vec![mean.add_assign(value), squared_mean.add_assign(square)]
        }
    };

    let std_dev = sqrt(max(
        squared_mean.clone().sub(mean.clone().mul(mean.clone())),
        float(0.0),
    ));

    block(
        vec![
            mean.clone(),
            squared_mean.clone(),
            loop_n("i", samples.clone().to(Type::I32), body),
            mean.div_assign(samples.clone()),
            squared_mean.div_assign(samples),
        ],
        vec2_join(vec![mean, std_dev]),
    )
}
