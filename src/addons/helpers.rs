//! Port of `three.js/examples/jsm/helpers/LightProbeHelperGPU.js`.

use crate::core::Node;
use crate::geometries::sphere_geometry;
use crate::materials::MeshBasicNodeMaterial;
use crate::nodes::tsl::{
    float, get_sh_irradiance_at, normal_world, uniform, uniform_array_live, vec4_join,
};
use crate::nodes::{Type, UniformGroup, UniformSource};
use crate::objects::Mesh;

/// `class LightProbeHelper extends Mesh` — a sphere shaded by nothing but a
/// [`LightProbe`](crate::lights::LightProbe)'s irradiance, divided by π:
/// what a white Lambertian ball lit by the probe alone would look like.
///
/// `sh` and `intensity` are live, as three's `onBeforeRender()` makes them —
/// the uniforms read the probe each draw — so changing the probe changes the
/// helper. The position and scale half of `onBeforeRender()` is
/// [`update`](Self::update), which the port has no per-object render hook to
/// call by itself; the constructor calls it once, as three's does.
pub struct LightProbeHelper {
    /// The `Mesh` itself.
    pub node: Node,
    /// `this.lightProbe`.
    pub light_probe: Node,
    /// `this.size` — the sphere's radius.
    pub size: f64,
}

impl LightProbeHelper {
    /// `new LightProbeHelper( lightProbe, size = 1 )`.
    ///
    /// # Panics
    ///
    /// When the helper draws after `light_probe` was dropped, or if
    /// `light_probe` is not a light.
    pub fn new(light_probe: &Node, size: f64) -> Self {
        let probe = light_probe.downgrade();
        let sh = uniform_array_live(9, move || {
            let probe = probe
                .upgrade()
                .expect("three-rs: a LightProbeHelper outlived its probe");
            let object = probe.borrow();
            let light = object
                .light()
                .expect("three-rs: LightProbeHelper wants a light");
            light
                .sh
                .coefficients
                .iter()
                .flat_map(|c| [c.x, c.y, c.z, 0.0])
                .collect()
        });
        let probe = light_probe.downgrade();
        let intensity = uniform(
            UniformSource::Live(crate::nodes::node::LiveValue::new(move || {
                let probe = probe
                    .upgrade()
                    .expect("three-rs: a LightProbeHelper outlived its probe");
                let object = probe.borrow();
                let light = object
                    .light()
                    .expect("three-rs: LightProbeHelper wants a light");
                vec![light.light.intensity]
            })),
            Type::F32,
            UniformGroup::Object,
            None,
        );

        let reciprocal_pi = float(std::f64::consts::FRAC_1_PI);

        let irradiance = get_sh_irradiance_at(normal_world(), &sh);
        let outgoing_light = reciprocal_pi.mul(irradiance).mul(intensity);

        let mut material = MeshBasicNodeMaterial::new();
        material.fragment_node = Some(vec4_join(vec![outgoing_light, float(1.0)]));

        let geometry = std::rc::Rc::new(sphere_geometry(1.0, 32, 16));
        let node = Mesh::new(geometry, material);
        node.borrow_mut().object_type = "LightProbeHelper";

        let helper = Self {
            node,
            light_probe: light_probe.clone(),
            size,
        };
        helper.update();
        helper
    }

    /// `onBeforeRender()`'s transform half: the helper sits where the probe
    /// is, scaled to `size`.
    pub fn update(&self) {
        let position = self.light_probe.borrow().position;
        let mut object = self.node.borrow_mut();
        object.position = position;
        object.scale.set(self.size, self.size, self.size);
    }
}
