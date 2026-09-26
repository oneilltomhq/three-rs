//! `IndirectStorageBufferAttribute` — `src/renderers/common/IndirectStorageBufferAttribute.js`.

use std::rc::Rc;

use crate::nodes::node::BufferId;

/// `new IndirectStorageBufferAttribute( new Uint32Array( n ), itemSize )`: a
/// `Uint32Array` that lives on the GPU as one buffer usable both as storage
/// (a compute kernel writes it through `storage( attribute, struct, count )`)
/// and as the argument buffer of `drawIndirect` / `drawIndexedIndirect` /
/// `dispatchWorkgroupsIndirect`.
///
/// A handle: clones are the same attribute, and the renderer keys the one GPU
/// buffer on [`id`](Self::id), which is also the id every storage node built
/// over it carries — so a kernel writing it and a draw reading it meet on the
/// same buffer, as they do in three through `backend.get( attribute )`.
///
/// The array is the buffer's initial contents only. It is uploaded once, when
/// the GPU buffer is first made; three.js never reads it back either, and a
/// kernel's writes are visible through `Renderer::read_indirect_buffer`.
#[derive(Clone, Debug)]
pub struct IndirectStorageBufferAttribute {
    id: BufferId,
    array: Rc<Vec<u32>>,
    /// `BufferAttribute.itemSize`.
    pub item_size: usize,
}

impl IndirectStorageBufferAttribute {
    pub fn new(array: Vec<u32>, item_size: usize) -> Self {
        Self {
            id: BufferId::next(),
            array: Rc::new(array),
            item_size,
        }
    }

    /// The identity the renderer's GPU buffer is keyed on.
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// `attribute.array` — the initial contents.
    pub fn array(&self) -> Rc<Vec<u32>> {
        self.array.clone()
    }

    /// `BufferAttribute.count` — `array.length / itemSize`.
    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}
