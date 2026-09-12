//! Port of `three.js/src/geometries/PolyhedronGeometry.js` and the four
//! platonic solids built on it (`Icosahedron`, `Octahedron`, `Tetrahedron`,
//! `Dodecahedron`).

use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

/// `new PolyhedronGeometry( vertices, indices, radius, detail )`.
pub fn polyhedron_geometry(
    vertices: &[f64],
    indices: &[usize],
    radius: f64,
    detail: usize,
) -> BufferGeometry {
    // default buffer data
    //
    // three.js keeps these as plain JS arrays (f64) through the whole
    // construction and only narrows to f32 in the Float32BufferAttribute.

    let mut vertex_buffer: Vec<f64> = Vec::new();
    let mut uv_buffer: Vec<f64> = Vec::new();

    // the subdivision creates the vertex buffer data

    subdivide(vertices, indices, detail, &mut vertex_buffer);

    // all vertices should lie on a conceptual sphere with a given radius

    apply_radius(radius, &mut vertex_buffer);

    // finally, create the uv data

    generate_uvs(&vertex_buffer, &mut uv_buffer);

    // build non-indexed geometry

    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(vertex_buffer.iter().map(|&v| v as f32).collect(), 3),
    );
    geometry.set_attribute(
        "normal",
        BufferAttribute::new(vertex_buffer.iter().map(|&v| v as f32).collect(), 3),
    );
    geometry.set_attribute(
        "uv",
        BufferAttribute::new(uv_buffer.iter().map(|&v| v as f32).collect(), 2),
    );

    if detail == 0 {
        geometry.compute_vertex_normals(); // flat normals
    } else {
        geometry.normalize_normals(); // smooth normals
    }

    geometry
}

fn get_vertex_by_index(vertices: &[f64], index: usize, vertex: &mut Vector3) {
    let stride = index * 3;

    vertex.x = vertices[stride];
    vertex.y = vertices[stride + 1];
    vertex.z = vertices[stride + 2];
}

fn push_vertex(vertex_buffer: &mut Vec<f64>, vertex: &Vector3) {
    vertex_buffer.push(vertex.x);
    vertex_buffer.push(vertex.y);
    vertex_buffer.push(vertex.z);
}

fn subdivide(vertices: &[f64], indices: &[usize], detail: usize, vertex_buffer: &mut Vec<f64>) {
    let mut a = Vector3::ZERO;
    let mut b = Vector3::ZERO;
    let mut c = Vector3::ZERO;

    // iterate over all faces and apply a subdivision with the given detail value

    let mut i = 0;
    while i < indices.len() {
        // get the vertices of the face

        get_vertex_by_index(vertices, indices[i], &mut a);
        get_vertex_by_index(vertices, indices[i + 1], &mut b);
        get_vertex_by_index(vertices, indices[i + 2], &mut c);

        // perform subdivision

        subdivide_face(&a, &b, &c, detail, vertex_buffer);

        i += 3;
    }
}

fn subdivide_face(
    a: &Vector3,
    b: &Vector3,
    c: &Vector3,
    detail: usize,
    vertex_buffer: &mut Vec<f64>,
) {
    let cols = detail + 1;

    // we use this multidimensional array as a data structure for creating the subdivision

    let mut v: Vec<Vec<Vector3>> = Vec::new();

    // construct all of the vertices for this subdivision

    for i in 0..=cols {
        v.push(Vec::new());

        let mut aj = *a;
        aj.lerp(c, i as f64 / cols as f64);
        let mut bj = *b;
        bj.lerp(c, i as f64 / cols as f64);

        let rows = cols - i;

        for j in 0..=rows {
            if j == 0 && i == cols {
                v[i].push(aj);
            } else {
                let mut p = aj;
                p.lerp(&bj, j as f64 / rows as f64);
                v[i].push(p);
            }
        }
    }

    // construct all of the faces

    for i in 0..cols {
        for j in 0..(2 * (cols - i) - 1) {
            let k = j / 2;

            if j % 2 == 0 {
                push_vertex(vertex_buffer, &v[i][k + 1]);
                push_vertex(vertex_buffer, &v[i + 1][k]);
                push_vertex(vertex_buffer, &v[i][k]);
            } else {
                push_vertex(vertex_buffer, &v[i][k + 1]);
                push_vertex(vertex_buffer, &v[i + 1][k + 1]);
                push_vertex(vertex_buffer, &v[i + 1][k]);
            }
        }
    }
}

fn apply_radius(radius: f64, vertex_buffer: &mut [f64]) {
    let mut vertex = Vector3::ZERO;

    // iterate over the entire buffer and apply the radius to each vertex

    let mut i = 0;
    while i < vertex_buffer.len() {
        vertex.x = vertex_buffer[i];
        vertex.y = vertex_buffer[i + 1];
        vertex.z = vertex_buffer[i + 2];

        vertex.normalize().multiply_scalar(radius);

        vertex_buffer[i] = vertex.x;
        vertex_buffer[i + 1] = vertex.y;
        vertex_buffer[i + 2] = vertex.z;

        i += 3;
    }
}

/// Angle around the Y axis, counter-clockwise when looking from above.
fn azimuth(vector: &Vector3) -> f64 {
    vector.z.atan2(-vector.x)
}

/// Angle above the XZ plane.
fn inclination(vector: &Vector3) -> f64 {
    (-vector.y).atan2((vector.x * vector.x + vector.z * vector.z).sqrt())
}

fn generate_uvs(vertex_buffer: &[f64], uv_buffer: &mut Vec<f64>) {
    let mut vertex = Vector3::ZERO;

    let mut i = 0;
    while i < vertex_buffer.len() {
        vertex.x = vertex_buffer[i];
        vertex.y = vertex_buffer[i + 1];
        vertex.z = vertex_buffer[i + 2];

        let u = azimuth(&vertex) / 2.0 / std::f64::consts::PI + 0.5;
        let v = inclination(&vertex) / std::f64::consts::PI + 0.5;
        uv_buffer.push(u);
        uv_buffer.push(1.0 - v);

        i += 3;
    }

    correct_uvs(vertex_buffer, uv_buffer);

    correct_seam(uv_buffer);
}

fn correct_uvs(vertex_buffer: &[f64], uv_buffer: &mut [f64]) {
    let mut a = Vector3::ZERO;
    let mut b = Vector3::ZERO;
    let mut c = Vector3::ZERO;

    let mut centroid;

    let mut i = 0;
    let mut j = 0;
    while i < vertex_buffer.len() {
        a.set(vertex_buffer[i], vertex_buffer[i + 1], vertex_buffer[i + 2]);
        b.set(
            vertex_buffer[i + 3],
            vertex_buffer[i + 4],
            vertex_buffer[i + 5],
        );
        c.set(
            vertex_buffer[i + 6],
            vertex_buffer[i + 7],
            vertex_buffer[i + 8],
        );

        let uv_a_x = uv_buffer[j];
        let uv_b_x = uv_buffer[j + 2];
        let uv_c_x = uv_buffer[j + 4];

        centroid = a;
        centroid.add(&b).add(&c).divide_scalar(3.0);

        let azi = azimuth(&centroid);

        correct_uv(uv_buffer, uv_a_x, j, &a, azi);
        correct_uv(uv_buffer, uv_b_x, j + 2, &b, azi);
        correct_uv(uv_buffer, uv_c_x, j + 4, &c, azi);

        i += 9;
        j += 6;
    }
}

fn correct_uv(uv_buffer: &mut [f64], uv_x: f64, stride: usize, vector: &Vector3, azimuth: f64) {
    if (azimuth < 0.0) && (uv_x == 1.0) {
        uv_buffer[stride] = uv_x - 1.0;
    }

    if (vector.x == 0.0) && (vector.z == 0.0) {
        uv_buffer[stride] = azimuth / 2.0 / std::f64::consts::PI + 0.5;
    }
}

fn correct_seam(uv_buffer: &mut [f64]) {
    // handle case when face straddles the seam, see #3269

    let mut i = 0;
    while i < uv_buffer.len() {
        // uv data of a single face

        let x0 = uv_buffer[i];
        let x1 = uv_buffer[i + 2];
        let x2 = uv_buffer[i + 4];

        let max = x0.max(x1).max(x2);
        let min = x0.min(x1).min(x2);

        // 0.9 is somewhat arbitrary

        if max > 0.9 && min < 0.1 {
            if x0 < 0.2 {
                uv_buffer[i] += 1.0;
            }
            if x1 < 0.2 {
                uv_buffer[i + 2] += 1.0;
            }
            if x2 < 0.2 {
                uv_buffer[i + 4] += 1.0;
            }
        }

        i += 6;
    }
}

/// `new IcosahedronGeometry( radius, detail )`.
pub fn icosahedron_geometry(radius: f64, detail: usize) -> BufferGeometry {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;

    let vertices = [
        -1.0, t, 0.0, 1.0, t, 0.0, -1.0, -t, 0.0, 1.0, -t, 0.0, //
        0.0, -1.0, t, 0.0, 1.0, t, 0.0, -1.0, -t, 0.0, 1.0, -t, //
        t, 0.0, -1.0, t, 0.0, 1.0, -t, 0.0, -1.0, -t, 0.0, 1.0,
    ];

    let indices = [
        0, 11, 5, 0, 5, 1, 0, 1, 7, 0, 7, 10, 0, 10, 11, //
        1, 5, 9, 5, 11, 4, 11, 10, 2, 10, 7, 6, 7, 1, 8, //
        3, 9, 4, 3, 4, 2, 3, 2, 6, 3, 6, 8, 3, 8, 9, //
        4, 9, 5, 2, 4, 11, 6, 2, 10, 8, 6, 7, 9, 8, 1,
    ];

    polyhedron_geometry(&vertices, &indices, radius, detail)
}

/// `new OctahedronGeometry( radius, detail )`.
pub fn octahedron_geometry(radius: f64, detail: usize) -> BufferGeometry {
    let vertices = [
        1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, //
        0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0,
    ];

    let indices = [
        0, 2, 4, 0, 4, 3, 0, 3, 5, //
        0, 5, 2, 1, 2, 5, 1, 5, 3, //
        1, 3, 4, 1, 4, 2,
    ];

    polyhedron_geometry(&vertices, &indices, radius, detail)
}

/// `new TetrahedronGeometry( radius, detail )`.
pub fn tetrahedron_geometry(radius: f64, detail: usize) -> BufferGeometry {
    let vertices = [
        1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0,
    ];

    let indices = [2, 1, 0, 0, 3, 2, 1, 3, 0, 2, 3, 1];

    polyhedron_geometry(&vertices, &indices, radius, detail)
}

/// `new DodecahedronGeometry( radius, detail )`.
pub fn dodecahedron_geometry(radius: f64, detail: usize) -> BufferGeometry {
    let t = (1.0 + 5.0f64.sqrt()) / 2.0;
    let r = 1.0 / t;

    let vertices = [
        // (±1, ±1, ±1)
        -1.0, -1.0, -1.0, -1.0, -1.0, 1.0, //
        -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, //
        1.0, -1.0, -1.0, 1.0, -1.0, 1.0, //
        1.0, 1.0, -1.0, 1.0, 1.0, 1.0, //
        // (0, ±1/φ, ±φ)
        0.0, -r, -t, 0.0, -r, t, //
        0.0, r, -t, 0.0, r, t, //
        // (±1/φ, ±φ, 0)
        -r, -t, 0.0, -r, t, 0.0, //
        r, -t, 0.0, r, t, 0.0, //
        // (±φ, 0, ±1/φ)
        -t, 0.0, -r, t, 0.0, -r, //
        -t, 0.0, r, t, 0.0, r,
    ];

    let indices = [
        3, 11, 7, 3, 7, 15, 3, 15, 13, //
        7, 19, 17, 7, 17, 6, 7, 6, 15, //
        17, 4, 8, 17, 8, 10, 17, 10, 6, //
        8, 0, 16, 8, 16, 2, 8, 2, 10, //
        0, 12, 1, 0, 1, 18, 0, 18, 16, //
        6, 10, 2, 6, 2, 13, 6, 13, 15, //
        2, 16, 18, 2, 18, 3, 2, 3, 13, //
        18, 1, 9, 18, 9, 11, 18, 11, 3, //
        4, 14, 12, 4, 12, 0, 4, 0, 8, //
        11, 9, 5, 11, 5, 19, 11, 19, 7, //
        19, 5, 14, 19, 14, 4, 19, 4, 17, //
        1, 12, 14, 1, 14, 5, 1, 5, 9,
    ];

    polyhedron_geometry(&vertices, &indices, radius, detail)
}
