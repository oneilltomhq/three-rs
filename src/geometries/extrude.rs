//! Port of `three.js/src/geometries/ExtrudeGeometry.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry};
use crate::extras::shape_utils;
use crate::extras::{Curve, Shape};
use crate::math::{Vector2, Vector3};

/// `options.UVGenerator`: turns the triangles `ExtrudeGeometry` emits into
/// uvs. `vertices` is the geometry's `verticesArray` so far, still in f64 as
/// in three.js; the indices are vertex (not float) indices into it.
///
/// three.js also passes the geometry under construction as the first
/// argument. `WorldUVGenerator` ignores it and nothing in the port has
/// anything to read from a half-built geometry, so it is left out.
pub trait UvGenerator {
    /// `generateTopUV( geometry, vertices, indexA, indexB, indexC )`.
    fn generate_top_uv(&self, vertices: &[f64], a: usize, b: usize, c: usize) -> [Vector2; 3];

    /// `generateSideWallUV( geometry, vertices, indexA, indexB, indexC, indexD )`.
    fn generate_side_wall_uv(
        &self,
        vertices: &[f64],
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    ) -> [Vector2; 4];
}

/// three.js' `WorldUVGenerator`, the default.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldUvGenerator;

impl UvGenerator for WorldUvGenerator {
    fn generate_top_uv(
        &self,
        vertices: &[f64],
        index_a: usize,
        index_b: usize,
        index_c: usize,
    ) -> [Vector2; 3] {
        let a_x = vertices[index_a * 3];
        let a_y = vertices[index_a * 3 + 1];
        let b_x = vertices[index_b * 3];
        let b_y = vertices[index_b * 3 + 1];
        let c_x = vertices[index_c * 3];
        let c_y = vertices[index_c * 3 + 1];

        [
            Vector2::new(a_x, a_y),
            Vector2::new(b_x, b_y),
            Vector2::new(c_x, c_y),
        ]
    }

    fn generate_side_wall_uv(
        &self,
        vertices: &[f64],
        index_a: usize,
        index_b: usize,
        index_c: usize,
        index_d: usize,
    ) -> [Vector2; 4] {
        let a_x = vertices[index_a * 3];
        let a_y = vertices[index_a * 3 + 1];
        let a_z = vertices[index_a * 3 + 2];
        let b_x = vertices[index_b * 3];
        let b_y = vertices[index_b * 3 + 1];
        let b_z = vertices[index_b * 3 + 2];
        let c_x = vertices[index_c * 3];
        let c_y = vertices[index_c * 3 + 1];
        let c_z = vertices[index_c * 3 + 2];
        let d_x = vertices[index_d * 3];
        let d_y = vertices[index_d * 3 + 1];
        let d_z = vertices[index_d * 3 + 2];

        if (a_y - b_y).abs() < (a_x - b_x).abs() {
            [
                Vector2::new(a_x, 1.0 - a_z),
                Vector2::new(b_x, 1.0 - b_z),
                Vector2::new(c_x, 1.0 - c_z),
                Vector2::new(d_x, 1.0 - d_z),
            ]
        } else {
            [
                Vector2::new(a_y, 1.0 - a_z),
                Vector2::new(b_y, 1.0 - b_z),
                Vector2::new(c_y, 1.0 - c_z),
                Vector2::new(d_y, 1.0 - d_z),
            ]
        }
    }
}

/// The `options` object of `new ExtrudeGeometry( shapes, options )`, with
/// three.js' defaults.
#[derive(Clone)]
pub struct ExtrudeGeometryOptions {
    /// `curveSegments`, default 12: points on the shape's curves.
    pub curve_segments: usize,
    /// `steps`, default 1: subdivisions along the depth (or the path).
    pub steps: usize,
    /// `depth`, default 1.
    pub depth: f64,
    /// `bevelEnabled`, default true.
    pub bevel_enabled: bool,
    /// `bevelThickness`, default 0.2.
    pub bevel_thickness: f64,
    /// `bevelSize`; `None` means three.js' `bevelThickness - 0.1`.
    pub bevel_size: Option<f64>,
    /// `bevelOffset`, default 0.
    pub bevel_offset: f64,
    /// `bevelSegments`, default 3.
    pub bevel_segments: usize,
    /// `extrudePath`: a 3D curve to extrude along. Turns bevels off.
    pub extrude_path: Option<Rc<dyn Curve<Point = Vector3>>>,
    /// `UVGenerator`; `None` is `WorldUVGenerator`.
    pub uv_generator: Option<Rc<dyn UvGenerator>>,
}

impl Default for ExtrudeGeometryOptions {
    fn default() -> Self {
        Self {
            curve_segments: 12,
            steps: 1,
            depth: 1.0,
            bevel_enabled: true,
            bevel_thickness: 0.2,
            bevel_size: None,
            bevel_offset: 0.0,
            bevel_segments: 3,
            extrude_path: None,
            uv_generator: None,
        }
    }
}

impl std::fmt::Debug for ExtrudeGeometryOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtrudeGeometryOptions")
            .field("curve_segments", &self.curve_segments)
            .field("steps", &self.steps)
            .field("depth", &self.depth)
            .field("bevel_enabled", &self.bevel_enabled)
            .field("bevel_thickness", &self.bevel_thickness)
            .field("bevel_size", &self.bevel_size)
            .field("bevel_offset", &self.bevel_offset)
            .field("bevel_segments", &self.bevel_segments)
            .field(
                "extrude_path",
                &self.extrude_path.as_ref().map(|p| p.type_name()),
            )
            .field("uv_generator", &self.uv_generator.is_some())
            .finish()
    }
}

/// The shape `new ExtrudeGeometry()` falls back to: a unit square.
pub fn extrude_geometry_default_shape() -> Shape {
    Shape::from_points(&[
        Vector2::new(0.5, 0.5),
        Vector2::new(-0.5, 0.5),
        Vector2::new(-0.5, -0.5),
        Vector2::new(0.5, -0.5),
    ])
}

/// `new ExtrudeGeometry( shapes, options )`. A single shape is the one-element
/// slice: three.js wraps a lone shape in an array, so both forms build the
/// same geometry (two groups per shape — lids, material 0, then sides,
/// material 1). The result is non-indexed, with `computeVertexNormals`
/// normals.
pub fn extrude_geometry(shapes: &[Shape], options: &ExtrudeGeometryOptions) -> BufferGeometry {
    let mut geometry = BufferGeometry::new();

    let mut vertices_array: Vec<f64> = Vec::new();
    let mut uv_array: Vec<f64> = Vec::new();

    for shape in shapes {
        add_shape(
            &mut geometry,
            &mut vertices_array,
            &mut uv_array,
            shape,
            options,
        );
    }

    geometry.set_attribute(
        "position",
        BufferAttribute::new(vertices_array.iter().map(|&v| v as f32).collect(), 3),
    );
    geometry.set_attribute(
        "uv",
        BufferAttribute::new(uv_array.iter().map(|&v| v as f32).collect(), 2),
    );

    geometry.compute_vertex_normals();

    geometry
}

/// `Math.sign`, which keeps a zero's sign and NaN; compared with `===`, so
/// `-0` equals `0` and NaN equals nothing — as `f64 ==` does.
fn js_sign(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        v
    }
}

/// `Math.max` over several values: NaN if any is.
fn js_max(values: &[f64]) -> f64 {
    let mut m = f64::NEG_INFINITY;
    for &v in values {
        if v.is_nan() {
            return f64::NAN;
        }
        if v > m {
            m = v;
        }
    }
    m
}

fn merge_overlapping_points(points: &mut Vec<Vector2>) {
    const THRESHOLD: f64 = 1e-10;
    const THRESHOLD_SQ: f64 = THRESHOLD * THRESHOLD;

    if points.is_empty() {
        return;
    }

    let mut prev_pos = points[0];

    // `for ( let i = 1; i <= points.length; i ++ )`, re-reading the length
    // after each splice.
    let mut i = 1;
    while i <= points.len() {
        let current_index = i % points.len();
        let current_pos = points[current_index];

        let dx = current_pos.x - prev_pos.x;
        let dy = current_pos.y - prev_pos.y;
        let dist_sq = dx * dx + dy * dy;

        let scaling_factor_sqrt = js_max(&[
            current_pos.x.abs(),
            current_pos.y.abs(),
            prev_pos.x.abs(),
            prev_pos.y.abs(),
        ]);
        let threshold_sq_scaled = THRESHOLD_SQ * scaling_factor_sqrt * scaling_factor_sqrt;

        if dist_sq <= threshold_sq_scaled {
            points.remove(current_index);
            if points.is_empty() {
                // three.js goes on to read `points[ NaN ]` and throws.
                return;
            }
            continue;
        }

        prev_pos = current_pos;
        i += 1;
    }
}

/// `position2.copy( extrudePts[ s ] ).add( normal.copy( N ).multiplyScalar( vert.x ) )
/// .add( binormal.copy( B ).multiplyScalar( vert.y ) )`, in that order.
fn frame_point(p: &Vector3, n: &Vector3, b: &Vector3, vert: &Vector2) -> Vector3 {
    Vector3::new(
        p.x + n.x * vert.x + b.x * vert.y,
        p.y + n.y * vert.x + b.y * vert.y,
        p.z + n.z * vert.x + b.z * vert.y,
    )
}

fn scale_pt2(pt: &Vector2, vec: &Vector2, size: f64) -> Vector2 {
    Vector2::new(pt.x + vec.x * size, pt.y + vec.y * size)
}

fn get_bevel_vec(in_pt: &Vector2, in_prev: &Vector2, in_next: &Vector2) -> Vector2 {
    // computes for inPt the corresponding point inPt' on a new contour
    //   shifted by 1 unit (length of normalized vector) to the left
    // if we walk along contour clockwise, this new contour is outside the old one
    //
    // inPt' is the intersection of the two lines parallel to the two
    //  adjacent edges of inPt at a distance of 1 unit on the left side.

    let v_trans_x;
    let v_trans_y;
    let shrink_by; // resulting translation vector for inPt

    // good reading for geometry algorithms (here: line-line intersection)
    // http://geomalgorithms.com/a05-_intersect-1.html

    let v_prev_x = in_pt.x - in_prev.x;
    let v_prev_y = in_pt.y - in_prev.y;
    let v_next_x = in_next.x - in_pt.x;
    let v_next_y = in_next.y - in_pt.y;

    let v_prev_lensq = v_prev_x * v_prev_x + v_prev_y * v_prev_y;

    // check for collinear edges
    let collinear0 = v_prev_x * v_next_y - v_prev_y * v_next_x;

    if collinear0.abs() > f64::EPSILON {
        // not collinear

        // length of vectors for normalizing

        let v_prev_len = v_prev_lensq.sqrt();
        let v_next_len = (v_next_x * v_next_x + v_next_y * v_next_y).sqrt();

        // shift adjacent points by unit vectors to the left

        let pt_prev_shift_x = in_prev.x - v_prev_y / v_prev_len;
        let pt_prev_shift_y = in_prev.y + v_prev_x / v_prev_len;

        let pt_next_shift_x = in_next.x - v_next_y / v_next_len;
        let pt_next_shift_y = in_next.y + v_next_x / v_next_len;

        // scaling factor for v_prev to intersection point

        let sf = ((pt_next_shift_x - pt_prev_shift_x) * v_next_y
            - (pt_next_shift_y - pt_prev_shift_y) * v_next_x)
            / (v_prev_x * v_next_y - v_prev_y * v_next_x);

        // vector from inPt to intersection point

        v_trans_x = pt_prev_shift_x + v_prev_x * sf - in_pt.x;
        v_trans_y = pt_prev_shift_y + v_prev_y * sf - in_pt.y;

        // Don't normalize!, otherwise sharp corners become ugly
        //  but prevent crazy spikes
        let v_trans_lensq = v_trans_x * v_trans_x + v_trans_y * v_trans_y;
        if v_trans_lensq <= 2.0 {
            return Vector2::new(v_trans_x, v_trans_y);
        } else {
            shrink_by = (v_trans_lensq / 2.0).sqrt();
        }
    } else {
        // handle special case of collinear edges

        let mut direction_eq = false; // assumes: opposite

        if v_prev_x > f64::EPSILON {
            if v_next_x > f64::EPSILON {
                direction_eq = true;
            }
        } else if v_prev_x < -f64::EPSILON {
            if v_next_x < -f64::EPSILON {
                direction_eq = true;
            }
        } else if js_sign(v_prev_y) == js_sign(v_next_y) {
            direction_eq = true;
        }

        if direction_eq {
            v_trans_x = -v_prev_y;
            v_trans_y = v_prev_x;
            shrink_by = v_prev_lensq.sqrt();
        } else {
            v_trans_x = v_prev_x;
            v_trans_y = v_prev_y;
            shrink_by = (v_prev_lensq / 2.0).sqrt();
        }
    }

    Vector2::new(v_trans_x / shrink_by, v_trans_y / shrink_by)
}

/// Bevel movements for one closed contour, `getBevelVec( pt[ i ], pt[ i-1 ], pt[ i+1 ] )`.
fn contour_movements(contour: &[Vector2]) -> Vec<Vector2> {
    let il = contour.len();
    let mut movements = Vec::with_capacity(il);
    for i in 0..il {
        let j = if i == 0 { il - 1 } else { i - 1 };
        let k = if i + 1 == il { 0 } else { i + 1 };
        movements.push(get_bevel_vec(&contour[i], &contour[j], &contour[k]));
    }
    movements
}

// Index loops kept as three.js writes them, so the port reads line for line.
#[allow(clippy::needless_range_loop)]
fn add_shape(
    geometry: &mut BufferGeometry,
    vertices_array: &mut Vec<f64>,
    uv_array: &mut Vec<f64>,
    shape: &Shape,
    options: &ExtrudeGeometryOptions,
) {
    let mut placeholder: Vec<f64> = Vec::new();

    let curve_segments = options.curve_segments;
    let steps = options.steps;
    let depth = options.depth;

    let mut bevel_enabled = options.bevel_enabled;
    let mut bevel_thickness = options.bevel_thickness;
    let mut bevel_size = options.bevel_size.unwrap_or(bevel_thickness - 0.1);
    let mut bevel_offset = options.bevel_offset;
    let mut bevel_segments = options.bevel_segments;

    let world_uv = WorldUvGenerator;
    let uvgen: &dyn UvGenerator = match &options.uv_generator {
        Some(g) => g.as_ref(),
        None => &world_uv,
    };

    let mut extrude_pts: Vec<Vector3> = Vec::new();
    let mut spline_tube = None;

    if let Some(extrude_path) = &options.extrude_path {
        extrude_pts = extrude_path.get_spaced_points(steps);

        bevel_enabled = false; // bevels not supported for path extrusion

        // SETUP TNB variables

        let is_closed = extrude_path.is_closed_catmull_rom();
        spline_tube = Some(extrude_path.compute_frenet_frames(steps, is_closed));
    }
    let extrude_by_path = options.extrude_path.is_some();

    // Safeguards if bevels are not enabled

    if !bevel_enabled {
        bevel_segments = 0;
        bevel_thickness = 0.0;
        bevel_size = 0.0;
        bevel_offset = 0.0;
    }

    // Variables initialization

    let shape_points = shape.extract_points(curve_segments);

    let mut vertices = shape_points.shape;
    let mut holes = shape_points.holes;

    let reverse = !shape_utils::is_clock_wise(&vertices);

    if reverse {
        vertices.reverse();

        // Maybe we should also check if holes are in the opposite direction, just to be safe ...

        for ahole in holes.iter_mut() {
            if shape_utils::is_clock_wise(ahole) {
                ahole.reverse();
            }
        }
    }

    merge_overlapping_points(&mut vertices);
    for hole in holes.iter_mut() {
        merge_overlapping_points(hole);
    }

    let num_holes = holes.len();

    // `contour` and `vertices` are one array in three.js until `vertices` is
    // reassigned to the concatenation; `triangulateShape` may later shorten
    // `contour` (not `vertices`) by popping a repeated end point.
    let mut contour = vertices.clone(); // vertices has all points but contour has only points of circumference

    for ahole in &holes {
        vertices.extend_from_slice(ahole);
    }

    let vlen = vertices.len();

    //

    let contour_movements = contour_movements(&contour);

    let mut holes_movements: Vec<Vec<Vector2>> = Vec::new();
    let mut vertices_movements = contour_movements.clone();

    for ahole in &holes {
        let one_hole_movements = self::contour_movements(ahole);
        vertices_movements.extend_from_slice(&one_hole_movements);
        holes_movements.push(one_hole_movements);
    }

    let mut v = |x: f64, y: f64, z: f64| {
        placeholder.push(x);
        placeholder.push(y);
        placeholder.push(z);
    };

    let faces = if bevel_segments == 0 {
        shape_utils::triangulate_shape(&mut contour, &mut holes)
    } else {
        let mut contracted_contour_vertices = Vec::new();
        let mut expanded_hole_vertices = Vec::new();

        // Loop bevelSegments, 1 for the front, 1 for the back

        for b in 0..bevel_segments {
            let t = b as f64 / bevel_segments as f64;
            let z = bevel_thickness * (t * std::f64::consts::PI / 2.0).cos();
            let bs = bevel_size * (t * std::f64::consts::PI / 2.0).sin() + bevel_offset;

            // contract shape

            for i in 0..contour.len() {
                let vert = scale_pt2(&contour[i], &contour_movements[i], bs);

                v(vert.x, vert.y, -z);
                if t == 0.0 {
                    contracted_contour_vertices.push(vert);
                }
            }

            // expand holes

            for h in 0..num_holes {
                let ahole = &holes[h];
                let one_hole_movements = &holes_movements[h];
                let mut one_hole_vertices = Vec::new();
                for i in 0..ahole.len() {
                    let vert = scale_pt2(&ahole[i], &one_hole_movements[i], bs);

                    v(vert.x, vert.y, -z);
                    if t == 0.0 {
                        one_hole_vertices.push(vert);
                    }
                }

                if t == 0.0 {
                    expanded_hole_vertices.push(one_hole_vertices);
                }
            }
        }

        shape_utils::triangulate_shape(
            &mut contracted_contour_vertices,
            &mut expanded_hole_vertices,
        )
    };

    let bs = bevel_size + bevel_offset;

    // Back facing vertices

    for i in 0..vlen {
        let vert = if bevel_enabled {
            scale_pt2(&vertices[i], &vertices_movements[i], bs)
        } else {
            vertices[i]
        };

        match &spline_tube {
            None => v(vert.x, vert.y, 0.0),
            Some(spline_tube) => {
                // v( vert.x, vert.y + extrudePts[ 0 ].y, extrudePts[ 0 ].x );

                let position2 = frame_point(
                    &extrude_pts[0],
                    &spline_tube.normals[0],
                    &spline_tube.binormals[0],
                    &vert,
                );

                v(position2.x, position2.y, position2.z);
            }
        }
    }

    // Add stepped vertices...
    // Including front facing vertices

    for s in 1..=steps {
        for i in 0..vlen {
            let vert = if bevel_enabled {
                scale_pt2(&vertices[i], &vertices_movements[i], bs)
            } else {
                vertices[i]
            };

            match &spline_tube {
                None => v(vert.x, vert.y, depth / steps as f64 * s as f64),
                Some(spline_tube) => {
                    // v( vert.x, vert.y + extrudePts[ s - 1 ].y, extrudePts[ s - 1 ].x );

                    let position2 = frame_point(
                        &extrude_pts[s],
                        &spline_tube.normals[s],
                        &spline_tube.binormals[s],
                        &vert,
                    );

                    v(position2.x, position2.y, position2.z);
                }
            }
        }
    }

    // Add bevel segments planes

    for b in (0..bevel_segments).rev() {
        let t = b as f64 / bevel_segments as f64;
        let z = bevel_thickness * (t * std::f64::consts::PI / 2.0).cos();
        let bs = bevel_size * (t * std::f64::consts::PI / 2.0).sin() + bevel_offset;

        // contract shape

        for i in 0..contour.len() {
            let vert = scale_pt2(&contour[i], &contour_movements[i], bs);
            v(vert.x, vert.y, depth + z);
        }

        // expand holes

        for h in 0..holes.len() {
            let ahole = &holes[h];
            let one_hole_movements = &holes_movements[h];

            for i in 0..ahole.len() {
                let vert = scale_pt2(&ahole[i], &one_hole_movements[i], bs);

                if !extrude_by_path {
                    v(vert.x, vert.y, depth + z);
                } else {
                    // Unreachable in three.js too: a path turns bevels off.
                    v(
                        vert.x,
                        vert.y + extrude_pts[steps - 1].y,
                        extrude_pts[steps - 1].x + z,
                    );
                }
            }
        }
    }

    // `f3` / `f4` append to `verticesArray` from `placeholder` and ask the
    // uv generator about what they appended.
    let add_vertex = |vertices_array: &mut Vec<f64>, index: usize| {
        vertices_array.push(placeholder[index * 3]);
        vertices_array.push(placeholder[index * 3 + 1]);
        vertices_array.push(placeholder[index * 3 + 2]);
    };

    let add_uv = |uv_array: &mut Vec<f64>, vector2: &Vector2| {
        uv_array.push(vector2.x);
        uv_array.push(vector2.y);
    };

    let f3 = |vertices_array: &mut Vec<f64>, uv_array: &mut Vec<f64>, a, b, c| {
        add_vertex(vertices_array, a);
        add_vertex(vertices_array, b);
        add_vertex(vertices_array, c);

        let next_index = vertices_array.len() / 3;
        let uvs = uvgen.generate_top_uv(
            vertices_array,
            next_index - 3,
            next_index - 2,
            next_index - 1,
        );

        add_uv(uv_array, &uvs[0]);
        add_uv(uv_array, &uvs[1]);
        add_uv(uv_array, &uvs[2]);
    };

    let f4 = |vertices_array: &mut Vec<f64>, uv_array: &mut Vec<f64>, a, b, c, d| {
        add_vertex(vertices_array, a);
        add_vertex(vertices_array, b);
        add_vertex(vertices_array, d);

        add_vertex(vertices_array, b);
        add_vertex(vertices_array, c);
        add_vertex(vertices_array, d);

        let next_index = vertices_array.len() / 3;
        let uvs = uvgen.generate_side_wall_uv(
            vertices_array,
            next_index - 6,
            next_index - 3,
            next_index - 2,
            next_index - 1,
        );

        add_uv(uv_array, &uvs[0]);
        add_uv(uv_array, &uvs[1]);
        add_uv(uv_array, &uvs[3]);

        add_uv(uv_array, &uvs[1]);
        add_uv(uv_array, &uvs[2]);
        add_uv(uv_array, &uvs[3]);
    };

    /* Faces */

    // Top and bottom faces

    // buildLidFaces
    {
        let start = vertices_array.len() / 3;

        if bevel_enabled {
            let mut layer = 0; // steps + 1
            let mut offset = vlen * layer;

            // Bottom faces

            for face in &faces {
                f3(
                    vertices_array,
                    uv_array,
                    face[2] + offset,
                    face[1] + offset,
                    face[0] + offset,
                );
            }

            layer = steps + bevel_segments * 2;
            offset = vlen * layer;

            // Top faces

            for face in &faces {
                f3(
                    vertices_array,
                    uv_array,
                    face[0] + offset,
                    face[1] + offset,
                    face[2] + offset,
                );
            }
        } else {
            // Bottom faces

            for face in &faces {
                f3(vertices_array, uv_array, face[2], face[1], face[0]);
            }

            // Top faces

            for face in &faces {
                f3(
                    vertices_array,
                    uv_array,
                    face[0] + vlen * steps,
                    face[1] + vlen * steps,
                    face[2] + vlen * steps,
                );
            }
        }

        geometry.add_group(start, vertices_array.len() / 3 - start, 0);
    }

    // Create faces for the z-sides of the shape

    // buildSideFaces
    {
        let start = vertices_array.len() / 3;
        let mut layeroffset = 0;

        let sidewalls = |vertices_array: &mut Vec<f64>,
                         uv_array: &mut Vec<f64>,
                         contour: &[Vector2],
                         layeroffset: usize| {
            let mut i = contour.len();

            while i > 0 {
                i -= 1;
                let j = i;
                let k = if i == 0 { contour.len() - 1 } else { i - 1 };

                for s in 0..(steps + bevel_segments * 2) {
                    let slen1 = vlen * s;
                    let slen2 = vlen * (s + 1);

                    let a = layeroffset + j + slen1;
                    let b = layeroffset + k + slen1;
                    let c = layeroffset + k + slen2;
                    let d = layeroffset + j + slen2;

                    f4(vertices_array, uv_array, a, b, c, d);
                }
            }
        };

        sidewalls(vertices_array, uv_array, &contour, layeroffset);
        layeroffset += contour.len();

        for ahole in &holes {
            sidewalls(vertices_array, uv_array, ahole, layeroffset);
            layeroffset += ahole.len();
        }

        geometry.add_group(start, vertices_array.len() / 3 - start, 1);
    }
}
