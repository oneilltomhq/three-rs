//! Shared helpers for the geometry tests.
//!
//! Two gates per geometry:
//!
//! * `run_std_geometry_tests` — the meaningful half of
//!   `three.js/test/unit/utils/qunit-utils.js`'s `runStdGeometryTests`. The
//!   QUnit version spends itself on `clone()`/`uuid`/`toJSON()` round trips,
//!   none of which this port has; what is left that actually constrains the
//!   generated buffers is: the attributes three.js sets are present and agree
//!   on vertex count, every value is finite, the index is in range and a whole
//!   number of triangles, and the bounding box/sphere are well formed.
//!
//! * `check_sample` — a bit-exactness check against values computed once by
//!   three.js itself (`node` over `~/src/vendor/three.js/src/Three.js`; the
//!   generator lives in the commit message for this file's tests). Samples are
//!   the first and last few items of each attribute plus both ends of the
//!   index, which is enough to catch an operation-order divergence that the
//!   loose QUnit tests would sail past.

#![allow(dead_code)]

use three_rs::core::{BufferGeometry, Index};
use three_rs::geometries::Group;

/// Every sampled value is a `Float32Array` entry widened to `f64`, so three.js
/// and the port must agree exactly; the epsilon only absorbs decimal printing.
pub const EPS: f64 = 1e-12;

pub struct AttrSample {
    pub count: usize,
    pub head: &'static [f64],
    pub tail: &'static [f64],
}

#[derive(PartialEq, Eq, Debug)]
pub enum IndexKind {
    U16,
    U32,
}

pub struct IndexSample {
    pub kind: IndexKind,
    pub count: usize,
    pub head: &'static [u32],
    pub tail: &'static [u32],
}

pub struct GeometrySample {
    /// The three.js expression the sample was generated from.
    pub expr: &'static str,
    pub position: Option<AttrSample>,
    pub normal: Option<AttrSample>,
    pub uv: Option<AttrSample>,
    pub index: Option<IndexSample>,
    pub groups: &'static [Group],
    pub bounding_box: (&'static [f64], &'static [f64]),
    pub bounding_sphere: (&'static [f64], f64),
}

fn check_attr(expr: &str, name: &str, got: Option<&three_rs::core::BufferAttribute>, want: &AttrSample) {
    let got = got.unwrap_or_else(|| panic!("{expr}: {name} attribute missing"));
    assert_eq!(got.count(), want.count, "{expr}: {name}.count");

    let array: Vec<f64> = got.array.iter().map(|&v| v as f64).collect();

    for (i, &w) in want.head.iter().enumerate() {
        assert!(
            (array[i] - w).abs() <= EPS,
            "{expr}: {name}[{i}] = {} want {w}",
            array[i]
        );
    }

    let offset = array.len() - want.tail.len();
    for (i, &w) in want.tail.iter().enumerate() {
        assert!(
            (array[offset + i] - w).abs() <= EPS,
            "{expr}: {name}[{}] = {} want {w}",
            offset + i,
            array[offset + i]
        );
    }
}

fn index_values(index: &Index) -> Vec<u32> {
    match index {
        Index::U16(v) => v.iter().map(|&i| i as u32).collect(),
        Index::U32(v) => v.clone(),
    }
}

pub fn check_sample(sample: &GeometrySample, geometry: &BufferGeometry) {
    check_sample_with_groups(sample, geometry, &[]);
}

pub fn check_sample_with_groups(
    sample: &GeometrySample,
    geometry: &BufferGeometry,
    groups: &[Group],
) {
    let expr = sample.expr;

    match &sample.position {
        Some(want) => check_attr(expr, "position", geometry.position.as_ref(), want),
        None => assert!(geometry.position.is_none(), "{expr}: unexpected position"),
    }
    match &sample.normal {
        Some(want) => check_attr(expr, "normal", geometry.normal.as_ref(), want),
        None => assert!(geometry.normal.is_none(), "{expr}: unexpected normal"),
    }
    match &sample.uv {
        Some(want) => check_attr(expr, "uv", geometry.uv.as_ref(), want),
        None => assert!(geometry.uv.is_none(), "{expr}: unexpected uv"),
    }

    match &sample.index {
        Some(want) => {
            let index = geometry
                .index
                .as_ref()
                .unwrap_or_else(|| panic!("{expr}: index missing"));

            // three.js picks Uint16Array when the largest index fits.
            let kind = match index {
                Index::U16(_) => IndexKind::U16,
                Index::U32(_) => IndexKind::U32,
            };
            assert_eq!(kind, want.kind, "{expr}: index array type");
            assert_eq!(index.count(), want.count, "{expr}: index length");

            let values = index_values(index);
            for (i, &w) in want.head.iter().enumerate() {
                assert_eq!(values[i], w, "{expr}: index[{i}]");
            }
            let offset = values.len() - want.tail.len();
            for (i, &w) in want.tail.iter().enumerate() {
                assert_eq!(values[offset + i], w, "{expr}: index[{}]", offset + i);
            }
        }
        None => assert!(geometry.index.is_none(), "{expr}: unexpected index"),
    }

    assert_eq!(groups, sample.groups, "{expr}: groups");

    // bounding box / sphere, the part of `runStdGeometryTests` that constrains
    // the data rather than the object identity.
    let (min, max) = bounding_box(geometry);
    for i in 0..3 {
        assert!(
            (min[i] - sample.bounding_box.0[i]).abs() <= EPS,
            "{expr}: boundingBox.min[{i}] = {} want {}",
            min[i],
            sample.bounding_box.0[i]
        );
        assert!(
            (max[i] - sample.bounding_box.1[i]).abs() <= EPS,
            "{expr}: boundingBox.max[{i}] = {} want {}",
            max[i],
            sample.bounding_box.1[i]
        );
    }

    let (center, radius) = bounding_sphere(geometry);
    for i in 0..3 {
        assert!(
            (center[i] - sample.bounding_sphere.0[i]).abs() <= EPS,
            "{expr}: boundingSphere.center[{i}] = {} want {}",
            center[i],
            sample.bounding_sphere.0[i]
        );
    }
    assert!(
        (radius - sample.bounding_sphere.1).abs() <= EPS,
        "{expr}: boundingSphere.radius = {radius} want {}",
        sample.bounding_sphere.1
    );
}

/// `BufferGeometry.computeBoundingBox()`, reduced to the position attribute.
pub fn bounding_box(geometry: &BufferGeometry) -> ([f64; 3], [f64; 3]) {
    let position = geometry.position.as_ref().expect("position");
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for i in 0..position.count() {
        let v = [position.get_x(i), position.get_y(i), position.get_z(i)];
        for c in 0..3 {
            if v[c] < min[c] {
                min[c] = v[c];
            }
            if v[c] > max[c] {
                max[c] = v[c];
            }
        }
    }
    (min, max)
}

/// `BufferGeometry.computeBoundingSphere()` — centre from the bounding box,
/// radius from the farthest vertex.
pub fn bounding_sphere(geometry: &BufferGeometry) -> ([f64; 3], f64) {
    let position = geometry.position.as_ref().expect("position");
    let (min, max) = bounding_box(geometry);
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];

    let mut max_radius_sq: f64 = 0.0;
    for i in 0..position.count() {
        let d = [
            position.get_x(i) - center[0],
            position.get_y(i) - center[1],
            position.get_z(i) - center[2],
        ];
        let sq = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        max_radius_sq = max_radius_sq.max(sq);
    }

    (center, max_radius_sq.sqrt())
}

/// The data-constraining half of `runStdGeometryTests`.
pub fn run_std_geometry_tests(label: &str, geometry: &BufferGeometry) {
    let position = geometry
        .position
        .as_ref()
        .unwrap_or_else(|| panic!("{label}: no position attribute"));
    assert_eq!(position.item_size, 3, "{label}: position.itemSize");
    assert!(position.count() > 0, "{label}: empty position");

    for (i, v) in position.array.iter().enumerate() {
        assert!(v.is_finite(), "{label}: position[{i}] is not finite");
    }

    if let Some(normal) = &geometry.normal {
        assert_eq!(normal.item_size, 3, "{label}: normal.itemSize");
        assert_eq!(normal.count(), position.count(), "{label}: normal.count");
        for (i, v) in normal.array.iter().enumerate() {
            assert!(v.is_finite(), "{label}: normal[{i}] is not finite");
        }
    }

    if let Some(uv) = &geometry.uv {
        assert_eq!(uv.item_size, 2, "{label}: uv.itemSize");
        assert_eq!(uv.count(), position.count(), "{label}: uv.count");
        for (i, v) in uv.array.iter().enumerate() {
            assert!(v.is_finite(), "{label}: uv[{i}] is not finite");
        }
    }

    if let Some(index) = &geometry.index {
        assert_eq!(index.count() % 3, 0, "{label}: index is not whole triangles");
        let values = index_values(index);
        let max = values.iter().copied().max().unwrap_or(0);
        assert!(
            (max as usize) < position.count(),
            "{label}: index {max} out of range for {} vertices",
            position.count()
        );
        if let Index::U16(_) = index {
            assert!(max <= 65535, "{label}: Uint16 index overflowed");
        }
    }

    let (min, max) = bounding_box(geometry);
    for c in 0..3 {
        assert!(min[c].is_finite() && max[c].is_finite(), "{label}: boundingBox");
        assert!(min[c] <= max[c], "{label}: boundingBox inverted");
    }
    let (_, radius) = bounding_sphere(geometry);
    assert!(radius.is_finite() && radius > 0.0, "{label}: boundingSphere.radius");
}

/// `BufferGeometry.setIndex( array )` picks the narrowest typed array that
/// fits; generators that hand `setIndex` a ready-made `Uint32Array` (the
/// `TeapotGeometry` addon) are exempt, hence the separate check.
pub fn check_index_is_narrowest(label: &str, geometry: &BufferGeometry) {
    let index = geometry.index.as_ref().expect("index");
    let max = index_values(index).into_iter().max().unwrap_or(0);
    match index {
        Index::U16(_) => assert!(max <= 65535, "{label}: Uint16 index overflowed"),
        Index::U32(_) => assert!(max > 65535, "{label}: Uint32 index where Uint16 fits"),
    }
}

/// Groups must tile the index buffer exactly, the way three.js' generators build
/// them.
pub fn check_groups_cover_index(label: &str, geometry: &BufferGeometry, groups: &[Group]) {
    let mut expected_start = 0usize;
    for (i, g) in groups.iter().enumerate() {
        assert_eq!(g.start, expected_start, "{label}: groups[{i}].start");
        expected_start += g.count;
    }
    let index_count = geometry.index.as_ref().map(|i| i.count()).unwrap_or(0);
    assert_eq!(expected_start, index_count, "{label}: groups do not cover the index");
}
