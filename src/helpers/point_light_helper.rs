//! Port of `three.js/src/helpers/PointLightHelper.js`.

use std::rc::Rc;

use crate::core::ObjectRef;
use crate::geometries::sphere_geometry;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::Color;
use crate::objects::Mesh;

use super::directional_light_helper::light_color;

/// `new PointLightHelper( light, sphereSize, color )` — a wireframe `Mesh`
/// of a coarse sphere (four segments around, two down) of radius
/// `sphere_size`, sharing the light's world matrix, in the light's colour
/// unless given one.
///
/// Three sets `this.matrix = light.matrixWorld`; the port's
/// [`Object3D::matrix_alias`](crate::core::Object3D::matrix_alias) is that
/// sharing, as for [`CameraHelper`](super::CameraHelper).
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper). Three's commented-out light-distance
/// sphere is not ported either.
pub struct PointLightHelper {
    /// The `Mesh` itself.
    pub node: ObjectRef,
    /// `PointLightHelper.light`.
    pub light: ObjectRef,
    /// `PointLightHelper.color`: `None` follows the light's colour.
    pub color: Option<Color>,
}

impl PointLightHelper {
    /// `new PointLightHelper( light, sphereSize, color )`. Three's
    /// `SphereGeometry` takes an undefined size as 1.
    ///
    /// Panics if `color` is `None` and `light` is not a light, here and on
    /// every [`update`](Self::update): only then does the helper read the
    /// light's colour.
    pub fn new(light: &ObjectRef, sphere_size: f64, color: Option<Color>) -> Self {
        let geometry = sphere_geometry(sphere_size, 4, 2);
        // `new MeshBasicMaterial( { wireframe: true, fog: false, toneMapped:
        // false } )`.
        let material = MeshBasicNodeMaterial {
            wireframe: true,
            fog: false,
            ..MeshBasicNodeMaterial::new()
        };

        let node = Mesh::new(Rc::new(geometry), material);
        {
            let mut object = node.borrow_mut();
            object.object_type = "PointLightHelper";
            // `this.matrix = this.light.matrixWorld; this.matrixAutoUpdate =
            // false`.
            object.matrix_alias = Some(light.downgrade());
            object.matrix_auto_update = false;
        }

        let helper = Self {
            node,
            light: light.clone(),
            color,
        };
        helper.update();
        helper
    }

    /// `PointLightHelper.update()`: brings the light's world matrix up to
    /// date and sets the colour.
    pub fn update(&self) {
        self.node.borrow_mut().matrix_world_needs_update = true;

        self.light.update_world_matrix(true, false);

        let color = self.color.unwrap_or_else(|| light_color(&self.light));
        if let Some(material) = self
            .node
            .borrow_mut()
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
        {
            material.color = color;
        }
    }
}
