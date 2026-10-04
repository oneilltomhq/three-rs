//! Port of `three.js/examples/jsm/controls/TransformControls.js`.

use std::f64::consts::PI;
use std::rc::Rc;

use crate::cameras::{OrthographicCamera, PerspectiveCamera, RenderCamera};
use crate::core::{BufferAttribute, BufferGeometry, Node, Object3D, Raycaster};
use crate::geometries::{
    box_geometry, cylinder_geometry, octahedron_geometry, plane_geometry, sphere_geometry,
    torus_geometry, torus_geometry_full,
};
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::math::{Color, Euler, Matrix4, Quaternion, Vector2, Vector3, Vector4};
use crate::objects::{Line, Mesh, Payload};

/// `mode`: which gizmo is shown and what a drag does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// `'translate'`, the default.
    #[default]
    Translate,
    /// `'rotate'`.
    Rotate,
    /// `'scale'`.
    Scale,
}

impl Mode {
    /// The JS string.
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Translate => "translate",
            Mode::Rotate => "rotate",
            Mode::Scale => "scale",
        }
    }

    /// The index of this mode's group in `gizmo`, `picker` and `helper`.
    fn index(self) -> usize {
        match self {
            Mode::Translate => 0,
            Mode::Rotate => 1,
            Mode::Scale => 2,
        }
    }
}

/// `space`: whether the gizmo and the drag follow the world axes or the
/// object's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Space {
    /// `'world'`, the default.
    #[default]
    World,
    /// `'local'`.
    Local,
}

impl Space {
    /// The JS string.
    pub fn as_str(self) -> &'static str {
        match self {
            Space::World => "world",
            Space::Local => "local",
        }
    }
}

/// `axis`: the name of the picker handle under the pointer, or being dragged.
///
/// Three keeps it as a string and tests it with `indexOf` / `search`; the
/// port keeps the same names and tests [`as_str`](Self::as_str) the same way,
/// so `XYZE` still "contains" `XY`, as three's `showXY` check relies on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// `'X'`.
    X,
    /// `'Y'`.
    Y,
    /// `'Z'`.
    Z,
    /// `'XY'`, the plane handle.
    XY,
    /// `'YZ'`.
    YZ,
    /// `'XZ'`.
    XZ,
    /// `'XYZ'`, the centre handle (translate, scale).
    XYZ,
    /// `'XYZE'`, the trackball sphere (rotate).
    XYZE,
    /// `'E'`, the outer ring that rotates about the eye direction (rotate).
    E,
}

impl Axis {
    /// The JS string, which is also the handle's `name`.
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::X => "X",
            Axis::Y => "Y",
            Axis::Z => "Z",
            Axis::XY => "XY",
            Axis::YZ => "YZ",
            Axis::XZ => "XZ",
            Axis::XYZ => "XYZ",
            Axis::XYZE => "XYZE",
            Axis::E => "E",
        }
    }

    /// The axis a picker handle's `name` stands for.
    pub fn from_name(name: &str) -> Option<Axis> {
        Some(match name {
            "X" => Axis::X,
            "Y" => Axis::Y,
            "Z" => Axis::Z,
            "XY" => Axis::XY,
            "YZ" => Axis::YZ,
            "XZ" => Axis::XZ,
            "XYZ" => Axis::XYZ,
            "XYZE" => Axis::XYZE,
            "E" => Axis::E,
            _ => return None,
        })
    }

    /// `axis.indexOf( s ) !== -1`.
    fn has(self, s: &str) -> bool {
        self.as_str().contains(s)
    }
}

/// What three's `dispatchEvent` would have sent, in the order it would have
/// sent it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformControlsEvent {
    /// `'change'`: something that affects the rendered gizmo changed.
    Change,
    /// `'objectChange'`: the attached object's transform changed.
    ObjectChange,
    /// `'mouseDown'` with its `mode`: a drag started on a handle.
    MouseDown(Mode),
    /// `'mouseUp'` with its `mode`: a drag on a handle ended.
    MouseUp(Mode),
    /// `'<name>-changed'`: one of the `defineProperty` properties took a new
    /// value through a setter. The name is the JS property name (`axis`,
    /// `dragging`, `object`, `mode`, `rotationAngle`, ...); read the value off
    /// the controls.
    PropertyChanged(&'static str),
}

impl TransformControlsEvent {
    /// The JS event's `type`.
    pub fn type_name(&self) -> String {
        match self {
            TransformControlsEvent::Change => "change".to_string(),
            TransformControlsEvent::ObjectChange => "objectChange".to_string(),
            TransformControlsEvent::MouseDown(_) => "mouseDown".to_string(),
            TransformControlsEvent::MouseUp(_) => "mouseUp".to_string(),
            TransformControlsEvent::PropertyChanged(name) => format!("{name}-changed"),
        }
    }
}

/// `pointer.pointerType`. Only a mouse or a pen hovers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerType {
    /// `'mouse'`.
    Mouse,
    /// `'pen'`.
    Pen,
    /// `'touch'`.
    Touch,
}

/// The `{ x, y, button }` object three's public `pointerHover` /
/// `pointerDown` / `pointerMove` / `pointerUp` take: normalised device
/// coordinates (`-1 … 1`, y up) and the DOM `button` (`0` the main button,
/// `-1` for a move with no button change).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pointer {
    /// NDC x.
    pub x: f64,
    /// NDC y.
    pub y: f64,
    /// `PointerEvent.button`.
    pub button: i32,
}

/// The fields of a DOM `PointerEvent` the handlers read.
///
/// `x` / `y` are `clientX - rect.left` and `clientY - rect.top`: relative to
/// the element, y down, as in the other controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransformPointerEvent {
    /// `pointerType`.
    pub pointer_type: PointerType,
    /// `button`: `0` for the main button, `-1` on a plain move.
    pub button: i32,
    /// Element-relative x, in CSS pixels.
    pub x: f64,
    /// Element-relative y, in CSS pixels, down.
    pub y: f64,
}

/// A camera [`TransformControls`] can work with: three branches on
/// `camera.isOrthographicCamera` to size the gizmo and to find the eye
/// direction, and reads `fov` / `zoom` or `top` / `bottom` / `zoom`.
pub trait TransformCamera: RenderCamera {
    /// The `factor` the gizmo's handles are scaled by (before `size / 4`),
    /// with `distance` the distance from the object to the camera.
    fn gizmo_factor(&self, distance: f64) -> f64;
}

impl TransformCamera for PerspectiveCamera {
    /// `worldPosition.distanceTo( cameraPosition ) * Math.min( 1.9 *
    /// Math.tan( Math.PI * camera.fov / 360 ) / camera.zoom, 7 )`.
    fn gizmo_factor(&self, distance: f64) -> f64 {
        distance * js_min(1.9 * (PI * self.fov / 360.0).tan() / self.zoom, 7.0)
    }
}

impl TransformCamera for OrthographicCamera {
    /// `( camera.top - camera.bottom ) / camera.zoom`.
    fn gizmo_factor(&self, _distance: f64) -> f64 {
        (self.top - self.bottom) / self.zoom
    }
}

/// `Math.round`: the nearest integer, ties toward +∞, `-0` kept for
/// `-0.5 <= x < 0`.
fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    let rounded = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && x < 0.0 {
        -0.0
    } else {
        rounded
    }
}

/// `Math.min( a, b )`: `NaN` if either is.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}

/// `Math.max( a, b )`: `NaN` if either is.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == b && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

/// JS truthiness of a snap: `null`, `0` and `NaN` are off.
fn snap_on(snap: Option<f64>) -> Option<f64> {
    snap.filter(|s| *s != 0.0 && !s.is_nan())
}

/// `x || fallback` for a number.
fn or_number(x: f64, fallback: f64) -> f64 {
    if x == 0.0 || x.is_nan() {
        fallback
    } else {
        x
    }
}

const UNIT_X: Vector3 = Vector3::new(1.0, 0.0, 0.0);
const UNIT_Y: Vector3 = Vector3::new(0.0, 1.0, 0.0);
const UNIT_Z: Vector3 = Vector3::new(0.0, 0.0, 1.0);

/// Which of three's gizmo materials a handle was built with. Three shares
/// one material per slot between handles; here each handle owns a copy and
/// [`TransformControls::update`] writes the slot's colour and opacity, or the
/// highlight, into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// `matInvisible`: white, 0.15. Every picker.
    Invisible,
    /// `matHelper`: a line material, white, 0.5.
    Helper,
    /// `matRed` (`materialLib.xAxis`).
    Red,
    /// `matGreen` (`materialLib.yAxis`).
    Green,
    /// `matBlue` (`materialLib.zAxis`).
    Blue,
    /// `matRedTransparent` (`materialLib.xAxisTransparent`), 0.5.
    RedTransparent,
    /// `matGreenTransparent`, 0.5.
    GreenTransparent,
    /// `matBlueTransparent`, 0.5.
    BlueTransparent,
    /// `matWhiteTransparent`, 0.25.
    WhiteTransparent,
    /// `matYellowTransparent` (`materialLib.activeTransparent`), 0.25.
    YellowTransparent,
    /// `matGray`, `0x787878`.
    Gray,
}

impl Slot {
    /// The material's `opacity`.
    fn opacity(self) -> f64 {
        match self {
            Slot::Invisible => 0.15,
            Slot::Helper => 0.5,
            Slot::RedTransparent | Slot::GreenTransparent | Slot::BlueTransparent => 0.5,
            Slot::WhiteTransparent | Slot::YellowTransparent => 0.25,
            Slot::Red | Slot::Green | Slot::Blue | Slot::Gray => 1.0,
        }
    }
}

/// `materialLib`'s colours: the four `setColors()` changes. `matYellow`
/// (`materialLib.active`) is not on any handle; its colour is the highlight.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Palette {
    x_axis: Color,
    y_axis: Color,
    z_axis: Color,
    active: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            x_axis: Color::from_hex(0xff0000),
            y_axis: Color::from_hex(0x00ff00),
            z_axis: Color::from_hex(0x0000ff),
            active: Color::from_hex(0xffff00),
        }
    }
}

impl Palette {
    /// The slot material's `color` (its `_color` cache).
    fn color(&self, slot: Slot) -> Color {
        match slot {
            Slot::Invisible | Slot::Helper | Slot::WhiteTransparent => Color::from_hex(0xffffff),
            Slot::Red | Slot::RedTransparent => self.x_axis,
            Slot::Green | Slot::GreenTransparent => self.y_axis,
            Slot::Blue | Slot::BlueTransparent => self.z_axis,
            Slot::YellowTransparent => self.active,
            Slot::Gray => Color::from_hex(0x787878),
        }
    }
}

/// The geometries the gizmo definitions use, before `setupGizmo` bakes the
/// handle's offset into a clone.
#[derive(Clone, Copy, Debug)]
enum Geo {
    /// `arrowGeometry`: `CylinderGeometry( 0, 0.04, 0.1, 12 )`, raised 0.05.
    Arrow,
    /// `scaleHandleGeometry`: `BoxGeometry( 0.08, 0.08, 0.08 )`, raised 0.04.
    ScaleHandle,
    /// `lineGeometry`: a two-point line from the origin to `(1, 0, 0)`.
    Line,
    /// `lineGeometry2`: `CylinderGeometry( 0.0075, 0.0075, 0.5, 3 )`, raised
    /// 0.25.
    Line2,
    /// `CircleGeometry( radius, arc )`: a thin torus arc stood on edge.
    Circle(f64, f64),
    /// `TranslateHelperGeometry()`: a line from the origin to `(1, 1, 1)`.
    TranslateHelper,
    /// `OctahedronGeometry( radius, detail )`.
    Octahedron(f64, usize),
    /// `BoxGeometry( w, h, d )`.
    Box(f64, f64, f64),
    /// `CylinderGeometry( radiusTop, radiusBottom, height, radialSegments )`.
    Cylinder(f64, f64, f64, usize),
    /// `SphereGeometry( radius, widthSegments, heightSegments )`.
    Sphere(f64, usize, usize),
    /// `TorusGeometry( radius, tube, radialSegments, tubularSegments )`.
    Torus(f64, f64, usize, usize),
}

impl Geo {
    fn build(self) -> BufferGeometry {
        match self {
            Geo::Arrow => {
                let mut g = cylinder_geometry(0.0, 0.04, 0.1, 12);
                g.translate(0.0, 0.05, 0.0);
                g
            }
            Geo::ScaleHandle => {
                let mut g = box_geometry(0.08, 0.08, 0.08, 1, 1, 1);
                g.translate(0.0, 0.04, 0.0);
                g
            }
            Geo::Line => line_geometry(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            Geo::Line2 => {
                let mut g = cylinder_geometry(0.0075, 0.0075, 0.5, 3);
                g.translate(0.0, 0.25, 0.0);
                g
            }
            Geo::Circle(radius, arc) => {
                let mut g =
                    torus_geometry_full(radius, 0.0075, 3, 64, arc * PI * 2.0, 0.0, PI * 2.0);
                g.rotate_y(PI / 2.0);
                g.rotate_x(PI / 2.0);
                g
            }
            Geo::TranslateHelper => line_geometry(&[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
            Geo::Octahedron(radius, detail) => octahedron_geometry(radius, detail),
            Geo::Box(w, h, d) => box_geometry(w, h, d, 1, 1, 1),
            Geo::Cylinder(top, bottom, height, segments) => {
                cylinder_geometry(top, bottom, height, segments)
            }
            Geo::Sphere(radius, ws, hs) => sphere_geometry(radius, ws, hs),
            Geo::Torus(radius, tube, rs, ts) => torus_geometry(radius, tube, rs, ts),
        }
    }

    /// Built as a `Line`, not a `Mesh`.
    fn is_line(self) -> bool {
        matches!(self, Geo::Line | Geo::TranslateHelper)
    }
}

fn line_geometry(points: &[f32]) -> BufferGeometry {
    let mut g = BufferGeometry::new();
    g.set_attribute("position", BufferAttribute::new(points.to_vec(), 3));
    g
}

/// One `[ object, position, rotation, scale, tag ]` row of a gizmo map.
struct Entry {
    geo: Geo,
    slot: Slot,
    position: Option<[f64; 3]>,
    rotation: Option<[f64; 3]>,
    scale: Option<[f64; 3]>,
    helper: bool,
}

fn e(geo: Geo, slot: Slot, position: Option<[f64; 3]>, rotation: Option<[f64; 3]>) -> Entry {
    Entry {
        geo,
        slot,
        position,
        rotation,
        scale: None,
        helper: false,
    }
}

fn h(
    geo: Geo,
    position: Option<[f64; 3]>,
    rotation: Option<[f64; 3]>,
    scale: Option<[f64; 3]>,
) -> Entry {
    Entry {
        geo,
        slot: Slot::Helper,
        position,
        rotation,
        scale,
        helper: true,
    }
}

type GizmoMap = Vec<(&'static str, Vec<Entry>)>;

/// A handle: one child of a gizmo, picker or helper group.
#[derive(Clone, Debug)]
struct Handle {
    node: Node,
    /// `handle.name`.
    name: &'static str,
    /// `handle.tag === 'helper'`.
    helper: bool,
    slot: Slot,
}

/// One `setupGizmo()` result.
#[derive(Clone, Debug)]
struct Group {
    node: Node,
    /// The children in `children` order.
    handles: Vec<Handle>,
}

/// `TransformControlsGizmo`: three groups per mode.
#[derive(Clone, Debug)]
struct Gizmo {
    node: Node,
    gizmo: [Group; 3],
    picker: [Group; 3],
    helper: [Group; 3],
}

const HALF_PI: f64 = PI / 2.0;

fn gizmo_translate() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    vec![
        (
            "X",
            vec![
                e(
                    Arrow,
                    Red,
                    Some([0.5, 0.0, 0.0]),
                    Some([0.0, 0.0, -HALF_PI]),
                ),
                e(
                    Arrow,
                    Red,
                    Some([-0.5, 0.0, 0.0]),
                    Some([0.0, 0.0, HALF_PI]),
                ),
                e(
                    Line2,
                    Red,
                    Some([0.0, 0.0, 0.0]),
                    Some([0.0, 0.0, -HALF_PI]),
                ),
            ],
        ),
        (
            "Y",
            vec![
                e(Arrow, Green, Some([0.0, 0.5, 0.0]), None),
                e(Arrow, Green, Some([0.0, -0.5, 0.0]), Some([PI, 0.0, 0.0])),
                e(Line2, Green, None, None),
            ],
        ),
        (
            "Z",
            vec![
                e(
                    Arrow,
                    Blue,
                    Some([0.0, 0.0, 0.5]),
                    Some([HALF_PI, 0.0, 0.0]),
                ),
                e(
                    Arrow,
                    Blue,
                    Some([0.0, 0.0, -0.5]),
                    Some([-HALF_PI, 0.0, 0.0]),
                ),
                e(Line2, Blue, None, Some([HALF_PI, 0.0, 0.0])),
            ],
        ),
        (
            "XYZ",
            vec![e(
                Octahedron(0.1, 0),
                WhiteTransparent,
                Some([0.0, 0.0, 0.0]),
                None,
            )],
        ),
        (
            "XY",
            vec![e(
                Box(0.15, 0.15, 0.01),
                BlueTransparent,
                Some([0.15, 0.15, 0.0]),
                None,
            )],
        ),
        (
            "YZ",
            vec![e(
                Box(0.15, 0.15, 0.01),
                RedTransparent,
                Some([0.0, 0.15, 0.15]),
                Some([0.0, HALF_PI, 0.0]),
            )],
        ),
        (
            "XZ",
            vec![e(
                Box(0.15, 0.15, 0.01),
                GreenTransparent,
                Some([0.15, 0.0, 0.15]),
                Some([-HALF_PI, 0.0, 0.0]),
            )],
        ),
    ]
}

/// The arrow pickers `pickerTranslate` and `pickerScale` share.
fn picker_arrows() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    let cone = Cylinder(0.2, 0.0, 0.6, 4);
    vec![
        (
            "X",
            vec![
                e(
                    cone,
                    Invisible,
                    Some([0.3, 0.0, 0.0]),
                    Some([0.0, 0.0, -HALF_PI]),
                ),
                e(
                    cone,
                    Invisible,
                    Some([-0.3, 0.0, 0.0]),
                    Some([0.0, 0.0, HALF_PI]),
                ),
            ],
        ),
        (
            "Y",
            vec![
                e(cone, Invisible, Some([0.0, 0.3, 0.0]), None),
                e(
                    cone,
                    Invisible,
                    Some([0.0, -0.3, 0.0]),
                    Some([0.0, 0.0, PI]),
                ),
            ],
        ),
        (
            "Z",
            vec![
                e(
                    cone,
                    Invisible,
                    Some([0.0, 0.0, 0.3]),
                    Some([HALF_PI, 0.0, 0.0]),
                ),
                e(
                    cone,
                    Invisible,
                    Some([0.0, 0.0, -0.3]),
                    Some([-HALF_PI, 0.0, 0.0]),
                ),
            ],
        ),
    ]
}

fn picker_planes() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    let plane = Box(0.2, 0.2, 0.01);
    vec![
        (
            "XY",
            vec![e(plane, Invisible, Some([0.15, 0.15, 0.0]), None)],
        ),
        (
            "YZ",
            vec![e(
                plane,
                Invisible,
                Some([0.0, 0.15, 0.15]),
                Some([0.0, HALF_PI, 0.0]),
            )],
        ),
        (
            "XZ",
            vec![e(
                plane,
                Invisible,
                Some([0.15, 0.0, 0.15]),
                Some([-HALF_PI, 0.0, 0.0]),
            )],
        ),
    ]
}

fn picker_translate() -> GizmoMap {
    let mut map = picker_arrows();
    map.push((
        "XYZ",
        vec![e(Geo::Octahedron(0.2, 0), Slot::Invisible, None, None)],
    ));
    map.extend(picker_planes());
    map
}

/// The three axis lines `helperTranslate` and `helperScale` share.
fn helper_axes() -> GizmoMap {
    let long = Some([1e6, 1.0, 1.0]);
    vec![
        ("X", vec![h(Geo::Line, Some([-1e3, 0.0, 0.0]), None, long)]),
        (
            "Y",
            vec![h(
                Geo::Line,
                Some([0.0, -1e3, 0.0]),
                Some([0.0, 0.0, HALF_PI]),
                long,
            )],
        ),
        (
            "Z",
            vec![h(
                Geo::Line,
                Some([0.0, 0.0, -1e3]),
                Some([0.0, -HALF_PI, 0.0]),
                long,
            )],
        ),
    ]
}

fn helper_translate() -> GizmoMap {
    let mut map: GizmoMap = vec![
        ("START", vec![h(Geo::Octahedron(0.01, 2), None, None, None)]),
        ("END", vec![h(Geo::Octahedron(0.01, 2), None, None, None)]),
        ("DELTA", vec![h(Geo::TranslateHelper, None, None, None)]),
    ];
    map.extend(helper_axes());
    map
}

fn gizmo_rotate() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    vec![
        (
            "XYZE",
            vec![e(Circle(0.5, 1.0), Gray, None, Some([0.0, HALF_PI, 0.0]))],
        ),
        ("X", vec![e(Circle(0.5, 0.5), Red, None, None)]),
        (
            "Y",
            vec![e(Circle(0.5, 0.5), Green, None, Some([0.0, 0.0, -HALF_PI]))],
        ),
        (
            "Z",
            vec![e(Circle(0.5, 0.5), Blue, None, Some([0.0, HALF_PI, 0.0]))],
        ),
        (
            "E",
            vec![e(
                Circle(0.75, 1.0),
                YellowTransparent,
                None,
                Some([0.0, HALF_PI, 0.0]),
            )],
        ),
    ]
}

fn helper_rotate() -> GizmoMap {
    vec![(
        "AXIS",
        vec![h(
            Geo::Line,
            Some([-1e3, 0.0, 0.0]),
            None,
            Some([1e6, 1.0, 1.0]),
        )],
    )]
}

fn picker_rotate() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    let ring = Torus(0.5, 0.1, 4, 24);
    let origin = Some([0.0, 0.0, 0.0]);
    vec![
        ("XYZE", vec![e(Sphere(0.25, 10, 8), Invisible, None, None)]),
        (
            "X",
            vec![e(ring, Invisible, origin, Some([0.0, -HALF_PI, -HALF_PI]))],
        ),
        (
            "Y",
            vec![e(ring, Invisible, origin, Some([HALF_PI, 0.0, 0.0]))],
        ),
        (
            "Z",
            vec![e(ring, Invisible, origin, Some([0.0, 0.0, -HALF_PI]))],
        ),
        ("E", vec![e(Torus(0.75, 0.1, 2, 24), Invisible, None, None)]),
    ]
}

fn gizmo_scale() -> GizmoMap {
    use Geo::*;
    use Slot::*;
    let origin = Some([0.0, 0.0, 0.0]);
    vec![
        (
            "X",
            vec![
                e(
                    ScaleHandle,
                    Red,
                    Some([0.5, 0.0, 0.0]),
                    Some([0.0, 0.0, -HALF_PI]),
                ),
                e(Line2, Red, origin, Some([0.0, 0.0, -HALF_PI])),
                e(
                    ScaleHandle,
                    Red,
                    Some([-0.5, 0.0, 0.0]),
                    Some([0.0, 0.0, HALF_PI]),
                ),
            ],
        ),
        (
            "Y",
            vec![
                e(ScaleHandle, Green, Some([0.0, 0.5, 0.0]), None),
                e(Line2, Green, None, None),
                e(
                    ScaleHandle,
                    Green,
                    Some([0.0, -0.5, 0.0]),
                    Some([0.0, 0.0, PI]),
                ),
            ],
        ),
        (
            "Z",
            vec![
                e(
                    ScaleHandle,
                    Blue,
                    Some([0.0, 0.0, 0.5]),
                    Some([HALF_PI, 0.0, 0.0]),
                ),
                e(Line2, Blue, origin, Some([HALF_PI, 0.0, 0.0])),
                e(
                    ScaleHandle,
                    Blue,
                    Some([0.0, 0.0, -0.5]),
                    Some([-HALF_PI, 0.0, 0.0]),
                ),
            ],
        ),
        (
            "XY",
            vec![e(
                Box(0.15, 0.15, 0.01),
                BlueTransparent,
                Some([0.15, 0.15, 0.0]),
                None,
            )],
        ),
        (
            "YZ",
            vec![e(
                Box(0.15, 0.15, 0.01),
                RedTransparent,
                Some([0.0, 0.15, 0.15]),
                Some([0.0, HALF_PI, 0.0]),
            )],
        ),
        (
            "XZ",
            vec![e(
                Box(0.15, 0.15, 0.01),
                GreenTransparent,
                Some([0.15, 0.0, 0.15]),
                Some([-HALF_PI, 0.0, 0.0]),
            )],
        ),
        (
            "XYZ",
            vec![e(Box(0.1, 0.1, 0.1), WhiteTransparent, None, None)],
        ),
    ]
}

fn picker_scale() -> GizmoMap {
    let mut map = picker_arrows();
    map.extend(picker_planes());
    map.push((
        "XYZ",
        vec![e(
            Geo::Box(0.2, 0.2, 0.2),
            Slot::Invisible,
            Some([0.0, 0.0, 0.0]),
            None,
        )],
    ));
    map
}

/// The shared `gizmoMaterial` / `gizmoLineMaterial` settings with a slot's
/// colour and opacity. `toneMapped: false` has no counterpart.
fn material(slot: Slot, palette: &Palette) -> MeshBasicNodeMaterial {
    let mut m = if slot == Slot::Helper {
        MeshBasicNodeMaterial::line(palette.color(slot))
    } else {
        let mut m = MeshBasicNodeMaterial::new();
        m.color = palette.color(slot);
        m
    };
    m.depth_test = false;
    m.depth_write = false;
    m.fog = false;
    m.transparent = true;
    m.opacity = slot.opacity();
    m
}

/// `setupGizmo( gizmoMap )`: each row's geometry is cloned with the row's
/// offset baked in, and the rows of a name are added last to first.
fn setup_gizmo(map: GizmoMap, palette: &Palette) -> Group {
    let node = Object3D::new_node();
    let mut handles = Vec::new();
    for (name, entries) in map {
        for entry in entries.iter().rev() {
            let position = entry.position.unwrap_or([0.0; 3]);
            let mut quaternion = Quaternion::default();
            if let Some([x, y, z]) = entry.rotation {
                quaternion.set_from_euler(&Euler::new(x, y, z));
            }
            let scale = entry.scale.unwrap_or([1.0; 3]);
            let mut matrix = Matrix4::identity();
            matrix.compose(
                &Vector3::new(position[0], position[1], position[2]),
                &quaternion,
                &Vector3::new(scale[0], scale[1], scale[2]),
            );
            let mut geometry = entry.geo.build();
            geometry.apply_matrix4(&matrix);
            let geometry = Rc::new(geometry);
            let material = material(entry.slot, palette);
            let child = if entry.geo.is_line() {
                Line::new(geometry, material)
            } else {
                Mesh::new(geometry, material)
            };
            {
                let mut object = child.borrow_mut();
                object.name = name.to_string();
                object.render_order = f64::INFINITY;
            }
            node.add(&child);
            handles.push(Handle {
                node: child,
                name,
                helper: entry.helper,
                slot: entry.slot,
            });
        }
    }
    Group { node, handles }
}

impl Gizmo {
    fn new(palette: &Palette) -> Self {
        let node = Object3D {
            object_type: "TransformControlsGizmo",
            ..Default::default()
        }
        .into_node();
        let gizmo = [
            setup_gizmo(gizmo_translate(), palette),
            setup_gizmo(gizmo_rotate(), palette),
            setup_gizmo(gizmo_scale(), palette),
        ];
        let picker = [
            setup_gizmo(picker_translate(), palette),
            setup_gizmo(picker_rotate(), palette),
            setup_gizmo(picker_scale(), palette),
        ];
        let helper = [
            setup_gizmo(helper_translate(), palette),
            setup_gizmo(helper_rotate(), palette),
            setup_gizmo(helper_scale(), palette),
        ];
        for group in gizmo.iter().chain(&picker).chain(&helper) {
            node.add(&group.node);
        }
        // Pickers should be hidden always.
        for group in &picker {
            group.node.borrow_mut().visible = false;
        }
        Self {
            node,
            gizmo,
            picker,
            helper,
        }
    }
}

fn helper_scale() -> GizmoMap {
    helper_axes()
}

/// Write a handle's material colour and, if given, opacity.
fn set_material(node: &Node, color: Color, opacity: Option<f64>) {
    let mut object = node.borrow_mut();
    let material = match &mut object.payload {
        Payload::Mesh(mesh) => mesh.material.as_mut(),
        Payload::Line(line) => line.material.as_mut(),
        _ => None,
    };
    if let Some(material) = material {
        material.color = color;
        if let Some(opacity) = opacity {
            material.opacity = opacity;
        }
    }
}

/// `intersectObjectWithRay( object, raycaster, includeInvisible )`: the
/// nearest hit whose object is `visible`, or the nearest hit at all.
fn intersect_object_with_ray(
    object: &Node,
    raycaster: &Raycaster,
    include_invisible: bool,
) -> Option<crate::core::Intersection> {
    raycaster
        .intersect_object(object, true)
        .into_iter()
        .find(|hit| include_invisible || hit.object.borrow().visible)
}

/// Whether `node` is `ancestor` or below it.
fn is_within(node: &Node, ancestor: &Node) -> bool {
    let mut current = Some(node.clone());
    while let Some(n) = current {
        if Node::ptr_eq(&n, ancestor) {
            return true;
        }
        current = n.parent();
    }
    false
}

/// Set a `defineProperty` property: a new value dispatches `<name>-changed`
/// and then `change`.
fn set_property<T: PartialEq>(
    slot: &mut T,
    value: T,
    name: &'static str,
    events: &mut Vec<TransformControlsEvent>,
) {
    if *slot != value {
        *slot = value;
        events.push(TransformControlsEvent::PropertyChanged(name));
        events.push(TransformControlsEvent::Change);
    }
}

/// Translate, rotate or scale an object by dragging the handles of a gizmo
/// drawn on top of it.
///
/// A port of `examples/jsm/controls/TransformControls.js` as of the pinned
/// 5f610f5 (r187dev): the same properties with the same defaults, the same
/// `pointerHover` / `pointerDown` / `pointerMove` / `pointerUp`, the same
/// snapping and clamping, and the gizmo, picker and helper graphs that
/// `TransformControlsGizmo` builds, handle for handle, with the same baked
/// geometry and the same per-frame placement, visibility and highlight.
/// [`get_helper`](Self::get_helper) is the `TransformControlsRoot` to add to
/// the scene; it holds the gizmo and the invisible `TransformControlsPlane`
/// the drag is projected onto.
///
/// # Input, as calls
///
/// The JS registers `pointerdown`, `pointermove` and `pointerup` on the
/// element. Here the host calls [`on_pointer_down`](Self::on_pointer_down),
/// [`on_pointer_move`](Self::on_pointer_move) and
/// [`on_pointer_up`](Self::on_pointer_up), which do what the three handlers
/// do, `getPointer()` included: element-relative pixels become NDC through
/// [`set_element_size`](Self::set_element_size), [`viewport`](Self::viewport)
/// and [`pointer_locked`](Self::pointer_locked). A move hovers (mouse and pen
/// only) and then, between a down and an up, drags, as three's two
/// `pointermove` listeners do. The JS captures the pointer on down, so keep
/// sending moves and the up after the pointer leaves the element. The public
/// NDC methods, [`pointer_hover`](Self::pointer_hover) and the rest, are
/// there too, `null` being `None` (reuse the last ray).
///
/// # Rendering is [`update`](Self::update)
///
/// Three does its per-frame work in `updateMatrixWorld` overrides on the
/// root, the gizmo and the plane, which the renderer reaches through
/// `scene.updateMatrixWorld()`. The port's `Node` has no overrides, so the
/// host calls `update(&mut camera)` once per frame after the scene's matrices
/// are current and before rendering: it decomposes the object's and the
/// camera's matrices, cancels the helper's parent transform, places, shows
/// and highlights the current mode's handles, aligns the plane, and updates
/// the helper's matrices. The pointer methods raycast against the matrices
/// the last `update` left, as three's raycast against the last render's.
///
/// # The camera is an argument
///
/// Any [`TransformCamera`]: [`PerspectiveCamera`] or [`OrthographicCamera`].
/// The methods that read it take it; there is no `camera` property and no
/// `camera-changed` event.
///
/// # Events are return values
///
/// Every method three dispatches from returns the events in dispatch order:
/// `change`, `objectChange`, `mouseDown` / `mouseUp` with the mode, and the
/// `<name>-changed` events the property setters send. The properties three
/// routes through setters and the port keeps behind methods are `object`
/// ([`attach`](Self::attach) / [`detach`](Self::detach)), `axis`, `dragging`,
/// `mode`, `space`, `size`, the three snaps and `rotationAngle`. The rest
/// (`enabled`, `showX` ... `showE`, `minX` ... `maxZ`) are plain fields;
/// assigning one sends nothing, where three would send `<name>-changed` and
/// `change`. `worldPosition`, `eye` and the other working vectors are
/// read-only getters.
///
/// # What is left out, and what differs
///
/// - No listeners: `connect()`, `dispose()`, `setPointerCapture` and
///   `touchAction` are listener and DOM bookkeeping, and are left out.
///   [`disconnect`](Self::disconnect) is kept for the one listener whose
///   removal the port can see, the drag one: it stops moves from dragging, so
///   a host can end a drag from outside. As in three, it leaves `dragging`
///   and `axis` as they are.
/// - `getRaycaster()` returns this instance's raycaster. Three's is one
///   module-level `Raycaster` all instances share.
/// - Three shares one material between handles (one `matInvisible` for every
///   picker), so its highlight is last-writer-wins on a shared material. Each
///   handle here owns its material, and gets the colour three computes for
///   it. The visible handles end up identical: within a mode's gizmo group no
///   two handle names share a material. The pickers, which are never drawn,
///   differ: three's `matInvisible` ends the frame with whatever the last
///   picker wrote.
/// - `toneMapped: false` on the materials: the port's materials have no
///   tone-mapping switch.
/// - The `_dirVector` the plane is aimed with is per instance; three's is
///   module-level, so two instances in one page share the stale direction
///   the plane keeps while `axis` is `null` in translate or scale mode.
/// - An object with no parent: three's root override logs and carries on,
///   and so does this; three's `pointerDown` then throws on
///   `object.parent.updateMatrixWorld()`, where this skips the call.
/// - `setColors()` takes [`Color`]s, not any `Color.set()` argument.
/// - [`reset`](Self::reset) after [`detach`](Self::detach) mid-drag: three
///   throws on `this.object.position` before dispatching anything; this
///   returns no events and changes nothing, `pointStart` included.
#[derive(Debug)]
pub struct TransformControls {
    /// `enabled`. Off, the handlers and [`reset`](Self::reset) do nothing and
    /// nothing is highlighted.
    pub enabled: bool,
    /// `showX`: show the handles whose name contains X.
    pub show_x: bool,
    /// `showY`.
    pub show_y: bool,
    /// `showZ`.
    pub show_z: bool,
    /// `showXY`: show the handles whose name contains XY (`XYZ` and `XYZE`
    /// included, as in three).
    pub show_xy: bool,
    /// `showYZ`.
    pub show_yz: bool,
    /// `showXZ`.
    pub show_xz: bool,
    /// `showXYZE`.
    pub show_xyze: bool,
    /// `showE`.
    pub show_e: bool,
    /// `minX`: the translate clamp on `object.position.x`.
    pub min_x: f64,
    /// `maxX`.
    pub max_x: f64,
    /// `minY`.
    pub min_y: f64,
    /// `maxY`.
    pub max_y: f64,
    /// `minZ`.
    pub min_z: f64,
    /// `maxZ`.
    pub max_z: f64,
    /// `viewport`: `(x, y, width, height)` of the render region within the
    /// element, y up from the bottom, as `renderer.setViewport` takes it.
    /// `None` is the whole element.
    pub viewport: Option<Vector4>,
    /// `domElement.ownerDocument.pointerLockElement` is set: every pointer is
    /// the centre of the view.
    pub pointer_locked: bool,

    object: Option<Node>,
    axis: Option<Axis>,
    mode: Mode,
    translation_snap: Option<f64>,
    rotation_snap: Option<f64>,
    scale_snap: Option<f64>,
    space: Space,
    size: f64,
    dragging: bool,

    world_position: Vector3,
    world_position_start: Vector3,
    world_quaternion: Quaternion,
    world_quaternion_start: Quaternion,
    camera_position: Vector3,
    camera_quaternion: Quaternion,
    point_start: Vector3,
    point_end: Vector3,
    rotation_axis: Vector3,
    rotation_angle: f64,
    eye: Vector3,

    offset: Vector3,
    start_norm: Vector3,
    end_norm: Vector3,
    camera_scale: Vector3,
    parent_position: Vector3,
    parent_quaternion: Quaternion,
    parent_quaternion_inv: Quaternion,
    parent_scale: Vector3,
    world_scale_start: Vector3,
    world_quaternion_inv: Quaternion,
    world_scale: Vector3,
    position_start: Vector3,
    quaternion_start: Quaternion,
    scale_start: Vector3,
    /// The module-level `_dirVector`, which keeps its last value when
    /// `axis` names no plane orientation.
    dir_vector: Vector3,

    raycaster: Raycaster,
    root: Node,
    gizmo: Gizmo,
    plane: Node,
    palette: Palette,

    element_width: f64,
    element_height: f64,
    /// The `pointermove` → `_onPointerMove` listener is registered.
    moving: bool,
}

impl Default for TransformControls {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformControls {
    /// `new TransformControls( camera )`, with the camera passed to the
    /// methods instead.
    pub fn new() -> Self {
        let palette = Palette::default();
        let root = Object3D {
            visible: false,
            ..Default::default()
        }
        .into_node();
        let gizmo = Gizmo::new(&palette);
        root.add(&gizmo.node);

        let mut plane_material = MeshBasicNodeMaterial::new();
        plane_material.visible = false;
        plane_material.wireframe = true;
        plane_material.side = Side::Double;
        plane_material.transparent = true;
        plane_material.opacity = 0.1;
        let plane = Mesh::new(
            Rc::new(plane_geometry(100000.0, 100000.0, 2, 2)),
            plane_material,
        );
        plane.borrow_mut().object_type = "TransformControlsPlane";
        root.add(&plane);

        Self {
            enabled: true,
            show_x: true,
            show_y: true,
            show_z: true,
            show_xy: true,
            show_yz: true,
            show_xz: true,
            show_xyze: true,
            show_e: true,
            min_x: f64::NEG_INFINITY,
            max_x: f64::INFINITY,
            min_y: f64::NEG_INFINITY,
            max_y: f64::INFINITY,
            min_z: f64::NEG_INFINITY,
            max_z: f64::INFINITY,
            viewport: None,
            pointer_locked: false,
            object: None,
            axis: None,
            mode: Mode::Translate,
            translation_snap: None,
            rotation_snap: None,
            scale_snap: None,
            space: Space::World,
            size: 1.0,
            dragging: false,
            world_position: Vector3::ZERO,
            world_position_start: Vector3::ZERO,
            world_quaternion: Quaternion::default(),
            world_quaternion_start: Quaternion::default(),
            camera_position: Vector3::ZERO,
            camera_quaternion: Quaternion::default(),
            point_start: Vector3::ZERO,
            point_end: Vector3::ZERO,
            rotation_axis: Vector3::ZERO,
            rotation_angle: 0.0,
            eye: Vector3::ZERO,
            offset: Vector3::ZERO,
            start_norm: Vector3::ZERO,
            end_norm: Vector3::ZERO,
            camera_scale: Vector3::ZERO,
            parent_position: Vector3::ZERO,
            parent_quaternion: Quaternion::default(),
            parent_quaternion_inv: Quaternion::default(),
            parent_scale: Vector3::ZERO,
            world_scale_start: Vector3::ZERO,
            world_quaternion_inv: Quaternion::default(),
            world_scale: Vector3::ZERO,
            position_start: Vector3::ZERO,
            quaternion_start: Quaternion::default(),
            scale_start: Vector3::ZERO,
            dir_vector: Vector3::ZERO,
            raycaster: Raycaster::default(),
            root,
            gizmo,
            plane,
            palette,
            element_width: 0.0,
            element_height: 0.0,
            moving: false,
        }
    }

    /// The element's `getBoundingClientRect()` width and height, which
    /// `getPointer()` divides by.
    pub fn set_element_size(&mut self, width: f64, height: f64) {
        self.element_width = width;
        self.element_height = height;
    }

    /// `getHelper()`: the `TransformControlsRoot` to add to the scene.
    pub fn get_helper(&self) -> &Node {
        &self.root
    }

    /// `getRaycaster()`.
    pub fn get_raycaster(&mut self) -> &mut Raycaster {
        &mut self.raycaster
    }

    /// The `tag` three gives the handle `node`: `Some(Some("helper"))` for a
    /// helper group's handle, `Some(None)` for any other handle (three's
    /// `undefined`), and `None` if `node` is not one of the gizmo's handles.
    ///
    /// A test hook, so the graph test can hold the port's tags to three's;
    /// the port keeps the tag beside the handle rather than on the
    /// [`Node`], and nothing outside reads it.
    #[doc(hidden)]
    pub fn handle_tag(&self, node: &Node) -> Option<Option<&'static str>> {
        let gizmo = &self.gizmo;
        gizmo
            .gizmo
            .iter()
            .chain(&gizmo.picker)
            .chain(&gizmo.helper)
            .flat_map(|group| &group.handles)
            .find(|handle| Node::ptr_eq(&handle.node, node))
            .map(|handle| handle.helper.then_some("helper"))
    }

    /// `object`: the attached object.
    pub fn object(&self) -> Option<&Node> {
        self.object.as_ref()
    }

    /// `axis`: the handle hovered or dragged.
    pub fn axis(&self) -> Option<Axis> {
        self.axis
    }

    /// `getMode()`.
    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    /// `space`.
    pub fn space(&self) -> Space {
        self.space
    }

    /// `size`.
    pub fn size(&self) -> f64 {
        self.size
    }

    /// `dragging`.
    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// `translationSnap`.
    pub fn translation_snap(&self) -> Option<f64> {
        self.translation_snap
    }

    /// `rotationSnap`, in radians.
    pub fn rotation_snap(&self) -> Option<f64> {
        self.rotation_snap
    }

    /// `scaleSnap`.
    pub fn scale_snap(&self) -> Option<f64> {
        self.scale_snap
    }

    /// `worldPosition`: the object's world position at the last
    /// [`update`](Self::update).
    pub fn world_position(&self) -> Vector3 {
        self.world_position
    }

    /// `worldPositionStart`: the object's world position when the drag began.
    pub fn world_position_start(&self) -> Vector3 {
        self.world_position_start
    }

    /// `worldQuaternion`.
    pub fn world_quaternion(&self) -> Quaternion {
        self.world_quaternion
    }

    /// `worldQuaternionStart`.
    pub fn world_quaternion_start(&self) -> Quaternion {
        self.world_quaternion_start
    }

    /// `cameraPosition`.
    pub fn camera_position(&self) -> Vector3 {
        self.camera_position
    }

    /// `cameraQuaternion`.
    pub fn camera_quaternion(&self) -> Quaternion {
        self.camera_quaternion
    }

    /// `pointStart`: where the drag began on the plane, from
    /// `worldPositionStart`.
    pub fn point_start(&self) -> Vector3 {
        self.point_start
    }

    /// `pointEnd`: where the drag is now, from `worldPositionStart`.
    pub fn point_end(&self) -> Vector3 {
        self.point_end
    }

    /// `rotationAxis`.
    pub fn rotation_axis(&self) -> Vector3 {
        self.rotation_axis
    }

    /// `rotationAngle`.
    pub fn rotation_angle(&self) -> f64 {
        self.rotation_angle
    }

    /// `eye`: the unit direction from the object to the camera (the camera's
    /// backward axis for an orthographic camera).
    pub fn eye(&self) -> Vector3 {
        self.eye
    }

    /// `attach( object )`. The object must be in the scene graph.
    pub fn attach(&mut self, object: &Node) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        let same = self
            .object
            .as_ref()
            .is_some_and(|o| Node::ptr_eq(o, object));
        if !same {
            self.object = Some(object.clone());
            events.push(TransformControlsEvent::PropertyChanged("object"));
            events.push(TransformControlsEvent::Change);
        }
        self.root.borrow_mut().visible = true;
        events
    }

    /// `detach()`.
    pub fn detach(&mut self) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        if self.object.take().is_some() {
            events.push(TransformControlsEvent::PropertyChanged("object"));
            events.push(TransformControlsEvent::Change);
        }
        set_property(&mut self.axis, None, "axis", &mut events);
        self.root.borrow_mut().visible = false;
        events
    }

    /// `reset()`: mid-drag, put the object back where the drag began.
    ///
    /// After [`detach`](Self::detach) mid-drag, `dragging` is still `true`
    /// and three throws on `this.object.position` before dispatching
    /// anything; this returns no events and changes nothing.
    pub fn reset(&mut self) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        if !self.enabled || !self.dragging {
            return events;
        }
        let Some(object) = &self.object else {
            return events;
        };
        {
            let mut o = object.borrow_mut();
            o.position = self.position_start;
            o.quaternion = self.quaternion_start;
            o.sync_rotation_from_quaternion();
            o.scale = self.scale_start;
        }
        events.push(TransformControlsEvent::Change);
        events.push(TransformControlsEvent::ObjectChange);
        self.point_start = self.point_end;
        events
    }

    /// `setMode( mode )`.
    pub fn set_mode(&mut self, mode: Mode) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(&mut self.mode, mode, "mode", &mut events);
        events
    }

    /// `setTranslationSnap( translationSnap )`; `None` is `null`.
    pub fn set_translation_snap(&mut self, snap: Option<f64>) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(
            &mut self.translation_snap,
            snap,
            "translationSnap",
            &mut events,
        );
        events
    }

    /// `setRotationSnap( rotationSnap )`, in radians.
    pub fn set_rotation_snap(&mut self, snap: Option<f64>) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(&mut self.rotation_snap, snap, "rotationSnap", &mut events);
        events
    }

    /// `setScaleSnap( scaleSnap )`.
    pub fn set_scale_snap(&mut self, snap: Option<f64>) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(&mut self.scale_snap, snap, "scaleSnap", &mut events);
        events
    }

    /// `setSize( size )`.
    pub fn set_size(&mut self, size: f64) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(&mut self.size, size, "size", &mut events);
        events
    }

    /// `setSpace( space )`.
    pub fn set_space(&mut self, space: Space) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        set_property(&mut self.space, space, "space", &mut events);
        events
    }

    /// `setColors( xAxis, yAxis, zAxis, active )`. The handles' materials
    /// take the new colours at once, as three's shared materials do; a
    /// highlighted handle keeps its opacity until the next
    /// [`update`](Self::update), as in three.
    pub fn set_colors(&mut self, x_axis: Color, y_axis: Color, z_axis: Color, active: Color) {
        self.palette = Palette {
            x_axis,
            y_axis,
            z_axis,
            active,
        };
        let gizmo = &self.gizmo;
        for group in gizmo.gizmo.iter().chain(&gizmo.picker).chain(&gizmo.helper) {
            for handle in &group.handles {
                if matches!(
                    handle.slot,
                    Slot::Red
                        | Slot::Green
                        | Slot::Blue
                        | Slot::RedTransparent
                        | Slot::GreenTransparent
                        | Slot::BlueTransparent
                        | Slot::YellowTransparent
                ) {
                    set_material(&handle.node, self.palette.color(handle.slot), None);
                }
            }
        }
    }

    /// `getPointer( event )`.
    fn get_pointer(&self, event: &TransformPointerEvent) -> Pointer {
        if self.pointer_locked {
            return Pointer {
                x: 0.0,
                y: 0.0,
                button: event.button,
            };
        }
        let (origin_x, origin_y, region_width, region_height) = match self.viewport {
            Some(viewport) => (
                viewport.x,
                self.element_height - viewport.y - viewport.w,
                viewport.z,
                viewport.w,
            ),
            None => (0.0, 0.0, self.element_width, self.element_height),
        };
        Pointer {
            x: (event.x - origin_x) / region_width * 2.0 - 1.0,
            y: -(event.y - origin_y) / region_height * 2.0 + 1.0,
            button: event.button,
        }
    }

    /// The `pointerdown` handler.
    pub fn on_pointer_down(
        &mut self,
        event: &TransformPointerEvent,
        camera: &mut impl TransformCamera,
    ) -> Vec<TransformControlsEvent> {
        if !self.enabled {
            return Vec::new();
        }
        self.moving = true;
        let pointer = self.get_pointer(event);
        let mut events = self.pointer_hover(Some(&pointer), camera);
        events.extend(self.pointer_down(Some(&pointer), camera));
        events
    }

    /// The `pointermove` handlers: the hover listener (mouse and pen), then,
    /// between a down and an up, the drag listener.
    pub fn on_pointer_move(
        &mut self,
        event: &TransformPointerEvent,
        camera: &mut impl TransformCamera,
    ) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        if !self.enabled {
            return events;
        }
        if matches!(event.pointer_type, PointerType::Mouse | PointerType::Pen) {
            let pointer = self.get_pointer(event);
            events.extend(self.pointer_hover(Some(&pointer), camera));
        }
        if self.moving && self.enabled {
            let pointer = self.get_pointer(event);
            events.extend(self.pointer_move(Some(&pointer), camera));
        }
        events
    }

    /// `disconnect()`: drop the drag listener, so that
    /// [`on_pointer_move`](Self::on_pointer_move) only hovers until the next
    /// [`on_pointer_down`](Self::on_pointer_down).
    ///
    /// Three's `disconnect()` removes all four listeners and touches no
    /// state: a drag in progress keeps `dragging` and `axis` (and sends no
    /// `mouseUp`) until a later `pointerUp`. The other three listeners are the
    /// host's own calls here, so stopping them is the host's; what this
    /// mirrors is that a move after `disconnect()` no longer drags.
    pub fn disconnect(&mut self) {
        self.moving = false;
    }

    /// The `pointerup` handler.
    pub fn on_pointer_up(&mut self, event: &TransformPointerEvent) -> Vec<TransformControlsEvent> {
        if !self.enabled {
            return Vec::new();
        }
        self.moving = false;
        let pointer = self.get_pointer(event);
        self.pointer_up(Some(&pointer))
    }

    /// `pointerHover( pointer )`: set `axis` to the visible picker handle
    /// under the pointer.
    pub fn pointer_hover(
        &mut self,
        pointer: Option<&Pointer>,
        camera: &impl TransformCamera,
    ) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        if self.object.is_none() || self.dragging {
            return events;
        }
        if let Some(pointer) = pointer {
            self.raycaster
                .set_from_camera(&Vector2::new(pointer.x, pointer.y), camera);
        }
        let picker = &self.gizmo.picker[self.mode.index()].node;
        let axis = intersect_object_with_ray(picker, &self.raycaster, false)
            .and_then(|hit| Axis::from_name(&hit.object.borrow().name));
        set_property(&mut self.axis, axis, "axis", &mut events);
        events
    }

    /// `pointerDown( pointer )`: with a handle under the pointer, start a
    /// drag.
    pub fn pointer_down(
        &mut self,
        pointer: Option<&Pointer>,
        camera: &mut impl TransformCamera,
    ) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        let Some(object) = self.object.clone() else {
            return events;
        };
        if self.dragging || pointer.is_some_and(|p| p.button != 0) {
            return events;
        }
        if self.axis.is_none() {
            return events;
        }
        if let Some(pointer) = pointer {
            self.raycaster
                .set_from_camera(&Vector2::new(pointer.x, pointer.y), &*camera);
        }
        if let Some(hit) = intersect_object_with_ray(&self.plane, &self.raycaster, true) {
            object.update_matrix_world(false);
            if let Some(parent) = object.parent() {
                parent.update_matrix_world(false);
                if is_within(&self.root, &parent) {
                    // `parent.updateMatrixWorld()` reaches the helper's
                    // overrides when the helper is below the parent.
                    self.update(camera);
                }
            }
            {
                let o = object.borrow();
                self.position_start = o.position;
                self.quaternion_start = o.quaternion;
                self.scale_start = o.scale;
                o.matrix_world.decompose(
                    &mut self.world_position_start,
                    &mut self.world_quaternion_start,
                    &mut self.world_scale_start,
                );
            }
            self.point_start = hit.point;
            self.point_start.sub(&self.world_position_start);
        }
        set_property(&mut self.dragging, true, "dragging", &mut events);
        events.push(TransformControlsEvent::MouseDown(self.mode));
        events
    }

    /// `pointerMove( pointer )`: drag the object.
    pub fn pointer_move(
        &mut self,
        pointer: Option<&Pointer>,
        camera: &impl TransformCamera,
    ) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        let mode = self.mode;
        let mut space = self.space;
        if mode == Mode::Scale {
            space = Space::Local;
        } else if matches!(self.axis, Some(Axis::E | Axis::XYZE | Axis::XYZ)) {
            space = Space::World;
        }
        let (Some(object), Some(axis)) = (self.object.clone(), self.axis) else {
            return events;
        };
        if !self.dragging || pointer.is_some_and(|p| p.button != -1) {
            return events;
        }
        if let Some(pointer) = pointer {
            self.raycaster
                .set_from_camera(&Vector2::new(pointer.x, pointer.y), camera);
        }
        let Some(hit) = intersect_object_with_ray(&self.plane, &self.raycaster, true) else {
            return events;
        };
        self.point_end = hit.point;
        self.point_end.sub(&self.world_position_start);

        match mode {
            Mode::Translate => self.translate(&object, axis, space),
            Mode::Scale => self.scale(&object, axis),
            Mode::Rotate => self.rotate(&object, axis, space, camera, &mut events),
        }

        events.push(TransformControlsEvent::Change);
        events.push(TransformControlsEvent::ObjectChange);
        events
    }

    fn translate(&mut self, object: &Node, axis: Axis, space: Space) {
        self.offset = self.point_end;
        self.offset.sub(&self.point_start);
        if space == Space::Local && axis != Axis::XYZ {
            self.offset.apply_quaternion(&self.world_quaternion_inv);
        }
        if !axis.has("X") {
            self.offset.x = 0.0;
        }
        if !axis.has("Y") {
            self.offset.y = 0.0;
        }
        if !axis.has("Z") {
            self.offset.z = 0.0;
        }
        if space == Space::Local && axis != Axis::XYZ {
            self.offset.apply_quaternion(&self.quaternion_start);
        } else {
            self.offset.apply_quaternion(&self.parent_quaternion_inv);
        }
        self.offset.divide(&self.parent_scale);
        {
            let mut o = object.borrow_mut();
            o.position = self.offset;
            o.position.add(&self.position_start);
        }

        if let Some(snap) = snap_on(self.translation_snap) {
            if space == Space::Local {
                let mut inverse = self.quaternion_start;
                inverse.invert();
                let mut o = object.borrow_mut();
                o.position.apply_quaternion(&inverse);
                if axis.has("X") {
                    o.position.x = js_round(o.position.x / snap) * snap;
                }
                if axis.has("Y") {
                    o.position.y = js_round(o.position.y / snap) * snap;
                }
                if axis.has("Z") {
                    o.position.z = js_round(o.position.z / snap) * snap;
                }
                o.position.apply_quaternion(&self.quaternion_start);
            }
            if space == Space::World {
                let mut world = object.get_world_position();
                if axis.has("X") {
                    world.x = js_round(world.x / snap) * snap;
                }
                if axis.has("Y") {
                    world.y = js_round(world.y / snap) * snap;
                }
                if axis.has("Z") {
                    world.z = js_round(world.z / snap) * snap;
                }
                if let Some(parent) = object.parent() {
                    parent.world_to_local(&mut world);
                }
                object.borrow_mut().position = world;
            }
        }

        let mut o = object.borrow_mut();
        o.position.x = js_max(self.min_x, js_min(self.max_x, o.position.x));
        o.position.y = js_max(self.min_y, js_min(self.max_y, o.position.y));
        o.position.z = js_max(self.min_z, js_min(self.max_z, o.position.z));
    }

    fn scale(&mut self, object: &Node, axis: Axis) {
        let mut factor;
        if axis.has("XYZ") {
            let mut d = self.point_end.length() / self.point_start.length();
            if self.point_end.dot(&self.point_start) < 0.0 {
                d *= -1.0;
            }
            factor = Vector3::new(d, d, d);
        } else {
            let mut start = self.point_start;
            factor = self.point_end;
            start.apply_quaternion(&self.world_quaternion_inv);
            factor.apply_quaternion(&self.world_quaternion_inv);
            factor.divide(&start);
            if !axis.has("X") {
                factor.x = 1.0;
            }
            if !axis.has("Y") {
                factor.y = 1.0;
            }
            if !axis.has("Z") {
                factor.z = 1.0;
            }
        }

        let mut o = object.borrow_mut();
        o.scale = self.scale_start;
        o.scale.multiply(&factor);

        if let Some(snap) = snap_on(self.scale_snap) {
            if axis.has("X") {
                o.scale.x = or_number(js_round(o.scale.x / snap) * snap, snap);
            }
            if axis.has("Y") {
                o.scale.y = or_number(js_round(o.scale.y / snap) * snap, snap);
            }
            if axis.has("Z") {
                o.scale.z = or_number(js_round(o.scale.z / snap) * snap, snap);
            }
        }
    }

    fn rotate(
        &mut self,
        object: &Node,
        axis: Axis,
        space: Space,
        camera: &impl TransformCamera,
        events: &mut Vec<TransformControlsEvent>,
    ) {
        self.offset = self.point_end;
        self.offset.sub(&self.point_start);

        let mut camera_position = Vector3::ZERO;
        camera_position.set_from_matrix_position(&camera.matrix_world());
        let rotation_speed = 20.0 / self.world_position.distance_to(&camera_position);

        let mut in_plane_rotation = false;

        if axis == Axis::XYZE {
            self.rotation_axis = self.offset;
            self.rotation_axis.cross(&self.eye).normalize();
            let mut v = self.rotation_axis;
            v.cross(&self.eye);
            let angle = self.offset.dot(&v) * rotation_speed;
            set_property(&mut self.rotation_angle, angle, "rotationAngle", events);
        } else if matches!(axis, Axis::X | Axis::Y | Axis::Z) {
            let unit = match axis {
                Axis::X => UNIT_X,
                Axis::Y => UNIT_Y,
                _ => UNIT_Z,
            };
            self.rotation_axis = unit;
            let mut v = unit;
            if space == Space::Local {
                v.apply_quaternion(&self.world_quaternion);
            }
            v.cross(&self.eye);
            // When `v` is 0 after the cross with the eye, the two are
            // parallel and the in-plane rotation applies.
            if v.length() == 0.0 {
                in_plane_rotation = true;
            } else {
                v.normalize();
                let angle = self.offset.dot(&v) * rotation_speed;
                set_property(&mut self.rotation_angle, angle, "rotationAngle", events);
            }
        }

        if axis == Axis::E || in_plane_rotation {
            self.rotation_axis = self.eye;
            let angle = self.point_end.angle_to(&self.point_start);
            set_property(&mut self.rotation_angle, angle, "rotationAngle", events);
            self.start_norm = self.point_start;
            self.start_norm.normalize();
            self.end_norm = self.point_end;
            self.end_norm.normalize();
            self.end_norm.cross(&self.start_norm);
            let sign = if self.end_norm.dot(&self.eye) < 0.0 {
                1.0
            } else {
                -1.0
            };
            let angle = self.rotation_angle * sign;
            set_property(&mut self.rotation_angle, angle, "rotationAngle", events);
        }

        // Apply rotation snap.
        if let Some(snap) = snap_on(self.rotation_snap) {
            let angle = js_round(self.rotation_angle / snap) * snap;
            set_property(&mut self.rotation_angle, angle, "rotationAngle", events);
        }

        // Apply rotate.
        let mut q = Quaternion::default();
        if space == Space::Local && axis != Axis::E && axis != Axis::XYZE {
            q.set_from_axis_angle(&self.rotation_axis, self.rotation_angle);
            let mut o = object.borrow_mut();
            o.quaternion = self.quaternion_start;
            o.quaternion.multiply(&q).normalize();
            o.sync_rotation_from_quaternion();
        } else {
            self.rotation_axis
                .apply_quaternion(&self.parent_quaternion_inv);
            q.set_from_axis_angle(&self.rotation_axis, self.rotation_angle);
            let mut o = object.borrow_mut();
            o.quaternion = q;
            o.quaternion.multiply(&self.quaternion_start).normalize();
            o.sync_rotation_from_quaternion();
        }
    }

    /// `pointerUp( pointer )`: end the drag.
    pub fn pointer_up(&mut self, pointer: Option<&Pointer>) -> Vec<TransformControlsEvent> {
        let mut events = Vec::new();
        if pointer.is_some_and(|p| p.button != 0) {
            return events;
        }
        if self.dragging && self.axis.is_some() {
            events.push(TransformControlsEvent::MouseUp(self.mode));
        }
        set_property(&mut self.dragging, false, "dragging", &mut events);
        set_property(&mut self.axis, None, "axis", &mut events);
        events
    }

    /// What three's `updateMatrixWorld` overrides on `TransformControlsRoot`,
    /// `TransformControlsGizmo` and `TransformControlsPlane` do when the
    /// renderer updates the scene, then the helper's matrices. Call once per
    /// frame, after the scene's other matrices are current.
    pub fn update(&mut self, camera: &mut impl TransformCamera) {
        // TransformControlsRoot.
        if let Some(object) = &self.object {
            object.update_matrix_world(false);
            match object.parent() {
                None => eprintln!(
                    "TransformControls: The attached 3D object must be a part of the scene graph."
                ),
                Some(parent) => parent.borrow().matrix_world.decompose(
                    &mut self.parent_position,
                    &mut self.parent_quaternion,
                    &mut self.parent_scale,
                ),
            }
            object.borrow().matrix_world.decompose(
                &mut self.world_position,
                &mut self.world_quaternion,
                &mut self.world_scale,
            );
            self.parent_quaternion_inv = self.parent_quaternion;
            self.parent_quaternion_inv.invert();
            self.world_quaternion_inv = self.world_quaternion;
            self.world_quaternion_inv.invert();
        }

        camera.update_matrix_world();
        camera.matrix_world().decompose(
            &mut self.camera_position,
            &mut self.camera_quaternion,
            &mut self.camera_scale,
        );
        if camera.is_orthographic_camera() {
            // `camera.getWorldDirection( eye ).negate()`.
            let e = camera.matrix_world().elements;
            self.eye = Vector3::new(-e[8], -e[9], -e[10]);
            self.eye.normalize().negate();
        } else {
            self.eye = self.camera_position;
            self.eye.sub(&self.world_position).normalize();
        }

        // Cancel out the parent's transform so the gizmo stays world-aligned.
        if let Some(parent) = self.root.parent() {
            let mut m = parent.borrow().matrix_world;
            m.invert();
            let mut root = self.root.borrow_mut();
            let root = &mut *root;
            m.decompose(&mut root.position, &mut root.quaternion, &mut root.scale);
            root.sync_rotation_from_quaternion();
        }

        let factor = camera.gizmo_factor(self.world_position.distance_to(&self.camera_position));
        self.update_gizmo(factor);
        self.update_plane();

        self.root.update_matrix_world(false);
    }

    /// `TransformControlsGizmo.updateMatrixWorld()`, before the matrices.
    fn update_gizmo(&self, factor: f64) {
        let mode = self.mode;
        // Scale is always oriented to local rotation.
        let space = if mode == Mode::Scale {
            Space::Local
        } else {
            self.space
        };
        let quaternion = if space == Space::Local {
            self.world_quaternion
        } else {
            Quaternion::default()
        };
        let gizmo = &self.gizmo;

        // Show only gizmos for current transform mode.
        for m in [Mode::Translate, Mode::Rotate, Mode::Scale] {
            gizmo.gizmo[m.index()].node.borrow_mut().visible = mode == m;
            gizmo.helper[m.index()].node.borrow_mut().visible = mode == m;
        }

        let i = mode.index();
        let handles = gizmo.picker[i]
            .handles
            .iter()
            .chain(&gizmo.gizmo[i].handles)
            .chain(&gizmo.helper[i].handles);
        let axis = self.axis.map(Axis::as_str);
        let handle_scale = factor * self.size / 4.0;

        for handle in handles {
            let name = handle.name;
            let mut h = handle.node.borrow_mut();
            // hide aligned to camera
            h.visible = true;
            h.set_rotation(0.0, 0.0, 0.0);
            h.position = self.world_position;
            h.scale = Vector3::new(1.0, 1.0, 1.0);
            h.scale.multiply_scalar(handle_scale);

            if handle.helper {
                h.visible = false;
                if name == "AXIS" {
                    h.visible = axis.is_some();
                    let aligned = |unit: Vector3, euler: Euler, h: &mut Object3D| {
                        let mut q = Quaternion::default();
                        q.set_from_euler(&euler);
                        h.quaternion = quaternion;
                        h.quaternion.multiply(&q);
                        let mut align = unit;
                        align.apply_quaternion(&quaternion);
                        if align.dot(&self.eye).abs() > 0.9 {
                            h.visible = false;
                        }
                    };
                    match axis {
                        Some("X") => aligned(UNIT_X, Euler::new(0.0, 0.0, 0.0), &mut h),
                        Some("Y") => aligned(UNIT_Y, Euler::new(0.0, 0.0, HALF_PI), &mut h),
                        Some("Z") => aligned(UNIT_Z, Euler::new(0.0, HALF_PI, 0.0), &mut h),
                        Some("XYZE") => {
                            let mut q = Quaternion::default();
                            q.set_from_euler(&Euler::new(0.0, HALF_PI, 0.0));
                            let mut look = Matrix4::identity();
                            look.look_at(&Vector3::ZERO, &self.rotation_axis, &UNIT_Y);
                            h.quaternion.set_from_rotation_matrix(&look);
                            h.quaternion.multiply(&q);
                            h.visible = self.dragging;
                        }
                        Some("E") => h.visible = false,
                        _ => {}
                    }
                } else if name == "START" {
                    h.position = self.world_position_start;
                    h.visible = self.dragging;
                } else if name == "END" {
                    h.position = self.world_position;
                    h.visible = self.dragging;
                } else if name == "DELTA" {
                    h.position = self.world_position_start;
                    h.quaternion = self.world_quaternion_start;
                    let mut v = Vector3::new(1e-10, 1e-10, 1e-10);
                    v.add(&self.world_position_start)
                        .sub(&self.world_position)
                        .multiply_scalar(-1.0);
                    let mut inverse = self.world_quaternion_start;
                    inverse.invert();
                    v.apply_quaternion(&inverse);
                    h.scale = v;
                    h.visible = self.dragging;
                } else {
                    h.quaternion = quaternion;
                    h.position = if self.dragging {
                        self.world_position_start
                    } else {
                        self.world_position
                    };
                    if let Some(axis) = axis {
                        h.visible = axis.contains(name);
                    }
                }
                h.sync_rotation_from_quaternion();
                // If updating helper, skip rest of the loop.
                continue;
            }

            // Align handles to current local or world rotation.
            h.quaternion = quaternion;

            if mode == Mode::Translate || mode == Mode::Scale {
                // Hide translate and scale axis facing the camera.
                const AXIS_HIDE_THRESHOLD: f64 = 0.99;
                const PLANE_HIDE_THRESHOLD: f64 = 0.2;
                let facing = |unit: Vector3| {
                    let mut align = unit;
                    align.apply_quaternion(&quaternion);
                    align.dot(&self.eye).abs()
                };
                let hide = match name {
                    "X" => facing(UNIT_X) > AXIS_HIDE_THRESHOLD,
                    "Y" => facing(UNIT_Y) > AXIS_HIDE_THRESHOLD,
                    "Z" => facing(UNIT_Z) > AXIS_HIDE_THRESHOLD,
                    "XY" => facing(UNIT_Z) < PLANE_HIDE_THRESHOLD,
                    "YZ" => facing(UNIT_X) < PLANE_HIDE_THRESHOLD,
                    "XZ" => facing(UNIT_Y) < PLANE_HIDE_THRESHOLD,
                    _ => false,
                };
                if hide {
                    h.scale = Vector3::new(1e-10, 1e-10, 1e-10);
                    h.visible = false;
                }
            } else if mode == Mode::Rotate {
                // Align handles to current local or world rotation.
                let mut inverse = quaternion;
                inverse.invert();
                let mut align = self.eye;
                align.apply_quaternion(&inverse);

                if name.contains('E') {
                    let mut look = Matrix4::identity();
                    look.look_at(&self.eye, &Vector3::ZERO, &UNIT_Y);
                    h.quaternion.set_from_rotation_matrix(&look);
                }
                let turn = match name {
                    "X" => Some((UNIT_X, (-align.y).atan2(align.z))),
                    "Y" => Some((UNIT_Y, align.x.atan2(align.z))),
                    "Z" => Some((UNIT_Z, align.y.atan2(align.x))),
                    _ => None,
                };
                if let Some((unit, angle)) = turn {
                    let mut q = Quaternion::default();
                    q.set_from_axis_angle(&unit, angle);
                    let mut turned = Quaternion::default();
                    turned.multiply_quaternions(&quaternion, &q);
                    h.quaternion = turned;
                }
            }
            h.sync_rotation_from_quaternion();

            // Hide disabled axes.
            let has = |s: &str| name.contains(s);
            h.visible = h.visible && (!has("X") || self.show_x);
            h.visible = h.visible && (!has("Y") || self.show_y);
            h.visible = h.visible && (!has("Z") || self.show_z);
            h.visible = h.visible && (!has("E") || (self.show_x && self.show_y && self.show_z));

            // Hide disabled plane helpers.
            h.visible = h.visible && (!has("XY") || self.show_xy);
            h.visible = h.visible && (!has("YZ") || self.show_yz);
            h.visible = h.visible && (!has("XZ") || self.show_xz);

            // Hide disabled rotation helpers.
            h.visible = h.visible && (name != "E" || self.show_e);
            h.visible = h.visible && (name != "XYZE" || self.show_xyze);
            drop(h);

            // Highlight selected axis.
            let mut color = self.palette.color(handle.slot);
            let mut opacity = handle.slot.opacity();
            if let (true, Some(axis)) = (self.enabled, axis) {
                let selected =
                    name == axis || axis.chars().any(|a| name.len() == 1 && name.starts_with(a));
                if selected {
                    color = self.palette.active;
                    opacity = 1.0;
                }
            }
            set_material(&handle.node, color, Some(opacity));
        }
    }

    /// `TransformControlsPlane.updateMatrixWorld()`, before the matrices.
    fn update_plane(&mut self) {
        let mut space = self.space;
        if self.mode == Mode::Scale {
            // Scale is always oriented to local rotation.
            space = Space::Local;
        }
        let quaternion = if space == Space::Local {
            self.world_quaternion
        } else {
            Quaternion::default()
        };
        let mut v1 = UNIT_X;
        v1.apply_quaternion(&quaternion);
        let mut v2 = UNIT_Y;
        v2.apply_quaternion(&quaternion);
        let mut v3 = UNIT_Z;
        v3.apply_quaternion(&quaternion);

        // Align the plane for current transform mode, axis and space.
        let mut align = v2;
        match self.mode {
            Mode::Translate | Mode::Scale => match self.axis {
                Some(Axis::X) => {
                    align = self.eye;
                    align.cross(&v1);
                    self.dir_vector = v1;
                    self.dir_vector.cross(&align);
                }
                Some(Axis::Y) => {
                    align = self.eye;
                    align.cross(&v2);
                    self.dir_vector = v2;
                    self.dir_vector.cross(&align);
                }
                Some(Axis::Z) => {
                    align = self.eye;
                    align.cross(&v3);
                    self.dir_vector = v3;
                    self.dir_vector.cross(&align);
                }
                Some(Axis::XY) => self.dir_vector = v3,
                Some(Axis::YZ) => self.dir_vector = v1,
                Some(Axis::XZ) => {
                    align = v3;
                    self.dir_vector = v2;
                }
                Some(Axis::XYZ | Axis::E) => self.dir_vector = Vector3::ZERO,
                // `_dirVector` keeps whatever it last held.
                Some(Axis::XYZE) | None => {}
            },
            // Special case for rotate.
            Mode::Rotate => self.dir_vector = Vector3::ZERO,
        }

        let mut plane = self.plane.borrow_mut();
        plane.position = self.world_position;
        if self.dir_vector.length() == 0.0 {
            // If in rotate mode, make the plane parallel to camera.
            plane.quaternion = self.camera_quaternion;
        } else {
            let mut m = Matrix4::identity();
            m.look_at(&Vector3::ZERO, &self.dir_vector, &align);
            plane.quaternion.set_from_rotation_matrix(&m);
        }
        plane.sync_rotation_from_quaternion();
    }
}
