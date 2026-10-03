//! The `Payload::Light` half of every light: what `AmbientLight`,
//! `PointLight`, `SpotLight`, `DirectionalLight`, `HemisphereLight` and
//! `LightProbe` add to `Object3D`.
//!
//! One struct covers all six because the renderer's light list is uniform —
//! `LightsNode.setupLights()` switches on the light's type, which `kind`
//! records.

use super::{Light, LightShadow};
use crate::core::{Node, Object3D};
use crate::math::{Color, Matrix4, SphericalHarmonics3, Vector3};
use crate::objects::Payload;

/// Which `Light` subclass this is. `LightsNode` sorts and sets up lights by
/// type, and the node graph differs per type.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum LightKind {
    /// `AmbientLight`.
    Ambient,
    /// `PointLight`.
    Point,
    /// `SpotLight`.
    Spot,
    /// `DirectionalLight`.
    Directional,
    /// `HemisphereLight`.
    Hemisphere,
    /// `LightProbe` — irradiance from nine spherical-harmonic coefficients,
    /// added to `irradiance` like an ambient light and never a direct term.
    Probe,
}

/// `class <X>Light extends Light extends Object3D`, minus the `Object3D` half
/// (which is the scene-graph [`Node`] carrying this as a [`Payload`]).
pub struct LightObject {
    /// The `color`/`intensity` state common to every light kind.
    pub light: Light,
    /// Which `Light` subclass this is.
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
    /// `LightProbe.sh` — the probe's irradiance as order-2 spherical
    /// harmonics. All zero, and unread, on every other kind.
    pub sh: SphericalHarmonics3,
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
            sh: self.sh,
            target: self.target.as_ref().map(|t| t.borrow().clone().into_node()),
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
            sh: SphericalHarmonics3::default(),
            target: None,
            cast_shadow: false,
            shadow: None,
        }
    }

    /// `light.color.clone().multiplyScalar( light.intensity )` —
    /// `AnalyticLightNode.update()`'s single `vec3` colour uniform.
    pub(crate) fn color_intensity(&self) -> Color {
        let c = self.light.color;
        let i = self.light.intensity;
        Color::new(c.r * i, c.g * i, c.b * i)
    }

    /// `LightProbeNode.update()`: each coefficient times the intensity,
    /// padded to the `vec4` a `uniformArray()` element occupies.
    pub(crate) fn sh_intensity(&self) -> [[f32; 4]; 9] {
        let i = self.light.intensity;
        self.sh
            .coefficients
            .map(|c| [(c.x * i) as f32, (c.y * i) as f32, (c.z * i) as f32, 0.0])
    }

    /// `cos( light.angle )` — `SpotLightNode.update()`.
    pub(crate) fn cone_cos(&self) -> f64 {
        self.angle.cos()
    }

    /// `cos( light.angle * ( 1 - light.penumbra ) )`.
    pub(crate) fn penumbra_cos(&self) -> f64 {
        (self.angle * (1.0 - self.penumbra)).cos()
    }

    /// The world-space position the lighting uniforms are built from
    /// (`Object3D.matrixWorld`'s translation). The matrix lives on the node, so
    /// the caller passes it in.
    pub(crate) fn world_position(matrix_world: &Matrix4) -> Vector3 {
        let mut v = Vector3::default();
        v.set_from_matrix_position(matrix_world);
        v
    }

    /// `HemisphereLightNode.update()`: the ground colour carries the intensity
    /// just as the sky colour does.
    pub(crate) fn ground_color_intensity(&self) -> Color {
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
    pub(crate) fn target_world_position(&self) -> Vector3 {
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
    let object = Object3D {
        object_type,
        is_light: true,
        payload: Payload::Light(light),
        ..Default::default()
    };
    object.into_node()
}

/// `class AmbientLight extends Light` — `new AmbientLight( color, intensity )`.
pub struct AmbientLight;

impl AmbientLight {
    /// `new AmbientLight( color, intensity )`, as a scene-graph [`Node`]. An
    /// ambient light adds no shadow and no direction: `LightsNode` reads only
    /// its colour and intensity.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
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
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
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
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
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
    ///
    /// Like `HemisphereLight`, the constructor sets `this.position.copy(
    /// Object3D.DEFAULT_UP )`. A page that never moves the light sees it:
    /// `webgpu_backdrop` hangs its spot light on the camera, so the light
    /// sits one unit above the eye and the highlights sit high on the spheres.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(color: Color, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Spot, color, intensity);
        light.target = Some(Object3D::new_node());
        light.shadow = Some(Box::new(LightShadow::spot()));
        let node = into_node("SpotLight", light);
        node.borrow_mut().position.set(0.0, 1.0, 0.0);
        node
    }
}

/// `class DirectionalLight extends Light`.
pub struct DirectionalLight;

impl DirectionalLight {
    /// `new DirectionalLight( color, intensity )` — a target at the origin,
    /// a `DirectionalLightShadow`, and `this.position.copy(
    /// Object3D.DEFAULT_UP )`, so an unmoved light shines straight down.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(color: Color, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Directional, color, intensity);
        light.target = Some(Object3D::new_node());
        light.shadow = Some(Box::new(LightShadow::directional()));
        let node = into_node("DirectionalLight", light);
        node.borrow_mut().position.set(0.0, 1.0, 0.0);
        node
    }
}

/// `class LightProbe extends Light`.
///
/// A probe holds the irradiance of an environment as nine spherical-harmonic
/// coefficients (see [`SphericalHarmonics3`]) and lights a material by
/// `LightProbeNode`: `irradiance += getShIrradianceAt( normalWorld, sh *
/// intensity )`. It has no direction, no shadow, and its position is not
/// read by the lighting — only `LightProbeHelper` honours it.
pub struct LightProbe;

impl LightProbe {
    /// `new LightProbe( sh = new SphericalHarmonics3(), intensity = 1 )`. The
    /// colour is `Light`'s default, white; `LightProbeNode` never reads it.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(sh: SphericalHarmonics3, intensity: f64) -> Node {
        let mut light = LightObject::base(LightKind::Probe, Color::default(), intensity);
        light.sh = sh;
        into_node("LightProbe", light)
    }

    /// `lightProbe.copy( source )`'s `Light` half: the colour, the intensity
    /// and `sh.copy( source.sh )`.
    ///
    /// Three's `copy()` also runs `Object3D.copy()`, which the port has no
    /// general form of; `webgpu_lightprobe` copies a probe straight from
    /// `LightProbeGenerator`, which sits at the origin untransformed, and then
    /// sets the position itself, so the transform is left as it is here.
    ///
    /// # Panics
    ///
    /// If either node is not a light.
    pub fn copy(target: &Node, source: &Node) {
        let (light, sh) = {
            let source = source.borrow();
            let source = source
                .light()
                .expect("three-rs: LightProbe::copy reads a light");
            (source.light.clone(), source.sh)
        };
        let mut target = target.borrow_mut();
        let target = target
            .light_mut()
            .expect("three-rs: LightProbe::copy writes a light");
        target.light = light;
        target.sh = sh;
    }
}
