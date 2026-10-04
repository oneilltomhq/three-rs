//! Port of `three.js/src/helpers/ArrowHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node, Object3D};
use crate::geometries::cone_geometry;
use crate::materials::{LineBasicNodeMaterial, MeshBasicNodeMaterial};
use crate::math::{Color, Vector3};
use crate::objects::{Line, Mesh};

thread_local! {
    /// The module's `_lineGeometry` and `_coneGeometry`: built by the first
    /// `ArrowHelper` and shared by every one after it, as three shares them.
    static GEOMETRIES: (Rc<BufferGeometry>, Rc<BufferGeometry>) = {
        let mut line = BufferGeometry::new();
        line.set_attribute(
            "position",
            BufferAttribute::new(vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0], 3),
        );

        let mut cone = cone_geometry(0.5, 1.0, 5, 1);
        cone.translate(0.0, -0.5, 0.0);

        (Rc::new(line), Rc::new(cone))
    };
}

/// `new ArrowHelper( dir, origin, length, color, headLength, headWidth )` —
/// a plain `Object3D` at `origin`, turned so its +Y is `dir`, holding a unit
/// `Line` up +Y (`line`) scaled to the shaft's length and a five-sided cone
/// (`cone`) scaled to the head and moved to the tip.
///
/// The two children have `matrixAutoUpdate = false`;
/// [`set_length`](Self::set_length) composes their matrices itself. Their geometries are
/// shared by every arrow on the thread, as three's module-level ones are.
///
/// The line and cone each get their own material, as in three, both with
/// `toneMapped: false` — which, as for [`GridHelper`](super::GridHelper), has
/// no counterpart in the port.
pub struct ArrowHelper {
    /// The `Object3D` itself.
    pub node: Node,
    /// `ArrowHelper.line`.
    pub line: Node,
    /// `ArrowHelper.cone`.
    pub cone: Node,
}

impl Default for ArrowHelper {
    /// `new ArrowHelper()`: +Z, from the origin, 1 long, yellow.
    fn default() -> Self {
        Self::new(
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, 0.0),
            1.0,
            Color::from_hex(0xffff00),
            None,
            None,
        )
    }
}

impl ArrowHelper {
    /// `new ArrowHelper( dir, origin, length, color, headLength, headWidth )`.
    /// `dir` must be normalized. `head_length` defaults to `length * 0.2` and
    /// `head_width` to `head_length * 0.2`.
    pub fn new(
        dir: Vector3,
        origin: Vector3,
        length: f64,
        color: Color,
        head_length: Option<f64>,
        head_width: Option<f64>,
    ) -> Self {
        let (line_geometry, cone_geometry) = GEOMETRIES.with(|g| (g.0.clone(), g.1.clone()));

        let node = Object3D {
            object_type: "ArrowHelper",
            position: origin,
            ..Default::default()
        }
        .into_node();

        // `new LineBasicMaterial( { color: color, toneMapped: false } )`.
        let line = Line::new(line_geometry, LineBasicNodeMaterial::line(color));
        line.borrow_mut().matrix_auto_update = false;
        node.add(&line);

        // `new MeshBasicMaterial( { color: color, toneMapped: false } )`.
        let cone_material = MeshBasicNodeMaterial {
            color,
            ..MeshBasicNodeMaterial::new()
        };
        let cone = Mesh::new(cone_geometry, cone_material);
        cone.borrow_mut().matrix_auto_update = false;
        node.add(&cone);

        let helper = Self { node, line, cone };
        helper.set_direction(&dir);
        helper.set_length(length, head_length, head_width);
        helper
    }

    /// `ArrowHelper.setDirection( dir )` — `dir` is assumed normalized. Within
    /// 1e-5 of ±Y the rotation is set outright; elsewhere it is the rotation
    /// about `( dir.z, 0, -dir.x )` by `acos( dir.y )`.
    pub fn set_direction(&self, dir: &Vector3) {
        let mut object = self.node.borrow_mut();

        if dir.y > 0.99999 {
            object.quaternion.set(0.0, 0.0, 0.0, 1.0);
        } else if dir.y < -0.99999 {
            object.quaternion.set(1.0, 0.0, 0.0, 0.0);
        } else {
            let mut axis = Vector3::new(dir.z, 0.0, -dir.x);
            axis.normalize();

            let radians = dir.y.acos();

            object.quaternion.set_from_axis_angle(&axis, radians);
        }

        object.sync_rotation_from_quaternion();
    }

    /// `ArrowHelper.setLength( length, headLength, headWidth )`, with the
    /// constructor's defaults for the two head sizes. The shaft is never
    /// shorter than 0.0001 (three.js #17458).
    pub fn set_length(&self, length: f64, head_length: Option<f64>, head_width: Option<f64>) {
        let head_length = head_length.unwrap_or(length * 0.2);
        let head_width = head_width.unwrap_or(head_length * 0.2);

        {
            let mut line = self.line.borrow_mut();
            line.scale.set(1.0, (length - head_length).max(0.0001), 1.0); // see #17458
            line.update_matrix();
        }

        let mut cone = self.cone.borrow_mut();
        cone.scale.set(head_width, head_length, head_width);
        cone.position.y = length;
        cone.update_matrix();
    }

    /// `ArrowHelper.setColor( color )` — both materials.
    pub fn set_color(&self, color: Color) {
        if let Some(material) = self
            .line
            .borrow_mut()
            .line_mut()
            .and_then(|line| line.material.as_mut())
        {
            material.color = color;
        }
        if let Some(material) = self
            .cone
            .borrow_mut()
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
        {
            material.color = color;
        }
    }
}
