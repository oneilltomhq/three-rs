//! Port of `three.js/src/helpers/PlaneHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::{LineBasicNodeMaterial, MeshBasicNodeMaterial};
use crate::math::{Color, Plane};
use crate::objects::{Line, Mesh};

/// `new PlaneHelper( plane, size = 1, hex = 0xffff00 )` — a `Line` tracing
/// the outline and one diagonal of a `size` × `size` square in the plane,
/// with a child `Mesh` filling the square at 20% opacity, both re-oriented
/// onto `plane` every `updateMatrixWorld()`.
///
/// # The `updateMatrixWorld` override
///
/// As for [`Box3Helper`](super::Box3Helper): three re-orients the helper in
/// an `updateMatrixWorld()` override, which the port's traversal cannot
/// call, so it is [`update_matrix_world`](Self::update_matrix_world) here.
/// Call it after the scene's own update. It reads the parent's world matrix
/// (`lookAt()` does), so it gives three's result once the parent is current.
///
/// `toneMapped: false` on both materials has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct PlaneHelper {
    /// The `Line` itself.
    pub node: Node,
    /// `PlaneHelper.plane`. Three holds the caller's `Plane` object; the port
    /// holds the value, so change the plane here.
    pub plane: Plane,
    /// `PlaneHelper.size`.
    pub size: f64,
}

impl PlaneHelper {
    /// `new PlaneHelper( plane, size, hex )`. Three's defaults are a size of
    /// 1 and `0xffff00`.
    pub fn new(plane: Plane, size: f64, color: Color) -> Self {
        #[rustfmt::skip]
        let positions = vec![
            1.0, -1.0, 0.0,  -1.0, 1.0, 0.0,  -1.0, -1.0, 0.0,  1.0, 1.0, 0.0,
            -1.0, 1.0, 0.0,  -1.0, -1.0, 0.0,  1.0, -1.0, 0.0,  1.0, 1.0, 0.0,
        ];

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(positions, 3));

        // `new LineBasicMaterial( { color: color, toneMapped: false } )`.
        let node = Line::new(Rc::new(geometry), LineBasicNodeMaterial::line(color));
        node.borrow_mut().object_type = "PlaneHelper";

        #[rustfmt::skip]
        let positions2 = vec![
            1.0, 1.0, 0.0,  -1.0, 1.0, 0.0,  -1.0, -1.0, 0.0,
            1.0, 1.0, 0.0,  -1.0, -1.0, 0.0,  1.0, -1.0, 0.0,
        ];

        let mut geometry2 = BufferGeometry::new();
        geometry2.set_attribute("position", BufferAttribute::new(positions2, 3));

        // `new MeshBasicMaterial( { color: color, opacity: 0.2, transparent:
        // true, depthWrite: false, toneMapped: false } )`.
        let material = MeshBasicNodeMaterial {
            color,
            opacity: 0.2,
            transparent: true,
            depth_write: false,
            ..MeshBasicNodeMaterial::new()
        };
        node.add(&Mesh::new(Rc::new(geometry2), material));

        Self { node, plane, size }
    }

    /// `PlaneHelper.updateMatrixWorld( force )`: at the origin, scaled to
    /// `size`, its +Z turned towards the point `plane.normal`, moved
    /// `-plane.constant` along that +Z, then
    /// `Object3D.updateMatrixWorld( force )`.
    pub fn update_matrix_world(&self, force: bool) {
        {
            let mut object = self.node.borrow_mut();
            object.position.set(0.0, 0.0, 0.0);
            object.scale.set(0.5 * self.size, 0.5 * self.size, 1.0);
        }

        self.node.look_at(&self.plane.normal);

        self.node.borrow_mut().translate_z(-self.plane.constant);

        self.node.update_matrix_world(force);
    }
}
