//! Port of `three.js/examples/jsm/utils/GeometryUtils.js` — the
//! `webgpu_lines_fat` slice.

use crate::math::Vector3;

/// `hilbert3D( center, size, iterations, v0 … v7 )` — the 3D Hilbert curve's
/// corner points, in visiting order.
///
/// One iteration is eight corners of the cube; each further iteration replaces
/// every corner with a half-size copy under a permutation of the corner
/// indices, so `iterations = n` gives `8^n` points. `webgpu_lines_fat` asks for
/// `hilbert3D( ( 0, 0, 0 ), 20.0, 1, 0 … 7 )` — 64 points, because the port's
/// recursion, like three's, decrements *before* the test and so does one level
/// more than the argument reads.
///
/// The arithmetic is `f64` throughout and the corners come out of `±half`
/// sums, which are exact in binary for `size = 20`; the oracle in
/// `scouts/webgpu_lines_fat/spline_oracle.json` is matched bit for bit by
/// `tests/extras_spline_oracle.rs`.
pub fn hilbert_3d(center: Vector3, size: f64, iterations: i32, v: [usize; 8]) -> Vec<Vector3> {
    let half = size / 2.0;

    let vec_s = [
        Vector3::new(center.x - half, center.y + half, center.z - half),
        Vector3::new(center.x - half, center.y + half, center.z + half),
        Vector3::new(center.x - half, center.y - half, center.z + half),
        Vector3::new(center.x - half, center.y - half, center.z - half),
        Vector3::new(center.x + half, center.y - half, center.z - half),
        Vector3::new(center.x + half, center.y - half, center.z + half),
        Vector3::new(center.x + half, center.y + half, center.z + half),
        Vector3::new(center.x + half, center.y + half, center.z - half),
    ];

    let vec: [Vector3; 8] = std::array::from_fn(|i| vec_s[v[i]]);

    let iterations = iterations - 1;
    if iterations < 0 {
        return vec.to_vec();
    }

    let [v0, v1, v2, v3, v4, v5, v6, v7] = v;
    let children = [
        (vec[0], [v0, v3, v4, v7, v6, v5, v2, v1]),
        (vec[1], [v0, v7, v6, v1, v2, v5, v4, v3]),
        (vec[2], [v0, v7, v6, v1, v2, v5, v4, v3]),
        (vec[3], [v2, v3, v0, v1, v6, v7, v4, v5]),
        (vec[4], [v2, v3, v0, v1, v6, v7, v4, v5]),
        (vec[5], [v4, v3, v2, v5, v6, v1, v0, v7]),
        (vec[6], [v4, v3, v2, v5, v6, v1, v0, v7]),
        (vec[7], [v6, v5, v2, v1, v0, v3, v4, v7]),
    ];

    children
        .into_iter()
        .flat_map(|(center, v)| hilbert_3d(center, half, iterations, v))
        .collect()
}

/// `hilbert3D( center, size, iterations )` with the default corner order.
pub fn hilbert_3d_default(center: Vector3, size: f64, iterations: i32) -> Vec<Vector3> {
    hilbert_3d(center, size, iterations, [0, 1, 2, 3, 4, 5, 6, 7])
}
