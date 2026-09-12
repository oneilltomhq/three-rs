//! Port of `three.js/src/lights/PointLight.js` (no `PointLightShadow` — rung 7
//! adds shadows).

use super::Light;
use crate::core::Object3D;
use crate::math::{Color, Vector3};
use crate::objects::Child;

/// `class PointLight extends Light`.
pub struct PointLight {
    pub light: Light,
    /// `this.distance` — the cutoff distance; `0` means no cutoff. The shader
    /// calls it `cutoffDistance`.
    pub distance: f64,
    /// `this.decay`, default 2.
    pub decay: f64,
    /// `light.add( mesh )`: the example hangs a small sphere off each light, so
    /// a light is the first object in this port with children of its own.
    pub children: Vec<Child>,
}

impl PointLight {
    /// `new PointLight( color, intensity, distance = 0, decay = 2 )`.
    pub fn new(color: Color, intensity: f64, distance: f64) -> Self {
        let mut light = Light::new(color, intensity);
        light.object.object_type = "PointLight";
        Self {
            light,
            distance,
            decay: 2.0,
            children: Vec::new(),
        }
    }

    pub fn object(&self) -> &Object3D {
        &self.light.object
    }

    pub fn object_mut(&mut self) -> &mut Object3D {
        &mut self.light.object
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

    /// `light.add( mesh )`.
    pub fn add(&mut self, child: impl Into<Child>) {
        self.children.push(child.into());
    }

    /// The world-space position the lighting uniforms are built from
    /// (`Object3D.matrixWorld`'s translation, before the camera view matrix).
    pub fn world_position(&self) -> Vector3 {
        let mut v = Vector3::default();
        v.set_from_matrix_position(&self.light.object.matrix_world);
        v
    }
}
