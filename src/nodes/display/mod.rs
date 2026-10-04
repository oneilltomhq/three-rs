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
mod bilateral_blur;
mod bloom;
mod box_blur;
mod chromatic_aberration;
mod depth_aware_blend;
mod dot_screen;
mod fxaa;
mod gaussian_blur;
mod godrays;
mod gtao;
mod hash_blur;
mod lensflare;
mod motion_blur;
mod pixelation_pass;
mod radial_blur;
mod rgb_shift;
mod rtt;
mod sobel;
mod toon_outline_pass;
mod traa;
mod transition;
mod viewport_texture;

pub use after_image::{after_image, AfterImageNode};
pub use bilateral_blur::{bilateral_blur, BilateralBlurNode};
pub use bloom::{bloom, luminosity_high_pass, BloomNode, HighPassInput};
pub use box_blur::{box_blur, BoxBlurOptions};
pub use chromatic_aberration::chromatic_aberration;
pub use depth_aware_blend::{depth_aware_blend, DepthAwareBlendOptions};
pub use dot_screen::dot_screen;
pub use fxaa::{fxaa, FxaaNode};
pub use gaussian_blur::{gaussian_blur, GaussianBlurNode, GaussianBlurOptions};
pub use godrays::{godrays, GodraysNode};
pub use gtao::{ao, GtaoNode};
pub use hash_blur::{hash_blur, hash_blur_with, HashBlurOptions};
pub use lensflare::{lensflare, LensflareNode, LensflareParams};
pub use motion_blur::motion_blur;
pub use pixelation_pass::{pixelation_pass, PixelationPassNode};
pub use radial_blur::{radial_blur, RadialBlurOptions};
pub use rgb_shift::rgb_shift;
pub use rtt::{convert_to_texture, rtt, RttNode};
pub use sobel::{sobel, SobelOperatorNode};
pub use toon_outline_pass::{toon_outline_pass, ToonOutlinePassNode};
pub use traa::{traa, TraaNode};
pub use transition::transition;
pub use viewport_texture::{
    viewport_depth_texture, viewport_depth_texture_at, viewport_linear_depth, viewport_safe_uv,
    viewport_shared_texture, viewport_shared_texture_at, viewport_texture, viewport_texture_at,
};
