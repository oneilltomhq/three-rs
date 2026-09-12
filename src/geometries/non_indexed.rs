//! `BufferGeometry.toNonIndexed()` — lives here rather than in `src/core`
//! because only a geometry generator (`RoundedBoxGeometry`) needs it so far.
//! Fold it into `core::BufferGeometry` when the core is next touched.

use crate::core::{BufferAttribute, BufferGeometry, Index};

fn convert_buffer_attribute(attribute: &BufferAttribute, indices: &[u32]) -> BufferAttribute {
    let item_size = attribute.item_size;
    let array = &attribute.array;

    let mut array2: Vec<f32> = Vec::with_capacity(indices.len() * item_size);

    for &index in indices {
        let start = index as usize * item_size;
        for j in 0..item_size {
            array2.push(array[start + j]);
        }
    }

    BufferAttribute::new(array2, item_size)
}

/// `BufferGeometry.toNonIndexed()`. A geometry with no index is returned
/// unchanged, as in three.js (which also logs a warning).
pub fn to_non_indexed(geometry: &BufferGeometry) -> BufferGeometry {
    let Some(index) = &geometry.index else {
        return geometry.clone();
    };

    let indices: Vec<u32> = match index {
        Index::U16(v) => v.iter().map(|&i| i as u32).collect(),
        Index::U32(v) => v.clone(),
    };

    let mut geometry2 = BufferGeometry::new();

    geometry2.position = geometry
        .position
        .as_ref()
        .map(|a| convert_buffer_attribute(a, &indices));
    geometry2.normal = geometry
        .normal
        .as_ref()
        .map(|a| convert_buffer_attribute(a, &indices));
    geometry2.uv = geometry
        .uv
        .as_ref()
        .map(|a| convert_buffer_attribute(a, &indices));

    geometry2
}
