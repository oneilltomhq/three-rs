//! `Morph.js`' `getEntry()`: the packing of `geometry.morphAttributes.position`
//! into one `DataArrayTexture`, which nothing on the graded frame exercises
//! (both influences are 0 there).

use std::rc::Rc;

use three_rs::core::BufferAttribute;
use three_rs::nodes::morph::get_entry;
use three_rs::{box_geometry, BufferGeometry};

fn geometry_with_morph(segments: usize, targets: usize) -> Rc<BufferGeometry> {
    let mut geometry = box_geometry(2.0, 2.0, 2.0, segments, segments, segments);
    let count = geometry.get_attribute("position").unwrap().count();
    let attributes = (0..targets)
        .map(|target| {
            BufferAttribute::new(
                (0..count * 3).map(|i| (target * 1000 + i) as f32).collect(),
                3,
            )
        })
        .collect();
    geometry.set_morph_attribute("position", attributes);
    Rc::new(geometry)
}

#[test]
fn no_morph_attributes_means_no_entry() {
    let geometry = Rc::new(box_geometry(2.0, 2.0, 2.0, 1, 1, 1));
    assert!(get_entry(&geometry).is_none());
}

#[test]
fn small_geometry_stays_one_row() {
    // 6 faces * 2x2 vertices = 24 positions, so `width` is under the 4096 cap.
    let geometry = geometry_with_morph(1, 2);
    let entry = get_entry(&geometry).unwrap();

    assert_eq!((entry.width, entry.height, entry.count), (24, 1, 2));
    assert_eq!(entry.stride, 1);
    assert_eq!(entry.texture.size(), (24, 1, 2));

    let inner = entry.texture.borrow();
    // `24 * 1 * 4 * 2` spells out width * height * channels * layers.
    #[allow(clippy::identity_op)]
    {
        assert_eq!(inner.data.len(), 24 * 1 * 4 * 2);
    }
    // The first texel of each layer is that target's first vertex, xyz then 0.
    assert_eq!(&inner.data[0..4], &[0.0, 1.0, 2.0, 0.0]);
    assert_eq!(
        &inner.data[24 * 4..24 * 4 + 4],
        &[1000.0, 1001.0, 1002.0, 0.0]
    );
}

#[test]
fn the_example_geometry_wraps_to_two_rows() {
    // `webgpu_morphtargets`: 6 * 33^2 = 6534 positions, so 4096 x 2.
    let geometry = geometry_with_morph(32, 2);
    let entry = get_entry(&geometry).unwrap();

    assert_eq!((entry.width, entry.height, entry.count), (4096, 2, 2));
    assert_eq!(entry.texture.borrow().data.len(), 4096 * 2 * 4 * 2);
}

#[test]
fn the_entry_is_cached_per_geometry() {
    let geometry = geometry_with_morph(1, 2);
    let first = get_entry(&geometry).unwrap();
    let second = get_entry(&geometry).unwrap();
    assert_eq!(first.texture.id(), second.texture.id());
}
