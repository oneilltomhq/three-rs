//! Port of `three.js/src/helpers/HemisphereLightHelper.js`.

use std::f64::consts::PI;
use std::rc::Rc;

use crate::core::{BufferAttribute, Node, Object3D};
use crate::geometries::octahedron_geometry;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, Vector3};
use crate::objects::Mesh;

/// `new HemisphereLightHelper( light, size, color )` — a plain `Object3D`
/// sharing the light's world matrix, holding a wireframe octahedron of
/// radius `size` turned so that its first half of vertices faces the sky.
/// Without a `color` the octahedron is vertex-coloured: the light's sky
/// colour on the first half of its vertices, its ground colour on the rest.
///
/// Three sets `this.matrix = light.matrixWorld`; the port's
/// [`Object3D::matrix_alias`] is that sharing, as for
/// [`CameraHelper`](super::CameraHelper).
///
/// Three keeps the octahedron's material as `this.material` on the helper as
/// well; the port's material lives in the mesh, so reach it through
/// [`mesh`](Self::mesh). `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper). `wireframe` is set as three sets it.
pub struct HemisphereLightHelper {
    /// The `Object3D` itself.
    pub node: Node,
    /// `HemisphereLightHelper.light`.
    pub light: Node,
    /// `this.children[ 0 ]`, the octahedron.
    pub mesh: Node,
    /// `HemisphereLightHelper.color`: `None` takes the light's two colours.
    pub color: Option<Color>,
}

impl HemisphereLightHelper {
    /// `new HemisphereLightHelper( light, size, color )`. Three's
    /// `OctahedronGeometry` takes an undefined size as 1.
    ///
    /// Panics if `color` is `None` and `light` is not a light, here and on
    /// every [`update`](Self::update): only then does the helper read the
    /// light's colour.
    pub fn new(light: &Node, size: f64, color: Option<Color>) -> Self {
        let node = Object3D {
            object_type: "HemisphereLightHelper",
            matrix_alias: Some(light.downgrade()),
            matrix_auto_update: false,
            ..Default::default()
        }
        .into_node();

        let mut geometry = octahedron_geometry(size, 0);
        geometry.rotate_y(PI * 0.5);

        // `new MeshBasicMaterial( { wireframe: true, fog: false, toneMapped:
        // false } )`, vertex-coloured when there is no colour.
        let material = MeshBasicNodeMaterial {
            wireframe: true,
            fog: false,
            vertex_colors: color.is_none(),
            ..MeshBasicNodeMaterial::new()
        };

        let count = geometry
            .get_attribute("position")
            .expect("three-rs: an OctahedronGeometry has positions")
            .count();
        geometry.set_attribute("color", BufferAttribute::new(vec![0.0; count * 3], 3));

        let mesh = Mesh::new(Rc::new(geometry), material);
        node.add(&mesh);

        let helper = Self {
            node,
            light: light.clone(),
            mesh,
            color,
        };
        helper.update();
        helper
    }

    /// `HemisphereLightHelper.update()`: the colour (or the vertex colours
    /// from the light's sky and ground colours), then the octahedron turned
    /// to look away from the light's position.
    pub fn update(&self) {
        match self.color {
            Some(color) => {
                if let Some(material) = self
                    .mesh
                    .borrow_mut()
                    .mesh_mut()
                    .and_then(|mesh| mesh.material.as_mut())
                {
                    material.color = color;
                }
            }
            None => {
                let (color1, color2) = {
                    let light = self.light.borrow();
                    let light = light
                        .light()
                        .expect("three-rs: a HemisphereLightHelper's light is a light");
                    (light.light.color, light.ground_color)
                };

                let geometry = self
                    .mesh
                    .borrow()
                    .geometry()
                    .expect("three-rs: the helper's octahedron is a mesh")
                    .clone();
                let colors = geometry
                    .get_attribute("color")
                    .expect("three-rs: the octahedron has a color attribute");
                let l = colors.count();
                {
                    let mut array = colors.array_mut();
                    for i in 0..l {
                        let color = if (i as f64) < (l as f64 / 2.0) {
                            color1
                        } else {
                            color2
                        };
                        array[i * 3..i * 3 + 3].copy_from_slice(&[
                            color.r as f32,
                            color.g as f32,
                            color.b as f32,
                        ]);
                    }
                }
                colors.set_needs_update();
            }
        }

        self.node.borrow_mut().matrix_world_needs_update = true;

        self.light.update_world_matrix(true, false);

        let mut vector = Vector3::default();
        vector.set_from_matrix_position(&self.light.borrow().matrix_world);
        vector.negate();
        self.mesh.look_at(&vector);
    }
}
