//! The keys of the per-draw binding caches (issue #137): which draw a
//! persistent buffer belongs to, and how long an unused entry survives.
//!
//! three.js keeps one GPU buffer per binding group for the object's lifetime
//! (`Bindings` / `UniformBuffer`) and `writeBuffer`s the changed bytes into it
//! each frame. The port re-created every one of them on every draw, which at a
//! hundred draws a frame was hundreds of wgpu allocations and the whole of
//! `webgpu_materials_basic`'s frame-time floor. Everything here is pure, so the
//! identity rules can be tested without a device.

use std::collections::HashMap;

use super::CACHE_GRACE_FRAMES;

/// One draw's identity, for the buffers that hold its per-draw data — its
/// uniform groups, its bone matrices and morph influences, its instance
/// matrix and colours.
///
/// Every field is an id from a never-reused counter (`Object3D.id`,
/// `BufferGeometry.id`, `Material.id`) or a constant, never an address, for
/// the reason `docs/scene-graph.md` gives under *Identity and eviction*.
///
/// `occurrence` is what makes the key unique within one pass. A pass resolves
/// all its draws before it submits them, and a `queue.write_buffer` lands at
/// the next submit, so two draws of one pass that shared a buffer would both
/// read whichever wrote last. The same object, geometry and material can be
/// drawn twice in one pass (a material array naming one material in two
/// groups, the renderer's own quad drawn with one material twice), so the
/// n-th such draw gets the n-th buffer. Across passes sharing is safe — each
/// pass is its own submit, and the queue applies the next pass's writes after
/// the previous pass has read its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct DrawKey {
    /// `Object3D.id`; `None` for the renderer's own draws (quad, background,
    /// output pass), which carry no application object.
    pub object: Option<u32>,
    pub geometry: usize,
    /// `MaterialKey::id` and `::variant` — the *source* material's id, which is
    /// stable across frames where the per-frame clone's is not.
    pub material: usize,
    pub variant: u64,
    pub occurrence: u32,
}

/// Hands out [`DrawKey::occurrence`] for one pass.
#[derive(Default)]
pub(super) struct Occurrences(HashMap<DrawKey, u32>);

impl Occurrences {
    /// `key` with its `occurrence` set to how many times this pass has already
    /// seen the same draw.
    pub fn next(&mut self, key: DrawKey) -> DrawKey {
        let base = DrawKey {
            occurrence: 0,
            ..key
        };
        let seen = self.0.entry(base).or_insert(0);
        let occurrence = *seen;
        *seen += 1;
        DrawKey { occurrence, ..base }
    }
}

/// Who a per-draw buffer belongs to: a draw, or a compute kernel (keyed by
/// its `ComputeProgram::cache_key`; a kernel's uniforms are the same on every
/// dispatch, and each dispatch is its own submit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SlotOwner {
    Draw(DrawKey),
    Compute(u64),
}

/// The group a vertex buffer's [`SlotKey`] is filed under — no bind group
/// has this index, so an instance matrix bound as a vertex buffer and the
/// same matrix bound as a uniform are two buffers, as their usages require.
pub(super) const VERTEX_SLOTS: u32 = u32::MAX;

/// One persistent per-draw buffer: the owner, the bind group and the binding
/// within it (or [`VERTEX_SLOTS`] and the vertex buffer slot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SlotKey {
    pub owner: SlotOwner,
    pub group: u32,
    pub binding: u32,
}

/// Whether an entry last used at `last_used` survives a sweep at `frames`:
/// the same [`CACHE_GRACE_FRAMES`] window `node_builder_states` and `buffers`
/// age out by.
pub(super) fn is_fresh(last_used: u64, frames: u64) -> bool {
    last_used >= frames.saturating_sub(CACHE_GRACE_FRAMES)
}

/// `bytes` padded with zeros to the 4-byte multiple `write_buffer` requires.
pub(super) fn padded(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if bytes.len().is_multiple_of(4) {
        std::borrow::Cow::Borrowed(bytes)
    } else {
        let mut padded = bytes.to_vec();
        padded.resize(bytes.len().next_multiple_of(4), 0);
        std::borrow::Cow::Owned(padded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(object: u32, material: usize) -> DrawKey {
        DrawKey {
            object: Some(object),
            geometry: 7,
            material,
            variant: 0,
            occurrence: 0,
        }
    }

    #[test]
    fn a_repeated_draw_in_one_pass_gets_its_own_slot() {
        let mut pass = Occurrences::default();
        let first = pass.next(draw(1, 2));
        let other = pass.next(draw(3, 2));
        let second = pass.next(draw(1, 2));
        assert_eq!((first.occurrence, other.occurrence), (0, 0));
        assert_eq!(second.occurrence, 1);
        assert_ne!(first, second);

        // The next pass starts counting again, so a steady frame keys every
        // draw the same way it did last frame.
        let mut next = Occurrences::default();
        assert_eq!(next.next(draw(1, 2)), first);
        assert_eq!(next.next(draw(1, 2)), second);
    }

    #[test]
    fn an_entry_survives_the_grace_window_and_no_longer() {
        assert!(is_fresh(10, 10));
        assert!(is_fresh(10, 10 + CACHE_GRACE_FRAMES));
        assert!(!is_fresh(10, 11 + CACHE_GRACE_FRAMES));
        // The first frames cannot underflow into evicting everything.
        assert!(is_fresh(0, 1));
    }

    #[test]
    fn padding_is_to_four_bytes_and_zero_filled() {
        assert_eq!(&*padded(&[1, 2, 3, 4]), &[1, 2, 3, 4]);
        assert_eq!(&*padded(&[1, 2, 3, 4, 5]), &[1, 2, 3, 4, 5, 0, 0, 0]);
        assert!(padded(&[]).is_empty());
    }
}
