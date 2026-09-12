//! `BufferGeometry.addGroup()` — three.js stores draw-range groups on the
//! geometry itself. The crate's `BufferGeometry` has no `groups` field yet, so
//! the generators that set groups expose them through a companion
//! `*_with_groups()` entry point returning this alongside the geometry.
//!
//! (Fold this into `core::BufferGeometry` as a `groups: Vec<Group>` field when
//! the core is next touched.)

/// One entry of `BufferGeometry.groups`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    pub start: usize,
    pub count: usize,
    pub material_index: usize,
}

impl Group {
    pub fn new(start: usize, count: usize, material_index: usize) -> Self {
        Self {
            start,
            count,
            material_index,
        }
    }
}
