//! Port of `three.js/src/helpers/BoxHelper.js`.

use std::cell::Cell;
use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::LineBasicNodeMaterial;
use crate::math::{Box3, Color};
use crate::objects::LineSegments;

thread_local! {
    /// The module's `_box`. Shared, as in three: a helper with no object
    /// draws whatever box the last `update()` on the thread computed.
    static BOX: Cell<Box3> = Cell::new(Box3::default());
}

/// `new BoxHelper( object, color = 0xffff00 )` — a `LineSegments` drawing
/// the twelve edges of `object`'s world-axis-aligned bounding box (children
/// included), as an indexed eight-vertex geometry.
///
/// The box is measured in world space, so the helper's own matrix stays the
/// identity (`matrixAutoUpdate = false`): add it to the scene root, or to a
/// parent with no transform. It is measured only by [`update`](Self::update),
/// which the constructor calls once; `Box3.setFromObject()` updates the
/// world matrix of each object it measures but not their parents', so an
/// object whose ancestors have never been updated is measured as if they
/// had no transform — exactly as three does it.
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct BoxHelper {
    /// The `LineSegments` itself.
    pub node: Node,
    /// `BoxHelper.object`.
    pub object: Option<Node>,
}

impl BoxHelper {
    /// `new BoxHelper( object, color )`. Three's default colour is
    /// `0xffff00`.
    pub fn new(object: Option<Node>, color: Color) -> Self {
        #[rustfmt::skip]
        let indices = [
            0, 1, 1, 2, 2, 3, 3, 0,
            4, 5, 5, 6, 6, 7, 7, 4,
            0, 4, 1, 5, 2, 6, 3, 7,
        ];

        let mut geometry = BufferGeometry::new();
        geometry.set_index(&indices);
        geometry.set_attribute("position", BufferAttribute::new(vec![0.0; 8 * 3], 3));

        // `new LineBasicMaterial( { color: color, toneMapped: false } )`.
        let node = LineSegments::new(Rc::new(geometry), LineBasicNodeMaterial::line(color));
        {
            let mut helper = node.borrow_mut();
            helper.object_type = "BoxHelper";
            helper.matrix_auto_update = false;
        }

        let helper = Self { node, object };
        helper.update();
        helper
    }

    /// `BoxHelper.update()`: measures the object, if there is one, and writes
    /// the box's corners into the geometry. An empty box leaves the geometry
    /// as it was.
    pub fn update(&self) {
        let mut bounds = BOX.with(Cell::get);
        if let Some(object) = &self.object {
            bounds.set_from_object(object, false);
            BOX.with(|b| b.set(bounds));
        }

        if bounds.is_empty() {
            return;
        }

        let (min, max) = (bounds.min, bounds.max);

        /*
          5____4
        1/___0/|
        | 6__|_7
        2/___3/

        0: max.x, max.y, max.z
        1: min.x, max.y, max.z
        2: min.x, min.y, max.z
        3: max.x, min.y, max.z
        4: max.x, max.y, min.z
        5: min.x, max.y, min.z
        6: min.x, min.y, min.z
        7: max.x, min.y, min.z
        */

        #[rustfmt::skip]
        let corners = [
            max.x, max.y, max.z,
            min.x, max.y, max.z,
            min.x, min.y, max.z,
            max.x, min.y, max.z,
            max.x, max.y, min.z,
            min.x, max.y, min.z,
            min.x, min.y, min.z,
            max.x, min.y, min.z,
        ];

        let geometry = self
            .node
            .borrow()
            .geometry()
            .expect("three-rs: a BoxHelper is a LineSegments")
            .clone();
        let position = geometry
            .get_attribute("position")
            .expect("three-rs: the helper has a position attribute");
        {
            let mut array = position.array_mut();
            for (slot, value) in array.iter_mut().zip(corners) {
                *slot = value as f32;
            }
        }
        // `geometry.computeBoundingSphere()` follows in three; the port's
        // bounds are recomputed from the attribute's new version on demand.
        position.set_needs_update();
    }

    /// `BoxHelper.setFromObject( object )`.
    pub fn set_from_object(&mut self, object: Node) -> &mut Self {
        self.object = Some(object);
        self.update();
        self
    }
}

impl Default for BoxHelper {
    /// `new BoxHelper()`: no object yet, `color = 0xffff00`.
    fn default() -> Self {
        Self::new(None, Color::from_hex(0xffff00))
    }
}
