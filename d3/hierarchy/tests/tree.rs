//! Tests for `src/tree.rs`. d3-hierarchy 3.1.2 ships no `test/tree-test.js`
//! (its `test/` holds only `data/`, `hierarchy/`, `pack/`, `treemap/` and
//! `stratify-test.js`), so every expectation below was generated from the
//! vendor JS itself with:
//!
//! ```sh
//! node --input-type=module -e 'import {hierarchy, tree} from "/home/tom/src/vendor/d3-hierarchy/src/index.js"; import fs from "fs"; const data = JSON.parse(fs.readFileSync(common::data("simple.json"))); const root = hierarchy(data).sum(d => d.value); tree()(root); console.log(JSON.stringify(root.descendants().map(d => [d.depth, d.height, d.value, d.x, d.y])));'
//! ```
//!
//! (the same one-liner with `tree().size([100,200])`, `.nodeSize([10,20])`, a
//! custom `.separation`, `hierarchy({})`, the lopsided literal in
//! `lopsided()` below, and `test/data/flare.json`; values printed with
//! `toPrecision(17)` so the literals are the exact f64s).

mod common;
use d3_hierarchy::tree::{tree, Tidy};
use d3_hierarchy::{hierarchy, Tree};
use serde_json::json;

fn simple() -> serde_json::Value {
    let s = std::fs::read_to_string(common::data("simple.json")).unwrap();
    serde_json::from_str(&s).unwrap()
}

fn lopsided() -> serde_json::Value {
    json!({"children": [
        {"children": [{"children": [{}, {}]}, {}]},
        {"children": [{}, {"children": [{}, {}, {}]}]},
        {}
    ]})
}

/// `root.descendants().map(d => [d.depth, d.x, d.y])`
fn layout(t: &Tree) -> Vec<(usize, f64, f64)> {
    t.descendants(t.root)
        .into_iter()
        .map(|i| (t.nodes[i].depth, t.nodes[i].x, t.nodes[i].y))
        .collect()
}

#[test]
fn tree_default_size_is_1x1() {
    let layout_fn = tree();
    assert_eq!(layout_fn.get_size(), Some([1.0, 1.0]));
    assert_eq!(layout_fn.get_node_size(), None);
    let mut root = hierarchy(&simple());
    root.sum(|d| d["value"].as_f64().unwrap_or(f64::NAN));
    layout_fn.tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.50000000000000000, 0.0),
            (1, 0.25000000000000000, 0.50000000000000000),
            (1, 0.75000000000000000, 0.50000000000000000),
            (2, 0.12500000000000000, 1.0),
            (2, 0.25000000000000000, 1.0),
            (2, 0.37500000000000000, 1.0),
            (2, 0.62500000000000000, 1.0),
            (2, 0.75000000000000000, 1.0),
            (2, 0.87500000000000000, 1.0),
        ]
    );
}

#[test]
fn tree_size_scales_x_and_y_to_the_extent() {
    let layout_fn = tree().size([100.0, 200.0]);
    assert_eq!(layout_fn.get_size(), Some([100.0, 200.0]));
    let mut root = hierarchy(&simple());
    root.sum(|d| d["value"].as_f64().unwrap_or(f64::NAN));
    layout_fn.tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 50.0, 0.0),
            (1, 25.0, 100.0),
            (1, 75.0, 100.0),
            (2, 12.500000000000000, 200.0),
            (2, 25.0, 200.0),
            (2, 37.500000000000000, 200.0),
            (2, 62.500000000000000, 200.0),
            (2, 75.0, 200.0),
            (2, 87.500000000000000, 200.0),
        ]
    );
}

#[test]
fn tree_node_size_does_not_normalize_the_extent() {
    let layout_fn = tree().node_size([10.0, 20.0]);
    assert_eq!(layout_fn.get_node_size(), Some([10.0, 20.0]));
    assert_eq!(layout_fn.get_size(), None);
    let mut root = hierarchy(&simple());
    root.sum(|d| d["value"].as_f64().unwrap_or(f64::NAN));
    layout_fn.tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.0, 0.0),
            (1, -20.0, 20.0),
            (1, 20.0, 20.0),
            (2, -30.0, 40.0),
            (2, -20.0, 40.0),
            (2, -10.0, 40.0),
            (2, 10.0, 40.0),
            (2, 20.0, 40.0),
            (2, 30.0, 40.0),
        ]
    );
}

#[test]
fn tree_separation_radial_like() {
    // `(a, b) => (a.parent === b.parent ? 1 : 2) / a.depth`, the commented-out
    // `radialSeparation` in tree.js. With the default 1x1 size the extent
    // normalization brings this back to the same layout as the default.
    let layout_fn = tree().separation(|t: &Tree, a, b| {
        (if t.nodes[a].parent == t.nodes[b].parent { 1.0 } else { 2.0 }) / t.nodes[a].depth as f64
    });
    let mut root = hierarchy(&simple());
    root.sum(|d| d["value"].as_f64().unwrap_or(f64::NAN));
    layout_fn.tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.50000000000000000, 0.0),
            (1, 0.25000000000000000, 0.50000000000000000),
            (1, 0.75000000000000000, 0.50000000000000000),
            (2, 0.12500000000000000, 1.0),
            (2, 0.25000000000000000, 1.0),
            (2, 0.37500000000000000, 1.0),
            (2, 0.62500000000000000, 1.0),
            (2, 0.75000000000000000, 1.0),
            (2, 0.87500000000000000, 1.0),
        ]
    );
}

#[test]
fn tree_a_single_node_hits_the_left_eq_right_and_depth_or_1_branches() {
    let mut root = hierarchy(&json!({}));
    tree().tree(&mut root);
    assert_eq!(layout(&root), [(0, 0.50000000000000000, 0.0)]);
}

#[test]
fn tree_lopsided_tree_exercises_apportion_threads_and_shifts() {
    let mut root = hierarchy(&lopsided());
    tree().tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.53333333333333333, 0.0),
            (1, 0.26666666666666666, 0.33333333333333331),
            (1, 0.66666666666666663, 0.33333333333333331),
            (1, 0.80000000000000004, 0.33333333333333331),
            (2, 0.20000000000000001, 0.66666666666666663),
            (2, 0.33333333333333331, 0.66666666666666663),
            (2, 0.59999999999999998, 0.66666666666666663),
            (2, 0.73333333333333328, 0.66666666666666663),
            (3, 0.13333333333333333, 1.0),
            (3, 0.26666666666666666, 1.0),
            (3, 0.59999999999999998, 1.0),
            (3, 0.73333333333333328, 1.0),
            (3, 0.86666666666666670, 1.0),
        ]
    );
}

#[test]
fn tree_lopsided_tree_unnormalized_prelim_coordinates() {
    // nodeSize keeps the raw Buchheim coordinates, so this asserts the thread
    // and shift bookkeeping directly rather than through the rescaling.
    let mut root = hierarchy(&lopsided());
    tree().node_size([1.0, 1.0]).tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.0, 0.0),
            (1, -2.0, 1.0),
            (1, 1.0, 1.0),
            (1, 2.0, 1.0),
            (2, -2.5000000000000000, 2.0),
            (2, -1.5000000000000000, 2.0),
            (2, 0.50000000000000000, 2.0),
            (2, 1.5000000000000000, 2.0),
            (3, -3.0, 3.0),
            (3, -2.0, 3.0),
            (3, 0.50000000000000000, 3.0),
            (3, 1.5000000000000000, 3.0),
            (3, 2.5000000000000000, 3.0),
        ]
    );
}

#[test]
fn tree_lopsided_tree_with_a_custom_separation() {
    // `(a, b) => a.parent === b.parent ? 1 : 3`, with nodeSize so the wider
    // separation of non-siblings is visible in the output.
    let mut root = hierarchy(&lopsided());
    let layout_fn: Tidy = tree()
        .node_size([1.0, 1.0])
        .separation(|t: &Tree, a, b| if t.nodes[a].parent == t.nodes[b].parent { 1.0 } else { 3.0 });
    layout_fn.tree(&mut root);
    assert_eq!(
        layout(&root),
        [
            (0, 0.0, 0.0),
            (1, -2.5000000000000000, 1.0),
            (1, 1.5000000000000000, 1.0),
            (1, 2.5000000000000000, 1.0),
            (2, -3.0, 2.0),
            (2, -2.0, 2.0),
            (2, 1.0, 2.0),
            (2, 2.0, 2.0),
            (3, -3.5000000000000000, 3.0),
            (3, -2.5000000000000000, 3.0),
            (3, 1.0, 3.0),
            (3, 2.0, 3.0),
            (3, 3.0, 3.0),
        ]
    );
}

#[test]
fn tree_flare() {
    let s = std::fs::read_to_string(common::data("flare.json")).unwrap();
    let data: serde_json::Value = serde_json::from_str(&s).unwrap();
    let mut root = hierarchy(&data);
    tree().size([1.0, 1.0]).tree(&mut root);
    let d = layout(&root);
    assert_eq!(d.len(), 252);
    // `root.descendants()` indices 0, 1, 2, 3, 100, 200, 251.
    assert_eq!(d[0], (0, 0.40547945205479452, 0.0));
    assert_eq!(d[1], (1, 0.039726027397260277, 0.25000000000000000));
    assert_eq!(d[2], (1, 0.11780821917808219, 0.25000000000000000));
    assert_eq!(d[3], (1, 0.17534246575342466, 0.25000000000000000));
    assert_eq!(d[100], (2, 0.60547945205479448, 0.50000000000000000));
    assert_eq!(d[200], (3, 0.76438356164383559, 0.75000000000000000));
    assert_eq!(d[251], (4, 0.99452054794520550, 1.0));
}
