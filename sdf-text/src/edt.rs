//! Exact Euclidean distance transform — a port of lib3's `src/sdf/edt.js`
//! (Felzenszwalb & Huttenlocher, the lower-envelope-of-parabolas 1-D pass run
//! over columns then rows). Not 8SSEDT, not a Chebyshev approximation.
//!
//! `f64` throughout, because the JS uses `Float64Array` and the `sqrt` results
//! feed the atlas encode directly; `f32` would differ in the last bits.
//!
//! One deliberate deviation in form, not in result: the JS inner hull-pop loop
//! is a `do … while (k >= 0)` that can read `v[-1]` (`undefined` → `NaN` → the
//! `s > z[k]` test is false → pop again). In practice `k` never goes below 0 for
//! these inputs, but the canonical guarded form is written here so the port
//! cannot index out of bounds. The `k < 0` exit keeps the JS's exact behaviour:
//! `s` retains the value computed against `v[0]`, and the following `k += 1`
//! puts it back at slot 0.

pub const INF: f64 = 1e20;

/// One 1-D pass. `f` is the input row/column, `d` the squared-distance output;
/// `v` and `z` are the hull scratch arrays (`z` needs `n + 1` slots).
pub fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64], n: usize) {
    if n == 0 {
        return;
    }
    v[0] = 0;
    z[0] = -INF;
    z[1] = INF;
    let mut k: isize = 0;

    for q in 1..n {
        let mut s;
        loop {
            let r = v[k as usize];
            s = (f[q] - f[r] + (q * q) as f64 - (r * r) as f64) / (2 * q - 2 * r) as f64;
            if s > z[k as usize] {
                break;
            }
            k -= 1;
            if k < 0 {
                break;
            }
        }
        k += 1;
        let ku = k as usize;
        v[ku] = q;
        z[ku] = s;
        z[ku + 1] = INF;
    }

    let mut k: usize = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dx = q as f64 - v[k] as f64;
        d[q] = dx * dx + f[v[k]];
    }
}

/// Columns first, then rows, in place. Scratch sized `max(w, h)` / `+1`.
pub fn edt_2d(grid: &mut [f64], width: usize, height: usize) {
    let max_dim = width.max(height);
    let mut f = vec![0.0f64; max_dim];
    let mut d = vec![0.0f64; max_dim];
    let mut v = vec![0usize; max_dim];
    let mut z = vec![0.0f64; max_dim + 1];

    for x in 0..width {
        for y in 0..height {
            f[y] = grid[y * width + x];
        }
        edt_1d(&f, &mut d, &mut v, &mut z, height);
        for y in 0..height {
            grid[y * width + x] = d[y];
        }
    }

    for y in 0..height {
        let offset = y * width;
        f[..width].copy_from_slice(&grid[offset..offset + width]);
        edt_1d(&f, &mut d, &mut v, &mut z, width);
        grid[offset..offset + width].copy_from_slice(&d[..width]);
    }
}

/// Signed distance field from a binary alpha image, negative inside.
///
/// The JS reads `imageData[i * 4 + 3]` out of an RGBA buffer; this takes the
/// alpha plane alone, which is all `computeSDF` ever looks at. Distances are in
/// source pixels. The image is **binarised** at `alpha >= alpha_threshold`, so
/// the rasteriser only has to agree with Chromium on which texels clear 50 %
/// coverage.
pub fn compute_sdf(alpha: &[u8], width: usize, height: usize, alpha_threshold: u8) -> Vec<f64> {
    let size = width * height;
    assert_eq!(alpha.len(), size, "alpha plane must be width * height");
    let mut outside = vec![0.0f64; size];
    let mut inside = vec![0.0f64; size];

    for i in 0..size {
        if alpha[i] >= alpha_threshold {
            outside[i] = 0.0;
            inside[i] = INF;
        } else {
            outside[i] = INF;
            inside[i] = 0.0;
        }
    }

    edt_2d(&mut outside, width, height);
    edt_2d(&mut inside, width, height);

    (0..size)
        .map(|i| outside[i].sqrt() - inside[i].sqrt())
        .collect()
}

/// `computeSDF(imageData, w, h)` with the JS default threshold of 128.
pub fn compute_sdf_default(alpha: &[u8], width: usize, height: usize) -> Vec<f64> {
    compute_sdf(alpha, width, height, 128)
}
