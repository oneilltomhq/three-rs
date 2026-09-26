//! Port of `three.js/examples/jsm/lines/` — fat lines.
//!
//! * `LineSegmentsGeometry.js` and `LineGeometry.js` verbatim.
//! * `webgpu/LineSegments2.js` and `webgpu/Line2.js`, the WebGPU variants that
//!   pair with [`Line2NodeMaterial`](crate::materials::Line2NodeMaterial).
//!   Only the object half is here; `raycast()` and its world-units cousin are
//!   not ported, because nothing on the ladder picks.
//!
//! A fat line is a `Mesh`, not a `Line`: the segment list becomes instanced
//! attributes over a fixed eight-vertex quad, and the vertex shader expands
//! each instance into a screen-space ribbon. That is why `LineSegments2`
//! returns a node whose payload is [`Payload::Mesh`], with the geometry the
//! quad and the segments hanging off
//! [`Mesh::line_segments`](crate::objects::Mesh::line_segments).

use std::rc::Rc;

use crate::core::{BoundingBox, BoundingSphere, BufferAttribute, BufferGeometry, Node};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Vector3;
use crate::nodes::lines::LineSegmentsAttributes;
use crate::objects::{Mesh, Payload};

/// `new LineSegmentsGeometry()` — the instanced quad, plus the segment and
/// colour arrays set on it.
///
/// three.js is an `InstancedBufferGeometry` carrying five attributes: the
/// shared `position` / `uv` quad and the three interleaved instanced pairs.
/// The port splits them: the quad is a [`BufferGeometry`], and the instanced
/// pairs are a [`LineSegmentsAttributes`] that travels with the object — see
/// [`crate::nodes::lines`].
#[derive(Clone)]
pub struct LineSegmentsGeometry {
    geometry: BufferGeometry,
    positions: Option<Rc<Vec<f32>>>,
    colors: Option<Rc<Vec<f32>>>,
}

impl Default for LineSegmentsGeometry {
    fn default() -> Self {
        Self::new()
    }
}

impl LineSegmentsGeometry {
    /// The eight-vertex, six-triangle quad every fat line instances.
    ///
    /// It spans `x ∈ [-1, 1]` and `y ∈ [-1, 2]`, which the vertex shader reads
    /// as "left or right of the segment" and "before the start / along it /
    /// past the end" — the three bands are what make the two round caps. `uv`
    /// runs `y ∈ [-2, 2]` so that the fragment stage's `abs( uv.y ) > 1` test
    /// picks exactly the cap bands.
    pub fn new() -> Self {
        let positions = vec![
            -1.0, 2.0, 0.0, 1.0, 2.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0,
            0.0, -1.0, -1.0, 0.0, 1.0, -1.0, 0.0,
        ];
        let uvs = vec![
            -1.0, 2.0, 1.0, 2.0, -1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -2.0, 1.0, -2.0,
        ];
        let index = [0, 2, 1, 2, 3, 1, 2, 4, 3, 4, 5, 3, 4, 6, 5, 6, 7, 5];

        let mut geometry = BufferGeometry::new();
        geometry.set_index(&index);
        geometry.set_attribute("position", BufferAttribute::new(positions, 3));
        geometry.set_attribute("uv", BufferAttribute::new(uvs, 2));

        Self {
            geometry,
            positions: None,
            colors: None,
        }
    }

    /// `setPositions( array )` — `xyz xyz` per segment, so a multiple of six.
    ///
    /// three.js wraps the array in an `InstancedInterleavedBuffer( array, 6, 1
    /// )` and takes two `InterleavedBufferAttribute` views at offsets 0 and 3;
    /// the port keeps the array and builds the two views at setup time, which
    /// is the same one vertex buffer of stride 24.
    ///
    /// It also recomputes the bounding volumes, because the quad's own
    /// `position` says nothing about where the line is.
    pub fn set_positions(&mut self, array: impl Into<Vec<f32>>) -> &mut Self {
        let array: Vec<f32> = array.into();
        assert!(
            array.len().is_multiple_of(6),
            "three-rs: LineSegmentsGeometry::set_positions wants xyz xyz per segment, got {} floats",
            array.len()
        );
        self.positions = Some(Rc::new(array));
        self.geometry.bounding_sphere = self.compute_bounding_sphere();
        self
    }

    /// `setColors( array )` — `rgb rgb` per segment.
    pub fn set_colors(&mut self, array: impl Into<Vec<f32>>) -> &mut Self {
        let array: Vec<f32> = array.into();
        assert!(
            array.len().is_multiple_of(6),
            "three-rs: LineSegmentsGeometry::set_colors wants rgb rgb per segment, got {} floats",
            array.len()
        );
        self.colors = Some(Rc::new(array));
        self
    }

    /// `geometry.instanceCount` — one instance per segment.
    pub fn instance_count(&self) -> usize {
        self.positions.as_ref().map_or(0, |p| p.len() / 6)
    }

    /// `LineSegmentsGeometry.computeBoundingBox()` — the union of the
    /// `instanceStart` and `instanceEnd` boxes, which over one interleaved
    /// array is just the box of every point in it.
    pub fn compute_bounding_box(&self) -> Option<BoundingBox> {
        let positions = self.positions.as_ref()?;
        let mut bounding_box = BoundingBox::empty();
        for point in positions.as_chunks::<3>().0 {
            let v = Vector3::new(point[0] as f64, point[1] as f64, point[2] as f64);
            bounding_box.expand_by_point(&v);
        }
        Some(bounding_box)
    }

    /// `LineSegmentsGeometry.computeBoundingSphere()` — the box's centre, then
    /// the farthest endpoint from it.
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        let positions = self.positions.as_ref()?;
        let center = self.compute_bounding_box()?.center();
        let mut max_radius_sq: f64 = 0.0;
        for point in positions.as_chunks::<3>().0 {
            let v = Vector3::new(point[0] as f64, point[1] as f64, point[2] as f64);
            max_radius_sq = max_radius_sq.max(center.distance_to_squared(&v));
        }
        Some(BoundingSphere {
            center,
            radius: max_radius_sq.sqrt(),
        })
    }

    /// The instanced attributes, for
    /// [`SetupContext::line_segments`](crate::materials::SetupContext).
    ///
    /// Panics if `set_positions` was never called: three.js would draw zero
    /// instances and show nothing, which on this stack is a silent wrong
    /// picture rather than an error.
    pub fn attributes(&self) -> LineSegmentsAttributes {
        let positions = self
            .positions
            .clone()
            .expect("three-rs: LineSegmentsGeometry has no positions; call set_positions() first");
        LineSegmentsAttributes {
            positions,
            colors: self.colors.clone(),
            distances: None,
        }
    }

    /// The shared quad, ready for a `Mesh`.
    pub fn geometry(&self) -> &BufferGeometry {
        &self.geometry
    }
}

/// `new LineGeometry()` — a `LineSegmentsGeometry` whose setters take a
/// *polyline* and duplicate the interior points into segment pairs.
#[derive(Clone, Default)]
pub struct LineGeometry(LineSegmentsGeometry);

impl LineGeometry {
    pub fn new() -> Self {
        Self(LineSegmentsGeometry::new())
    }

    /// `setPositions( array )` — `[ x1, y1, z1, x2, y2, z2, … ]` to pairs.
    ///
    /// `n` points become `n - 1` segments of six floats each, which is three's
    /// `2 * ( array.length - 3 )`.
    pub fn set_positions(&mut self, array: &[f32]) -> &mut Self {
        self.0.set_positions(pairs(array));
        self
    }

    /// `setColors( array )` — the same duplication over `rgb` triples.
    pub fn set_colors(&mut self, array: &[f32]) -> &mut Self {
        self.0.set_colors(pairs(array));
        self
    }

    pub fn as_segments(&self) -> &LineSegmentsGeometry {
        &self.0
    }
}

/// `[ a, b, c, d ] → [ a, b, b, c, c, d ]` over triples — the conversion both
/// of `LineGeometry`'s setters do.
fn pairs(array: &[f32]) -> Vec<f32> {
    let length = array.len().saturating_sub(3);
    let mut out = vec![0.0; 2 * length];
    for i in (0..length).step_by(3) {
        out[2 * i..2 * i + 6].copy_from_slice(&array[i..i + 6]);
    }
    out
}

/// `new LineSegments2( geometry, material )` — `extends Mesh`.
pub struct LineSegments2;

impl LineSegments2 {
    /// three.js' constructor, as a scene-graph [`Node`].
    ///
    /// `object_type` is `"LineSegments2"`, but the payload is a
    /// [`Payload::Mesh`]: `WebGPUUtils.getPrimitiveTopology()` reads
    /// `object.isLine`, which a `LineSegments2` does *not* set, so it draws
    /// `triangle-list` like any other mesh.
    #[allow(clippy::new_ret_no_self)] // mirrors three.js' constructor: it returns a scene-graph `Node`.
    pub fn new(geometry: &LineSegmentsGeometry, material: MeshBasicNodeMaterial) -> Node {
        Self::from_parts("LineSegments2", geometry, material)
    }

    fn from_parts(
        object_type: &'static str,
        geometry: &LineSegmentsGeometry,
        material: MeshBasicNodeMaterial,
    ) -> Node {
        let mut object = crate::core::Object3D {
            object_type,
            ..Default::default()
        };
        object.payload = Payload::Mesh(Mesh {
            geometry: Rc::new(geometry.geometry.clone()),
            material: Some(material),
            morph_target_influences: Vec::new(),
            line_segments: Some(geometry.attributes()),
            count: None,
        });
        object.into_node()
    }
}

/// `new Line2( geometry, material )` — `extends LineSegments2`.
pub struct Line2;

impl Line2 {
    #[allow(clippy::new_ret_no_self)] // mirrors three.js' constructor: it returns a scene-graph `Node`.
    pub fn new(geometry: &LineGeometry, material: MeshBasicNodeMaterial) -> Node {
        LineSegments2::from_parts("Line2", geometry.as_segments(), material)
    }
}
