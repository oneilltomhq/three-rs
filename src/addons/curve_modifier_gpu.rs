//! Port of `three.js/examples/jsm/modifiers/CurveModifierGPU.js` — `Flow`,
//! which bends a mesh along a curve in its vertex shader.
//!
//! The curve is baked into a `TEXTURE_WIDTH` x `TEXTURE_HEIGHT` half-float
//! `DataTexture` per curve: row 0 the spaced points, rows 1–3 the Frenet
//! tangents, normals and binormals. The material's `positionNode` looks the
//! frame up at the vertex's arc-length position and rebuilds the vertex in it;
//! `normalNode` is the rotated normal, carried across as a varying.
//!
//! Upstream's `Flow` clones an `Object3D` and patches every `Mesh` /
//! `InstancedMesh` under it. The port has no generic scene-graph clone, so
//! [`Flow::new`] takes the one mesh's geometry and material instead and
//! builds the (single) bent mesh itself — which is every use the examples
//! make of it.

use std::rc::Rc;

use crate::core::{BufferGeometry, Node};
use crate::extras::{to_half_float, Curve, FrenetFrames};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Vector3;
use crate::nodes::node::SettableValue;
use crate::nodes::tsl::{
    block, float, join, mod_float, model_world_matrix, normal_local, position_local, texture_level,
    to_var, uniform_settable, varying_property, vec2_join, vec3_join, vec4_join,
};
use crate::nodes::{NodeRef, Type};
use crate::objects::Mesh;
use crate::textures::{Texture, Wrapping};

const CHANNELS: usize = 4;
const TEXTURE_WIDTH: usize = 1024;
const TEXTURE_HEIGHT: usize = 4;

/// `initSplineTexture( numberOfCurves )`.
///
/// Upstream sets `dataTexture.wrapS = RepeatWrapping` and then
/// `dataTexture.wrapY = RepeatWrapping` — `wrapY` is not a `Texture`
/// property, so `wrapT` stays `ClampToEdgeWrapping`. The port reproduces
/// that: repeat in `s`, clamp in `t`.
fn init_spline_texture(number_of_curves: usize) -> Texture {
    let data = vec![0u16; TEXTURE_WIDTH * TEXTURE_HEIGHT * number_of_curves * CHANNELS];
    let texture = Texture::data_rgba16float(
        TEXTURE_WIDTH as u32,
        (TEXTURE_HEIGHT * number_of_curves) as u32,
        &data,
    );
    texture.set_wrapping(Wrapping::Repeat, Wrapping::ClampToEdge);
    // `magFilter = minFilter = LinearFilter` is what `data_rgba16float`
    // already sets.
    texture
}

/// A curve seen through a different `arcLengthDivisions`.
///
/// `updateSplineTexture` sets `splineCurve.arcLengthDivisions = numberOfPoints
/// / 2` on the caller's curve before sampling it. The port's [`Curve`] keeps
/// no per-instance `arcLengthDivisions` (the trait method is the default of
/// 200), so the override is this adapter rather than a mutation. Every
/// arc-length consumer — `getSpacedPoints`, and `computeFrenetFrames` through
/// `getTangentAt` — goes through the trait's `arc_length_divisions()`, so
/// they all see it.
struct ArcLengthDivisions<'a, C: ?Sized> {
    curve: &'a C,
    divisions: usize,
}

impl<C: Curve + ?Sized> Curve for ArcLengthDivisions<'_, C> {
    type Point = C::Point;

    fn get_point(&self, t: f64) -> Self::Point {
        self.curve.get_point(t)
    }

    fn type_name(&self) -> &'static str {
        self.curve.type_name()
    }

    fn curve_path_resolution(&self, divisions: usize) -> usize {
        self.curve.curve_path_resolution(divisions)
    }

    fn is_closed_catmull_rom(&self) -> bool {
        self.curve.is_closed_catmull_rom()
    }

    fn arc_length_divisions(&self) -> usize {
        self.divisions
    }

    fn get_tangent(&self, t: f64) -> Self::Point {
        self.curve.get_tangent(t)
    }
}

/// `updateSplineTexture( texture, splineCurve, offset )`, writing into the
/// `Uint16Array` it hands back.
fn update_spline_texture<C: Curve<Point = Vector3> + ?Sized>(
    data: &mut [u16],
    spline_curve: &C,
    offset: usize,
) {
    let number_of_points = TEXTURE_WIDTH * (TEXTURE_HEIGHT / 4);
    // `splineCurve.arcLengthDivisions = numberOfPoints / 2;
    // splineCurve.updateArcLengths();` — see [`ArcLengthDivisions`].
    let curve = ArcLengthDivisions {
        curve: spline_curve,
        divisions: number_of_points / 2,
    };
    let points = curve.get_spaced_points(number_of_points);
    let FrenetFrames {
        tangents,
        normals,
        binormals,
    } = curve.compute_frenet_frames(number_of_points, true);

    for i in 0..number_of_points {
        let row_offset = i / TEXTURE_WIDTH;
        let row_index = i % TEXTURE_WIDTH;

        let base = row_offset + TEXTURE_HEIGHT * offset;
        set_texture_value(data, row_index, &points[i], base);
        set_texture_value(data, row_index, &tangents[i], 1 + base);
        set_texture_value(data, row_index, &normals[i], 2 + base);
        set_texture_value(data, row_index, &binormals[i], 3 + base);
    }
}

/// `setTextureValue( texture, index, x, y, z, o )`.
fn set_texture_value(data: &mut [u16], index: usize, pt: &Vector3, o: usize) {
    let i = CHANNELS * TEXTURE_WIDTH * o; // Row Offset
    data[index * CHANNELS + i] = to_half_float(pt.x);
    data[index * CHANNELS + i + 1] = to_half_float(pt.y);
    data[index * CHANNELS + i + 2] = to_half_float(pt.z);
    data[index * CHANNELS + i + 3] = to_half_float(1.0);
}

/// `getUniforms( splineTexture )`, as the values the `reference()` nodes
/// read. Each is a settable uniform: `reference( name, 'float', uniforms )`
/// re-reads `uniforms[ name ]` every frame, and writing the cell is the
/// port's `uniforms.name = value`.
pub struct FlowUniforms {
    /// `pathOffset` — time of path curve.
    pub path_offset: SettableValue,
    /// `pathSegment` — fractional length of path.
    pub path_segment: SettableValue,
    pub spine_offset: SettableValue,
    pub spine_length: SettableValue,
    /// `flow` — int.
    pub flow: SettableValue,
}

struct FlowNodes {
    path_offset: NodeRef,
    path_segment: NodeRef,
    spine_offset: NodeRef,
    spine_length: NodeRef,
    flow: NodeRef,
}

fn get_uniforms() -> (FlowUniforms, FlowNodes) {
    let (path_offset, path_offset_cell) = uniform_settable(Type::F32, vec![0.0]);
    let (path_segment, path_segment_cell) = uniform_settable(Type::F32, vec![1.0]);
    let (spine_offset, spine_offset_cell) = uniform_settable(Type::F32, vec![161.0]);
    let (spine_length, spine_length_cell) = uniform_settable(Type::F32, vec![400.0]);
    let (flow, flow_cell) = uniform_settable(Type::F32, vec![1.0]);
    (
        FlowUniforms {
            path_offset: path_offset_cell,
            path_segment: path_segment_cell,
            spine_offset: spine_offset_cell,
            spine_length: spine_length_cell,
            flow: flow_cell,
        },
        FlowNodes {
            path_offset,
            path_segment,
            spine_offset,
            spine_length,
            flow,
        },
    )
}

/// `modifyShader( material, uniforms, numberOfCurves )`.
fn modify_shader(
    material: &mut MeshBasicNodeMaterial,
    spine_texture: &Texture,
    uniforms: &FlowNodes,
    number_of_curves: usize,
) {
    let texture_stacks = (TEXTURE_HEIGHT / 4) as f64;
    let texture_scale = (TEXTURE_HEIGHT * number_of_curves) as f64;

    let world_pos = to_var(
        None,
        model_world_matrix().mul(vec4_join(vec![position_local(), float(1.0)])),
    );

    let bend = to_var(None, uniforms.flow.greater_than(float(0.0)));
    let x_weight = to_var(None, bend.select(float(0.0), float(1.0)));
    let spine_portion = bend.select(
        world_pos
            .x()
            .add(uniforms.spine_offset.clone())
            .div(uniforms.spine_length.clone()),
        float(0.0),
    );
    let mt = to_var(
        None,
        spine_portion
            .mul(uniforms.path_segment.clone())
            .add(uniforms.path_offset.clone())
            .mul(float(texture_stacks)),
    );

    let assign_mt = mt.assign(mod_float(mt.clone(), float(texture_stacks)));

    let row_offset = to_var(None, mt.floor());

    // `texture( spineTexture, uv )` in the vertex stage: three's builder
    // samples at level 0 there, `textureSampleLevel( …, 0 )`.
    let sample = |row: f64| {
        texture_level(
            spine_texture,
            vec2_join(vec![
                mt.clone(),
                row_offset.add(float(row)).div(float(texture_scale)),
            ]),
            float(0.0),
        )
        .xyz()
    };

    let spine_pos = sample(0.5);
    let a = sample(1.5);
    let b = sample(2.5);
    let c = sample(3.5);
    let basis = to_var(None, join(Type::Mat3, vec![a, b, c]));

    let curve_normal = varying_property("curveNormal", Type::Vec3, false);
    let assign_normal = curve_normal.assign(basis.mul(normal_local()));

    let result = basis
        .mul(vec3_join(vec![
            world_pos.x().mul(x_weight),
            world_pos.y(),
            world_pos.z(),
        ]))
        .add(spine_pos);

    material.position_node = Some(block(vec![assign_mt, assign_normal], result));
    material.normal_node = Some(curve_normal);
}

/// `Flow`.
pub struct Flow {
    /// `flow.object3D` — the bent mesh.
    pub object3d: Node,
    /// `flow.splineTexture`.
    pub spline_texture: Texture,
    /// `flow.uniforms`, minus `spineTexture` (which is [`Flow::spline_texture`]).
    pub uniforms: FlowUniforms,
    /// `flow.curveLengthArray`.
    pub curve_length_array: Vec<Option<f64>>,
    data: Vec<u16>,
}

impl Flow {
    /// `new Flow( mesh, numberOfCurves = 1 )`, for one mesh: its geometry is
    /// shared and its material cloned, as `mesh.clone()` and
    /// `child.material.clone()` do.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: &MeshBasicNodeMaterial,
        number_of_curves: usize,
    ) -> Self {
        let spline_texture = init_spline_texture(number_of_curves);
        let (uniforms, nodes) = get_uniforms();

        let mut material = material.clone();
        modify_shader(&mut material, &spline_texture, &nodes, number_of_curves);

        let object3d = Mesh::new(geometry, material);

        Self {
            object3d,
            spline_texture,
            uniforms,
            curve_length_array: vec![None; number_of_curves],
            data: vec![0u16; TEXTURE_WIDTH * TEXTURE_HEIGHT * number_of_curves * CHANNELS],
        }
    }

    /// `flow.updateCurve( index, curve )`.
    pub fn update_curve<C: Curve<Point = Vector3> + ?Sized>(&mut self, index: usize, curve: &C) {
        assert!(
            index < self.curve_length_array.len(),
            "Flow: Index out of range."
        );
        let curve_length = curve.get_length();
        self.uniforms.spine_length.set(vec![curve_length]);
        self.curve_length_array[index] = Some(curve_length);
        update_spline_texture(&mut self.data, curve, index);
        // `texture.needsUpdate = true`.
        self.spline_texture
            .set_data(bytemuck::cast_slice(&self.data).to_vec());
    }

    /// `flow.moveAlongCurve( amount )`.
    pub fn move_along_curve(&self, amount: f64) {
        let offset = self.uniforms.path_offset.get()[0];
        self.uniforms.path_offset.set(vec![offset + amount]);
    }

    /// The half-float texels [`Flow::update_curve`] last wrote — the
    /// `splineTexture.image.data` `Uint16Array`.
    pub fn spline_data(&self) -> &[u16] {
        &self.data
    }
}
