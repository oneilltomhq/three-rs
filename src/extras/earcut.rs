//! Port of `three.js/src/extras/Earcut.js`, which wraps the copy of
//! [mapbox/earcut](https://github.com/mapbox/earcut) v3.0.2 three.js vendors
//! as `src/extras/lib/earcut.js`.
//!
//! The port is function for function and keeps every comparison, so it emits
//! the same triangles **in the same order** as three.js — `ExtrudeGeometry`'s
//! and `ShapeGeometry`'s vertex order depends on it, and
//! `tests/geometries_shape_oracle.rs` checks it against three.js itself.
//!
//! The JavaScript builds a circular doubly linked list out of node objects.
//! Here the nodes live in one `Vec` and link by index; a removed node keeps
//! its own `prev` / `next`, exactly as a JS node object does after
//! `removeNode`, because the algorithm reads them afterwards
//! (`p = end = p.prev`).

/// `Earcut.triangulate( data, holeIndices, dim )`: triangulates a polygon
/// given as flat coordinates (`[x0, y0, x1, y1, ...]` for `dim = 2`), with
/// holes starting at the given vertex indices. Returns vertex indices, three
/// per triangle.
pub fn triangulate(data: &[f64], hole_indices: &[usize], dim: usize) -> Vec<usize> {
    earcut(data, hole_indices, dim)
}

type NodeId = usize;

#[derive(Clone, Debug)]
struct Node {
    /// vertex index in coordinates array
    i: usize,
    /// vertex coordinates
    x: f64,
    y: f64,
    /// previous and next vertex nodes in a polygon ring
    prev: NodeId,
    next: NodeId,
    /// z-order curve value
    z: i32,
    /// previous and next nodes in z-order
    prev_z: Option<NodeId>,
    next_z: Option<NodeId>,
    /// indicates whether this is a steiner point
    steiner: bool,
}

/// The node arena, and the `minX` / `minY` / `invSize` the recursion threads
/// through (`invSize` is `undefined` — here `None` — for small inputs, and
/// three.js tests it for truthiness, so `Some(0.0)` counts as unset too).
struct Earcut {
    nodes: Vec<Node>,
}

/// `invSize ? ... : ...`: set and non-zero.
fn truthy(inv_size: Option<f64>) -> bool {
    matches!(inv_size, Some(v) if v != 0.0 && !v.is_nan())
}

fn earcut(data: &[f64], hole_indices: &[usize], dim: usize) -> Vec<usize> {
    let mut ec = Earcut { nodes: Vec::new() };

    let has_holes = !hole_indices.is_empty();
    let outer_len = if has_holes {
        hole_indices[0] * dim
    } else {
        data.len()
    };
    let outer_node = ec.linked_list(data, 0, outer_len, dim, true);
    let mut triangles = Vec::new();

    let Some(mut outer_node) = outer_node else {
        return triangles;
    };
    if ec.nodes[outer_node].next == ec.nodes[outer_node].prev {
        return triangles;
    }

    let mut min_x = f64::NAN;
    let mut min_y = f64::NAN;
    let mut inv_size = None;

    if has_holes {
        outer_node = ec.eliminate_holes(data, hole_indices, outer_node, dim);
    }

    // if the shape is not too simple, we'll use z-order curve hash later; calculate polygon bbox
    if data.len() > 80 * dim {
        min_x = data[0];
        min_y = data[1];
        let mut max_x = min_x;
        let mut max_y = min_y;

        let mut i = dim;
        while i < outer_len {
            let x = data[i];
            let y = data[i + 1];
            if x < min_x {
                min_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if x > max_x {
                max_x = x;
            }
            if y > max_y {
                max_y = y;
            }
            i += dim;
        }

        // minX, minY and invSize are later used to transform coords into integers for z-order calculation
        let size = (max_x - min_x).max(max_y - min_y);
        inv_size = Some(if size != 0.0 { 32767.0 / size } else { 0.0 });
    }

    ec.earcut_linked(
        Some(outer_node),
        &mut triangles,
        dim,
        min_x,
        min_y,
        inv_size,
        0,
    );

    triangles
}

impl Earcut {
    fn n(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    fn equals(&self, p1: NodeId, p2: NodeId) -> bool {
        let (a, b) = (self.n(p1), self.n(p2));
        a.x == b.x && a.y == b.y
    }

    // signed area of a triangle
    fn area(&self, p: NodeId, q: NodeId, r: NodeId) -> f64 {
        let (p, q, r) = (self.n(p), self.n(q), self.n(r));
        (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y)
    }

    // create a circular doubly linked list from polygon points in the specified winding order
    fn linked_list(
        &mut self,
        data: &[f64],
        start: usize,
        end: usize,
        dim: usize,
        clockwise: bool,
    ) -> Option<NodeId> {
        let mut last: Option<NodeId> = None;

        if clockwise == (signed_area(data, start, end, dim) > 0.0) {
            let mut i = start;
            while i < end {
                last = Some(self.insert_node(i / dim, data[i], data[i + 1], last));
                i += dim;
            }
        } else if end >= dim {
            // `for (let i = end - dim; i >= start; i -= dim)`, with `i` signed.
            let mut i = (end - dim) as isize;
            while i >= start as isize {
                let iu = i as usize;
                last = Some(self.insert_node(iu / dim, data[iu], data[iu + 1], last));
                i -= dim as isize;
            }
        }

        if let Some(l) = last {
            if self.equals(l, self.n(l).next) {
                self.remove_node(l);
                last = Some(self.n(l).next);
            }
        }

        last
    }

    // eliminate colinear or duplicate points
    fn filter_points(&mut self, start: Option<NodeId>, end: Option<NodeId>) -> Option<NodeId> {
        let start = start?;
        let mut end = end.unwrap_or(start);

        let mut p = start;
        loop {
            let mut again = false;

            let (prev, next) = (self.n(p).prev, self.n(p).next);
            if !self.n(p).steiner && (self.equals(p, next) || self.area(prev, p, next) == 0.0) {
                self.remove_node(p);
                p = self.n(p).prev;
                end = p;
                if p == self.n(p).next {
                    break;
                }
                again = true;
            } else {
                p = self.n(p).next;
            }

            if !(again || p != end) {
                break;
            }
        }

        Some(end)
    }

    // main ear slicing loop which triangulates a polygon (given as a linked list)
    #[allow(clippy::too_many_arguments)]
    fn earcut_linked(
        &mut self,
        ear: Option<NodeId>,
        triangles: &mut Vec<usize>,
        dim: usize,
        min_x: f64,
        min_y: f64,
        inv_size: Option<f64>,
        pass: u8,
    ) {
        let Some(mut ear) = ear else {
            return;
        };

        // interlink polygon nodes in z-order
        if pass == 0 && truthy(inv_size) {
            self.index_curve(ear, min_x, min_y, inv_size.unwrap_or(0.0));
        }

        let mut stop = ear;

        // iterate through ears, slicing them one by one
        while self.n(ear).prev != self.n(ear).next {
            let prev = self.n(ear).prev;
            let next = self.n(ear).next;

            let is_ear = if truthy(inv_size) {
                self.is_ear_hashed(ear, min_x, min_y, inv_size.unwrap_or(0.0))
            } else {
                self.is_ear(ear)
            };

            if is_ear {
                triangles.push(self.n(prev).i);
                triangles.push(self.n(ear).i);
                triangles.push(self.n(next).i); // cut off the triangle

                self.remove_node(ear);

                // skipping the next vertex leads to less sliver triangles
                ear = self.n(next).next;
                stop = self.n(next).next;

                continue;
            }

            ear = next;

            // if we looped through the whole remaining polygon and can't find any more ears
            if ear == stop {
                // try filtering points and slicing again
                if pass == 0 {
                    let filtered = self.filter_points(Some(ear), None);
                    self.earcut_linked(filtered, triangles, dim, min_x, min_y, inv_size, 1);

                // if this didn't work, try curing all small self-intersections locally
                } else if pass == 1 {
                    let filtered = self.filter_points(Some(ear), None);
                    let cured = self.cure_local_intersections(filtered, triangles);
                    self.earcut_linked(cured, triangles, dim, min_x, min_y, inv_size, 2);

                // as a last resort, try splitting the remaining polygon into two
                } else if pass == 2 {
                    self.split_earcut(ear, triangles, dim, min_x, min_y, inv_size);
                }

                break;
            }
        }
    }

    // check whether a polygon node forms a valid ear with adjacent nodes
    fn is_ear(&self, ear: NodeId) -> bool {
        let a = self.n(ear).prev;
        let b = ear;
        let c = self.n(ear).next;

        if self.area(a, b, c) >= 0.0 {
            return false; // reflex, can't be an ear
        }

        // now make sure we don't have other points inside the potential ear
        let (ax, bx, cx) = (self.n(a).x, self.n(b).x, self.n(c).x);
        let (ay, by, cy) = (self.n(a).y, self.n(b).y, self.n(c).y);

        // triangle bbox
        let x0 = js_min3(ax, bx, cx);
        let y0 = js_min3(ay, by, cy);
        let x1 = js_max3(ax, bx, cx);
        let y1 = js_max3(ay, by, cy);

        let mut p = self.n(c).next;
        while p != a {
            let pn = self.n(p);
            if pn.x >= x0
                && pn.x <= x1
                && pn.y >= y0
                && pn.y <= y1
                && point_in_triangle_except_first(ax, ay, bx, by, cx, cy, pn.x, pn.y)
                && self.area(pn.prev, p, pn.next) >= 0.0
            {
                return false;
            }
            p = pn.next;
        }

        true
    }

    /// One candidate test of `isEarHashed`, shared by its three loops.
    #[allow(clippy::too_many_arguments)]
    fn blocks_ear(
        &self,
        p: NodeId,
        a: NodeId,
        c: NodeId,
        tri: (f64, f64, f64, f64, f64, f64),
        bbox: (f64, f64, f64, f64),
    ) -> bool {
        let (ax, ay, bx, by, cx, cy) = tri;
        let (x0, y0, x1, y1) = bbox;
        let pn = self.n(p);
        pn.x >= x0
            && pn.x <= x1
            && pn.y >= y0
            && pn.y <= y1
            && p != a
            && p != c
            && point_in_triangle_except_first(ax, ay, bx, by, cx, cy, pn.x, pn.y)
            && self.area(pn.prev, p, pn.next) >= 0.0
    }

    fn is_ear_hashed(&self, ear: NodeId, min_x: f64, min_y: f64, inv_size: f64) -> bool {
        let a = self.n(ear).prev;
        let b = ear;
        let c = self.n(ear).next;

        if self.area(a, b, c) >= 0.0 {
            return false; // reflex, can't be an ear
        }

        let (ax, bx, cx) = (self.n(a).x, self.n(b).x, self.n(c).x);
        let (ay, by, cy) = (self.n(a).y, self.n(b).y, self.n(c).y);

        // triangle bbox
        let x0 = js_min3(ax, bx, cx);
        let y0 = js_min3(ay, by, cy);
        let x1 = js_max3(ax, bx, cx);
        let y1 = js_max3(ay, by, cy);

        // z-order range for the current triangle bbox;
        let min_z = z_order(x0, y0, min_x, min_y, inv_size);
        let max_z = z_order(x1, y1, min_x, min_y, inv_size);

        let tri = (ax, ay, bx, by, cx, cy);
        let bbox = (x0, y0, x1, y1);

        let mut p = self.n(ear).prev_z;
        let mut n = self.n(ear).next_z;

        // look for points inside the triangle in both directions
        while let (Some(pp), Some(nn)) = (p, n) {
            if !(self.n(pp).z >= min_z && self.n(nn).z <= max_z) {
                break;
            }
            if self.blocks_ear(pp, a, c, tri, bbox) {
                return false;
            }
            p = self.n(pp).prev_z;

            if self.blocks_ear(nn, a, c, tri, bbox) {
                return false;
            }
            n = self.n(nn).next_z;
        }

        // look for remaining points in decreasing z-order
        while let Some(pp) = p {
            if self.n(pp).z < min_z {
                break;
            }
            if self.blocks_ear(pp, a, c, tri, bbox) {
                return false;
            }
            p = self.n(pp).prev_z;
        }

        // look for remaining points in increasing z-order
        while let Some(nn) = n {
            if self.n(nn).z > max_z {
                break;
            }
            if self.blocks_ear(nn, a, c, tri, bbox) {
                return false;
            }
            n = self.n(nn).next_z;
        }

        true
    }

    // go through all polygon nodes and cure small local self-intersections
    fn cure_local_intersections(
        &mut self,
        start: Option<NodeId>,
        triangles: &mut Vec<usize>,
    ) -> Option<NodeId> {
        // `filterPoints` only returns null for a null start, and three.js then
        // throws on `p.prev`; nothing reaches here with one.
        let mut start = start?;
        let mut p = start;
        loop {
            let a = self.n(p).prev;
            let b = self.n(self.n(p).next).next;

            if !self.equals(a, b)
                && self.intersects(a, p, self.n(p).next, b)
                && self.locally_inside(a, b)
                && self.locally_inside(b, a)
            {
                triangles.push(self.n(a).i);
                triangles.push(self.n(p).i);
                triangles.push(self.n(b).i);

                // remove two nodes involved
                self.remove_node(p);
                let pn = self.n(p).next;
                self.remove_node(pn);

                p = b;
                start = b;
            }
            p = self.n(p).next;

            if p == start {
                break;
            }
        }

        self.filter_points(Some(p), None)
    }

    // try splitting polygon into two and triangulate them independently
    fn split_earcut(
        &mut self,
        start: NodeId,
        triangles: &mut Vec<usize>,
        dim: usize,
        min_x: f64,
        min_y: f64,
        inv_size: Option<f64>,
    ) {
        // look for a valid diagonal that divides the polygon into two
        let mut a = start;
        loop {
            let mut b = self.n(self.n(a).next).next;
            while b != self.n(a).prev {
                if self.n(a).i != self.n(b).i && self.is_valid_diagonal(a, b) {
                    // split the polygon in two by the diagonal
                    let c = self.split_polygon(a, b);

                    // filter colinear points around the cuts
                    let a_next = self.n(a).next;
                    let a = self.filter_points(Some(a), Some(a_next));
                    let c_next = self.n(c).next;
                    let c = self.filter_points(Some(c), Some(c_next));

                    // run earcut on each half
                    self.earcut_linked(a, triangles, dim, min_x, min_y, inv_size, 0);
                    self.earcut_linked(c, triangles, dim, min_x, min_y, inv_size, 0);
                    return;
                }
                b = self.n(b).next;
            }
            a = self.n(a).next;

            if a == start {
                break;
            }
        }
    }

    // link every hole into the outer loop, producing a single-ring polygon without holes
    fn eliminate_holes(
        &mut self,
        data: &[f64],
        hole_indices: &[usize],
        mut outer_node: NodeId,
        dim: usize,
    ) -> NodeId {
        let mut queue: Vec<NodeId> = Vec::new();

        let len = hole_indices.len();
        for i in 0..len {
            let start = hole_indices[i] * dim;
            let end = if i < len - 1 {
                hole_indices[i + 1] * dim
            } else {
                data.len()
            };
            // three.js reads `list.next` unguarded, so an empty hole throws
            // there; the port panics at the same point.
            let list = self
                .linked_list(data, start, end, dim, false)
                .expect("three-rs: earcut: an empty hole (three.js throws on the null list)");
            if list == self.n(list).next {
                self.nodes[list].steiner = true;
            }
            queue.push(self.get_leftmost(list));
        }

        queue.sort_by(|&a, &b| {
            let result = self.compare_x_y_slope(a, b);
            if result < 0.0 {
                std::cmp::Ordering::Less
            } else if result > 0.0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // process holes from left to right
        for &hole in &queue {
            outer_node = self.eliminate_hole(hole, outer_node);
        }

        outer_node
    }

    fn compare_x_y_slope(&self, a: NodeId, b: NodeId) -> f64 {
        let (an, bn) = (self.n(a), self.n(b));
        let mut result = an.x - bn.x;
        // when the left-most point of 2 holes meet at a vertex, sort the holes counterclockwise so that when we find
        // the bridge to the outer shell is always the point that they meet at.
        if result == 0.0 {
            result = an.y - bn.y;
            if result == 0.0 {
                let (anx, bnx) = (self.n(an.next), self.n(bn.next));
                let a_slope = (anx.y - an.y) / (anx.x - an.x);
                let b_slope = (bnx.y - bn.y) / (bnx.x - bn.x);
                result = a_slope - b_slope;
            }
        }
        result
    }

    // find a bridge between vertices that connects hole with an outer ring and link it
    fn eliminate_hole(&mut self, hole: NodeId, outer_node: NodeId) -> NodeId {
        let Some(bridge) = self.find_hole_bridge(hole, outer_node) else {
            return outer_node;
        };

        let bridge_reverse = self.split_polygon(bridge, hole);

        // filter collinear points around the cuts
        let brn = self.n(bridge_reverse).next;
        self.filter_points(Some(bridge_reverse), Some(brn));
        let bn = self.n(bridge).next;
        self.filter_points(Some(bridge), Some(bn))
            .expect("three-rs: earcut: filterPoints of a non-null start is non-null")
    }

    // David Eberly's algorithm for finding a bridge between hole and outer polygon
    fn find_hole_bridge(&self, hole: NodeId, outer_node: NodeId) -> Option<NodeId> {
        let mut p = outer_node;
        let hx = self.n(hole).x;
        let hy = self.n(hole).y;
        let mut qx = f64::NEG_INFINITY;
        let mut m: Option<NodeId> = None;

        // find a segment intersected by a ray from the hole's leftmost point to the left;
        // segment's endpoint with lesser x will be potential connection point
        // unless they intersect at a vertex, then choose the vertex
        if self.equals(hole, p) {
            return Some(p);
        }
        loop {
            let pn = self.n(p);
            let next = self.n(pn.next);
            if self.equals(hole, pn.next) {
                return Some(pn.next);
            } else if hy <= pn.y && hy >= next.y && next.y != pn.y {
                let x = pn.x + (hy - pn.y) * (next.x - pn.x) / (next.y - pn.y);
                if x <= hx && x > qx {
                    qx = x;
                    let mm = if pn.x < next.x { p } else { pn.next };
                    m = Some(mm);
                    if x == hx {
                        return Some(mm); // hole touches outer segment; pick leftmost endpoint
                    }
                }
            }
            p = pn.next;

            if p == outer_node {
                break;
            }
        }

        let mut m = m?;

        // look for points inside the triangle of hole point, segment intersection and endpoint;
        // if there are no points found, we have a valid connection;
        // otherwise choose the point of the minimum angle with the ray as connection point

        let stop = m;
        let mx = self.n(m).x;
        let my = self.n(m).y;
        let mut tan_min = f64::INFINITY;

        p = m;

        loop {
            let (px, py) = (self.n(p).x, self.n(p).y);
            if hx >= px
                && px >= mx
                && hx != px
                && point_in_triangle(
                    if hy < my { hx } else { qx },
                    hy,
                    mx,
                    my,
                    if hy < my { qx } else { hx },
                    hy,
                    px,
                    py,
                )
            {
                let tan = (hy - py).abs() / (hx - px); // tangential

                if self.locally_inside(p, hole)
                    && (tan < tan_min
                        || (tan == tan_min
                            && (px > self.n(m).x
                                || (px == self.n(m).x && self.sector_contains_sector(m, p)))))
                {
                    m = p;
                    tan_min = tan;
                }
            }

            p = self.n(p).next;

            if p == stop {
                break;
            }
        }

        Some(m)
    }

    // whether sector in vertex m contains sector in vertex p in the same coordinates
    fn sector_contains_sector(&self, m: NodeId, p: NodeId) -> bool {
        self.area(self.n(m).prev, m, self.n(p).prev) < 0.0
            && self.area(self.n(p).next, m, self.n(m).next) < 0.0
    }

    // interlink polygon nodes in z-order
    fn index_curve(&mut self, start: NodeId, min_x: f64, min_y: f64, inv_size: f64) {
        let mut p = start;
        loop {
            if self.nodes[p].z == 0 {
                self.nodes[p].z = z_order(self.nodes[p].x, self.nodes[p].y, min_x, min_y, inv_size);
            }
            self.nodes[p].prev_z = Some(self.nodes[p].prev);
            self.nodes[p].next_z = Some(self.nodes[p].next);
            p = self.nodes[p].next;

            if p == start {
                break;
            }
        }

        let pz = self.nodes[p]
            .prev_z
            .expect("three-rs: earcut: indexCurve just set prevZ");
        self.nodes[pz].next_z = None;
        self.nodes[p].prev_z = None;

        self.sort_linked(p);
    }

    // Simon Tatham's linked list merge sort algorithm
    // http://www.chiark.greenend.org.uk/~sgtatham/algorithms/listsort.html
    fn sort_linked(&mut self, list: NodeId) -> Option<NodeId> {
        let mut list = Some(list);
        let mut in_size = 1;

        loop {
            let mut p = list;
            list = None;
            let mut tail: Option<NodeId> = None;
            let mut num_merges = 0;

            while let Some(pp) = p {
                num_merges += 1;
                let mut q = Some(pp);
                let mut p_size = 0;
                for _ in 0..in_size {
                    p_size += 1;
                    q = q.and_then(|q| self.nodes[q].next_z);
                    if q.is_none() {
                        break;
                    }
                }
                let mut q_size = in_size;

                let mut p_cur = Some(pp);
                while p_size > 0 || (q_size > 0 && q.is_some()) {
                    let e;
                    let take_p = p_size != 0
                        && (q_size == 0
                            || q.is_none()
                            || self.nodes[p_cur.expect("pSize > 0")].z
                                <= self.nodes[q.expect("q checked")].z);
                    if take_p {
                        let ep = p_cur.expect("three-rs: earcut: pSize > 0 means p is set");
                        e = ep;
                        p_cur = self.nodes[ep].next_z;
                        p_size -= 1;
                    } else {
                        let eq = q.expect("three-rs: earcut: the q branch runs with q set");
                        e = eq;
                        q = self.nodes[eq].next_z;
                        q_size -= 1;
                    }

                    if let Some(t) = tail {
                        self.nodes[t].next_z = Some(e);
                    } else {
                        list = Some(e);
                    }

                    self.nodes[e].prev_z = tail;
                    tail = Some(e);
                }

                p = q;
            }

            if let Some(t) = tail {
                self.nodes[t].next_z = None;
            }
            in_size *= 2;

            if num_merges <= 1 {
                break;
            }
        }

        list
    }

    // find the leftmost node of a polygon ring
    fn get_leftmost(&self, start: NodeId) -> NodeId {
        let mut p = start;
        let mut leftmost = start;
        loop {
            let (pn, ln) = (self.n(p), self.n(leftmost));
            if pn.x < ln.x || (pn.x == ln.x && pn.y < ln.y) {
                leftmost = p;
            }
            p = pn.next;

            if p == start {
                break;
            }
        }

        leftmost
    }

    // check if a diagonal between two polygon nodes is valid (lies in polygon interior)
    fn is_valid_diagonal(&self, a: NodeId, b: NodeId) -> bool {
        let (an, bn) = (self.n(a), self.n(b));
        // `area(...) || area(...)`: a number is truthy when non-zero and not NaN.
        let area_truthy = |v: f64| v != 0.0 && !v.is_nan();
        self.n(an.next).i != bn.i
            && self.n(an.prev).i != bn.i
            && !self.intersects_polygon(a, b) // doesn't intersect other edges
            && ((self.locally_inside(a, b)
                && self.locally_inside(b, a)
                && self.middle_inside(a, b) // locally visible
                && (area_truthy(self.area(an.prev, a, bn.prev))
                    || area_truthy(self.area(a, bn.prev, b)))) // does not create opposite-facing sectors
                || (self.equals(a, b)
                    && self.area(an.prev, a, an.next) > 0.0
                    && self.area(bn.prev, b, bn.next) > 0.0)) // special zero-length case
    }

    // check if two segments intersect
    fn intersects(&self, p1: NodeId, q1: NodeId, p2: NodeId, q2: NodeId) -> bool {
        let o1 = sign(self.area(p1, q1, p2));
        let o2 = sign(self.area(p1, q1, q2));
        let o3 = sign(self.area(p2, q2, p1));
        let o4 = sign(self.area(p2, q2, q1));

        if o1 != o2 && o3 != o4 {
            return true; // general case
        }

        if o1 == 0 && self.on_segment(p1, p2, q1) {
            return true; // p1, q1 and p2 are collinear and p2 lies on p1q1
        }
        if o2 == 0 && self.on_segment(p1, q2, q1) {
            return true; // p1, q1 and q2 are collinear and q2 lies on p1q1
        }
        if o3 == 0 && self.on_segment(p2, p1, q2) {
            return true; // p2, q2 and p1 are collinear and p1 lies on p2q2
        }
        if o4 == 0 && self.on_segment(p2, q1, q2) {
            return true; // p2, q2 and q1 are collinear and q1 lies on p2q2
        }

        false
    }

    // for collinear points p, q, r, check if point q lies on segment pr
    fn on_segment(&self, p: NodeId, q: NodeId, r: NodeId) -> bool {
        let (p, q, r) = (self.n(p), self.n(q), self.n(r));
        q.x <= js_max2(p.x, r.x)
            && q.x >= js_min2(p.x, r.x)
            && q.y <= js_max2(p.y, r.y)
            && q.y >= js_min2(p.y, r.y)
    }

    // check if a polygon diagonal intersects any polygon segments
    fn intersects_polygon(&self, a: NodeId, b: NodeId) -> bool {
        let (ai, bi) = (self.n(a).i, self.n(b).i);
        let mut p = a;
        loop {
            let pn = self.n(p);
            let next_i = self.n(pn.next).i;
            if pn.i != ai
                && next_i != ai
                && pn.i != bi
                && next_i != bi
                && self.intersects(p, pn.next, a, b)
            {
                return true;
            }
            p = pn.next;

            if p == a {
                break;
            }
        }

        false
    }

    // check if a polygon diagonal is locally inside the polygon
    fn locally_inside(&self, a: NodeId, b: NodeId) -> bool {
        let an = self.n(a);
        if self.area(an.prev, a, an.next) < 0.0 {
            self.area(a, b, an.next) >= 0.0 && self.area(a, an.prev, b) >= 0.0
        } else {
            self.area(a, b, an.prev) < 0.0 || self.area(a, an.next, b) < 0.0
        }
    }

    // check if the middle point of a polygon diagonal is inside the polygon
    fn middle_inside(&self, a: NodeId, b: NodeId) -> bool {
        let mut p = a;
        let mut inside = false;
        let px = (self.n(a).x + self.n(b).x) / 2.0;
        let py = (self.n(a).y + self.n(b).y) / 2.0;
        loop {
            let pn = self.n(p);
            let next = self.n(pn.next);
            if ((pn.y > py) != (next.y > py))
                && next.y != pn.y
                && (px < (next.x - pn.x) * (py - pn.y) / (next.y - pn.y) + pn.x)
            {
                inside = !inside;
            }
            p = pn.next;

            if p == a {
                break;
            }
        }

        inside
    }

    // link two polygon vertices with a bridge; if the vertices belong to the same ring, it splits polygon into two;
    // if one belongs to the outer ring and another to a hole, it merges it into a single ring
    fn split_polygon(&mut self, a: NodeId, b: NodeId) -> NodeId {
        let a2 = self.create_node(self.n(a).i, self.n(a).x, self.n(a).y);
        let b2 = self.create_node(self.n(b).i, self.n(b).x, self.n(b).y);
        let an = self.n(a).next;
        let bp = self.n(b).prev;

        self.nodes[a].next = b;
        self.nodes[b].prev = a;

        self.nodes[a2].next = an;
        self.nodes[an].prev = a2;

        self.nodes[b2].next = a2;
        self.nodes[a2].prev = b2;

        self.nodes[bp].next = b2;
        self.nodes[b2].prev = bp;

        b2
    }

    // create a node and optionally link it with previous one (in a circular doubly linked list)
    fn insert_node(&mut self, i: usize, x: f64, y: f64, last: Option<NodeId>) -> NodeId {
        let p = self.create_node(i, x, y);

        match last {
            None => {
                self.nodes[p].prev = p;
                self.nodes[p].next = p;
            }
            Some(last) => {
                let last_next = self.n(last).next;
                self.nodes[p].next = last_next;
                self.nodes[p].prev = last;
                self.nodes[last_next].prev = p;
                self.nodes[last].next = p;
            }
        }
        p
    }

    fn remove_node(&mut self, p: NodeId) {
        let (prev, next) = (self.n(p).prev, self.n(p).next);
        self.nodes[next].prev = prev;
        self.nodes[prev].next = next;

        let (prev_z, next_z) = (self.n(p).prev_z, self.n(p).next_z);
        if let Some(pz) = prev_z {
            self.nodes[pz].next_z = next_z;
        }
        if let Some(nz) = next_z {
            self.nodes[nz].prev_z = prev_z;
        }
    }

    fn create_node(&mut self, i: usize, x: f64, y: f64) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node {
            i,
            x,
            y,
            // `null` in three.js until linked; every caller links it at once.
            prev: id,
            next: id,
            z: 0,
            prev_z: None,
            next_z: None,
            steiner: false,
        });
        id
    }
}

// z-order of a point given coords and inverse of the longer side of data bbox
fn z_order(x: f64, y: f64, min_x: f64, min_y: f64, inv_size: f64) -> i32 {
    // coords are transformed into non-negative 15-bit integer range
    let mut x = to_int32((x - min_x) * inv_size);
    let mut y = to_int32((y - min_y) * inv_size);

    x = (x | (x << 8)) & 0x00FF00FF;
    x = (x | (x << 4)) & 0x0F0F0F0F;
    x = (x | (x << 2)) & 0x33333333;
    x = (x | (x << 1)) & 0x55555555;

    y = (y | (y << 8)) & 0x00FF00FF;
    y = (y | (y << 4)) & 0x0F0F0F0F;
    y = (y | (y << 2)) & 0x33333333;
    y = (y | (y << 1)) & 0x55555555;

    x | (y << 1)
}

/// JavaScript's `v | 0` (ToInt32): truncate, wrap modulo 2^32, NaN and the
/// infinities to 0.
fn to_int32(v: f64) -> i32 {
    if !v.is_finite() {
        return 0;
    }
    (v.trunc().rem_euclid(4294967296.0) as u64 as u32) as i32
}

// check if a point lies within a convex triangle
#[allow(clippy::too_many_arguments)]
fn point_in_triangle(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    px: f64,
    py: f64,
) -> bool {
    (cx - px) * (ay - py) >= (ax - px) * (cy - py)
        && (ax - px) * (by - py) >= (bx - px) * (ay - py)
        && (bx - px) * (cy - py) >= (cx - px) * (by - py)
}

// check if a point lies within a convex triangle but false if its equal to the first point of the triangle
#[allow(clippy::too_many_arguments)]
fn point_in_triangle_except_first(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    px: f64,
    py: f64,
) -> bool {
    !(ax == px && ay == py) && point_in_triangle(ax, ay, bx, by, cx, cy, px, py)
}

fn sign(num: f64) -> i32 {
    if num > 0.0 {
        1
    } else if num < 0.0 {
        -1
    } else {
        0
    }
}

/// `Math.min( a, b )`: NaN if either is.
fn js_min2(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// `Math.max( a, b )`: NaN if either is.
fn js_max2(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn js_min3(a: f64, b: f64, c: f64) -> f64 {
    js_min2(js_min2(a, b), c)
}

fn js_max3(a: f64, b: f64, c: f64) -> f64 {
    js_max2(js_max2(a, b), c)
}

fn signed_area(data: &[f64], start: usize, end: usize, dim: usize) -> f64 {
    let mut sum = 0.0;
    if end < dim {
        return sum;
    }
    let mut i = start;
    let mut j = end - dim;
    while i < end {
        sum += (data[j] - data[i]) * (data[i + 1] + data[j + 1]);
        j = i;
        i += dim;
    }
    sum
}
