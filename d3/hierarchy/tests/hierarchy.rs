//! Ports of d3-hierarchy/test/hierarchy/{each,find,links,index,copy}-test.js.

use d3_hierarchy::{hierarchy, Tree};
use serde_json::json;

fn id(t: &Tree, i: usize) -> String {
    t.nodes[i].data["id"].as_str().unwrap().to_string()
}

fn tree() -> serde_json::Value {
    json!({"id": "root", "children": [{"id": "a", "children": [{"id": "ab"}]}, {"id": "b", "children": [{"id": "ba"}]}]})
}

// each-test.js
#[test]
fn node_each_traverses_a_hierarchy_in_breadth_first_order() {
    let root = hierarchy(&tree());
    let a: Vec<String> = root.order_each(root.root).iter().map(|&i| id(&root, i)).collect();
    assert_eq!(a, ["root", "a", "b", "ab", "ba"]);
}

#[test]
fn node_each_before_traverses_a_hierarchy_in_pre_order_traversal() {
    let root = hierarchy(&tree());
    let a: Vec<String> = root.order_before(root.root).iter().map(|&i| id(&root, i)).collect();
    assert_eq!(a, ["root", "a", "ab", "b", "ba"]);
}

#[test]
fn node_each_after_traverses_a_hierarchy_in_post_order_traversal() {
    let root = hierarchy(&tree());
    let a: Vec<String> = root.order_after(root.root).iter().map(|&i| id(&root, i)).collect();
    assert_eq!(a, ["ab", "a", "ba", "b", "root"]);
}

#[test]
fn a_hierarchy_is_an_iterable_equivalent_to_node_each() {
    let root = hierarchy(&tree());
    let mut a = Vec::new();
    root.each(root.root, |n, _| a.push(n.data["id"].as_str().unwrap().to_string()));
    assert_eq!(a, ["root", "a", "b", "ab", "ba"]);
}

// find-test.js
#[test]
fn node_find_finds_nodes() {
    let mut root = hierarchy(&json!({"id": "root", "children": [{"id": "a"}, {"id": "b", "children": [{"id": "ba"}]}]}));
    root.count();
    let f = root.find(root.root, |d, _, _| d.data["id"] == "b").unwrap();
    assert_eq!(id(&root, f), "b");
    let f = root.find(root.root, |_, i, _| i == 0).unwrap();
    assert_eq!(id(&root, f), "root");
    // `(d, i, e) => d !== e`: the first node that is not the root.
    let f = root.find(root.root, |d, _, e| !std::ptr::eq(d, e)).unwrap();
    assert_eq!(id(&root, f), "a");
}

// links-test.js
#[test]
fn node_links_returns_an_array_of_source_target() {
    let root = hierarchy(&json!({"id": "root", "children": [{"id": "a"}, {"id": "b", "children": [{"id": "ba"}]}]}));
    let r = root.root;
    let a = root.nodes[r].children()[0];
    let b = root.nodes[r].children()[1];
    let ba = root.nodes[b].children()[0];
    assert_eq!(root.links(r), vec![(r, a), (r, b), (b, ba)]);
}

// index-test.js — d3's first case uses a JS Set for children; serde_json has no
// Set, so the array form below is the same assertion. See README "skipped".
#[test]
fn hierarchy_supports_iterable_children() {
    let root = hierarchy(&json!({"id": "root", "children": [{"id": "a"}, {"id": "b", "children": [{"id": "ba"}]}]}));
    let r = root.root;
    let a = root.nodes[r].children()[0];
    let b = root.nodes[r].children()[1];
    let ba = root.nodes[b].children()[0];
    assert_eq!(root.links(r), vec![(r, a), (r, b), (b, ba)]);
}

#[test]
fn hierarchy_ignores_non_iterable_children() {
    let root = hierarchy(&json!({"id": "root", "children": [{"id": "a", "children": null}, {"id": "b", "children": 42}]}));
    let r = root.root;
    let a = root.nodes[r].children()[0];
    let b = root.nodes[r].children()[1];
    assert_eq!(root.links(r), vec![(r, a), (r, b)]);
}

// copy-test.js
#[test]
fn node_copy_copies_values() {
    let mut root = hierarchy(&json!({"id": "root", "children": [{"id": "a"}, {"id": "b", "children": [{"id": "ba"}]}]}));
    root.count();
    let c = root.copy(root.root);
    assert_eq!(c.root().value, Some(2.0));
}

// Methods with no upstream tape test of their own, covered here so the port is
// exercised: sum, sort, path, ancestors, descendants, leaves.
#[test]
fn sum_sort_path_ancestors_descendants_leaves() {
    let data = json!({"id": "root", "children": [
        {"id": "a", "value": 1},
        {"id": "b", "children": [{"id": "ba", "value": 2}, {"id": "bb", "value": 3}]}]});
    let mut root = hierarchy(&data);
    root.sum(|d| d["value"].as_f64().unwrap_or(f64::NAN));
    assert_eq!(root.root().value, Some(6.0));
    let r = root.root;
    let a = root.nodes[r].children()[0];
    let b = root.nodes[r].children()[1];
    let bb = root.nodes[b].children()[1];
    assert_eq!(root.nodes[b].value, Some(5.0));
    assert_eq!(root.nodes[a].value, Some(1.0));
    assert_eq!(root.ancestors(bb), vec![bb, b, r]);
    assert_eq!(root.path(a, bb), vec![a, r, b, bb]);
    assert_eq!(root.leaves(r), vec![a, root.nodes[b].children()[0], bb]);
    assert_eq!(root.descendants(r).len(), 5);
    assert_eq!(root.nodes[r].height, 2);
    assert_eq!(root.nodes[bb].depth, 2);
    // descending by value
    root.sort(|x, y| y.value.partial_cmp(&x.value).unwrap());
    assert_eq!(root.nodes[r].children(), &[b, a]);
}
