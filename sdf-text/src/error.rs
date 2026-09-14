//! The crate's error type.
//!
//! One enum for `sdf-text`, as issue #9 asks for. The JS this ports throws from
//! `opentype.parse`; here that is the [`Error::Parse`] variant, and the rest of
//! the crate's surface is infallible — layout, rasterisation and the atlas take
//! a font that has already parsed.

use std::fmt;

/// Everything the public API of `sdf-text` can fail with.
///
/// Non-exhaustive: more of the crate's surface may become fallible as the port
/// grows, and matching code should carry a `_` arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Font bytes `ttf-parser` could not read a face out of. `src` is the
    /// label the caller passed to [`crate::VectorFont::parse`].
    Parse {
        src: String,
        source: owned_ttf_parser::FaceParsingError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { src, source } => {
                write!(f, "VectorFont: cannot parse the font {src}: {source}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse { source, .. } => Some(source),
        }
    }
}
