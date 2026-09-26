//! Port of `three.js/examples/jsm/loaders/KTX2Loader.js` — KTX 2.0 textures,
//! Basis Universal (ETC1S, UASTC, UASTC HDR 4x4) transcoded to whatever the
//! device can sample, everything else handed through as stored.
//!
//! Three does this in three pieces, and so does the port:
//!
//! * the container is read by `ktx-parse` upstream and by the [`ktx2`] crate
//!   here;
//! * Basis Universal payloads go through Binomial's C++ transcoder, compiled
//!   to wasm (`examples/jsm/libs/basis/basis_transcoder.wasm`) and run in a
//!   Web Worker upstream, and through [`basisu`], a pure-Rust port of the same
//!   transcoder, here — synchronously, because the port's loaders are (see
//!   `crate::io`);
//! * Zstandard-supercompressed levels of non-Basis files are inflated by
//!   `zstddec` upstream and by [`ruzstd`] here.
//!
//! Everything around those — which transcode target a device gets
//! (`getTranscoderFormat`), which texture class and filters come out
//! (`_createTextureFrom`, `createRawTexture`), the colour space
//! (`parseColorSpace`) — is ported line for line. `tests/ktx2_loader.rs` runs
//! three's own `KTX2Loader` under node (`tools/ktx2_reference.mjs`) over every
//! `.ktx2` in the three.js checkout, for four device profiles, and asserts this
//! loader produces the same class, format, type, colour space, filters and —
//! byte for byte — the same mip levels.
//!
//! # `basisu` against three's wasm
//!
//! `basisu` tracks Basis Universal v2.1; three vendors an older build. They
//! agree bit for bit on every target the loader asks for but one: ETC1S to
//! BC7, where v2 added a cross-block chroma-filtering pass that three's build
//! does not run. The port turns it off (`NO_ETC1S_CHROMA_FILTERING`), which is
//! what makes that path exact too; see [`transcode_flags`].
//!
//! # What the result is
//!
//! [`Ktx2Loader::parse`] returns a [`Ktx2Texture`]: the class three would
//! construct plus every field three's texture carries, in three's own terms
//! ([`EngineFormat`], [`EngineType`], [`Ktx2ColorSpace`]). That is what the
//! oracle test compares. [`Ktx2Texture::into_texture`] turns it into the
//! port's [`Texture`], resolving `( format, type, colorSpace )` to one
//! `wgpu::TextureFormat` the way `WebGPUTextureUtils.getFormat()` does.
//! `CompressedCubeTexture` and `Data3DTexture` results load (and are checked
//! by the oracle) but have no renderer path yet, so `into_texture` refuses
//! them rather than hand back something that samples wrong.

use std::path::Path;

use basisu::{DecodeFlags, SourceFormat, TargetFormat, Transcoder};
use ktx2::{ColorPrimaries, Format, SupercompressionScheme, TransferFunction};

use crate::error::Error;
use crate::textures::{ColorSpace, MinFilter, Mipmap, Texture, TextureFilter};

/// `KTX2Loader.workerConfig` — which compressed families the device can
/// sample, as `detectSupport()` found them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ktx2Support {
    pub astc: bool,
    pub etc1: bool,
    pub etc2: bool,
    pub dxt: bool,
    pub bptc: bool,
    pub pvrtc: bool,
}

impl Ktx2Support {
    /// `detectSupport( renderer )` for a `WebGPURenderer`: each flag is
    /// `renderer.hasFeature( 'texture-compression-…' )`.
    ///
    /// WebGPU names three of the families three asks about —
    /// `texture-compression-astc`, `-etc2` and `-bc` — and those map onto
    /// wgpu's features one to one. `-etc1`, `-s3tc` and `-pvrtc` are not
    /// WebGPU feature names at all, so `hasFeature` is false for them in every
    /// browser: DXT is reached through `bptc` (which ranks above it for every
    /// Basis format anyway), and ETC1 and PVRTC never are. `astcHDRSupported`
    /// is hardcoded false upstream (gpuweb/gpuweb#3856), so UASTC HDR always
    /// transcodes.
    pub fn from_features(features: wgpu::Features) -> Self {
        Self {
            astc: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC),
            etc1: false,
            etc2: features.contains(wgpu::Features::TEXTURE_COMPRESSION_ETC2),
            dxt: false,
            bptc: features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
            pvrtc: false,
        }
    }
}

/// `KTX2Loader`.
#[derive(Clone, Debug, Default)]
pub struct Ktx2Loader {
    config: Ktx2Support,
}

/// Which texture class three's loader constructs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ktx2Class {
    CompressedTexture,
    CompressedArrayTexture,
    CompressedCubeTexture,
    DataTexture,
    Data3DTexture,
}

impl Ktx2Class {
    /// `texture.constructor.name`.
    pub fn name(self) -> &'static str {
        match self {
            Ktx2Class::CompressedTexture => "CompressedTexture",
            Ktx2Class::CompressedArrayTexture => "CompressedArrayTexture",
            Ktx2Class::CompressedCubeTexture => "CompressedCubeTexture",
            Ktx2Class::DataTexture => "DataTexture",
            Ktx2Class::Data3DTexture => "Data3DTexture",
        }
    }
}

/// `texture.format` — the three.js constants `KTX2Loader` can produce. The
/// discriminant *is* the constant (`constants.js`), so `format as u32` is
/// what three's texture holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineFormat {
    RGB = 1022,
    RGBA = 1023,
    Red = 1028,
    RG = 1030,
    RgbS3tcDxt1 = 33776,
    RgbaS3tcDxt1 = 33777,
    RgbaS3tcDxt5 = 33779,
    RgbPvrtc4bppV1 = 35840,
    RgbaPvrtc4bppV1 = 35842,
    RgbaPvrtc2bppV1 = 35843,
    RgbEtc1 = 36196,
    RedRgtc1 = 36283,
    SignedRedRgtc1 = 36284,
    RedGreenRgtc2 = 36285,
    SignedRedGreenRgtc2 = 36286,
    RgbaBptc = 36492,
    RgbBptcUnsigned = 36495,
    R11Eac = 37488,
    SignedR11Eac = 37489,
    Rg11Eac = 37490,
    SignedRg11Eac = 37491,
    RgbEtc2 = 37492,
    RgbaEtc2Eac = 37496,
    RgbaAstc4x4 = 37808,
    RgbaAstc6x6 = 37812,
}

/// `texture.type` — likewise the `constants.js` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum EngineType {
    UnsignedByte = 1009,
    UnsignedShort = 1012,
    Float = 1015,
    HalfFloat = 1016,
    UnsignedInt101111 = 35899,
    UnsignedInt5999 = 35902,
}

/// `parseColorSpace( container )`'s answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ktx2ColorSpace {
    /// `NoColorSpace` — `''`.
    NoColorSpace,
    /// `SRGBColorSpace` — `'srgb'`.
    SRGB,
    /// `LinearSRGBColorSpace` — `'srgb-linear'`.
    LinearSRGB,
    /// `DisplayP3ColorSpace` — `'display-p3'`.
    DisplayP3,
    /// `LinearDisplayP3ColorSpace` — `'display-p3-linear'`.
    LinearDisplayP3,
}

impl Ktx2ColorSpace {
    /// The string three's colour-space constants are.
    pub fn name(self) -> &'static str {
        match self {
            Ktx2ColorSpace::NoColorSpace => "",
            Ktx2ColorSpace::SRGB => "srgb",
            Ktx2ColorSpace::LinearSRGB => "srgb-linear",
            Ktx2ColorSpace::DisplayP3 => "display-p3",
            Ktx2ColorSpace::LinearDisplayP3 => "display-p3-linear",
        }
    }

    /// `ColorManagement.getTransfer( colorSpace ) === SRGBTransfer` — whether
    /// `getFormat()` picks the `-srgb` GPU format.
    pub fn is_srgb_transfer(self) -> bool {
        matches!(self, Ktx2ColorSpace::SRGB | Ktx2ColorSpace::DisplayP3)
    }
}

/// What `KTX2Loader.parse()` hands to `onLoad`: the texture three builds, as
/// data.
#[derive(Clone, Debug)]
pub struct Ktx2Texture {
    pub class: Ktx2Class,
    pub format: EngineFormat,
    pub texture_type: EngineType,
    pub color_space: Ktx2ColorSpace,
    pub premultiply_alpha: bool,
    pub min_filter: MinFilter,
    pub mag_filter: TextureFilter,
    pub generate_mipmaps: bool,
    /// `texture.normalized` — true only for `R16G16B16A16_UNORM`.
    pub normalized: bool,
    /// `image.width` / `image.height` (face 0's, for a cube).
    pub width: u32,
    pub height: u32,
    /// `image.depth`: the layer count of a `CompressedArrayTexture`, the
    /// depth of a `Data3DTexture`, 0 otherwise.
    pub depth: u32,
    /// `texture.mipmaps`, or for a `CompressedCubeTexture` each of the six
    /// faces' `mipmaps` — always one entry otherwise.
    pub faces: Vec<Vec<Mipmap>>,
}

impl Ktx2Loader {
    /// `new KTX2Loader()` before `detectSupport()`: no compressed family at
    /// all, so every Basis texture transcodes to its uncompressed fallback
    /// (`RGBA32`, or `RGBA_HALF` for HDR), which any device samples.
    pub fn new() -> Self {
        Self::default()
    }

    /// `loader.detectSupport( renderer )`.
    pub fn detect_support(mut self, renderer: &crate::renderer::Renderer) -> Self {
        self.config = Ktx2Support::from_features(renderer.features());
        self
    }

    /// `detectSupport()` against a feature set rather than a renderer — for
    /// a host that knows its device, and for the tests, which describe one.
    pub fn with_support(mut self, support: Ktx2Support) -> Self {
        self.config = support;
        self
    }

    pub fn support(&self) -> Ktx2Support {
        self.config
    }

    /// `loader.load( url, onLoad )`, synchronously.
    pub fn load(&self, url: impl AsRef<Path>) -> Result<Ktx2Texture, Error> {
        let bytes = crate::io::read(url.as_ref())?;
        self.parse(&bytes)
    }

    /// `loader.parse( buffer, onLoad )` — `_createTexture( buffer )`.
    pub fn parse(&self, buffer: &[u8]) -> Result<Ktx2Texture, Error> {
        let container = ktx2::Reader::new(buffer)
            .map_err(|error| ktx2_error(format!("Invalid or unsupported .ktx2 file: {error}")))?;
        let header = container.header();

        // Basis UASTC HDR is a subset of ASTC, which can be transcoded
        // efficiently to BC6H. To detect whether a KTX2 file uses Basis UASTC
        // HDR, or default ASTC, inspect the DFD color model.
        let is_basis_hdr = header.format == Some(Format::ASTC_4x4_SFLOAT_BLOCK)
            && container.color_model().map(|model| model.value()) == Some(0xA7);

        // `astcHDRSupported` is always false on WebGPU (see `Ktx2Support`).
        let needs_transcoder = header.format.is_none() || is_basis_hdr;

        if !needs_transcoder {
            return create_raw_texture(&container);
        }

        let transcoded = self.transcode(buffer)?;
        Ok(create_texture_from(transcoded, &container))
    }

    /// `KTX2Loader.BasisWorker`'s `transcode( buffer )`.
    fn transcode(&self, buffer: &[u8]) -> Result<Transcoded, Error> {
        let file = Transcoder::new(buffer)
            .map_err(|_| ktx2_error("Invalid or unsupported .ktx2 file".to_string()))?;

        let basis_format = match file.source_format() {
            SourceFormat::UastcLdr => BasisFormat::Uastc,
            SourceFormat::Etc1s => BasisFormat::Etc1s,
            SourceFormat::UastcHdr4x4 => BasisFormat::UastcHdr,
            // three's transcoder predates XUASTC and the 6x6 HDR codecs:
            // `ktx2File.isUASTC() / isETC1S() / isHDR()` all answer no.
            _ => return Err(ktx2_error("Unknown Basis encoding".to_string())),
        };

        let (width, height) = file.base_dimensions();
        let layer_count = file.layer_count().max(1);
        let level_count = file.level_count();
        let face_count = file.face_count().max(1);
        let has_alpha = file.has_alpha();

        let choice = transcoder_format(&self.config, basis_format, width, height, has_alpha)?;

        if width == 0 || height == 0 || level_count == 0 {
            return Err(ktx2_error("Invalid texture".to_string()));
        }

        let flags = transcode_flags(basis_format, choice.transcoder_format);

        let mut faces = Vec::with_capacity(face_count as usize);
        for face in 0..face_count {
            let mut mipmaps = Vec::with_capacity(level_count as usize);
            for mip in 0..level_count {
                let info = file
                    .image_level_info(mip)
                    .map_err(|error| ktx2_error(format!(".transcodeImage failed: {error:?}")))?;

                let (mip_width, mip_height) = if level_count > 1 {
                    // `levelInfo.origWidth` / `origHeight`.
                    ((width >> mip).max(1), (height >> mip).max(1))
                } else {
                    // `levelInfo.width` / `height`: the size rounded up to
                    // whole 4x4 blocks, which is what lets a single-level
                    // texture with non-multiple-of-four dimensions upload
                    // (mrdoob/three.js#25908).
                    let (block_width, block_height) = file.source_format().block_dims();
                    (
                        info.num_blocks_x * block_width,
                        info.num_blocks_y * block_height,
                    )
                };

                // `concat( layerMips )`.
                let mut data = Vec::new();
                for layer in 0..layer_count {
                    let image = file
                        .transcode_image(mip, layer, face, choice.transcoder_format, flags)
                        .map_err(|error| {
                            ktx2_error(format!(".transcodeImage failed: {error:?}"))
                        })?;
                    data.extend_from_slice(&image);
                }

                mipmaps.push(Mipmap {
                    data,
                    width: mip_width,
                    height: mip_height,
                });
            }
            faces.push(mipmaps);
        }

        Ok(Transcoded {
            faces,
            width,
            height,
            format: choice.engine_format,
            texture_type: choice.engine_type,
        })
    }
}

/// `KTX2Loader.BasisFormat`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BasisFormat {
    Etc1s,
    Uastc,
    UastcHdr,
}

/// The worker's reply: `{ faces, width, height, format, type }`.
struct Transcoded {
    faces: Vec<Vec<Mipmap>>,
    width: u32,
    height: u32,
    format: EngineFormat,
    texture_type: EngineType,
}

struct Choice {
    transcoder_format: TargetFormat,
    engine_format: EngineFormat,
    engine_type: EngineType,
}

/// One row of the worker's `FORMAT_OPTIONS`.
struct FormatOption {
    /// `opt.if` — `None` for the uncompressed fallbacks.
    supported: Option<fn(&Ktx2Support) -> bool>,
    basis_format: &'static [BasisFormat],
    /// `[ opaque, alpha ]`, or just one when the target has no alpha variant.
    transcoder_format: &'static [TargetFormat],
    engine_format: &'static [EngineFormat],
    engine_type: EngineType,
    priority_etc1s: u32,
    priority_uastc: u32,
    priority_hdr: u32,
    needs_power_of_two: bool,
}

/// `Infinity` in the priority columns: never chosen for that format.
const NEVER: u32 = u32::MAX;

/// `FORMAT_OPTIONS`, in the worker's order. The ETC1S and UASTC rankings are
/// "high quality > low quality > uncompressed"; UASTC HDR has BC6H and the
/// half-float fallback.
const FORMAT_OPTIONS: &[FormatOption] = &[
    FormatOption {
        supported: Some(|c| c.astc),
        basis_format: &[BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Astc4x4Rgba, TargetFormat::Astc4x4Rgba],
        engine_format: &[EngineFormat::RgbaAstc4x4, EngineFormat::RgbaAstc4x4],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: NEVER,
        priority_uastc: 1,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: Some(|c| c.bptc),
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        // `TranscoderFormat.BC7_M5` is 7 upstream, which the current
        // transcoder no longer has: its `transcoder_texture_format` 6 is the
        // one BC7 target, and three's wasm treats 7 as 6 (the output of the
        // two is identical there).
        transcoder_format: &[TargetFormat::Bc7Rgba, TargetFormat::Bc7Rgba],
        engine_format: &[EngineFormat::RgbaBptc, EngineFormat::RgbaBptc],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 3,
        priority_uastc: 2,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: Some(|c| c.dxt),
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Bc1Rgb, TargetFormat::Bc3Rgba],
        engine_format: &[EngineFormat::RgbaS3tcDxt1, EngineFormat::RgbaS3tcDxt5],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 4,
        priority_uastc: 5,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: Some(|c| c.etc2),
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Etc1Rgb, TargetFormat::Etc2Rgba],
        engine_format: &[EngineFormat::RgbEtc2, EngineFormat::RgbaEtc2Eac],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 1,
        priority_uastc: 3,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: Some(|c| c.etc1),
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Etc1Rgb],
        engine_format: &[EngineFormat::RgbEtc1],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 2,
        priority_uastc: 4,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: Some(|c| c.pvrtc),
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Pvrtc1_4Rgb, TargetFormat::Pvrtc1_4Rgba],
        engine_format: &[EngineFormat::RgbPvrtc4bppV1, EngineFormat::RgbaPvrtc4bppV1],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 5,
        priority_uastc: 6,
        priority_hdr: NEVER,
        needs_power_of_two: true,
    },
    FormatOption {
        supported: Some(|c| c.bptc),
        basis_format: &[BasisFormat::UastcHdr],
        transcoder_format: &[TargetFormat::Bc6h],
        engine_format: &[EngineFormat::RgbBptcUnsigned],
        engine_type: EngineType::HalfFloat,
        priority_etc1s: NEVER,
        priority_uastc: NEVER,
        priority_hdr: 1,
        needs_power_of_two: false,
    },
    // Uncompressed fallbacks.
    FormatOption {
        supported: None,
        basis_format: &[BasisFormat::Etc1s, BasisFormat::Uastc],
        transcoder_format: &[TargetFormat::Rgba32, TargetFormat::Rgba32],
        engine_format: &[EngineFormat::RGBA, EngineFormat::RGBA],
        engine_type: EngineType::UnsignedByte,
        priority_etc1s: 100,
        priority_uastc: 100,
        priority_hdr: NEVER,
        needs_power_of_two: false,
    },
    FormatOption {
        supported: None,
        basis_format: &[BasisFormat::UastcHdr],
        transcoder_format: &[TargetFormat::RgbaHalf],
        engine_format: &[EngineFormat::RGBA],
        engine_type: EngineType::HalfFloat,
        priority_etc1s: NEVER,
        priority_uastc: NEVER,
        priority_hdr: 100,
        needs_power_of_two: false,
    },
];

/// `getTranscoderFormat( basisFormat, width, height, hasAlpha )`: the first
/// option, by the format's own priority, that the device supports.
fn transcoder_format(
    config: &Ktx2Support,
    basis_format: BasisFormat,
    width: u32,
    height: u32,
    has_alpha: bool,
) -> Result<Choice, Error> {
    let priority = |option: &FormatOption| match basis_format {
        BasisFormat::Etc1s => option.priority_etc1s,
        BasisFormat::Uastc => option.priority_uastc,
        BasisFormat::UastcHdr => option.priority_hdr,
    };

    // `OPTIONS[ basisFormat ]`: filtered to the format, stably sorted by its
    // priority, as `Array.prototype.sort` is.
    let mut options: Vec<&FormatOption> = FORMAT_OPTIONS
        .iter()
        .filter(|option| option.basis_format.contains(&basis_format))
        .collect();
    options.sort_by_key(|option| priority(option));

    for option in options {
        if let Some(supported) = option.supported {
            if !supported(config) {
                continue;
            }
        }
        if has_alpha && option.transcoder_format.len() < 2 {
            continue;
        }
        if option.needs_power_of_two && !(is_power_of_two(width) && is_power_of_two(height)) {
            continue;
        }

        let index = usize::from(has_alpha);
        return Ok(Choice {
            transcoder_format: option.transcoder_format[index],
            engine_format: option.engine_format[index],
            engine_type: option.engine_type,
        });
    }

    Err(ktx2_error(
        "Failed to identify transcoding target.".to_string(),
    ))
}

fn is_power_of_two(value: u32) -> bool {
    if value <= 2 {
        return true;
    }
    value & (value - 1) == 0
}

/// The worker calls `transcodeImage( dst, mip, layer, face, format, 0, -1, -1 )`
/// — decode flags 0 — on a Basis Universal 1.x build.
///
/// **Divergence, closed**: Basis Universal v2 (which `basisu` tracks) added a
/// cross-block chroma-filtering pass to the ETC1S → BC7 transcode and turns it
/// on by default. With it on, 13 of the 100 BC7 blocks of `2d_etc1s.ktx2`'s
/// top level differ from three's — and are further from the ETC1S → RGBA32
/// decode than three's are. `NO_ETC1S_CHROMA_FILTERING` is v2's own switch
/// back to the 1.x behaviour, and with it the output matches three's byte for
/// byte. No other target is affected by the flag (`docs/nodes.md` divergence
/// list, KTX2).
fn transcode_flags(basis_format: BasisFormat, target: TargetFormat) -> DecodeFlags {
    if basis_format == BasisFormat::Etc1s && target == TargetFormat::Bc7Rgba {
        DecodeFlags::NO_ETC1S_CHROMA_FILTERING
    } else {
        DecodeFlags::NONE
    }
}

/// `_createTextureFrom( transcodeResult, container )`.
fn create_texture_from(transcoded: Transcoded, container: &ktx2::Reader<&[u8]>) -> Ktx2Texture {
    let header = container.header();
    let single_level = transcoded.faces[0].len() == 1;

    let (class, depth) = if header.face_count == 6 {
        (Ktx2Class::CompressedCubeTexture, 0)
    } else if header.layer_count > 1 {
        (Ktx2Class::CompressedArrayTexture, header.layer_count)
    } else {
        (Ktx2Class::CompressedTexture, 0)
    };

    Ktx2Texture {
        class,
        format: transcoded.format,
        texture_type: transcoded.texture_type,
        color_space: parse_color_space(container),
        premultiply_alpha: container.is_alpha_premultiplied().unwrap_or(false),
        min_filter: if single_level {
            MinFilter::Linear
        } else {
            MinFilter::LinearMipmapLinear
        },
        mag_filter: TextureFilter::Linear,
        generate_mipmaps: false,
        normalized: false,
        width: transcoded.width,
        height: transcoded.height,
        depth,
        faces: transcoded.faces,
    }
}

/// `FORMAT_MAP[ vkFormat ]` and `TYPE_MAP[ vkFormat ]` together.
fn raw_format(format: Format) -> Option<(EngineFormat, EngineType)> {
    use EngineFormat as F;
    use EngineType as T;
    Some(match format {
        Format::R32G32B32A32_SFLOAT => (F::RGBA, T::Float),
        Format::R32G32_SFLOAT => (F::RG, T::Float),
        Format::R32_SFLOAT => (F::Red, T::Float),

        Format::R16G16B16A16_SFLOAT => (F::RGBA, T::HalfFloat),
        Format::R16G16_SFLOAT => (F::RG, T::HalfFloat),
        Format::R16_SFLOAT => (F::Red, T::HalfFloat),

        Format::R16G16B16A16_UNORM => (F::RGBA, T::UnsignedShort),

        Format::R8G8B8A8_SRGB | Format::R8G8B8A8_UNORM => (F::RGBA, T::UnsignedByte),
        Format::R8G8_SRGB | Format::R8G8_UNORM => (F::RG, T::UnsignedByte),
        Format::R8_SRGB | Format::R8_UNORM => (F::Red, T::UnsignedByte),

        Format::E5B9G9R9_UFLOAT_PACK32 => (F::RGB, T::UnsignedInt5999),
        Format::B10G11R11_UFLOAT_PACK32 => (F::RGB, T::UnsignedInt101111),

        Format::ETC2_R8G8B8A8_SRGB_BLOCK => (F::RgbaEtc2Eac, T::UnsignedByte),
        Format::ETC2_R8G8B8_SRGB_BLOCK => (F::RgbEtc2, T::UnsignedByte),
        Format::EAC_R11_UNORM_BLOCK => (F::R11Eac, T::UnsignedByte),
        Format::EAC_R11_SNORM_BLOCK => (F::SignedR11Eac, T::UnsignedByte),
        Format::EAC_R11G11_UNORM_BLOCK => (F::Rg11Eac, T::UnsignedByte),
        Format::EAC_R11G11_SNORM_BLOCK => (F::SignedRg11Eac, T::UnsignedByte),

        Format::ASTC_4x4_SFLOAT_BLOCK => (F::RgbaAstc4x4, T::HalfFloat),
        Format::ASTC_4x4_SRGB_BLOCK | Format::ASTC_4x4_UNORM_BLOCK => {
            (F::RgbaAstc4x4, T::UnsignedByte)
        }
        Format::ASTC_6x6_SFLOAT_BLOCK => (F::RgbaAstc6x6, T::HalfFloat),
        Format::ASTC_6x6_SRGB_BLOCK | Format::ASTC_6x6_UNORM_BLOCK => {
            (F::RgbaAstc6x6, T::UnsignedByte)
        }

        Format::BC1_RGBA_SRGB_BLOCK | Format::BC1_RGBA_UNORM_BLOCK => {
            (F::RgbaS3tcDxt1, T::UnsignedByte)
        }
        Format::BC1_RGB_SRGB_BLOCK | Format::BC1_RGB_UNORM_BLOCK => {
            (F::RgbS3tcDxt1, T::UnsignedByte)
        }

        Format::BC3_SRGB_BLOCK | Format::BC3_UNORM_BLOCK => (F::RgbaS3tcDxt5, T::UnsignedByte),

        Format::BC4_SNORM_BLOCK => (F::SignedRedRgtc1, T::UnsignedByte),
        Format::BC4_UNORM_BLOCK => (F::RedRgtc1, T::UnsignedByte),

        Format::BC5_SNORM_BLOCK => (F::SignedRedGreenRgtc2, T::UnsignedByte),
        Format::BC5_UNORM_BLOCK => (F::RedGreenRgtc2, T::UnsignedByte),

        Format::BC7_SRGB_BLOCK | Format::BC7_UNORM_BLOCK => (F::RgbaBptc, T::UnsignedByte),

        Format::PVRTC1_4BPP_SRGB_BLOCK | Format::PVRTC1_4BPP_UNORM_BLOCK => {
            (F::RgbaPvrtc4bppV1, T::UnsignedByte)
        }
        Format::PVRTC1_2BPP_SRGB_BLOCK | Format::PVRTC1_2BPP_UNORM_BLOCK => {
            (F::RgbaPvrtc2bppV1, T::UnsignedByte)
        }

        _ => return None,
    })
}

/// `createRawTexture( container )` — a non-Basis file: its levels, inflated
/// if they are Zstandard-supercompressed, as the texture's mipmaps.
fn create_raw_texture(container: &ktx2::Reader<&[u8]>) -> Result<Ktx2Texture, Error> {
    let header = container.header();
    let vk_format = header.format.expect("a raw texture has a vkFormat");

    let (format, texture_type) = raw_format(vk_format)
        .ok_or_else(|| ktx2_error(format!("Unsupported vkFormat: {}", vk_format.value())))?;

    let mut mipmaps = Vec::new();
    for (level_index, level) in container.levels().enumerate() {
        let level_width = (header.pixel_width >> level_index).max(1);
        let level_height = (header.pixel_height >> level_index).max(1);

        let data = match header.supercompression_scheme {
            None => level.data.to_vec(),
            Some(SupercompressionScheme::Zstandard) => {
                zstd_decode(level.data, level.uncompressed_byte_length as usize)?
            }
            Some(_) => {
                return Err(ktx2_error(
                    "Unsupported supercompressionScheme.".to_string(),
                ))
            }
        };

        // The typed-array views three wraps the bytes in (`Float32Array`,
        // `Uint16Array`, `Uint32Array`) are views over these same bytes.
        mipmaps.push(Mipmap {
            data,
            width: level_width,
            height: level_height,
        });
    }

    // levelCount = 0 implies runtime-generated mipmaps.
    let use_mipmaps = header.level_count == 0 || mipmaps.len() > 1;

    let uncompressed = matches!(
        format,
        EngineFormat::RGBA | EngineFormat::RGB | EngineFormat::RG | EngineFormat::Red
    );

    let color_space = parse_color_space(container);

    if uncompressed {
        let (class, depth) = if header.pixel_depth == 0 {
            (Ktx2Class::DataTexture, 0)
        } else {
            (Ktx2Class::Data3DTexture, header.pixel_depth)
        };
        return Ok(Ktx2Texture {
            class,
            format,
            texture_type,
            color_space,
            premultiply_alpha: false,
            min_filter: if use_mipmaps {
                MinFilter::NearestMipmapNearest
            } else {
                MinFilter::Nearest
            },
            mag_filter: TextureFilter::Nearest,
            generate_mipmaps: header.level_count == 0,
            normalized: vk_format == Format::R16G16B16A16_UNORM,
            width: header.pixel_width,
            height: header.pixel_height,
            depth,
            faces: vec![mipmaps],
        });
    }

    if header.pixel_depth > 0 {
        return Err(ktx2_error("Unsupported pixelDepth.".to_string()));
    }

    let (class, faces) = if header.face_count == 6 {
        // Each level holds the six faces back to back; `subarray` splits it.
        let faces = (0..6)
            .map(|face| {
                mipmaps
                    .iter()
                    .map(|mipmap| {
                        let face_length = mipmap.data.len() / 6;
                        Mipmap {
                            data: mipmap.data[face * face_length..(face + 1) * face_length]
                                .to_vec(),
                            width: mipmap.width,
                            height: mipmap.height,
                        }
                    })
                    .collect()
            })
            .collect();
        (Ktx2Class::CompressedCubeTexture, faces)
    } else {
        (Ktx2Class::CompressedTexture, vec![mipmaps])
    };

    Ok(Ktx2Texture {
        class,
        format,
        texture_type,
        color_space,
        premultiply_alpha: false,
        min_filter: if use_mipmaps {
            MinFilter::LinearMipmapLinear
        } else {
            MinFilter::Linear
        },
        mag_filter: TextureFilter::Linear,
        // `CompressedTexture`'s constructor default, untouched by the loader.
        generate_mipmaps: false,
        normalized: false,
        width: header.pixel_width,
        height: header.pixel_height,
        depth: 0,
        faces,
    })
}

/// `zstd.decode( levelData, uncompressedByteLength )`.
fn zstd_decode(compressed: &[u8], uncompressed_length: usize) -> Result<Vec<u8>, Error> {
    use ruzstd::io::Read;

    let mut decoder = ruzstd::StreamingDecoder::new(compressed)
        .map_err(|error| ktx2_error(format!("zstd: {error}")))?;
    let mut out = vec![0u8; uncompressed_length];
    decoder
        .read_exact(&mut out)
        .map_err(|error| ktx2_error(format!("zstd: {error:?}")))?;
    Ok(out)
}

/// `parseColorSpace( container )`, from the first DFD block.
fn parse_color_space(container: &ktx2::Reader<&[u8]>) -> Ktx2ColorSpace {
    let srgb = container.transfer_function() == Some(TransferFunction::SRGB);
    match container.color_primaries() {
        Some(ColorPrimaries::BT709) => {
            if srgb {
                Ktx2ColorSpace::SRGB
            } else {
                Ktx2ColorSpace::LinearSRGB
            }
        }
        Some(ColorPrimaries::DISPLAYP3) => {
            if srgb {
                Ktx2ColorSpace::DisplayP3
            } else {
                Ktx2ColorSpace::LinearDisplayP3
            }
        }
        // `KHR_DF_PRIMARIES_UNSPECIFIED`, and — after a `console.warn` —
        // everything else.
        _ => Ktx2ColorSpace::NoColorSpace,
    }
}

fn ktx2_error(reason: String) -> Error {
    Error::Ktx2 { reason }
}

impl Ktx2Texture {
    /// `WebGPUTextureUtils.getFormat( texture )`: the GPU format three's
    /// backend creates this texture with.
    ///
    /// `RGBA16Unorm` is what three picks when the device has
    /// `texture-formats-tier1`, and `RGBA16Uint` (which a float sampler
    /// cannot read) when it does not; the port always asks for the former,
    /// and wgpu gates it behind `TEXTURE_FORMAT_16BIT_NORM`.
    ///
    /// ASTC `SFLOAT` files (not Basis UASTC HDR, which transcodes) come out of
    /// `FORMAT_MAP` as `RGBA_ASTC_4x4_Format` + `HalfFloatType`, and three
    /// then creates an LDR `astc-4x4-unorm` texture for them — WebGPU has no
    /// HDR ASTC at all. The port does the same rather than invent an HDR
    /// path; no file in the three.js checkout is one.
    pub fn gpu_format(&self) -> Result<wgpu::TextureFormat, Error> {
        use wgpu::{AstcBlock, AstcChannel, TextureFormat as G};
        let srgb = self.color_space.is_srgb_transfer();
        let pick = |linear: G, srgb_format: G| if srgb { srgb_format } else { linear };
        let astc = |block| {
            if srgb {
                G::Astc {
                    block,
                    channel: AstcChannel::UnormSrgb,
                }
            } else {
                G::Astc {
                    block,
                    channel: AstcChannel::Unorm,
                }
            }
        };

        use EngineFormat as F;
        use EngineType as T;
        Ok(match (self.format, self.texture_type) {
            (F::RGBA, T::UnsignedByte) => pick(G::Rgba8Unorm, G::Rgba8UnormSrgb),
            (F::RGBA, T::HalfFloat) => G::Rgba16Float,
            (F::RGBA, T::Float) => G::Rgba32Float,
            (F::RGBA, T::UnsignedShort) => G::Rgba16Unorm,
            (F::RG, T::UnsignedByte) => G::Rg8Unorm,
            (F::RG, T::HalfFloat) => G::Rg16Float,
            (F::RG, T::Float) => G::Rg32Float,
            (F::Red, T::UnsignedByte) => G::R8Unorm,
            (F::Red, T::HalfFloat) => G::R16Float,
            (F::Red, T::Float) => G::R32Float,
            (F::RGB, T::UnsignedInt5999) => G::Rgb9e5Ufloat,
            (F::RGB, T::UnsignedInt101111) => G::Rg11b10Ufloat,

            (F::RgbS3tcDxt1 | F::RgbaS3tcDxt1, _) => pick(G::Bc1RgbaUnorm, G::Bc1RgbaUnormSrgb),
            (F::RgbaS3tcDxt5, _) => pick(G::Bc3RgbaUnorm, G::Bc3RgbaUnormSrgb),
            (F::RedRgtc1, _) => G::Bc4RUnorm,
            (F::SignedRedRgtc1, _) => G::Bc4RSnorm,
            (F::RedGreenRgtc2, _) => G::Bc5RgUnorm,
            (F::SignedRedGreenRgtc2, _) => G::Bc5RgSnorm,
            (F::RgbaBptc, _) => pick(G::Bc7RgbaUnorm, G::Bc7RgbaUnormSrgb),
            (F::RgbBptcUnsigned, _) => G::Bc6hRgbUfloat,

            (F::RgbEtc1 | F::RgbEtc2, _) => pick(G::Etc2Rgb8Unorm, G::Etc2Rgb8UnormSrgb),
            (F::RgbaEtc2Eac, _) => pick(G::Etc2Rgba8Unorm, G::Etc2Rgba8UnormSrgb),
            (F::R11Eac, _) => G::EacR11Unorm,
            (F::SignedR11Eac, _) => G::EacR11Snorm,
            (F::Rg11Eac, _) => G::EacRg11Unorm,
            (F::SignedRg11Eac, _) => G::EacRg11Snorm,

            (F::RgbaAstc4x4, _) => astc(AstcBlock::B4x4),
            (F::RgbaAstc6x6, _) => astc(AstcBlock::B6x6),

            (format, texture_type) => {
                return Err(Error::UnsupportedFormat {
                    what: "KTX2 texture format",
                    value: format!("{format:?} / {texture_type:?} has no WebGPU format"),
                })
            }
        })
    }

    /// The port's [`Texture`] for this result, carrying the same mipmaps,
    /// filters and colour space three's texture does.
    ///
    /// `LinearSRGBColorSpace` becomes [`ColorSpace::NoColorSpace`]: the port's
    /// colour spaces are about the transfer function only (the working space
    /// is linear sRGB), and a linear texture is sampled with no transfer.
    /// Display P3 is sampled with the sRGB transfer, as `getFormat()` does,
    /// but no gamut conversion follows — the port has no P3 working space.
    pub fn into_texture(self) -> Result<Texture, Error> {
        let format = self.gpu_format()?;
        let color_space = if self.color_space.is_srgb_transfer() {
            ColorSpace::SRGB
        } else {
            ColorSpace::NoColorSpace
        };

        let mut faces = self.faces;
        let mipmaps = faces.swap_remove(0);

        let texture = match self.class {
            Ktx2Class::CompressedTexture | Ktx2Class::DataTexture => {
                Texture::compressed(mipmaps, self.width, self.height, format)
            }
            Ktx2Class::CompressedArrayTexture => {
                Texture::compressed_array(mipmaps, self.width, self.height, self.depth, format)
            }
            Ktx2Class::CompressedCubeTexture | Ktx2Class::Data3DTexture => {
                return Err(Error::UnsupportedFormat {
                    what: "KTX2 texture class",
                    value: format!(
                        "{} (the loader reads it; the renderer has no upload path for it yet)",
                        self.class.name()
                    ),
                })
            }
        };

        {
            let mut inner = texture.borrow_mut_inner();
            inner.color_space = color_space;
            inner.min_filter = self.min_filter;
            inner.mag_filter = self.mag_filter;
            inner.premultiply_alpha = self.premultiply_alpha;
            // `DataTexture` with `levelCount = 0` asks for a generated chain:
            // mip 0 becomes the image and the renderer builds the rest.
            if self.generate_mipmaps {
                let base = inner.mipmaps.remove(0);
                inner.data = Some(base.data);
                inner.mipmaps.clear();
                inner.generate_mipmaps = true;
            }
        }

        Ok(texture)
    }
}
