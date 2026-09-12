//! `d3.tree()`, ported from d3-hierarchy 3.1.2 `src/tree.js` — the
//! Reingold-Tilford "tidy" layout via Buchheim et al.
//!
//! The JS builds a parallel `TreeNode` tree (`treeRoot`) of mutable objects
//! whose `_` points at the real hierarchy node. Here that auxiliary tree is its
//! own arena local to this module (`Vec<TreeNode>` with `usize`/`Option<usize>`
//! links), including the synthetic parent node `treeRoot` gives the root. The
//! pre/post-order walks over it are the same stack algorithms as
//! `Tree::order_before`/`order_after`.

use crate::node::Tree;

/// `defaultSeparation`: `a.parent === b.parent ? 1 : 2`.
pub fn default_separation(t: &Tree, a: usize, b: usize) -> f64 {
    if t.nodes[a].parent == t.nodes[b].parent {
        1.0
    } else {
        2.0
    }
}

/// One node of the auxiliary tree (`function TreeNode(node, i)`).
struct TreeNode {
    /// `this._`: the real hierarchy node, or `None` for the synthetic parent.
    underlying: Option<usize>,
    parent: Option<usize>,
    children: Option<Vec<usize>>,
    /// `A`: default ancestor.
    big_a: Option<usize>,
    /// `a`: ancestor (initially itself).
    a: usize,
    /// `z`: prelim.
    z: f64,
    /// `m`: mod.
    m: f64,
    /// `c`: change.
    c: f64,
    /// `s`: shift.
    s: f64,
    /// `t`: thread.
    t: Option<usize>,
    /// `i`: index among its siblings.
    i: usize,
}

impl TreeNode {
    fn new(self_index: usize, underlying: Option<usize>, i: usize) -> Self {
        TreeNode {
            underlying,
            parent: None,
            children: None,
            big_a: None,
            a: self_index,
            z: 0.0,
            m: 0.0,
            c: 0.0,
            s: 0.0,
            t: None,
            i,
        }
    }
}

struct Aux {
    nodes: Vec<TreeNode>,
    /// index of the auxiliary root (always 0)
    root: usize,
}

/// `treeRoot(root)`.
fn tree_root(t: &Tree) -> Aux {
    let mut aux = Aux {
        nodes: vec![TreeNode::new(0, Some(t.root), 0)],
        root: 0,
    };
    let mut stack = vec![0usize];
    while let Some(node) = stack.pop() {
        let under = aux.nodes[node].underlying.unwrap();
        let children = t.nodes[under].children.clone();
        if let Some(children) = children {
            let n = children.len();
            let mut idx = vec![usize::MAX; n];
            // JS: `for (i = n - 1; i >= 0; --i)` — children created and pushed
            // in reverse order, so the pop order below matches the JS.
            for i in (0..n).rev() {
                let new = aux.nodes.len();
                let mut child = TreeNode::new(new, Some(children[i]), i);
                child.parent = Some(node);
                aux.nodes.push(child);
                idx[i] = new;
                stack.push(new);
            }
            aux.nodes[node].children = Some(idx);
        }
    }
    // `(tree.parent = new TreeNode(null, 0)).children = [tree]`
    let synthetic = aux.nodes.len();
    let mut s = TreeNode::new(synthetic, None, 0);
    s.children = Some(vec![aux.root]);
    aux.nodes.push(s);
    aux.nodes[aux.root].parent = Some(synthetic);
    aux
}

impl Aux {
    fn children(&self, i: usize) -> &[usize] {
        match &self.nodes[i].children {
            Some(c) => c,
            None => &[],
        }
    }

    /// `eachBefore` over the auxiliary tree.
    fn order_before(&self, start: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            out.push(node);
            for &c in self.children(node).iter().rev() {
                stack.push(c);
            }
        }
        out
    }

    /// `eachAfter` over the auxiliary tree.
    fn order_after(&self, start: usize) -> Vec<usize> {
        let mut stack = vec![start];
        let mut next = Vec::new();
        while let Some(node) = stack.pop() {
            next.push(node);
            for &c in self.children(node) {
                stack.push(c);
            }
        }
        next.reverse();
        next
    }

    /// `nextLeft(v)`
    fn next_left(&self, v: usize) -> Option<usize> {
        match &self.nodes[v].children {
            Some(c) => Some(c[0]),
            None => self.nodes[v].t,
        }
    }

    /// `nextRight(v)`
    fn next_right(&self, v: usize) -> Option<usize> {
        match &self.nodes[v].children {
            Some(c) => Some(c[c.len() - 1]),
            None => self.nodes[v].t,
        }
    }

    /// `nextAncestor(vim, v, ancestor)`
    fn next_ancestor(&self, vim: usize, v: usize, ancestor: usize) -> usize {
        let a = self.nodes[vim].a;
        if self.nodes[a].parent == self.nodes[v].parent {
            a
        } else {
            ancestor
        }
    }

    /// `moveSubtree(wm, wp, shift)`, mutations applied in the JS order.
    fn move_subtree(&mut self, wm: usize, wp: usize, shift: f64) {
        let change = shift / (self.nodes[wp].i as f64 - self.nodes[wm].i as f64);
        self.nodes[wp].c -= change;
        self.nodes[wp].s += shift;
        self.nodes[wm].c += change;
        self.nodes[wp].z += shift;
        self.nodes[wp].m += shift;
    }

    /// `executeShifts(v)`
    fn execute_shifts(&mut self, v: usize) {
        let mut shift = 0.0;
        let mut change = 0.0;
        let children = self.nodes[v].children.clone().unwrap();
        let mut i = children.len();
        while i > 0 {
            i -= 1;
            let w = children[i];
            self.nodes[w].z += shift;
            self.nodes[w].m += shift;
            change += self.nodes[w].c;
            shift += self.nodes[w].s + change;
        }
    }
}

/// `d3.tree()`.
pub struct Tidy {
    pub separation: Box<dyn Fn(&Tree, usize, usize) -> f64>,
    pub dx: f64,
    pub dy: f64,
    /// `nodeSize` in the JS is `null` / `false` / `true`; only truthiness is
    /// read inside `tree`, so a bool is enough.
    pub node_size: bool,
}

impl Default for Tidy {
    fn default() -> Self {
        Tidy {
            separation: Box::new(default_separation),
            dx: 1.0,
            dy: 1.0,
            node_size: false,
        }
    }
}

/// `d3.tree()`
pub fn tree() -> Tidy {
    Tidy::default()
}

impl Tidy {
    pub fn new() -> Self {
        Tidy::default()
    }

    /// `tree.separation(x)`
    pub fn separation(mut self, x: impl Fn(&Tree, usize, usize) -> f64 + 'static) -> Self {
        self.separation = Box::new(x);
        self
    }

    /// `tree.size([dx, dy])`
    pub fn size(mut self, x: [f64; 2]) -> Self {
        self.node_size = false;
        self.dx = x[0];
        self.dy = x[1];
        self
    }

    /// `tree.nodeSize([dx, dy])`
    pub fn node_size(mut self, x: [f64; 2]) -> Self {
        self.node_size = true;
        self.dx = x[0];
        self.dy = x[1];
        self
    }

    /// `tree.size()`: `null` when a node size is set.
    pub fn get_size(&self) -> Option<[f64; 2]> {
        if self.node_size {
            None
        } else {
            Some([self.dx, self.dy])
        }
    }

    /// `tree.nodeSize()`: `null` unless a node size is set.
    pub fn get_node_size(&self) -> Option<[f64; 2]> {
        if self.node_size {
            Some([self.dx, self.dy])
        } else {
            None
        }
    }

    /// `tree(root)`: writes `x` and `y` on every node of `root`.
    pub fn tree(&self, root: &mut Tree) {
        let mut t = tree_root(root);

        // Compute the layout using Buchheim et al.'s algorithm.
        for v in t.order_after(t.root) {
            self.first_walk(root, &mut t, v);
        }
        let synthetic = t.nodes[t.root].parent.unwrap();
        t.nodes[synthetic].m = -t.nodes[t.root].z;
        for v in t.order_before(t.root) {
            // secondWalk
            let pm = t.nodes[t.nodes[v].parent.unwrap()].m;
            let under = t.nodes[v].underlying.unwrap();
            root.nodes[under].x = t.nodes[v].z + pm;
            t.nodes[v].m += pm;
        }

        let order = root.order_before(root.root);

        if self.node_size {
            // sizeNode
            for &i in &order {
                root.nodes[i].x *= self.dx;
                root.nodes[i].y = root.nodes[i].depth as f64 * self.dy;
            }
        } else {
            let mut left = root.root;
            let mut right = root.root;
            let mut bottom = root.root;
            for &i in &order {
                if root.nodes[i].x < root.nodes[left].x {
                    left = i;
                }
                if root.nodes[i].x > root.nodes[right].x {
                    right = i;
                }
                if root.nodes[i].depth > root.nodes[bottom].depth {
                    bottom = i;
                }
            }
            let s = if left == right {
                1.0
            } else {
                (self.separation)(root, left, right) / 2.0
            };
            let tx = s - root.nodes[left].x;
            let kx = self.dx / (root.nodes[right].x + s + tx);
            // `bottom.depth || 1`
            let depth = root.nodes[bottom].depth;
            let ky = self.dy / if depth == 0 { 1.0 } else { depth as f64 };
            for &i in &order {
                root.nodes[i].x = (root.nodes[i].x + tx) * kx;
                root.nodes[i].y = root.nodes[i].depth as f64 * ky;
            }
        }
    }

    /// `firstWalk(v)`
    fn first_walk(&self, root: &Tree, t: &mut Aux, v: usize) {
        let parent = t.nodes[v].parent.unwrap();
        let siblings = t.nodes[parent].children.clone().unwrap();
        // `v.i ? siblings[v.i - 1] : null` — index 0 is falsy.
        let w = if t.nodes[v].i != 0 {
            Some(siblings[t.nodes[v].i - 1])
        } else {
            None
        };
        let has_children = t.nodes[v].children.is_some();
        if has_children {
            t.execute_shifts(v);
            let children = t.nodes[v].children.clone().unwrap();
            let midpoint = (t.nodes[children[0]].z + t.nodes[children[children.len() - 1]].z) / 2.0;
            if let Some(w) = w {
                let sep = (self.separation)(
                    root,
                    t.nodes[v].underlying.unwrap(),
                    t.nodes[w].underlying.unwrap(),
                );
                t.nodes[v].z = t.nodes[w].z + sep;
                t.nodes[v].m = t.nodes[v].z - midpoint;
            } else {
                t.nodes[v].z = midpoint;
            }
        } else if let Some(w) = w {
            let sep = (self.separation)(
                root,
                t.nodes[v].underlying.unwrap(),
                t.nodes[w].underlying.unwrap(),
            );
            t.nodes[v].z = t.nodes[w].z + sep;
        }
        // `v.parent.A || siblings[0]`: `A` is only ever falsy when null, and
        // `siblings[0]` is always a node (index 0 of the arena is not falsy in
        // the JS sense — it is an object there).
        let ancestor = t.nodes[parent].big_a.unwrap_or(siblings[0]);
        let a = self.apportion(root, t, v, w, ancestor);
        t.nodes[parent].big_a = Some(a);
    }

    /// `apportion(v, w, ancestor)`
    fn apportion(
        &self,
        root: &Tree,
        t: &mut Aux,
        v: usize,
        w: Option<usize>,
        ancestor: usize,
    ) -> usize {
        let mut ancestor = ancestor;
        if let Some(w) = w {
            let mut vip = v;
            let mut vop = v;
            let mut vim = w;
            let mut vom = t.nodes[t.nodes[vip].parent.unwrap()].children.as_ref().unwrap()[0];
            let mut sip = t.nodes[vip].m;
            let mut sop = t.nodes[vop].m;
            let mut sim = t.nodes[vim].m;
            let mut som = t.nodes[vom].m;
            // `while (vim = nextRight(vim), vip = nextLeft(vip), vim && vip)`
            let mut cur_vim = t.next_right(vim);
            let mut cur_vip = t.next_left(vip);
            while let (Some(nvim), Some(nvip)) = (cur_vim, cur_vip) {
                vim = nvim;
                vip = nvip;
                vom = t.next_left(vom).unwrap();
                vop = t.next_right(vop).unwrap();
                t.nodes[vop].a = v;
                let sep = (self.separation)(
                    root,
                    t.nodes[vim].underlying.unwrap(),
                    t.nodes[vip].underlying.unwrap(),
                );
                let shift = t.nodes[vim].z + sim - t.nodes[vip].z - sip + sep;
                if shift > 0.0 {
                    let wm = t.next_ancestor(vim, v, ancestor);
                    t.move_subtree(wm, v, shift);
                    sip += shift;
                    sop += shift;
                }
                sim += t.nodes[vim].m;
                sip += t.nodes[vip].m;
                som += t.nodes[vom].m;
                sop += t.nodes[vop].m;

                cur_vim = t.next_right(vim);
                cur_vip = t.next_left(vip);
            }
            // After the loop, `vim`/`vip` hold the latest assignment from the
            // comma expression — which may be null.
            if cur_vim.is_some() && t.next_right(vop).is_none() {
                t.nodes[vop].t = cur_vim;
                t.nodes[vop].m += sim - sop;
            }
            if cur_vip.is_some() && t.next_left(vom).is_none() {
                t.nodes[vom].t = cur_vip;
                t.nodes[vom].m += sip - som;
                ancestor = v;
            }
        }
        ancestor
    }
}
