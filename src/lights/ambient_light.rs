//! Port of `three.js/src/lights/AmbientLight.js`.

use super::{Light, LightPayload};
use crate::core::{Node, Object3D};
use crate::math::Color;
use crate::objects::Payload;

/// `class AmbientLight extends Light`. It adds nothing to `Light` but the
/// `isAmbientLight` flag, which is what `LightsNode` uses to pick
/// `AmbientLightNode` — here, the [`LightPayload`] variant.
#[derive(Clone)]
pub struct AmbientLight {
    pub light: Light,
}

impl AmbientLight {
    /// `new AmbientLight( color, intensity = 1 )`, as a scene-graph [`Node`].
    pub fn new(color: Color, intensity: f64) -> Node {
        let mut object = Object3D::default();
        object.object_type = "AmbientLight";
        object.is_light = true;
        object.payload = Payload::Light(LightPayload::Ambient(Self {
            light: Light::new(color, intensity),
        }));
        object.into_node()
    }
}
