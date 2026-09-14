//! Port of `three.js/src/loaders/TextureLoader.js` (+ the JPEG decode that
//! `ImageLoader` gets from the browser).
//!
//! The page's load is asynchronous, but the harness fires its single RAF only
//! once the network is idle, so the image is always present for the graded
//! frame; here it is decoded synchronously.

use std::path::Path;

use crate::error::Error;
use crate::textures::Texture;

pub struct TextureLoader;

impl Default for TextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureLoader {
    pub fn new() -> Self {
        Self
    }

    /// `new TextureLoader().load( url )`. The loader leaves every `Texture`
    /// default in place — `flipY` true, `generateMipmaps` true,
    /// `LinearFilter` / `LinearMipmapLinearFilter`, `ClampToEdgeWrapping`,
    /// anisotropy 1, `NoColorSpace` (the example never sets `colorSpace`, so
    /// `uv_grid_opengl.jpg` is sampled as raw bytes and no sRGB decode
    /// happens in the shader).
    pub fn load<P: AsRef<Path>>(&self, url: P) -> Result<Texture, Error> {
        let path = url.as_ref();
        let image = decode_jpeg(path)?;
        Ok(Texture::new(image.width, image.height, Some(image.data)))
    }
}

/// What the browser hands `copyExternalImageToTexture`: 8-bit RGBA, top-down,
/// opaque.
///
/// Chromium decodes through libjpeg-turbo and this goes through `zune-jpeg`,
/// so individual samples can differ by the rounding of the inverse DCT. The
/// test `jpeg_decode_matches_browser` measures that against the browser's own
/// decode of this exact file and holds it to ±1 per channel.
fn decode_jpeg(path: &Path) -> Result<crate::textures::Image, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;

    let options = zune_jpeg::zune_core::options::DecoderOptions::default()
        .jpeg_set_out_colorspace(zune_jpeg::zune_core::colorspace::ColorSpace::RGBA);

    let mut decoder =
        zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(&bytes[..]), options);

    let data = decoder
        .decode()
        .map_err(|e| Error::image(path, e.to_string()))?;
    let (width, height) = decoder
        .dimensions()
        .ok_or_else(|| Error::image(path, "the JPEG has no dimensions"))?;

    Ok(crate::textures::Image {
        width: width as u32,
        height: height as u32,
        data,
    })
}
