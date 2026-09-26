//! Port of `three.js/src/nodes/core/LightingModel.js` as a trait a user can
//! implement, and of the part of `LightsNode.setup()` that drives one.
//!
//! The port's built-in models (Phong, Lambert, Physical) stay the `match` on
//! [`MaterialKind`](super::MaterialKind) in `node_material.rs`, which is how
//! they were ported; this trait is the way *in* for a model the port does not
//! ship, as `webgpu_lights_custom` does:
//!
//! ```js
//! class CustomLightingModel extends THREE.LightingModel {
//!     direct( { lightColor, reflectedLight } ) {
//!         reflectedLight.directDiffuse.addAssign( lightColor );
//!     }
//! }
//! material.lightsNode = lights( [ … ] ).context( { lightingModel } );
//! ```
//!
//! three's methods append to the builder's current stack implicitly; here
//! they push onto [`LightingBuilder::stack`], the same `Vec<NodeRef>` of
//! statements the rest of the material setup writes into. See
//! `docs/nodes.md` §36.

use std::fmt::Debug;

use super::phong::{self, LightDesc};
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;

/// `LightingContextNode.getContext().reflectedLight` — the four accumulators,
/// each `vec3().toVar( name )`, so each is declared (and zeroed) where it is
/// first read rather than up front. That is why three's dump of
/// `webgpu_lights_custom` zeroes `directDiffuse` above the first light but
/// `indirectDiffuse` only in `totalDiffuse`'s line.
#[derive(Clone, Debug)]
pub struct ReflectedLight {
    pub direct_diffuse: NodeRef,
    pub direct_specular: NodeRef,
    pub indirect_diffuse: NodeRef,
    pub indirect_specular: NodeRef,
}

impl ReflectedLight {
    fn new() -> Self {
        let zero = || vec3(0.0, 0.0, 0.0);
        Self {
            direct_diffuse: zero().to_var("directDiffuse"),
            direct_specular: zero().to_var("directSpecular"),
            indirect_diffuse: zero().to_var("indirectDiffuse"),
            indirect_specular: zero().to_var("indirectSpecular"),
        }
    }
}

/// `LightingModel.direct()`'s argument: `AnalyticLightNode.setupDirect()`'s
/// `{ lightDirection, lightColor }`, spread together with the context's
/// `reflectedLight` by `LightsNode.setupDirectLight()`.
#[derive(Clone, Debug)]
pub struct DirectLightData {
    pub light_direction: NodeRef,
    /// The light's colour times its intensity, shadow and distance / cone
    /// attenuation.
    pub light_color: NodeRef,
    pub reflected_light: ReflectedLight,
}

/// The part of `NodeBuilder` a lighting model sees: the statement stack its
/// methods append to, the `reflectedLight` context and `builder.lightsNode`'s
/// list of lights.
#[derive(Debug)]
pub struct LightingBuilder<'a> {
    /// The statements this lighting pass appends, in order.
    pub stack: Vec<NodeRef>,
    pub reflected_light: ReflectedLight,
    lights: &'a [LightDesc],
    received_shadow_position: Option<&'a NodeRef>,
}

impl LightingBuilder<'_> {
    /// `stack.add( node )`.
    pub fn push(&mut self, node: NodeRef) {
        self.stack.push(node);
    }

    /// `builder.lightsNode.setupLights( builder, lightNodes )`: every light
    /// is built in `LightsNode` order, and each direct one ends in
    /// `lightingModel.direct()`. Ambient and hemisphere lights add to the
    /// `irradiance` property instead, as their `setup()`s do.
    pub fn setup_lights<M: LightingModel + ?Sized>(&mut self, model: &M) {
        for light in self.lights {
            let Some((light_direction, light_color)) =
                phong::setup_light(light, self.received_shadow_position, &mut self.stack)
            else {
                continue;
            };
            let data = DirectLightData {
                light_direction,
                light_color,
                reflected_light: self.reflected_light.clone(),
            };
            model.direct(&data, self);
        }
    }
}

/// `LightingModel` — the base class a custom model extends. Every method
/// defaults to three's base implementation: `start()` sets up the lights and
/// then calls `indirect()`, and the rest do nothing.
///
/// `directRectArea()` and `ambientOcclusion()` are not here: the port has no
/// `RectAreaLight`, and nothing calls `ambientOcclusion()` on a model the
/// port does not ship.
pub trait LightingModel: Debug {
    /// `start( builder )`.
    fn start(&self, builder: &mut LightingBuilder) {
        builder.setup_lights(self);
        self.indirect(builder);
    }

    /// `finish( builder )`.
    fn finish(&self, _builder: &mut LightingBuilder) {}

    /// `direct( lightData, builder )`.
    fn direct(&self, _data: &DirectLightData, _builder: &mut LightingBuilder) {}

    /// `indirect( builder )`.
    fn indirect(&self, _builder: &mut LightingBuilder) {}
}

/// `LightsNode.setup()` with a `lightingModel` in the context: `start()`,
/// the `totalDiffuse` / `totalSpecular` / `outgoingLight` tail, `finish()`.
/// Returns the `outgoingLight` property. The port has no backdrop on this
/// path, so `totalDiffuse` is always `directDiffuse + indirectDiffuse`.
pub(crate) fn lights_node(
    model: &dyn LightingModel,
    lights: &[LightDesc],
    received_shadow_position: Option<&NodeRef>,
    out: &mut Vec<NodeRef>,
) -> NodeRef {
    let mut builder = LightingBuilder {
        stack: Vec::new(),
        reflected_light: ReflectedLight::new(),
        lights,
        received_shadow_position,
    };

    model.start(&mut builder);

    let ReflectedLight {
        direct_diffuse,
        direct_specular,
        indirect_diffuse,
        indirect_specular,
    } = builder.reflected_light.clone();
    builder.push(total_diffuse().assign(direct_diffuse.add(indirect_diffuse)));
    builder.push(total_specular().assign(direct_specular.add(indirect_specular)));
    builder.push(outgoing_light().assign(total_diffuse().add(total_specular())));

    model.finish(&mut builder);

    out.extend(builder.stack);
    outgoing_light()
}
