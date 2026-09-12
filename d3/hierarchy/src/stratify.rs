//! `stratify()`, ported from d3-hierarchy 3.1.2 `src/stratify.js`.
//!
//! The JS builds `Node` objects in place in the input array and uses sentinel
//! objects (`preroot`, `ambiguous`, `imputed`) for identity tricks. Here the
//! nodes live in a `Tree` arena in input order (so arena index == input index,
//! with imputed nodes appended), `ambiguous` is `None` in the key map, and
//! `imputed` is a parallel `Vec<bool>`. The `preroot = {depth: -1}` trick is
//! kept as an explicit "parent depth is -1 for the root" in the depth pass.

use std::collections::{HashMap, HashSet};

use crate::node::{Datum, Node, Tree};

/// The errors `stratify` throws; `Display` reproduces d3's messages verbatim,
/// because d3's tests assert on them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StratifyError {
    Missing(String),
    Ambiguous(String),
    MultipleRoots,
    NoRoot,
    Cycle,
}

impl std::fmt::Display for StratifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StratifyError::Missing(id) => write!(f, "missing: {}", id),
            StratifyError::Ambiguous(id) => write!(f, "ambiguous: {}", id),
            StratifyError::MultipleRoots => write!(f, "multiple roots"),
            StratifyError::NoRoot => write!(f, "no root"),
            StratifyError::Cycle => write!(f, "cycle"),
        }
    }
}

impl std::error::Error for StratifyError {}

/// d3's `(nodeId = accessor(d, i, data)) != null && (nodeId += "")`: the value
/// is coerced to a string, and null/undefined (absent) yields nothing. The
/// empty-string case is handled by the caller, as in the JS.
fn coerce(v: Option<&Datum>) -> Option<String> {
    match v {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(other) => Some(other.to_string()),
    }
}

/// `defaultId`: `d.id`.
pub fn default_id(d: &Datum, _i: usize) -> Option<String> {
    coerce(d.get("id"))
}

/// `defaultParentId`: `d.parentId`.
pub fn default_parent_id(d: &Datum, _i: usize) -> Option<String> {
    coerce(d.get("parentId"))
}

type IdFn = Box<dyn Fn(&Datum, usize) -> Option<String>>;
type PathFn = Box<dyn Fn(&Datum, usize) -> String>;

/// `d3.stratify()`. `None` accessors mean d3's defaults.
#[derive(Default)]
pub struct Stratify {
    pub id: Option<IdFn>,
    pub parent_id: Option<IdFn>,
    pub path: Option<PathFn>,
}

impl Stratify {
    pub fn new() -> Self {
        Stratify {
            id: None,
            parent_id: None,
            path: None,
        }
    }

    /// `stratify.id(x)`
    pub fn with_id(mut self, f: impl Fn(&Datum, usize) -> Option<String> + 'static) -> Self {
        self.id = Some(Box::new(f));
        self
    }

    /// `stratify.parentId(x)`
    pub fn with_parent_id(mut self, f: impl Fn(&Datum, usize) -> Option<String> + 'static) -> Self {
        self.parent_id = Some(Box::new(f));
        self
    }

    /// `stratify.path(x)`
    pub fn with_path(mut self, f: impl Fn(&Datum, usize) -> String + 'static) -> Self {
        self.path = Some(Box::new(f));
        self
    }

    /// `stratify(data)`
    pub fn stratify(&self, data: &[Datum]) -> Result<Tree, StratifyError> {
        let id_fn: Box<dyn Fn(&Datum, usize) -> Option<String> + '_> = match &self.id {
            Some(f) => Box::new(move |d, i| f(d, i)),
            None => Box::new(default_id),
        };
        let parent_id_fn: Box<dyn Fn(&Datum, usize) -> Option<String> + '_> = match &self.parent_id {
            Some(f) => Box::new(move |d, i| f(d, i)),
            None => Box::new(default_parent_id),
        };

        // Data per node, plus the `imputed` marker as a parallel flag.
        let mut datums: Vec<Datum> = data.to_vec();
        let mut imputed: Vec<bool> = vec![false; datums.len()];

        // Ids and parent ids, as the two "current" accessors produce them.
        let mut ids: Vec<Option<String>>;
        let mut parent_ids: Vec<Option<String>>;

        if let Some(path) = &self.path {
            let mut i_list: Vec<String> = data
                .iter()
                .enumerate()
                .map(|(i, d)| normalize(&path(d, i)))
                .collect();
            let mut p_list: Vec<String> = i_list.iter().map(|p| parentof(p)).collect();
            let mut s: HashSet<String> = i_list.iter().cloned().collect();
            s.insert(String::new());
            // `for (const i of P)` also visits the entries pushed inside the loop.
            let mut k = 0;
            while k < p_list.len() {
                let i = p_list[k].clone();
                if !s.contains(&i) {
                    s.insert(i.clone());
                    let pi = parentof(&i);
                    i_list.push(i);
                    p_list.push(pi);
                    datums.push(serde_json::Value::Null);
                    imputed.push(true);
                }
                k += 1;
            }
            ids = i_list.into_iter().map(Some).collect();
            parent_ids = p_list.into_iter().map(Some).collect();
        } else {
            ids = Vec::with_capacity(datums.len());
            parent_ids = Vec::with_capacity(datums.len());
            for (i, d) in data.iter().enumerate() {
                ids.push(id_fn(d, i));
                parent_ids.push(parent_id_fn(d, i));
            }
        }

        // `nodeId != null && (nodeId += "")`: the empty string is ignored too.
        for v in ids.iter_mut().chain(parent_ids.iter_mut()) {
            if v.as_deref() == Some("") {
                *v = None;
            }
        }

        let mut n = datums.len();
        let mut nodes: Vec<Node> = Vec::with_capacity(n);
        // `ambiguous` is represented by `None`.
        let mut node_by_key: HashMap<String, Option<usize>> = HashMap::new();

        for i in 0..n {
            let mut node = Node::new(datums[i].clone());
            if let Some(node_id) = &ids[i] {
                node.id = Some(node_id.clone());
                match node_by_key.entry(node_id.clone()) {
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        e.insert(None);
                    }
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(Some(i));
                    }
                }
            }
            nodes.push(node);
        }

        let mut root: Option<usize> = None;
        for i in 0..n {
            match &parent_ids[i] {
                Some(node_id) => {
                    let parent = match node_by_key.get(node_id) {
                        None => return Err(StratifyError::Missing(node_id.clone())),
                        Some(None) => return Err(StratifyError::Ambiguous(node_id.clone())),
                        Some(Some(p)) => *p,
                    };
                    match &mut nodes[parent].children {
                        Some(c) => c.push(i),
                        none => *none = Some(vec![i]),
                    }
                    nodes[i].parent = Some(parent);
                }
                None => {
                    if root.is_some() {
                        return Err(StratifyError::MultipleRoots);
                    }
                    root = Some(i);
                }
            }
        }

        let mut root = match root {
            Some(r) => r,
            None => return Err(StratifyError::NoRoot),
        };

        // When imputing internal nodes, only introduce roots if needed.
        // Then replace the imputed marker data with null.
        if self.path.is_some() {
            while imputed[root] && nodes[root].children().len() == 1 {
                root = nodes[root].children()[0];
                n -= 1;
            }
            for i in (0..nodes.len()).rev() {
                if !imputed[i] {
                    break;
                }
                nodes[i].data = serde_json::Value::Null;
            }
        }

        nodes[root].parent = None;
        let mut tree = Tree { nodes, root };

        // `root.parent = preroot` (depth -1), then eachBefore depth, then
        // eachBefore computeHeight.
        for i in tree.order_before(root) {
            let depth = match tree.nodes[i].parent {
                Some(p) => tree.nodes[p].depth + 1,
                None => 0, // preroot.depth + 1
            };
            tree.nodes[i].depth = depth;
            n -= 1;
        }
        tree.compute_height();

        if n > 0 {
            return Err(StratifyError::Cycle);
        }

        Ok(tree)
    }
}

/// `d3.stratify()(data)` with the default accessors.
pub fn stratify(data: &[Datum]) -> Result<Tree, StratifyError> {
    Stratify::new().stratify(data)
}

// To normalize a path, we coerce to a string, strip the trailing slash if any
// (as long as the trailing slash is not immediately preceded by another slash),
// and add leading slash if missing.
fn normalize(path: &str) -> String {
    let b = path.as_bytes();
    let i = b.len() as isize;
    let p = if slash(b, i - 1) && !slash(b, i - 2) {
        &path[..path.len() - 1]
    } else {
        path
    };
    if p.as_bytes().first() == Some(&b'/') {
        p.to_string()
    } else {
        format!("/{}", p)
    }
}

// Walk backwards to find the first slash that is not the leading slash, e.g.:
// "/foo/bar" ⇥ "/foo", "/foo" ⇥ "/", "/" ↦ "". (The root is special-cased
// because the id of the root must be a truthy value.)
fn parentof(path: &str) -> String {
    let b = path.as_bytes();
    let mut i = b.len();
    if i < 2 {
        return String::new();
    }
    loop {
        i -= 1;
        if i <= 1 {
            break;
        }
        if slash(b, i as isize) {
            break;
        }
    }
    path[..i].to_string()
}

// Slashes can be escaped; to determine whether a slash is a path delimiter, we
// count the number of preceding backslashes escaping the forward slash: an odd
// number indicates an escaped forward slash.
fn slash(path: &[u8], i: isize) -> bool {
    if i < 0 || i as usize >= path.len() {
        return false;
    }
    let mut i = i as usize;
    if path[i] == b'/' {
        let mut k = 0;
        while i > 0 {
            i -= 1;
            if path[i] == b'\\' {
                k += 1;
            } else {
                break;
            }
        }
        if k & 1 == 0 {
            return true;
        }
    }
    false
}
