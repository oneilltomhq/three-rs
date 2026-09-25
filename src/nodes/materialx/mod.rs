//! Port of `three.js/src/nodes/materialx/` — the MaterialX node library three
//! ships auto-converted from MaterialX's own GLSL.
//!
//! [`mx_nodes`] is the public surface (`MaterialXNodes.js`, which is what TSL
//! re-exports); the other modules hold the converted functions behind it.

/// A converted `Fn().setLayout()`: its `fn` is built once per thread and
/// emitted once per shader.
macro_rules! mx_fn {
    ($rust:ident, $wgsl:literal, $params:expr, $ret:expr, $body:expr) => {
        fn $rust() -> Rc<FnDef> {
            thread_local! { static CELL: Lazy<Rc<FnDef>> = Lazy::new(); }
            CELL.with(|c| c.get(|| shader_fn(Some($wgsl), $params, $ret, $body)))
        }
    };
}

/// A layout-less `Fn()`, built once per thread and inlined at every call.
macro_rules! mx_inline {
    ($rust:ident, $params:expr, $ret:expr, $body:expr) => {
        fn $rust() -> Rc<FnDef> {
            thread_local! { static CELL: Lazy<Rc<FnDef>> = const { Lazy::new() }; }
            CELL.with(|c| c.get(|| inline_fn($params, $ret, $body)))
        }
    };
}

mod mx_color;
mod mx_core;
pub mod mx_nodes;
mod mx_noise;

pub use mx_nodes::*;
