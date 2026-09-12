//! Port of `three.js/src/core/BufferGeometry.js` (rung 1 subset: interleaved-free
//! `f32` attributes plus a `u16`/`u32` index).

#[derive(Clone, Debug)]
pub struct BufferAttribute {
    pub array: Vec<f32>,
    pub item_size: usize,
}

impl BufferAttribute {
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self { array, item_size }
    }

    pub fn count(&self) -> usize {
        self.array.len() / self.item_size
    }
}

#[derive(Clone, Debug)]
pub enum Index {
    U16(Vec<u16>),
    U32(Vec<u32>),
}

impl Index {
    pub fn count(&self) -> usize {
        match self {
            Index::U16(v) => v.len(),
            Index::U32(v) => v.len(),
        }
    }
}

/// A geometry with the named attributes three.js uses (`position`, `normal`,
/// `uv`). Rung 1 only consumes `position`, but the full set is generated so the
/// data matches three.js byte for byte.
#[derive(Clone, Debug, Default)]
pub struct BufferGeometry {
    pub position: Option<BufferAttribute>,
    pub normal: Option<BufferAttribute>,
    pub uv: Option<BufferAttribute>,
    pub index: Option<Index>,
}

impl BufferGeometry {
    pub fn new() -> Self {
        Self::default()
    }

    /// `BufferGeometry.setIndex( array )` — picks `Uint16` when it fits, the
    /// same rule three.js uses.
    pub fn set_index(&mut self, indices: &[u32]) {
        let max = indices.iter().copied().max().unwrap_or(0);
        self.index = Some(if max > 65535 {
            Index::U32(indices.to_vec())
        } else {
            Index::U16(indices.iter().map(|&i| i as u16).collect())
        });
    }
}
