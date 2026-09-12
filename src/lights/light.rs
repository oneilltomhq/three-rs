//! Port of `three.js/src/lights/Light.js`.

use crate::math::Color;

/// `class Light extends Object3D`. In this port the `Object3D` half is the
/// scene-graph [`Node`](crate::core::Node) that carries the light as a
/// [`Payload`](crate::objects::Payload), so this struct is only what `Light`
/// adds to `Object3D` — the same shape `Mesh` has.
#[derive(Clone)]
pub struct Light {
    /// `this.color = new Color( color )`.
    pub color: Color,
    /// `this.intensity = intensity` (candela for the punctual lights).
    pub intensity: f64,
}

impl Light {
    /// `new Light( color, intensity = 1 )`.
    pub fn new(color: Color, intensity: f64) -> Self {
        Self { color, intensity }
    }
}

impl Default for Light {
    fn default() -> Self {
        // three.js' `new Color( undefined )` is white.
        Self::new(Color::default(), 1.0)
    }
}
