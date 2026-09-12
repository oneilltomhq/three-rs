//! Ports of d3-hierarchy/test/treemap/{index,squarify,dice,slice,sliceDice,
//! binary,resquarify}-test.js, with the expectations exactly as written there.
//!
//! The JS compares `root.descendants().map(round)` with deepEqual, where
//! `round` (test/treemap/round.js) keeps only x0/y0/x1/y1 rounded to two
//! decimals; `round_node_xy` below is the equivalent, returning
//! `[x0, y0, x1, y1]` (the literals are reordered from the JS's
//! `{x0, x1, y0, y1}` spelling accordingly). All comparisons are exact f64.
//!
//! Skipped, with reasons, are listed at the bottom of this file.

use d3_hierarchy::node::{Node, Tree};
use d3_hierarchy::treemap::{
    binary, dice, js_round, phi, resquarify, slice, slice_dice, squarify, squarify_ratio, treemap,
};
use d3_hierarchy::{hierarchy, Datum};
use serde_json::json;

const DATA: &str = "/home/tom/src/vendor/d3-hierarchy/test/data";

fn data(name: &str) -> Datum {
    serde_json::from_str(&std::fs::read_to_string(format!("{DATA}/{name}")).unwrap()).unwrap()
}

fn simple2() -> Datum {
    data("simple2.json")
}

fn default_value(d: &Datum) -> f64 {
    d.get("value").and_then(|v| v.as_f64()).unwrap_or(f64::NAN)
}

/// test/treemap/round.js
fn r(x: f64) -> f64 {
    js_round(x * 100.0) / 100.0
}

fn round_node_xy(n: &Node) -> [f64; 4] {
    [r(n.x0), r(n.y0), r(n.x1), r(n.y1)]
}

fn rounded_children(t: &Tree, parent: usize) -> Vec<[f64; 4]> {
    t.nodes[parent]
        .children()
        .iter()
        .map(|&i| round_node_xy(&t.nodes[i]))
        .collect()
}

fn rounded_descendants(t: &Tree) -> Vec<[f64; 4]> {
    t.descendants(t.root)
        .into_iter()
        .map(|i| round_node_xy(&t.nodes[i]))
        .collect()
}

/// The tiling tests build a bare `{value, children: [{value}, …]}` object
/// rather than a real hierarchy; this is the arena equivalent.
fn bare(depth: usize, value: f64, children: &[f64]) -> Tree {
    let mut nodes = vec![Node::new(json!({}))];
    nodes[0].value = Some(value);
    nodes[0].depth = depth;
    let mut idx = Vec::new();
    for (i, &v) in children.iter().enumerate() {
        let mut n = Node::new(json!({}));
        n.value = Some(v);
        n.depth = depth + 1;
        n.parent = Some(0);
        nodes.push(n);
        idx.push(i + 1);
    }
    nodes[0].children = Some(idx);
    Tree { nodes, root: 0 }
}

const SEVEN: [f64; 7] = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];

fn sum_sort_desc(data: &Datum) -> Tree {
    let mut root = hierarchy(data);
    root.sum(default_value);
    root.sort(|a, b| b.value.partial_cmp(&a.value).unwrap());
    root
}

// ------------------------------------------------------------- dice-test.js

#[test]
fn treemap_dice_generates_a_diced_layout() {
    let mut root = bare(0, 24.0, &SEVEN);
    dice(&mut root, 0, 0.0, 0.0, 4.0, 6.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 1.00, 6.00],
            [1.00, 0.00, 2.00, 6.00],
            [2.00, 0.00, 2.67, 6.00],
            [2.67, 0.00, 3.17, 6.00],
            [3.17, 0.00, 3.50, 6.00],
            [3.50, 0.00, 3.83, 6.00],
            [3.83, 0.00, 4.00, 6.00],
        ]
    );
}

#[test]
fn treemap_dice_handles_a_degenerate_empty_parent() {
    let mut root = bare(0, 0.0, &[0.0, 0.0]);
    dice(&mut root, 0, 0.0, 0.0, 0.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.00, 0.00, 0.00, 4.00], [0.00, 0.00, 0.00, 4.00]]
    );
}

// ------------------------------------------------------------ slice-test.js

#[test]
fn treemap_slice_generates_a_sliced_layout() {
    let mut root = bare(0, 24.0, &SEVEN);
    slice(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 6.00, 1.00],
            [0.00, 1.00, 6.00, 2.00],
            [0.00, 2.00, 6.00, 2.67],
            [0.00, 2.67, 6.00, 3.17],
            [0.00, 3.17, 6.00, 3.50],
            [0.00, 3.50, 6.00, 3.83],
            [0.00, 3.83, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_slice_handles_a_degenerate_empty_parent() {
    let mut root = bare(0, 0.0, &[0.0, 0.0]);
    slice(&mut root, 0, 0.0, 0.0, 4.0, 0.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.00, 0.00, 4.00, 0.00], [0.00, 0.00, 4.00, 0.00]]
    );
}

// -------------------------------------------------------- sliceDice-test.js

#[test]
fn treemap_slice_dice_uses_slice_for_odd_depth() {
    let mut root = bare(1, 24.0, &SEVEN);
    slice_dice(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 6.00, 1.00],
            [0.00, 1.00, 6.00, 2.00],
            [0.00, 2.00, 6.00, 2.67],
            [0.00, 2.67, 6.00, 3.17],
            [0.00, 3.17, 6.00, 3.50],
            [0.00, 3.50, 6.00, 3.83],
            [0.00, 3.83, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_slice_dice_uses_dice_for_even_depth() {
    let mut root = bare(2, 24.0, &SEVEN);
    slice_dice(&mut root, 0, 0.0, 0.0, 4.0, 6.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 1.00, 6.00],
            [1.00, 0.00, 2.00, 6.00],
            [2.00, 0.00, 2.67, 6.00],
            [2.67, 0.00, 3.17, 6.00],
            [3.17, 0.00, 3.50, 6.00],
            [3.50, 0.00, 3.83, 6.00],
            [3.83, 0.00, 4.00, 6.00],
        ]
    );
}

// --------------------------------------------------------- squarify-test.js

#[test]
fn treemap_squarify_generates_a_squarified_layout() {
    let mut root = bare(0, 24.0, &SEVEN);
    squarify(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 5.40, 3.17],
            [3.00, 3.17, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_squarify_does_not_produce_a_stable_update() {
    let mut root = bare(0, 20.0, &[10.0, 10.0]);
    squarify(&mut root, 0, 0.0, 0.0, 20.0, 10.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 20.0, 10.0]]
    );
    squarify(&mut root, 0, 0.0, 0.0, 10.0, 20.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [0.0, 10.0, 10.0, 20.0]]
    );
}

#[test]
fn treemap_squarify_ratio_observes_the_specified_ratio() {
    let tile = squarify_ratio(1.0);
    let mut root = bare(0, 24.0, &SEVEN);
    tile(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 4.20, 4.00],
            [4.20, 2.33, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_squarify_handles_a_degenerate_tall_empty_parent() {
    let mut root = bare(0, 0.0, &[0.0, 0.0]);
    squarify(&mut root, 0, 0.0, 0.0, 0.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.00, 0.00, 0.00, 4.00], [0.00, 0.00, 0.00, 4.00]]
    );
}

#[test]
fn treemap_squarify_handles_a_degenerate_wide_empty_parent() {
    let mut root = bare(0, 0.0, &[0.0, 0.0]);
    squarify(&mut root, 0, 0.0, 0.0, 4.0, 0.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.00, 0.00, 4.00, 0.00], [0.00, 0.00, 4.00, 0.00]]
    );
}

#[test]
fn treemap_squarify_handles_a_leading_zero_value() {
    let mut root = bare(0, 24.0, &[0.0, 6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0]);
    squarify(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 3.00, 0.00],
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 5.40, 3.17],
            [3.00, 3.17, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

// ----------------------------------------------------------- binary-test.js

#[test]
fn treemap_binary_generates_a_binary_treemap_layout() {
    let mut root = bare(0, 24.0, &SEVEN);
    binary(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 4.20, 4.00],
            [4.20, 2.33, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_binary_does_not_break_on_0_sized_inputs() {
    let mut root = hierarchy(&json!({"children": [{"value": 0}, {"value": 0}, {"value": 1}]}));
    root.sum(default_value);
    let t = treemap().tile(Box::new(binary));
    t.treemap(&mut root);
    let a: Vec<[f64; 4]> = root
        .leaves(root.root)
        .into_iter()
        .map(|i| {
            let n = &root.nodes[i];
            [n.x0, n.x1, n.y0, n.y1]
        })
        .collect();
    assert_eq!(
        a,
        [[0.0, 1.0, 0.0, 0.0], [1.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 1.0]]
    );
}

// ------------------------------------------------------- resquarify-test.js

#[test]
fn treemap_resquarify_produces_a_stable_update() {
    let tile = resquarify();
    let mut root = bare(0, 20.0, &[10.0, 10.0]);
    tile.tile(&mut root, 0, 0.0, 0.0, 20.0, 10.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 20.0, 10.0]]
    );
    tile.tile(&mut root, 0, 0.0, 0.0, 10.0, 20.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 5.0, 20.0], [5.0, 0.0, 10.0, 20.0]]
    );
}

#[test]
fn treemap_resquarify_ratio_observes_the_specified_ratio() {
    let tile = resquarify().ratio(1.0);
    let mut root = bare(0, 24.0, &SEVEN);
    tile.tile(&mut root, 0, 0.0, 0.0, 6.0, 4.0);
    assert_eq!(
        rounded_children(&root, 0),
        [
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 4.20, 4.00],
            [4.20, 2.33, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_resquarify_ratio_is_stable_if_the_ratio_is_unchanged() {
    let tile = resquarify();
    let mut root = bare(0, 20.0, &[10.0, 10.0]);
    tile.tile(&mut root, 0, 0.0, 0.0, 20.0, 10.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 20.0, 10.0]]
    );
    tile.ratio(phi()).tile(&mut root, 0, 0.0, 0.0, 10.0, 20.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 5.0, 20.0], [5.0, 0.0, 10.0, 20.0]]
    );
}

#[test]
fn treemap_resquarify_ratio_is_unstable_if_the_ratio_is_changed() {
    let tile = resquarify();
    let mut root = bare(0, 20.0, &[10.0, 10.0]);
    tile.tile(&mut root, 0, 0.0, 0.0, 20.0, 10.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [10.0, 0.0, 20.0, 10.0]]
    );
    tile.ratio(1.0).tile(&mut root, 0, 0.0, 0.0, 10.0, 20.0);
    assert_eq!(
        rounded_children(&root, 0),
        [[0.0, 0.0, 10.0, 10.0], [0.0, 10.0, 10.0, 20.0]]
    );
}

#[test]
fn treemap_resquarify_does_not_break_on_0_sized_inputs() {
    let mut root = hierarchy(&json!({"children": [{"children": [{"value": 0}]}, {"value": 1}]}));
    let rq = resquarify();
    let rq2 = rq.clone();
    let t = treemap().tile(Box::new(move |tr, p, x0, y0, x1, y1| {
        rq2.tile(tr, p, x0, y0, x1, y1)
    }));
    root.sum(default_value);
    t.treemap(&mut root);
    // `d.sum` is undefined in the JS: `+undefined || 0` is 0.
    root.sum(|d| d.get("sum").and_then(|v| v.as_f64()).unwrap_or(f64::NAN));
    t.treemap(&mut root);
    let a: Vec<[f64; 4]> = root
        .leaves(root.root)
        .into_iter()
        .map(|i| {
            let n = &root.nodes[i];
            [n.x0, n.x1, n.y0, n.y1]
        })
        .collect();
    assert_eq!(a, [[0.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]]);
}

// ------------------------------------------------------------ index-test.js

#[test]
fn treemap_has_the_expected_defaults() {
    let t = treemap();
    // `assert.strictEqual(t.tile(), treemapSquarify)` compares function
    // identity, which has no Rust equivalent; the default tiling's behaviour is
    // covered by treemap_size_observes_the_specified_size below.
    assert_eq!(t.get_size(), [1.0, 1.0]);
    assert_eq!(t.get_round(), false);
}

#[test]
fn treemap_round_observes_the_specified_rounding() {
    let t = treemap().size([600.0, 400.0]).round(true);
    let mut root = sum_sort_desc(&simple2());
    t.treemap(&mut root);
    assert_eq!(t.get_round(), true);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.0, 0.0, 600.0, 400.0],
            [0.0, 0.0, 300.0, 200.0],
            [0.0, 200.0, 300.0, 400.0],
            [300.0, 0.0, 471.0, 233.0],
            [471.0, 0.0, 600.0, 233.0],
            [300.0, 233.0, 540.0, 317.0],
            [300.0, 317.0, 540.0, 400.0],
            [540.0, 233.0, 600.0, 400.0],
        ]
    );
}

#[test]
fn treemap_padding_sets_the_inner_and_outer_padding_to_the_specified_value() {
    let t = treemap().padding(42.0);
    let n = Node::new(json!({}));
    assert_eq!(t.get_padding_inner()(&n), 42.0);
    assert_eq!(t.get_padding_top()(&n), 42.0);
    assert_eq!(t.get_padding_right()(&n), 42.0);
    assert_eq!(t.get_padding_bottom()(&n), 42.0);
    assert_eq!(t.get_padding_left()(&n), 42.0);
}

#[test]
fn treemap_padding_inner_observes_the_specified_padding() {
    let t = treemap().size([6.0, 4.0]).padding_inner(0.5);
    let mut root = sum_sort_desc(&simple2());
    t.treemap(&mut root);
    assert_eq!(t.get_padding_inner()(&Node::new(json!({}))), 0.5);
    assert_eq!(t.get_size(), [6.0, 4.0]);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.00, 0.00, 6.00, 4.00],
            [0.00, 0.00, 2.75, 1.75],
            [0.00, 2.25, 2.75, 4.00],
            [3.25, 0.00, 4.61, 2.13],
            [5.11, 0.00, 6.00, 2.13],
            [3.25, 2.63, 5.35, 3.06],
            [3.25, 3.56, 5.35, 4.00],
            [5.85, 2.63, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_padding_outer_observes_the_specified_padding() {
    let t = treemap().size([6.0, 4.0]).padding_outer(0.5);
    let mut root = sum_sort_desc(&simple2());
    t.treemap(&mut root);
    let n = Node::new(json!({}));
    assert_eq!(t.get_padding_top()(&n), 0.5);
    assert_eq!(t.get_padding_right()(&n), 0.5);
    assert_eq!(t.get_padding_bottom()(&n), 0.5);
    assert_eq!(t.get_padding_left()(&n), 0.5);
    assert_eq!(t.get_size(), [6.0, 4.0]);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.00, 0.00, 6.00, 4.00],
            [0.50, 0.50, 3.00, 2.00],
            [0.50, 2.00, 3.00, 3.50],
            [3.00, 0.50, 4.43, 2.25],
            [4.43, 0.50, 5.50, 2.25],
            [3.00, 2.25, 5.00, 2.88],
            [3.00, 2.88, 5.00, 3.50],
            [5.00, 2.25, 5.50, 3.50],
        ]
    );
}

#[test]
fn treemap_size_observes_the_specified_size() {
    let t = treemap().size([6.0, 4.0]);
    let mut root = sum_sort_desc(&simple2());
    t.treemap(&mut root);
    assert_eq!(t.get_size(), [6.0, 4.0]);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.00, 0.00, 6.00, 4.00],
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 5.40, 3.17],
            [3.00, 3.17, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_tile_observes_the_specified_tile_function() {
    let t = treemap().size([6.0, 4.0]).tile(Box::new(slice));
    let mut root = sum_sort_desc(&simple2());
    t.treemap(&mut root);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.00, 0.00, 6.00, 4.00],
            [0.00, 0.00, 6.00, 1.00],
            [0.00, 1.00, 6.00, 2.00],
            [0.00, 2.00, 6.00, 2.67],
            [0.00, 2.67, 6.00, 3.17],
            [0.00, 3.17, 6.00, 3.50],
            [0.00, 3.50, 6.00, 3.83],
            [0.00, 3.83, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_data_observes_the_specified_values() {
    let t = treemap().size([6.0, 4.0]);
    let mut root = hierarchy(&data("simple3.json"));
    root.sum(|d| d.get("foo").and_then(|v| v.as_f64()).unwrap_or(f64::NAN));
    root.sort(|a, b| b.value.partial_cmp(&a.value).unwrap());
    t.treemap(&mut root);
    assert_eq!(t.get_size(), [6.0, 4.0]);
    assert_eq!(
        rounded_descendants(&root),
        [
            [0.00, 0.00, 6.00, 4.00],
            [0.00, 0.00, 3.00, 2.00],
            [0.00, 2.00, 3.00, 4.00],
            [3.00, 0.00, 4.71, 2.33],
            [4.71, 0.00, 6.00, 2.33],
            [3.00, 2.33, 5.40, 3.17],
            [3.00, 3.17, 5.40, 4.00],
            [5.40, 2.33, 6.00, 4.00],
        ]
    );
}

#[test]
fn treemap_data_observes_the_specified_sibling_order() {
    let t = treemap();
    let mut root = hierarchy(&simple2());
    root.sum(default_value);
    root.sort(|a, b| a.value.partial_cmp(&b.value).unwrap());
    t.treemap(&mut root);
    let values: Vec<f64> = root
        .descendants(root.root)
        .into_iter()
        .map(|i| root.nodes[i].value.unwrap())
        .collect();
    assert_eq!(values, [24.0, 1.0, 2.0, 2.0, 3.0, 4.0, 6.0, 6.0]);
}

// Skipped tests (see the report):
//
// index-test.js "treemap.round(round) coerces the specified round to boolean"
//   — JS `!!x` coercion of a string; `round` takes a `bool` here.
// index-test.js "treemap.size(size) coerces the specified size to numbers"
//   — JS `+x[0]` coercion of a string / valueOf object; `size` takes `[f64; 2]`.
// index-test.js "treemap.size(size) makes defensive copies"
//   — `[f64; 2]` is copied by value, so there is no aliasing to defend against.
// flare-test.js (both cases)
//   — needs `d3-dsv` CSV parsing plus `stratify` (another module), and the
//     expected files are in the D3 3.x x/y/dx/dy shape.
