//! Port of `three.js/src/helpers/GridHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::objects::LineSegments;

/// `new GridHelper( size, divisions, color1, color2 )`.
///
/// A `LineSegments` whose geometry is `( divisions + 1 ) * 2` segments — one
/// run along each axis per division line — with a three-component `color`
/// attribute and `vertexColors: true` on the material. The centre line takes
/// `color1`, every other line `color2`; `webgpu_materials` passes the same
/// colour twice, so the whole grid is one hue.
///
/// `toneMapped: false` on three's material has no counterpart here: the port's
/// tone mapping is applied in the output pass, from the renderer's setting, and
/// `webgpu_materials` uses `NoToneMapping` anyway. It is a divergence only on a
/// page that tone maps, and none on the ladder does.
pub struct GridHelper;

impl GridHelper {
    /// Builds the helper's geometry and material and returns the scene-graph
    /// node, exactly as three's constructor does.
    #[allow(clippy::new_ret_no_self)] // mirrors three.js's constructor: a scene-graph `Node`, not `Self`.
    pub fn new(size: f64, divisions: usize, color1: Color, color2: Color) -> Node {
        let (geometry, material) = Self::parts(size, divisions, color1, color2);
        let node = LineSegments::new(Rc::new(geometry), material);
        node.borrow_mut().object_type = "GridHelper";
        node
    }

    /// The constructor's two products, for a unit test that wants the buffers
    /// without a scene.
    pub fn parts(
        size: f64,
        divisions: usize,
        color1: Color,
        color2: Color,
    ) -> (BufferGeometry, MeshBasicNodeMaterial) {
        let center = divisions / 2;
        let step = size / divisions as f64;
        let half_size = size / 2.0;

        let mut vertices: Vec<f32> = Vec::with_capacity((divisions + 1) * 12);
        let mut colors: Vec<f32> = Vec::with_capacity((divisions + 1) * 12);

        let mut k_acc = -half_size;
        for i in 0..=divisions {
            // `k = - halfSize; … k += step` — accumulated, not `i * step`, so
            // the rounding matches three's vertex for vertex.
            let k = k_acc as f32;
            let half = half_size as f32;
            vertices.extend_from_slice(&[-half, 0.0, k, half, 0.0, k]);
            vertices.extend_from_slice(&[k, 0.0, -half, k, 0.0, half]);

            // `i === center ? color1 : color2`. `divisions / 2` is JS' float
            // division, so an odd `divisions` never matches an integer `i` and
            // `color1` is simply unused — the integer division here agrees for
            // even counts and differs only where three's centre line does not
            // exist at all.
            let color = if divisions.is_multiple_of(2) && i == center {
                color1
            } else {
                color2
            };
            for _ in 0..4 {
                colors.extend_from_slice(&[color.r as f32, color.g as f32, color.b as f32]);
            }

            k_acc += step;
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
        geometry.set_attribute("color", BufferAttribute::new(colors, 3));

        // `new LineBasicMaterial( { vertexColors: true, toneMapped: false } )`.
        let mut material = MeshBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.vertex_colors = true;

        (geometry, material)
    }
}
