//! Port of `three.js/examples/jsm/math/ImprovedNoise.js` — Ken Perlin's
//! improved noise (2002), the CPU noise `webgpu_volume_perlin` fills its
//! 128³ volume with.
//!
//! f64 throughout, like the JavaScript numbers it reproduces: the volume is
//! quantised to bytes afterwards, and a value that lands on a byte boundary
//! must land on the same side of it as three's does.

/// Perlin's permutation, doubled so `P[ i + 1 ]` never wraps.
const PERMUTATION: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

fn p(i: usize) -> usize {
    PERMUTATION[i & 255] as usize
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn grad(hash: usize, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}

/// `MathUtils.lerp( x, y, t )` — `( 1 - t ) * x + t * y`, in that order.
fn lerp(x: f64, y: f64, t: f64) -> f64 {
    (1.0 - t) * x + t * y
}

/// `new ImprovedNoise()`. Stateless — the permutation is a constant — so it
/// is a unit struct, kept for the shape of the call sites.
#[derive(Clone, Copy, Debug, Default)]
pub struct ImprovedNoise;

impl ImprovedNoise {
    pub fn new() -> Self {
        Self
    }

    /// `noise( x, y, z )`, in `[ -1, 1 ]`.
    pub fn noise(&self, x: f64, y: f64, z: f64) -> f64 {
        let (floor_x, floor_y, floor_z) = (x.floor(), y.floor(), z.floor());
        // `floorX & 255`: JS's `&` works on the two's-complement int32, so a
        // negative cell index wraps the same way `rem_euclid` does.
        let cell = |f: f64| (f as i64).rem_euclid(256) as usize;
        let (cx, cy, cz) = (cell(floor_x), cell(floor_y), cell(floor_z));

        let (x, y, z) = (x - floor_x, y - floor_y, z - floor_z);
        let (x1, y1, z1) = (x - 1.0, y - 1.0, z - 1.0);
        let (u, v, w) = (fade(x), fade(y), fade(z));

        // `_p` is 512 entries long, `_p[ 256 + i ] = _p[ i ]`: every index
        // below is at most 255 + 255 + 1, and `p()` wraps it.
        let a = p(cx) + cy;
        let aa = p(a) + cz;
        let ab = p(a + 1) + cz;
        let b = p(cx + 1) + cy;
        let ba = p(b) + cz;
        let bb = p(b + 1) + cz;

        lerp(
            lerp(
                lerp(grad(p(aa), x, y, z), grad(p(ba), x1, y, z), u),
                lerp(grad(p(ab), x, y1, z), grad(p(bb), x1, y1, z), u),
                v,
            ),
            lerp(
                lerp(grad(p(aa + 1), x, y, z1), grad(p(ba + 1), x1, y, z1), u),
                lerp(grad(p(ab + 1), x, y1, z1), grad(p(bb + 1), x1, y1, z1), u),
                v,
            ),
            w,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values printed by three's own `ImprovedNoise.js` under Node.
    #[test]
    fn matches_three() {
        let noise = ImprovedNoise::new();
        for (x, y, z, expected) in [
            (0.5, 0.25, 0.125, -0.08358576893806458),
            (3.3, 1.7, 6.1, 0.08262725293125113),
            (-1.25, 2.5, 0.75, -0.16121816635131836),
            (6.45, 0.05, 3.2, 0.030666881056761217),
        ] {
            let got = noise.noise(x, y, z);
            assert!(
                (got - expected).abs() < 1e-15,
                "noise( {x}, {y}, {z} ) = {got}, three says {expected}"
            );
        }
    }
}
