//! Port of `three.js/examples/jsm/loaders/HDRLoader.js` — the Radiance RGBE
//! (`.hdr`) decoder, itself adapted from Bruce Walter's `rgbe.c`.
//!
//! An RGBE file stores each texel as four bytes: a shared exponent and three
//! mantissas, so `( r, g, b ) * 2^( e - 128 ) / 255` is the linear colour. That
//! is a lossy but very compact HDR encoding, and it is what the pisa cube faces
//! `webgpu_pmrem_cubemap` loads are written in.
//!
//! The decode is pure CPU and has no GPU dependency, which is why it is graded
//! against three.js' own parser rather than against pixels: `tests/hdr/gen.mjs`
//! runs `HDRLoader.parse` under node over the six vendor faces and writes
//! `tests/hdr/oracle.json`, and `tests/hdr_loader.rs` asserts this decoder
//! reproduces it bit for bit — including
//! [`to_half_float`](crate::extras::to_half_float)'s truncating rounding.
//!
//! Two upstream behaviours are reproduced on purpose and flagged where they
//! happen: the flat (non-RLE) path returns *every remaining byte* of the file
//! rather than `width * height * 4` of them, and the exponent scale is computed
//! in f64 and only then rounded to f32, because JavaScript numbers are f64 and
//! `DataUtils.toHalfFloat` does the narrowing itself.

use std::path::Path;

use crate::error::Error;
use crate::extras::to_half_float;
use crate::textures::{MinFilter, Texture, TextureFilter, TextureType};

/// `HDRLoader.parse()`'s return value, less the fields that are constants
/// (`colorSpace: LinearSRGBColorSpace`, `minFilter`/`magFilter: LinearFilter`,
/// `generateMipmaps: false`, `flipY: true`).
#[derive(Debug, Clone)]
pub struct HdrTexData {
    pub width: u32,
    pub height: u32,
    /// The header text, `\n`-terminated per line, as upstream's `header`.
    pub header: String,
    /// `GAMMA=` in the header, 1.0 when absent.
    pub gamma: f64,
    /// `EXPOSURE=` in the header, 1.0 when absent.
    pub exposure: f64,
    pub data: HdrData,
}

/// The decoded texels, in whichever of the two types the loader was set to.
/// Both are RGBA — the alpha channel is a literal 1.
#[derive(Debug, Clone)]
pub enum HdrData {
    /// `HalfFloatType`: a `Uint16Array` of IEEE-754 binary16 bit patterns.
    HalfFloat(Vec<u16>),
    /// `FloatType`: a `Float32Array`.
    Float(Vec<f32>),
}

impl HdrData {
    /// The number of RGBA texels, whichever type this is.
    pub fn len(&self) -> usize {
        match self {
            HdrData::HalfFloat(data) => data.len() / 4,
            HdrData::Float(data) => data.len() / 4,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The channel values as the little-endian bytes a `write_texture` of an
    /// `rgba16float` / `rgba32float` texture wants.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            HdrData::HalfFloat(data) => data.iter().flat_map(|h| h.to_le_bytes()).collect(),
            HdrData::Float(data) => data.iter().flat_map(|f| f.to_le_bytes()).collect(),
        }
    }
}

/// `new HDRLoader()`.
#[derive(Debug, Clone)]
pub struct HdrLoader {
    texture_type: TextureType,
}

impl Default for HdrLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl HdrLoader {
    /// `new HDRLoader()` — `type = HalfFloatType`.
    pub fn new() -> Self {
        Self {
            texture_type: TextureType::HalfFloat,
        }
    }

    /// `loader.setDataType( value )`. Only `HalfFloatType` and `FloatType` are
    /// legal, which upstream discovers by throwing from `parse()`; here the
    /// mistake is caught where it is made.
    pub fn set_data_type(&mut self, texture_type: TextureType) -> Result<&mut Self, Error> {
        if !matches!(texture_type, TextureType::HalfFloat | TextureType::Float) {
            return Err(Error::UnsupportedTextureType {
                what: "HDR",
                texture_type,
            });
        }
        self.texture_type = texture_type;
        Ok(self)
    }

    pub fn data_type(&self) -> TextureType {
        self.texture_type
    }

    /// `loader.load( url )` — the file, decoded, as the `DataTexture`
    /// `DataTextureLoader` builds from `parse()`'s result.
    ///
    /// `DataTextureLoader.load()` copies `texData`'s `minFilter`, `magFilter`,
    /// `generateMipmaps` and `flipY` onto the texture, so this is
    /// `LinearFilter` on both, no mip chain, and `flipY = true`.
    /// The colour space is `LinearSRGBColorSpace`, which is the port's
    /// [`ColorSpace::NoColorSpace`](crate::textures::ColorSpace::NoColorSpace):
    /// the working space carries no transfer function, so nothing is applied on
    /// sample and no colour-space node appears in the generated WGSL.
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Texture, Error> {
        let path = path.as_ref();
        let bytes = crate::io::read(path)?;
        let tex_data = self.parse(&bytes)?;

        let texture = match &tex_data.data {
            HdrData::HalfFloat(data) => {
                Texture::data_rgba16float(tex_data.width, tex_data.height, data)
            }
            HdrData::Float(data) => {
                Texture::data_rgba32float(tex_data.width, tex_data.height, data)
            }
        };
        texture.set_flip_y(true);
        texture.set_generate_mipmaps(false);
        texture.set_min_filter(MinFilter::Linear);
        texture.set_mag_filter(TextureFilter::Linear);
        Ok(texture)
    }

    /// `HDRLoader.parse( buffer )`.
    pub fn parse(&self, buffer: &[u8]) -> Result<HdrTexData, Error> {
        let header = read_header(buffer)?;
        let (width, height) = (header.width, header.height);

        let rgba = read_pixels_rle(&buffer[header.body..], width, height)?;

        let texels = rgba.len() / 4;
        let data = match self.texture_type {
            TextureType::HalfFloat => {
                let mut out = vec![0u16; texels * 4];
                for j in 0..texels {
                    rgbe_to_rgb_half(&rgba[j * 4..j * 4 + 4], &mut out[j * 4..j * 4 + 4]);
                }
                HdrData::HalfFloat(out)
            }
            TextureType::Float => {
                let mut out = vec![0f32; texels * 4];
                for j in 0..texels {
                    rgbe_to_rgb_float(&rgba[j * 4..j * 4 + 4], &mut out[j * 4..j * 4 + 4]);
                }
                HdrData::Float(out)
            }
            texture_type => {
                return Err(Error::UnsupportedTextureType {
                    what: "HDR",
                    texture_type,
                })
            }
        };

        Ok(HdrTexData {
            width,
            height,
            header: header.string,
            gamma: header.gamma,
            exposure: header.exposure,
            data,
        })
    }
}

/// `RGBEByteToRGBFloat`.
fn rgbe_to_rgb_float(rgbe: &[u8], out: &mut [f32]) {
    let scale = scale_of(rgbe[3]);
    out[0] = (rgbe[0] as f64 * scale) as f32;
    out[1] = (rgbe[1] as f64 * scale) as f32;
    out[2] = (rgbe[2] as f64 * scale) as f32;
    out[3] = 1.0;
}

/// `RGBEByteToRGBHalf`, including its clamp at 65504 — which is what
/// `to_half_float` would do anyway, but upstream writes it out and a value
/// above it would otherwise warn.
fn rgbe_to_rgb_half(rgbe: &[u8], out: &mut [u16]) {
    let scale = scale_of(rgbe[3]);
    out[0] = to_half_float((rgbe[0] as f64 * scale).min(65504.0));
    out[1] = to_half_float((rgbe[1] as f64 * scale).min(65504.0));
    out[2] = to_half_float((rgbe[2] as f64 * scale).min(65504.0));
    out[3] = to_half_float(1.0);
}

/// `Math.pow( 2.0, e - 128.0 ) / 255.0`, in f64 as JavaScript computes it. The
/// narrowing to f32 happens after the multiply, inside the conversion, exactly
/// as the `Float32Array` / `Uint16Array` store does upstream; rounding the
/// scale to f32 first would double-round and move the last mantissa bit.
fn scale_of(exponent: u8) -> f64 {
    (2.0f64).powf(exponent as f64 - 128.0) / 255.0
}

struct Header {
    string: String,
    gamma: f64,
    exposure: f64,
    width: u32,
    height: u32,
    /// The offset of the first pixel byte — upstream's `byteArray.pos`.
    body: usize,
}

/// `RGBE_ReadHeader`.
///
/// Upstream's `fgets` reads in 128-byte chunks and gives up after a
/// `lineLimit` of 1024 characters; this scans for the newline a byte at a
/// time and has no limit. The two differ only for a header line longer than
/// ~1 KB, which upstream would reject as "no header found" and this would
/// accept — no `.hdr` file the port reads has one, and a line that long is not
/// a well-formed Radiance header either way.
fn read_header(buffer: &[u8]) -> Result<Header, Error> {
    let mut pos = 0usize;

    let mut line = fgets(buffer, &mut pos).ok_or_else(|| rgbe("Read Error: no header found"))?;

    // `if ( ! ( match = line.match( magic_token_re ) ) )` — `/^#\?(\S+)/`.
    if !(line.starts_with("#?") && line[2..].chars().next().is_some_and(|c| !c.is_whitespace())) {
        return Err(rgbe("Bad File Format: bad initial token"));
    }

    let mut string = String::new();
    let mut gamma = 1.0f64;
    let mut exposure = 1.0f64;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut have_format = false;
    let mut have_dimensions = false;

    string.push_str(&line);
    string.push('\n');

    loop {
        line = match fgets(buffer, &mut pos) {
            Some(line) => line,
            None => break,
        };
        string.push_str(&line);
        string.push('\n');

        if line.starts_with('#') {
            continue; // comment line
        }

        if let Some(value) = key_value(&line, "GAMMA") {
            if let Ok(value) = value.parse::<f64>() {
                gamma = value;
            }
        }

        if let Some(value) = key_value(&line, "EXPOSURE") {
            if let Ok(value) = value.parse::<f64>() {
                exposure = value;
            }
        }

        // `/^\s*FORMAT=(\S+)\s*$/` — note no whitespace is allowed around the
        // `=`, unlike GAMMA and EXPOSURE.
        if let Some(rest) = line.trim_start().strip_prefix("FORMAT=") {
            let value = rest.trim_end();
            if !value.is_empty() && !value.contains(char::is_whitespace) {
                have_format = true;
            }
        }

        // `/^\s*\-Y\s+(\d+)\s+\+X\s+(\d+)\s*$/`.
        if let Some((h, w)) = dimensions(&line) {
            have_dimensions = true;
            height = h;
            width = w;
        }

        if have_format && have_dimensions {
            break;
        }
    }

    if !have_format {
        return Err(rgbe("Bad File Format: missing format specifier"));
    }
    if !have_dimensions {
        return Err(rgbe("Bad File Format: missing image size specifier"));
    }

    Ok(Header {
        string,
        gamma,
        exposure,
        width,
        height,
        body: pos,
    })
}

/// One `\n`-terminated line, as Latin-1 — upstream builds it with
/// `String.fromCharCode` over the bytes, so each byte is one character.
/// Advances `pos` past the newline. `None` at the end of the buffer with no
/// newline left, which is upstream's `return false`.
fn fgets(buffer: &[u8], pos: &mut usize) -> Option<String> {
    if *pos >= buffer.len() {
        return None;
    }
    let newline = buffer[*pos..].iter().position(|&b| b == b'\n')?;
    let line = buffer[*pos..*pos + newline]
        .iter()
        .map(|&b| b as char)
        .collect();
    *pos += newline + 1;
    Some(line)
}

/// `/^\s*KEY\s*=\s*(\d+(\.\d+)?)\s*$/` for the two numeric header keys.
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix(key)?;
    let rest = rest.trim_start().strip_prefix('=')?.trim();
    // `(\d+(\.\d+)?)` — unsigned, no exponent, and the whole rest of the line.
    let mut parts = rest.splitn(2, '.');
    let integer = parts.next()?;
    if integer.is_empty() || !integer.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if let Some(fraction) = parts.next() {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    Some(rest)
}

/// `/^\s*\-Y\s+(\d+)\s+\+X\s+(\d+)\s*$/` — returns `( height, width )`, in the
/// resolution string's own order.
fn dimensions(line: &str) -> Option<(u32, u32)> {
    let mut fields = line.split_whitespace();
    if fields.next()? != "-Y" {
        return None;
    }
    let height = fields.next()?.parse().ok()?;
    if fields.next()? != "+X" {
        return None;
    }
    let width = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    Some((height, width))
}

/// `RGBE_ReadPixels_RLE`.
///
/// The adaptive-RLE scanline format stores each scanline's four channels one
/// after another, each run-length encoded on its own, which is why the
/// de-interleave at the bottom strides by `scanline_width`.
fn read_pixels_rle(buffer: &[u8], w: u32, h: u32) -> Result<Vec<u8>, Error> {
    let scanline_width = w as usize;

    // Not RLE, or a scanline too narrow or too wide for the format: the rest of
    // the file is already flat RGBE.
    //
    // Upstream returns `new Uint8Array( buffer )` here — the whole remaining
    // buffer, however long it is, so a file with trailing bytes decodes to more
    // texels than `width * height`. That is reproduced rather than truncated,
    // because `parse()`'s `numElements` is taken from the returned length and a
    // caller that trusted `width * height` would read past what upstream built.
    if !(8..=0x7fff).contains(&scanline_width)
        || buffer.len() < 4
        || buffer[0] != 2
        || buffer[1] != 2
        || buffer[2] & 0x80 != 0
    {
        return Ok(buffer.to_vec());
    }

    if scanline_width != (((buffer[2] as usize) << 8) | buffer[3] as usize) {
        return Err(rgbe("Bad File Format: wrong scanline width"));
    }

    let mut data_rgba = vec![0u8; 4 * w as usize * h as usize];
    if data_rgba.is_empty() {
        return Err(rgbe("Memory Error: unable to allocate buffer space"));
    }

    let mut offset = 0usize;
    let mut pos = 0usize;

    let ptr_end = 4 * scanline_width;
    let mut scanline_buffer = vec![0u8; ptr_end];
    let mut num_scanlines = h;

    while num_scanlines > 0 && pos < buffer.len() {
        if pos + 4 > buffer.len() {
            return Err(rgbe("Read Error: "));
        }

        let start = [
            buffer[pos],
            buffer[pos + 1],
            buffer[pos + 2],
            buffer[pos + 3],
        ];
        pos += 4;

        if start[0] != 2
            || start[1] != 2
            || (((start[2] as usize) << 8) | start[3] as usize) != scanline_width
        {
            return Err(rgbe("Bad File Format: bad rgbe scanline format"));
        }

        // Each of the four channels of the scanline, in order: red, green,
        // blue, exponent.
        let mut ptr = 0usize;
        while ptr < ptr_end && pos < buffer.len() {
            let mut count = buffer[pos] as usize;
            pos += 1;
            let is_encoded_run = count > 128;
            if is_encoded_run {
                count -= 128;
            }

            if count == 0 || ptr + count > ptr_end {
                return Err(rgbe("Bad File Format: bad scanline data"));
            }

            if is_encoded_run {
                let byte_value = buffer[pos];
                pos += 1;
                scanline_buffer[ptr..ptr + count].fill(byte_value);
                ptr += count;
            } else {
                // A literal run. `subarray` clamps at the end of the buffer, so
                // a truncated file copies what is there and leaves the rest of
                // the scanline at zero rather than failing.
                let end = (pos + count).min(buffer.len());
                let run = &buffer[pos..end];
                scanline_buffer[ptr..ptr + run.len()].copy_from_slice(run);
                ptr += count;
                pos += count;
            }
        }

        // De-interleave the four channel planes into RGBA texels.
        for i in 0..scanline_width {
            data_rgba[offset] = scanline_buffer[i];
            data_rgba[offset + 1] = scanline_buffer[i + scanline_width];
            data_rgba[offset + 2] = scanline_buffer[i + 2 * scanline_width];
            data_rgba[offset + 3] = scanline_buffer[i + 3 * scanline_width];
            offset += 4;
        }

        num_scanlines -= 1;
    }

    Ok(data_rgba)
}

/// `rgbe_error()` — the messages are upstream's, less the `THREE.HDRLoader: `
/// prefix that [`Error`]'s `Display` would duplicate.
fn rgbe(reason: &str) -> Error {
    Error::Rgbe {
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-texel flat (non-RLE) file: the scanline is under 8 wide, so
    /// `RGBE_ReadPixels_RLE` takes the flat path and hands the body back
    /// untouched.
    fn flat_hdr(body: &[u8], w: u32, h: u32) -> Vec<u8> {
        let mut file =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {h} +X {w}\n").into_bytes();
        file.extend_from_slice(body);
        file
    }

    #[test]
    fn the_header_carries_the_dimensions_gamma_and_exposure() {
        let file = b"#?RADIANCE\n# a comment\nGAMMA=2.2\nEXPOSURE= 0.5 \nFORMAT=32-bit_rle_rgbe\n\n-Y 3 +X 4\n\
                     \x00\x00\x00\x00".to_vec();
        let parsed = HdrLoader::new().parse(&file).unwrap();
        assert_eq!((parsed.width, parsed.height), (4, 3));
        assert_eq!(parsed.gamma, 2.2);
        assert_eq!(parsed.exposure, 0.5);
        assert!(parsed.header.starts_with("#?RADIANCE\n"));
    }

    #[test]
    fn a_file_without_the_magic_token_is_rejected() {
        let error = HdrLoader::new().parse(b"RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 1 +X 1\n");
        assert!(matches!(error, Err(Error::Rgbe { .. })));
    }

    #[test]
    fn a_file_without_a_format_line_is_rejected() {
        let error = HdrLoader::new().parse(b"#?RADIANCE\n\n-Y 1 +X 1\n");
        assert!(matches!(error, Err(Error::Rgbe { .. })));
    }

    /// `( 128, 128, 128, 129 )` is `128 * 2^1 / 255` = 1.00392… on all three
    /// channels, and the alpha is a literal 1.
    #[test]
    fn the_exponent_scale_is_the_rgbe_one() {
        let file = flat_hdr(&[128, 128, 128, 129], 1, 1);

        let mut loader = HdrLoader::new();
        loader.set_data_type(TextureType::Float).unwrap();
        let HdrData::Float(data) = loader.parse(&file).unwrap().data else {
            panic!("the loader was set to FloatType");
        };
        assert_eq!(data.len(), 4);
        let expected = (128.0f64 * 2.0f64.powf(1.0) / 255.0) as f32;
        assert_eq!(data[0], expected);
        assert_eq!(data[3], 1.0);

        let HdrData::HalfFloat(data) = HdrLoader::new().parse(&file).unwrap().data else {
            panic!("the loader defaults to HalfFloatType");
        };
        assert_eq!(data[0], to_half_float(128.0 * 2.0 / 255.0));
        // `toHalfFloat( 1 )`.
        assert_eq!(data[3], 0x3c00);
    }

    /// A zero exponent is a black texel, not `2^-128 / 255` of something: the
    /// mantissas are zero too, so the product is zero either way.
    #[test]
    fn a_zero_texel_decodes_to_zero() {
        let file = flat_hdr(&[0, 0, 0, 0], 1, 1);
        let HdrData::HalfFloat(data) = HdrLoader::new().parse(&file).unwrap().data else {
            unreachable!()
        };
        assert_eq!(data, vec![0, 0, 0, 0x3c00]);
    }

    /// The RLE path: one 8-wide scanline whose four channel planes are each a
    /// single encoded run, which must de-interleave into eight identical
    /// texels.
    #[test]
    fn an_rle_scanline_de_interleaves_into_texels() {
        let width = 8u32;
        let mut body = vec![2u8, 2, (width >> 8) as u8, width as u8];
        for channel in [10u8, 20, 30, 129] {
            // count | 0x80 means "a run of `count` copies of the next byte".
            body.push(0x80 | width as u8);
            body.push(channel);
        }
        let file = flat_hdr(&body, width, 1);

        let mut loader = HdrLoader::new();
        loader.set_data_type(TextureType::Float).unwrap();
        let HdrData::Float(data) = loader.parse(&file).unwrap().data else {
            unreachable!()
        };

        assert_eq!(data.len(), 8 * 4);
        let scale = 2.0f64.powf(1.0) / 255.0;
        for texel in data.chunks(4) {
            assert_eq!(texel[0], (10.0 * scale) as f32);
            assert_eq!(texel[1], (20.0 * scale) as f32);
            assert_eq!(texel[2], (30.0 * scale) as f32);
            assert_eq!(texel[3], 1.0);
        }
    }

    /// A literal run, the other half of the RLE alphabet.
    #[test]
    fn a_literal_run_is_copied_verbatim() {
        let width = 8u32;
        let mut body = vec![2u8, 2, 0, 8];
        for channel in 0..4u8 {
            body.push(width as u8); // count <= 128: a literal run
            for i in 0..width as u8 {
                body.push(if channel == 3 { 129 } else { i * 8 + channel });
            }
        }
        let file = flat_hdr(&body, width, 1);
        let HdrData::HalfFloat(data) = HdrLoader::new().parse(&file).unwrap().data else {
            unreachable!()
        };

        let scale = 2.0f64.powf(1.0) / 255.0;
        for (i, texel) in data.chunks(4).enumerate() {
            assert_eq!(texel[0], to_half_float((i as u8 * 8) as f64 * scale));
            assert_eq!(texel[1], to_half_float((i as u8 * 8 + 1) as f64 * scale));
        }
    }

    #[test]
    fn only_the_two_float_types_are_legal() {
        assert!(HdrLoader::new()
            .set_data_type(TextureType::UnsignedByte)
            .is_err());
        assert!(HdrLoader::new().set_data_type(TextureType::Float).is_ok());
    }
}
