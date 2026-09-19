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
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        // `ImageLoader` gives the bytes to the browser, which picks the decoder
        // from the sniffed type and not from the extension. Three magic numbers
        // cover everything the ladder loads.
        let image = decode_image(path, &bytes)?;
        Ok(Texture::new(image.width, image.height, Some(image.data)))
    }

    /// The bytes of an image the loader never sees as a file: a glTF image
    /// stored in the BIN chunk or a data URI. `ImageBitmapLoader` /
    /// `createImageBitmap` in the browser; here the decode is picked by the
    /// glTF `mimeType` when there is one and by the file's magic otherwise,
    /// because `images[ i ].mimeType` is only required for a `bufferView`
    /// source.
    ///
    /// The `Texture` comes back with the same defaults `load()` gives one;
    /// `GLTFLoader` is what sets `flipY`, the wrapping and the colour space.
    pub fn from_bytes(&self, bytes: &[u8], mime_type: Option<&str>) -> Result<Texture, Error> {
        let png = match mime_type {
            Some("image/png") => true,
            Some("image/jpeg") => false,
            _ => bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        };

        let image = if png {
            decode_png(Path::new("<glTF image>"), bytes)?
        } else {
            decode_jpeg_bytes(Path::new("<glTF image>"), bytes)?
        };

        Ok(Texture::new(image.width, image.height, Some(image.data)))
    }
}

/// `ImageLoader` gives the bytes to the browser, which picks the decoder from
/// the sniffed type and not from the extension. Three magic numbers cover
/// everything the ladder loads. Shared with [`CubeTextureLoader`], whose six
/// faces are PNG in `webgpu_materials_basic` and JPEG in
/// `webgpu_pmrem_scene`.
///
/// [`CubeTextureLoader`]: super::CubeTextureLoader
pub(crate) fn decode_image(path: &Path, bytes: &[u8]) -> Result<crate::textures::Image, Error> {
    if bytes.starts_with(b"GIF") {
        super::gif::decode(path, bytes)
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        decode_png(path, bytes)
    } else {
        decode_jpeg_bytes(path, bytes)
    }
}

/// `createImageBitmap` on a PNG: 8-bit RGBA, top-down, with every source
/// format expanded to it (palette and grey both reach the GPU as RGBA8 in the
/// browser too). Unlike the JPEG path this is exact — PNG is lossless and the
/// decoders agree byte for byte.
fn decode_png(path: &Path, bytes: &[u8]) -> Result<crate::textures::Image, Error> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // `EXPAND` takes palette, grey and `tRNS` up to 8-bit RGB/RGBA, so the only
    // cases left below are the four 8-bit ones (and 16-bit, which is stripped).
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);

    let mut reader = decoder
        .read_info()
        .map_err(|e| Error::image(path, e.to_string()))?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| Error::image(path, e.to_string()))?;
    buffer.truncate(info.buffer_size());

    let pixels = (info.width as usize) * (info.height as usize);
    let data = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity(pixels * 4);
            for pixel in buffer.as_chunks::<3>().0 {
                out.extend_from_slice(pixel);
                out.push(255);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity(pixels * 4);
            for &value in &buffer {
                out.extend_from_slice(&[value, value, value, 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity(pixels * 4);
            for pixel in buffer.as_chunks::<2>().0 {
                out.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
            }
            out
        }
        png::ColorType::Indexed => {
            return Err(Error::image(path, "the PNG palette was not expanded"));
        }
    };

    Ok(crate::textures::Image {
        width: info.width,
        height: info.height,
        data,
    })
}

/// What the browser hands `copyExternalImageToTexture`: 8-bit RGBA, top-down,
/// opaque.
///
/// Chromium decodes through libjpeg-turbo and this goes through `zune-jpeg`,
/// so individual samples can differ by the rounding of the inverse DCT; the
/// PNG and GIF paths beside it are exact.
fn decode_jpeg_bytes(path: &Path, bytes: &[u8]) -> Result<crate::textures::Image, Error> {
    let options = zune_jpeg::zune_core::options::DecoderOptions::default()
        .jpeg_set_out_colorspace(zune_jpeg::zune_core::colorspace::ColorSpace::RGBA);

    let mut decoder =
        zune_jpeg::JpegDecoder::new_with_options(std::io::Cursor::new(bytes), options);

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
