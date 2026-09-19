//! Port of `three.js/examples/jsm/loaders/UltraHDRLoader.js`.
//!
//! An UltraHDR file is an ordinary baseline JPEG — the SDR rendition — with a
//! second JPEG, the *gain map*, appended after its `EOI`, and two pieces of
//! metadata that say how to combine them:
//!
//! * an **MPF** (Multi-Picture Format) `APP2` segment in the first image's
//!   header, holding the byte offset and length of each of the two streams; and
//! * the gain-map parameters, either as Adobe's `hdrgm:` **XMP** attributes in
//!   an `APP1` segment (the legacy encoding, and what every asset in three's
//!   `examples/textures/` uses) or as the binary **ISO 21496-1** block in an
//!   `APP2` segment (the current standard).
//!
//! Decoding is then per channel: raise the gain-map sample to `1 / gamma`,
//! interpolate between `gainMapMin` and `gainMapMax` in log2 space, scale by a
//! weight derived from the display's headroom, and apply that as an exponent to
//! the SDR value before an sRGB-to-linear transfer. [`apply_gain_map`] is that
//! loop, and the doc comment there lists the three places it reproduces a
//! quirk of upstream's arithmetic on purpose.
//!
//! # What differs from the browser
//!
//! Upstream decodes both JPEGs with `createImageBitmap` and reads them back
//! through a 2-D canvas, which does three things this port does differently:
//!
//! * **JPEG decode.** Chromium goes through libjpeg-turbo, this through
//!   `zune-jpeg` (the same decoder [`TextureLoader`](super::TextureLoader)
//!   already uses), so individual samples can differ by the rounding of the
//!   inverse DCT.
//! * **The ICC profile.** `createImageBitmap` converts the decoded image into
//!   the canvas' `srgb` colour space. Every UltraHDR asset three ships carries
//!   a plain sRGB profile, so that conversion is the identity and this port
//!   ignores the `APP2` `ICC_PROFILE` segment, as upstream's own feature list
//!   says it does ("ICC profile (not implemented)").
//! * **Rescaling the gain map.** Upstream lets `ctx.drawImage` scale the gain
//!   map up to the SDR image's resolution with whatever filter the browser
//!   picks. [`resize_bilinear`] is a plain bilinear resample, which is *not*
//!   gated against Chromium. It is dead code for every asset in the tree:
//!   three's UltraHDR files all store the gain map at full resolution, which
//!   [`apply_gain_map`] takes as a straight copy.
//!
//! Recorded in `docs/nodes.md` §19.

use std::path::Path;

use crate::error::Error;
use crate::extras::to_half_float;
use crate::textures::{MinFilter, Texture, TextureFilter, TextureType, Wrapping};

/// `SRGB_TO_LINEAR`, upstream's 1024-entry table over the *0-255-scaled*
/// sRGB axis — so entry `i` is the linear value of the SDR code `i`, and
/// entries past 255 continue the same curve into the boosted range.
///
/// `(1/255) * 0.9478672986 = 0.003717127` is upstream's own folded constant.
fn srgb_to_linear_table() -> &'static [f64; 1024] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[f64; 1024]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0; 1024];
        for (i, entry) in table.iter_mut().enumerate() {
            *entry = (i as f64 * 0.003717127 + 0.0521327014).powf(2.4);
        }
        table
    })
}

/// `_srgbToLinear( value )`.
///
/// The middle branch is the quirk: for `10.31475 <= value < 1024` upstream
/// reads `SRGB_TO_LINEAR[ value | 0 ]`, so the boosted value is **truncated to
/// an integer** before the transfer. Everything above 1024 gets the exact
/// `Math.pow`. Reproduced deliberately — the comment upstream leaves on
/// `maxDisplayBoost` ("1.8 instead of 2 near-perfectly rectifies
/// approximations introduced by precalculated SRGB_TO_LINEAR values") is a
/// correction *for* this truncation, so dropping one without the other would
/// move every texel.
pub fn srgb_to_linear(value: f64) -> f64 {
    // 0.04045 * 255 = 10.31475
    if value < 10.31475 {
        // (1/255) * 0.0773993808
        return value * 0.000303527;
    }

    if value < 1024.0 {
        // `value | 0` is JavaScript's ToInt32: truncation towards zero.
        return srgb_to_linear_table()[value as usize];
    }

    (value * 0.003717127 + 0.0521327014).powf(2.4)
}

/// The gain-map parameters, upstream's `metadata` object.
///
/// The numbers are in the units upstream leaves them in after parsing, which
/// are not all the units the file stores: `offset_sdr` / `offset_hdr` are
/// scaled to the 0-255 SDR axis (the XMP branch divides by `1/64`, the ISO
/// branch multiplies by `255`), while `gain_map_min` / `gain_map_max` and the
/// two capacities stay in log2 stops.
#[derive(Debug, Clone, PartialEq)]
pub struct UltraHdrMetadata {
    /// `hdrgm:Version`, or `"1.0"` for an ISO 21496-1 block, which carries no
    /// version string. `None` means no gain-map metadata was found at all,
    /// which is the one thing upstream validates.
    pub version: Option<String>,
    /// `hdrgm:BaseRenditionIsHDR` / the ISO "backward direction" flag. Parsed
    /// and carried; neither upstream nor this port acts on it.
    pub base_rendition_is_hdr: bool,
    pub gain_map_min: f64,
    pub gain_map_max: f64,
    pub gamma: f64,
    pub offset_sdr: f64,
    pub offset_hdr: f64,
    pub hdr_capacity_min: f64,
    pub hdr_capacity_max: f64,
}

impl Default for UltraHdrMetadata {
    /// Upstream starts every field at `null`; the numeric ones are only ever
    /// read after a parser has filled them, and `null` coerces to `0` in the
    /// arithmetic that would read one early, so zero is the same start.
    fn default() -> Self {
        Self {
            version: None,
            base_rendition_is_hdr: false,
            gain_map_min: 0.0,
            gain_map_max: 0.0,
            gamma: 0.0,
            offset_sdr: 0.0,
            offset_hdr: 0.0,
            hdr_capacity_min: 0.0,
            hdr_capacity_max: 0.0,
        }
    }
}

/// `parse()`'s `texData`, less the fields that are constants (`format:
/// RGBAFormat`).
#[derive(Debug, Clone)]
pub struct UltraHdrTexData {
    pub width: u32,
    pub height: u32,
    /// The gain-map parameters the pixels were reconstructed with. Upstream
    /// keeps this to itself; it is public here because it is the only thing
    /// about the file a unit test can assert without a GPU.
    pub metadata: UltraHdrMetadata,
    pub data: UltraHdrData,
}

/// The reconstructed texels, in whichever of the two types the loader was set
/// to. Both are RGBA, and the alpha channel is a literal 1.
#[derive(Debug, Clone)]
pub enum UltraHdrData {
    /// `HalfFloatType`: a `Uint16Array` of IEEE-754 binary16 bit patterns.
    HalfFloat(Vec<u16>),
    /// `FloatType`: a `Float32Array`.
    Float(Vec<f32>),
}

impl UltraHdrData {
    /// The number of RGBA texels, whichever type this is.
    pub fn len(&self) -> usize {
        match self {
            UltraHdrData::HalfFloat(data) => data.len() / 4,
            UltraHdrData::Float(data) => data.len() / 4,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// `new UltraHDRLoader()`.
#[derive(Debug, Clone)]
pub struct UltraHdrLoader {
    texture_type: TextureType,
}

impl Default for UltraHdrLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl UltraHdrLoader {
    /// `new UltraHDRLoader()` — `type = HalfFloatType`.
    pub fn new() -> Self {
        Self {
            texture_type: TextureType::HalfFloat,
        }
    }

    /// `loader.setDataType( value )`. Only `HalfFloatType` and `FloatType` are
    /// legal; upstream would silently write an unusable buffer, so the mistake
    /// is caught where it is made.
    pub fn set_data_type(&mut self, texture_type: TextureType) -> Result<&mut Self, Error> {
        if !matches!(texture_type, TextureType::HalfFloat | TextureType::Float) {
            return Err(Error::UnsupportedTextureType {
                what: "UltraHDR",
                texture_type,
            });
        }
        self.texture_type = texture_type;
        Ok(self)
    }

    pub fn data_type(&self) -> TextureType {
        self.texture_type
    }

    /// `loader.load( url )` — the file, decoded, as the `DataTexture` the
    /// loader builds around `parse()`'s result.
    ///
    /// Unlike the other data loaders this one does not go through
    /// `DataTextureLoader`: it constructs the `DataTexture` itself, with
    /// `UVMapping`, `ClampToEdgeWrapping` on both axes, `LinearFilter` /
    /// `LinearMipMapLinearFilter`, `LinearSRGBColorSpace` — the port's
    /// [`ColorSpace::NoColorSpace`](crate::textures::ColorSpace::NoColorSpace),
    /// the working space, which carries no transfer function — and then
    /// `generateMipmaps = true` and `flipY = true`.
    ///
    /// `generateMipmaps` is upstream's, and costs a mip chain that nothing in
    /// the PMREM path reads: `_getEquirectMaterial` samples `texture( map,
    /// equirectUV( … ), 0 )` at an explicit level 0. It is kept because
    /// three's dump has the mip passes and a texture with 12 levels.
    pub fn load<P: AsRef<Path>>(&self, url: P) -> Result<Texture, Error> {
        let path = url.as_ref();
        let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        let tex_data = self.parse(path, &bytes)?;

        let texture = match &tex_data.data {
            UltraHdrData::HalfFloat(data) => {
                Texture::data_rgba16float(tex_data.width, tex_data.height, data)
            }
            UltraHdrData::Float(data) => {
                Texture::data_rgba32float(tex_data.width, tex_data.height, data)
            }
        };
        texture.set_wrapping(Wrapping::ClampToEdge, Wrapping::ClampToEdge);
        texture.set_mag_filter(TextureFilter::Linear);
        texture.set_min_filter(MinFilter::LinearMipmapLinear);
        texture.set_generate_mipmaps(true);
        texture.set_flip_y(true);
        Ok(texture)
    }

    /// `UltraHDRLoader.parse( buffer, onLoad )`.
    ///
    /// `path` is only ever used to name the file in an error.
    pub fn parse(&self, path: &Path, buffer: &[u8]) -> Result<UltraHdrTexData, Error> {
        let mut metadata = UltraHdrMetadata::default();
        let mut images: Option<(std::ops::Range<usize>, std::ops::Range<usize>)> = None;

        for section in scan_jpeg_sections(buffer) {
            match section.marker {
                // APP0: the JFIF header, which carries nothing useful.
                0xe0 => {}
                // APP1: XMP metadata.
                0xe1 => {
                    // `textDecoder.decode()` is lossy UTF-8 over the whole
                    // segment, marker and length bytes included.
                    let text = String::from_utf8_lossy(&buffer[section.range.clone()]);
                    parse_xmp_metadata(&text, &mut metadata);
                }
                // APP2: MPF / ICC profile / ISO 21496-1 metadata.
                0xe2 => {
                    let section_bytes = &buffer[section.range.clone()];
                    // `sectionData` is the segment past its two marker bytes.
                    let data = &section_bytes[2.min(section_bytes.len())..];

                    const ISO_NAMESPACE: &[u8] = b"urn:iso:std:iso:ts:21496:-1\0";
                    if data.len() >= ISO_NAMESPACE.len()
                        && &data[..ISO_NAMESPACE.len()] == ISO_NAMESPACE
                    {
                        parse_iso_metadata(&data[ISO_NAMESPACE.len()..], &mut metadata);
                        continue;
                    }

                    // `sectionData.getUint32( 2, false )` — the two length
                    // bytes, then the four-byte tag.
                    if read_u32_be(data, 2) == Some(0x4d50_4600) {
                        images = parse_mpf(data, section.data_offset, buffer.len());
                    }
                }
                _ => {}
            }
        }

        // The whole of upstream's "minimal sufficient validation".
        if metadata.version.is_none() {
            return Err(Error::image(path, "not a valid UltraHDR image"));
        }

        let Some((primary, gain_map)) = images else {
            return Err(Error::image(path, "could not parse the UltraHDR images"));
        };

        let primary = super::texture_loader::decode_jpeg_bytes(path, &buffer[primary])?;
        let gain_map = super::texture_loader::decode_jpeg_bytes(path, &buffer[gain_map])?;

        // `sdrImageAspect !== gainmapImageAspect`.
        if primary.width as f64 / primary.height as f64
            != gain_map.width as f64 / gain_map.height as f64
        {
            return Err(Error::image(
                path,
                "aspect ratio mismatch between the SDR and gain-map images",
            ));
        }

        let data = apply_gain_map(&metadata, &primary, &gain_map, self.texture_type);

        Ok(UltraHdrTexData {
            width: primary.width,
            height: primary.height,
            metadata,
            data,
        })
    }
}

/// One JPEG segment the scanner kept.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Section {
    /// The marker's second byte: `0xd8` for SOI, `0xe0`..`0xe2` for APP0-APP2.
    marker: u8,
    /// The segment's bytes, marker included, as a range into the whole file.
    range: std::ops::Range<usize>,
    /// `sectionOffset` — the offset of the first byte past the marker, which
    /// is what the MPF offsets are relative to.
    data_offset: usize,
}

/// The `while ( offset < bytes.length - 1 )` scanner at the top of `parse()`.
///
/// It walks the *whole* file, not just the first image's header, which is how
/// the gain map's own `APP1` — the one that actually carries the `hdrgm:`
/// attributes — is reached: it sits after the first image's `EOI`.
///
/// Markers with a length field are stepped over by that length; `SOI`, `EOI`
/// and the restart markers are stepped over by two. Inside entropy-coded data
/// a `0xff` is always followed by `0x00` or a restart marker, neither of which
/// matches a length-bearing marker, so the walk stays in step.
fn scan_jpeg_sections(bytes: &[u8]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut offset = 0usize;

    while offset + 1 < bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }

        let marker = bytes[offset + 1];

        // SOI: no length field.
        if marker == 0xd8 {
            sections.push(Section {
                marker,
                range: offset..(offset + 2).min(bytes.len()),
                data_offset: offset + 2,
            });
            offset += 2;
            continue;
        }

        let length = || {
            read_u16_be(bytes, offset + 2)
                .map(|length| (offset + 2 + length as usize).min(bytes.len()))
        };

        // APP0-APP2.
        if marker == 0xe0 || marker == 0xe1 || marker == 0xe2 {
            let Some(end) = length() else { break };
            sections.push(Section {
                marker,
                range: offset..end,
                data_offset: offset + 2,
            });
            offset = end;
            continue;
        }

        // Every other marker that carries a length: skipped, not kept.
        if (0xc0..=0xfe).contains(&marker) && marker != 0xd9 && !(0xd0..=0xd7).contains(&marker) {
            let Some(end) = length() else { break };
            offset = end;
            continue;
        }

        // EOI and the restart markers: no length field.
        offset += 2;
    }

    sections
}

/// The MPF index: 60 bytes of tags and versions, then the two images' sizes
/// and offsets.
///
/// `data` is the segment past its marker, so offset 0 is the length field and
/// offset 2 is the `MPF\0` tag; `section_offset` is the file offset those two
/// bytes sit at, because the MPF offsets are relative to the *first* byte past
/// the marker rather than to the start of the file.
fn parse_mpf(
    data: &[u8],
    section_offset: usize,
    file_len: usize,
) -> Option<(std::ops::Range<usize>, std::ops::Range<usize>)> {
    // 0x49492a00 is little-endian, 0x4d4d002a big-endian.
    let little_endian = read_u32_be(data, 6) == Some(0x4949_2a00);
    let read = |offset: usize| {
        if little_endian {
            read_u32_le(data, offset)
        } else {
            read_u32_be(data, offset)
        }
    };

    const MPF_BYTES_OFFSET: usize = 60;

    // The primary image's size includes the metadata; its offset is always 0.
    let primary_size = read(MPF_BYTES_OFFSET)? as usize;
    let primary_offset = read(MPF_BYTES_OFFSET + 4)? as usize;

    // The gain map's size is absolute from its own offset, and its offset
    // needs six bytes of padding for the `0x00` bytes at the end of the XMP.
    let gain_map_size = read(MPF_BYTES_OFFSET + 16)? as usize;
    let gain_map_offset = read(MPF_BYTES_OFFSET + 20)? as usize + section_offset + 6;

    let primary = primary_offset..primary_offset.checked_add(primary_size)?;
    let gain_map = gain_map_offset..gain_map_offset.checked_add(gain_map_size)?;
    if primary.end > file_len || gain_map.end > file_len {
        // `new Uint8Array( buffer, offset, length )` would throw here.
        return None;
    }
    Some((primary, gain_map))
}

/// `_parseXMPMetadata( xmpDataString, metadata )`.
///
/// Upstream hands the segment to a `DOMParser` and reads attributes off the
/// first `rdf:Description` element — but only when the document is *not* the
/// GContainer descriptor, which it recognises by a `Container:Directory`
/// element. A file has both: the primary image's `APP1` is the container and
/// carries no parameters, and the gain map's own `APP1` is the descriptor.
///
/// The port scans for the same two things rather than building a DOM, because
/// only element names and attribute values are ever read.
fn parse_xmp_metadata(text: &str, metadata: &mut UltraHdrMetadata) {
    // `xmpDataString.substring( indexOf( '<' ), lastIndexOf( '>' ) + 1 )`.
    let (Some(start), Some(end)) = (text.find('<'), text.rfind('>')) else {
        return;
    };
    let xml = &text[start..=end];

    if find_element(xml, "Container:Directory").is_some() {
        // The container descriptor holds nothing but memory validation.
        return;
    }

    let Some(attributes) = find_element(xml, "rdf:Description") else {
        return;
    };

    // `getAttribute` returns null for a missing attribute. Upstream's two
    // shapes for that: `parseFloat( attr || default )`, which takes the
    // default, and `parseFloat( attr / ( 1 / 64 ) )`, where `null / n` is 0.
    let attribute = |name: &str| attribute_value(attributes, name);
    let float_or = |name: &str, default: f64| match attribute(name) {
        // `'' || 0.0` is the default too: an empty string is falsy.
        Some(value) if !value.is_empty() => js_parse_float(&value),
        _ => default,
    };

    metadata.version = attribute("hdrgm:Version");
    metadata.base_rendition_is_hdr =
        attribute("hdrgm:BaseRenditionIsHDR").as_deref() == Some("True");
    metadata.gain_map_min = float_or("hdrgm:GainMapMin", 0.0);
    metadata.gain_map_max = float_or("hdrgm:GainMapMax", 1.0);
    metadata.gamma = float_or("hdrgm:Gamma", 1.0);
    // Stored as a fraction of the SDR range; upstream rescales both offsets to
    // the 0-255 axis the recovery loop works on by dividing by `1 / 64`.
    metadata.offset_sdr =
        attribute("hdrgm:OffsetSDR").map_or(0.0, |value| js_number(&value)) / (1.0 / 64.0);
    metadata.offset_hdr =
        attribute("hdrgm:OffsetHDR").map_or(0.0, |value| js_number(&value)) / (1.0 / 64.0);
    metadata.hdr_capacity_min = float_or("hdrgm:HDRCapacityMin", 0.0);
    metadata.hdr_capacity_max = float_or("hdrgm:HDRCapacityMax", 1.0);
}

/// The attribute list of the first element in `xml` whose tag name is `name`,
/// as the slice between the end of the name and the closing `>`.
fn find_element<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let mut search = 0;
    while let Some(found) = xml[search..].find(name) {
        let open = search + found;
        search = open + name.len();
        // The name must be preceded by `<` and followed by whitespace, `/` or
        // `>`, or `rdf:Description` would also match `rdf:Descriptions`.
        if open == 0 || !xml[..open].ends_with('<') {
            continue;
        }
        let rest = &xml[search..];
        if !rest.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            continue;
        }
        let end = rest.find('>')?;
        return Some(&rest[..end]);
    }
    None
}

/// `element.getAttribute( name )` over an attribute list.
fn attribute_value(attributes: &str, name: &str) -> Option<String> {
    let mut search = 0;
    while let Some(found) = attributes[search..].find(name) {
        let at = search + found;
        search = at + name.len();
        // Preceded by whitespace (or nothing) and followed by `=`, so that
        // `hdrgm:Gamma` does not match a hypothetical `hdrgm:GammaFoo`.
        if at != 0 && !attributes[..at].ends_with(char::is_whitespace) {
            continue;
        }
        let rest = attributes[search..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next()?;
        if quote != '"' && quote != '\'' {
            continue;
        }
        let rest = &rest[quote.len_utf8()..];
        let end = rest.find(quote)?;
        return Some(rest[..end].to_string());
    }
    None
}

/// `parseFloat( string )`: the longest leading prefix that is a number, or NaN.
///
/// Rust's `str::parse` rejects trailing characters where `parseFloat` stops at
/// them, so the prefix is found first.
fn js_parse_float(text: &str) -> f64 {
    let text = text.trim_start();
    let mut end = 0;
    let bytes = text.as_bytes();
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        let mut exponent = end + 1;
        if exponent < bytes.len() && (bytes[exponent] == b'+' || bytes[exponent] == b'-') {
            exponent += 1;
        }
        let digits = exponent;
        while exponent < bytes.len() && bytes[exponent].is_ascii_digit() {
            exponent += 1;
        }
        if exponent > digits {
            end = exponent;
        }
    }
    text[..end].parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number( string )`, which is what a `string / number` coercion uses: the
/// whole string must be a number, and an empty one is zero.
fn js_number(text: &str) -> f64 {
    let text = text.trim();
    if text.is_empty() {
        return 0.0;
    }
    text.parse::<f64>().unwrap_or(f64::NAN)
}

/// `_parseISOMetadata( data, metadata )` — the ISO 21496-1 binary block.
///
/// Every parameter is a rational. The `useCommonDenominator` flag says whether
/// they share one denominator read once up front, or each carries its own; the
/// two branches are otherwise the same eight values in the same order.
fn parse_iso_metadata(data: &[u8], metadata: &mut UltraHdrMetadata) {
    // Minimum version (2 bytes) and writer version (2 bytes) are skipped.
    let mut offset = 4usize;

    let Some(flags) = data.get(offset).copied() else {
        return;
    };
    offset += 1;

    let backward_direction = flags & 0x4 != 0;
    let use_common_denominator = flags & 0x8 != 0;

    let unsigned = |offset: &mut usize| {
        let value = read_u32_be(data, *offset).unwrap_or(0) as f64;
        *offset += 4;
        value
    };

    let (
        gain_map_min,
        gain_map_max,
        gamma,
        offset_sdr,
        offset_hdr,
        hdr_capacity_min,
        hdr_capacity_max,
    );

    if use_common_denominator {
        let denominator = unsigned(&mut offset);
        hdr_capacity_min = (unsigned(&mut offset) / denominator).log2();
        hdr_capacity_max = (unsigned(&mut offset) / denominator).log2();
        gain_map_min = as_i32(unsigned(&mut offset)) / denominator;
        gain_map_max = as_i32(unsigned(&mut offset)) / denominator;
        gamma = unsigned(&mut offset) / denominator;
        offset_sdr = as_i32(unsigned(&mut offset)) / denominator * 255.0;
        offset_hdr = as_i32(unsigned(&mut offset)) / denominator * 255.0;
    } else {
        let rational = |offset: &mut usize, signed: bool| {
            let numerator = read_u32_be(data, *offset).unwrap_or(0) as f64;
            let numerator = if signed { as_i32(numerator) } else { numerator };
            *offset += 4;
            let denominator = read_u32_be(data, *offset).unwrap_or(0) as f64;
            *offset += 4;
            numerator / denominator
        };
        hdr_capacity_min = rational(&mut offset, false).log2();
        hdr_capacity_max = rational(&mut offset, false).log2();
        gain_map_min = rational(&mut offset, true);
        gain_map_max = rational(&mut offset, true);
        gamma = rational(&mut offset, false);
        offset_sdr = rational(&mut offset, true) * 255.0;
        offset_hdr = rational(&mut offset, true) * 255.0;
    }

    // The ISO standard encodes no version string; upstream substitutes "1.0",
    // which is also what makes `parse()`'s one validation pass.
    metadata.version = Some("1.0".to_string());
    metadata.base_rendition_is_hdr = backward_direction;
    metadata.gain_map_min = gain_map_min;
    metadata.gain_map_max = gain_map_max;
    metadata.gamma = gamma;
    metadata.offset_sdr = offset_sdr;
    metadata.offset_hdr = offset_hdr;
    metadata.hdr_capacity_min = hdr_capacity_min;
    metadata.hdr_capacity_max = hdr_capacity_max;
}

/// `DataView.getInt32()` over a value read as unsigned.
fn as_i32(value: f64) -> f64 {
    (value as u32) as i32 as f64
}

fn read_u16_be(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// `_applyGainmapToSDR`'s recovery loop — the HDR rendition from the SDR image
/// and the gain map.
///
/// Three pieces of arithmetic here are upstream's exactly, and each is a
/// deliberate approximation rather than the formula Android documents:
///
/// 1. **`maxDisplayBoost` uses `1.8 ** ( hdrCapacityMax * 0.5 )`**, not `2 **`.
///    Upstream's comment: "1.8 instead of 2 near-perfectly rectifies
///    approximations introduced by precalculated SRGB_TO_LINEAR values." It is
///    a correction for (3), not a display model.
/// 2. **`logBoost * weightFactor == 0` short-circuits to a factor of 1**
///    rather than evaluating `2 ^ 0`, which is the same number; kept so the
///    branch structure matches.
/// 3. **[`srgb_to_linear`] truncates** its argument to an integer over most of
///    its range. See that function.
///
/// The alpha channel is never written: the buffer starts filled with a literal
/// 1 (`15360` is binary16 for 1.0) and only the three colour channels are
/// touched, exactly as upstream's `for ( c = 0; c < 3; c ++ )`.
pub fn apply_gain_map(
    metadata: &UltraHdrMetadata,
    sdr: &crate::textures::Image,
    gain_map: &crate::textures::Image,
    texture_type: TextureType,
) -> UltraHdrData {
    // `ctx.drawImage( gainmapImage, 0, 0, gm.width, gm.height, 0, 0, sdrWidth,
    // sdrHeight )` — a straight copy when the two already agree, which is the
    // case for every UltraHDR asset three ships.
    let scaled;
    let gain_map_data = if gain_map.width == sdr.width && gain_map.height == sdr.height {
        &gain_map.data
    } else {
        scaled = resize_bilinear(gain_map, sdr.width, sdr.height);
        &scaled
    };

    let max_display_boost = 1.8f64.powf(metadata.hdr_capacity_max * 0.5);
    let unclamped_weight_factor = (max_display_boost.log2() - metadata.hdr_capacity_min)
        / (metadata.hdr_capacity_max - metadata.hdr_capacity_min);
    let weight_factor = unclamped_weight_factor.clamp(0.0, 1.0);

    let inv_gamma = 1.0 / metadata.gamma;
    let use_gamma_one = metadata.gamma == 1.0;

    let length = sdr.data.len();
    let mut half = Vec::new();
    let mut float = Vec::new();
    match texture_type {
        TextureType::Float => float = vec![1.0f32; length],
        // 15360 is binary16 1.0.
        _ => half = vec![15360u16; length],
    }

    for i in (0..length).step_by(4) {
        for c in 0..3 {
            let index = i + c;
            let sdr_value = sdr.data[index] as f64;
            // 1 / 255.
            let gain_map_value = gain_map_data[index] as f64 * 0.003_921_568_627_450_98;

            let log_recovery = if use_gamma_one {
                gain_map_value
            } else {
                gain_map_value.powf(inv_gamma)
            };

            let log_boost = metadata.gain_map_min
                + (metadata.gain_map_max - metadata.gain_map_min) * log_recovery;

            let boost = if log_boost * weight_factor == 0.0 {
                1.0
            } else {
                2f64.powf(log_boost * weight_factor)
            };
            let hdr_value = (sdr_value + metadata.offset_sdr) * boost - metadata.offset_hdr;

            let linear = srgb_to_linear(hdr_value).clamp(0.0, 65504.0);

            match texture_type {
                TextureType::Float => float[index] = linear as f32,
                _ => half[index] = to_half_float(linear),
            }
        }
    }

    match texture_type {
        TextureType::Float => UltraHdrData::Float(float),
        _ => UltraHdrData::HalfFloat(half),
    }
}

/// A bilinear resample of an RGBA8 image, standing in for `ctx.drawImage`'s
/// scaling when a file stores the gain map at a lower resolution than the SDR
/// image.
///
/// Not gated against Chromium — see the module docs. Nothing in the tree
/// reaches it.
fn resize_bilinear(image: &crate::textures::Image, width: u32, height: u32) -> Vec<u8> {
    let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
    let scale_x = image.width as f64 / width as f64;
    let scale_y = image.height as f64 / height as f64;

    for y in 0..height {
        // Sample at the centre of the destination texel.
        let source_y = ((y as f64 + 0.5) * scale_y - 0.5).max(0.0);
        let y0 = source_y.floor() as u32;
        let y1 = (y0 + 1).min(image.height - 1);
        let ty = source_y - y0 as f64;

        for x in 0..width {
            let source_x = ((x as f64 + 0.5) * scale_x - 0.5).max(0.0);
            let x0 = source_x.floor() as u32;
            let x1 = (x0 + 1).min(image.width - 1);
            let tx = source_x - x0 as f64;

            for c in 0..4 {
                let at = |x: u32, y: u32| {
                    image.data[((y as usize * image.width as usize) + x as usize) * 4 + c] as f64
                };
                let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
                let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
                let value = top * (1.0 - ty) + bottom * ty;
                out[((y as usize * width as usize) + x as usize) * 4 + c] =
                    value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gain-map descriptor from
    /// `examples/textures/equirectangular/royal_esplanade_2k.hdr.jpg`, which
    /// is the shape every UltraHDR asset three ships uses.
    const GAIN_MAP_XMP: &str = concat!(
        "http://ns.adobe.com/xap/1.0/\0<?xpacket begin=\"\" id=\"W5M0Mp\"?>\n",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\" x:xmptk=\"Adobe XMP Core 5.1.2\">\n",
        "  <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "    <rdf:Description\n",
        "      xmlns:hdrgm=\"http://ns.adobe.com/hdr-gain-map/1.0/\"\n",
        "      hdrgm:Version=\"1.0\"\n",
        "      hdrgm:GainMapMin=\"0\"\n",
        "      hdrgm:GainMapMax=\"15.999075124875333\"\n",
        "      hdrgm:Gamma=\"1\"\n",
        "      hdrgm:OffsetSDR=\"0.015625\"\n",
        "      hdrgm:OffsetHDR=\"0.015625\"\n",
        "      hdrgm:HDRCapacityMin=\"0\"\n",
        "      hdrgm:HDRCapacityMax=\"15.999075124875333\"\n",
        "      hdrgm:BaseRenditionIsHDR=\"False\"\n",
        "      rdf:about=\"\"/>\n",
        "  </rdf:RDF>\n",
        "</x:xmpmeta>\n<?xpacket end=\"w\"?>",
    );

    /// The GContainer descriptor that sits in the *primary* image's APP1 and
    /// must be ignored.
    const CONTAINER_XMP: &str = concat!(
        "http://ns.adobe.com/xap/1.0/\0<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n",
        "  <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n",
        "    <rdf:Description xmlns:Container=\"http://ns.google.com/photos/1.0/container/\"\n",
        "      hdrgm:Version=\"1.0\" rdf:about=\"\">\n",
        "      <Container:Directory>\n",
        "        <rdf:Seq>\n",
        "          <rdf:li rdf:parseType=\"Resource\">\n",
        "            <Container:Item Item:Semantic=\"Primary\" Item:Mime=\"image/jpeg\"/>\n",
        "          </rdf:li>\n",
        "        </rdf:Seq>\n",
        "      </Container:Directory>\n",
        "    </rdf:Description>\n",
        "  </rdf:RDF>\n",
        "</x:xmpmeta>",
    );

    #[test]
    fn xmp_gain_map_descriptor() {
        let mut metadata = UltraHdrMetadata::default();
        parse_xmp_metadata(GAIN_MAP_XMP, &mut metadata);

        assert_eq!(metadata.version.as_deref(), Some("1.0"));
        assert!(!metadata.base_rendition_is_hdr);
        assert_eq!(metadata.gain_map_min, 0.0);
        assert_eq!(metadata.gain_map_max, 15.999075124875333);
        assert_eq!(metadata.gamma, 1.0);
        // 0.015625 / ( 1 / 64 ).
        assert_eq!(metadata.offset_sdr, 1.0);
        assert_eq!(metadata.offset_hdr, 1.0);
        assert_eq!(metadata.hdr_capacity_min, 0.0);
        assert_eq!(metadata.hdr_capacity_max, 15.999075124875333);
    }

    /// The container descriptor carries a `hdrgm:Version` of its own; the
    /// `Container:Directory` test is what keeps it from being read as a
    /// gain-map descriptor.
    #[test]
    fn xmp_container_descriptor_is_ignored() {
        let mut metadata = UltraHdrMetadata::default();
        parse_xmp_metadata(CONTAINER_XMP, &mut metadata);
        assert_eq!(metadata, UltraHdrMetadata::default());
    }

    #[test]
    fn xmp_defaults_for_missing_attributes() {
        let mut metadata = UltraHdrMetadata::default();
        parse_xmp_metadata(
            "<rdf:Description hdrgm:Version=\"1.0\" rdf:about=\"\"/>",
            &mut metadata,
        );
        assert_eq!(metadata.version.as_deref(), Some("1.0"));
        assert_eq!(metadata.gain_map_min, 0.0);
        assert_eq!(metadata.gain_map_max, 1.0);
        assert_eq!(metadata.gamma, 1.0);
        assert_eq!(metadata.offset_sdr, 0.0);
        assert_eq!(metadata.offset_hdr, 0.0);
        assert_eq!(metadata.hdr_capacity_min, 0.0);
        assert_eq!(metadata.hdr_capacity_max, 1.0);
    }

    /// `BaseRenditionIsHDR` is a string compare against `'True'`, so nothing
    /// else — not `true`, not `1` — sets it.
    #[test]
    fn xmp_base_rendition_is_hdr_is_a_string_compare() {
        for (value, expected) in [("True", true), ("true", false), ("1", false)] {
            let mut metadata = UltraHdrMetadata::default();
            parse_xmp_metadata(
                &format!(
                    "<rdf:Description hdrgm:Version=\"1.0\" hdrgm:BaseRenditionIsHDR=\"{value}\"/>"
                ),
                &mut metadata,
            );
            assert_eq!(metadata.base_rendition_is_hdr, expected, "{value}");
        }
    }

    /// An ISO 21496-1 block with one denominator shared by every rational,
    /// built to decode to the same parameters as the XMP above.
    #[test]
    fn iso_common_denominator() {
        let denominator: u32 = 1_000_000;
        let mut data = Vec::new();
        data.extend_from_slice(&[0, 0, 0, 0]); // minimum + writer version
        data.push(0x8 | 0x4); // useCommonDenominator, backwardDirection
        data.extend_from_slice(&denominator.to_be_bytes());
        // baseHdrHeadroom = 1 → log2( 1 ) = 0; alternate = 4 → 2.
        data.extend_from_slice(&denominator.to_be_bytes());
        data.extend_from_slice(&(4 * denominator).to_be_bytes());
        data.extend_from_slice(&(-(denominator as i32)).to_be_bytes()); // gainMapMin = -1
        data.extend_from_slice(&(3 * denominator).to_be_bytes()); // gainMapMax = 3
        data.extend_from_slice(&(2 * denominator).to_be_bytes()); // gamma = 2
        data.extend_from_slice(&((denominator / 64) as i32).to_be_bytes()); // offsetSDR
        data.extend_from_slice(&((denominator / 64) as i32).to_be_bytes()); // offsetHDR

        let mut metadata = UltraHdrMetadata::default();
        parse_iso_metadata(&data, &mut metadata);

        assert_eq!(metadata.version.as_deref(), Some("1.0"));
        assert!(metadata.base_rendition_is_hdr);
        assert_eq!(metadata.hdr_capacity_min, 0.0);
        assert_eq!(metadata.hdr_capacity_max, 2.0);
        assert_eq!(metadata.gain_map_min, -1.0);
        assert_eq!(metadata.gain_map_max, 3.0);
        assert_eq!(metadata.gamma, 2.0);
        // 0.015625 * 255, the ISO branch's scaling.
        assert!((metadata.offset_sdr - 0.015625 * 255.0).abs() < 1e-9);
        assert!((metadata.offset_hdr - 0.015625 * 255.0).abs() < 1e-9);
    }

    /// The same parameters with per-value denominators, which is the other
    /// branch of `_parseISOMetadata`.
    #[test]
    fn iso_per_value_denominators() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.push(0); // neither flag
        let mut rational = |numerator: i32, denominator: u32| {
            data.extend_from_slice(&numerator.to_be_bytes());
            data.extend_from_slice(&denominator.to_be_bytes());
        };
        rational(1, 1); // baseHdrHeadroom = 1 → 0 stops
        rational(4, 1); // alternateHdrHeadroom = 4 → 2 stops
        rational(-1, 1); // gainMapMin
        rational(3, 1); // gainMapMax
        rational(2, 1); // gamma
        rational(1, 64); // offsetSDR
        rational(1, 64); // offsetHDR

        let mut metadata = UltraHdrMetadata::default();
        parse_iso_metadata(&data, &mut metadata);

        assert!(!metadata.base_rendition_is_hdr);
        assert_eq!(metadata.hdr_capacity_min, 0.0);
        assert_eq!(metadata.hdr_capacity_max, 2.0);
        assert_eq!(metadata.gain_map_min, -1.0);
        assert_eq!(metadata.gain_map_max, 3.0);
        assert_eq!(metadata.gamma, 2.0);
        assert_eq!(metadata.offset_sdr, 255.0 / 64.0);
        assert_eq!(metadata.offset_hdr, 255.0 / 64.0);
    }

    /// The scanner keeps SOI and APP0-APP2 and steps over everything else by
    /// its length field, including a start-of-scan followed by entropy-coded
    /// bytes that contain `0xff 0x00` stuffing.
    #[test]
    fn jpeg_section_scan() {
        let mut bytes = vec![0xff, 0xd8]; // SOI
        bytes.extend_from_slice(&[0xff, 0xe1, 0x00, 0x06, b'x', b'm', b'p', b'!']);
        bytes.extend_from_slice(&[0xff, 0xdb, 0x00, 0x04, 0x00, 0x00]); // DQT, skipped
        bytes.extend_from_slice(&[0xff, 0xda, 0x00, 0x04, 0x00, 0x00]); // SOS
        bytes.extend_from_slice(&[0x12, 0xff, 0x00, 0x34]); // entropy data
        bytes.extend_from_slice(&[0xff, 0xd9]); // EOI

        let sections = scan_jpeg_sections(&bytes);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].marker, 0xd8);
        assert_eq!(sections[1].marker, 0xe1);
        assert_eq!(sections[1].range, 2..10);
        assert_eq!(sections[1].data_offset, 4);
    }

    /// The truncation in the middle of the sRGB transfer, at the two ends of
    /// the branch that has it.
    #[test]
    fn srgb_to_linear_truncates_in_the_table_range() {
        // Below 0.04045 * 255, the linear segment.
        assert_eq!(srgb_to_linear(10.0), 10.0 * 0.000303527);
        // Inside the table: 20.9 and 20.0 land on the same entry.
        assert_eq!(srgb_to_linear(20.9), srgb_to_linear(20.0));
        assert_ne!(srgb_to_linear(21.0), srgb_to_linear(20.0));
        // Past the table, the exact curve, which no longer truncates.
        assert_ne!(srgb_to_linear(1024.5), srgb_to_linear(1024.0));
    }

    /// The whole recovery loop over a two-texel image, against the formula
    /// written out by hand.
    #[test]
    fn gain_map_recovery() {
        let metadata = UltraHdrMetadata {
            version: Some("1.0".to_string()),
            base_rendition_is_hdr: false,
            gain_map_min: 0.0,
            gain_map_max: 4.0,
            gamma: 1.0,
            offset_sdr: 1.0,
            offset_hdr: 1.0,
            hdr_capacity_min: 0.0,
            hdr_capacity_max: 4.0,
        };
        let sdr = crate::textures::Image {
            width: 2,
            height: 1,
            data: vec![10, 128, 255, 7, 0, 64, 200, 7],
        };
        let gain_map = crate::textures::Image {
            width: 2,
            height: 1,
            data: vec![0, 128, 255, 7, 255, 0, 128, 7],
        };

        let data = apply_gain_map(&metadata, &sdr, &gain_map, TextureType::Float);
        let UltraHdrData::Float(values) = data else {
            panic!("asked for FloatType");
        };

        let weight = (1.8f64.powf(2.0).log2() / 4.0).clamp(0.0, 1.0);
        for i in [0usize, 4] {
            for c in 0..3 {
                let gain = gain_map.data[i + c] as f64 / 255.0;
                let log_boost = 4.0 * gain;
                let boost = if log_boost * weight == 0.0 {
                    1.0
                } else {
                    2f64.powf(log_boost * weight)
                };
                let hdr = (sdr.data[i + c] as f64 + 1.0) * boost - 1.0;
                let expected = srgb_to_linear(hdr).clamp(0.0, 65504.0) as f32;
                assert_eq!(values[i + c], expected, "texel {i} channel {c}");
            }
            // The alpha channel is never touched.
            assert_eq!(values[i + 3], 1.0);
        }
    }
}
