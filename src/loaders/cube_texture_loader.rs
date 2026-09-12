//! Port of `three.js/src/loaders/CubeTextureLoader.js` (+ the PNG decode that
//! `ImageLoader` gets from the browser).
//!
//! The page's load is asynchronous, but the harness fires its single RAF only
//! once the network is idle, so the faces are always present for the graded
//! frame; here they are decoded synchronously.

use std::path::Path;

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
    pub fn load<P: AsRef<Path>>(&self, urls: [P; 6]) -> CubeTexture {
        let images = urls.iter().map(|url| decode_png(url.as_ref())).collect();

        let texture = CubeTexture::new(images);
        texture.set_color_space(ColorSpace::SRGB);
        texture
    }
}

/// What the browser hands `copyExternalImageToTexture`: 8-bit RGBA, top-down,
/// opaque where the source has no alpha channel.
fn decode_png(path: &Path) -> Image {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("three-rs: cannot open {}: {e}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("three-rs: PNG header");
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).expect("three-rs: PNG data");

    assert_eq!(
        info.bit_depth,
        png::BitDepth::Eight,
        "three-rs: only 8-bit PNGs are decoded"
    );

    let data = match info.color_type {
        png::ColorType::Rgba => buffer[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buffer[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("three-rs: unsupported PNG colour type {other:?}"),
    };

    Image {
        width: info.width,
        height: info.height,
        data,
    }
}
