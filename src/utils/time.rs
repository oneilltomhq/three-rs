//! The one clock everything in this crate reads.
//!
//! three.js has two of them, and which one a page uses is visible in its
//! source: `performance.now()` — milliseconds since the document's time
//! origin, monotonic — and `Date.now()` — milliseconds since the Unix epoch.
//! [`now_ms`] and [`date_now_ms`] are those two, and nothing else in the crate
//! or in the ported examples reads a clock directly.
//!
//! Having exactly one seam is what lets the graded frame stay fixed while the
//! examples animate. three.js' own e2e harness
//! (`test/e2e/deterministic-injection.js`) replaces both browser clocks with
//! constants so that a screenshot is reproducible:
//!
//! ```js
//! Date.now = () => 0;
//! performance.now = () => 0;
//! ```
//!
//! [`crate::testing::pin_time`] is that injection. With it set the ported
//! examples compute exactly the frame the reference screenshot holds, however
//! faithfully their `animate()` reads the clock; without it the same
//! `animate()` runs on the wall clock in the viewer and in a browser.
//!
//! The override is a thread-local, because the e2e harness runs its rungs on
//! Rust's test threads and each must pin its own.

use std::cell::Cell;

thread_local! {
    /// The pinned value, in milliseconds, or `None` for the real clock.
    static PINNED: Cell<Option<f64>> = const { Cell::new(None) };
}

/// Pins both clocks to `ms`, or restores the real ones with `None`.
///
/// The public door is [`crate::testing::pin_time`]; this is where the state
/// lives, next to the readers.
pub(crate) fn pin(ms: Option<f64>) {
    PINNED.with(|pinned| pinned.set(ms));
}

/// What is pinned on this thread, if anything.
pub(crate) fn pinned() -> Option<f64> {
    PINNED.with(|pinned| pinned.get())
}

/// `performance.now()`: milliseconds since an origin fixed when the process
/// (or the document) started, monotonic and never adjusted.
///
/// Natively the origin is the first call, which is as close to "process start"
/// as a program can get without an OS-specific query; in a browser it is
/// `performance.timeOrigin`, the page's own.
pub fn now_ms() -> f64 {
    if let Some(ms) = pinned() {
        return ms;
    }
    real_now_ms()
}

/// `Date.now()`: milliseconds since the Unix epoch.
///
/// Wall-clock, so it can step backwards; the pages that read it
/// (`webgpu_materials`, `webgpu_instance_mesh`, …) only ever feed it to a sine.
pub fn date_now_ms() -> f64 {
    if let Some(ms) = pinned() {
        return ms;
    }
    real_date_now_ms()
}

#[cfg(not(target_arch = "wasm32"))]
fn real_now_ms() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    /// The process' time origin: the first read of the monotonic clock.
    static ORIGIN: OnceLock<Instant> = OnceLock::new();

    ORIGIN.get_or_init(Instant::now).elapsed().as_secs_f64() * 1e3
}

/// The unpinned wall clock, for the few readers that want entropy rather than
/// a page's time (`generate_uuid()`'s seed). `std::time::SystemTime::now()`
/// panics on wasm32-unknown-unknown, so they come here instead (issue #128).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn real_date_now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64() * 1e3)
        // A clock set before 1970 is not a reason to panic in a render loop.
        .unwrap_or(0.0)
}

/// `window.performance.now()`, through web-sys.
///
/// A worker has no `window`, and this crate's browser host (the shell in
/// `web/`) runs on the main thread, so a missing `window` falls back to 0
/// rather than panicking inside a frame.
#[cfg(target_arch = "wasm32")]
fn real_now_ms() -> f64 {
    web_sys::window()
        .and_then(|window| window.performance())
        .map(|performance| performance.now())
        .unwrap_or(0.0)
}

/// `Date.now()`, through js-sys.
#[cfg(target_arch = "wasm32")]
pub(crate) fn real_date_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinning_freezes_both_clocks() {
        pin(Some(0.0));
        assert_eq!(now_ms(), 0.0);
        assert_eq!(date_now_ms(), 0.0);

        pin(Some(1234.5));
        assert_eq!(now_ms(), 1234.5);
        assert_eq!(date_now_ms(), 1234.5);

        pin(None);
        // `Date.now()` is past the epoch and `performance.now()` is not
        // negative; that is all that can be asserted about a real clock.
        assert!(date_now_ms() > 1_600_000_000_000.0);
        assert!(now_ms() >= 0.0);
    }

    #[test]
    fn the_monotonic_clock_does_not_run_backwards() {
        pin(None);
        let first = now_ms();
        let second = now_ms();
        assert!(second >= first);
    }
}
