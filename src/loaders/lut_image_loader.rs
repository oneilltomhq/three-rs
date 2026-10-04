//! Port of `three.js/examples/jsm/loaders/LUTImageLoader.js` — a 3D colour
//! lookup table stored as a strip of square slices in an ordinary image, the
//! Unreal / Unity convention. `webgpu_postprocessing_3dlut` loads three:
//! `NeutralLUT.png`, `B&WLUT.png` and `NightLUT.png`, each a 1024×32 strip
//! of 32 slices.
//!
//! Upstream draws the image onto a 2D canvas and reads it back with
//! `getImageData`: as-is when the image is taller than wide (the slices
//! already stacked), or slice by slice into a vertical stack when it is not
//! (`_horz2Vert`). The port does the same copies on the decoded RGBA8 texels
//! (`texture_loader::decode_image`). A canvas round-trip is exact
//! for an opaque image with no colour profile, which the vendor LUTs are (no
//! `gAMA`, `iCCP` or `sRGB` chunk; a grey PNG reads back as `( v, v, v, 255 )`
//! on both sides); one with partial alpha would come back premultiplied and
//! un-premultiplied by the canvas, which the port does not imitate.
//!
//! `flip = true` keeps upstream's transform exactly, including its off-by-one
//! in `_horz2Vert`: the canvas is flipped with `scale( 1, -1 )` and
//! `translate( 0, -height )`, and slice `i` is then drawn at `height - i *
//! size`, so slice 0 lands above the canvas and is lost, slice `i` fills rows
//! `( i - 1 ) * size ..` upside down, and the last `size` rows stay
//! transparent black. `tests/loaders_lut.rs` checks both settings against
//! three's own loader run in Chrome by `tests/lut/gen.mjs`.
//!
//! Not ported: `Loader`'s `manager`, `path` and `crossOrigin`, and the
//! callback-style `load()`. Where the image is not `size` slices of `size²`
//! texels, upstream builds a `Data3DTexture` whose data does not match its
//! dimensions and leaves the failure to the GPU upload; this port refuses it
//! in `load()` with an [`Error::Lut`].

use std::path::Path;

use super::lut_cube_loader::lut_texture;
use super::texture_loader::decode_image;
use crate::error::Error;
use crate::textures::Data3DTexture;

const LOADER: &str = "LUTImageLoader";

/// `LUTImageLoader.parse()`'s return value.
#[derive(Debug, Clone)]
pub struct LutImage {
    /// The side of the cube — the shorter image dimension.
    pub size: u32,
    /// The table: `size³` `rgba8unorm` texels, red fastest; `LinearFilter`
    /// on both filters, `ClampToEdgeWrapping` on all three axes, no mips.
    pub texture_3d: Data3DTexture,
}

/// `new LUTImageLoader()`.
#[derive(Debug, Clone, Default)]
pub struct LutImageLoader {
    /// `loader.flip`, `false` by default: whether green runs top to bottom
    /// (Unity URP colour lookup strips) rather than bottom to top (Unreal).
    pub flip: bool,
}

impl LutImageLoader {
    /// `new LUTImageLoader()` — `flip = false`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `loader.load( url )`: decode the image, stack its slices vertically,
    /// [`parse`](Self::parse) with `Math.min( width, height )`.
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<LutImage, Error> {
        let path = path.as_ref();
        let bytes = crate::io::read(path)?;
        let image = decode_image(path, &bytes)?;
        let data = if image.width < image.height {
            self.image_data(image.width, image.height, &image.data)
        } else {
            self.horz_to_vert(image.width, image.height, &image.data)
        };
        let size = image.width.min(image.height);
        let expected = (size as usize).pow(3) * 4;
        if data.len() != expected {
            return Err(Error::Lut {
                loader: LOADER,
                reason: format!(
                    "a {}x{} image is not {size} slices of {size}x{size}",
                    image.width, image.height
                ),
            });
        }
        Ok(self.parse(data, size))
    }

    /// `loader.parse( dataArray, size )`: `data` is `size³` RGBA8 texels, red
    /// fastest. Panics if it is not, as [`Data3DTexture::new`] does.
    pub fn parse(&self, data: Vec<u8>, size: u32) -> LutImage {
        LutImage {
            size,
            texture_3d: lut_texture(data, size, wgpu::TextureFormat::Rgba8Unorm),
        }
    }

    /// `_getImageData( texture )`: the image as-is, or upside down with `flip`.
    fn image_data(&self, width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
        let row = width as usize * 4;
        if !self.flip {
            return pixels.to_vec();
        }
        let mut out = vec![0u8; pixels.len()];
        for y in 0..height as usize {
            let flipped = height as usize - 1 - y;
            out[flipped * row..][..row].copy_from_slice(&pixels[y * row..][..row]);
        }
        out
    }

    /// `_horz2Vert( texture )`: a `size × width` canvas, `size = height`,
    /// with slice `i` (source columns `i * size ..`) drawn at rows `i * size
    /// ..` — or, with `flip`, through upstream's flipped transform (see the
    /// module doc). Source columns past the image are transparent, as
    /// `drawImage` clips its source rectangle.
    fn horz_to_vert(&self, image_width: u32, image_height: u32, pixels: &[u8]) -> Vec<u8> {
        let size = image_height as usize;
        let (canvas_width, canvas_height) = (size, image_width as usize);
        let mut out = vec![0u8; canvas_width * canvas_height * 4];
        for i in 0..size {
            for r in 0..size {
                // Where source row `r` of slice `i` lands on the canvas.
                let dest = if self.flip {
                    // y' = height - ( height - i * size + r ) - 1, for the
                    // pixel row [ y, y + 1 ) mapped through y' = height - y.
                    match (i * size).checked_sub(r + 1) {
                        Some(row) => row,
                        None => continue,
                    }
                } else {
                    i * size + r
                };
                if dest >= canvas_height {
                    continue;
                }
                for x in 0..size {
                    let sx = i * size + x;
                    if sx >= image_width as usize {
                        break;
                    }
                    let from = (r * image_width as usize + sx) * 4;
                    let to = (dest * canvas_width + x) * 4;
                    out[to..to + 4].copy_from_slice(&pixels[from..from + 4]);
                }
            }
        }
        out
    }
}
