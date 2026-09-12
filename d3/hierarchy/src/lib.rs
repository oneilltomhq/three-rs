//! A port of d3-hierarchy 3.1.2 to Rust.
//!
//! f64 throughout, and the same operation order as the JS, so results match to
//! the bit. See `README.md` for the node representation.

pub mod node;
pub mod cluster;
pub mod tree;
pub mod partition;
pub mod pack;
pub mod treemap;
pub mod stratify;

pub use node::{hierarchy, hierarchy_with, object_children, Datum, Node, Tree};

/// `src/constant.js` / `src/array.js`: d3's `lcg()`.
pub fn lcg() -> impl FnMut() -> f64 {
    const A: f64 = 1664525.0;
    const C: f64 = 1013904223.0;
    const M: f64 = 4294967296.0; // 2^32
    let mut s: f64 = 1.0;
    move || {
        s = (A * s + C) % M;
        s / M
    }
}

/// `src/array.js`'s `shuffle(array, random)`.
pub fn shuffle<T>(array: &mut [T], random: &mut impl FnMut() -> f64) {
    let mut m = array.len();
    while m != 0 {
        let i = (random() * m as f64) as usize; // `| 0`
        m -= 1;
        array.swap(m, i);
    }
}
