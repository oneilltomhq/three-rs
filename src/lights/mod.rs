//! Ports of `three.js/src/lights`: `Light`, the five light types this port
//! uses, the `LightShadow` family and `PointShadowNode`.

mod light;
mod light_object;
mod light_shadow;
pub(crate) mod point_shadow;
mod shadow_filter;

pub use light::Light;
pub use light_object::{
    AmbientLight, DirectionalLight, HemisphereLight, LightKind, LightObject, PointLight, SpotLight,
};
pub use light_shadow::{LightShadow, ShadowCamera};
pub use point_shadow::{
    basic_point_shadow_filter, point_shadow, point_shadow_filter, point_shadow_filtered,
    CUBE_DIRECTIONS, CUBE_UPS,
};
pub use shadow_filter::{
    basic_shadow_filter, pcf_shadow_filter, vsm_pass_horizontal, vsm_pass_vertical,
    vsm_shadow_filter, ShadowFilter, ShadowFilterFn, ShadowFilterInputs, ShadowFilterMap,
    ShadowMapType,
};
