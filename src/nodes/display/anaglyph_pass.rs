//! Port of `three.js/examples/jsm/tsl/display/AnaglyphPassNode.js`.
//!
//! Both eyes are rendered through [`frame_corners`]: a virtual screen
//! `planeDistance` in front of the camera, as wide and tall as the camera's
//! field of view makes it there, framed exactly from each eye, so that
//! whatever sits on that plane has zero parallax. The two images are then
//! mixed into one by a pair of `mat3` colour matrices,
//! `clamp( Mₗ · L.rgb + Mᵣ · R.rgb )`, with the larger of the two alphas.
//!
//! All seven algorithms in all three colour modes are here, transcribed from
//! three's `ANAGLYPH_MATRICES` through the same row-major-to-column-major
//! `createMatrixPair()`; `tests/cameras_stereo_camera.rs` checks all 21
//! pairs against what three's own node writes into its uniforms.
//!
//! # Not ported
//!
//! * `material.contextNode = context( builder.getSharedContext() )`: the
//!   port's quad materials build in their own context, as every other
//!   display node's do.
//! * `dispose()`: the targets and the material are dropped with the node.

use std::cell::{Cell, RefMut};
use std::rc::Rc;

use crate::addons::camera_utils::frame_corners;
use crate::cameras::{PerspectiveCamera, StereoCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{CoordinateSystem, Matrix3, Vector3, DEG2RAD};
use crate::nodes::node::SettableValue;
use crate::nodes::tsl::{max, texture_sample, to_var, uniform_settable, uv, vec4_join};
use crate::nodes::{NodeRef, Type};
use crate::objects::QuadMesh;
use crate::renderer::SceneRef;

use super::stereo_composite_pass::{AnaglyphEyes, CompositeState};

/// `AnaglyphAlgorithm` — how each eye's colour reaches the output channels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnaglyphAlgorithm {
    /// `TRUE` — red from the left eye, the right eye's luminance in the
    /// other filter's channels.
    True,
    /// `GREY` — luminance only: no colour, minimal ghosting.
    Grey,
    /// `COLOUR` — full colour, high retinal rivalry.
    Colour,
    /// `HALF_COLOUR` — the left eye's luminance, the right eye in colour.
    HalfColour,
    /// `DUBOIS` — Dubois' least-squares matrices. The default.
    #[default]
    Dubois,
    /// `OPTIMISED` — the left eye's green and blue as red, to reduce
    /// retinal rivalry.
    Optimised,
    /// `COMPROMISE` — Ahtik's balance of colour and stereo effect.
    Compromise,
}

impl AnaglyphAlgorithm {
    /// Every algorithm, in the order three declares them.
    pub const ALL: [Self; 7] = [
        Self::True,
        Self::Grey,
        Self::Colour,
        Self::HalfColour,
        Self::Dubois,
        Self::Optimised,
        Self::Compromise,
    ];

    /// The string three's enum holds: `'true'`, `'grey'`, `'colour'`,
    /// `'halfColour'`, `'dubois'`, `'optimised'`, `'compromise'`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::True => "true",
            Self::Grey => "grey",
            Self::Colour => "colour",
            Self::HalfColour => "halfColour",
            Self::Dubois => "dubois",
            Self::Optimised => "optimised",
            Self::Compromise => "compromise",
        }
    }
}

/// `AnaglyphColorMode` — which pair of filters the glasses have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnaglyphColorMode {
    /// `RED_CYAN` — the default.
    #[default]
    RedCyan,
    /// `MAGENTA_CYAN`.
    MagentaCyan,
    /// `MAGENTA_GREEN`.
    MagentaGreen,
}

impl AnaglyphColorMode {
    /// Every colour mode, in the order three declares them.
    pub const ALL: [Self; 3] = [Self::RedCyan, Self::MagentaCyan, Self::MagentaGreen];

    /// The string three's enum holds: `'redCyan'`, `'magentaCyan'`,
    /// `'magentaGreen'`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedCyan => "redCyan",
            Self::MagentaCyan => "magentaCyan",
            Self::MagentaGreen => "magentaGreen",
        }
    }
}

/// `LUM` — the ITU-R BT.601 luminance coefficients.
const LUM: [f64; 3] = [0.299, 0.587, 0.114];

/// One eye's `createMatrixPair()` specification: for each output channel,
/// how much of the input red, green and blue reaches it. A missing channel
/// is `[ 0, 0, 0 ]`.
#[derive(Clone, Copy, Default)]
struct Spec {
    r: Option<[f64; 3]>,
    g: Option<[f64; 3]>,
    b: Option<[f64; 3]>,
}

const fn r(r: [f64; 3]) -> Spec {
    Spec {
        r: Some(r),
        g: None,
        b: None,
    }
}

impl Spec {
    const fn g(self, g: [f64; 3]) -> Self {
        Self { g: Some(g), ..self }
    }

    const fn b(self, b: [f64; 3]) -> Self {
        Self { b: Some(b), ..self }
    }

    /// `specToColumnMajor( spec )` — the array `Matrix3.fromArray()` takes.
    fn to_column_major(self) -> [f64; 9] {
        let r = self.r.unwrap_or([0.0; 3]);
        let g = self.g.unwrap_or([0.0; 3]);
        let b = self.b.unwrap_or([0.0; 3]);
        [
            r[0], g[0], b[0], // Column 0: coefficients for input R
            r[1], g[1], b[1], // Column 1: coefficients for input G
            r[2], g[2], b[2], // Column 2: coefficients for input B
        ]
    }
}

const NONE: Spec = Spec {
    r: None,
    g: None,
    b: None,
};

/// `ANAGLYPH_MATRICES[ algorithm ][ colorMode ]`'s `{ left, right }` specs.
fn specs(algorithm: AnaglyphAlgorithm, color_mode: AnaglyphColorMode) -> (Spec, Spec) {
    use AnaglyphAlgorithm as A;
    use AnaglyphColorMode as M;
    match (algorithm, color_mode) {
        // True Anaglyph - Red channel from left, luminance to cyan channel
        // for right. Paper: Left=[R,0,0], Right=[0,0,Lum]
        (A::True, M::RedCyan) => (r([1.0, 0.0, 0.0]), NONE.g(LUM).b(LUM)),
        (A::True, M::MagentaCyan) => (
            r([1.0, 0.0, 0.0]).b([0.0, 0.0, 0.5]),
            NONE.g(LUM).b([0.0, 0.0, 0.5]),
        ),
        (A::True, M::MagentaGreen) => (r([1.0, 0.0, 0.0]).b(LUM), NONE.g(LUM)),

        // Grey Anaglyph - Luminance-based, no color, minimal ghosting.
        // Paper: Left=[Lum,0,0], Right=[0,0,Lum]
        (A::Grey, M::RedCyan) => (r(LUM), NONE.g(LUM).b(LUM)),
        (A::Grey, M::MagentaCyan) => (
            r(LUM).b([0.15, 0.29, 0.06]),
            NONE.g(LUM).b([0.15, 0.29, 0.06]),
        ),
        (A::Grey, M::MagentaGreen) => (r(LUM).b(LUM), NONE.g(LUM)),

        // Colour Anaglyph - Full color, high retinal rivalry.
        // Paper: Left=[R,0,0], Right=[0,G,B]
        (A::Colour, M::RedCyan) => (
            r([1.0, 0.0, 0.0]),
            NONE.g([0.0, 1.0, 0.0]).b([0.0, 0.0, 1.0]),
        ),
        (A::Colour, M::MagentaCyan) => (
            r([1.0, 0.0, 0.0]).b([0.0, 0.0, 0.5]),
            NONE.g([0.0, 1.0, 0.0]).b([0.0, 0.0, 0.5]),
        ),
        (A::Colour, M::MagentaGreen) => (
            r([1.0, 0.0, 0.0]).b([0.0, 0.0, 1.0]),
            NONE.g([0.0, 1.0, 0.0]),
        ),

        // Half-Colour Anaglyph - Luminance for left red, full color for
        // right cyan. Paper: Left=[Lum,0,0], Right=[0,G,B]
        (A::HalfColour, M::RedCyan) => (r(LUM), NONE.g([0.0, 1.0, 0.0]).b([0.0, 0.0, 1.0])),
        (A::HalfColour, M::MagentaCyan) => (
            r(LUM).b([0.15, 0.29, 0.06]),
            NONE.g([0.0, 1.0, 0.0]).b([0.15, 0.29, 0.06]),
        ),
        (A::HalfColour, M::MagentaGreen) => (r(LUM).b(LUM), NONE.g([0.0, 1.0, 0.0])),

        // Dubois Anaglyph - Least-squares optimized for specific glasses.
        (A::Dubois, M::RedCyan) => (
            r([0.4561, 0.500484, 0.176381])
                .g([-0.0400822, -0.0378246, -0.0157589])
                .b([-0.0152161, -0.0205971, -0.00546856]),
            r([-0.0434706, -0.0879388, -0.00155529])
                .g([0.378476, 0.73364, -0.0184503])
                .b([-0.0721527, -0.112961, 1.2264]),
        ),
        (A::Dubois, M::MagentaCyan) => (
            r([0.4561, 0.500484, 0.176381])
                .g([-0.0400822, -0.0378246, -0.0157589])
                .b([0.088, 0.088, -0.003]),
            r([-0.0434706, -0.0879388, -0.00155529])
                .g([0.378476, 0.73364, -0.0184503])
                .b([0.088, 0.088, 0.613]),
        ),
        (A::Dubois, M::MagentaGreen) => (
            r([0.4561, 0.500484, 0.176381]).b([-0.0434706, -0.0879388, -0.00155529]),
            NONE.g([0.378476 + 0.4561, 0.73364 + 0.500484, -0.0184503 + 0.176381]),
        ),

        // Optimised Anaglyph - Improved color with reduced retinal rivalry.
        // Paper: Left=[0,0.7G+0.3B,0,0], Right=[0,G,B]
        (A::Optimised, M::RedCyan) => (
            r([0.0, 0.7, 0.3]),
            NONE.g([0.0, 1.0, 0.0]).b([0.0, 0.0, 1.0]),
        ),
        (A::Optimised, M::MagentaCyan) => (
            r([0.0, 0.7, 0.3]).b([0.0, 0.0, 0.5]),
            NONE.g([0.0, 1.0, 0.0]).b([0.0, 0.0, 0.5]),
        ),
        (A::Optimised, M::MagentaGreen) => (
            r([0.0, 0.7, 0.3]).b([0.0, 0.0, 1.0]),
            NONE.g([0.0, 1.0, 0.0]),
        ),

        // Compromise Anaglyph - Best balance of color and stereo effect.
        // Paper matrix [8]: Left=[0.439R+0.447G+0.148B, 0, 0],
        // Right=[0, 0.095R+0.934G+0.005B, 0.018R+0.028G+1.057B]
        (A::Compromise, M::RedCyan) => (
            r([0.439, 0.447, 0.148]),
            NONE.g([0.095, 0.934, 0.005]).b([0.018, 0.028, 1.057]),
        ),
        (A::Compromise, M::MagentaCyan) => (
            r([0.439, 0.447, 0.148]).b([0.009, 0.014, 0.074]),
            NONE.g([0.095, 0.934, 0.005]).b([0.009, 0.014, 0.528]),
        ),
        (A::Compromise, M::MagentaGreen) => (
            r([0.439, 0.447, 0.148]).b([0.018, 0.028, 1.057]),
            NONE.g([0.095 + 0.439, 0.934 + 0.447, 0.005 + 0.148]),
        ),
    }
}

/// `ANAGLYPH_MATRICES[ algorithm ][ colorMode ]`, as the two `Matrix3`s
/// `_updateMatrices()` writes into the left and right uniforms.
pub fn anaglyph_matrices(
    algorithm: AnaglyphAlgorithm,
    color_mode: AnaglyphColorMode,
) -> (Matrix3, Matrix3) {
    let (left, right) = specs(algorithm, color_mode);
    let matrix = |spec: Spec| {
        let mut m = Matrix3::identity();
        m.from_array(&spec.to_column_major(), 0);
        m
    };
    (matrix(left), matrix(right))
}

/// The values a `mat3` uniform holds: the matrix's columns, each padded to
/// a `vec4`, as `Matrix3` is laid out in a uniform buffer.
fn mat3_values(m: &Matrix3) -> Vec<f64> {
    m.to_padded_f32_array().iter().map(|&v| v as f64).collect()
}

/// `anaglyphPass( scene, camera )`.
pub fn anaglyph_pass(
    scene: SceneRef,
    camera: std::rc::Rc<std::cell::RefCell<PerspectiveCamera>>,
) -> AnaglyphPassNode {
    AnaglyphPassNode::new(scene, camera)
}

/// `AnaglyphPassNode` — see the module docs.
pub struct AnaglyphPassNode {
    state: Rc<CompositeState>,
    /// `this._algorithm`.
    algorithm: Cell<AnaglyphAlgorithm>,
    /// `this._colorMode`.
    color_mode: Cell<AnaglyphColorMode>,
    /// `this._colorMatrixLeft.value` / `_colorMatrixRight.value`.
    color_matrices: Cell<(Matrix3, Matrix3)>,
    /// The two uniforms' handles.
    color_matrix_left: SettableValue,
    color_matrix_right: SettableValue,
}

impl AnaglyphPassNode {
    /// `AnaglyphPassNode.type`.
    pub const TYPE: &'static str = "AnaglyphPassNode";

    /// `new AnaglyphPassNode( scene, camera )`, with the material `setup()`
    /// builds.
    pub fn new(
        scene: SceneRef,
        camera: std::rc::Rc<std::cell::RefCell<PerspectiveCamera>>,
    ) -> Self {
        let algorithm = AnaglyphAlgorithm::Dubois;
        let color_mode = AnaglyphColorMode::RedCyan;
        // `uniform( new Matrix3() )`, then `_updateMatrices()`.
        let matrices = anaglyph_matrices(algorithm, color_mode);
        let (left_node, left) = uniform_settable(Type::Mat3, mat3_values(&matrices.0));
        let (right_node, right) = uniform_settable(Type::Mat3, mat3_values(&matrices.1));

        let eyes = AnaglyphEyes {
            eye_sep: Cell::new(0.064),
            plane_distance: Cell::new(0.5),
        };
        let state = CompositeState::new(scene, camera, Some(eyes), |map_left, map_right| {
            let mut material = MeshBasicNodeMaterial::new();
            material.name = "Anaglyph";
            material.fragment_node =
                Some(anaglyph_node(map_left, map_right, left_node, right_node));
            material
        });

        Self {
            state,
            algorithm: Cell::new(algorithm),
            color_mode: Cell::new(color_mode),
            color_matrices: Cell::new(matrices),
            color_matrix_left: left,
            color_matrix_right: right,
        }
    }

    /// `getTextureNode()` — the anaglyph, for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.state.node()
    }

    /// `this.stereo` — whose `cameraL` / `cameraR` the eyes are rendered
    /// with. Its `eyeSep` and `aspect` are not read: this node frames its
    /// eyes itself ([`update_stereo_camera`](Self::update_stereo_camera)).
    pub fn stereo(&self) -> RefMut<'_, StereoCamera> {
        self.state.stereo()
    }

    fn eyes(&self) -> &AnaglyphEyes {
        self.state
            .anaglyph()
            .expect("three-rs: an AnaglyphPassNode's state carries its eyes")
    }

    /// `anaglyphPass.eyeSep` — the interpupillary distance in world units.
    /// `0.064` by default.
    pub fn eye_sep(&self) -> f64 {
        self.eyes().eye_sep.get()
    }

    /// `anaglyphPass.eyeSep = value`.
    pub fn set_eye_sep(&self, value: f64) {
        self.eyes().eye_sep.set(value);
    }

    /// `anaglyphPass.planeDistance` — how far in front of the camera the
    /// zero-parallax plane is. `0.5` by default.
    pub fn plane_distance(&self) -> f64 {
        self.eyes().plane_distance.get()
    }

    /// `anaglyphPass.planeDistance = value`.
    pub fn set_plane_distance(&self, value: f64) {
        self.eyes().plane_distance.set(value);
    }

    /// `anaglyphPass.algorithm`.
    pub fn algorithm(&self) -> AnaglyphAlgorithm {
        self.algorithm.get()
    }

    /// `anaglyphPass.algorithm = value` — rewrites both colour matrices
    /// when it changes.
    pub fn set_algorithm(&self, value: AnaglyphAlgorithm) {
        if self.algorithm.get() != value {
            self.algorithm.set(value);
            self.update_matrices();
        }
    }

    /// `anaglyphPass.colorMode`.
    pub fn color_mode(&self) -> AnaglyphColorMode {
        self.color_mode.get()
    }

    /// `anaglyphPass.colorMode = value` — as
    /// [`set_algorithm`](Self::set_algorithm).
    pub fn set_color_mode(&self, value: AnaglyphColorMode) {
        if self.color_mode.get() != value {
            self.color_mode.set(value);
            self.update_matrices();
        }
    }

    /// `this._colorMatrixLeft.value`.
    pub fn color_matrix_left(&self) -> Matrix3 {
        self.color_matrices.get().0
    }

    /// `this._colorMatrixRight.value`.
    pub fn color_matrix_right(&self) -> Matrix3 {
        self.color_matrices.get().1
    }

    /// `_updateMatrices()`.
    fn update_matrices(&self) {
        let matrices = anaglyph_matrices(self.algorithm.get(), self.color_mode.get());
        self.color_matrix_left.set(mat3_values(&matrices.0));
        self.color_matrix_right.set(mat3_values(&matrices.1));
        self.color_matrices.set(matrices);
    }

    /// `updateStereoCamera( coordinateSystem )` — what `updateBefore()`
    /// calls with the renderer's coordinate system, public as three's is.
    pub fn update_stereo_camera(&self, coordinate_system: CoordinateSystem) {
        self.state.update_stereo_camera(coordinate_system);
    }

    /// The `Anaglyph` quad material, for `examples/dump_wgsl.rs` and the
    /// dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        let quad: std::cell::Ref<'_, QuadMesh> = self.state.quad();
        quad.material.clone()
    }
}

/// `AnaglyphPassNode.setup()`'s `anaglyph` `Fn()`.
fn anaglyph_node(
    map_left: &crate::textures::Texture,
    map_right: &crate::textures::Texture,
    color_matrix_left: NodeRef,
    color_matrix_right: NodeRef,
) -> NodeRef {
    let uv_node = uv();
    // `this._mapLeft.sample( uvNode )`: read twice, so a var.
    let color_l = to_var(None, texture_sample(map_left, uv_node.clone()));
    let color_r = to_var(None, texture_sample(map_right, uv_node));
    // `clamp( x )` — `clamp( x, 0, 1 )`.
    let color = color_matrix_left
        .mul(color_l.rgb())
        .add(color_matrix_right.mul(color_r.rgb()))
        .clamp(0.0, 1.0);
    // `vec4( color.rgb, … )` — `.rgb` of a `vec3` is the `vec3`.
    vec4_join(vec![color, max(color_l.a(), color_r.a())])
}

/// `AnaglyphPassNode.updateStereoCamera( coordinateSystem )`: place each eye
/// `eyeSep / 2` along the camera's right axis and frame, from there, the
/// screen `planeDistance` in front of the camera.
///
/// The eyes' `projectionMatrix` is `frameCorners()`'s, which is OpenGL-style
/// whatever their `coordinateSystem` says — as in three, where the WebGPU
/// renderer, seeing the eye already in its own coordinate system, does not
/// rebuild it. Their world matrices are composed here and left alone by the
/// render: `matrixAutoUpdate` is off and `matrixWorldNeedsUpdate` false, so
/// `updateMatrixWorld()` keeps them.
pub(super) fn update_stereo_camera(
    stereo: &mut StereoCamera,
    camera: &PerspectiveCamera,
    eye_sep: f64,
    plane_distance: f64,
    coordinate_system: CoordinateSystem,
) {
    stereo.camera_l.coordinate_system = coordinate_system;
    stereo.camera_r.coordinate_system = coordinate_system;

    // Get the camera's local coordinate axes from its world matrix
    let (mut right, mut up, mut forward) = (Vector3::ZERO, Vector3::ZERO, Vector3::ZERO);
    let (matrix_world, position) = {
        let object = camera.node.borrow();
        (object.matrix_world, object.position)
    };
    matrix_world.extract_basis(&mut right, &mut up, &mut forward);
    right.normalize();
    up.normalize();
    forward.normalize();

    // Calculate eye positions
    let half_sep = eye_sep / 2.0;
    let mut eye_l = position;
    eye_l.add_scaled_vector(&right, -half_sep);
    let mut eye_r = position;
    eye_r.add_scaled_vector(&right, half_sep);

    // Calculate screen center (at planeDistance in front of the camera center)
    let mut screen_center = position;
    screen_center.add_scaled_vector(&forward, -plane_distance);

    // Calculate screen dimensions from camera FOV and aspect ratio
    let half_height = plane_distance * (DEG2RAD * camera.fov / 2.0).tan();
    let half_width = half_height * camera.aspect;

    // Calculate screen corners
    let mut screen_bottom_left = screen_center;
    screen_bottom_left
        .add_scaled_vector(&right, -half_width)
        .add_scaled_vector(&up, -half_height);
    let mut screen_bottom_right = screen_center;
    screen_bottom_right
        .add_scaled_vector(&right, half_width)
        .add_scaled_vector(&up, -half_height);
    let mut screen_top_left = screen_center;
    screen_top_left
        .add_scaled_vector(&right, -half_width)
        .add_scaled_vector(&up, half_height);

    for (eye, position) in [(&mut stereo.camera_l, eye_l), (&mut stereo.camera_r, eye_r)] {
        eye.node.borrow_mut().position = position;
        eye.near = camera.near;
        eye.far = camera.far;
        frame_corners(
            eye,
            &screen_bottom_left,
            &screen_bottom_right,
            &screen_top_left,
            true,
        );
        let matrix_world = {
            let mut object = eye.node.borrow_mut();
            let (position, quaternion, scale) = (object.position, object.quaternion, object.scale);
            object.matrix_world.compose(&position, &quaternion, &scale);
            object.matrix_world
        };
        eye.matrix_world_inverse = matrix_world;
        eye.matrix_world_inverse.invert();
    }
}
