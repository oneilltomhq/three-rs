//! `hierarchy()` and the Node methods, ported from d3-hierarchy 3.1.2
//! `src/hierarchy/*.js`.
//!
//! The JS uses mutable node objects with parent pointers; here the tree is an
//! arena (`Tree { nodes: Vec<Node> }`) and parent/children are `usize` indices
//! into it. See `README.md`.

pub type Datum = serde_json::Value;

/// One node of the hierarchy. Layout fields (`x`, `y`, `x0`..`y1`, `r`) are
/// dynamic properties on the JS node; here they are always present and default
/// to 0, so `Option` only covers `value`, which d3 itself leaves undefined
/// until `sum`/`count` runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub data: Datum,
    pub depth: usize,
    pub height: usize,
    pub parent: Option<usize>,
    pub children: Option<Vec<usize>>,
    pub value: Option<f64>,
    /// stratify only
    pub id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub r: f64,
}

impl Node {
    pub fn new(data: Datum) -> Self {
        Node {
            data,
            depth: 0,
            height: 0,
            parent: None,
            children: None,
            value: None,
            id: None,
            x: 0.0,
            y: 0.0,
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
            r: 0.0,
        }
    }

    pub fn children(&self) -> &[usize] {
        match &self.children {
            Some(c) => c,
            None => &[],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: usize,
}

/// `objectChildren`: `d.children`, an array or nothing. d3 also accepts any
/// iterable (`Array.from`); with `serde_json::Value` data the only iterable is
/// an array, and anything else (null, a number) yields no children, which is
/// what d3's "ignores non-iterable children" test asserts.
pub fn object_children(d: &Datum) -> Option<Vec<Datum>> {
    match d.get("children") {
        Some(serde_json::Value::Array(a)) => Some(a.clone()),
        _ => None,
    }
}

/// `d3.hierarchy(data)`.
pub fn hierarchy(data: &Datum) -> Tree {
    hierarchy_with(data, &object_children)
}

/// `d3.hierarchy(data, children)`.
pub fn hierarchy_with(data: &Datum, children: &dyn Fn(&Datum) -> Option<Vec<Datum>>) -> Tree {
    let mut t = Tree {
        nodes: vec![Node::new(data.clone())],
        root: 0,
    };
    let mut stack = vec![0usize];
    while let Some(node) = stack.pop() {
        let childs = children(&t.nodes[node].data);
        if let Some(childs) = childs {
            if !childs.is_empty() {
                let depth = t.nodes[node].depth + 1;
                let mut idx = Vec::with_capacity(childs.len());
                for c in childs {
                    let mut n = Node::new(c);
                    n.parent = Some(node);
                    n.depth = depth;
                    t.nodes.push(n);
                    idx.push(t.nodes.len() - 1);
                }
                // d3 pushes children onto the stack in reverse so that the
                // first child is popped first.
                for &i in idx.iter().rev() {
                    stack.push(i);
                }
                t.nodes[node].children = Some(idx);
            }
        }
    }
    t.compute_height();
    t
}

impl Tree {
    pub fn root(&self) -> &Node {
        &self.nodes[self.root]
    }

    pub fn node(&self, i: usize) -> &Node {
        &self.nodes[i]
    }

    /// `computeHeight` applied in eachBefore order, as `hierarchy()` does.
    pub fn compute_height(&mut self) {
        for i in self.order_before(self.root) {
            let mut height = 0usize;
            let mut cur = i;
            loop {
                self.nodes[cur].height = height;
                match self.nodes[cur].parent {
                    Some(p) => {
                        height += 1;
                        if self.nodes[p].height < height {
                            cur = p;
                        } else {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    // ---- traversal orders (the exact visit orders of d3's each* methods) ----

    /// `node.each()` / the iterator: breadth-first.
    pub fn order_each(&self, start: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut next = vec![start];
        while !next.is_empty() {
            let mut current = next;
            current.reverse();
            next = Vec::new();
            while let Some(node) = current.pop() {
                out.push(node);
                for &c in self.nodes[node].children() {
                    next.push(c);
                }
            }
        }
        out
    }

    /// `node.eachBefore()`: pre-order.
    pub fn order_before(&self, start: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            out.push(node);
            let ch = self.nodes[node].children();
            for &c in ch.iter().rev() {
                stack.push(c);
            }
        }
        out
    }

    /// `node.eachAfter()`: post-order.
    pub fn order_after(&self, start: usize) -> Vec<usize> {
        let mut stack = vec![start];
        let mut next = Vec::new();
        while let Some(node) = stack.pop() {
            next.push(node);
            for &c in self.nodes[node].children() {
                stack.push(c);
            }
        }
        next.reverse();
        next
    }

    pub fn each(&self, start: usize, mut f: impl FnMut(&Node, usize)) {
        for (i, n) in self.order_each(start).into_iter().enumerate() {
            f(&self.nodes[n], i);
        }
    }

    pub fn each_before(&self, start: usize, mut f: impl FnMut(&Node, usize)) {
        for (i, n) in self.order_before(start).into_iter().enumerate() {
            f(&self.nodes[n], i);
        }
    }

    pub fn each_after(&self, start: usize, mut f: impl FnMut(&Node, usize)) {
        for (i, n) in self.order_after(start).into_iter().enumerate() {
            f(&self.nodes[n], i);
        }
    }

    /// `node.find(callback)`: the callback gets (node, index, root).
    pub fn find(&self, start: usize, mut f: impl FnMut(&Node, usize, &Node) -> bool) -> Option<usize> {
        let root = &self.nodes[start];
        for (i, n) in self.order_each(start).into_iter().enumerate() {
            if f(&self.nodes[n], i, root) {
                return Some(n);
            }
        }
        None
    }

    // ---- values ----

    /// `node.count()`
    pub fn count(&mut self) -> &mut Self {
        self.count_from(self.root)
    }

    pub fn count_from(&mut self, start: usize) -> &mut Self {
        for i in self.order_after(start) {
            let mut sum = 0.0;
            let n = self.nodes[i].children.clone();
            match n {
                Some(children) if !children.is_empty() => {
                    for &c in children.iter().rev() {
                        sum += self.nodes[c].value.unwrap_or(f64::NAN);
                    }
                }
                _ => sum = 1.0,
            }
            self.nodes[i].value = Some(sum);
        }
        self
    }

    /// `node.sum(value)`; `value` is applied to `node.data` and coerced with
    /// `+v || 0` as in the JS.
    pub fn sum(&mut self, value: impl Fn(&Datum) -> f64) -> &mut Self {
        self.sum_from(self.root, value)
    }

    pub fn sum_from(&mut self, start: usize, value: impl Fn(&Datum) -> f64) -> &mut Self {
        for i in self.order_after(start) {
            // `+value(node.data) || 0`: only NaN differs from the raw number.
            let mut sum = value(&self.nodes[i].data);
            if sum.is_nan() {
                sum = 0.0;
            }
            let children = self.nodes[i].children.clone();
            if let Some(children) = children {
                for &c in children.iter().rev() {
                    sum += self.nodes[c].value.unwrap_or(f64::NAN);
                }
            }
            self.nodes[i].value = Some(sum);
        }
        self
    }

    /// `node.sort(compare)`; the comparator sees two nodes.
    pub fn sort(&mut self, compare: impl Fn(&Node, &Node) -> std::cmp::Ordering) -> &mut Self {
        for i in self.order_before(self.root) {
            if let Some(mut children) = self.nodes[i].children.take() {
                // `Array.prototype.sort` is stable, as is `sort_by`.
                children.sort_by(|&a, &b| compare(&self.nodes[a], &self.nodes[b]));
                self.nodes[i].children = Some(children);
            }
        }
        self
    }

    // ---- structure ----

    /// `node.ancestors()`
    pub fn ancestors(&self, start: usize) -> Vec<usize> {
        let mut out = vec![start];
        let mut node = start;
        while let Some(p) = self.nodes[node].parent {
            out.push(p);
            node = p;
        }
        out
    }

    /// `node.descendants()`
    pub fn descendants(&self, start: usize) -> Vec<usize> {
        self.order_each(start)
    }

    /// `node.leaves()`
    pub fn leaves(&self, start: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for i in self.order_before(start) {
            if self.nodes[i].children.is_none() {
                out.push(i);
            }
        }
        out
    }

    /// `node.links()`
    pub fn links(&self, start: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in self.order_each(start) {
            if i != start {
                out.push((self.nodes[i].parent.unwrap(), i));
            }
        }
        out
    }

    /// `node.path(end)`
    pub fn path(&self, start: usize, end: usize) -> Vec<usize> {
        let ancestor = self.least_common_ancestor(start, end);
        let mut nodes = vec![start];
        let mut s = start;
        while Some(s) != ancestor {
            s = self.nodes[s].parent.unwrap();
            nodes.push(s);
        }
        let k = nodes.len();
        let mut e = end;
        while Some(e) != ancestor {
            nodes.insert(k, e);
            e = self.nodes[e].parent.unwrap();
        }
        nodes
    }

    fn least_common_ancestor(&self, a: usize, b: usize) -> Option<usize> {
        if a == b {
            return Some(a);
        }
        let mut a_nodes = self.ancestors(a);
        let mut b_nodes = self.ancestors(b);
        let mut c = None;
        let mut x = a_nodes.pop();
        let mut y = b_nodes.pop();
        while x == y && x.is_some() {
            c = x;
            x = a_nodes.pop();
            y = b_nodes.pop();
        }
        c
    }

    /// `node.copy()`: a new tree rooted at `start`, carrying `value`.
    pub fn copy(&self, start: usize) -> Tree {
        let mut t = Tree {
            nodes: Vec::new(),
            root: 0,
        };
        // Walk in the same order `hierarchy()` would, so indices and children
        // order match a fresh build.
        let mut src_of = vec![start];
        let mut n = Node::new(self.nodes[start].data.clone());
        n.value = self.nodes[start].value;
        t.nodes.push(n);
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let src = src_of[i];
            let children = self.nodes[src].children.clone();
            if let Some(children) = children {
                if !children.is_empty() {
                    let depth = t.nodes[i].depth + 1;
                    let mut idx = Vec::with_capacity(children.len());
                    for &c in &children {
                        let mut n = Node::new(self.nodes[c].data.clone());
                        n.value = self.nodes[c].value;
                        n.parent = Some(i);
                        n.depth = depth;
                        t.nodes.push(n);
                        src_of.push(c);
                        idx.push(t.nodes.len() - 1);
                    }
                    for &k in idx.iter().rev() {
                        stack.push(k);
                    }
                    t.nodes[i].children = Some(idx);
                }
            }
        }
        t.compute_height();
        t
    }
}
