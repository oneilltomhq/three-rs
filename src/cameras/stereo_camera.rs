//! Port of `three.js/src/cameras/StereoCamera.js`.

use crate::math::{Matrix4, DEG2RAD};

use super::PerspectiveCamera;

/// `StereoCamera._cache`: what the eye projections were last built from.
/// three's starts as seven `null`s, which no number equals, so the first
/// [`StereoCamera::update`] always rebuilds; `None` is that state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cache {
    focus: f64,
    fov: f64,
    aspect: f64,
    near: f64,
    far: f64,
    zoom: f64,
    eye_sep: f64,
}

/// three.js' `StereoCamera`: two [`PerspectiveCamera`]s with an off-axis
/// stereoscopic projection, kept in step with a third camera by
/// [`update`](Self::update). The passes in
/// [`nodes::display`](crate::nodes::display) render through it.
///
/// One divergence, invisible to every caller three.js has: three's
/// `_eyeLeft` / `_eyeRight` translations are module-scoped and written only
/// when a camera's cache misses, so of two `StereoCamera`s with *different*
/// `eyeSep`, the one whose cache hits offsets its eyes by whatever the other
/// wrote last. The port keeps the translation per camera, derived from the
/// cached `eyeSep`, which is what three computes whenever only one
/// `eyeSep` is in play (`webgpu_display_stereo` sets both of its stereo
/// cameras' `eyeSep` together).
///
/// Not `Clone`: a [`PerspectiveCamera`] clone shares its scene-graph node,
/// so a cloned `StereoCamera` would move the original's eyes.
pub struct StereoCamera {
    /// `StereoCamera.aspect`: multiplies the source camera's aspect. `1` by
    /// default; [`StereoPassNode`](crate::nodes::display::StereoPassNode),
    /// whose eyes each get half the target, sets `0.5`.
    pub aspect: f64,
    /// `StereoCamera.eyeSep`: the distance between the two eyes, in world
    /// units. `0.064` by default.
    pub eye_sep: f64,
    /// `StereoCamera.cameraL` — layer 1 enabled, `matrixAutoUpdate` off.
    pub camera_l: PerspectiveCamera,
    /// `StereoCamera.cameraR` — layer 2 enabled, `matrixAutoUpdate` off.
    pub camera_r: PerspectiveCamera,
    cache: Option<Cache>,
}

impl Default for StereoCamera {
    fn default() -> Self {
        Self::new()
    }
}

impl StereoCamera {
    /// `StereoCamera.type`.
    pub const TYPE: &'static str = "StereoCamera";

    /// `new StereoCamera()`.
    pub fn new() -> Self {
        let eye = |layer: u32| {
            // `new PerspectiveCamera()`: fov 50, aspect 1, near 0.1, far 2000.
            let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 2000.0);
            {
                let mut object = camera.node.borrow_mut();
                object.layers.enable(layer);
                object.matrix_auto_update = false;
            }
            camera
        };
        Self {
            aspect: 1.0,
            eye_sep: 0.064,
            camera_l: eye(1),
            camera_r: eye(2),
            cache: None,
        }
    }

    /// `StereoCamera.update( camera )`: rebuild the eye projections when
    /// anything they depend on changed, then place both eyes on `camera`'s
    /// world matrix, `eyeSep / 2` to either side.
    ///
    /// Only `projectionMatrix` is written, as in three.js:
    /// `projectionMatrixInverse` of the eyes keeps whatever it held.
    pub fn update(&mut self, camera: &PerspectiveCamera) {
        let current = Cache {
            focus: camera.focus,
            fov: camera.fov,
            aspect: camera.aspect * self.aspect,
            near: camera.near,
            far: camera.far,
            zoom: camera.zoom,
            eye_sep: self.eye_sep,
        };

        // `cache.focus !== camera.focus || …` — `!=` on each field, so a NaN
        // misses every time, as `!==` does.
        let needs_update = match self.cache {
            None => true,
            Some(cache) => {
                cache.focus != current.focus
                    || cache.fov != current.fov
                    || cache.aspect != current.aspect
                    || cache.near != current.near
                    || cache.far != current.far
                    || cache.zoom != current.zoom
                    || cache.eye_sep != current.eye_sep
            }
        };

        if needs_update {
            self.cache = Some(current);
            let cache = current;

            // Off-axis stereoscopic effect based on
            // http://paulbourke.net/stereographics/stereorender/

            let mut projection_matrix = camera.projection_matrix;
            let eye_sep_half = cache.eye_sep / 2.0;
            let eye_sep_on_projection = eye_sep_half * cache.near / cache.focus;
            let ymax = (cache.near * (DEG2RAD * cache.fov * 0.5).tan()) / cache.zoom;

            // for left eye

            let xmin = -ymax * cache.aspect + eye_sep_on_projection;
            let xmax = ymax * cache.aspect + eye_sep_on_projection;

            projection_matrix.elements[0] = 2.0 * cache.near / (xmax - xmin);
            projection_matrix.elements[8] = (xmax + xmin) / (xmax - xmin);

            self.camera_l.projection_matrix = projection_matrix;

            // for right eye

            let xmin = -ymax * cache.aspect - eye_sep_on_projection;
            let xmax = ymax * cache.aspect - eye_sep_on_projection;

            projection_matrix.elements[0] = 2.0 * cache.near / (xmax - xmin);
            projection_matrix.elements[8] = (xmax + xmin) / (xmax - xmin);

            self.camera_r.projection_matrix = projection_matrix;
        }

        // translate xOffset — `_eyeLeft.elements[ 12 ] = - eyeSepHalf` and
        // `_eyeRight.elements[ 12 ] = eyeSepHalf`, from the cache.
        let eye_sep_half = self.cache.map_or(0.0, |cache| cache.eye_sep) / 2.0;
        let mut eye_left = Matrix4::identity();
        eye_left.elements[12] = -eye_sep_half;
        let mut eye_right = Matrix4::identity();
        eye_right.elements[12] = eye_sep_half;

        let matrix_world = camera.node.borrow().matrix_world;

        for (eye, offset) in [(&self.camera_l, &eye_left), (&self.camera_r, &eye_right)] {
            let mut object = eye.node.borrow_mut();
            object.matrix = matrix_world;
            object.matrix.multiply(offset);
            object.matrix_world_needs_update = true;
        }
    }
}
