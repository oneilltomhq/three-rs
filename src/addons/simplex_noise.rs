//! Port of `three.js/examples/jsm/math/SimplexNoise.js` — Stefan Gustavson's
//! 2D, 3D and 4D simplex noise ("Simplex noise demystified", 2005), the CPU
//! noise `DenoiseNode` fills its 64×64 rotation texture with.
//!
//! f64 throughout, like the JavaScript numbers it reproduces. Three seeds the
//! permutation from `Math.random()` by default; the port has no global random
//! source, so the constructor takes the `random()` closure three's optional
//! `r` argument supplies.

/// `this.grad3`: the twelve edge midpoints of a cube.
const GRAD3: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// `this.grad4`: the 32 edge midpoints of a tesseract.
const GRAD4: [[f64; 4]; 32] = [
    [0.0, 1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, -1.0],
    [0.0, 1.0, -1.0, 1.0],
    [0.0, 1.0, -1.0, -1.0],
    [0.0, -1.0, 1.0, 1.0],
    [0.0, -1.0, 1.0, -1.0],
    [0.0, -1.0, -1.0, 1.0],
    [0.0, -1.0, -1.0, -1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, -1.0],
    [1.0, 0.0, -1.0, 1.0],
    [1.0, 0.0, -1.0, -1.0],
    [-1.0, 0.0, 1.0, 1.0],
    [-1.0, 0.0, 1.0, -1.0],
    [-1.0, 0.0, -1.0, 1.0],
    [-1.0, 0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, -1.0],
    [1.0, -1.0, 0.0, 1.0],
    [1.0, -1.0, 0.0, -1.0],
    [-1.0, 1.0, 0.0, 1.0],
    [-1.0, 1.0, 0.0, -1.0],
    [-1.0, -1.0, 0.0, 1.0],
    [-1.0, -1.0, 0.0, -1.0],
    [1.0, 1.0, 1.0, 0.0],
    [1.0, 1.0, -1.0, 0.0],
    [1.0, -1.0, 1.0, 0.0],
    [1.0, -1.0, -1.0, 0.0],
    [-1.0, 1.0, 1.0, 0.0],
    [-1.0, 1.0, -1.0, 0.0],
    [-1.0, -1.0, 1.0, 0.0],
    [-1.0, -1.0, -1.0, 0.0],
];

/// `this.simplex`: the traversal order of the 4D simplex for each of the 64
/// orderings `noise4d` encodes in six comparison bits (only 24 occur).
#[rustfmt::skip]
const SIMPLEX: [[u8; 4]; 64] = [
    [0, 1, 2, 3], [0, 1, 3, 2], [0, 0, 0, 0], [0, 2, 3, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [1, 2, 3, 0],
    [0, 2, 1, 3], [0, 0, 0, 0], [0, 3, 1, 2], [0, 3, 2, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [1, 3, 2, 0],
    [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
    [1, 2, 0, 3], [0, 0, 0, 0], [1, 3, 0, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [2, 3, 0, 1], [2, 3, 1, 0],
    [1, 0, 2, 3], [1, 0, 3, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [2, 0, 3, 1], [0, 0, 0, 0], [2, 1, 3, 0],
    [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0],
    [2, 0, 1, 3], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [3, 0, 1, 2], [3, 0, 2, 1], [0, 0, 0, 0], [3, 1, 2, 0],
    [2, 1, 0, 3], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [3, 1, 0, 2], [0, 0, 0, 0], [3, 2, 0, 1], [3, 2, 1, 0],
];

/// `new SimplexNoise( r )`: a random permutation plus the three noise
/// functions that hash through it.
#[derive(Clone, Debug)]
pub struct SimplexNoise {
    /// `this.p`: 256 entries of `Math.floor( r.random() * 256 )`. Not a true
    /// permutation (entries can repeat), exactly as in three.
    pub p: [u8; 256],
    /// `this.perm`: `p` doubled so `perm[ i + perm[ j ] ]` never wraps.
    pub perm: [u8; 512],
}

/// One corner's contribution: `t <- t0 - |d|²`, zero outside the kernel,
/// else `t⁴ · dot( g, d )`.
fn corner(t0: f64, g: &[f64], d: &[f64]) -> f64 {
    let mut t = t0;
    for x in d {
        t -= x * x;
    }
    if t < 0.0 {
        return 0.0;
    }
    t *= t;
    t * t * g.iter().zip(d).map(|(a, b)| a * b).sum::<f64>()
}

impl SimplexNoise {
    /// `new SimplexNoise( r )`, where `random` is `r.random` (three's default
    /// `r` is `Math`). Draws exactly 256 values.
    pub fn new(mut random: impl FnMut() -> f64) -> Self {
        let mut p = [0u8; 256];
        for v in p.iter_mut() {
            *v = (random() * 256.0).floor() as u8;
        }
        let mut perm = [0u8; 512];
        for (i, v) in perm.iter_mut().enumerate() {
            *v = p[i & 255];
        }
        Self { p, perm }
    }

    fn perm(&self, i: i64) -> i64 {
        self.perm[i as usize] as i64
    }

    /// `noise( xin, yin )`: 2D simplex noise in [-1, 1].
    pub fn noise(&self, xin: f64, yin: f64) -> f64 {
        let f2 = 0.5 * (3.0f64.sqrt() - 1.0);
        let s = (xin + yin) * f2;
        let i = (xin + s).floor();
        let j = (yin + s).floor();
        let g2 = (3.0 - 3.0f64.sqrt()) / 6.0;
        let t = (i + j) * g2;
        let x0 = xin - (i - t);
        let y0 = yin - (j - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f64 + g2;
        let y1 = y0 - j1 as f64 + g2;
        let x2 = x0 - 1.0 + 2.0 * g2;
        let y2 = y0 - 1.0 + 2.0 * g2;
        let ii = (i as i64) & 255;
        let jj = (j as i64) & 255;
        let gi0 = (self.perm(ii + self.perm(jj)) % 12) as usize;
        let gi1 = (self.perm(ii + i1 + self.perm(jj + j1)) % 12) as usize;
        let gi2 = (self.perm(ii + 1 + self.perm(jj + 1)) % 12) as usize;
        let n0 = corner(0.5, &GRAD3[gi0][..2], &[x0, y0]);
        let n1 = corner(0.5, &GRAD3[gi1][..2], &[x1, y1]);
        let n2 = corner(0.5, &GRAD3[gi2][..2], &[x2, y2]);
        70.0 * (n0 + n1 + n2)
    }

    /// `noise3d( xin, yin, zin )`: 3D simplex noise, just inside [-1, 1].
    pub fn noise3d(&self, xin: f64, yin: f64, zin: f64) -> f64 {
        let f3 = 1.0 / 3.0;
        let s = (xin + yin + zin) * f3;
        let i = (xin + s).floor();
        let j = (yin + s).floor();
        let k = (zin + s).floor();
        let g3 = 1.0 / 6.0;
        let t = (i + j + k) * g3;
        let x0 = xin - (i - t);
        let y0 = yin - (j - t);
        let z0 = zin - (k - t);
        let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
            if y0 >= z0 {
                (1, 0, 0, 1, 1, 0)
            } else if x0 >= z0 {
                (1, 0, 0, 1, 0, 1)
            } else {
                (0, 0, 1, 1, 0, 1)
            }
        } else if y0 < z0 {
            (0, 0, 1, 0, 1, 1)
        } else if x0 < z0 {
            (0, 1, 0, 0, 1, 1)
        } else {
            (0, 1, 0, 1, 1, 0)
        };
        let d0 = [x0, y0, z0];
        let d1 = [x0 - i1 as f64 + g3, y0 - j1 as f64 + g3, z0 - k1 as f64 + g3];
        let d2 = [
            x0 - i2 as f64 + 2.0 * g3,
            y0 - j2 as f64 + 2.0 * g3,
            z0 - k2 as f64 + 2.0 * g3,
        ];
        let d3 = [x0 - 1.0 + 3.0 * g3, y0 - 1.0 + 3.0 * g3, z0 - 1.0 + 3.0 * g3];
        let ii = (i as i64) & 255;
        let jj = (j as i64) & 255;
        let kk = (k as i64) & 255;
        let hash = |a: i64, b: i64, c: i64| {
            (self.perm(ii + a + self.perm(jj + b + self.perm(kk + c))) % 12) as usize
        };
        let n0 = corner(0.6, &GRAD3[hash(0, 0, 0)], &d0);
        let n1 = corner(0.6, &GRAD3[hash(i1, j1, k1)], &d1);
        let n2 = corner(0.6, &GRAD3[hash(i2, j2, k2)], &d2);
        let n3 = corner(0.6, &GRAD3[hash(1, 1, 1)], &d3);
        32.0 * (n0 + n1 + n2 + n3)
    }

    /// `noise4d( x, y, z, w )`: 4D simplex noise in [-1, 1].
    pub fn noise4d(&self, x: f64, y: f64, z: f64, w: f64) -> f64 {
        let f4 = (5.0f64.sqrt() - 1.0) / 4.0;
        let g4 = (5.0 - 5.0f64.sqrt()) / 20.0;
        let s = (x + y + z + w) * f4;
        let i = (x + s).floor();
        let j = (y + s).floor();
        let k = (z + s).floor();
        let l = (w + s).floor();
        let t = (i + j + k + l) * g4;
        let d0 = [x - (i - t), y - (j - t), z - (k - t), w - (l - t)];
        let [x0, y0, z0, w0] = d0;
        let c = (if x0 > y0 { 32 } else { 0 })
            + (if x0 > z0 { 16 } else { 0 })
            + (if y0 > z0 { 8 } else { 0 })
            + (if x0 > w0 { 4 } else { 0 })
            + (if y0 > w0 { 2 } else { 0 })
            + (if z0 > w0 { 1 } else { 0 });
        let sc = SIMPLEX[c];
        // Corner offsets: the coordinate ranked >= `rank` steps first.
        let offsets = |rank: u8| sc.map(|v| i64::from(v >= rank));
        let o1 = offsets(3);
        let o2 = offsets(2);
        let o3 = offsets(1);
        let shift = |o: [i64; 4], n: f64| {
            let mut d = d0;
            for (a, b) in d.iter_mut().zip(o) {
                *a = *a - b as f64 + n * g4;
            }
            d
        };
        let d1 = shift(o1, 1.0);
        let d2 = shift(o2, 2.0);
        let d3 = shift(o3, 3.0);
        let d4 = shift([1; 4], 4.0);
        let ii = (i as i64) & 255;
        let jj = (j as i64) & 255;
        let kk = (k as i64) & 255;
        let ll = (l as i64) & 255;
        let hash = |o: [i64; 4]| {
            let inner = self.perm(ll + o[3]);
            let inner = self.perm(kk + o[2] + inner);
            let inner = self.perm(jj + o[1] + inner);
            (self.perm(ii + o[0] + inner) % 32) as usize
        };
        let n0 = corner(0.6, &GRAD4[hash([0; 4])], &d0);
        let n1 = corner(0.6, &GRAD4[hash(o1)], &d1);
        let n2 = corner(0.6, &GRAD4[hash(o2)], &d2);
        let n3 = corner(0.6, &GRAD4[hash(o3)], &d3);
        let n4 = corner(0.6, &GRAD4[hash([1; 4])], &d4);
        27.0 * (n0 + n1 + n2 + n3 + n4)
    }
}

#[cfg(test)]
mod tests {
    use super::SimplexNoise;

    /// Park–Miller minimal standard generator, exact in f64, so the same
    /// sequence drives three's JS and this port.
    fn park_miller() -> impl FnMut() -> f64 {
        let mut s: f64 = 12345.0;
        move || {
            s = (s * 16807.0) % 2147483647.0;
            s / 2147483647.0
        }
    }

    /// Reference values printed by three's `SimplexNoise.js` under node with
    /// `r = { random: park_miller }` (seed 12345).
    #[test]
    fn matches_three() {
        let n = SimplexNoise::new(park_miller());
        assert_eq!(&n.perm[..8], &[24, 213, 242, 9, 2, 13, 196, 149]);
        assert_eq!(n.p[255], 204);
        assert_eq!(n.perm[256 + 255], 204);
        let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} vs {b}");
        close(n.noise(0.3, 0.7), 0.2829394682535208);
        close(n.noise(-1.25, 3.5), -0.3893883622150483);
        close(n.noise(10.1, -7.9), 0.0);
        close(n.noise(64.0, 0.0), 0.21296103333786495);
        close(n.noise(5.0, 129.0), -3.0715353432682404e-14);
        close(n.noise(2.6, 1.3), -0.5858681707541288);
        close(n.noise(-0.4, -17.2), -0.601769452293015);
        close(n.noise3d(0.3, 0.7, 0.1), -0.13681240599176955);
        close(n.noise3d(-1.25, 3.5, 2.2), 0.27330374407021635);
        close(n.noise3d(10.1, -7.9, -0.4), -0.5257415573333342);
        close(n.noise4d(0.3, 0.7, 0.1, 0.9), 0.10622713406473042);
        close(n.noise4d(-1.25, 3.5, 2.2, -3.3), 0.5017196472877987);
        close(n.noise4d(10.1, -7.9, -0.4, 1.7), 0.06578909473719569);
    }
}
