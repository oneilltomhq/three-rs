//! Tests for `src/partition.rs`. d3-hierarchy 3.1.2 ships no
//! test/partition-test.js, so the expectations below were generated from the
//! vendor JS itself, e.g.:
//!
//! ```text
//! node --input-type=module -e 'import {hierarchy, partition} from "/home/tom/src/vendor/d3-hierarchy/src/index.js"; import fs from "fs"; const root = hierarchy(JSON.parse(fs.readFileSync(common::data("simple.json")))).sum(d => d.value); partition().size([100,200]).padding(1).round(true)(root); console.log(JSON.stringify(root.descendants().map(d => [d.depth,d.height,d.value,d.x0,d.y0,d.x1,d.y1])));'
//! ```
//!
//! Each row is `[depth, height, value, x0, y0, x1, y1]` in `descendants()`
//! (breadth-first) order, and is compared for exact f64 equality.

mod common;
use d3_hierarchy::hierarchy;
use d3_hierarchy::partition::partition;
use serde_json::json;


fn data(name: &str) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(common::data(name)).unwrap()).unwrap()
}

fn value(d: &serde_json::Value) -> f64 {
    d.get("value").and_then(|v| v.as_f64()).unwrap_or(f64::NAN)
}

fn rows(t: &d3_hierarchy::Tree) -> Vec<[f64; 7]> {
    t.descendants(t.root)
        .into_iter()
        .map(|i| {
            let n = &t.nodes[i];
            [
                n.depth as f64,
                n.height as f64,
                n.value.unwrap(),
                n.x0,
                n.y0,
                n.x1,
                n.y1,
            ]
        })
        .collect()
}

#[test]
fn partition_has_the_expected_defaults() {
    let p = partition();
    assert_eq!(p.get_size(), [1.0, 1.0]);
    assert_eq!(p.get_padding(), 0.0);
    assert_eq!(p.get_round(), false);
}

#[test]
fn partition_with_the_default_size() {
    let mut root = hierarchy(&data("simple.json"));
    root.sum(value);
    partition().partition(&mut root);
    assert_eq!(
        rows(&root),
        [
            [0.0, 2.0, 15.0, 0.0, 0.0, 1.0, 0.3333333333333333],
            [1.0, 1.0, 6.0, 0.0, 0.3333333333333333, 0.4, 0.6666666666666666],
            [1.0, 1.0, 9.0, 0.4, 0.3333333333333333, 1.0, 0.6666666666666666],
            [2.0, 0.0, 1.0, 0.0, 0.6666666666666666, 0.06666666666666667, 1.0],
            [2.0, 0.0, 2.0, 0.06666666666666667, 0.6666666666666666, 0.2, 1.0],
            [2.0, 0.0, 3.0, 0.2, 0.6666666666666666, 0.4, 1.0],
            [2.0, 0.0, 4.0, 0.4, 0.6666666666666666, 0.6666666666666667, 1.0],
            [2.0, 0.0, 3.0, 0.6666666666666667, 0.6666666666666666, 0.8666666666666667, 1.0],
            [2.0, 0.0, 2.0, 0.8666666666666667, 0.6666666666666666, 1.0, 1.0],
        ]
    );
}

#[test]
fn partition_size_observes_the_specified_size() {
    let mut root = hierarchy(&data("simple.json"));
    root.sum(value);
    let p = partition().size([100.0, 200.0]);
    p.partition(&mut root);
    assert_eq!(p.get_size(), [100.0, 200.0]);
    assert_eq!(
        rows(&root),
        [
            [0.0, 2.0, 15.0, 0.0, 0.0, 100.0, 66.66666666666667],
            [1.0, 1.0, 6.0, 0.0, 66.66666666666667, 40.0, 133.33333333333334],
            [1.0, 1.0, 9.0, 40.0, 66.66666666666667, 100.0, 133.33333333333334],
            [2.0, 0.0, 1.0, 0.0, 133.33333333333334, 6.666666666666667, 200.0],
            [2.0, 0.0, 2.0, 6.666666666666667, 133.33333333333334, 20.0, 200.0],
            [2.0, 0.0, 3.0, 20.0, 133.33333333333334, 40.0, 200.0],
            [2.0, 0.0, 4.0, 40.0, 133.33333333333334, 66.66666666666667, 200.0],
            [2.0, 0.0, 3.0, 66.66666666666667, 133.33333333333334, 86.66666666666667, 200.0],
            [2.0, 0.0, 2.0, 86.66666666666667, 133.33333333333334, 100.0, 200.0],
        ]
    );
}

#[test]
fn partition_size_observes_the_specified_size_on_a_flat_hierarchy() {
    let mut root = hierarchy(&data("simple2.json"));
    root.sum(value);
    partition().size([6.0, 4.0]).partition(&mut root);
    assert_eq!(
        rows(&root),
        [
            [0.0, 1.0, 24.0, 0.0, 0.0, 6.0, 2.0],
            [1.0, 0.0, 6.0, 0.0, 2.0, 1.5, 4.0],
            [1.0, 0.0, 6.0, 1.5, 2.0, 3.0, 4.0],
            [1.0, 0.0, 4.0, 3.0, 2.0, 4.0, 4.0],
            [1.0, 0.0, 3.0, 4.0, 2.0, 4.75, 4.0],
            [1.0, 0.0, 2.0, 4.75, 2.0, 5.25, 4.0],
            [1.0, 0.0, 2.0, 5.25, 2.0, 5.75, 4.0],
            [1.0, 0.0, 1.0, 5.75, 2.0, 6.0, 4.0],
        ]
    );
}

#[test]
fn partition_padding_observes_the_specified_padding() {
    let mut root = hierarchy(&data("simple.json"));
    root.sum(value);
    let p = partition().size([100.0, 200.0]).padding(1.0);
    p.partition(&mut root);
    assert_eq!(p.get_padding(), 1.0);
    assert_eq!(
        rows(&root),
        [
            [0.0, 2.0, 15.0, 1.0, 1.0, 99.0, 65.66666666666667],
            [1.0, 1.0, 6.0, 1.0, 66.66666666666667, 39.599999999999994, 132.33333333333334],
            [1.0, 1.0, 9.0, 40.599999999999994, 66.66666666666667, 99.0, 132.33333333333334],
            [2.0, 0.0, 1.0, 1.0, 133.33333333333334, 6.599999999999999, 199.0],
            [2.0, 0.0, 2.0, 7.599999999999999, 133.33333333333334, 19.799999999999997, 199.0],
            [2.0, 0.0, 3.0, 20.799999999999997, 133.33333333333334, 39.599999999999994, 199.0],
            [2.0, 0.0, 4.0, 40.599999999999994, 133.33333333333334, 66.0, 199.0],
            [2.0, 0.0, 3.0, 67.0, 133.33333333333334, 85.8, 199.0],
            [2.0, 0.0, 2.0, 86.8, 133.33333333333334, 99.0, 199.0],
        ]
    );
}

#[test]
fn partition_round_observes_the_specified_rounding() {
    let mut root = hierarchy(&data("simple.json"));
    root.sum(value);
    let p = partition().size([100.0, 200.0]).padding(1.0).round(true);
    p.partition(&mut root);
    assert_eq!(p.get_round(), true);
    assert_eq!(
        rows(&root),
        [
            [0.0, 2.0, 15.0, 1.0, 1.0, 99.0, 66.0],
            [1.0, 1.0, 6.0, 1.0, 67.0, 40.0, 132.0],
            [1.0, 1.0, 9.0, 41.0, 67.0, 99.0, 132.0],
            [2.0, 0.0, 1.0, 1.0, 133.0, 7.0, 199.0],
            [2.0, 0.0, 2.0, 8.0, 133.0, 20.0, 199.0],
            [2.0, 0.0, 3.0, 21.0, 133.0, 40.0, 199.0],
            [2.0, 0.0, 4.0, 41.0, 133.0, 66.0, 199.0],
            [2.0, 0.0, 3.0, 67.0, 133.0, 86.0, 199.0],
            [2.0, 0.0, 2.0, 87.0, 133.0, 99.0, 199.0],
        ]
    );
}

#[test]
fn partition_positions_a_single_node_hierarchy() {
    let mut root = hierarchy(&json!({"value": 42}));
    root.sum(value);
    partition().size([100.0, 200.0]).partition(&mut root);
    assert_eq!(rows(&root), [[0.0, 0.0, 42.0, 0.0, 0.0, 100.0, 200.0]]);
}

#[test]
fn partition_positions_a_single_node_hierarchy_with_padding() {
    let mut root = hierarchy(&json!({"value": 42}));
    root.sum(value);
    partition()
        .size([100.0, 200.0])
        .padding(3.0)
        .partition(&mut root);
    assert_eq!(rows(&root), [[0.0, 0.0, 42.0, 3.0, 3.0, 97.0, 197.0]]);
}
