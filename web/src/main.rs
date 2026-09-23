//! The browser shell's target gate. Its body is `shell.rs` beside it.
//!
//! Same split as `src/bin/viewer.rs`, and for the mirror-image reason: the
//! viewer is a window and a Vulkan surface, which have no meaning in a page;
//! this is a canvas, `navigator.gpu` and `fetch`, which have none on a desktop
//! (`wgpu::SurfaceTarget::Canvas` does not exist off wasm32). The crate is a
//! workspace member so that `cargo fmt`, `cargo clippy` and `cargo build
//! --workspace` cover it, and this gate is what lets those run on a desktop.

#[cfg(target_arch = "wasm32")]
mod shell;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!(
        "three-rs-web is the browser shell; it runs on wasm32 only.\n\
         Build it with web/build.sh and serve web/dist (see web/README.md)."
    );
}

// On wasm32 the entry point is `shell::start`, which wasm-bindgen calls from
// the generated JS glue; a `main` would run before the page has a canvas.
#[cfg(target_arch = "wasm32")]
fn main() {}
