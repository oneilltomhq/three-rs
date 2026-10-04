//! Port of `three.js/examples/jsm/loaders/LUT3dlLoader.js` — the Autodesk /
//! Lustre `.3dl` 3D colour lookup table `webgpu_postprocessing_3dlut` loads
//! as `Presetpro-Cinematic.3dl`.
//!
//! The parse is upstream's (see `lut_text` for the regexes): the first line
//! made only of digits and spaces is the grid, whose length is the table size
//! and whose steps must all be equal; every line that is exactly three numeric
//! tokens is a row, blue fastest, transposed into the red-fastest order a 3D
//! texture wants; and the whole table is divided by the smallest power of two
//! at or above its largest value, so 10-, 12- and 16-bit tables all land in
//! `[0, 1]`. Upstream's quirks are kept:
//!
//! - the grid line is matched by the row pattern too when it has exactly three
//!   entries, so a size-3 table reads it as its first row;
//! - a grid line of spaces only is a grid of `[ 0 ]` — size 1;
//! - an all-zero table divides by `2^-Infinity = 0` and is NaN throughout
//!   (stored as 0 in a `Uint8Array`), and a token `Number()` cannot read makes
//!   the maximum, and so every texel, NaN;
//! - a `Uint8Array` store truncates and wraps rather than clamping, and rows
//!   past `size³` overwrite earlier texels, the layer indices being taken
//!   modulo `size`.
//!
//! Not ported: `Loader`'s `manager`, `path` and `crossOrigin`, and the
//! callback-style `load()`. `tests/loaders_lut.rs` grades the parse against
//! three.js' own, run under node by `tests/lut/gen.mjs`.

use std::path::Path;

use super::lut_cube_loader::lut_texture;
use super::lut_text::{data_points, f32_bytes, js_max, lines, number, table_length, to_uint8};
use crate::error::Error;
use crate::textures::{Data3DTexture, TextureType};

const LOADER: &str = "LUT3dlLoader";

/// `LUT3dlLoader.parse()`'s return value.
#[derive(Debug, Clone)]
pub struct Lut3dl {
    /// The grid length — the side of the cube.
    pub size: u32,
    /// The table: `size³` RGBA texels, red fastest, `rgba8unorm` for
    /// `UnsignedByteType` and `rgba32float` for `FloatType`; `LinearFilter`
    /// on both filters, `ClampToEdgeWrapping` on all three axes, no mips.
    pub texture_3d: Data3DTexture,
}

/// `new LUT3dlLoader()`.
#[derive(Debug, Clone)]
pub struct Lut3dlLoader {
    texture_type: TextureType,
}

impl Default for Lut3dlLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Lut3dlLoader {
    /// `new LUT3dlLoader()` — `type = UnsignedByteType`.
    pub fn new() -> Self {
        Self {
            texture_type: TextureType::UnsignedByte,
        }
    }

    /// `loader.setType( type )`: `UnsignedByteType` or `FloatType`, anything
    /// else refused here (upstream treats it as `FloatType`). A `FloatType`
    /// table is `rgba32float`, filterable only with `float32-filterable`.
    pub fn set_type(&mut self, texture_type: TextureType) -> Result<&mut Self, Error> {
        if !matches!(texture_type, TextureType::UnsignedByte | TextureType::Float) {
            return Err(Error::UnsupportedTextureType {
                what: "LUT",
                texture_type,
            });
        }
        self.texture_type = texture_type;
        Ok(self)
    }

    /// `loader.type`.
    pub fn texture_type(&self) -> TextureType {
        self.texture_type
    }

    /// `loader.load( url )`: the file as text, then [`parse`](Self::parse).
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Lut3dl, Error> {
        let bytes = crate::io::read(path.as_ref())?;
        self.parse(&String::from_utf8_lossy(&bytes))
    }

    /// `loader.parse( input )`.
    pub fn parse(&self, input: &str) -> Result<Lut3dl, Error> {
        // `/^[\d ]+$/m`: the first whole line of digits and spaces.
        let grid = lines(input)
            .find(|line| !line.is_empty() && line.chars().all(|c| c.is_ascii_digit() || c == ' '))
            .ok_or_else(|| Error::Lut {
                loader: LOADER,
                reason: "Missing grid information".into(),
            })?;
        // `.trim().split( /\s+/g ).map( Number )`: '' splits to [ '' ], which
        // is [ 0 ].
        let grid_lines: Vec<f64> = match grid.trim() {
            "" => vec![0.0],
            trimmed => trimmed.split_whitespace().map(number).collect(),
        };
        // `gridLines[ 1 ] - gridLines[ 0 ]` is `undefined - n`, NaN, for a
        // one-entry grid; the check below never runs then.
        let grid_step = grid_lines
            .get(1)
            .map_or(f64::NAN, |second| second - grid_lines[0]);
        if grid_lines
            .windows(2)
            .any(|pair| grid_step != pair[1] - pair[0])
        {
            return Err(Error::Lut {
                loader: LOADER,
                reason: "Inconsistent grid size".into(),
            });
        }

        let unsigned_byte = self.texture_type == TextureType::UnsignedByte;
        let (size, length) = table_length(
            LOADER,
            grid_lines.len() as f64,
            if unsigned_byte { 1 } else { 4 },
        )?;
        let side = size as usize;
        let side_sq = side * side;

        let mut data_float = vec![0f32; length];
        let mut max_value = 0.0f64;
        for (index, [r, g, b]) in data_points(input).enumerate() {
            max_value = js_max(js_max(js_max(max_value, r), g), b);

            let b_layer = index % side;
            let g_layer = (index / side) % side;
            let r_layer = (index / side_sq) % side;

            // b grows first, then g, then r.
            let d4 = (b_layer * side_sq + g_layer * side + r_layer) * 4;
            data_float[d4] = r as f32;
            data_float[d4 + 1] = g as f32;
            data_float[d4 + 2] = b as f32;
        }

        // Determine the bit depth to scale the values to [0.0, 1.0].
        let bits = max_value.log2().ceil();
        let max_bit_value = 2f64.powf(bits);

        let scale = if unsigned_byte { 255.0 } else { 1.0 };
        let texel = |i: usize, c: usize| f64::from(data_float[i + c]) / max_bit_value * scale;
        let (bytes, format) = if unsigned_byte {
            let mut data = vec![0u8; length];
            for i in (0..length).step_by(4) {
                data[i] = to_uint8(texel(i, 0));
                data[i + 1] = to_uint8(texel(i, 1));
                data[i + 2] = to_uint8(texel(i, 2));
                data[i + 3] = 255;
            }
            (data, wgpu::TextureFormat::Rgba8Unorm)
        } else {
            // `data` is `dataFloat` itself for `FloatType`: each element is
            // read once, then overwritten.
            let mut data = vec![0f32; length];
            for i in (0..length).step_by(4) {
                data[i] = texel(i, 0) as f32;
                data[i + 1] = texel(i, 1) as f32;
                data[i + 2] = texel(i, 2) as f32;
                data[i + 3] = 1.0;
            }
            (f32_bytes(&data), wgpu::TextureFormat::Rgba32Float)
        };

        Ok(Lut3dl {
            size,
            texture_3d: lut_texture(bytes, size, format),
        })
    }
}
