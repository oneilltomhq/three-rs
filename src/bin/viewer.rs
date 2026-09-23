//! The viewer binary's target gate. Its body is `viewer_app.rs` next door; see
//! that file's header for what the viewer is and how to run it.
//!
//! The split exists so that the workspace builds for `wasm32-unknown-unknown`
//! (issue #128). The viewer is a winit window on a Vulkan surface: winit is a
//! native-only dependency now, and neither a window nor a Vulkan surface means
//! anything in a browser. A cargo `[[bin]]` cannot be target-gated in the
//! manifest, so the gate is here, on the contents, and wasm32 gets a `main`
//! that does nothing rather than a compile error. The browser's driver of the
//! same example modules is the shell in `web/`.

#[cfg(not(target_arch = "wasm32"))]
#[path = "viewer_app.rs"]
mod app;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    app::main()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
