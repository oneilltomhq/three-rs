//! The `Payload::Light` half of every light: what `AmbientLight`,
//! `PointLight`, `SpotLight`, `DirectionalLight` and `HemisphereLight` add to
//! `Object3D`.
//!
//! One struct covers all five because the renderer's light list is uniform —
//! `LightsNode.setupLights()` switches on the light's type, which `kind`
//! records.

use super::{Light, LightShadow};
use crate::core::{Node, Object3D};
use crate::math::{Color, Matrix4, Vector3};
use crate::objects::Payload;

/// Which `Light` subclass this is. `LightsNode` sorts and sets up lights by
/// type, and the node graph differs per type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightKind {
    Ambient,
    Point,
    Spot,
    Directional,
    Hemisphere,
}

/// `class <X>Light extends Light extends Object3D`, minus the `Object3D` half
/// (which is the scene-graph [`Node`] carrying this as a [`Payload`]).
pub struct LightObject {
    pub light: Light,
    pub kind: LightKind,
    /// `PointLight.distance` / `SpotLight.distance` — the cutoff distance, `0`
    /// meaning no cutoff. The shader calls it `cutoffDistance`.
    pub distance: f64,
    /// `PointLight.decay` / `SpotLight.decay`, default 2.
    pub decay: f64,
    /// `SpotLight.angle`, in radians.
    pub angle: f64,
    /// `SpotLight.penumbra`.
    pub penumbra: f64,
    /// `HemisphereLight.groundColor`.
    pub ground_color: Color,
    /// `SpotLight.target` / `DirectionalLight.target` — an `Object3D` at the
    /// origin by default, never added to the scene, so its `matrixWorld` is
    /// just its local matrix.
    pub target: Option<Node>,
    /// `Object3D.castShadow`.
    pub cast_shadow: bool,
    /// `light.shadow` — present for the shadow-casting light types. Boxed
    /// because the shadow owns a camera, which owns an `Object3D`, which can
    /// own a light.
    pub shadow: Option<Box<LightShadow>>,
}

/// `SpotLight.copy()` / `DirectionalLight.copy()` clone the target rather
/// than sharing it, because the target is a real `Object3D` with its own
/// transform.
impl Clone for LightObject {
    fn clone(&self) -> Self {
        Self {
            light: self.light.clone(),
            kind: self.kind,
            distance: self.distance,
            decay: self.decay,
            angle: self.angle,
            penumbra: self.penumbra,
            ground_color: self.ground_color,
            target: self
                .target
                .as_ref()
                .map(|t| t.borrow().clone().into_node()),
            cast_shadow: self.cast_shadow,
            shadow: self.shadow.clone(),
        }
    }
}

impl LightObject {
    fn base(kind: LightKind, color: Color, intensity: f64) -> Self {
        Self {
            light: Light::new(color, intensity),
            kind,
            distance: 0.0,
            decay: 2.0,
            angle: std::f64::consts::FRAC_PI_3,
            penumbra: 0.0,
            ground_color: Color::new(0.0, 0.0, 0.0),
            target: None,
            cast_shadow: false,
            shadow: None,
        }
    }

    /// `light.color.clone().multiplyScalar( light.intensity )` —
    /// `AnalyticLightNode.update()`'s single `vec3` colour uniform.
    pub fn color_intensity(&self) -> Color {
        let c = self.light.color;
        let i = self.light.intensity;
        Color::new(c.r * i, c.g * i, c.b * i)
    }

    /// `cos( light.angle )` — `SpotLightNode.update()`.
    pub fn cone_cos(&self) -> f64 {
        self.angle.cos()
    }

    /// `cos( light.angle * ( 1 - light.penumbra ) )`.
    pub fn penumbra_cos(&self) -> f64 {
        (self.angle * (1.0 - self.penumbra)).cos()
    }

    /// The world-space position the lighting uniforms are built from
    /// (`Object3D.matrixWorld`'s translation). The matrix lives on the node, so
    /// the caller passes it in.
    pub fn world_position(matrix_world: &Matrix4) -> Vector3 {
        let mut v = Vector3::default();
        v.set_from_matrix_position(matrix_world);
        v
    }

    /// `HemisphereLightNode.update()`: the ground colour carries the intensity
    /// just as the sky colour does.
    pub fn ground_color_intensity(&self) -> Color {
        let c = self.ground_color;
        let i = self.light.intensity;
        Color::new(c.r * i, c.g * i, c.b * i)
    }

    /// `set power( power )` — `PointLight` only.
    pub fn set_power(&mut self, power: f64) {
        self.light.intensity = power / (4.0 * std::f64::consts::PI);
    }

    /// `get power()` — luminous power in lumens, `intensity * 4π`.
    pub fn power(&self) -> f64 {
        self.light.intensity * 4.0 * std::f64::consts::PI
    }

    /// `light.target.matrixWorld`'s translation, or the origin when the light
    /// has no target.
    pub fn target_world_position(&self) -> Vector3 {
        match &self.target {
            Some(target) => {
                let m = target.borrow().matrix_world;
                Self::world_position(&m)
            }
            None => Vector3::new(0.0, 0.0, 0.0),
        }
    }
}

fn into_node(object_type: &'static str, light: LightObject) -> Node {
    let mut object = Object3D::default();
    object.object_type = object_type;
    object.is_light = true;
    object.payload = Payload::Light(light);
    object.into_node()
}

/// `class AmbientLight extends Light` — `new AmbientLight( color, intensity )`.
pub struct AmbientLight;

impl AmbientLight {
    pub fn new(color: Color, intensity: f64) -> Node {
        into_node(
            "AmbientLight",
            LightObject::base(LightKind::Ambient, color, intensity),
        )
    }
}

/// `class PointLight extends Light`.
pub struct PointLight;

impl PointLight {
    /// `new PointLight( color, intensity, distance = 0, decay = 2 )`, as a
    /// scene-graph [`Node`]. `object.is_light` is what
    /// `Renderer._projectObject()` branches on, so the node is collected into
    /// `RenderList.lights` and never drawn — while anything added under it (the
    /// bulb sphere of `webgpu_lights_phong`) is an ordinary child and draws
    /// through the walk.
    ///
    /// `this.shadow = new PointLightShadow()` — present whether or not the
    /// light casts; `Object3D.castShadow` is the switch.
    pub fn new(color: Color, intensity: f64, distance: f64) -> Node {
        let mut light = LightObject::base(LightKind::Point, color, intensity);
        light.distance = distance;
        light.shadow = Some(Box::new(LightShadow::point()));
        into_node("PointLight", light)
    }
}

/// `class HemisphereLight extends Light`.
pub struct HemisphereLight;

impl HemisphereLight {
    /// `new HemisphereLight( skyColor, groundColor, intensity = 1 )`.
    ///
    /// The constructor does one thing beyond `Light`'s: `this.position.copy(
    /// Object3D.DEFAULT_UP )`. That matters, because `HemisphereLightNode` takes
    /// its direction from `lightPosition( light ).normalize()` — at the origin
    /// the normalize would be undefined.
    pub fn new(sky_color: Color, ground_color: Color, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Hemisphere, sky_color, intensity);
        light.ground_color = ground_color;
        let node = into_node("HemisphereLight", light);
        node.borrow_mut().position.set(0.0, 1.0, 0.0);
        node
    }
}

/// `class SpotLight extends Light`.
pub struct SpotLight;

impl SpotLight {
    /// `new SpotLight( color, intensity )` — `distance 0`, `angle π/3`,
    /// `penumbra 0`, `decay 2`, a target at the origin and a
    /// `SpotLightShadow`.
    pub fn new(color: Color, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Spot, color, intensity);
        light.target = Some(Object3D::new_node());
        light.shadow = Some(Box::new(LightShadow::spot()));
        into_node("SpotLight", light)
    }
}

/// `class DirectionalLight extends Light`.
pub struct DirectionalLight;

impl DirectionalLight {
    /// `new DirectionalLight( color, intensity )` — a target at the origin and
    /// a `DirectionalLightShadow`.
    pub fn new(color: Color, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Directional, color, intensity);
        light.target = Some(Object3D::new_node());
        light.shadow = Some(Box::new(LightShadow::directional()));
        into_node("DirectionalLight", light)
    }
}
