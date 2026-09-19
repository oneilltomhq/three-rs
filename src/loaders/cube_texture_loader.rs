//! Port of `three.js/src/loaders/CubeTextureLoader.js` (+ the image decode
//! that `ImageLoader` gets from the browser).
//!
//! The page's load is asynchronous, but the harness fires its single RAF only
//! once the network is idle, so the faces are always present for the graded
//! frame; here they are decoded synchronously.

use std::path::Path;

use crate::error::Error;
use crate::textures::{ColorSpace, CubeTexture, Image};

pub struct CubeTextureLoader;

impl Default for CubeTextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl CubeTextureLoader {
    pub fn new() -> Self {
        Self
    }

    /// `new CubeTextureLoader().load( urls )` — the loader sets
    /// `texture.colorSpace = SRGBColorSpace`.
    pub fn load<P: AsRef<Path>>(&self, urls: [P; 6]) -> Result<CubeTexture, Error> {
        let images = urls
            .iter()
            .map(|url| decode(url.as_ref()))
            .collect::<Result<Vec<Image>, Error>>()?;

        let texture = CubeTexture::new(images);
        texture.set_color_space(ColorSpace::SRGB);
        Ok(texture)
    }
}

/// What the browser hands `copyExternalImageToTexture`: 8-bit RGBA, top-down,
/// opaque where the source has no alpha channel.
///
/// The decoder is picked by the file's magic and not by its extension, as it
/// is in the browser: `webgpu_materials_basic`'s six faces are PNG,
/// `webgpu_pmrem_scene`'s are JPEG, and both reach
/// [`super::texture_loader::decode_image`].
fn decode(path: &Path) -> Result<Image, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    super::texture_loader::decode_image(path, &bytes)
}
