//! Port of `three.js/src/helpers/AxesHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::LineBasicNodeMaterial;
use crate::math::Color;
use crate::objects::LineSegments;

/// `new AxesHelper( size = 1 )` — a `LineSegments` of three lines from the
/// origin, `size` long along +X, +Y and +Z, drawn with vertex colours: each
/// axis runs from its full colour at the origin (red, green, blue) to a
/// lighter tint at its tip.
///
/// `toneMapped: false` has no counterpart, as in
/// [`GridHelper`](super::GridHelper): the port tone maps in the output pass.
pub struct AxesHelper {
    /// The `LineSegments` itself.
    pub node: Node,
}

impl AxesHelper {
    /// `new AxesHelper( size )`.
    pub fn new(size: f64) -> Self {
        let size = size as f32;

        #[rustfmt::skip]
        let vertices = vec![
            0.0, 0.0, 0.0,  size, 0.0, 0.0,
            0.0, 0.0, 0.0,  0.0, size, 0.0,
            0.0, 0.0, 0.0,  0.0, 0.0, size,
        ];

        #[rustfmt::skip]
        let colors = vec![
            1.0, 0.0, 0.0,  1.0, 0.6, 0.0,
            0.0, 1.0, 0.0,  0.6, 1.0, 0.0,
            0.0, 0.0, 1.0,  0.0, 0.6, 1.0,
        ];

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
        geometry.set_attribute("color", BufferAttribute::new(colors, 3));

        // `new LineBasicMaterial( { vertexColors: true, toneMapped: false } )`.
        let mut material = LineBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.vertex_colors = true;

        let node = LineSegments::new(Rc::new(geometry), material);
        node.borrow_mut().object_type = "AxesHelper";

        Self { node }
    }

    /// `AxesHelper.setColors( xAxisColor, yAxisColor, zAxisColor )`: both
    /// vertices of each axis take the one colour, so the tint at the tips is
    /// gone after the first call.
    pub fn set_colors(
        &self,
        x_axis_color: Color,
        y_axis_color: Color,
        z_axis_color: Color,
    ) -> &Self {
        let geometry = self
            .node
            .borrow()
            .geometry()
            .expect("three-rs: an AxesHelper is a LineSegments")
            .clone();
        let attribute = geometry
            .get_attribute("color")
            .expect("three-rs: the helper has a color attribute");
        {
            let mut array = attribute.array_mut();
            for (axis, color) in [x_axis_color, y_axis_color, z_axis_color]
                .into_iter()
                .enumerate()
            {
                let rgb = [color.r as f32, color.g as f32, color.b as f32];
                array[axis * 6..axis * 6 + 3].copy_from_slice(&rgb);
                array[axis * 6 + 3..axis * 6 + 6].copy_from_slice(&rgb);
            }
        }
        attribute.set_needs_update();

        self
    }
}

impl Default for AxesHelper {
    /// `new AxesHelper()`: `size = 1`.
    fn default() -> Self {
        Self::new(1.0)
    }
}
