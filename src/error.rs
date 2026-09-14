//! The crate's error type.
//!
//! Nothing in three.js corresponds to this file: JavaScript throws, and the
//! port's public entry points return `Result` instead (issue #9, and decision 4
//! in `docs/api.md`). One enum covers the whole crate, so a caller writes one
//! `match` — or none, since [`Error`] implements [`std::error::Error`].
//!
//! Panics are kept for invariants the caller cannot reach: a light list that
//! only holds lights, a texture the renderer uploaded a moment earlier. Every
//! such message starts `three-rs:` and names the invariant.

use std::fmt;
use std::path::PathBuf;

use crate::animation::property_binding::ParseTrackNameError;

/// Everything the public API of `three-rs` can fail with.
///
/// Non-exhaustive: new variants are added as more of the port's surface stops
/// panicking, and matching code should carry a `_` arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A file a loader was pointed at could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A JSON document could not be parsed. `path` is `None` when the caller
    /// handed the JSON over directly, as `BufferGeometryLoader::parse` does.
    Json {
        path: Option<PathBuf>,
        source: serde_json::Error,
    },
    /// An image file the decoder rejected.
    Image { path: PathBuf, reason: String },
    /// A file names a type, format or encoding this port does not implement.
    /// `what` says which field it was read from.
    UnsupportedFormat { what: &'static str, value: String },
    /// A texture type that is not a legal depth format (`gpu_format`) or not a
    /// legal colour format (`color_gpu_format`).
    UnsupportedTextureType {
        what: &'static str,
        texture_type: crate::textures::TextureType,
    },
    /// A glTF asset the loader could not make sense of.
    Gltf(GltfError),
    /// `new KeyframeTrack()` with an empty `times` array.
    NoKeyframes { track: String },
    /// A track type name that no `KeyframeTrack` subclass answers to.
    UnsupportedTrackType { name: String },
    /// An interpolation mode this track's value type does not support, where
    /// the value type's own default does not support it either.
    UnsupportedInterpolation {
        value_type: &'static str,
        track: String,
    },
    /// A track name `PropertyBinding` cannot parse.
    TrackName(ParseTrackNameError),
    /// No Vulkan adapter to render on. `wanted` is the `THREE_RS_ADAPTER_NAME`
    /// filter, when one was set and matched nothing.
    NoAdapter { wanted: Option<String> },
    /// `adapter.request_device()` failed.
    Device(wgpu::RequestDeviceError),
    /// Reading pixels back off the GPU failed; `reason` is what wgpu said.
    Readback { reason: String },
}

/// What went wrong inside a glTF asset, as far as the loader reads it.
#[derive(Debug)]
#[non_exhaustive]
pub enum GltfError {
    /// A JSON chunk, or a `.gltf` file, that is not UTF-8.
    NotUtf8,
    /// `asset.version` is not 2.x.
    UnsupportedVersion(String),
    /// The 12-byte GLB header has the wrong magic.
    BadHeader,
    /// A GLB written against the draft container format.
    LegacyBinary,
    /// A GLB chunk that runs past the end of the file.
    TruncatedChunk,
    /// A GLB with no JSON chunk.
    NoJsonChunk,
    /// A buffer with no `uri` in a GLB that has no BIN chunk.
    NoBinChunk,
    /// A field a definition cannot be read without (`accessor.type`,
    /// `sampler.input`, …). `what` names it.
    MissingField { what: &'static str },
    /// An index into one of the asset's arrays that is not there. `kind` is the
    /// array (`accessor`, `bufferView`, `buffer`, `mesh`).
    MissingIndex { kind: &'static str, index: usize },
    /// An accessor `componentType` the port does not read.
    UnsupportedComponentType(i64),
    /// An accessor `type` the port does not read.
    UnsupportedAccessorType(String),
    /// A `data:` URI that is not `data:<mime>;base64,<payload>`.
    BadDataUri(String),
    /// A character that is not in the base64 alphabet.
    BadBase64(char),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Json {
                path: Some(path),
                source,
            } => write!(f, "cannot parse {}: {source}", path.display()),
            Self::Json { path: None, source } => write!(f, "cannot parse JSON: {source}"),
            Self::Image { path, reason } => {
                write!(f, "cannot decode {}: {reason}", path.display())
            }
            Self::UnsupportedFormat { what, value } => {
                write!(f, "unsupported {what}: {value}")
            }
            Self::UnsupportedTextureType { what, texture_type } => {
                write!(f, "{texture_type:?} is not a {what} texture type")
            }
            Self::Gltf(error) => write!(f, "THREE.GLTFLoader: {error}"),
            Self::NoKeyframes { track } => {
                write!(f, "no keyframes in track named {track}")
            }
            Self::UnsupportedTrackType { name } => write!(f, "unsupported trackType: {name}"),
            Self::UnsupportedInterpolation { value_type, track } => write!(
                f,
                "unsupported interpolation for {value_type} keyframe track named {track}"
            ),
            Self::TrackName(error) => write!(f, "{error}"),
            Self::NoAdapter { wanted: Some(name) } => {
                write!(f, "no Vulkan adapter matching THREE_RS_ADAPTER_NAME={name}")
            }
            Self::NoAdapter { wanted: None } => write!(f, "no Vulkan adapter found"),
            Self::Device(source) => write!(f, "cannot create the device: {source}"),
            Self::Readback { reason } => write!(f, "cannot read the canvas back: {reason}"),
        }
    }
}

impl fmt::Display for GltfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUtf8 => write!(f, "not UTF-8 JSON"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported asset version {version}")
            }
            Self::BadHeader => write!(f, "unsupported glTF-Binary header"),
            Self::LegacyBinary => write!(f, "legacy binary file detected"),
            Self::TruncatedChunk => write!(f, "truncated glTF-Binary chunk"),
            Self::NoJsonChunk => write!(f, "glTF-Binary without JSON content"),
            Self::NoBinChunk => write!(f, "glTF-Binary without BIN chunk"),
            Self::MissingField { what } => write!(f, "no {what}"),
            Self::MissingIndex { kind, index } => write!(f, "no {kind} {index}"),
            Self::UnsupportedComponentType(value) => {
                write!(f, "unsupported componentType {value}")
            }
            Self::UnsupportedAccessorType(name) => write!(f, "unsupported accessor type {name}"),
            Self::BadDataUri(uri) => write!(f, "bad data URI {uri}"),
            Self::BadBase64(c) => write!(f, "bad base64 character {c:?}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Gltf(source) => Some(source),
            Self::TrackName(source) => Some(source),
            Self::Device(source) => Some(source),
            _ => None,
        }
    }
}

impl std::error::Error for GltfError {}

impl From<GltfError> for Error {
    fn from(error: GltfError) -> Self {
        Self::Gltf(error)
    }
}

impl From<ParseTrackNameError> for Error {
    fn from(error: ParseTrackNameError) -> Self {
        Self::TrackName(error)
    }
}

impl From<wgpu::RequestDeviceError> for Error {
    fn from(error: wgpu::RequestDeviceError) -> Self {
        Self::Device(error)
    }
}

impl Error {
    /// `std::fs::read`'s error, with the path that produced it.
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// A decode failure, with the file that produced it.
    pub(crate) fn image(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::Image {
            path: path.into(),
            reason: reason.into(),
        }
    }
}
