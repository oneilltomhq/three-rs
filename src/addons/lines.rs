//! Port of `three.js/examples/jsm/lines/` — fat lines.
//!
//! * `LineSegmentsGeometry.js` and `LineGeometry.js` verbatim.
//! * `webgpu/LineSegments2.js` and `webgpu/Line2.js`, the WebGPU variants that
//!   pair with [`Line2NodeMaterial`](crate::materials::Line2NodeMaterial).
//!   The object half and `raycast()`, both its world-units and its
//!   screen-space branch, are here.
//!
//! A fat line is a `Mesh`, not a `Line`: the segment list becomes instanced
//! attributes over a fixed eight-vertex quad, and the vertex shader expands
//! each instance into a screen-space ribbon. That is why `LineSegments2`
//! returns a node whose payload is [`Payload::Mesh`], with the geometry the
//! quad and the segments hanging off
//! [`Mesh::line_segments`](crate::objects::Mesh::line_segments).

use std::rc::Rc;

use crate::core::{
    BoundingBox, BoundingSphere, BufferAttribute, BufferGeometry, Intersection, Node, Raycaster,
    RaycasterCamera,
};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Box3, Line3, Matrix4, Ray, Sphere, Vector2, Vector3, Vector4};
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
            resolution: Vector2::new(0.0, 0.0),
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
            count: 1,
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

/// `LineSegments2.raycast( raycaster, intersects )` for a node whose payload
/// is `mesh`, a fat line (its [`Mesh::line_segments`] is set).
///
/// The segments are the instanced `instanceStart` / `instanceEnd` pairs, not
/// the quad geometry, so the bounds tested first are
/// `LineSegmentsGeometry`'s — the box and sphere of every endpoint.
pub fn raycast(
    mesh: &Mesh,
    matrix_world: &Matrix4,
    object: &Node,
    raycaster: &Raycaster,
    intersects: &mut Vec<Intersection>,
) {
    let Some(segments) = &mesh.line_segments else {
        return;
    };
    let Some(material) = &mesh.material else {
        return;
    };
    let world_units = material.world_units;
    let camera = raycaster.camera.as_ref();
    if camera.is_none() && !world_units {
        eprintln!(
            "THREE.LineSegments2: \"Raycaster.camera\" needs to be set in order to raycast against LineSegments2 while worldUnits is set to false."
        );
        return;
    }
    let resolution = segments.resolution;
    if !world_units && (resolution.x == 0.0 || resolution.y == 0.0) {
        return;
    }

    let threshold = raycaster.params.line2.threshold;
    let ray = &raycaster.ray;
    let line_width = material.linewidth + threshold;

    let positions: &[f32] = &segments.positions;
    let Some(bounds) = segment_bounds(positions) else {
        return;
    };
    let (bounding_box, bounding_sphere) = bounds;

    let mut sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
    sphere.apply_matrix4(matrix_world);
    let sphere_margin = match camera {
        Some(camera) if !world_units => {
            let distance_to_sphere = camera.near.max(sphere.distance_to_point(&ray.origin));
            world_space_half_width(camera, distance_to_sphere, &resolution, line_width)
        }
        _ => line_width * 0.5,
    };
    sphere.radius += sphere_margin;
    if !ray.intersects_sphere(&sphere) {
        return;
    }

    let mut box3 = Box3::new(bounding_box.min, bounding_box.max);
    box3.apply_matrix4(matrix_world);
    let box_margin = match camera {
        Some(camera) if !world_units => {
            let distance_to_box = camera.near.max(box3.distance_to_point(&ray.origin));
            world_space_half_width(camera, distance_to_box, &resolution, line_width)
        }
        _ => line_width * 0.5,
    };
    box3.expand_by_scalar(box_margin);
    if !ray.intersects_box(&box3) {
        return;
    }

    match camera {
        Some(camera) if !world_units => raycast_screen_space(
            positions,
            matrix_world,
            &resolution,
            line_width,
            camera,
            ray,
            object,
            intersects,
        ),
        _ => raycast_world_units(positions, matrix_world, line_width, ray, object, intersects),
    }
}

/// `LineSegmentsGeometry.computeBoundingBox()` / `computeBoundingSphere()`
/// over the flat `[ start, end, start, end, … ]` array.
fn segment_bounds(positions: &[f32]) -> Option<(BoundingBox, BoundingSphere)> {
    if positions.is_empty() {
        return None;
    }
    let points = || {
        positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| Vector3::new(p[0] as f64, p[1] as f64, p[2] as f64))
    };
    let mut bounding_box = BoundingBox::empty();
    for v in points() {
        bounding_box.expand_by_point(&v);
    }
    let center = bounding_box.center();
    let max_radius_sq = points().fold(0.0_f64, |r, v| r.max(center.distance_to_squared(&v)));
    Some((
        bounding_box,
        BoundingSphere {
            center,
            radius: max_radius_sq.sqrt(),
        },
    ))
}

/// Segment `i`'s two endpoints, in object space.
fn segment(positions: &[f32], i: usize) -> (Vector3, Vector3) {
    let p = &positions[i * 6..i * 6 + 6];
    (
        Vector3::new(p[0] as f64, p[1] as f64, p[2] as f64),
        Vector3::new(p[3] as f64, p[4] as f64, p[5] as f64),
    )
}

/// `getWorldSpaceHalfWidth( camera, distance, resolution )` — how wide
/// `line_width` screen pixels are, in world units, `distance` in front of the
/// camera. (three names it a half width; it is the full pixel width mapped
/// through the projection, and the margin it feeds is only a coarse test.)
fn world_space_half_width(
    camera: &RaycasterCamera,
    distance: f64,
    resolution: &Vector2,
    line_width: f64,
) -> f64 {
    let mut clip_to_world = Vector4::new(0.0, 0.0, -distance, 1.0);
    clip_to_world.apply_matrix4(&camera.projection_matrix);
    clip_to_world.multiply_scalar(1.0 / clip_to_world.w);
    clip_to_world.x = line_width / resolution.width();
    clip_to_world.y = line_width / resolution.height();
    clip_to_world.apply_matrix4(&camera.projection_matrix_inverse);
    clip_to_world.multiply_scalar(1.0 / clip_to_world.w);
    clip_to_world.x.max(clip_to_world.y).abs()
}

/// The hit record both branches push: the closest points on the ray and on
/// the (world-space) segment.
fn segment_intersection(ray: &Ray, line: &Line3, i: usize, object: &Node) -> Intersection {
    let mut point = Vector3::ZERO;
    let mut point_on_line = Vector3::ZERO;
    ray.distance_sq_to_segment(
        &line.start,
        &line.end,
        Some(&mut point),
        Some(&mut point_on_line),
    );
    let mut intersection = Intersection::new(ray.origin.distance_to(&point), point, object.clone());
    intersection.point_on_line = Some(point_on_line);
    intersection.face_index = Some(i);
    intersection
}

/// `raycastWorldUnits( lineSegments, intersects )` — the ray against each
/// world-space segment, inside when within half a line width. Unlike every
/// core `raycast()`, it does not check `raycaster.near` / `far`.
fn raycast_world_units(
    positions: &[f32],
    matrix_world: &Matrix4,
    line_width: f64,
    ray: &Ray,
    object: &Node,
    intersects: &mut Vec<Intersection>,
) {
    for i in 0..positions.len() / 6 {
        let (start, end) = segment(positions, i);
        let mut line = Line3 { start, end };
        line.apply_matrix4(matrix_world);

        let intersection = segment_intersection(ray, &line, i, object);
        let point_on_line = intersection.point_on_line.unwrap_or(Vector3::ZERO);
        let is_inside = intersection.point.distance_to(&point_on_line) < line_width * 0.5;
        if is_inside {
            intersects.push(intersection);
        }
    }
}

/// `raycastScreenSpace( lineSegments, camera, intersects )` — each segment
/// clipped to the near plane and projected to pixels, then the pixel under
/// the ray tested against it with the screen-space line width.
#[allow(clippy::too_many_arguments)]
fn raycast_screen_space(
    positions: &[f32],
    matrix_world: &Matrix4,
    resolution: &Vector2,
    line_width: f64,
    camera: &RaycasterCamera,
    ray: &Ray,
    object: &Node,
    intersects: &mut Vec<Intersection>,
) {
    let projection_matrix = &camera.projection_matrix;
    let near = -camera.near;

    // The pixel the ray passes through: one unit along it, projected.
    let at = ray.at(1.0);
    let mut ss_origin = Vector4::new(at.x, at.y, at.z, 1.0);
    ss_origin.apply_matrix4(&camera.matrix_world_inverse);
    ss_origin.apply_matrix4(projection_matrix);
    ss_origin.multiply_scalar(1.0 / ss_origin.w);
    ss_origin.x *= resolution.x / 2.0;
    ss_origin.y *= resolution.y / 2.0;
    ss_origin.z = 0.0;
    let ss_origin3 = Vector3::new(ss_origin.x, ss_origin.y, ss_origin.z);

    let mut mv_matrix = Matrix4::identity();
    mv_matrix.multiply_matrices(&camera.matrix_world_inverse, matrix_world);

    for i in 0..positions.len() / 6 {
        let (start, end) = segment(positions, i);
        let mut start4 = Vector4::new(start.x, start.y, start.z, 1.0);
        let mut end4 = Vector4::new(end.x, end.y, end.z, 1.0);
        start4.apply_matrix4(&mv_matrix);
        end4.apply_matrix4(&mv_matrix);

        // Skip the segment if it is entirely behind the camera's near plane.
        let is_behind_camera_near = start4.z > near && end4.z > near;
        if is_behind_camera_near {
            continue;
        }

        // Trim the segment if it extends behind the camera's near plane.
        if start4.z > near {
            let delta_dist = start4.z - end4.z;
            let t = (start4.z - near) / delta_dist;
            start4.lerp(&end4, t);
        } else if end4.z > near {
            let delta_dist = end4.z - start4.z;
            let t = (end4.z - near) / delta_dist;
            end4.lerp(&start4, t);
        }

        // Clip space, then NDC, then screen space.
        start4.apply_matrix4(projection_matrix);
        end4.apply_matrix4(projection_matrix);
        start4.multiply_scalar(1.0 / start4.w);
        end4.multiply_scalar(1.0 / end4.w);
        start4.x *= resolution.x / 2.0;
        start4.y *= resolution.y / 2.0;
        end4.x *= resolution.x / 2.0;
        end4.y *= resolution.y / 2.0;

        // A 2D segment, to check the pixel against.
        let line = Line3 {
            start: Vector3::new(start4.x, start4.y, 0.0),
            end: Vector3::new(end4.x, end4.y, 0.0),
        };

        // The closest screen-space point on the segment, and whether its
        // depth lies inside the clip volume.
        let param = line.closest_point_to_point_parameter(&ss_origin3, true);
        let closest_point = line.at(param);
        let z_pos = crate::math::math_utils::lerp(start4.z, end4.z, param);
        let is_in_clip_space = (-1.0..=1.0).contains(&z_pos);
        let is_inside = ss_origin3.distance_to(&closest_point) < line_width * 0.5;

        if is_in_clip_space && is_inside {
            let mut line = Line3 { start, end };
            line.apply_matrix4(matrix_world);
            intersects.push(segment_intersection(ray, &line, i, object));
        }
    }
}
