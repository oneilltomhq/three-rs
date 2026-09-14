//! Port of `three.js/src/math/Euler.js`.
//!
//! `Euler` in three.js holds its fields behind accessors so it can fire an
//! `onChange` callback (that is how `Object3D.rotation` keeps
//! `Object3D.quaternion` in step). Rust has no such hook, so the fields are
//! plain and the callers that need the sync do it explicitly — see
//! [`crate::core::Object3D::set_rotation`].

use super::math_utils::clamp;
use super::{Matrix4, Quaternion, Vector3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EulerOrder {
    /// `Euler.DEFAULT_ORDER`.
    #[default]
    XYZ,
    YXZ,
    ZXY,
    ZYX,
    YZX,
    XZY,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Euler {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub order: EulerOrder,
}

impl Euler {
    /// `new Euler( x, y, z )` — order defaults to `Euler.DEFAULT_ORDER`.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            order: EulerOrder::XYZ,
        }
    }

    /// `new Euler( x, y, z, order )`.
    pub fn new_with_order(x: f64, y: f64, z: f64, order: EulerOrder) -> Self {
        Self { x, y, z, order }
    }

    /// `Euler.set( x, y, z )` — keeps the current order.
    pub fn set(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }

    /// `Euler.set( x, y, z, order )`.
    pub fn set_with_order(&mut self, x: f64, y: f64, z: f64, order: EulerOrder) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self.order = order;
        self
    }

    /// `Euler.copy()`.
    pub fn copy(&mut self, euler: &Self) -> &mut Self {
        *self = *euler;
        self
    }

    /// `Euler.setFromRotationMatrix()`. Assumes the upper 3x3 of `m` is a pure
    /// rotation (i.e. unscaled).
    pub fn set_from_rotation_matrix(&mut self, m: &Matrix4, order: EulerOrder) -> &mut Self {
        let te = &m.elements;
        let (m11, m12, m13) = (te[0], te[4], te[8]);
        let (m21, m22, m23) = (te[1], te[5], te[9]);
        let (m31, m32, m33) = (te[2], te[6], te[10]);

        match order {
            EulerOrder::XYZ => {
                self.y = clamp(m13, -1.0, 1.0).asin();

                if m13.abs() < 0.9999999 {
                    self.x = (-m23).atan2(m33);
                    self.z = (-m12).atan2(m11);
                } else {
                    self.x = m32.atan2(m22);
                    self.z = 0.0;
                }
            }
            EulerOrder::YXZ => {
                self.x = (-clamp(m23, -1.0, 1.0)).asin();

                if m23.abs() < 0.9999999 {
                    self.y = m13.atan2(m33);
                    self.z = m21.atan2(m22);
                } else {
                    self.y = (-m31).atan2(m11);
                    self.z = 0.0;
                }
            }
            EulerOrder::ZXY => {
                self.x = clamp(m32, -1.0, 1.0).asin();

                if m32.abs() < 0.9999999 {
                    self.y = (-m31).atan2(m33);
                    self.z = (-m12).atan2(m22);
                } else {
                    self.y = 0.0;
                    self.z = m21.atan2(m11);
                }
            }
            EulerOrder::ZYX => {
                self.y = (-clamp(m31, -1.0, 1.0)).asin();

                if m31.abs() < 0.9999999 {
                    self.x = m32.atan2(m33);
                    self.z = m21.atan2(m11);
                } else {
                    self.x = 0.0;
                    self.z = (-m12).atan2(m22);
                }
            }
            EulerOrder::YZX => {
                self.z = clamp(m21, -1.0, 1.0).asin();

                if m21.abs() < 0.9999999 {
                    self.x = (-m23).atan2(m22);
                    self.y = (-m31).atan2(m11);
                } else {
                    self.x = 0.0;
                    self.y = m13.atan2(m33);
                }
            }
            EulerOrder::XZY => {
                self.z = (-clamp(m12, -1.0, 1.0)).asin();

                if m12.abs() < 0.9999999 {
                    self.x = m32.atan2(m22);
                    self.y = m13.atan2(m11);
                } else {
                    self.x = (-m23).atan2(m33);
                    self.y = 0.0;
                }
            }
        }

        self.order = order;

        self
    }

    /// `Euler.setFromQuaternion()`.
    pub fn set_from_quaternion(&mut self, q: &Quaternion, order: EulerOrder) -> &mut Self {
        let mut matrix = Matrix4::identity();
        matrix.make_rotation_from_quaternion(q);
        self.set_from_rotation_matrix(&matrix, order)
    }

    /// `Euler.setFromVector3()` — keeps the current order.
    pub fn set_from_vector3(&mut self, v: &Vector3) -> &mut Self {
        self.set(v.x, v.y, v.z)
    }

    /// `Euler.reorder()`: same rotation, different order. Note that information
    /// can be lost, exactly as in three.js.
    pub fn reorder(&mut self, new_order: EulerOrder) -> &mut Self {
        let mut q = Quaternion::default();
        q.set_from_euler(self);
        self.set_from_quaternion(&q, new_order)
    }

    /// `Euler.equals()`.
    pub fn equals(&self, euler: &Self) -> bool {
        euler.x == self.x && euler.y == self.y && euler.z == self.z && euler.order == self.order
    }

    /// `Euler.fromArray()` — the `order` element of three.js' array form has no
    /// numeric equivalent, so only the three angles are read.
    pub fn from_array(&mut self, array: &[f64]) -> &mut Self {
        self.x = array[0];
        self.y = array[1];
        self.z = array[2];
        self
    }

    /// `Euler.toArray()`, angles only.
    pub fn to_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}
