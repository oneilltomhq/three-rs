//! Port of `three.js/src/helpers/DirectionalLightHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Object3D, ObjectRef};
use crate::materials::LineBasicNodeMaterial;
use crate::math::{Color, Vector3};
use crate::objects::Line;

/// `new DirectionalLightHelper( light, size = 1, color )` — a plain
/// `Object3D` that shares the light's world matrix, holding a `size` × `size`
/// square outline (`light_plane`) and a unit line (`target_line`), both
/// turned towards the light's target, the line stretched to reach it.
///
/// Three sets `this.matrix = light.matrixWorld`; the port's
/// [`Object3D::matrix_alias`] is that sharing, as for
/// [`CameraHelper`](super::CameraHelper). Without a `color` the lines take
/// the light's colour at each [`update`](Self::update).
///
/// Three gives both lines the *same* `LineBasicMaterial`. A port material
/// lives in its object, so each line has its own copy, and
/// [`update`](Self::update) writes the colour into both, which is all three
/// ever changes on it. `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct DirectionalLightHelper {
    /// The `Object3D` itself.
    pub node: ObjectRef,
    /// `DirectionalLightHelper.light`.
    pub light: ObjectRef,
    /// `DirectionalLightHelper.lightPlane`.
    pub light_plane: ObjectRef,
    /// `DirectionalLightHelper.targetLine`.
    pub target_line: ObjectRef,
    /// `DirectionalLightHelper.color`: `None` follows the light's colour.
    pub color: Option<Color>,
}

impl DirectionalLightHelper {
    /// `new DirectionalLightHelper( light, size, color )`. Three's default
    /// size is 1.
    ///
    /// Panics if `light` is not a light with a target.
    pub fn new(light: &ObjectRef, size: f64, color: Option<Color>) -> Self {
        let node = Object3D {
            object_type: "DirectionalLightHelper",
            matrix_alias: Some(light.downgrade()),
            matrix_auto_update: false,
            ..Default::default()
        }
        .into_node();

        let s = size as f32;
        #[rustfmt::skip]
        let positions = vec![
            -s, s, 0.0,
            s, s, 0.0,
            s, -s, 0.0,
            -s, -s, 0.0,
            -s, s, 0.0,
        ];
        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(positions, 3));

        // `new LineBasicMaterial( { fog: false, toneMapped: false } )`.
        let mut material = LineBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.fog = false;

        let light_plane = Line::new(Rc::new(geometry), material.clone());
        node.add(&light_plane);

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute(
            "position",
            BufferAttribute::new(vec![0.0, 0.0, 0.0, 0.0, 0.0, 1.0], 3),
        );

        let target_line = Line::new(Rc::new(geometry), material);
        node.add(&target_line);

        let helper = Self {
            node,
            light: light.clone(),
            light_plane,
            target_line,
            color,
        };
        helper.update();
        helper
    }

    /// `DirectionalLightHelper.update()`: brings the light's and its target's
    /// world matrices up to date, turns both lines towards the target,
    /// stretches the target line to the distance between them, and sets the
    /// colour.
    pub fn update(&self) {
        self.node.borrow_mut().matrix_world_needs_update = true;

        self.light.update_world_matrix(true, false);
        let target = self
            .light
            .borrow()
            .light()
            .and_then(|light| light.target.clone())
            .expect("three-rs: a DirectionalLightHelper's light has a target");
        target.update_world_matrix(true, false);

        let mut v1 = Vector3::default();
        v1.set_from_matrix_position(&self.light.borrow().matrix_world);
        let mut v2 = Vector3::default();
        v2.set_from_matrix_position(&target.borrow().matrix_world);
        let mut v3 = Vector3::default();
        v3.sub_vectors(&v2, &v1);

        self.light_plane.look_at(&v2);

        let color = self.color.unwrap_or_else(|| light_color(&self.light));
        for line in [&self.light_plane, &self.target_line] {
            if let Some(material) = line
                .borrow_mut()
                .line_mut()
                .and_then(|line| line.material.as_mut())
            {
                material.color = color;
            }
        }

        self.target_line.look_at(&v2);
        self.target_line.borrow_mut().scale.z = v3.length();
    }
}

/// `light.color`.
pub(super) fn light_color(light: &ObjectRef) -> Color {
    light
        .borrow()
        .light()
        .expect("three-rs: a light helper's light is a light")
        .light
        .color
}
