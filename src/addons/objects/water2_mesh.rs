//! Port of `three.js/examples/jsm/objects/Water2Mesh.js` — the advanced
//! water: two normal maps scrolled along a flow (a direction, or a flow map),
//! cross-faded on a half-cycle so neither one ever visibly resets, and a
//! Fresnel mix between a refraction of what is behind the surface and a
//! planar reflection.
//!
//! References:
//!
//! - [Water Flow in Portal 2](https://alex.vlachos.com/graphics/Vlachos-SIGGRAPH10-WaterFlow.pdf)
//! - [Water using flow maps](http://graphicsrunner.blogspot.de/2010/08/water-using-flow-maps.html)
//!
//! Upstream's module defines a `Mesh` subclass it exports as `WaterMesh` and
//! a private `WaterNode` (a `Node` of type `vec4`) that is the material's
//! `colorNode`. The crate already has a `WaterMesh`, the port of
//! `WaterMesh.js`, so this one is [`Water2Mesh`], after its file, as three's
//! own docs call the module.
//!
//! Upstream reaches the uniforms through the node,
//! `water.material.colorNode.scale.value = … `; the port's [`Mesh`] is a
//! constructor returning a scene-graph [`Node`], so [`Water2Mesh`] holds that
//! node next to the `WaterNode`'s uniforms, each a [`SettableValue`], and
//! `waterNode.scale.value = 2` is `water.scale.set( vec![ 2.0 ] )`.
//!
//! The `WaterNode` is a [`CustomNode`]: `updateBeforeType = RENDER`, and its
//! `updateBefore( frame )` is `updateFlow( frame.deltaTime )`, which
//! [`Water2Mesh::update_flow`] also exposes. Its `setup()` is a `Fn()` called
//! on the spot, which the port writes out node for node.
//!
//! Refraction is not a second render: it is `viewportSharedTexture(
//! viewportSafeUV( screenUV + offset ) )`, the pass drawn so far, which is why
//! upstream sets `renderOrder = Infinity` on the water in its page.
//!
//! # Divergences
//!
//! * **The graph, and so the reflector, is made at construction.** Upstream's
//!   `WaterNode.setup()` builds the graph — including `reflector()` and
//!   `this.waterBody.add( reflectionSampler.target )` — every time the
//!   material is built. The port builds it once in [`Water2Mesh::new`] and
//!   its `setup` hands back that graph, as for `WaterMesh` (§76). The add
//!   keeps its timing through [`ReflectorNode::add_target_on_setup`], so the
//!   first frame mirrors about the target's identity matrixWorld (the plane
//!   `z = 0`, facing +Z) as three's does. A material rebuild in three makes a
//!   new reflector and adds a second target; the port keeps its one.
//! * **The maps are [`Texture`]s.** Upstream wraps each in a `texture()` node
//!   whose `.value` can be swapped; the port stores the textures, so their
//!   images can change but the textures cannot be replaced.
//! * **The normal maps are required.** Upstream documents both as required
//!   but would build `texture( undefined )` without them;
//!   [`Water2MeshOptions::new`] takes them.
//! * **`color` is linear RGB.** Upstream's option takes a number, a
//!   [`Color`] or a CSS string; the port's is a [`Color`], which
//!   [`Color::from_hex`] makes from the page's `'#99e0ff'` as three's
//!   `new Color( '#99e0ff' )` does.
//! * **No `isWater` flag.** Not ported, as `WaterMesh`'s `isWaterMesh` is
//!   not.
//! * **`NodeMaterial`.** A bare `NodeMaterial` is unlit with no `colorNode`
//!   of its own, which the port's [`MeshBasicNodeMaterial`] is with its
//!   defaults, as for `WaterMesh` and `SkyMesh`.

use crate::core::{BufferGeometry, Node};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Vector2};
use crate::nodes::display::{viewport_safe_uv, viewport_shared_texture_at};
use crate::nodes::node::{CustomNode, SettableValue};
use crate::nodes::reflector_node::{reflector, ReflectorNode, ReflectorParameters};
use crate::nodes::tsl::{
    abs, block, camera_position, custom, dot, float, max, mix, position_world, screen_uv, texture,
    texture_with_uv, to_const, to_var, to_var_intent, uniform_settable, uv, vec2_join, vec3_join,
    vec4_join,
};
use crate::nodes::{NodeBuilder, NodeRef, NodeUpdateType, Type};
use crate::objects::Mesh;
use crate::renderer::Renderer;
use crate::textures::Texture;

use std::rc::Rc;

/// `this._cycle` — a cycle of a flow map phase.
const CYCLE: f64 = 0.15;
/// `this._halfCycle`.
const HALF_CYCLE: f64 = CYCLE * 0.5;

/// `Water2Mesh~Options` — the constructor's options, with upstream's
/// defaults.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Water2MeshOptions {
    /// `color` — the water colour. Default `0xffffff`.
    pub color: Color,
    /// `flowDirection` — default `( 1, 0 )`. Used only without a
    /// [`flow_map`](Self::flow_map).
    pub flow_direction: Vector2,
    /// `flowSpeed` — default `0.03`.
    pub flow_speed: f64,
    /// `reflectivity` — default `0.02`.
    pub reflectivity: f64,
    /// `scale` — how many times the normal maps repeat across the uv.
    /// Default `1`.
    pub scale: f64,
    /// `flowMap` — default none, in which case the flow is
    /// [`flow_direction`](Self::flow_direction).
    pub flow_map: Option<Texture>,
    /// `normalMap0` — the first water normal map.
    pub normal_map0: Texture,
    /// `normalMap1` — the second water normal map.
    pub normal_map1: Texture,
}

impl Water2MeshOptions {
    /// The options with every default and the two normal maps.
    pub fn new(normal_map0: Texture, normal_map1: Texture) -> Self {
        Self {
            color: Color::from_hex(0xffffff),
            flow_direction: Vector2::new(1.0, 0.0),
            flow_speed: 0.03,
            reflectivity: 0.02,
            scale: 1.0,
            flow_map: None,
            normal_map0,
            normal_map1,
        }
    }
}

/// `new WaterMesh( geometry, options )` from `Water2Mesh.js` — water with
/// reflections, refractions and flow maps.
pub struct Water2Mesh {
    /// The `Mesh` itself: the geometry with the water material, and the
    /// reflector's `target` as its child (from the first render on).
    pub mesh: Node,
    /// `waterNode.normalMap0`'s map.
    pub normal_map0: Texture,
    /// `waterNode.normalMap1`'s map.
    pub normal_map1: Texture,
    /// `waterNode.flowMap`'s map, if one was given (`_USE_FLOW`).
    pub flow_map: Option<Texture>,
    /// `waterNode.color` — linear RGB, default white.
    pub color: SettableValue,
    /// `waterNode.flowDirection` — default `( 1, 0 )`. The shader uses it as
    /// given. The page passes `( 1, 1 )` unnormalised at construction, as
    /// `examples/webgpu_water.rs` does, and normalises it only in its GUI
    /// callbacks.
    pub flow_direction: SettableValue,
    /// `waterNode.flowSpeed` — default `0.03`.
    pub flow_speed: SettableValue,
    /// `waterNode.reflectivity` — default `0.02`.
    pub reflectivity: SettableValue,
    /// `waterNode.scale` — default `1`.
    pub scale: SettableValue,
    /// `waterNode.flowConfig` — `( flowMapOffset0, flowMapOffset1,
    /// halfCycle )`, advanced by every render's `updateBefore`.
    pub flow_config: SettableValue,
    /// The graph's `reflectionSampler`: the `reflector()` whose target is a
    /// child of [`mesh`](Self::mesh).
    pub reflection_sampler: ReflectorNode,
}

/// The uniforms as nodes, for the shader.
struct Uniforms {
    color: NodeRef,
    flow_direction: NodeRef,
    reflectivity: NodeRef,
    scale: NodeRef,
    flow_config: NodeRef,
}

impl Water2Mesh {
    /// `new WaterMesh( geometry, options )`.
    pub fn new(geometry: Rc<BufferGeometry>, options: Water2MeshOptions) -> Self {
        let c = options.color;
        let d = options.flow_direction;

        // In upstream's order.
        let (color, color_cell) = uniform_settable(Type::Vec3, vec![c.r, c.g, c.b]);
        let (flow_direction, flow_direction_cell) = uniform_settable(Type::Vec2, vec![d.x, d.y]);
        let (_flow_speed, flow_speed_cell) = uniform_settable(Type::F32, vec![options.flow_speed]);
        let (reflectivity, reflectivity_cell) =
            uniform_settable(Type::F32, vec![options.reflectivity]);
        let (scale, scale_cell) = uniform_settable(Type::F32, vec![options.scale]);
        let (flow_config, flow_config_cell) = uniform_settable(Type::Vec3, vec![0.0; 3]);

        let uniforms = Uniforms {
            color,
            flow_direction,
            reflectivity,
            scale,
            flow_config,
        };

        let mut reflection_sampler = reflector(ReflectorParameters::default());
        let output = water_output(
            &uniforms,
            &options.normal_map0,
            &options.normal_map1,
            options.flow_map.as_ref(),
            &mut reflection_sampler,
        );

        let water_node = custom(WaterNode {
            flow_speed: flow_speed_cell.clone(),
            flow_config: flow_config_cell.clone(),
            output,
        });

        let mut material = MeshBasicNodeMaterial::new();
        material.transparent = true;
        material.color_node = Some(water_node);

        let mesh = Mesh::new(geometry, material);

        // `this.waterBody.add( reflectionSampler.target )`, which upstream runs
        // inside `setup()`, i.e. as the material is first built.
        reflection_sampler.add_target_on_setup(&mesh);

        Self {
            mesh,
            normal_map0: options.normal_map0,
            normal_map1: options.normal_map1,
            flow_map: options.flow_map,
            color: color_cell,
            flow_direction: flow_direction_cell,
            flow_speed: flow_speed_cell,
            reflectivity: reflectivity_cell,
            scale: scale_cell,
            flow_config: flow_config_cell,
            reflection_sampler,
        }
    }

    /// `waterNode.updateFlow( delta )` — what every render's `updateBefore`
    /// runs with `frame.deltaTime`.
    pub fn update_flow(&self, delta: f64) {
        update_flow(&self.flow_config, self.flow_speed.get()[0], delta);
    }
}

/// `WaterNode.updateFlow( delta )`.
fn update_flow(flow_config: &SettableValue, flow_speed: f64, delta: f64) {
    let mut config = flow_config.get();

    config[0] += flow_speed * delta; // flowMapOffset0
    config[1] = config[0] + HALF_CYCLE; // flowMapOffset1

    // Important: The distance between offsets should be always the value of "halfCycle".
    // Moreover, both offsets should be in the range of [ 0, cycle ].
    // This approach ensures a smooth water flow and avoids "reset" effects.

    if config[0] >= CYCLE {
        config[0] = 0.0;
        config[1] = HALF_CYCLE;
    } else if config[1] >= CYCLE {
        config[1] -= CYCLE;
    }

    config[2] = HALF_CYCLE;

    flow_config.set(config);
}

/// `WaterNode` — the material's `colorNode`.
struct WaterNode {
    flow_speed: SettableValue,
    flow_config: SettableValue,
    /// What `setup()`'s `Fn()` returns, built once; see the module docs.
    output: NodeRef,
}

impl CustomNode for WaterNode {
    fn type_name(&self) -> &'static str {
        "WaterNode"
    }

    /// `super( 'vec4' )`.
    fn node_type(&self) -> Type {
        Type::Vec4
    }

    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        self.output.clone()
    }

    /// `this.updateBeforeType = NodeUpdateType.RENDER`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Render
    }

    /// `updateBefore( frame )`: `this.updateFlow( frame.deltaTime )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let delta = renderer.node_frame().delta_time;
        update_flow(&self.flow_config, self.flow_speed.get()[0], delta);
        true
    }
}

/// The body of `WaterNode.setup()`'s `Fn()`.
fn water_output(
    u: &Uniforms,
    normal_map0: &Texture,
    normal_map1: &Texture,
    flow_map: Option<&Texture>,
    reflection_sampler: &mut ReflectorNode,
) -> NodeRef {
    let flow_map_offset0 = u.flow_config.x();
    let flow_map_offset1 = u.flow_config.y();
    let half_cycle = u.flow_config.z();

    let to_eye = camera_position().sub(position_world()).normalize();

    // `vec2( … )` and the operator chain are both assignable proxies, so the
    // `mulAssign` below declares them as a function-scope var.
    let flow = match flow_map {
        // `this._USE_FLOW === true`
        Some(flow_map) => to_var_intent(texture(flow_map).xy().mul(2.0).sub(1.0)),
        None => to_var_intent(vec2_join(vec![u.flow_direction.x(), u.flow_direction.y()])),
    };

    let flip = flow.x().mul_assign(-1.0);

    // sample normal maps (distort uvs with flowdata)

    let uvs = uv();

    let normal_uv0 = uvs
        .mul(u.scale.clone())
        .add(flow.mul(flow_map_offset0.clone()));
    let normal_uv1 = uvs.mul(u.scale.clone()).add(flow.mul(flow_map_offset1));

    // `this.normalMapN.sample( uv )`: a clone of the texture node at a new uv,
    // through the map's own uv matrix.
    let normal_color0 = texture_with_uv(normal_map0, normal_uv0);
    let normal_color1 = texture_with_uv(normal_map1, normal_uv1);

    // linear interpolate to get the final normal color
    let flow_lerp = abs(half_cycle.clone().sub(flow_map_offset0)).div(half_cycle);
    // Read three times, so three emits a `let` (§8, "Usage-promoted temps");
    // so is every other `to_const` below.
    let normal_color = to_const(None, mix(normal_color0, normal_color1, flow_lerp));

    // calculate normal vector
    let normal = to_const(
        None,
        vec3_join(vec![
            normal_color.x().mul(2.0).sub(1.0),
            normal_color.z(),
            normal_color.y().mul(2.0).sub(1.0),
        ])
        .normalize(),
    );

    // calculate the fresnel term to blend reflection and refraction maps
    let theta = max(dot(to_eye, normal.clone()), 0.0);
    let reflectance = float(1.0)
        .sub(theta)
        .pow(5.0)
        .mul(float(1.0).sub(u.reflectivity.clone()))
        .add(u.reflectivity.clone());

    // reflector, refractor

    let offset = to_var(None, normal.xz().mul(0.05));

    reflection_sampler.uv_node = reflection_sampler.uv_node.add(offset.clone());

    let refractor_uv = to_const(None, screen_uv().add(offset));
    let refraction_sampler = viewport_shared_texture_at(viewport_safe_uv(refractor_uv));

    // calculate final uv coords

    let output = vec4_join(vec![u.color.clone(), float(1.0)]).mul(mix(
        refraction_sampler,
        reflection_sampler.node(),
        reflectance,
    ));

    block(vec![flip], output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(x: f64, y: f64, z: f64) -> SettableValue {
        SettableValue::new(vec![x, y, z])
    }

    /// The offsets stay half a cycle apart and inside `[ 0, cycle ]`.
    #[test]
    fn update_flow_advances_offset0_and_keeps_offset1_half_a_cycle_ahead() {
        let flow_config = config(0.0, 0.0, 0.0);
        update_flow(&flow_config, 0.03, 1.0);
        assert_eq!(flow_config.get(), vec![0.03, 0.03 + HALF_CYCLE, HALF_CYCLE]);
    }

    /// `flowMapOffset1` wraps by a cycle once it reaches one.
    #[test]
    fn update_flow_wraps_offset1() {
        let flow_config = config(0.07, 0.0, 0.0);
        update_flow(&flow_config, 0.03, 1.0);
        let [x, y, z] = flow_config.get()[..] else {
            unreachable!()
        };
        assert_eq!(x, 0.1);
        assert_eq!(y, 0.1 + HALF_CYCLE - CYCLE);
        assert_eq!(z, HALF_CYCLE);
    }

    /// `flowMapOffset0` reaching a cycle resets both.
    #[test]
    fn update_flow_resets_at_a_full_cycle() {
        let flow_config = config(0.14, 0.0, 0.0);
        update_flow(&flow_config, 0.03, 1.0);
        assert_eq!(flow_config.get(), vec![0.0, HALF_CYCLE, HALF_CYCLE]);
    }

    /// The first frame's delta is 0: the offsets start at `( 0, halfCycle )`.
    #[test]
    fn update_flow_with_no_time_sets_the_half_cycle() {
        let flow_config = config(0.0, 0.0, 0.0);
        update_flow(&flow_config, 0.03, 0.0);
        assert_eq!(flow_config.get(), vec![0.0, HALF_CYCLE, HALF_CYCLE]);
    }
}
