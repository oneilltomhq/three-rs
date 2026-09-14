//! Ports of `three.js/src/objects/Line.js` and `three.js/src/objects/LineSegments.js`.
//!
//! `Line extends Object3D` — it is *not* a `Mesh`, and the renderer keeps them
//! apart: `WebGPUUtils.getPrimitiveTopology()` reads the object, not the
//! material, so the same `LineBasicNodeMaterial` draws a `line-strip` under a
//! `Line` and a `line-list` under a `LineSegments`.
//!
//! `LineLoop` is deliberately absent. `Renderer._projectObject()` errors on it
//! ("Objects of type THREE.LineLoop are not supported. Please use THREE.Line or
//! THREE.LineSegments."), so there is nothing to port.
//!
//! `computeLineDistances()` is not ported either: its only consumer is
//! `LineDashedMaterial` / `dashSize`, which no rung needs.

use std::rc::Rc;

use crate::core::{BufferGeometry, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Matrix4, Sphere};
use crate::objects::Payload;

/// The state `Line` adds to `Object3D`: `geometry` and `material`, exactly as
/// `Mesh` has them. It lives in a node's [`Payload`](crate::objects::Payload) —
/// see `docs/scene-graph.md`.
#[derive(Clone)]
pub struct Line {
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
    /// `LineSegments.isLineSegments`. The single bit that separates the two
    /// subclasses for everything this port does with them.
    pub is_line_segments: bool,
}

impl Line {
    /// `new Line( geometry, material )`.
    ///
    /// three.js defaults the material to `new LineBasicMaterial()`, which under
    /// `WebGPURenderer` is a `LineBasicNodeMaterial`; the port takes it by value
    /// because every call site on the ladder passes one.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Node {
        Self::node(geometry, material, false)
    }

    /// The geometry half of `Line.intersectsFrustum( frustum )`, i.e.
    /// `frustum.intersectsObject( line )`: the bounding sphere pushed through
    /// `matrixWorld`. Identical to `Mesh`', because `Frustum.intersectsObject()`
    /// only ever reads `geometry.boundingSphere`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<Sphere> {
        let bounding_sphere = self.geometry.compute_bounding_sphere()?;
        let mut sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(matrix_world);
        Some(sphere)
    }

    fn node(
        geometry: Rc<BufferGeometry>,
        material: MeshBasicNodeMaterial,
        is_line_segments: bool,
    ) -> Node {
        let mut object = Object3D {
            object_type: if is_line_segments {
                "LineSegments"
            } else {
                "Line"
            },
            ..Default::default()
        };
        object.payload = Payload::Line(Self {
            geometry,
            material: Some(material),
            is_line_segments,
        });
        object.into_node()
    }
}

/// `class LineSegments extends Line` — a series of lines drawn between *pairs*
/// of vertices, i.e. `line-list` rather than `line-strip`. The only state it
/// adds is `isLineSegments`, so it is a constructor on [`Line`]'s payload
/// rather than a payload of its own.
pub struct LineSegments;

impl LineSegments {
    /// `new LineSegments( geometry, material )`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(geometry: Rc<BufferGeometry>, material: MeshBasicNodeMaterial) -> Node {
        Line::node(geometry, material, true)
    }
}
