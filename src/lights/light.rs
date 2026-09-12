//! Port of `three.js/src/lights/Light.js`.

use crate::core::Object3D;
use crate::math::Color;

/// `class Light extends Object3D`. The subclasses here keep a `Light` by
/// composition, the same shape `Mesh`/`Scene` use for `Object3D`.
pub struct Light {
    pub object: Object3D,
    /// `this.color = new Color( color )`.
    pub color: Color,
    /// `this.intensity = intensity` (candela for the punctual lights).
    pub intensity: f64,
}

impl Light {
    /// `new Light( color, intensity = 1 )`.
    pub fn new(color: Color, intensity: f64) -> Self {
        let mut object = Object3D::default();
        object.object_type = "Light";
        Self {
            object,
            color,
            intensity,
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        // three.js' `new Color( undefined )` is white.
        Self::new(Color::default(), 1.0)
    }
}
