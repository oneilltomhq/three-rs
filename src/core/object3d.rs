//! Port of `three.js/src/core/Object3D.js` (rung 1 subset).

use crate::math::{Euler, Matrix4, Quaternion, Vector3};

#[derive(Clone, Debug)]
pub struct Object3D {
    pub position: Vector3,
    /// Kept in sync with `quaternion` by [`Object3D::set_rotation`], the same way
    /// three.js' `Euler`/`Quaternion` `onChange` callbacks keep them in sync.
    pub rotation: Euler,
    pub quaternion: Quaternion,
    pub scale: Vector3,
    pub up: Vector3,
    pub matrix: Matrix4,
    pub matrix_world: Matrix4,
}

impl Default for Object3D {
    fn default() -> Self {
        Self {
            position: Vector3::ZERO,
            rotation: Euler::default(),
            quaternion: Quaternion::default(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            // `Object3D.DEFAULT_UP`
            up: Vector3::new(0.0, 1.0, 0.0),
            matrix: Matrix4::identity(),
            matrix_world: Matrix4::identity(),
        }
    }
}

impl Object3D {
    /// `object.rotation.set( x, y, z )` — the Euler `onChange` callback then
    /// refreshes the quaternion, which is what `updateMatrix()` composes from.
    pub fn set_rotation(&mut self, x: f64, y: f64, z: f64) {
        self.rotation.set(x, y, z);
        self.quaternion.set_from_euler(&self.rotation);
    }

    /// `Object3D.updateMatrix()`.
    pub fn update_matrix(&mut self) {
        let (position, quaternion, scale) = (self.position, self.quaternion, self.scale);
        self.matrix.compose(&position, &quaternion, &scale);
    }

    /// `Object3D.updateMatrixWorld()` for an object whose parent's world matrix
    /// is `parent_matrix_world` (`None` for a root).
    pub fn update_matrix_world(&mut self, parent_matrix_world: Option<&Matrix4>) {
        self.update_matrix();

        match parent_matrix_world {
            None => self.matrix_world = self.matrix,
            Some(parent) => {
                let matrix = self.matrix;
                self.matrix_world.multiply_matrices(parent, &matrix);
            }
        }
    }
}
