//! Ports of `three.js/examples/jsm/tsl/display/` — the addon effect `Fn()`s a
//! `RenderPipeline`'s `outputNode` is built from.
//!
//! They live in the main crate rather than an `addons/` member because they are
//! node graphs, not scene-graph objects: an effect is a pure function of the
//! pass texture and a handful of uniforms, and it needs nothing from the
//! renderer that `src/nodes/tsl.rs` does not already export.
//!
//! [`bloom`] is the exception, and it is three.js' exception too: `BloomNode`
//! is a `TempNode` that owns eleven render targets and draws twelve quads of
//! its own in `updateBefore()`, so it reaches the renderer the way
//! [`SsaaPassNode`](crate::renderer::SsaaPassNode) does.

mod after_image;
mod anaglyph_pass;
mod bayer;
mod bilateral_blur;
mod bleach_bypass;
mod bloom;
mod box_blur;
mod chromatic_aberration;
mod crt;
mod depth_aware_blend;
mod depth_of_field;
mod dot_screen;
mod film;
mod fxaa;
mod gaussian_blur;
mod godrays;
mod gtao;
mod hash_blur;
mod importance_sampled_environment;
mod lensflare;
mod lut_3d;
mod motion_blur;
mod oit_pass;
mod outline;
mod parallax_barrier_pass;
mod pixelation_pass;
mod radial_blur;
mod recurrent_denoise;
mod retro_pass;
mod rgb_shift;
mod rtt;
mod sepia;
mod shape;
mod sharpen;
mod smaa;
mod sobel;
mod ssgi;
mod ssr;
mod sss;
mod stereo_composite_pass;
mod stereo_pass;
mod taa_utils;
mod taau;
mod temporal_reproject;
mod toon_outline_pass;
mod traa;
mod transition;
mod viewport_texture;

pub use after_image::{after_image, AfterImageNode};
pub use anaglyph_pass::{
    anaglyph_matrices, anaglyph_pass, AnaglyphAlgorithm, AnaglyphColorMode, AnaglyphPassNode,
};
pub use bayer::bayer_dither;
pub use bilateral_blur::{bilateral_blur, BilateralBlurNode};
pub use bleach_bypass::bleach;
pub use bloom::{bloom, luminosity_high_pass, BloomNode, HighPassInput};
pub use box_blur::{box_blur, BoxBlurOptions};
pub use chromatic_aberration::chromatic_aberration;
pub use crt::{barrel_mask, barrel_uv, color_bleeding, scanlines, vignette};
pub use depth_aware_blend::{depth_aware_blend, DepthAwareBlendOptions};
pub use depth_of_field::{dof, DepthOfFieldNode};
pub use dot_screen::dot_screen;
pub use film::film;
pub use fxaa::{fxaa, FxaaNode};
pub use gaussian_blur::{gaussian_blur, GaussianBlurNode, GaussianBlurOptions};
pub use godrays::{godrays, GodraysNode};
pub use gtao::{ao, GtaoNode};
pub use hash_blur::{hash_blur, hash_blur_with, HashBlurOptions};
pub use importance_sampled_environment::{
    EnvMapCdfGenerator, EnvironmentLobe, ImportanceSampledEnvironment,
};
pub use lensflare::{lensflare, LensflareNode, LensflareParams};
pub use lut_3d::{lut_3d, Lut3DNode};
pub use motion_blur::motion_blur;
pub use oit_pass::{is_oit_capable, oit_pass, OitPassNode, OitRenderObjects};
pub use outline::{outline, OutlineNode, OutlineParams, OutlineState};
pub use parallax_barrier_pass::{parallax_barrier_pass, ParallaxBarrierPassNode};
pub use pixelation_pass::{pixelation_pass, PixelationPassNode};
pub use radial_blur::{radial_blur, RadialBlurOptions};
pub use recurrent_denoise::{
    recurrent_denoise, DenoiseAlphaSource, DenoiseMode, RecurrentDenoiseNode,
    RecurrentDenoiseOptions,
};
pub use retro_pass::{retro_pass, RetroPassNode, RetroPassOptions};
pub use rgb_shift::rgb_shift;
pub use rtt::{convert_to_texture, rtt, RttNode};
pub use sepia::sepia;
pub use shape::circle;
pub use sharpen::{sharpen, SharpenNode};
pub use smaa::{smaa, SmaaNode};
pub use sobel::{sobel, SobelOperatorNode};
pub use ssgi::{ssgi, SsgiNode};
pub use ssr::{ssr, SampleFn, SsrNode, SsrOptions};
pub use sss::{sss, SssNode};
pub use stereo_pass::{stereo_pass, StereoPassNode};
pub use taau::{taau, TaauNode};
pub use temporal_reproject::{
    temporal_reproject, TemporalReprojectMode, TemporalReprojectNode, TemporalReprojectOptions,
};
pub use toon_outline_pass::{toon_outline_pass, ToonOutlinePassNode};
pub use traa::{traa, TraaNode};
pub use transition::transition;
pub use viewport_texture::{
    viewport_depth_texture, viewport_depth_texture_at, viewport_linear_depth, viewport_safe_uv,
    viewport_shared_texture, viewport_shared_texture_at, viewport_texture, viewport_texture_at,
};
