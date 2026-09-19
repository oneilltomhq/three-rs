//! The instanced attributes a `LineSegmentsGeometry` carries, and the TSL
//! accessors `Line2NodeMaterial` reads them through.
//!
//! In three.js the fat-line material writes `attribute( 'instanceStart' )` and
//! `NodeBuilder` resolves the name against `builder.geometry`, where
//! `LineSegmentsGeometry` has put an `InstancedInterleavedBuffer( array, 6, 1 )`
//! and two `InterleavedBufferAttribute` views of it. The port's instanced
//! attribute node carries its data instead — "the node graph is built from the
//! material and the renderer has no attribute-name table"
//! ([`crate::nodes::tsl::instanced_data_attribute`]) — so the two arrays have
//! to reach `setup()`. They travel on
//! [`SetupContext::line_segments`](crate::materials::SetupContext), which is
//! the same shape `morph` and `batch` already use for geometry-derived setup
//! input.
//!
//! This is option **(a)** of `scouts/webgpu_lines_fat/PLAN.md` §4.4, taken
//! deliberately over (b) — giving `BufferGeometry` real interleaved instanced
//! attributes, which is the right end state and is a follow-up, because it
//! runs through `ensure_geometry` / `vertex_buffers()` / `programs.rs`, the
//! path every example uses.

use std::rc::Rc;

use crate::nodes::node::{InstanceBuffer, Type};
use crate::nodes::tsl::{instanced_buffer_attribute, instanced_data_buffer};
use crate::nodes::NodeRef;

/// `LineSegmentsGeometry`'s instanced attributes: one interleaved array per
/// pair of views, exactly as three.js lays them out.
///
/// * `positions` is `instanceStart` (offset 0) and `instanceEnd` (offset 3) of
///   an `InstancedInterleavedBuffer( array, 6, 1 )` — six floats per segment.
/// * `colors` is `instanceColorStart` / `instanceColorEnd`, the same shape.
/// * `distances` is `instanceDistanceStart` / `instanceDistanceEnd`, two floats
///   per segment, written by `LineSegments2::compute_line_distances()`.
///
/// The arrays are `Rc`, and [`Hash`] hashes them **by pointer**, so the render
/// object's cache key stays cheap: two draws of the same geometry hash the
/// same, and a different geometry cannot collide with it.
#[derive(Clone, Debug)]
pub struct LineSegmentsAttributes {
    pub positions: Rc<Vec<f32>>,
    pub colors: Option<Rc<Vec<f32>>>,
    pub distances: Option<Rc<Vec<f32>>>,
}

impl LineSegmentsAttributes {
    /// The number of segments — `InstancedInterleavedBuffer.count`, i.e. the
    /// draw's `instanceCount`.
    pub fn instance_count(&self) -> usize {
        self.positions.len() / 6
    }

    /// `attribute( 'instanceStart' )` and `attribute( 'instanceEnd' )`, as two
    /// views of **one** `stepMode: 'instance'` vertex buffer of stride 24.
    ///
    /// The buffer is built once and shared, because
    /// [`NodeProgram::vertex_buffers`](crate::nodes::NodeProgram::vertex_buffers)
    /// groups instanced attributes by the buffer's `Rc` identity: two separate
    /// `instanced_data_attribute` calls over the same array would be two
    /// vertex buffers, and three's dump has one.
    pub fn start_end(&self) -> (NodeRef, NodeRef) {
        Self::pair(&self.positions)
    }

    /// `attribute( 'instanceColorStart' )` / `attribute( 'instanceColorEnd' )`.
    pub fn color_start_end(&self) -> Option<(NodeRef, NodeRef)> {
        self.colors.as_ref().map(Self::pair)
    }

    /// `attribute( 'instanceDistanceStart' )` / `…End` — two floats per
    /// segment rather than two `vec3`s.
    pub fn distance_start_end(&self) -> Option<(NodeRef, NodeRef)> {
        self.distances.as_ref().map(|data| {
            let buffer = instanced_data_buffer(data, 2);
            (
                instanced_buffer_attribute(&buffer, 0, Type::F32),
                instanced_buffer_attribute(&buffer, 1, Type::F32),
            )
        })
    }

    fn pair(data: &Rc<Vec<f32>>) -> (NodeRef, NodeRef) {
        let buffer: Rc<InstanceBuffer> = instanced_data_buffer(data, 6);
        (
            instanced_buffer_attribute(&buffer, 0, Type::Vec3),
            instanced_buffer_attribute(&buffer, 3, Type::Vec3),
        )
    }
}

impl std::hash::Hash for LineSegmentsAttributes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = |data: &Rc<Vec<f32>>| Rc::as_ptr(data) as *const u8 as usize;
        ptr(&self.positions).hash(state);
        self.colors.as_ref().map(ptr).hash(state);
        self.distances.as_ref().map(ptr).hash(state);
    }
}
