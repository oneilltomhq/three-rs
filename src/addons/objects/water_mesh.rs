//! Port of `three.js/examples/jsm/objects/WaterMesh.js` — a basic flat,
//! reflective water surface: a planar mirror ([`reflector()`]) distorted by
//! four scrolling taps of one normal map, with a sun highlight, a diffuse sun
//! term and a Fresnel mix between the water colour and the reflection.
//!
//! Upstream's `WaterMesh extends Mesh`; the port's [`Mesh`] is a constructor
//! returning a scene-graph [`Node`], so [`WaterMesh`] holds that node next to
//! the uniforms, which upstream keeps as fields of the mesh itself. Each
//! uniform is a [`SettableValue`]: `water.sunDirection.value.copy( sun )` is
//! `water.sun_direction.set( vec![ sun.x, sun.y, sun.z ] )`.
//!
//! The shader is upstream's node for node. `getNoise` is an inline `Fn()`,
//! which the port writes as a Rust function that builds the same nodes; the
//! `colorNode` is a `Fn()` called on the spot, which is the same thing.
//!
//! # Divergences
//!
//! * **The mirror is made at construction.** Upstream creates the
//!   `reflector()` inside `colorNode`'s `Fn()`, so it comes into being when
//!   the material is first built, and sets its `resolutionScale` from
//!   `water.resolutionScale` there. The port has no build-time hook on a
//!   material's graph, so it does both in [`WaterMesh::new`], from
//!   [`WaterMeshOptions::resolution_scale`]. In three, setting
//!   `water.resolutionScale` after the first render changes nothing either;
//!   between construction and the first render it would, and here the
//!   reflector's own `resolution_scale` ([`WaterMesh::mirror_sampler`]) is
//!   the knob for that. The third thing that `Fn()` does,
//!   `this.add( mirrorSampler.target )`, keeps its timing:
//!   [`ReflectorNode::add_target_on_setup`] has the renderer make the add
//!   just before the reflector's first update. As in three, the first frame
//!   therefore mirrors about the target's identity matrixWorld (the plane
//!   `z = 0`, facing +Z), not the water's surface; from the second frame on
//!   the target sits where the mesh puts it.
//! * **`waterNormals` is required.** Upstream types it `?Texture` with a
//!   `null` default, but a `texture( null )` tap has nothing to sample;
//!   [`WaterMeshOptions::new`] takes the map.
//! * **`NodeMaterial`.** A bare `NodeMaterial` is unlit with no `colorNode`
//!   of its own, which the port's [`MeshBasicNodeMaterial`] is with its
//!   defaults, as for `SkyMesh`.
//!
//! `material.receivedShadowPositionNode` is set as upstream sets it
//! (`positionWorld + distortion`), but no ported page casts a shadow onto the
//! water, so nothing gates it.

use crate::core::{BufferGeometry, Node};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Vector3};
use crate::nodes::node::SettableValue;
use crate::nodes::reflector_node::{reflector, ReflectorNode, ReflectorParameters};
use crate::nodes::tsl::{
    block, camera_position, dot, float, length, max, mix, position_world, reflect, texture_with_uv,
    time, to_const, to_var, uniform_settable, vec2, vec2_join,
};
use crate::nodes::{NodeRef, Type};
use crate::objects::Mesh;
use crate::textures::Texture;

use std::rc::Rc;

/// `WaterMesh~Options` — the constructor's options, with upstream's
/// defaults.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WaterMeshOptions {
    /// `resolutionScale` — the mirror's render-target size relative to the
    /// drawing buffer. Default `0.5`.
    pub resolution_scale: f64,
    /// `waterNormals` — the water's normal map. Required here; see the
    /// module docs.
    pub water_normals: Texture,
    /// `alpha` — the material's opacity. Default `1`.
    pub alpha: f64,
    /// `size` — scales the world position the normal map is read at.
    /// Default `1`.
    pub size: f64,
    /// `sunColor` — default `0xffffff`.
    pub sun_color: Color,
    /// `sunDirection` — default `( 0.70707, 0.70707, 0 )`.
    pub sun_direction: Vector3,
    /// `waterColor` — default `0x7f7f7f`.
    pub water_color: Color,
    /// `distortionScale` — how far the normals push the reflection's uv.
    /// Default `20`.
    pub distortion_scale: f64,
}

impl WaterMeshOptions {
    /// The options with every default and `water_normals` as the normal map.
    pub fn new(water_normals: Texture) -> Self {
        Self {
            resolution_scale: 0.5,
            water_normals,
            alpha: 1.0,
            size: 1.0,
            sun_color: Color::from_hex(0xffffff),
            sun_direction: Vector3::new(0.70707, 0.70707, 0.0),
            water_color: Color::from_hex(0x7f7f7f),
            distortion_scale: 20.0,
        }
    }
}

/// `new WaterMesh( geometry, options )` — a basic flat, reflective water
/// effect.
///
/// References:
///
/// - [Flat mirror for three.js](https://github.com/Slayvin)
/// - [An implementation of water shader based on the flat mirror](https://home.adelphi.edu/~stemkoski/)
/// - [Water shader explanations in WebGL](http://29a.ch/slides/2012/webglwater/)
pub struct WaterMesh {
    /// The `Mesh` itself: the geometry with the water material, and the
    /// mirror's `target` as its child.
    pub mesh: Node,
    /// `water.resolutionScale`, as constructed. The mirror read it then; see
    /// the module docs.
    pub resolution_scale: f64,
    /// `water.waterNormals`'s map.
    pub water_normals: Texture,
    /// `water.alpha` — default `1`.
    pub alpha: SettableValue,
    /// `water.size` — default `1`.
    pub size: SettableValue,
    /// `water.sunColor` — linear RGB, default white.
    pub sun_color: SettableValue,
    /// `water.sunDirection` — default `( 0.70707, 0.70707, 0 )`. The shader
    /// uses it as given; the page normalises it.
    pub sun_direction: SettableValue,
    /// `water.waterColor` — linear RGB, default `0x7f7f7f`.
    pub water_color: SettableValue,
    /// `water.distortionScale` — default `20`.
    pub distortion_scale: SettableValue,
    /// The `colorNode`'s `mirrorSampler`: the `reflector()` whose target is a
    /// child of [`mesh`](Self::mesh).
    pub mirror_sampler: ReflectorNode,
}

/// The uniforms as nodes, for the shader.
struct Uniforms {
    alpha: NodeRef,
    size: NodeRef,
    sun_color: NodeRef,
    sun_direction: NodeRef,
    water_color: NodeRef,
    distortion_scale: NodeRef,
}

impl WaterMesh {
    /// `new WaterMesh( geometry, options )`.
    pub fn new(geometry: Rc<BufferGeometry>, options: WaterMeshOptions) -> Self {
        let color = |c: Color| vec![c.r, c.g, c.b];
        let v = &options.sun_direction;

        let (alpha, alpha_cell) = uniform_settable(Type::F32, vec![options.alpha]);
        let (size, size_cell) = uniform_settable(Type::F32, vec![options.size]);
        let (sun_color, sun_color_cell) = uniform_settable(Type::Vec3, color(options.sun_color));
        let (sun_direction, sun_direction_cell) = uniform_settable(Type::Vec3, vec![v.x, v.y, v.z]);
        let (water_color, water_color_cell) =
            uniform_settable(Type::Vec3, color(options.water_color));
        let (distortion_scale, distortion_scale_cell) =
            uniform_settable(Type::F32, vec![options.distortion_scale]);

        let uniforms = Uniforms {
            alpha,
            size,
            sun_color,
            sun_direction,
            water_color,
            distortion_scale,
        };

        // `const mirrorSampler = reflector()`, then
        // `mirrorSampler.reflector.resolutionScale = this.resolutionScale`.
        let mut mirror_sampler = reflector(ReflectorParameters {
            resolution_scale: options.resolution_scale,
            ..ReflectorParameters::default()
        });

        let material = Self::material(&uniforms, &options.water_normals, &mut mirror_sampler);
        let mesh = Mesh::new(geometry, material);

        // `this.add( mirrorSampler.target )`, which upstream runs inside the
        // `colorNode` `Fn()`, i.e. as the material is first built.
        mirror_sampler.add_target_on_setup(&mesh);

        Self {
            mesh,
            resolution_scale: options.resolution_scale,
            water_normals: options.water_normals,
            alpha: alpha_cell,
            size: size_cell,
            sun_color: sun_color_cell,
            sun_direction: sun_direction_cell,
            water_color: water_color_cell,
            distortion_scale: distortion_scale_cell,
            mirror_sampler,
        }
    }

    /// The `NodeMaterial` the constructor builds.
    fn material(
        u: &Uniforms,
        water_normals: &Texture,
        mirror_sampler: &mut ReflectorNode,
    ) -> MeshBasicNodeMaterial {
        // `getNoise( positionWorld.xz.mul( this.size ) )`. The argument is
        // read four times, so three emits it as a `let` (§8, "Usage-promoted
        // temps"); so is every other `to_const` below.
        let noise = get_noise(
            water_normals,
            to_const(None, position_world().xz().mul(u.size.clone())),
        );
        // `noise.xzy.mul( 1.5, 1.0, 1.5 )` is `OperatorNode`'s variadic
        // `mul`: three multiplications by scalars, not one by
        // `vec3( 1.5, 1.0, 1.5 )`. Ported as written.
        let surface_normal = to_const(
            None,
            noise.swizzle("xzy").mul(1.5).mul(1.0).mul(1.5).normalize(),
        );

        let world_to_eye = to_const(None, camera_position().sub(position_world()));
        let eye_direction = to_const(None, world_to_eye.normalize());

        let reflection = reflect(u.sun_direction.negate(), surface_normal.clone()).normalize();
        let direction = max(0.0, dot(eye_direction.clone(), reflection));
        let specular_light = direction.pow(100.0).mul(u.sun_color.clone()).mul(2.0);
        let diffuse_light = max(dot(u.sun_direction.clone(), surface_normal.clone()), 0.0)
            .mul(u.sun_color.clone())
            .mul(0.5);

        let distance = length(world_to_eye);

        let distortion = surface_normal
            .xz()
            .mul(float(0.001).add(float(1.0).div(distance)))
            .mul(u.distortion_scale.clone());

        // Material

        let mut material = MeshBasicNodeMaterial::new();
        material.transparent = true;

        material.opacity_node = Some(u.alpha.clone());

        material.received_shadow_position_node = Some(position_world().add(distortion.clone()));

        // `colorNode = Fn( () => { … } )()`.
        mirror_sampler.uv_node = mirror_sampler.uv_node.add(distortion);

        let theta = max(dot(eye_direction.clone(), surface_normal.clone()), 0.0);
        let rf0 = float(0.02);
        let reflectance = float(1.0)
            .sub(theta)
            .pow(5.0)
            .mul(float(1.0).sub(rf0.clone()))
            .add(rf0);
        let scatter = max(0.0, dot(surface_normal, eye_direction)).mul(u.water_color.clone());
        let albedo = mix(
            u.sun_color.mul(diffuse_light).mul(0.3).add(scatter),
            mirror_sampler.node().rgb().add(specular_light),
            reflectance,
        );

        material.color_node = Some(albedo);
        material
    }
}

/// The inline `getNoise( [ uv ] )` Fn: four taps of the normal map at
/// different scales, each scrolling with `time` at its own rate, summed and
/// mapped to about `[ -1, 1 ]`.
fn get_noise(water_normals: &Texture, uv: NodeRef) -> NodeRef {
    let offset = time();

    let uv0 = to_var(
        None,
        uv.div(103.0)
            .add(vec2_join(vec![offset.div(17.0), offset.div(29.0)])),
    );
    let uv1 = to_var(
        None,
        uv.div(107.0)
            .sub(vec2_join(vec![offset.div(-19.0), offset.div(31.0)])),
    );
    let uv2 = to_var(
        None,
        uv.div(vec2(8907.0, 9803.0))
            .add(vec2_join(vec![offset.div(101.0), offset.div(97.0)])),
    );
    let uv3 = to_var(
        None,
        uv.div(vec2(1091.0, 1027.0))
            .sub(vec2_join(vec![offset.div(109.0), offset.div(-113.0)])),
    );

    // `this.waterNormals.sample( uvN )`: `sample()` clones the texture node,
    // but the clones share the base node's one `uniform( texture.matrix )`.
    let sample0 = texture_with_uv(water_normals, uv0.clone());
    let sample1 = texture_with_uv(water_normals, uv1.clone());
    let sample2 = texture_with_uv(water_normals, uv2.clone());
    let sample3 = texture_with_uv(water_normals, uv3.clone());

    let noise = sample0.add(sample1).add(sample2).add(sample3);

    // The four `toVar()`s are declared in order before any tap reads one.
    block(vec![uv0, uv1, uv2, uv3], noise.mul(0.5).sub(1.0))
}
