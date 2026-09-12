//! `d3.partition()`, ported from d3-hierarchy 3.1.2 `src/partition.js`.

use crate::node::Tree;
use crate::treemap::{dice, round_node};

/// `d3.partition()`.
pub struct Partition {
    dx: f64,
    dy: f64,
    padding: f64,
    round: bool,
}

impl Default for Partition {
    fn default() -> Self {
        Self::new()
    }
}

impl Partition {
    pub fn new() -> Self {
        Partition {
            dx: 1.0,
            dy: 1.0,
            padding: 0.0,
            round: false,
        }
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

    pub fn padding(mut self, x: f64) -> Self {
        self.padding = x;
        self
    }

    pub fn get_round(&self) -> bool {
        self.round
    }

    pub fn get_size(&self) -> [f64; 2] {
        [self.dx, self.dy]
    }

    pub fn get_padding(&self) -> f64 {
        self.padding
    }

    /// `partition(root)`.
    pub fn partition(&self, root: &mut Tree) {
        let r = root.root;
        let n = (root.nodes[r].height + 1) as f64;
        root.nodes[r].x0 = self.padding;
        root.nodes[r].y0 = self.padding;
        root.nodes[r].x1 = self.dx;
        root.nodes[r].y1 = self.dy / n;
        for i in root.order_before(r) {
            self.position_node(root, i, n);
        }
        if self.round {
            for i in root.order_before(r) {
                round_node(&mut root.nodes[i]);
            }
        }
    }

    fn position_node(&self, t: &mut Tree, i: usize, n: f64) {
        if t.nodes[i].children.is_some() {
            let depth = t.nodes[i].depth as f64;
            let x0 = t.nodes[i].x0;
            let x1 = t.nodes[i].x1;
            dice(
                t,
                i,
                x0,
                self.dy * (depth + 1.0) / n,
                x1,
                self.dy * (depth + 2.0) / n,
            );
        }
        let mut x0 = t.nodes[i].x0;
        let mut y0 = t.nodes[i].y0;
        let mut x1 = t.nodes[i].x1 - self.padding;
        let mut y1 = t.nodes[i].y1 - self.padding;
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
    }
}

/// `d3.partition()`.
pub fn partition() -> Partition {
    Partition::new()
}
