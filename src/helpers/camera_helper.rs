//! Port of `three.js/src/helpers/CameraHelper.js`.

use std::collections::HashMap;
use std::rc::Rc;

use crate::cameras::RenderCamera;
use crate::core::{BufferAttribute, BufferGeometry, ObjectRef};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, CoordinateSystem, Vector3};
use crate::objects::LineSegments;

/// `new CameraHelper( camera )` — a `LineSegments` of the camera's frustum,
/// cone, up triangle, target line and cross hairs, in the camera's local
/// space, drawn with vertex colours.
///
/// Three sets `this.matrix = camera.matrixWorld` — the helper *shares* the
/// camera's world matrix object — with `matrixAutoUpdate = false`, so every
/// `updateMatrixWorld()` traversal that reaches the helper multiplies in
/// whatever the camera's world matrix holds at that moment. The port's
/// [`Object3D::matrix_alias`](crate::core::Object3D::matrix_alias) is that
/// sharing, read at the same point of the traversal; `webgpu_camera` depends
/// on it, since the helper sits before the camera in the scene and so sees the
/// camera's matrix from the previous traversal.
///
/// A camera with no scene-graph node (the port's `OrthographicCamera`) cannot
/// be aliased; its helper takes a copy of the camera's world matrix instead.
pub struct CameraHelper {
    /// The `LineSegments` itself.
    pub node: ObjectRef,
    /// `this.pointMap` — each named point's vertex indices.
    point_map: HashMap<&'static str, Vec<usize>>,
}

/// The constructor's `addLine()` calls, in order.
const LINES: [(&str, &str); 25] = [
    // near
    ("n1", "n2"),
    ("n2", "n4"),
    ("n4", "n3"),
    ("n3", "n1"),
    // far
    ("f1", "f2"),
    ("f2", "f4"),
    ("f4", "f3"),
    ("f3", "f1"),
    // sides
    ("n1", "f1"),
    ("n2", "f2"),
    ("n3", "f3"),
    ("n4", "f4"),
    // cone
    ("p", "n1"),
    ("p", "n2"),
    ("p", "n3"),
    ("p", "n4"),
    // up
    ("u1", "u2"),
    ("u2", "u3"),
    ("u3", "u1"),
    // target
    ("c", "t"),
    ("p", "c"),
    // cross
    ("cn1", "cn2"),
    ("cn3", "cn4"),
    ("cf1", "cf2"),
    ("cf3", "cf4"),
];

impl CameraHelper {
    /// `new CameraHelper( camera )`.
    ///
    /// Three's constructor calls `camera.updateProjectionMatrix()` first; both
    /// of the port's cameras compute their projection in `new` and on every
    /// `update_projection_matrix()`, so the call is already made by the time a
    /// helper can be built for them.
    pub fn new(camera: &dyn RenderCamera) -> Self {
        let mut vertices: Vec<f32> = Vec::new();
        let mut colors: Vec<f32> = Vec::new();
        let mut point_map: HashMap<&'static str, Vec<usize>> = HashMap::new();

        let mut add_point = |id: &'static str| {
            vertices.extend([0.0, 0.0, 0.0]);
            colors.extend([0.0, 0.0, 0.0]);
            point_map
                .entry(id)
                .or_default()
                .push(vertices.len() / 3 - 1);
        };
        for (a, b) in LINES {
            add_point(a);
            add_point(b);
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
        geometry.set_attribute("color", BufferAttribute::new(colors, 3));

        // `new LineBasicMaterial( { color: 0xffffff, vertexColors: true,
        // toneMapped: false } )`. `toneMapped` has no counterpart, as in
        // `GridHelper`: the port tone maps in the output pass.
        let mut material = MeshBasicNodeMaterial::line(Color::from_hex(0xffffff));
        material.vertex_colors = true;

        let node = LineSegments::new(Rc::new(geometry), material);
        {
            let mut object = node.borrow_mut();
            object.object_type = "CameraHelper";
            // `this.matrix = camera.matrixWorld; this.matrixAutoUpdate = false`.
            match camera.node() {
                Some(camera_node) => object.matrix_alias = Some(camera_node.downgrade()),
                None => object.matrix = camera.matrix_world(),
            }
            object.matrix_auto_update = false;
        }

        let helper = Self { node, point_map };
        helper.update(camera);

        // colors
        helper.set_colors(
            Color::from_hex(0xffaa00),
            Color::from_hex(0xff0000),
            Color::from_hex(0x00aaff),
            Color::from_hex(0xffffff),
            Color::from_hex(0x333333),
        );

        helper
    }

    fn geometry(&self) -> Rc<BufferGeometry> {
        self.node
            .borrow()
            .geometry()
            .expect("three-rs: a CameraHelper is a LineSegments")
            .clone()
    }

    /// `CameraHelper.setColors( frustum, cone, up, target, cross )` — vertices
    /// 0–23 frustum, 24–31 cone, 32–37 up, 38–39 target, 40–49 cross (the
    /// `p`–`c` line included).
    pub fn set_colors(&self, frustum: Color, cone: Color, up: Color, target: Color, cross: Color) {
        let geometry = self.geometry();
        let color_attribute = geometry
            .get_attribute("color")
            .expect("three-rs: the helper has a color attribute");
        {
            let mut array = color_attribute.array_mut();
            for i in 0..50 {
                let color = match i {
                    0..=23 => frustum,
                    24..=31 => cone,
                    32..=37 => up,
                    38..=39 => target,
                    _ => cross,
                };
                array[i * 3..i * 3 + 3].copy_from_slice(&[
                    color.r as f32,
                    color.g as f32,
                    color.b as f32,
                ]);
            }
        }
        color_attribute.set_needs_update();
    }

    /// `CameraHelper.update()`: every named point un-projected from NDC
    /// through the camera's `projectionMatrixInverse` alone (three's `_camera`
    /// has an identity world matrix — the helper's own matrix places it).
    ///
    /// `camera.reversedDepth` is never set on the port's cameras, so the near
    /// and far planes are the coordinate system's: `-1`/`1` under WebGL, `0`/`1`
    /// under WebGPU.
    pub fn update(&self, camera: &dyn RenderCamera) {
        let (w, h) = (1.0, 1.0);

        let (near_z, far_z) = match camera.coordinate_system() {
            CoordinateSystem::WebGl => (-1.0, 1.0),
            CoordinateSystem::WebGpu => (0.0, 1.0),
        };

        let projection_matrix_inverse = camera.projection_matrix_inverse();
        let geometry = self.geometry();
        let position = geometry
            .get_attribute("position")
            .expect("three-rs: the helper has a position attribute");

        let points: [(&str, f64, f64, f64); 21] = [
            // center / target
            ("c", 0.0, 0.0, near_z),
            ("t", 0.0, 0.0, far_z),
            // near
            ("n1", -w, -h, near_z),
            ("n2", w, -h, near_z),
            ("n3", -w, h, near_z),
            ("n4", w, h, near_z),
            // far
            ("f1", -w, -h, far_z),
            ("f2", w, -h, far_z),
            ("f3", -w, h, far_z),
            ("f4", w, h, far_z),
            // up
            ("u1", w * 0.7, h * 1.1, near_z),
            ("u2", -w * 0.7, h * 1.1, near_z),
            ("u3", 0.0, h * 2.0, near_z),
            // cross
            ("cf1", -w, 0.0, far_z),
            ("cf2", w, 0.0, far_z),
            ("cf3", 0.0, -h, far_z),
            ("cf4", 0.0, h, far_z),
            ("cn1", -w, 0.0, near_z),
            ("cn2", w, 0.0, near_z),
            ("cn3", 0.0, -h, near_z),
            ("cn4", 0.0, h, near_z),
        ];
        // `p` is never set: its vertices stay at the origin, the camera's eye.

        {
            let mut array = position.array_mut();
            for (point, x, y, z) in points {
                // `setPoint()`: `_vector.set( x, y, z ).unproject( _camera )`.
                let Some(indices) = self.point_map.get(point) else {
                    continue;
                };
                let mut vector = Vector3::new(x, y, z);
                vector.apply_matrix4(&projection_matrix_inverse);
                for &i in indices {
                    array[i * 3..i * 3 + 3].copy_from_slice(&[
                        vector.x as f32,
                        vector.y as f32,
                        vector.z as f32,
                    ]);
                }
            }
        }
        position.set_needs_update();
    }
}
