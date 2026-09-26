//! Port of `three.js/examples/jsm/textures/FlakesTexture.js`.

/// `new FlakesTexture( width, height )` — a normal map of 4000 randomly
/// tilted round flakes over a flat `rgb( 127, 127, 255 )` ground, the car-paint
/// sparkle `webgpu_clearcoat` puts under its clearcoat.
///
/// three draws it on a 2D canvas and hands the canvas to a `CanvasTexture`;
/// there is no canvas here, so [`FlakesTexture::new`] rasterises the same
/// sequence of `arc()` fills into RGBA8 rows, top row first, which is what the
/// canvas upload reads. Two things decide whether the result is the page's
/// image:
///
/// * **The random sequence.** Each flake takes five `Math.random()` draws in
///   the order the JS makes them — `x`, `y`, the radius, then the normal's `x`
///   and `y` — so `random` must be the page's own sequence at the point the
///   constructor runs. Under the e2e harness that is the grader's seeded
///   generator after whatever the page drew before (`webgpu_clearcoat`: the
///   inspector's five).
/// * **The fill colour.** `'rgb(' + ( nx * 127 + 127 ) + ',' + … + ')'`
///   carries fractional channels; CSS rounds each to the nearest integer
///   (half away from zero) when the style is parsed, so the flake is painted
///   in the rounded colour.
///
/// The one approximation is the antialiasing at a flake's rim; see
/// `fill_circle`. Against the canvas Chrome paints for `webgpu_clearcoat`
/// (read back from the page, not from its screenshot) every flake interior is
/// exact and the mean difference over the whole image is under one level in
/// 255 — rim pixels differ by up to ~40 where two antialiased edges overlap,
/// which a normal map scaled by `0.15` and tiled ten times over a sphere does
/// not show.
pub struct FlakesTexture;

impl FlakesTexture {
    /// Generates the `width` × `height` RGBA8 image, drawing from `random`
    /// exactly as the JS draws from `Math.random`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(width: u32, height: u32, mut random: impl FnMut() -> f64) -> Vec<u8> {
        // `context.fillStyle = 'rgb(127,127,255)'; context.fillRect( … )`.
        let mut pixels = [127u8, 127, 255, 255].repeat((width * height) as usize);

        for _ in 0..4000 {
            let x = random() * width as f64;
            let y = random() * height as f64;
            let r = random() * 3.0 + 3.0;

            let mut nx = random() * 2.0 - 1.0;
            let mut ny = random() * 2.0 - 1.0;
            let mut nz = 1.5;

            let l = (nx * nx + ny * ny + nz * nz).sqrt();

            nx /= l;
            ny /= l;
            nz /= l;

            let color = [
                css_channel(nx * 127.0 + 127.0),
                css_channel(ny * 127.0 + 127.0),
                css_channel(nz * 255.0),
            ];

            fill_circle(&mut pixels, width, height, x, y, r, color);
        }

        pixels
    }
}

/// A CSS `rgb()` channel: clamped to `[ 0, 255 ]` and rounded to the nearest
/// integer.
fn css_channel(value: f64) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

/// `beginPath(); arc( x, y, r, 0, 2π ); fill()` with an opaque colour under
/// `source-over`: each pixel the disc touches moves towards `color` by the
/// disc's coverage of it. The canvas does not wrap, so a flake near an edge is
/// simply clipped.
///
/// The coverage is the distance ramp `clamp( r - d + 0.5, 0, 1 )`, `d` being
/// the distance from the pixel centre to the flake's — one pixel wide and
/// centred on the rim, which is the shape of the antialiasing Chrome's canvas
/// gives a filled arc.
fn fill_circle(pixels: &mut [u8], width: u32, height: u32, x: f64, y: f64, r: f64, color: [u8; 3]) {
    let reach = r + 0.5;
    let x0 = (x - reach).floor().max(0.0) as u32;
    let y0 = (y - reach).floor().max(0.0) as u32;
    let x1 = (x + reach).ceil().min(width as f64) as u32;
    let y1 = (y + reach).ceil().min(height as f64) as u32;

    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f64 + 0.5 - x;
            let dy = py as f64 + 0.5 - y;
            let coverage = (r - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
            if coverage == 0.0 {
                continue;
            }
            let i = ((py * width + px) * 4) as usize;
            for c in 0..3 {
                let dst = pixels[i + c] as f64;
                pixels[i + c] = (dst + (color[c] as f64 - dst) * coverage).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::DeterministicRandom;

    /// The first flake `webgpu_clearcoat` paints, read off the page with the
    /// grader's seeded `Math.random` after the inspector's five draws:
    /// `arc( 90.09996055904776, 339.59546484192833, 4.472265021508065, … )`
    /// in `#c378d7`.
    #[test]
    fn first_flake_matches_the_page() {
        let mut random = DeterministicRandom::new();
        random.skip(5);
        let draws: Vec<f64> = (0..5).map(|_| random.next()).collect();
        assert!((draws[0] * 512.0 - 90.09996055904776).abs() < 1e-9);
        assert!((draws[1] * 512.0 - 339.59546484192833).abs() < 1e-9);
        assert!((draws[2] * 3.0 + 3.0 - 4.472265021508065).abs() < 1e-9);

        let mut random = DeterministicRandom::new();
        random.skip(5);
        let pixels = FlakesTexture::new(512, 512, || random.next());
        assert_eq!(pixels.len(), 512 * 512 * 4);
        // Flake centres are painted solid in their own colour, unless a later
        // flake covers them; this one's centre is not covered.
        let i = (339 * 512 + 90) * 4;
        assert_eq!(&pixels[i..i + 4], &[0xc3, 0x78, 0xd7, 0xff]);
    }
}
