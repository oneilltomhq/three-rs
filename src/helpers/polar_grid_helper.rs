//! Port of `three.js/src/helpers/PolarGridHelper.js`.

use std::f64::consts::PI;
use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, ObjectRef};
use crate::materials::LineBasicNodeMaterial;
use crate::math::Color;
use crate::objects::LineSegments;

/// `new PolarGridHelper( radius = 10, sectors = 16, rings = 8, divisions =
/// 64, color1 = 0x444444, color2 = 0x888888 )`.
///
/// A `LineSegments` in the XZ plane: `sectors` spokes from the centre (none
/// when `sectors` is 1 or less), then `rings` concentric circles of
/// `divisions` segments each, outermost first. Spokes and rings alternate
/// colours, `color2` for the even ones and `color1` for the odd, through a
/// `color` attribute and `vertexColors: true`.
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct PolarGridHelper;

impl PolarGridHelper {
    /// `new PolarGridHelper( radius, sectors, rings, divisions, color1,
    /// color2 )`, as a scene-graph node.
    #[allow(clippy::new_ret_no_self)] // mirrors three.js's constructor: a scene-graph `ObjectRef`, not `Self`.
    pub fn new(
        radius: f64,
        sectors: usize,
        rings: usize,
        divisions: usize,
        color1: Color,
        color2: Color,
    ) -> ObjectRef {
        let mut vertices: Vec<f32> = Vec::new();
        let mut colors: Vec<f32> = Vec::new();

        // `( i & 1 ) ? color1 : color2`
        let pick = |i: usize| if i & 1 == 1 { color1 } else { color2 };
        let push_color = |colors: &mut Vec<f32>, color: Color| {
            colors.extend_from_slice(&[color.r as f32, color.g as f32, color.b as f32]);
        };

        // create the sectors

        if sectors > 1 {
            for i in 0..sectors {
                let v = (i as f64 / sectors as f64) * (PI * 2.0);

                let x = v.sin() * radius;
                let z = v.cos() * radius;

                vertices.extend_from_slice(&[0.0, 0.0, 0.0]);
                vertices.extend_from_slice(&[x as f32, 0.0, z as f32]);

                let color = pick(i);

                push_color(&mut colors, color);
                push_color(&mut colors, color);
            }
        }

        // create the rings

        for i in 0..rings {
            let color = pick(i);

            let r = radius - (radius / rings as f64 * i as f64);

            for j in 0..divisions {
                // first vertex

                let v = (j as f64 / divisions as f64) * (PI * 2.0);

                let x = v.sin() * r;
                let z = v.cos() * r;

                vertices.extend_from_slice(&[x as f32, 0.0, z as f32]);
                push_color(&mut colors, color);

                // second vertex

                let v = ((j + 1) as f64 / divisions as f64) * (PI * 2.0);

                let x = v.sin() * r;
                let z = v.cos() * r;

                vertices.extend_from_slice(&[x as f32, 0.0, z as f32]);
                push_color(&mut colors, color);
            }
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(vertices, 3));
        geometry.set_attribute("color", BufferAttribute::new(colors, 3));

        // `new LineBasicMaterial( { vertexColors: true, toneMapped: false } )`.
        let mut material = LineBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.vertex_colors = true;

        let node = LineSegments::new(Rc::new(geometry), material);
        node.borrow_mut().object_type = "PolarGridHelper";
        node
    }
}
