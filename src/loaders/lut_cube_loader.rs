//! Port of `three.js/examples/jsm/loaders/LUTCubeLoader.js` — the Adobe /
//! Resolve `.cube` 3D colour lookup table, as `webgpu_postprocessing_3dlut`
//! loads five of them.
//!
//! The parse is upstream's, regex by regex (see `lut_text`): `TITLE`,
//! `LUT_3D_SIZE`, `DOMAIN_MIN` and `DOMAIN_MAX` are each the first match
//! anywhere in the file, and every line that is exactly three numeric tokens is
//! a table row, written in file order (red fastest, as the format specifies, so
//! no reordering is needed). Upstream's quirks are kept:
//!
//! - a row is stored as `value * 255` through a `Uint8Array`, which truncates
//!   and wraps rather than clamping — `-0.5` is 129, `2` is 254 — and a token
//!   `Number()` cannot read is NaN, stored as 0;
//! - rows past `size³` are dropped, and missing rows leave texels of zero
//!   (alpha included);
//! - `domainMin` / `domainMax` are parsed, checked and returned but never
//!   applied to the table, so a non-unit domain is the caller's to handle
//!   (`Lut3DNode` does not);
//! - `LUT_1D_SIZE` tables are not recognised: with no `LUT_3D_SIZE` the load
//!   fails with upstream's message.
//!
//! Not ported: `Loader`'s `manager`, `path` and `crossOrigin`, and the
//! callback-style `load()` — this one is synchronous and returns the result.
//! `tests/loaders_lut.rs` grades the parse against three.js' own, run under
//! node by `tests/lut/gen.mjs`.

use std::path::Path;

use super::lut_text::{
    data_points, digits, f32_bytes, find_after_keyword, quoted, table_length, three_numbers,
    to_uint8,
};
use crate::error::Error;
use crate::math::Vector3;
use crate::textures::{Data3DTexture, MinFilter, TextureFilter, TextureType, Wrapping};

const LOADER: &str = "LUTCubeLoader";

/// `LUTCubeLoader.parse()`'s return value.
#[derive(Debug, Clone)]
pub struct LutCube {
    /// The `TITLE "…"` string, or `None` (upstream's `null`) without one.
    pub title: Option<String>,
    /// `LUT_3D_SIZE`, the side of the cube.
    pub size: u32,
    /// `DOMAIN_MIN`, `( 0, 0, 0 )` by default. Returned, never applied.
    pub domain_min: Vector3,
    /// `DOMAIN_MAX`, `( 1, 1, 1 )` by default. Returned, never applied.
    pub domain_max: Vector3,
    /// The table: `size³` RGBA texels, red fastest, `rgba8unorm` for
    /// `UnsignedByteType` and `rgba32float` for `FloatType`; `LinearFilter`
    /// on both filters, `ClampToEdgeWrapping` on all three axes, no mips.
    pub texture_3d: Data3DTexture,
}

/// `new LUTCubeLoader()`.
#[derive(Debug, Clone)]
pub struct LutCubeLoader {
    texture_type: TextureType,
}

impl Default for LutCubeLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl LutCubeLoader {
    /// `new LUTCubeLoader()` — `type = UnsignedByteType`.
    pub fn new() -> Self {
        Self {
            texture_type: TextureType::UnsignedByte,
        }
    }

    /// `loader.setType( type )`. Upstream documents `UnsignedByteType` and
    /// `FloatType` and treats anything else as `FloatType`; here anything
    /// else is refused where the mistake is made.
    ///
    /// A `FloatType` table is `rgba32float`, which WebGPU only filters with
    /// the `float32-filterable` feature, as in a browser.
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

    /// `loader.load( url )`: the file as text (UTF-8, malformed sequences
    /// replaced, as `FileLoader`'s `TextDecoder` does), then
    /// [`parse`](Self::parse).
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<LutCube, Error> {
        let bytes = crate::io::read(path.as_ref())?;
        self.parse(&String::from_utf8_lossy(&bytes))
    }

    /// `loader.parse( input )`.
    pub fn parse(&self, input: &str) -> Result<LutCube, Error> {
        let title = find_after_keyword(input, "TITLE", quoted).map(str::to_owned);

        let size = find_after_keyword(input, "LUT_3D_SIZE", digits).ok_or_else(|| Error::Lut {
            loader: LOADER,
            reason: "Missing LUT_3D_SIZE information".into(),
        })?;
        let unsigned_byte = self.texture_type == TextureType::UnsignedByte;
        let (side, length) = table_length(LOADER, size, if unsigned_byte { 1 } else { 4 })?;

        let mut domain_min = [0.0, 0.0, 0.0];
        let mut domain_max = [1.0, 1.0, 1.0];
        if let Some(min) = find_after_keyword(input, "DOMAIN_MIN", three_numbers) {
            domain_min = min;
        }
        if let Some(max) = find_after_keyword(input, "DOMAIN_MAX", three_numbers) {
            domain_max = max;
        }
        // NaN compares false, so an unreadable bound passes, as upstream.
        if domain_min[0] > domain_max[0]
            || domain_min[1] > domain_max[1]
            || domain_min[2] > domain_max[2]
        {
            return Err(Error::Lut {
                loader: LOADER,
                reason: "Invalid input domain".into(),
            });
        }

        let scale = if unsigned_byte { 255.0 } else { 1.0 };
        let values =
            data_points(input).flat_map(|[r, g, b]| [r * scale, g * scale, b * scale, scale]);
        let (bytes, format) = if unsigned_byte {
            let mut data = vec![0u8; length];
            // `data[ i ++ ] = …` past the end of a typed array is a no-op.
            for (slot, value) in data.iter_mut().zip(values) {
                *slot = to_uint8(value);
            }
            (data, wgpu::TextureFormat::Rgba8Unorm)
        } else {
            let mut data = vec![0f32; length];
            for (slot, value) in data.iter_mut().zip(values) {
                *slot = value as f32;
            }
            (f32_bytes(&data), wgpu::TextureFormat::Rgba32Float)
        };

        Ok(LutCube {
            title,
            size: side,
            domain_min: Vector3::new(domain_min[0], domain_min[1], domain_min[2]),
            domain_max: Vector3::new(domain_max[0], domain_max[1], domain_max[2]),
            texture_3d: lut_texture(bytes, side, format),
        })
    }
}

/// The `Data3DTexture` every LUT loader ends on: `LinearFilter` both ways,
/// `ClampToEdgeWrapping` on all three axes. `generateMipmaps = false` is the
/// `Data3DTexture` default here already.
pub(super) fn lut_texture(bytes: Vec<u8>, size: u32, format: wgpu::TextureFormat) -> Data3DTexture {
    let texture = Data3DTexture::new(bytes, size, size, size, format);
    texture.set_min_filter(MinFilter::Linear);
    texture.set_mag_filter(TextureFilter::Linear);
    texture.set_wrapping(
        Wrapping::ClampToEdge,
        Wrapping::ClampToEdge,
        Wrapping::ClampToEdge,
    );
    texture
}
