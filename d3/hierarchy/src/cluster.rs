//! `d3.cluster()`, ported from d3-hierarchy 3.1.2 `src/cluster.js`.
//!
//! The JS separation accessor takes two nodes; here it takes the arena plus the
//! two node indices, since a node alone cannot answer `a.parent === b.parent`.

use crate::node::Tree;

/// `defaultSeparation`: `a.parent === b.parent ? 1 : 2`.
pub fn default_separation(tree: &Tree, a: usize, b: usize) -> f64 {
    if tree.nodes[a].parent == tree.nodes[b].parent {
        1.0
    } else {
        2.0
    }
}

/// `meanX(children)`: `children.reduce((x, c) => x + c.x, 0) / children.length`.
fn mean_x(tree: &Tree, children: &[usize]) -> f64 {
    let mut x = 0.0;
    for &c in children {
        x += tree.nodes[c].x;
    }
    x / children.len() as f64
}

/// `maxY(children)`: `1 + children.reduce((y, c) => Math.max(y, c.y), 0)`.
fn max_y(tree: &Tree, children: &[usize]) -> f64 {
    let mut y = 0.0f64;
    for &c in children {
        y = y.max(tree.nodes[c].y);
    }
    1.0 + y
}

/// `leafLeft`: descend through first children.
///
/// The JS loop is `while (children = node.children)`; an *empty* children array
/// is truthy there and `children[0]` is `undefined`, so the JS throws. Here an
/// empty `Some(vec![])` simply ends the descent.
fn leaf_left(tree: &Tree, mut node: usize) -> usize {
    loop {
        let ch = tree.nodes[node].children();
        if ch.is_empty() {
            return node;
        }
        node = ch[0];
    }
}

/// `leafRight`: descend through last children.
fn leaf_right(tree: &Tree, mut node: usize) -> usize {
    loop {
        let ch = tree.nodes[node].children();
        if ch.is_empty() {
            return node;
        }
        node = ch[ch.len() - 1];
    }
}

/// `d3.cluster()`.
pub struct Cluster {
    pub separation: Box<dyn Fn(&Tree, usize, usize) -> f64>,
    pub dx: f64,
    pub dy: f64,
    pub node_size: bool,
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster {
            separation: Box::new(default_separation),
            dx: 1.0,
            dy: 1.0,
            node_size: false,
        }
    }
}

impl Cluster {
    pub fn new() -> Self {
        Self::default()
    }

    /// `cluster.separation(x)`.
    pub fn separation(mut self, f: impl Fn(&Tree, usize, usize) -> f64 + 'static) -> Self {
        self.separation = Box::new(f);
        self
    }

    /// `cluster.size([dx, dy])`.
    pub fn size(mut self, size: [f64; 2]) -> Self {
        self.node_size = false;
        self.dx = size[0];
        self.dy = size[1];
        self
    }

    /// `cluster.nodeSize([dx, dy])`.
    pub fn node_size(mut self, size: [f64; 2]) -> Self {
        self.node_size = true;
        self.dx = size[0];
        self.dy = size[1];
        self
    }

    /// `cluster(root)`: writes `x` and `y` on every node.
    pub fn cluster(&self, root: &mut Tree) {
        let order = root.order_after(root.root);
        let r = root.root;

        // First walk, computing the initial x & y values.
        let mut previous_node: Option<usize> = None;
        let mut x = 0.0f64;
        for &node in &order {
            let children = root.nodes[node].children.clone();
            match children {
                Some(children) if !children.is_empty() => {
                    let mx = mean_x(root, &children);
                    let my = max_y(root, &children);
                    root.nodes[node].x = mx;
                    root.nodes[node].y = my;
                }
                // `if (children)`: an empty children array is truthy in JS, but
                // meanX/maxY of an empty array give NaN/1 there; this arm keeps
                // the leaf path, which is what a childless node gets.
                _ => {
                    root.nodes[node].x = match previous_node {
                        Some(p) => {
                            x += (self.separation)(root, node, p);
                            x
                        }
                        None => 0.0,
                    };
                    root.nodes[node].y = 0.0;
                    previous_node = Some(node);
                }
            }
        }

        let left = leaf_left(root, r);
        let right = leaf_right(root, r);
        let x0 = root.nodes[left].x - (self.separation)(root, left, right) / 2.0;
        let x1 = root.nodes[right].x + (self.separation)(root, right, left) / 2.0;

        // Second walk, normalizing x & y to the desired size.
        let (root_x, root_y) = (root.nodes[r].x, root.nodes[r].y);
        for &node in &order {
            if self.node_size {
                root.nodes[node].x = (root.nodes[node].x - root_x) * self.dx;
                root.nodes[node].y = (root_y - root.nodes[node].y) * self.dy;
            } else {
                root.nodes[node].x = (root.nodes[node].x - x0) / (x1 - x0) * self.dx;
                // `1 - (root.y ? node.y / root.y : 1)`: root.y is 0 (falsy) for
                // a single-node hierarchy.
                let t = if root_y != 0.0 && !root_y.is_nan() {
                    root.nodes[node].y / root_y
                } else {
                    1.0
                };
                root.nodes[node].y = (1.0 - t) * self.dy;
            }
        }
    }
}
