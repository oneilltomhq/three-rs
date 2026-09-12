//! The whole treemap family, ported from d3-hierarchy 3.1.2
//! `src/treemap/{index,dice,slice,sliceDice,binary,squarify,resquarify,round}.js`.
//!
//! Tiling functions take the arena plus the *index* of the parent whose
//! children are to be positioned, in place of the JS node object:
//! `fn(&mut Tree, usize, f64, f64, f64, f64)`.
//!
//! JS truthiness matters here: `parent.value && (x1 - x0) / parent.value`
//! yields `parent.value` itself when it is 0 or NaN, so `truthy()` below
//! reproduces that rather than guarding only against zero.

use crate::node::{Node, Tree};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `x && y` for the numbers d3 multiplies: 0 and NaN are falsy.
fn truthy(v: f64) -> bool {
    v != 0.0 && !v.is_nan()
}

fn value_of(t: &Tree, i: usize) -> f64 {
    t.nodes[i].value.unwrap_or(f64::NAN)
}

// ---------------------------------------------------------------- round.js

/// `roundNode`.
pub fn round_node(node: &mut Node) {
    node.x0 = js_round(node.x0);
    node.y0 = js_round(node.y0);
    node.x1 = js_round(node.x1);
    node.y1 = js_round(node.y1);
}

/// `Math.round`: ties go *up* (towards +∞), unlike Rust's `f64::round`, which
/// rounds ties away from zero.
pub fn js_round(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        return x;
    }
    (x + 0.5).floor()
}

// ----------------------------------------------------------- dice.js/slice.js

/// `treemapDice(parent, x0, y0, x1, y1)`.
pub fn dice(t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    let children = t.nodes[parent].children().to_vec();
    let value = value_of(t, parent);
    dice_children(t, &children, value, x0, y0, x1, y1);
}

/// The body of `dice`, over an explicit child list and value, so that
/// squarify's synthetic rows (which are not nodes) can use it.
fn dice_children(t: &mut Tree, nodes: &[usize], value: f64, x0: f64, y0: f64, x1: f64, y1: f64) {
    let k = if truthy(value) { (x1 - x0) / value } else { value };
    let mut x0 = x0;
    for &c in nodes {
        t.nodes[c].y0 = y0;
        t.nodes[c].y1 = y1;
        t.nodes[c].x0 = x0;
        x0 += value_of(t, c) * k;
        t.nodes[c].x1 = x0;
    }
}

/// `treemapSlice(parent, x0, y0, x1, y1)`.
pub fn slice(t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    let children = t.nodes[parent].children().to_vec();
    let value = value_of(t, parent);
    slice_children(t, &children, value, x0, y0, x1, y1);
}

fn slice_children(t: &mut Tree, nodes: &[usize], value: f64, x0: f64, y0: f64, x1: f64, y1: f64) {
    let k = if truthy(value) { (y1 - y0) / value } else { value };
    let mut y0 = y0;
    for &c in nodes {
        t.nodes[c].x0 = x0;
        t.nodes[c].x1 = x1;
        t.nodes[c].y0 = y0;
        y0 += value_of(t, c) * k;
        t.nodes[c].y1 = y0;
    }
}

/// `treemapSliceDice(parent, x0, y0, x1, y1)`.
pub fn slice_dice(t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    if t.nodes[parent].depth & 1 == 1 {
        slice(t, parent, x0, y0, x1, y1)
    } else {
        dice(t, parent, x0, y0, x1, y1)
    }
}

// -------------------------------------------------------------- binary.js

/// `treemapBinary(parent, x0, y0, x1, y1)`.
pub fn binary(t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    let nodes = t.nodes[parent].children().to_vec();
    let n = nodes.len();
    let mut sums = vec![0.0f64; n + 1];
    let mut sum = 0.0;
    for i in 0..n {
        sum += value_of(t, nodes[i]);
        sums[i + 1] = sum;
    }
    let value = value_of(t, parent);
    binary_partition(t, &nodes, &sums, 0, n, value, x0, y0, x1, y1);
}

#[allow(clippy::too_many_arguments)]
fn binary_partition(
    t: &mut Tree,
    nodes: &[usize],
    sums: &[f64],
    i: usize,
    j: usize,
    value: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) {
    if i + 1 >= j {
        let node = nodes[i];
        t.nodes[node].x0 = x0;
        t.nodes[node].y0 = y0;
        t.nodes[node].x1 = x1;
        t.nodes[node].y1 = y1;
        return;
    }

    let value_offset = sums[i];
    let value_target = (value / 2.0) + value_offset;
    let mut k = i + 1;
    let mut hi = j - 1;

    while k < hi {
        let mid = (k + hi) >> 1;
        if sums[mid] < value_target {
            k = mid + 1;
        } else {
            hi = mid;
        }
    }

    if (value_target - sums[k - 1]) < (sums[k] - value_target) && i + 1 < k {
        k -= 1;
    }

    let value_left = sums[k] - value_offset;
    let value_right = value - value_left;

    if (x1 - x0) > (y1 - y0) {
        let xk = if truthy(value) {
            (x0 * value_right + x1 * value_left) / value
        } else {
            x1
        };
        binary_partition(t, nodes, sums, i, k, value_left, x0, y0, xk, y1);
        binary_partition(t, nodes, sums, k, j, value_right, xk, y0, x1, y1);
    } else {
        let yk = if truthy(value) {
            (y0 * value_right + y1 * value_left) / value
        } else {
            y1
        };
        binary_partition(t, nodes, sums, i, k, value_left, x0, y0, x1, yk);
        binary_partition(t, nodes, sums, k, j, value_right, x0, yk, x1, y1);
    }
}

// ------------------------------------------------------------- squarify.js

/// `phi`.
pub fn phi() -> f64 {
    (1.0 + 5.0f64.sqrt()) / 2.0
}

/// One row of a squarified layout: d3's `{value, dice, children}`.
#[derive(Clone, Debug)]
pub struct Row {
    pub value: f64,
    pub dice: bool,
    pub children: Vec<usize>,
}

/// `squarifyRatio(ratio, parent, x0, y0, x1, y1)`.
pub fn squarify_ratio_rows(
    ratio: f64,
    t: &mut Tree,
    parent: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    let nodes = t.nodes[parent].children().to_vec();
    let mut x0 = x0;
    let mut y0 = y0;
    let mut i0 = 0usize;
    let mut i1 = 0usize;
    let n = nodes.len();
    let mut value = value_of(t, parent);

    while i0 < n {
        let dx = x1 - x0;
        let dy = y1 - y0;

        // Find the next non-empty node.
        let mut sum_value;
        loop {
            sum_value = value_of(t, nodes[i1]);
            i1 += 1;
            if truthy(sum_value) || i1 >= n {
                break;
            }
        }
        let mut min_value = sum_value;
        let mut max_value = sum_value;
        let alpha = (dy / dx).max(dx / dy) / (value * ratio);
        let mut beta = sum_value * sum_value * alpha;
        let mut min_ratio = (max_value / beta).max(beta / min_value);

        // Keep adding nodes while the aspect ratio maintains or improves.
        while i1 < n {
            let node_value = value_of(t, nodes[i1]);
            sum_value += node_value;
            if node_value < min_value {
                min_value = node_value;
            }
            if node_value > max_value {
                max_value = node_value;
            }
            beta = sum_value * sum_value * alpha;
            let new_ratio = (max_value / beta).max(beta / min_value);
            if new_ratio > min_ratio {
                sum_value -= node_value;
                break;
            }
            min_ratio = new_ratio;
            i1 += 1;
        }

        // Position and record the row orientation.
        let row = Row {
            value: sum_value,
            dice: dx < dy,
            children: nodes[i0..i1].to_vec(),
        };
        if row.dice {
            // JS evaluates the arguments left to right, so `y0` is passed
            // before the `y0 +=` in the last argument runs.
            let y0_old = y0;
            let y1_arg = if truthy(value) {
                y0 += dy * sum_value / value;
                y0
            } else {
                y1
            };
            dice_children(t, &row.children, row.value, x0, y0_old, x1, y1_arg);
        } else {
            let x0_old = x0;
            let x1_arg = if truthy(value) {
                x0 += dx * sum_value / value;
                x0
            } else {
                x1
            };
            slice_children(t, &row.children, row.value, x0_old, y0, x1_arg, y1);
        }
        value -= sum_value;
        i0 = i1;
        rows.push(row);
    }

    rows
}

/// `treemapSquarify`, the default tiling (ratio φ).
pub fn squarify(t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
    squarify_ratio_rows(phi(), t, parent, x0, y0, x1, y1);
}

/// `treemapSquarify.ratio(ratio)`: the ratio is clamped to at least 1, as in
/// `custom((x = +x) > 1 ? x : 1)`.
pub fn squarify_ratio(ratio: f64) -> impl Fn(&mut Tree, usize, f64, f64, f64, f64) {
    let ratio = if ratio > 1.0 { ratio } else { 1.0 };
    move |t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64| {
        squarify_ratio_rows(ratio, t, parent, x0, y0, x1, y1);
    }
}

// ----------------------------------------------------------- resquarify.js

#[derive(Clone, Debug)]
struct StoredRows {
    ratio: f64,
    rows: Vec<Row>,
}

/// `treemapResquarify`. d3 is stateful here too: it caches the rows on the
/// node itself (`parent._squarify`, with a `ratio` property), so *every*
/// resquarify instance shares the same per-node state. We keep that state in a
/// `HashMap<usize, …>` keyed by node index, shared (`Rc`) between an instance
/// and the instances derived from it with `ratio()`, which is what makes d3's
/// "stable if the ratio is unchanged" / "unstable if the ratio is changed"
/// tests come out the same way.
#[derive(Clone)]
pub struct Resquarify {
    ratio: f64,
    rows: Rc<RefCell<HashMap<usize, StoredRows>>>,
}

impl Default for Resquarify {
    fn default() -> Self {
        Self::new()
    }
}

impl Resquarify {
    pub fn new() -> Self {
        Resquarify {
            ratio: phi(),
            rows: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// `resquarify.ratio(x)`: a new tiling function over the same node state.
    pub fn ratio(&self, x: f64) -> Resquarify {
        Resquarify {
            ratio: if x > 1.0 { x } else { 1.0 },
            rows: Rc::clone(&self.rows),
        }
    }

    pub fn tile(&self, t: &mut Tree, parent: usize, x0: f64, y0: f64, x1: f64, y1: f64) {
        let cached = {
            let map = self.rows.borrow();
            match map.get(&parent) {
                Some(s) if s.ratio == self.ratio => Some(s.rows.clone()),
                _ => None,
            }
        };
        match cached {
            Some(rows) => {
                let mut x0 = x0;
                let mut y0 = y0;
                let mut value = value_of(t, parent);
                let mut updated = Vec::with_capacity(rows.len());
                for row in rows {
                    let mut row = row;
                    row.value = 0.0;
                    for &c in &row.children {
                        row.value += value_of(t, c);
                    }
                    if row.dice {
                        let y0_old = y0;
                        let y1_arg = if truthy(value) {
                            y0 += (y1 - y0) * row.value / value;
                            y0
                        } else {
                            y1
                        };
                        dice_children(t, &row.children, row.value, x0, y0_old, x1, y1_arg);
                    } else {
                        let x0_old = x0;
                        let x1_arg = if truthy(value) {
                            x0 += (x1 - x0) * row.value / value;
                            x0
                        } else {
                            x1
                        };
                        slice_children(t, &row.children, row.value, x0_old, y0, x1_arg, y1);
                    }
                    value -= row.value;
                    updated.push(row);
                }
                // d3 mutates the stored rows in place (row.value), so keep them.
                self.rows.borrow_mut().insert(
                    parent,
                    StoredRows {
                        ratio: self.ratio,
                        rows: updated,
                    },
                );
            }
            None => {
                let rows = squarify_ratio_rows(self.ratio, t, parent, x0, y0, x1, y1);
                self.rows.borrow_mut().insert(
                    parent,
                    StoredRows {
                        ratio: self.ratio,
                        rows,
                    },
                );
            }
        }
    }
}

/// `treemapResquarify`.
pub fn resquarify() -> Resquarify {
    Resquarify::new()
}

// --------------------------------------------------------------- index.js

type Tile = Box<dyn Fn(&mut Tree, usize, f64, f64, f64, f64)>;
type Pad = Box<dyn Fn(&Node) -> f64>;

fn constant_zero() -> Pad {
    Box::new(|_| 0.0)
}

fn constant(x: f64) -> Pad {
    Box::new(move |_| x)
}

/// `d3.treemap()`.
pub struct Treemap {
    tile: Tile,
    round: bool,
    dx: f64,
    dy: f64,
    padding_inner: Pad,
    padding_top: Pad,
    padding_right: Pad,
    padding_bottom: Pad,
    padding_left: Pad,
}

impl Default for Treemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Treemap {
    pub fn new() -> Self {
        Treemap {
            tile: Box::new(squarify),
            round: false,
            dx: 1.0,
            dy: 1.0,
            padding_inner: constant_zero(),
            padding_top: constant_zero(),
            padding_right: constant_zero(),
            padding_bottom: constant_zero(),
            padding_left: constant_zero(),
        }
    }

    // ---- setters (chaining as in the JS) ----

    pub fn tile(mut self, tile: Tile) -> Self {
        self.tile = tile;
        self
    }

    pub fn round(mut self, x: bool) -> Self {
        self.round = x;
        self
    }

    pub fn size(mut self, size: [f64; 2]) -> Self {
        self.dx = size[0];
        self.dy = size[1];
        self
    }

    pub fn padding(self, x: f64) -> Self {
        self.padding_inner(x).padding_outer(x)
    }

    pub fn padding_inner(mut self, x: f64) -> Self {
        self.padding_inner = constant(x);
        self
    }

    pub fn padding_inner_fn(mut self, x: Pad) -> Self {
        self.padding_inner = x;
        self
    }

    pub fn padding_outer(self, x: f64) -> Self {
        self.padding_top(x)
            .padding_right(x)
            .padding_bottom(x)
            .padding_left(x)
    }

    pub fn padding_top(mut self, x: f64) -> Self {
        self.padding_top = constant(x);
        self
    }

    pub fn padding_top_fn(mut self, x: Pad) -> Self {
        self.padding_top = x;
        self
    }

    pub fn padding_right(mut self, x: f64) -> Self {
        self.padding_right = constant(x);
        self
    }

    pub fn padding_right_fn(mut self, x: Pad) -> Self {
        self.padding_right = x;
        self
    }

    pub fn padding_bottom(mut self, x: f64) -> Self {
        self.padding_bottom = constant(x);
        self
    }

    pub fn padding_bottom_fn(mut self, x: Pad) -> Self {
        self.padding_bottom = x;
        self
    }

    pub fn padding_left(mut self, x: f64) -> Self {
        self.padding_left = constant(x);
        self
    }

    pub fn padding_left_fn(mut self, x: Pad) -> Self {
        self.padding_left = x;
        self
    }

    // ---- getters ----

    pub fn get_round(&self) -> bool {
        self.round
    }

    pub fn get_size(&self) -> [f64; 2] {
        [self.dx, self.dy]
    }

    pub fn get_padding_inner(&self) -> &dyn Fn(&Node) -> f64 {
        &*self.padding_inner
    }

    pub fn get_padding_top(&self) -> &dyn Fn(&Node) -> f64 {
        &*self.padding_top
    }

    pub fn get_padding_right(&self) -> &dyn Fn(&Node) -> f64 {
        &*self.padding_right
    }

    pub fn get_padding_bottom(&self) -> &dyn Fn(&Node) -> f64 {
        &*self.padding_bottom
    }

    pub fn get_padding_left(&self) -> &dyn Fn(&Node) -> f64 {
        &*self.padding_left
    }

    /// `treemap(root)`.
    pub fn treemap(&self, root: &mut Tree) {
        let r = root.root;
        root.nodes[r].x0 = 0.0;
        root.nodes[r].y0 = 0.0;
        root.nodes[r].x1 = self.dx;
        root.nodes[r].y1 = self.dy;
        // `paddingStack` is indexed by depth; pre-order guarantees a parent
        // writes its children's entry before they are visited.
        let mut padding_stack: Vec<f64> = vec![0.0];
        for i in root.order_before(r) {
            self.position_node(root, i, &mut padding_stack);
        }
        if self.round {
            for i in root.order_before(r) {
                round_node(&mut root.nodes[i]);
            }
        }
    }

    fn position_node(&self, t: &mut Tree, i: usize, padding_stack: &mut Vec<f64>) {
        let depth = t.nodes[i].depth;
        let mut p = padding_stack[depth];
        let mut x0 = t.nodes[i].x0 + p;
        let mut y0 = t.nodes[i].y0 + p;
        let mut x1 = t.nodes[i].x1 - p;
        let mut y1 = t.nodes[i].y1 - p;
        if x1 < x0 {
            x0 = (x0 + x1) / 2.0;
            x1 = x0;
        }
        if y1 < y0 {
            y0 = (y0 + y1) / 2.0;
            y1 = y0;
        }
        t.nodes[i].x0 = x0;
        t.nodes[i].y0 = y0;
        t.nodes[i].x1 = x1;
        t.nodes[i].y1 = y1;
        if t.nodes[i].children.is_some() {
            p = (self.padding_inner)(&t.nodes[i]) / 2.0;
            while padding_stack.len() <= depth + 1 {
                padding_stack.push(0.0);
            }
            padding_stack[depth + 1] = p;
            x0 += (self.padding_left)(&t.nodes[i]) - p;
            y0 += (self.padding_top)(&t.nodes[i]) - p;
            x1 -= (self.padding_right)(&t.nodes[i]) - p;
            y1 -= (self.padding_bottom)(&t.nodes[i]) - p;
            if x1 < x0 {
                x0 = (x0 + x1) / 2.0;
                x1 = x0;
            }
            if y1 < y0 {
                y0 = (y0 + y1) / 2.0;
                y1 = y0;
            }
            (self.tile)(t, i, x0, y0, x1, y1);
        }
    }
}

/// `d3.treemap()`.
pub fn treemap() -> Treemap {
    Treemap::new()
}
