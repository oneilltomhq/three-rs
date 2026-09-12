//! Port of `three.js/src/core/Layers.js`.
//!
//! A membership bit mask over 32 layers. `Object3D.layers` and `Camera.layers`
//! both default to layer 0, and the renderer's `_projectObject` only pushes an
//! object whose `layers.test( camera.layers )` passes.

/// `class Layers`.
///
/// three.js' `mask` is a JS number used as an `int32`: `1 << layer | 0` and the
/// `>>> 0` in `set()`. A `u32` is the same 32 bits with no sign games.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layers {
    /// `Layers.mask`.
    pub mask: u32,
}

impl Default for Layers {
    /// `new Layers()` — membership of layer 0 only.
    fn default() -> Self {
        Self { mask: 1 }
    }
}

impl Layers {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Layers.set( layer )`.
    pub fn set(&mut self, layer: u32) {
        self.mask = 1u32 << layer;
    }

    /// `Layers.enable( layer )`.
    pub fn enable(&mut self, layer: u32) {
        self.mask |= 1u32 << layer;
    }

    /// `Layers.enableAll()`.
    pub fn enable_all(&mut self) {
        self.mask = 0xffffffff;
    }

    /// `Layers.toggle( layer )`.
    pub fn toggle(&mut self, layer: u32) {
        self.mask ^= 1u32 << layer;
    }

    /// `Layers.disable( layer )`.
    pub fn disable(&mut self, layer: u32) {
        self.mask &= !(1u32 << layer);
    }

    /// `Layers.disableAll()`.
    pub fn disable_all(&mut self) {
        self.mask = 0;
    }

    /// `Layers.test( layers )`.
    pub fn test(&self, layers: &Layers) -> bool {
        (self.mask & layers.mask) != 0
    }

    /// `Layers.isEnabled( layer )`.
    pub fn is_enabled(&self, layer: u32) -> bool {
        (self.mask & (1u32 << layer)) != 0
    }
}
