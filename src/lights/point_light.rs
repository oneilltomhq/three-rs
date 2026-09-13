//! Port of `three.js/src/lights/PointLight.js` (no `PointLightShadow` — rung 7
//! adds shadows).

use super::Light;
use crate::core::{Node, Object3D};
use crate::math::{Color, Vector3};
use crate::objects::Payload;

/// `class PointLight extends Light`.
#[derive(Clone)]
pub struct PointLight {
    pub light: Light,
    /// `this.distance` — the cutoff distance; `0` means no cutoff. The shader
    /// calls it `cutoffDistance`.
    pub distance: f64,
    /// `this.decay`, default 2.
    pub decay: f64,
}

impl PointLight {
    /// `new PointLight( color, intensity, distance = 0, decay = 2 )`, as a
    /// scene-graph [`Node`]. `object.is_light` is what
    /// `Renderer._projectObject()` branches on, so the node is collected into
    /// `RenderList.lights` and never drawn — while anything added under it (the
    /// example's bulb sphere) is an ordinary child and draws through the walk.
    pub fn new(color: Color, intensity: f64, distance: f64) -> Node {
        let mut object = Object3D::default();
        object.object_type = "PointLight";
        object.is_light = true;
        object.payload = Payload::Light(super::LightPayload::Point(Self {
            light: Light::new(color, intensity),
            distance,
            decay: 2.0,
        }));
        object.into_node()
    }

    /// `get power()` — luminous power in lumens, `intensity * 4π` for an
    /// isotropic source.
    pub fn power(&self) -> f64 {
        self.light.intensity * 4.0 * std::f64::consts::PI
    }

    /// `set power( power )`.
    pub fn set_power(&mut self, power: f64) {
        self.light.intensity = power / (4.0 * std::f64::consts::PI);
    }

    /// The world-space position the lighting uniforms are built from
    /// (`Object3D.matrixWorld`'s translation, before the camera view matrix).
    /// The matrix lives on the node, so the caller passes it in.
    pub fn world_position(matrix_world: &crate::math::Matrix4) -> Vector3 {
        let mut v = Vector3::default();
        v.set_from_matrix_position(matrix_world);
        v
    }
}
