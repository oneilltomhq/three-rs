//! The one seam through which the loaders read bytes.
//!
//! Nothing in three.js corresponds to this file. A browser has no filesystem,
//! so `TextureLoader` and friends take a URL and go through `fetch()`, which is
//! asynchronous and returns a placeholder texture that fills in later. This
//! port's loaders are synchronous and take a `&Path`, because the graded
//! examples' `init()` is synchronous and the harness grades the *first* frame:
//! a texture that arrives a frame late is a wrong picture, not a slower one.
//!
//! Keeping that shape on wasm32 means the bytes have to be in memory before
//! `init()` runs. So this module is the one place the loaders touch the outside
//! world, and it has two implementations:
//!
//! * natively, `read` and `read_to_string` are `std::fs`;
//! * on wasm32, they look the path up in a map the host filled with
//!   `preload` beforehand (wasm32 only, so not a link from these docs), and a
//!   miss is an `Error::Io` carrying
//!   `ErrorKind::NotFound` — the same error a missing file gives natively, so a
//!   caller's error handling does not fork per target.
//!
//! What the host has to preload is not guessed: `start_recording` makes a
//! native run report every path it read, and the browser shell's per-example
//! asset manifests are derived from exactly that (`examples/web_manifests.rs`,
//! issue #128).

use std::path::{Path, PathBuf};

use crate::error::Error;

thread_local! {
    /// The paths read since `start_recording`, or `None` when no one is
    /// recording — which is every ordinary run, so the cost of the seam on the
    /// grading path is one thread-local read of an `Option` that is `None`.
    static RECORDING: std::cell::RefCell<Option<Vec<PathBuf>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// The bytes `preload` was given, keyed by the very path the loaders will
    /// ask for. There is no normalisation: the shell builds its keys as
    /// `three_js_dir().join(relative)`, which is how every example builds the
    /// paths it hands a loader.
    // Not a `const` initialiser: `HashMap::new()` is not a const fn, because
    // `RandomState` is not. One lazy initialisation per page is nothing.
    static PRELOADED: std::cell::RefCell<std::collections::HashMap<PathBuf, Vec<u8>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Hands the loaders the bytes of one file, under the path they will ask for.
///
/// This is how assets reach a browser build: the shell fetches each file named
/// by the example's manifest and preloads it under
/// `three_rs::testing::three_js_dir().join(relative_path)` before calling
/// `init()`. wasm32 only — natively the loaders read the disk and there is
/// nothing to preload.
///
/// Preloading the same path twice replaces the bytes; the map is never cleared,
/// because a page loads one example and then ends.
#[cfg(target_arch = "wasm32")]
pub fn preload(path: impl Into<PathBuf>, bytes: Vec<u8>) {
    PRELOADED.with(|map| map.borrow_mut().insert(path.into(), bytes));
}

/// Starts recording the paths `read` and `read_to_string` are called with,
/// discarding anything an earlier recording had collected.
///
/// The asset manifests the browser shell ships are derived from a native run of
/// each example's `init()` between this and [`stop_recording`], so that they
/// describe what the code actually reads rather than what someone believed it
/// read. Never call it from library code.
#[doc(hidden)]
pub fn start_recording() {
    RECORDING.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
}

/// Stops recording and returns the paths read since [`start_recording`], in the
/// order they were read and with repeats kept — deduplication and sorting are
/// the manifest generator's business, not this seam's. Returns an empty vector
/// when nothing was recording.
#[doc(hidden)]
pub fn stop_recording() -> Vec<PathBuf> {
    RECORDING.with(|slot| slot.borrow_mut().take().unwrap_or_default())
}

fn record(path: &Path) {
    RECORDING.with(|slot| {
        if let Some(paths) = slot.borrow_mut().as_mut() {
            paths.push(path.to_path_buf());
        }
    });
}

/// `std::fs::read`, or the preloaded map on wasm32.
pub(crate) fn read(path: &Path) -> Result<Vec<u8>, Error> {
    record(path);
    read_inner(path)
}

/// `std::fs::read_to_string`, or the preloaded map on wasm32.
///
/// The wasm32 side decodes the preloaded bytes as UTF-8 and reports a failure
/// the way `std::fs::read_to_string` does, as an `InvalidData` io error, so
/// that a caller sees one error shape per target.
pub(crate) fn read_to_string(path: &Path) -> Result<String, Error> {
    let bytes = read(path)?;
    String::from_utf8(bytes).map_err(|error| {
        Error::io(
            path,
            std::io::Error::new(std::io::ErrorKind::InvalidData, error.utf8_error()),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn read_inner(path: &Path) -> Result<Vec<u8>, Error> {
    std::fs::read(path).map_err(|error| Error::io(path, error))
}

#[cfg(target_arch = "wasm32")]
fn read_inner(path: &Path) -> Result<Vec<u8>, Error> {
    PRELOADED
        .with(|map| map.borrow().get(path).cloned())
        .ok_or_else(|| {
            Error::io(
                path,
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not preloaded: no three_rs::io::preload() for this path",
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recording is off by default, so an ordinary run allocates nothing.
    #[test]
    fn not_recording_by_default() {
        assert!(stop_recording().is_empty());
    }

    #[test]
    fn records_every_read_in_order() {
        start_recording();
        // The paths need not exist: the seam records the request, and the
        // manifest generator only ever looks at paths a read succeeded on
        // because a failing loader stops the example.
        let _ = read(Path::new("/nowhere/a.png"));
        let _ = read(Path::new("/nowhere/b.png"));
        let _ = read(Path::new("/nowhere/a.png"));
        assert_eq!(
            stop_recording(),
            vec![
                PathBuf::from("/nowhere/a.png"),
                PathBuf::from("/nowhere/b.png"),
                PathBuf::from("/nowhere/a.png"),
            ]
        );
        // And the recording is over.
        let _ = read(Path::new("/nowhere/c.png"));
        assert!(stop_recording().is_empty());
    }

    /// A missing file is the same `Error::Io` / `NotFound` on either target.
    #[test]
    fn a_miss_is_not_found() {
        let error = read(Path::new("/nowhere/at/all.png")).unwrap_err();
        match error {
            Error::Io { source, .. } => {
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound)
            }
            other => panic!("expected Error::Io, got {other:?}"),
        }
    }
}
