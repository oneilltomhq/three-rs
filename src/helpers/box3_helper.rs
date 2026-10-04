//! Port of `three.js/src/helpers/Box3Helper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::LineBasicNodeMaterial;
use crate::math::{Box3, Color};
use crate::objects::LineSegments;

/// `new Box3Helper( box, color = 0xffff00 )` — a `LineSegments` of the
/// twelve edges of the cube from -1 to 1, placed and scaled onto `box` every
/// `updateMatrixWorld()`.
///
/// # The `updateMatrixWorld` override
///
/// Three moves the helper onto the box in an `updateMatrixWorld()`
/// override, so the scene's own traversal keeps it on the box. The port's
/// traversal ([`Node::update_matrix_world`]) has no per-type overrides, so
/// the override is [`update_matrix_world`](Self::update_matrix_world) here,
/// and something has to call it: call it after the scene's own update (or
/// before handing the scene to the renderer, whose update then recomputes
/// the same matrix). An empty box returns before the world matrix is
/// touched, in three and here; but the scene's traversal, which three would
/// skip for it, still updates the node's world matrix from its last local
/// one.
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct Box3Helper {
    /// The `LineSegments` itself.
    pub node: Node,
    /// `Box3Helper.box`, read on every
    /// [`update_matrix_world`](Self::update_matrix_world). Three holds the
    /// caller's `Box3` object; the port holds the value, so change the box
    /// here.
    pub box3: Box3,
}

impl Box3Helper {
    /// `new Box3Helper( box, color )`. Three's default colour is `0xffff00`.
    pub fn new(box3: Box3, color: Color) -> Self {
        #[rustfmt::skip]
        let indices = [
            0, 1, 1, 2, 2, 3, 3, 0,
            4, 5, 5, 6, 6, 7, 7, 4,
            0, 4, 1, 5, 2, 6, 3, 7,
        ];

        #[rustfmt::skip]
        let positions = vec![
            1.0, 1.0, 1.0,  -1.0, 1.0, 1.0,  -1.0, -1.0, 1.0,  1.0, -1.0, 1.0,
            1.0, 1.0, -1.0,  -1.0, 1.0, -1.0,  -1.0, -1.0, -1.0,  1.0, -1.0, -1.0,
        ];

        let mut geometry = BufferGeometry::new();
        geometry.set_index(&indices);
        geometry.set_attribute("position", BufferAttribute::new(positions, 3));
        // `this.geometry.computeBoundingSphere()`: the port computes bounds on
        // demand.

        // `new LineBasicMaterial( { color: color, toneMapped: false } )`.
        let node = LineSegments::new(Rc::new(geometry), LineBasicNodeMaterial::line(color));
        node.borrow_mut().object_type = "Box3Helper";

        Self { node, box3 }
    }

    /// `Box3Helper.updateMatrixWorld( force )`: centre and half-size from
    /// the box, then `Object3D.updateMatrixWorld( force )`. Does nothing for
    /// an empty box.
    pub fn update_matrix_world(&self, force: bool) {
        let bounds = self.box3;

        if bounds.is_empty() {
            return;
        }

        {
            let mut object = self.node.borrow_mut();
            object.position = bounds.get_center();
            object.scale = bounds.get_size();
            object.scale.multiply_scalar(0.5);
        }

        self.node.update_matrix_world(force);
    }
}
