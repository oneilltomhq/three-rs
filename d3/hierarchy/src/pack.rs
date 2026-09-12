//! `pack`, ported from d3-hierarchy 3.1.2 `src/pack/{enclose,siblings,index}.js`.
//!
//! The operation order of every expression is the JS's, so results match to the
//! bit. `Circle` stands in for the `{x, y, r}` object literals the JS passes
//! around; the layout itself writes `x`, `y` and `r` onto the arena nodes.

use crate::node::{Node, Tree};
use crate::{lcg, shuffle};

/// The `{x, y, r}` circles `packEnclose`/`packSiblings` work with.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Circle {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

impl Circle {
    pub fn new(x: f64, y: f64, r: f64) -> Self {
        Circle { x, y, r }
    }

    /// A circle with only a radius, as the tests' `{r: …}` literals (x, y are 0
    /// but are always overwritten by the layout before being read).
    pub fn radius(r: f64) -> Self {
        Circle { x: 0.0, y: 0.0, r }
    }
}

// ---------------------------------------------------------------- enclose.js

/// `d3.packEnclose(circles)`: creates its own `lcg()`, as the JS does.
pub fn enclose(circles: &[Circle]) -> Option<Circle> {
    let mut random = lcg();
    enclose_random(circles, &mut random)
}

/// `packEncloseRandom(circles, random)`.
pub fn enclose_random(circles: &[Circle], random: &mut impl FnMut() -> f64) -> Option<Circle> {
    let mut circles = circles.to_vec();
    shuffle(&mut circles, random);
    let n = circles.len();
    let mut i = 0usize;
    let mut b: Vec<Circle> = Vec::new();
    let mut e: Option<Circle> = None;

    while i < n {
        let p = circles[i];
        let weak = match &e {
            Some(e) => encloses_weak(e, &p),
            None => false,
        };
        if weak {
            i += 1;
        } else {
            b = extend_basis(&b, &p);
            e = enclose_basis(&b);
            i = 0;
        }
    }

    e
}

fn extend_basis(b: &[Circle], p: &Circle) -> Vec<Circle> {
    if encloses_weak_all(p, b) {
        return vec![*p];
    }

    // If we get here then B must have at least one element.
    for i in 0..b.len() {
        if encloses_not(p, &b[i]) && encloses_weak_all(&enclose_basis2(&b[i], p), b) {
            return vec![b[i], *p];
        }
    }

    // If we get here then B must have at least two elements.
    for i in 0..b.len().saturating_sub(1) {
        for j in i + 1..b.len() {
            if encloses_not(&enclose_basis2(&b[i], &b[j]), p)
                && encloses_not(&enclose_basis2(&b[i], p), &b[j])
                && encloses_not(&enclose_basis2(&b[j], p), &b[i])
                && encloses_weak_all(&enclose_basis3(&b[i], &b[j], p), b)
            {
                return vec![b[i], b[j], *p];
            }
        }
    }

    // If we get here then something is very wrong.
    panic!("extendBasis: something is very wrong");
}

fn encloses_not(a: &Circle, b: &Circle) -> bool {
    let dr = a.r - b.r;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dr < 0.0 || dr * dr < dx * dx + dy * dy
}

fn encloses_weak(a: &Circle, b: &Circle) -> bool {
    let dr = a.r - b.r + a.r.max(b.r).max(1.0) * 1e-9;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

fn encloses_weak_all(a: &Circle, b: &[Circle]) -> bool {
    for c in b {
        if !encloses_weak(a, c) {
            return false;
        }
    }
    true
}

fn enclose_basis(b: &[Circle]) -> Option<Circle> {
    match b.len() {
        1 => Some(enclose_basis1(&b[0])),
        2 => Some(enclose_basis2(&b[0], &b[1])),
        3 => Some(enclose_basis3(&b[0], &b[1], &b[2])),
        _ => None,
    }
}

fn enclose_basis1(a: &Circle) -> Circle {
    Circle {
        x: a.x,
        y: a.y,
        r: a.r,
    }
}

fn enclose_basis2(a: &Circle, b: &Circle) -> Circle {
    let (x1, y1, r1) = (a.x, a.y, a.r);
    let (x2, y2, r2) = (b.x, b.y, b.r);
    let x21 = x2 - x1;
    let y21 = y2 - y1;
    let r21 = r2 - r1;
    let l = (x21 * x21 + y21 * y21).sqrt();
    Circle {
        x: (x1 + x2 + x21 / l * r21) / 2.0,
        y: (y1 + y2 + y21 / l * r21) / 2.0,
        r: (l + r1 + r2) / 2.0,
    }
}

fn enclose_basis3(a: &Circle, b: &Circle, c: &Circle) -> Circle {
    let (x1, y1, r1) = (a.x, a.y, a.r);
    let (x2, y2, r2) = (b.x, b.y, b.r);
    let (x3, y3, r3) = (c.x, c.y, c.r);
    let a2 = x1 - x2;
    let a3 = x1 - x3;
    let b2 = y1 - y2;
    let b3 = y1 - y3;
    let c2 = r2 - r1;
    let c3 = r3 - r1;
    let d1 = x1 * x1 + y1 * y1 - r1 * r1;
    let d2 = d1 - x2 * x2 - y2 * y2 + r2 * r2;
    let d3 = d1 - x3 * x3 - y3 * y3 + r3 * r3;
    let ab = a3 * b2 - a2 * b3;
    let xa = (b2 * d3 - b3 * d2) / (ab * 2.0) - x1;
    let xb = (b3 * c2 - b2 * c3) / ab;
    let ya = (a3 * d2 - a2 * d3) / (ab * 2.0) - y1;
    let yb = (a2 * c3 - a3 * c2) / ab;
    let aa = xb * xb + yb * yb - 1.0;
    let bb = 2.0 * (r1 + xa * xb + ya * yb);
    let cc = xa * xa + ya * ya - r1 * r1;
    let r = -(if aa.abs() > 1e-6 {
        (bb + (bb * bb - 4.0 * aa * cc).sqrt()) / (2.0 * aa)
    } else {
        cc / bb
    });
    Circle {
        x: x1 + xa + xb * r,
        y: y1 + ya + yb * r,
        r,
    }
}

// --------------------------------------------------------------- siblings.js

/// `place(b, a, c)`: positions `circles[ci]` tangent to `circles[bi]` and
/// `circles[ai]`. Indices, because the JS mutates `c` in place.
fn place(circles: &mut [Circle], bi: usize, ai: usize, ci: usize) {
    let b = circles[bi];
    let a = circles[ai];
    let cr = circles[ci].r;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let d2 = dx * dx + dy * dy;
    if d2 != 0.0 {
        let mut a2 = a.r + cr;
        a2 *= a2;
        let mut b2 = b.r + cr;
        b2 *= b2;
        if a2 > b2 {
            let x = (d2 + b2 - a2) / (2.0 * d2);
            let y = (b2 / d2 - x * x).max(0.0).sqrt();
            circles[ci].x = b.x - x * dx - y * dy;
            circles[ci].y = b.y - x * dy + y * dx;
        } else {
            let x = (d2 + a2 - b2) / (2.0 * d2);
            let y = (a2 / d2 - x * x).max(0.0).sqrt();
            circles[ci].x = a.x + x * dx - y * dy;
            circles[ci].y = a.y + x * dy + y * dx;
        }
    } else {
        circles[ci].x = a.x + cr;
        circles[ci].y = a.y;
    }
}

fn intersects(a: &Circle, b: &Circle) -> bool {
    let dr = a.r + b.r - 1e-6;
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    dr > 0.0 && dr * dr > dx * dx + dy * dy
}

/// The front-chain: d3's doubly-linked `Node(circle)` list, as an arena.
struct Chain {
    /// index into the circles slice
    c: Vec<usize>,
    next: Vec<usize>,
    previous: Vec<usize>,
}

impl Chain {
    fn push(&mut self, c: usize) -> usize {
        self.c.push(c);
        self.next.push(usize::MAX);
        self.previous.push(usize::MAX);
        self.c.len() - 1
    }
}

/// `score(node)`.
fn score(circles: &[Circle], chain: &Chain, node: usize) -> f64 {
    let a = circles[chain.c[node]];
    let b = circles[chain.c[chain.next[node]]];
    let ab = a.r + b.r;
    let dx = (a.x * b.r + b.x * a.r) / ab;
    let dy = (a.y * b.r + b.y * a.r) / ab;
    dx * dx + dy * dy
}

/// `packSiblingsRandom(circles, random)`: lays out `circles` in place and
/// returns the radius of the enclosing circle (0 for no circles).
pub fn siblings_random(circles: &mut [Circle], random: &mut impl FnMut() -> f64) -> f64 {
    let n = circles.len();
    if n == 0 {
        return 0.0;
    }

    // Place the first circle.
    circles[0].x = 0.0;
    circles[0].y = 0.0;
    if !(n > 1) {
        return circles[0].r;
    }

    // Place the second circle.
    let ar = circles[0].r;
    let br = circles[1].r;
    circles[0].x = -br;
    circles[1].x = ar;
    circles[1].y = 0.0;
    if !(n > 2) {
        return ar + br;
    }

    // Place the third circle.
    place(circles, 1, 0, 2);

    // Initialize the front-chain using the first three circles a, b and c.
    let mut chain = Chain {
        c: Vec::new(),
        next: Vec::new(),
        previous: Vec::new(),
    };
    let mut a = chain.push(0);
    let mut b = chain.push(1);
    let mut c = chain.push(2);
    chain.next[a] = b;
    chain.previous[c] = b;
    chain.next[b] = c;
    chain.previous[a] = c;
    chain.next[c] = a;
    chain.previous[b] = a;

    // Attempt to place each remaining circle…
    let mut i = 3usize;
    'pack: while i < n {
        place(circles, chain.c[a], chain.c[b], i);
        c = chain.push(i);

        // Find the closest intersecting circle on the front-chain, if any.
        let mut j = chain.next[b];
        let mut k = chain.previous[a];
        let mut sj = circles[chain.c[b]].r;
        let mut sk = circles[chain.c[a]].r;
        loop {
            if sj <= sk {
                if intersects(&circles[chain.c[j]], &circles[chain.c[c]]) {
                    b = j;
                    chain.next[a] = b;
                    chain.previous[b] = a;
                    continue 'pack; // --i; ++i
                }
                sj += circles[chain.c[j]].r;
                j = chain.next[j];
            } else {
                if intersects(&circles[chain.c[k]], &circles[chain.c[c]]) {
                    a = k;
                    chain.next[a] = b;
                    chain.previous[b] = a;
                    continue 'pack; // --i; ++i
                }
                sk += circles[chain.c[k]].r;
                k = chain.previous[k];
            }
            if j == chain.next[k] {
                break;
            }
        }

        // Success! Insert the new circle c between a and b.
        chain.previous[c] = a;
        chain.next[c] = b;
        chain.next[a] = c;
        chain.previous[b] = c;
        b = c;

        // Compute the new closest circle pair to the centroid.
        let mut aa = score(circles, &chain, a);
        loop {
            c = chain.next[c];
            if c == b {
                break;
            }
            let ca = score(circles, &chain, c);
            if ca < aa {
                a = c;
                aa = ca;
            }
        }
        b = chain.next[a];

        i += 1;
    }

    // Compute the enclosing circle of the front chain.
    let mut front = vec![circles[chain.c[b]]];
    c = b;
    loop {
        c = chain.next[c];
        if c == b {
            break;
        }
        front.push(circles[chain.c[c]]);
    }
    let e = enclose_random(&front, random).unwrap();

    // Translate the circles to put the enclosing circle around the origin.
    for i in 0..n {
        circles[i].x -= e.x;
        circles[i].y -= e.y;
    }

    e.r
}

/// `d3.packSiblings(circles)`: lays out `circles` in place (creating its own
/// `lcg()`, as the JS does) and returns the enclosing circle, which is centred
/// on the origin; `None` for no circles.
pub fn siblings(circles: &mut [Circle]) -> Option<Circle> {
    if circles.is_empty() {
        return None;
    }
    let mut random = lcg();
    let r = siblings_random(circles, &mut random);
    Some(Circle { x: 0.0, y: 0.0, r })
}

// ------------------------------------------------------------------ index.js

fn default_radius(node: &Node) -> f64 {
    node.value.unwrap_or(f64::NAN).sqrt()
}

fn constant_zero(_node: &Node) -> f64 {
    0.0
}

/// `d3.pack()`. `radius` is d3's optional radius accessor (null by default) and
/// `padding` defaults to `constantZero`.
pub struct Pack<'a> {
    radius: Option<Box<dyn Fn(&Node) -> f64 + 'a>>,
    dx: f64,
    dy: f64,
    padding: Box<dyn Fn(&Node) -> f64 + 'a>,
}

impl<'a> Default for Pack<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Pack<'a> {
    pub fn new() -> Self {
        Pack {
            radius: None,
            dx: 1.0,
            dy: 1.0,
            padding: Box::new(constant_zero),
        }
    }

    /// `pack.radius(x)`
    pub fn radius(mut self, x: impl Fn(&Node) -> f64 + 'a) -> Self {
        self.radius = Some(Box::new(x));
        self
    }

    /// `pack.radius(null)`
    pub fn radius_none(mut self) -> Self {
        self.radius = None;
        self
    }

    /// `pack.radius()`: whether a radius accessor is set.
    pub fn get_radius(&self) -> Option<&dyn Fn(&Node) -> f64> {
        self.radius.as_deref()
    }

    /// `pack.size([dx, dy])`
    pub fn size(mut self, x: [f64; 2]) -> Self {
        self.dx = x[0];
        self.dy = x[1];
        self
    }

    /// `pack.size()`
    pub fn get_size(&self) -> [f64; 2] {
        [self.dx, self.dy]
    }

    /// `pack.padding(x)` with a function.
    pub fn padding(mut self, x: impl Fn(&Node) -> f64 + 'a) -> Self {
        self.padding = Box::new(x);
        self
    }

    /// `pack.padding(+x)` with a constant.
    pub fn padding_constant(mut self, x: f64) -> Self {
        self.padding = Box::new(move |_| x);
        self
    }

    /// `pack.padding()` applied to a node.
    pub fn get_padding(&self, node: &Node) -> f64 {
        (self.padding)(node)
    }

    /// `pack(root)`: writes `x`, `y` and `r` on every node.
    pub fn pack(&self, root: &mut Tree) {
        let mut random = lcg();
        let r = root.root;
        root.nodes[r].x = self.dx / 2.0;
        root.nodes[r].y = self.dy / 2.0;
        if let Some(radius) = &self.radius {
            radius_leaf(root, &**radius);
            pack_children(root, &*self.padding, 0.5, &mut random);
            translate_child(root, 1.0);
        } else {
            radius_leaf(root, &default_radius);
            pack_children(root, &constant_zero, 1.0, &mut random);
            let k = root.nodes[r].r / self.dx.min(self.dy);
            pack_children(root, &*self.padding, k, &mut random);
            let k = self.dx.min(self.dy) / (2.0 * root.nodes[r].r);
            translate_child(root, k);
        }
    }
}

/// `radiusLeaf(radius)` in eachBefore order.
fn radius_leaf(root: &mut Tree, radius: &dyn Fn(&Node) -> f64) {
    for i in root.order_before(root.root) {
        if root.nodes[i].children.is_none() {
            // `Math.max(0, +radius(node) || 0)`
            let mut r = radius(&root.nodes[i]);
            if r.is_nan() {
                r = 0.0;
            }
            root.nodes[i].r = 0.0f64.max(r);
        }
    }
}

/// `packChildrenRandom(padding, k, random)` in eachAfter order.
fn pack_children(
    root: &mut Tree,
    padding: &dyn Fn(&Node) -> f64,
    k: f64,
    random: &mut impl FnMut() -> f64,
) {
    for i in root.order_after(root.root) {
        let children = match &root.nodes[i].children {
            Some(c) if !c.is_empty() => c.clone(),
            _ => continue,
        };
        // `r = padding(node) * k || 0`
        let mut r = padding(&root.nodes[i]) * k;
        if r == 0.0 || r.is_nan() {
            r = 0.0;
        }
        let n = children.len();
        let mut circles: Vec<Circle> = Vec::with_capacity(n);
        for &c in &children {
            let node = &root.nodes[c];
            circles.push(Circle {
                x: node.x,
                y: node.y,
                r: node.r,
            });
        }
        if r != 0.0 {
            for c in circles.iter_mut() {
                c.r += r;
            }
        }
        let e = siblings_random(&mut circles, random);
        for (j, &c) in children.iter().enumerate() {
            root.nodes[c].x = circles[j].x;
            root.nodes[c].y = circles[j].y;
            // the JS adds then subtracts the padding on each child's radius;
            // the round trip is reproduced so the bits match.
            if r != 0.0 {
                root.nodes[c].r = circles[j].r - r;
            }
        }
        root.nodes[i].r = e + r;
    }
}

/// `translateChild(k)` in eachBefore order.
fn translate_child(root: &mut Tree, k: f64) {
    for i in root.order_before(root.root) {
        let parent = root.nodes[i].parent;
        root.nodes[i].r *= k;
        if let Some(p) = parent {
            let (px, py) = (root.nodes[p].x, root.nodes[p].y);
            root.nodes[i].x = px + k * root.nodes[i].x;
            root.nodes[i].y = py + k * root.nodes[i].y;
        }
    }
}
