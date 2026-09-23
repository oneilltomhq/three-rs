//! Port of `three.js/examples/jsm/loaders/HDRCubeTextureLoader.js`.
//!
//! Six `.hdr` files through [`HdrLoader`] into one [`CubeTexture`], in the face
//! order `px, nx, py, ny, pz, nz` — the order the array is given in, which is
//! WebGPU's `+X −X +Y −Y +Z −Z` layer order, so face *i* becomes array layer
//! *i* with no reshuffling.
//!
//! Orientation is the part that is silently wrong if it is got wrong. Upstream
//! wraps each decoded face in a `DataTexture`, which leaves `flipY` at
//! `DataTexture`'s `false` — it does **not** take `HDRLoader`'s
//! `texData.flipY = true`, which only `DataTextureLoader.load()` would have
//! applied. `CubeTexture.flipY` is `false` as well. So the faces are uploaded
//! exactly as the RGBE scanlines were decoded, and a Radiance `-Y h +X w`
//! resolution string means the first scanline is the *top* row. Any flip added
//! here would mirror the environment vertically and still produce a plausible
//! image.
//!
//! The example's own call:
//!
//! ```no_run
//! use three_rs::loaders::HdrCubeTextureLoader;
//!
//! let map = HdrCubeTextureLoader::new()
//!     .set_path("examples/textures/cube/pisaHDR/")
//!     .load(["px.hdr", "nx.hdr", "py.hdr", "ny.hdr", "pz.hdr", "nz.hdr"])
//!     .unwrap();
//! ```

use std::path::{Path, PathBuf};

use super::hdr_loader::{HdrData, HdrLoader};
use crate::error::Error;
use crate::textures::{ColorSpace, CubeTexture, Image, MinFilter, TextureFilter, TextureType};

/// `new HDRCubeTextureLoader()`.
#[derive(Debug, Clone)]
pub struct HdrCubeTextureLoader {
    /// `Loader.path`, prefixed to every url.
    path: PathBuf,
    texture_type: TextureType,
}

impl Default for HdrCubeTextureLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl HdrCubeTextureLoader {
    /// `new HDRCubeTextureLoader()` — `type = HalfFloatType`.
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            texture_type: TextureType::HalfFloat,
        }
    }

    /// `loader.setPath( path )`.
    pub fn set_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_path_buf();
        self
    }

    /// `loader.setDataType( value )` — `HalfFloatType` or `FloatType`.
    pub fn set_data_type(mut self, texture_type: TextureType) -> Result<Self, Error> {
        HdrLoader::new().set_data_type(texture_type)?;
        self.texture_type = texture_type;
        Ok(self)
    }

    /// `loader.load( urls, onLoad )`, synchronously.
    ///
    /// The page's load is asynchronous, but the e2e harness fires its single
    /// RAF only once the network is idle, so the six faces are always present
    /// for the graded frame; here they are decoded in order, as
    /// `CubeTextureLoader` already is.
    ///
    /// The texture comes back with `colorSpace = LinearSRGBColorSpace`
    /// ([`ColorSpace::NoColorSpace`] — the working space, no transfer
    /// function), `minFilter = magFilter = LinearFilter` and
    /// `generateMipmaps = false`. The mip policy matters downstream: PMREM
    /// builds its own mip pyramid inside a 2-D cubeUV atlas and never samples a
    /// mip of the source cube, so generating one here would be six wasted
    /// blits, and the atlas would still look identical — which is why the dump
    /// shows the HDR cube with `mips: 1`.
    pub fn load<P: AsRef<Path>>(&self, urls: [P; 6]) -> Result<CubeTexture, Error> {
        let mut loader = HdrLoader::new();
        loader.set_data_type(self.texture_type)?;

        let mut images = Vec::with_capacity(6);
        for url in urls.iter() {
            let path = self.path.join(url.as_ref());
            let bytes = crate::io::read(&path)?;
            let tex_data = loader.parse(&bytes)?;

            // `new DataTexture( texData.data, texData.width, texData.height )`.
            let image = match &tex_data.data {
                HdrData::HalfFloat(data) => {
                    Image::rgba16float(tex_data.width, tex_data.height, data)
                }
                HdrData::Float(_) => {
                    return Err(Error::UnsupportedFormat {
                        what: "HDR cube texture type",
                        value: "FloatType (rgba32float cube faces are not uploaded yet)".into(),
                    })
                }
            };
            images.push(image);
        }

        let texture = CubeTexture::new(images);
        texture.set_texture_type(self.texture_type)?;
        texture.set_color_space(ColorSpace::NoColorSpace);
        texture.set_filters(MinFilter::Linear, TextureFilter::Linear);
        texture.set_generate_mipmaps(false);
        Ok(texture)
    }
}
