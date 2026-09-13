//! Ports of `three.js/src/lights` (`Light`, `AmbientLight`, `PointLight`).

mod ambient_light;
mod light;
mod point_light;

pub use ambient_light::AmbientLight;
pub use light::Light;
pub use point_light::PointLight;

/// Which `Light` subclass a scene-graph node is — three.js' `isAmbientLight` /
/// `isPointLight` flags, which `LightsNode.setupLightsNode()` switches on to
/// pick the lighting node for each light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightKind {
    Ambient,
    Point,
}

/// The `Light` subclass state held by [`crate::objects::Payload::Light`].
#[derive(Clone)]
pub enum LightPayload {
    Ambient(AmbientLight),
    Point(PointLight),
}

impl LightPayload {
    pub fn kind(&self) -> LightKind {
        match self {
            LightPayload::Ambient(_) => LightKind::Ambient,
            LightPayload::Point(_) => LightKind::Point,
        }
    }

    /// The `Light` base-class half: `color` and `intensity`.
    pub fn light(&self) -> &Light {
        match self {
            LightPayload::Ambient(light) => &light.light,
            LightPayload::Point(light) => &light.light,
        }
    }

    pub fn light_mut(&mut self) -> &mut Light {
        match self {
            LightPayload::Ambient(light) => &mut light.light,
            LightPayload::Point(light) => &mut light.light,
        }
    }

    /// The `PointLight` state (`distance`, `decay`), if this is one.
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
}
