//! Port of `three.js/src/lights/HemisphereLight.js`.

use super::Light;
use crate::core::{Node, Object3D};
use crate::math::Color;
use crate::objects::Payload;

use super::LightPayload;

/// `class HemisphereLight extends Light`.
#[derive(Clone)]
pub struct HemisphereLight {
    pub light: Light,
    /// `this.groundColor`.
    pub ground_color: Color,
}

impl HemisphereLight {
    /// `new HemisphereLight( skyColor, groundColor, intensity = 1 )`.
    ///
    /// The constructor does one thing beyond `Light`'s: `this.position.copy(
    /// Object3D.DEFAULT_UP )`. That matters, because `HemisphereLightNode` takes
    /// its direction from `lightPosition( light ).normalize()` — at the origin
    /// the normalize would be undefined.
    pub fn new(sky_color: Color, ground_color: Color, intensity: f64) -> Node {
        let mut object = Object3D::default();
        object.object_type = "HemisphereLight";
        object.is_light = true;
        object.position.set(0.0, 1.0, 0.0);
        object.payload = Payload::Light(LightPayload::Hemisphere(Self {
            light: Light::new(sky_color, intensity),
            ground_color,
        }));
        object.into_node()
    }
}
