//! Port of d3-hierarchy/test/stratify-test.js.
//!
//! d3 compares whole trees with `deepStrictEqual(noparent(root), {...})`, where
//! `noparent` drops `parent` and keeps whatever own properties the node has:
//! `data`, `depth`, `height`, plus `id` and `children` when present (`value` is
//! never set by stratify). `dump` below reproduces exactly that shape as JSON,
//! so each expectation can be written as the JS object literal was.
//!
//! Skipped tests (JS-only behaviour) are listed at the bottom of this file.

use d3_hierarchy::stratify::{default_id, default_parent_id, Stratify};
use d3_hierarchy::Tree;
use serde_json::{json, Value};

fn dump(t: &Tree, i: usize) -> Value {
    let n = &t.nodes[i];
    let mut o = serde_json::Map::new();
    if let Some(id) = &n.id {
        o.insert("id".into(), json!(id));
    }
    o.insert("depth".into(), json!(n.depth));
    o.insert("height".into(), json!(n.height));
    if let Some(v) = n.value {
        o.insert("value".into(), json!(v));
    }
    o.insert("data".into(), n.data.clone());
    if let Some(children) = &n.children {
        o.insert(
            "children".into(),
            Value::Array(children.iter().map(|&c| dump(t, c)).collect()),
        );
    }
    Value::Object(o)
}

fn root_dump(t: &Tree) -> Value {
    dump(t, t.root)
}

fn path_of(d: &Value, _i: usize) -> String {
    d["path"].as_str().unwrap().to_string()
}

#[test]
fn stratify_has_the_expected_defaults() {
    // The JS asserts `s.id()({id: "foo"}) === "foo"`; the getter overloading is
    // JS-only, so this checks the default accessors themselves.
    assert_eq!(default_id(&json!({"id": "foo"}), 0), Some("foo".to_string()));
    assert_eq!(
        default_parent_id(&json!({"parentId": "bar"}), 0),
        Some("bar".to_string())
    );
}

#[test]
fn stratify_data_returns_the_root_node() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "a"}),
            json!({"id": "aa", "parentId": "a"}),
            json!({"id": "ab", "parentId": "a"}),
            json!({"id": "aaa", "parentId": "aa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"id": "a"},
            "children": [
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"id": "aa", "parentId": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"id": "aaa", "parentId": "aa"}}
                    ]
                },
                {"id": "ab", "depth": 1, "height": 0, "data": {"id": "ab", "parentId": "a"}}
            ]
        })
    );
}

#[test]
fn stratify_data_does_not_require_the_data_to_be_in_topological_order() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "aaa", "parentId": "aa"}),
            json!({"id": "aa", "parentId": "a"}),
            json!({"id": "ab", "parentId": "a"}),
            json!({"id": "a"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"id": "a"},
            "children": [
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"id": "aa", "parentId": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"id": "aaa", "parentId": "aa"}}
                    ]
                },
                {"id": "ab", "depth": 1, "height": 0, "data": {"id": "ab", "parentId": "a"}}
            ]
        })
    );
}

#[test]
fn stratify_data_preserves_the_input_order_of_siblings() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "aaa", "parentId": "aa"}),
            json!({"id": "ab", "parentId": "a"}),
            json!({"id": "aa", "parentId": "a"}),
            json!({"id": "a"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"id": "a"},
            "children": [
                {"id": "ab", "depth": 1, "height": 0, "data": {"id": "ab", "parentId": "a"}},
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"id": "aa", "parentId": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"id": "aaa", "parentId": "aa"}}
                    ]
                }
            ]
        })
    );
}

// SKIPPED: "stratify(data) accepts an iterable" — see the list at the bottom.

#[test]
fn stratify_data_treats_an_empty_parent_id_as_the_root() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "a", "parentId": ""}),
            json!({"id": "aa", "parentId": "a"}),
            json!({"id": "ab", "parentId": "a"}),
            json!({"id": "aaa", "parentId": "aa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"id": "a", "parentId": ""},
            "children": [
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"id": "aa", "parentId": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"id": "aaa", "parentId": "aa"}}
                    ]
                },
                {"id": "ab", "depth": 1, "height": 0, "data": {"id": "ab", "parentId": "a"}}
            ]
        })
    );
}

#[test]
fn stratify_data_does_not_treat_a_falsy_but_non_empty_parent_id_as_the_root() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": 0, "parentId": null}),
            json!({"id": 1, "parentId": 0}),
            json!({"id": 2, "parentId": 0}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "0",
            "depth": 0,
            "height": 1,
            "data": {"id": 0, "parentId": null},
            "children": [
                {"id": "1", "depth": 1, "height": 0, "data": {"id": 1, "parentId": 0}},
                {"id": "2", "depth": 1, "height": 0, "data": {"id": 2, "parentId": 0}}
            ]
        })
    );
}

#[test]
fn stratify_data_throws_an_error_if_the_data_does_not_have_a_single_root() {
    let s = Stratify::new();
    assert_eq!(
        s.stratify(&[json!({"id": "a"}), json!({"id": "b"})])
            .unwrap_err()
            .to_string(),
        "multiple roots"
    );
    assert_eq!(
        s.stratify(&[json!({"id": "a", "parentId": "a"})])
            .unwrap_err()
            .to_string(),
        "no root"
    );
    assert_eq!(
        s.stratify(&[
            json!({"id": "a", "parentId": "b"}),
            json!({"id": "b", "parentId": "a"})
        ])
        .unwrap_err()
        .to_string(),
        "no root"
    );
}

#[test]
fn stratify_data_throws_an_error_if_the_hierarchy_is_cyclical() {
    let s = Stratify::new();
    assert_eq!(
        s.stratify(&[json!({"id": "root"}), json!({"id": "a", "parentId": "a"})])
            .unwrap_err()
            .to_string(),
        "cycle"
    );
    assert_eq!(
        s.stratify(&[
            json!({"id": "root"}),
            json!({"id": "a", "parentId": "b"}),
            json!({"id": "b", "parentId": "a"})
        ])
        .unwrap_err()
        .to_string(),
        "cycle"
    );
}

#[test]
fn stratify_data_throws_an_error_if_multiple_parents_have_the_same_id() {
    let s = Stratify::new();
    assert_eq!(
        s.stratify(&[
            json!({"id": "a"}),
            json!({"id": "b", "parentId": "a"}),
            json!({"id": "b", "parentId": "a"}),
            json!({"id": "c", "parentId": "b"})
        ])
        .unwrap_err()
        .to_string(),
        "ambiguous: b"
    );
}

#[test]
fn stratify_data_throws_an_error_if_the_specified_parent_is_not_found() {
    let s = Stratify::new();
    assert_eq!(
        s.stratify(&[json!({"id": "a"}), json!({"id": "b", "parentId": "c"})])
            .unwrap_err()
            .to_string(),
        "missing: c"
    );
}

#[test]
fn stratify_data_allows_the_id_to_be_undefined_for_leaf_nodes() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "a"}),
            json!({"parentId": "a"}),
            json!({"parentId": "a"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 1,
            "data": {"id": "a"},
            "children": [
                {"depth": 1, "height": 0, "data": {"parentId": "a"}},
                {"depth": 1, "height": 0, "data": {"parentId": "a"}}
            ]
        })
    );
}

#[test]
fn stratify_data_allows_the_id_to_be_non_unique_for_leaf_nodes() {
    let s = Stratify::new();
    let root = s
        .stratify(&[
            json!({"id": "a", "parentId": null}),
            json!({"id": "b", "parentId": "a"}),
            json!({"id": "b", "parentId": "a"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 1,
            "data": {"id": "a", "parentId": null},
            "children": [
                {"id": "b", "depth": 1, "height": 0, "data": {"id": "b", "parentId": "a"}},
                {"id": "b", "depth": 1, "height": 0, "data": {"id": "b", "parentId": "a"}}
            ]
        })
    );
}

#[test]
fn stratify_data_coerces_the_id_to_a_string_if_not_null_and_not_empty() {
    let s = Stratify::new();
    // The `{id: {toString() {...}}}` and `{id: undefined}` cases are skipped;
    // see the list at the bottom.
    assert_eq!(s.stratify(&[json!({"id": ""})]).unwrap().root().id, None);
    assert_eq!(s.stratify(&[json!({"id": null})]).unwrap().root().id, None);
    assert_eq!(s.stratify(&[json!({})]).unwrap().root().id, None);
    // The coercion itself, which the toString case exercises in JS:
    assert_eq!(
        s.stratify(&[json!({"id": 42})]).unwrap().root().id,
        Some("42".to_string())
    );
}

// SKIPPED: the second "stratify(data) allows the id to be undefined for leaf
// nodes" (the `parentId: {toString()}` one) — see the list at the bottom.

#[test]
fn stratify_id_id_observes_the_specified_id_function() {
    let s = Stratify::new().with_id(|d, _| {
        d.get("foo").and_then(|v| v.as_str()).map(|s| s.to_string())
    });
    let root = s
        .stratify(&[
            json!({"foo": "a"}),
            json!({"foo": "aa", "parentId": "a"}),
            json!({"foo": "ab", "parentId": "a"}),
            json!({"foo": "aaa", "parentId": "aa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"foo": "a"},
            "children": [
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"foo": "aa", "parentId": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"foo": "aaa", "parentId": "aa"}}
                    ]
                },
                {"id": "ab", "depth": 1, "height": 0, "data": {"foo": "ab", "parentId": "a"}}
            ]
        })
    );
}

// SKIPPED: "stratify.id(id) tests that id is a function".

#[test]
fn stratify_parent_id_id_observes_the_specified_parent_id_function() {
    let s = Stratify::new().with_parent_id(|d, _| {
        d.get("foo").and_then(|v| v.as_str()).map(|s| s.to_string())
    });
    let root = s
        .stratify(&[
            json!({"id": "a"}),
            json!({"id": "aa", "foo": "a"}),
            json!({"id": "ab", "foo": "a"}),
            json!({"id": "aaa", "foo": "aa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "a",
            "depth": 0,
            "height": 2,
            "data": {"id": "a"},
            "children": [
                {
                    "id": "aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"id": "aa", "foo": "a"},
                    "children": [
                        {"id": "aaa", "depth": 2, "height": 0, "data": {"id": "aaa", "foo": "aa"}}
                    ]
                },
                {"id": "ab", "depth": 1, "height": 0, "data": {"id": "ab", "foo": "a"}}
            ]
        })
    );
}

// SKIPPED: "stratify.parentId(id) tests that id is a function".

#[test]
fn stratify_path_path_returns_the_root_node() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/"}),
            json!({"path": "/aa"}),
            json!({"path": "/ab"}),
            json!({"path": "/aa/aaa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": "/"},
            "children": [
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/aa"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa"}}
                    ]
                },
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab"}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_correctly_handles_single_character_folders() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/"}),
            json!({"path": "/d"}),
            json!({"path": "/d/123"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": "/"},
            "children": [
                {
                    "id": "/d",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/d"},
                    "children": [
                        {"id": "/d/123", "depth": 2, "height": 0, "data": {"path": "/d/123"}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_correctly_handles_empty_folders() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/"}),
            json!({"path": "//"}),
            json!({"path": "///"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": "/"},
            "children": [
                {
                    "id": "//",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "//"},
                    "children": [
                        {"id": "///", "depth": 2, "height": 0, "data": {"path": "///"}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_correctly_handles_single_character_folders_with_trailing_slashes() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/"}),
            json!({"path": "/d/"}),
            json!({"path": "/d/123/"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": "/"},
            "children": [
                {
                    "id": "/d",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/d/"},
                    "children": [
                        {"id": "/d/123", "depth": 2, "height": 0, "data": {"path": "/d/123/"}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_correctly_handles_imputed_single_character_folders() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[json!({"path": "/"}), json!({"path": "/d/123"})])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": "/"},
            "children": [
                {
                    "id": "/d",
                    "depth": 1,
                    "height": 1,
                    "data": null,
                    "children": [
                        {"id": "/d/123", "depth": 2, "height": 0, "data": {"path": "/d/123"}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_allows_slashes_to_be_escaped() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/"}),
            json!({"path": "/aa"}),
            json!({"path": "\\/ab"}),
            json!({"path": "/aa\\/aaa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 1,
            "data": {"path": "/"},
            "children": [
                {"id": "/aa", "depth": 1, "height": 0, "data": {"path": "/aa"}},
                {"id": "/\\/ab", "depth": 1, "height": 0, "data": {"path": "\\/ab"}},
                {"id": "/aa\\/aaa", "depth": 1, "height": 0, "data": {"path": "/aa\\/aaa"}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_imputes_internal_nodes() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[json!({"path": "/aa/aaa"}), json!({"path": "/ab"})])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": null,
            "children": [
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab"}},
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": null,
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa"}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_allows_duplicate_leaf_paths() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/aa/aaa", "number": 1}),
            json!({"path": "/aa/aaa", "number": 2}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/aa",
            "depth": 0,
            "height": 1,
            "data": null,
            "children": [
                {"id": "/aa/aaa", "depth": 1, "height": 0, "data": {"path": "/aa/aaa", "number": 1}},
                {"id": "/aa/aaa", "depth": 1, "height": 0, "data": {"path": "/aa/aaa", "number": 2}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_does_not_allow_duplicate_internal_paths() {
    let err = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/aa"}),
            json!({"path": "/aa"}),
            json!({"path": "/aa/aaa"}),
            json!({"path": "/aa/aaa"}),
        ])
        .unwrap_err()
        .to_string();
    assert!(err.contains("ambiguous"), "{}", err);
}

#[test]
fn stratify_path_path_implicitly_adds_leading_slashes() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": ""}),
            json!({"path": "aa"}),
            json!({"path": "ab"}),
            json!({"path": "aa/aaa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": {"path": ""},
            "children": [
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "aa"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "aa/aaa"}}
                    ]
                },
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "ab"}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_implicitly_trims_trailing_slashes() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/aa/"}),
            json!({"path": "/ab/"}),
            json!({"path": "/aa/aaa/"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": null,
            "children": [
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/aa/"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa/"}}
                    ]
                },
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab/"}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_does_not_trim_trailing_slashes_preceded_by_a_slash() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[json!({"path": "/aa//"}), json!({"path": "/b"})])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 3,
            "data": null,
            "children": [
                {"id": "/b", "depth": 1, "height": 0, "data": {"path": "/b"}},
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 2,
                    "data": null,
                    "children": [
                        {
                            "id": "/aa/",
                            "depth": 2,
                            "height": 1,
                            "data": null,
                            "children": [
                                {"id": "/aa//", "depth": 3, "height": 0, "data": {"path": "/aa//"}}
                            ]
                        }
                    ]
                }
            ]
        })
    );
}

#[test]
fn stratify_path_path_does_not_require_the_data_to_be_in_topological_order() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/aa/aaa"}),
            json!({"path": "/aa"}),
            json!({"path": "/ab"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": null,
            "children": [
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/aa"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa"}}
                    ]
                },
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab"}}
            ]
        })
    );
}

#[test]
fn stratify_path_path_preserves_the_input_order_of_siblings() {
    let root = Stratify::new()
        .with_path(path_of)
        .stratify(&[
            json!({"path": "/ab"}),
            json!({"path": "/aa"}),
            json!({"path": "/aa/aaa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": null,
            "children": [
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab"}},
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/aa"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa"}}
                    ]
                }
            ]
        })
    );
}

// SKIPPED: "stratify.path(path) accepts an iterable".

#[test]
fn stratify_path_path_coerces_paths_to_strings() {
    // The JS wraps each path in a `class Path { toString() }` — but note it
    // passes that mapping as a *second* argument to stratify(data), which
    // stratify ignores, so the test is the array case with a coercing path
    // accessor. Here the accessor does the coercion explicitly.
    let root = Stratify::new()
        .with_path(|d, _| format!("{}", d["path"].as_str().unwrap()))
        .stratify(&[
            json!({"path": "/ab"}),
            json!({"path": "/aa"}),
            json!({"path": "/aa/aaa"}),
        ])
        .unwrap();
    assert_eq!(
        root_dump(&root),
        json!({
            "id": "/",
            "depth": 0,
            "height": 2,
            "data": null,
            "children": [
                {"id": "/ab", "depth": 1, "height": 0, "data": {"path": "/ab"}},
                {
                    "id": "/aa",
                    "depth": 1,
                    "height": 1,
                    "data": {"path": "/aa"},
                    "children": [
                        {"id": "/aa/aaa", "depth": 2, "height": 0, "data": {"path": "/aa/aaa"}}
                    ]
                }
            ]
        })
    );
}

// ---------------------------------------------------------------------------
// Skipped tests, with reasons:
//
// * "stratify(data) accepts an iterable" and "stratify.path(path) accepts an
//   iterable": the input is a JS `Set`. The Rust API takes `&[Datum]`, so these
//   are byte-for-byte the preceding array tests ("preserves the input order of
//   siblings").
// * the second "stratify(data) allows the id to be undefined for leaf nodes"
//   (line 318 of the JS): the datum is `{parentId: {toString() {...}}}`, an
//   object with a custom `toString`, which `serde_json::Value` cannot express.
// * "stratify.id(id) tests that id is a function" and "stratify.parentId(id)
//   tests that id is a function": `optional(x)` throwing on a non-function is
//   JS-only; the Rust accessors are typed.
// * parts of "stratify(data) coerces the id to a string, if not null and not
//   empty": the `{id: {toString() {...}}}` case (no Value equivalent) and the
//   `{id: undefined}` case (JSON has no undefined — it is the same as absent,
//   which is covered by the `{}` case). The empty/null/absent cases and the
//   numeric coercion are ported above.
// * the `assert(root instanceof hierarchy)` and `assert.strictEqual(s.id(), foo)`
//   assertions inside ported tests: type identity and accessor getters are
//   JS-only.
// ---------------------------------------------------------------------------
