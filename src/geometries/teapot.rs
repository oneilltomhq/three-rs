//! Port of the addon `three.js/examples/jsm/geometries/TeapotGeometry.js`.
//!
//! Tessellates the Utah teapot database by Martin Newell into triangles.
//! `webgpu_lights_phong` builds `new TeapotGeometry( .8, 18 )` and
//! `webgpu_materials` `new TeapotGeometry( 50, 18 )`, both leaving
//! `bottom`/`lid`/`body`/`fitLid`/`blinn` at their defaults (all `true`).
//!
//! The control-point tables below are copied verbatim out of the addon.

use crate::core::{BufferAttribute, BufferGeometry, Index};
use crate::geometries::math_extras::{matrix4_set, matrix4_transpose, Vector4};
use crate::math::{Matrix4, Vector3};

/// `teapotPatches` from `TeapotGeometry.js`, 32 * 4 * 4 Bezier spline patches.
const TEAPOT_PATCHES: [usize; 512] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    3, 16, 17, 18, 7, 19, 20, 21, 11, 22, 23, 24, 15, 25, 26, 27,
    18, 28, 29, 30, 21, 31, 32, 33, 24, 34, 35, 36, 27, 37, 38, 39,
    30, 40, 41, 0, 33, 42, 43, 4, 36, 44, 45, 8, 39, 46, 47, 12,
    12, 13, 14, 15, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
    15, 25, 26, 27, 51, 60, 61, 62, 55, 63, 64, 65, 59, 66, 67, 68,
    27, 37, 38, 39, 62, 69, 70, 71, 65, 72, 73, 74, 68, 75, 76, 77,
    39, 46, 47, 12, 71, 78, 79, 48, 74, 80, 81, 52, 77, 82, 83, 56,
    56, 57, 58, 59, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
    59, 66, 67, 68, 87, 96, 97, 98, 91, 99, 100, 101, 95, 102, 103, 104,
    68, 75, 76, 77, 98, 105, 106, 107, 101, 108, 109, 110, 104, 111, 112, 113,
    77, 82, 83, 56, 107, 114, 115, 84, 110, 116, 117, 88, 113, 118, 119, 92,
    120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135,
    123, 136, 137, 120, 127, 138, 139, 124, 131, 140, 141, 128, 135, 142, 143, 132,
    132, 133, 134, 135, 144, 145, 146, 147, 148, 149, 150, 151, 68, 152, 153, 154,
    135, 142, 143, 132, 147, 155, 156, 144, 151, 157, 158, 148, 154, 159, 160, 68,
    161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176,
    164, 177, 178, 161, 168, 179, 180, 165, 172, 181, 182, 169, 176, 183, 184, 173,
    173, 174, 175, 176, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196,
    176, 183, 184, 173, 188, 197, 198, 185, 192, 199, 200, 189, 196, 201, 202, 193,
    203, 203, 203, 203, 204, 205, 206, 207, 208, 208, 208, 208, 209, 210, 211, 212,
    203, 203, 203, 203, 207, 213, 214, 215, 208, 208, 208, 208, 212, 216, 217, 218,
    203, 203, 203, 203, 215, 219, 220, 221, 208, 208, 208, 208, 218, 222, 223, 224,
    203, 203, 203, 203, 221, 225, 226, 204, 208, 208, 208, 208, 224, 227, 228, 209,
    209, 210, 211, 212, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240,
    212, 216, 217, 218, 232, 241, 242, 243, 236, 244, 245, 246, 240, 247, 248, 249,
    218, 222, 223, 224, 243, 250, 251, 252, 246, 253, 254, 255, 249, 256, 257, 258,
    224, 227, 228, 209, 252, 259, 260, 229, 255, 261, 262, 233, 258, 263, 264, 237,
    265, 265, 265, 265, 266, 267, 268, 269, 270, 271, 272, 273, 92, 119, 118, 113,
    265, 265, 265, 265, 269, 274, 275, 276, 273, 277, 278, 279, 113, 112, 111, 104,
    265, 265, 265, 265, 276, 280, 281, 282, 279, 283, 284, 285, 104, 103, 102, 95,
    265, 265, 265, 265, 282, 286, 287, 266, 285, 288, 289, 270, 95, 94, 93, 92,
];

/// `teapotVertices` from `TeapotGeometry.js`, 290 control points.
const TEAPOT_VERTICES: [f64; 870] = [
    1.4, 0.0, 2.4,
    1.4, -0.784, 2.4,
    0.784, -1.4, 2.4,
    0.0, -1.4, 2.4,
    1.3375, 0.0, 2.53125,
    1.3375, -0.749, 2.53125,
    0.749, -1.3375, 2.53125,
    0.0, -1.3375, 2.53125,
    1.4375, 0.0, 2.53125,
    1.4375, -0.805, 2.53125,
    0.805, -1.4375, 2.53125,
    0.0, -1.4375, 2.53125,
    1.5, 0.0, 2.4,
    1.5, -0.84, 2.4,
    0.84, -1.5, 2.4,
    0.0, -1.5, 2.4,
    -0.784, -1.4, 2.4,
    -1.4, -0.784, 2.4,
    -1.4, 0.0, 2.4,
    -0.749, -1.3375, 2.53125,
    -1.3375, -0.749, 2.53125,
    -1.3375, 0.0, 2.53125,
    -0.805, -1.4375, 2.53125,
    -1.4375, -0.805, 2.53125,
    -1.4375, 0.0, 2.53125,
    -0.84, -1.5, 2.4,
    -1.5, -0.84, 2.4,
    -1.5, 0.0, 2.4,
    -1.4, 0.784, 2.4,
    -0.784, 1.4, 2.4,
    0.0, 1.4, 2.4,
    -1.3375, 0.749, 2.53125,
    -0.749, 1.3375, 2.53125,
    0.0, 1.3375, 2.53125,
    -1.4375, 0.805, 2.53125,
    -0.805, 1.4375, 2.53125,
    0.0, 1.4375, 2.53125,
    -1.5, 0.84, 2.4,
    -0.84, 1.5, 2.4,
    0.0, 1.5, 2.4,
    0.784, 1.4, 2.4,
    1.4, 0.784, 2.4,
    0.749, 1.3375, 2.53125,
    1.3375, 0.749, 2.53125,
    0.805, 1.4375, 2.53125,
    1.4375, 0.805, 2.53125,
    0.84, 1.5, 2.4,
    1.5, 0.84, 2.4,
    1.75, 0.0, 1.875,
    1.75, -0.98, 1.875,
    0.98, -1.75, 1.875,
    0.0, -1.75, 1.875,
    2.0, 0.0, 1.35,
    2.0, -1.12, 1.35,
    1.12, -2.0, 1.35,
    0.0, -2.0, 1.35,
    2.0, 0.0, 0.9,
    2.0, -1.12, 0.9,
    1.12, -2.0, 0.9,
    0.0, -2.0, 0.9,
    -0.98, -1.75, 1.875,
    -1.75, -0.98, 1.875,
    -1.75, 0.0, 1.875,
    -1.12, -2.0, 1.35,
    -2.0, -1.12, 1.35,
    -2.0, 0.0, 1.35,
    -1.12, -2.0, 0.9,
    -2.0, -1.12, 0.9,
    -2.0, 0.0, 0.9,
    -1.75, 0.98, 1.875,
    -0.98, 1.75, 1.875,
    0.0, 1.75, 1.875,
    -2.0, 1.12, 1.35,
    -1.12, 2.0, 1.35,
    0.0, 2.0, 1.35,
    -2.0, 1.12, 0.9,
    -1.12, 2.0, 0.9,
    0.0, 2.0, 0.9,
    0.98, 1.75, 1.875,
    1.75, 0.98, 1.875,
    1.12, 2.0, 1.35,
    2.0, 1.12, 1.35,
    1.12, 2.0, 0.9,
    2.0, 1.12, 0.9,
    2.0, 0.0, 0.45,
    2.0, -1.12, 0.45,
    1.12, -2.0, 0.45,
    0.0, -2.0, 0.45,
    1.5, 0.0, 0.225,
    1.5, -0.84, 0.225,
    0.84, -1.5, 0.225,
    0.0, -1.5, 0.225,
    1.5, 0.0, 0.15,
    1.5, -0.84, 0.15,
    0.84, -1.5, 0.15,
    0.0, -1.5, 0.15,
    -1.12, -2.0, 0.45,
    -2.0, -1.12, 0.45,
    -2.0, 0.0, 0.45,
    -0.84, -1.5, 0.225,
    -1.5, -0.84, 0.225,
    -1.5, 0.0, 0.225,
    -0.84, -1.5, 0.15,
    -1.5, -0.84, 0.15,
    -1.5, 0.0, 0.15,
    -2.0, 1.12, 0.45,
    -1.12, 2.0, 0.45,
    0.0, 2.0, 0.45,
    -1.5, 0.84, 0.225,
    -0.84, 1.5, 0.225,
    0.0, 1.5, 0.225,
    -1.5, 0.84, 0.15,
    -0.84, 1.5, 0.15,
    0.0, 1.5, 0.15,
    1.12, 2.0, 0.45,
    2.0, 1.12, 0.45,
    0.84, 1.5, 0.225,
    1.5, 0.84, 0.225,
    0.84, 1.5, 0.15,
    1.5, 0.84, 0.15,
    -1.6, 0.0, 2.025,
    -1.6, -0.3, 2.025,
    -1.5, -0.3, 2.25,
    -1.5, 0.0, 2.25,
    -2.3, 0.0, 2.025,
    -2.3, -0.3, 2.025,
    -2.5, -0.3, 2.25,
    -2.5, 0.0, 2.25,
    -2.7, 0.0, 2.025,
    -2.7, -0.3, 2.025,
    -3.0, -0.3, 2.25,
    -3.0, 0.0, 2.25,
    -2.7, 0.0, 1.8,
    -2.7, -0.3, 1.8,
    -3.0, -0.3, 1.8,
    -3.0, 0.0, 1.8,
    -1.5, 0.3, 2.25,
    -1.6, 0.3, 2.025,
    -2.5, 0.3, 2.25,
    -2.3, 0.3, 2.025,
    -3.0, 0.3, 2.25,
    -2.7, 0.3, 2.025,
    -3.0, 0.3, 1.8,
    -2.7, 0.3, 1.8,
    -2.7, 0.0, 1.575,
    -2.7, -0.3, 1.575,
    -3.0, -0.3, 1.35,
    -3.0, 0.0, 1.35,
    -2.5, 0.0, 1.125,
    -2.5, -0.3, 1.125,
    -2.65, -0.3, 0.9375,
    -2.65, 0.0, 0.9375,
    -2.0, -0.3, 0.9,
    -1.9, -0.3, 0.6,
    -1.9, 0.0, 0.6,
    -3.0, 0.3, 1.35,
    -2.7, 0.3, 1.575,
    -2.65, 0.3, 0.9375,
    -2.5, 0.3, 1.125,
    -1.9, 0.3, 0.6,
    -2.0, 0.3, 0.9,
    1.7, 0.0, 1.425,
    1.7, -0.66, 1.425,
    1.7, -0.66, 0.6,
    1.7, 0.0, 0.6,
    2.6, 0.0, 1.425,
    2.6, -0.66, 1.425,
    3.1, -0.66, 0.825,
    3.1, 0.0, 0.825,
    2.3, 0.0, 2.1,
    2.3, -0.25, 2.1,
    2.4, -0.25, 2.025,
    2.4, 0.0, 2.025,
    2.7, 0.0, 2.4,
    2.7, -0.25, 2.4,
    3.3, -0.25, 2.4,
    3.3, 0.0, 2.4,
    1.7, 0.66, 0.6,
    1.7, 0.66, 1.425,
    3.1, 0.66, 0.825,
    2.6, 0.66, 1.425,
    2.4, 0.25, 2.025,
    2.3, 0.25, 2.1,
    3.3, 0.25, 2.4,
    2.7, 0.25, 2.4,
    2.8, 0.0, 2.475,
    2.8, -0.25, 2.475,
    3.525, -0.25, 2.49375,
    3.525, 0.0, 2.49375,
    2.9, 0.0, 2.475,
    2.9, -0.15, 2.475,
    3.45, -0.15, 2.5125,
    3.45, 0.0, 2.5125,
    2.8, 0.0, 2.4,
    2.8, -0.15, 2.4,
    3.2, -0.15, 2.4,
    3.2, 0.0, 2.4,
    3.525, 0.25, 2.49375,
    2.8, 0.25, 2.475,
    3.45, 0.15, 2.5125,
    2.9, 0.15, 2.475,
    3.2, 0.15, 2.4,
    2.8, 0.15, 2.4,
    0.0, 0.0, 3.15,
    0.8, 0.0, 3.15,
    0.8, -0.45, 3.15,
    0.45, -0.8, 3.15,
    0.0, -0.8, 3.15,
    0.0, 0.0, 2.85,
    0.2, 0.0, 2.7,
    0.2, -0.112, 2.7,
    0.112, -0.2, 2.7,
    0.0, -0.2, 2.7,
    -0.45, -0.8, 3.15,
    -0.8, -0.45, 3.15,
    -0.8, 0.0, 3.15,
    -0.112, -0.2, 2.7,
    -0.2, -0.112, 2.7,
    -0.2, 0.0, 2.7,
    -0.8, 0.45, 3.15,
    -0.45, 0.8, 3.15,
    0.0, 0.8, 3.15,
    -0.2, 0.112, 2.7,
    -0.112, 0.2, 2.7,
    0.0, 0.2, 2.7,
    0.45, 0.8, 3.15,
    0.8, 0.45, 3.15,
    0.112, 0.2, 2.7,
    0.2, 0.112, 2.7,
    0.4, 0.0, 2.55,
    0.4, -0.224, 2.55,
    0.224, -0.4, 2.55,
    0.0, -0.4, 2.55,
    1.3, 0.0, 2.55,
    1.3, -0.728, 2.55,
    0.728, -1.3, 2.55,
    0.0, -1.3, 2.55,
    1.3, 0.0, 2.4,
    1.3, -0.728, 2.4,
    0.728, -1.3, 2.4,
    0.0, -1.3, 2.4,
    -0.224, -0.4, 2.55,
    -0.4, -0.224, 2.55,
    -0.4, 0.0, 2.55,
    -0.728, -1.3, 2.55,
    -1.3, -0.728, 2.55,
    -1.3, 0.0, 2.55,
    -0.728, -1.3, 2.4,
    -1.3, -0.728, 2.4,
    -1.3, 0.0, 2.4,
    -0.4, 0.224, 2.55,
    -0.224, 0.4, 2.55,
    0.0, 0.4, 2.55,
    -1.3, 0.728, 2.55,
    -0.728, 1.3, 2.55,
    0.0, 1.3, 2.55,
    -1.3, 0.728, 2.4,
    -0.728, 1.3, 2.4,
    0.0, 1.3, 2.4,
    0.224, 0.4, 2.55,
    0.4, 0.224, 2.55,
    0.728, 1.3, 2.55,
    1.3, 0.728, 2.55,
    0.728, 1.3, 2.4,
    1.3, 0.728, 2.4,
    0.0, 0.0, 0.0,
    1.425, 0.0, 0.0,
    1.425, 0.798, 0.0,
    0.798, 1.425, 0.0,
    0.0, 1.425, 0.0,
    1.5, 0.0, 0.075,
    1.5, 0.84, 0.075,
    0.84, 1.5, 0.075,
    0.0, 1.5, 0.075,
    -0.798, 1.425, 0.0,
    -1.425, 0.798, 0.0,
    -1.425, 0.0, 0.0,
    -0.84, 1.5, 0.075,
    -1.5, 0.84, 0.075,
    -1.5, 0.0, 0.075,
    -1.425, -0.798, 0.0,
    -0.798, -1.425, 0.0,
    0.0, -1.425, 0.0,
    -1.5, -0.84, 0.075,
    -0.84, -1.5, 0.075,
    0.0, -1.5, 0.075,
    0.798, -1.425, 0.0,
    1.425, -0.798, 0.0,
    0.84, -1.5, 0.075,
    1.5, -0.84, 0.075,
];

/// `new TeapotGeometry( size, segments )` with the default
/// `bottom`/`lid`/`body`/`fitLid`/`blinn` (all `true`).
pub fn teapot_geometry(size: f64, segments: usize) -> BufferGeometry {
    teapot_geometry_full(size, segments, true, true, true, true, true)
}

#[allow(clippy::needless_range_loop)]
pub fn teapot_geometry_full(
    size: f64,
    segments: usize,
    bottom: bool,
    lid: bool,
    body: bool,
    fit_lid: bool,
    blinn: bool,
) -> BufferGeometry {
    // number of segments per patch
    let segments = segments.max(2);

    // Jim Blinn scaled the teapot down in size by about 1.3 for
    // some rendering tests. He liked the new proportions that he kept
    // the data in this form. The model was distributed with these new
    // proportions and became the norm. Trivia: comparing images of the
    // real teapot and the computer model, the ratio for the bowl of the
    // real teapot is more like 1.25, but since 1.3 is the traditional
    // value given, we use it here.
    let blinn_scale = 1.3;

    // scale the size to be the real scaling factor
    let max_height = 3.15 * (if blinn { 1.0 } else { blinn_scale });

    let max_height2 = max_height / 2.0;
    let true_size = size / max_height2;

    // Number of elements depends on what is needed. Subtract degenerate
    // triangles at tip of bottom and lid out in advance.
    let mut num_triangles = if bottom {
        (8 * segments - 4) * segments
    } else {
        0
    };
    num_triangles += if lid { (16 * segments - 4) * segments } else { 0 };
    num_triangles += if body { 40 * segments * segments } else { 0 };

    let mut indices: Vec<u32> = vec![0; num_triangles * 3];

    let mut num_vertices = if bottom { 4 } else { 0 };
    num_vertices += if lid { 8 } else { 0 };
    num_vertices += if body { 20 } else { 0 };
    num_vertices *= (segments + 1) * (segments + 1);

    let mut vertices: Vec<f32> = vec![0.0; num_vertices * 3];
    let mut normals: Vec<f32> = vec![0.0; num_vertices * 3];
    let mut uvs: Vec<f32> = vec![0.0; num_vertices * 2];

    // Bezier form
    let mut ms = Matrix4::identity();
    matrix4_set(
        &mut ms,
        -1.0, 3.0, -3.0, 1.0, //
        3.0, -6.0, 3.0, 0.0, //
        -3.0, 3.0, 0.0, 0.0, //
        1.0, 0.0, 0.0, 0.0,
    );

    let mut g = [0.0f64; 16];

    let mut sp = [0.0f64; 4];
    let mut tp = [0.0f64; 4];
    let mut dsp = [0.0f64; 4];
    let mut dtp = [0.0f64; 4];

    // M * G * M matrix, sort of see
    // http://www.cs.helsinki.fi/group/goa/mallinnus/curves/surfaces.html
    let mut mgm = [Matrix4::identity(); 3];

    let mut vert = [0.0f64; 3];
    let mut sdir = [0.0f64; 3];
    let mut tdir = [0.0f64; 3];

    let mut norm = Vector3::ZERO;

    let mut tcoord;

    let mut sval;
    let mut tval;
    let mut dsval;
    let mut dtval;

    let mut norm_out = Vector3::ZERO;

    let mut gmx = Matrix4::identity();
    let mut tmtx = Matrix4::identity();

    let mut vsp;
    let mut vtp;
    let mut vdsp;
    let mut vdtp;

    let mut vsdir = Vector3::ZERO;
    let mut vtdir = Vector3::ZERO;

    let mut mst = ms;
    matrix4_transpose(&mut mst);

    let min_patches = if body { 0 } else { 20 };
    let max_patches = if bottom { 32 } else { 28 };

    let vert_per_row = segments + 1;

    let mut surf_count = 0usize;

    let mut vert_count = 0usize;
    let mut norm_count = 0usize;
    let mut uv_count = 0usize;

    let mut index_count = 0usize;

    for surf in min_patches..max_patches {
        // lid is in the middle of the data, patches 20-27,
        // so ignore it for this part of the loop if the lid is not desired
        if !(lid || (surf < 20 || surf >= 28)) {
            continue;
        }

        // get M * G * M matrix for x,y,z
        for i in 0..3 {
            // get control patches
            for r in 0..4 {
                for c in 0..4 {
                    // transposed
                    g[c * 4 + r] = TEAPOT_VERTICES[TEAPOT_PATCHES[surf * 16 + r * 4 + c] * 3 + i];

                    // is the lid to be made larger, and is this a point on the lid
                    // that is X or Y?
                    if fit_lid && (surf >= 20 && surf < 28) && (i != 2) {
                        // increase XY size by 7.7%, found empirically. I don't
                        // increase Z so that the teapot will continue to fit in the
                        // space -1 to 1 for Y (Y is up for the final model).
                        g[c * 4 + r] *= 1.077;
                    }

                    // Blinn "fixed" the teapot by dividing Z by blinnScale, and that's the
                    // data we now use. The original teapot is taller. Fix it:
                    if !blinn && (i == 2) {
                        g[c * 4 + r] *= blinn_scale;
                    }
                }
            }

            matrix4_set(
                &mut gmx, g[0], g[1], g[2], g[3], g[4], g[5], g[6], g[7], g[8], g[9], g[10], g[11],
                g[12], g[13], g[14], g[15],
            );

            tmtx.multiply_matrices(&gmx, &ms);
            mgm[i].multiply_matrices(&mst, &tmtx);
        }

        // step along, get points, and output
        for sstep in 0..=segments {
            let s = sstep as f64 / segments as f64;

            for tstep in 0..=segments {
                let t = tstep as f64 / segments as f64;

                // point from basis
                // get power vectors and their derivatives
                sval = 1.0;
                tval = 1.0;
                dsval = 0.0;
                dtval = 0.0;
                let mut p = 4usize;
                while p > 0 {
                    p -= 1;

                    sp[p] = sval;
                    tp[p] = tval;
                    sval *= s;
                    tval *= t;

                    if p == 3 {
                        dsp[p] = 0.0;
                        dtp[p] = 0.0;
                        dsval = 1.0;
                        dtval = 1.0;
                    } else {
                        dsp[p] = dsval * (3 - p) as f64;
                        dtp[p] = dtval * (3 - p) as f64;
                        dsval *= s;
                        dtval *= t;
                    }
                }

                vsp = Vector4::from_array(&sp);
                vtp = Vector4::from_array(&tp);
                vdsp = Vector4::from_array(&dsp);
                vdtp = Vector4::from_array(&dtp);

                // do for x,y,z
                for i in 0..3 {
                    // multiply power vectors times matrix to get value
                    tcoord = vsp;
                    tcoord.apply_matrix4(&mgm[i]);
                    vert[i] = tcoord.dot(&vtp);

                    // get s and t tangent vectors
                    tcoord = vdsp;
                    tcoord.apply_matrix4(&mgm[i]);
                    sdir[i] = tcoord.dot(&vtp);

                    tcoord = vsp;
                    tcoord.apply_matrix4(&mgm[i]);
                    tdir[i] = tcoord.dot(&vdtp);
                }

                // find normal
                vsdir.set(sdir[0], sdir[1], sdir[2]);
                vtdir.set(tdir[0], tdir[1], tdir[2]);
                norm.cross_vectors(&vtdir, &vsdir);
                norm.normalize();

                // if X and Z length is 0, at the cusp, so point the normal up or down, depending on patch number
                if vert[0] == 0.0 && vert[1] == 0.0 {
                    // if above the middle of the teapot, normal points up, else down
                    norm_out.set(0.0, if vert[2] > max_height2 { 1.0 } else { -1.0 }, 0.0);
                } else {
                    // standard output: rotate on X axis
                    norm_out.set(norm.x, norm.z, -norm.y);
                }

                // store it all
                vertices[vert_count] = (true_size * vert[0]) as f32;
                vert_count += 1;
                vertices[vert_count] = (true_size * (vert[2] - max_height2)) as f32;
                vert_count += 1;
                vertices[vert_count] = (-true_size * vert[1]) as f32;
                vert_count += 1;

                normals[norm_count] = norm_out.x as f32;
                norm_count += 1;
                normals[norm_count] = norm_out.y as f32;
                norm_count += 1;
                normals[norm_count] = norm_out.z as f32;
                norm_count += 1;

                uvs[uv_count] = (1.0 - t) as f32;
                uv_count += 1;
                uvs[uv_count] = (1.0 - s) as f32;
                uv_count += 1;
            }
        }

        // internal closure: test if triangle has any matching vertices;
        // if so, don't save triangle, since it won't display anything.
        // (the comparison is on the stored `Float32Array` values, as in the addon)
        let not_degenerate = |vertices: &[f32], vtx1: usize, vtx2: usize, vtx3: usize| -> bool {
            !(((vertices[vtx1 * 3] == vertices[vtx2 * 3])
                && (vertices[vtx1 * 3 + 1] == vertices[vtx2 * 3 + 1])
                && (vertices[vtx1 * 3 + 2] == vertices[vtx2 * 3 + 2]))
                || ((vertices[vtx1 * 3] == vertices[vtx3 * 3])
                    && (vertices[vtx1 * 3 + 1] == vertices[vtx3 * 3 + 1])
                    && (vertices[vtx1 * 3 + 2] == vertices[vtx3 * 3 + 2]))
                || (vertices[vtx2 * 3] == vertices[vtx3 * 3])
                    && (vertices[vtx2 * 3 + 1] == vertices[vtx3 * 3 + 1])
                    && (vertices[vtx2 * 3 + 2] == vertices[vtx3 * 3 + 2]))
        };

        // save the faces
        for sstep in 0..segments {
            for tstep in 0..segments {
                let v1 = surf_count * vert_per_row * vert_per_row + sstep * vert_per_row + tstep;
                let v2 = v1 + 1;
                let v3 = v2 + vert_per_row;
                let v4 = v1 + vert_per_row;

                // Normals and UVs cannot be shared.
                if not_degenerate(&vertices, v1, v2, v3) {
                    indices[index_count] = v1 as u32;
                    index_count += 1;
                    indices[index_count] = v2 as u32;
                    index_count += 1;
                    indices[index_count] = v3 as u32;
                    index_count += 1;
                }

                if not_degenerate(&vertices, v1, v3, v4) {
                    indices[index_count] = v1 as u32;
                    index_count += 1;
                    indices[index_count] = v3 as u32;
                    index_count += 1;
                    indices[index_count] = v4 as u32;
                    index_count += 1;
                }
            }
        }

        // increment only if a surface was used
        surf_count += 1;
    }

    debug_assert_eq!(index_count, num_triangles * 3);
    debug_assert_eq!(vert_count, num_vertices * 3);

    let mut geometry = BufferGeometry::new();
    // the addon hands `setIndex` a `Uint32Array` BufferAttribute, so the index
    // stays 32-bit even when the values would fit in 16.
    geometry.set_index_attribute(Index::U32(indices));
    geometry.position = Some(BufferAttribute::new(vertices, 3));
    geometry.normal = Some(BufferAttribute::new(normals, 3));
    geometry.uv = Some(BufferAttribute::new(uvs, 2));
    geometry
}
