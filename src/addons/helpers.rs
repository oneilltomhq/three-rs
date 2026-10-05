//! Port of `three.js/examples/jsm/helpers/LightProbeHelperGPU.js`.

use crate::core::ObjectRef;
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
/// helper. The position and scale half of `onBeforeRender()` is the
/// [`on_before_render`](crate::core::Object3D::on_before_render) the
/// constructor installs on [`node`](Self::node), so the helper follows the
/// probe by itself; the constructor also runs it once, as three's does.
pub struct LightProbeHelper {
    /// The `Mesh` itself.
    pub node: ObjectRef,
    /// `this.lightProbe`.
    pub light_probe: ObjectRef,
    /// `this.size` — the sphere's radius. The installed hook reads its own
    /// copy, which [`update`](Self::update) refreshes: after changing this,
    /// call `update()` once.
    pub size: f64,
    /// The installed hook's copy of `size`.
    hook_size: std::rc::Rc<std::cell::Cell<f64>>,
}

impl LightProbeHelper {
    /// `new LightProbeHelper( lightProbe, size = 1 )`.
    ///
    /// # Panics
    ///
    /// When the helper draws after `light_probe` was dropped, or if
    /// `light_probe` is not a light.
    pub fn new(light_probe: &ObjectRef, size: f64) -> Self {
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

        let probe = light_probe.downgrade();
        let hook_size = std::rc::Rc::new(std::cell::Cell::new(size));
        let shared_size = hook_size.clone();
        node.borrow_mut()
            .set_on_before_render(move |node, _renderer, _scene, _camera, _group| {
                if let Some(probe) = probe.upgrade() {
                    let position = probe.borrow().position;
                    let size = shared_size.get();
                    let mut object = node.borrow_mut();
                    object.position = position;
                    object.scale.set(size, size, size);
                }
            });

        let helper = Self {
            node,
            light_probe: light_probe.clone(),
            size,
            hook_size,
        };
        helper.update();
        helper
    }

    /// `onBeforeRender()`'s transform half, run by hand: the helper sits where
    /// the probe is, scaled to [`size`](Self::size). The renderer runs the
    /// same thing before every draw of the helper, so this is only needed to
    /// place it outside a render, or to apply a changed `size`.
    pub fn update(&self) {
        self.hook_size.set(self.size);
        let position = self.light_probe.borrow().position;
        let mut object = self.node.borrow_mut();
        object.position = position;
        object.scale.set(self.size, self.size, self.size);
    }
}
