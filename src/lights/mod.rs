//! Ports of `three.js/src/lights` (rung 5 subset: `Light` and `PointLight`).

mod hemisphere_light;
mod light;
mod point_light;
mod point_shadow;

pub use hemisphere_light::HemisphereLight;
pub use light::Light;
pub use point_light::PointLight;
pub use point_shadow::{point_shadow, PointLightShadow, CUBE_DIRECTIONS, CUBE_UPS};

/// Which `Light` subclass a light node is — the payload a
/// [`Payload::Light`](crate::objects::Payload) carries.
///
/// `LightsNode` keys its per-light node on the light's *type*, so the renderer
/// has to know which kind each entry of `RenderList.lights` is before it can
/// build the uniforms or the lighting flow.
#[derive(Clone)]
pub enum LightPayload {
    Point(PointLight),
    Hemisphere(HemisphereLight),
}

impl LightPayload {
    /// The `Light` half every subclass shares: colour and intensity.
    pub fn light(&self) -> &Light {
        match self {
            LightPayload::Point(light) => &light.light,
            LightPayload::Hemisphere(light) => &light.light,
        }
    }

    pub fn light_mut(&mut self) -> &mut Light {
        match self {
            LightPayload::Point(light) => &mut light.light,
            LightPayload::Hemisphere(light) => &mut light.light,
        }
    }

    pub fn point(&self) -> Option<&PointLight> {
        match self {
            LightPayload::Point(light) => Some(light),
            _ => None,
        }
    }

    pub fn point_mut(&mut self) -> Option<&mut PointLight> {
        match self {
            LightPayload::Point(light) => Some(light),
            _ => None,
        }
    }

    pub fn hemisphere(&self) -> Option<&HemisphereLight> {
        match self {
            LightPayload::Hemisphere(light) => Some(light),
            _ => None,
        }
    }

    pub fn hemisphere_mut(&mut self) -> Option<&mut HemisphereLight> {
        match self {
            LightPayload::Hemisphere(light) => Some(light),
            _ => None,
        }
    }
}
