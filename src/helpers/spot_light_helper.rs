//! Port of `three.js/src/helpers/SpotLightHelper.js`.

use std::f64::consts::PI;
use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node, Object3D};
use crate::materials::LineBasicNodeMaterial;
use crate::math::{Color, Vector3};
use crate::objects::LineSegments;

use super::directional_light_helper::light_color;

/// `new SpotLightHelper( light, color )` — a plain `Object3D` placed on the
/// light, holding a `LineSegments` cone (`cone`): five lines from the apex
/// and a 32-segment circle at unit distance, scaled to the light's `angle`
/// and `distance` (1000 when the distance is 0) and turned towards its
/// target.
///
/// Unlike the other light helpers this one does not share the light's world
/// matrix: [`update`](Self::update) copies it into the helper's `matrix`,
/// taking the parent's world matrix out first when the helper has a parent —
/// so a helper built before it is added to a transformed parent sits in the
/// wrong place until its next `update()`, in three and here.
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct SpotLightHelper {
    /// The `Object3D` itself.
    pub node: Node,
    /// `SpotLightHelper.light`.
    pub light: Node,
    /// `SpotLightHelper.cone`.
    pub cone: Node,
    /// `SpotLightHelper.color`: `None` follows the light's colour.
    pub color: Option<Color>,
}

impl SpotLightHelper {
    /// `new SpotLightHelper( light, color )`.
    ///
    /// Panics if `light` is not a light with a target.
    pub fn new(light: &Node, color: Option<Color>) -> Self {
        let node = Object3D {
            object_type: "SpotLightHelper",
            matrix_auto_update: false,
            ..Default::default()
        }
        .into_node();

        #[rustfmt::skip]
        let mut positions: Vec<f32> = vec![
            0.0, 0.0, 0.0,  0.0, 0.0, 1.0,
            0.0, 0.0, 0.0,  1.0, 0.0, 1.0,
            0.0, 0.0, 0.0,  -1.0, 0.0, 1.0,
            0.0, 0.0, 0.0,  0.0, 1.0, 1.0,
            0.0, 0.0, 0.0,  0.0, -1.0, 1.0,
        ];

        let l = 32;
        for i in 0..l {
            let j = i + 1;
            // `( i / l ) * Math.PI * 2`, in that order.
            let p1 = (i as f64 / l as f64) * PI * 2.0;
            let p2 = (j as f64 / l as f64) * PI * 2.0;

            positions.extend_from_slice(&[
                p1.cos() as f32,
                p1.sin() as f32,
                1.0,
                p2.cos() as f32,
                p2.sin() as f32,
                1.0,
            ]);
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(positions, 3));

        // `new LineBasicMaterial( { fog: false, toneMapped: false } )`.
        let mut material = LineBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.fog = false;

        let cone = LineSegments::new(Rc::new(geometry), material);
        node.add(&cone);

        let helper = Self {
            node,
            light: light.clone(),
            cone,
            color,
        };
        helper.update();
        helper
    }

    /// `SpotLightHelper.update()`: the helper's matrix from the light's (in
    /// the parent's space), the cone scaled to the light's angle and distance
    /// and turned towards the target, and the colour.
    pub fn update(&self) {
        self.light.update_world_matrix(true, false);
        let target = self
            .light
            .borrow()
            .light()
            .and_then(|light| light.target.clone())
            .expect("three-rs: a SpotLightHelper's light has a target");
        target.update_world_matrix(true, false);

        let light_matrix_world = self.light.borrow().matrix_world;

        // update the local matrix based on the parent and light target transforms
        match self.node.parent() {
            Some(parent) => {
                parent.update_world_matrix(true, false);

                let mut matrix = parent.borrow().matrix_world;
                matrix.invert();
                matrix.multiply(&light_matrix_world);
                self.node.borrow_mut().matrix = matrix;
            }
            None => self.node.borrow_mut().matrix = light_matrix_world,
        }

        self.node.borrow_mut().matrix_world_needs_update = true;

        let (distance, angle) = {
            let light = self.light.borrow();
            let light = light
                .light()
                .expect("three-rs: a SpotLightHelper's light is a light");
            (light.distance, light.angle)
        };

        // `this.light.distance ? this.light.distance : 1000` — 0 and NaN are
        // falsy.
        let cone_length = if distance != 0.0 && !distance.is_nan() {
            distance
        } else {
            1000.0
        };
        let cone_width = cone_length * angle.tan();

        self.cone
            .borrow_mut()
            .scale
            .set(cone_width, cone_width, cone_length);

        let mut vector = Vector3::default();
        vector.set_from_matrix_position(&target.borrow().matrix_world);

        self.cone.look_at(&vector);

        let color = self.color.unwrap_or_else(|| light_color(&self.light));
        if let Some(material) = self
            .cone
            .borrow_mut()
            .line_mut()
            .and_then(|line| line.material.as_mut())
        {
            material.color = color;
        }
    }
}
