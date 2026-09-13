//! Ports of `three.js/src/lights/LightShadow.js`, `SpotLightShadow.js` and
//! `DirectionalLightShadow.js`.

use crate::cameras::{OrthographicCamera, PerspectiveCamera};
use crate::math::{Matrix4, Vector2, Vector3, RAD2DEG};

/// `LightShadow.camera` — a `PerspectiveCamera` for `SpotLightShadow`, an
/// `OrthographicCamera` for `DirectionalLightShadow`.
#[derive(Clone)]
pub enum ShadowCamera {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl ShadowCamera {
    pub fn near(&self) -> f64 {
        match self {
            ShadowCamera::Perspective(c) => c.near,
            ShadowCamera::Orthographic(c) => c.near,
        }
    }

    pub fn set_near(&mut self, near: f64) {
        match self {
            ShadowCamera::Perspective(c) => c.near = near,
            ShadowCamera::Orthographic(c) => c.near = near,
        }
    }

    pub fn far(&self) -> f64 {
        match self {
            ShadowCamera::Perspective(c) => c.far,
            ShadowCamera::Orthographic(c) => c.far,
        }
    }

    pub fn set_far(&mut self, far: f64) {
        match self {
            ShadowCamera::Perspective(c) => c.far = far,
            ShadowCamera::Orthographic(c) => c.far = far,
        }
    }

    /// `shadow.camera.isOrthographicCamera` — `ShadowNode.setupShadowCoord()`
    /// branches on it (and so does `_updateMatrix`, through
    /// `coordinateSystem`).
    pub fn is_orthographic(&self) -> bool {
        matches!(self, ShadowCamera::Orthographic(_))
    }

    pub fn projection_matrix(&self) -> Matrix4 {
        match self {
            ShadowCamera::Perspective(c) => c.projection_matrix,
            ShadowCamera::Orthographic(c) => c.projection_matrix,
        }
    }

    pub fn matrix_world(&self) -> Matrix4 {
        match self {
            ShadowCamera::Perspective(c) => c.object.matrix_world,
            ShadowCamera::Orthographic(c) => c.object.matrix_world,
        }
    }

    pub fn matrix_world_inverse(&self) -> Matrix4 {
        match self {
            ShadowCamera::Perspective(c) => c.matrix_world_inverse,
            ShadowCamera::Orthographic(c) => c.matrix_world_inverse,
        }
    }

    pub fn update_projection_matrix(&mut self) {
        match self {
            ShadowCamera::Perspective(c) => c.update_projection_matrix(),
            ShadowCamera::Orthographic(c) => c.update_projection_matrix(),
        }
    }

    /// `shadowCamera.position.copy( lightPositionWorld )`, `lookAt( target )`,
    /// `updateMatrixWorld()`.
    fn place(&mut self, position: Vector3, target: &Vector3) {
        match self {
            ShadowCamera::Perspective(c) => {
                c.object.position = position;
                c.look_at(target);
                c.update_matrix_world();
            }
            ShadowCamera::Orthographic(c) => {
                c.object.position = position;
                let mut m = Matrix4::identity();
                m.look_at(&c.object.position, target, &c.object.up);
                c.object.quaternion.set_from_rotation_matrix(&m);
                c.update_matrix_world();
            }
        }
    }
}

/// `class LightShadow`, plus the `SpotLightShadow`/`DirectionalLightShadow`
/// fields (`focus`, `aspect`) — the two subclasses differ only in their camera
/// and in `updateMatrices`, so one struct with a camera enum covers both.
#[derive(Clone)]
pub struct LightShadow {
    pub camera: ShadowCamera,
    /// `this.intensity`.
    pub intensity: f64,
    /// `this.bias` — added to the shadow coordinate's `z` in the shader.
    pub bias: f64,
    /// `this.normalBias` — scales `normalWorld` before the shadow matrix.
    pub normal_bias: f64,
    /// `this.radius` — the PCF disk radius, in texels.
    pub radius: f64,
    /// `this.blurSamples` (VSM only; carried for completeness).
    pub blur_samples: usize,
    /// `this.mapSize`.
    pub map_size: Vector2,
    /// `this.matrix` — the bias matrix composed with
    /// `projectionMatrix * matrixWorldInverse`.
    pub matrix: Matrix4,
    /// `SpotLightShadow.focus`.
    pub focus: f64,
    /// `SpotLightShadow.aspect`.
    pub aspect: f64,
}

impl LightShadow {
    /// `new SpotLightShadow()` — `new PerspectiveCamera( 50, 1, 0.5, 500 )`.
    pub fn spot() -> Self {
        Self::new(ShadowCamera::Perspective(PerspectiveCamera::new(
            50.0, 1.0, 0.5, 500.0,
        )))
    }

    /// `new DirectionalLightShadow()` —
    /// `new OrthographicCamera( -5, 5, 5, -5, 0.5, 500 )`.
    pub fn directional() -> Self {
        Self::new(ShadowCamera::Orthographic(OrthographicCamera::new(
            -5.0, 5.0, 5.0, -5.0, 0.5, 500.0,
        )))
    }

    fn new(camera: ShadowCamera) -> Self {
        Self {
            camera,
            intensity: 1.0,
            bias: 0.0,
            normal_bias: 0.0,
            radius: 1.0,
            blur_samples: 8,
            map_size: Vector2::new(512.0, 512.0),
            matrix: Matrix4::identity(),
            focus: 1.0,
            aspect: 1.0,
        }
    }

    /// `SpotLightShadow.updateMatrices()`'s projection half: the spot light's
    /// cone angle and cutoff distance drive the shadow camera.
    pub fn update_spot_projection(&mut self, angle: f64, distance: f64) {
        let fov = RAD2DEG * 2.0 * angle * self.focus;
        let aspect = (self.map_size.x / self.map_size.y) * self.aspect;
        let far = if distance != 0.0 {
            distance
        } else {
            self.camera.far()
        };

        if let ShadowCamera::Perspective(camera) = &mut self.camera {
            if fov != camera.fov || aspect != camera.aspect || far != camera.far {
                camera.fov = fov;
                camera.aspect = aspect;
                camera.far = far;
                camera.update_projection_matrix();
            }
        }
    }

    /// `LightShadow.updateMatrices( light )`.
    pub fn update_matrices(&mut self, light_matrix_world: &Matrix4, target_matrix_world: &Matrix4) {
        let mut light_position_world = Vector3::default();
        light_position_world.set_from_matrix_position(light_matrix_world);

        let mut look_target = Vector3::default();
        look_target.set_from_matrix_position(target_matrix_world);

        self.camera.place(light_position_world, &look_target);
        self.update_matrix();
    }

    /// `LightShadow._updateMatrix()` with no atlas viewport. Under
    /// `WebGPUCoordinateSystem` the Z row is the identity, because the
    /// projection matrix already produces `[0, 1]`.
    fn update_matrix(&mut self) {
        let mut proj_screen_matrix = self.camera.projection_matrix();
        proj_screen_matrix.multiply(&self.camera.matrix_world_inverse());

        let mut shadow_matrix = Matrix4::identity();
        shadow_matrix.set(
            0.5, 0.0, 0.0, 0.5, //
            0.0, 0.5, 0.0, 0.5, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );
        shadow_matrix.multiply(&proj_screen_matrix);

        self.matrix = shadow_matrix;
    }
}
