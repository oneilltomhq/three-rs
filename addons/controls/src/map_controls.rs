//! A camera over a [`Ground`] driven the way a map is driven: the ground is
//! what you grab.
//!
//! This is three.js' `MapControls` — `OrbitControls` with
//! `screenSpacePanning = false` and the preset
//! `{ LEFT: PAN, MIDDLE: DOLLY, RIGHT: ROTATE }` — with two changes. The orbit
//! target is not a free point in space but a point *on the ground*, carried in
//! ground coordinates; and the ground is a sphere, so when its radius comes
//! down the same controls are Google Earth rather than Google Maps.
//!
//! The damping is yomotsu's `camera-controls`
//! ([`smooth_damp`](crate::math::math_utils::smooth_damp), `smoothTime = 0.25`,
//! `draggingSmoothTime = 0.125`, `restThreshold = 0.01`), applied field by
//! field exactly as `CameraControls.update()` does.

use three_rs::cameras::PerspectiveCamera;
use three_rs::math::math_utils::{euclidean_modulo, DEG2RAD};
use three_rs::math::Vector3;

use crate::ground::{Frame, Ground};
use crate::smooth_damp::smooth_damp;

use std::f64::consts::{PI, TAU};

/// `CameraControls.smoothTime`.
pub const SMOOTH_TIME: f64 = 0.25;
/// `CameraControls.draggingSmoothTime`.
pub const DRAGGING_SMOOTH_TIME: f64 = 0.125;
/// `CameraControls.restThreshold`.
pub const REST_THRESHOLD: f64 = 0.01;
/// The smooth time a wheel notch is damped with. `camera-controls` has no such
/// thing — a notch eases over `smoothTime` — and `SMOOTH_TIME` keeps that; it
/// is a separate knob so a zoom can be made to snap without the release glide
/// going with it.
pub const WHEEL_SMOOTH_TIME: f64 = SMOOTH_TIME;

/// The three smooth times, so a demo can A/B the feel. Every one is
/// `CameraControls`' default until told otherwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damping {
    /// While nothing is held: the release glide, and the arrows.
    pub smooth_time: f64,
    /// While a mouse button is held.
    pub dragging_smooth_time: f64,
    /// For a while after a wheel notch, when nothing is held.
    pub wheel_smooth_time: f64,
    /// One wheel notch, as a factor on the distance: the sensitivity, as
    /// distinct from the ease. `0.95` is `OrbitControls`' `zoomSpeed = 1`.
    pub dolly_step: f64,
}

impl Default for Damping {
    fn default() -> Self {
        Self {
            smooth_time: SMOOTH_TIME,
            dragging_smooth_time: DRAGGING_SMOOTH_TIME,
            wheel_smooth_time: WHEEL_SMOOTH_TIME,
            dolly_step: DOLLY_STEP,
        }
    }
}

/// How long after a wheel notch `wheel_smooth_time` stays selected, as a
/// multiple of it: long enough for the notch to settle, so a zoom that starts
/// crisp does not finish soft.
const WHEEL_WINDOW: f64 = 3.0;

/// Below this difference a field is put on its target and its velocity zeroed,
/// so a settled controller is bit-stable and [`MapControls::update`] can report
/// "nothing moved".
const SNAP_THRESHOLD: f64 = 1e-6;

/// `OrbitControls.minDistance`.
pub const MIN_DISTANCE: f64 = 1.0;
/// `OrbitControls.maxDistance`.
pub const MAX_DISTANCE: f64 = 5000.0;

/// `OrbitControls.minPolarAngle`. Not `0`: there the offset is parallel to
/// `up` and `Matrix4.lookAt()` has to nudge itself out of the degeneracy.
pub const MIN_POLAR: f64 = 1e-3;
/// `OrbitControls.maxPolarAngle` — `misc_controls_map.html` uses `π / 2`, which
/// puts the eye on the ground. `85°` keeps the horizon in shot instead.
pub const MAX_POLAR: f64 = 85.0 * DEG2RAD;

/// One notch of the wheel, as a factor on the distance. `OrbitControls`'
/// `zoomSpeed = 1` works out at `0.95` per notch.
pub const DOLLY_STEP: f64 = 0.95;
/// Arrow-key pan, in ground units per second per unit of distance.
const PAN_SPEED: f64 = 0.4;

/// The polar angle the overview looks down at.
const OVERVIEW_POLAR: f64 = MIN_POLAR;
/// How far a pane floats above the ground, so that it is not z-fighting the
/// surface it lies on.
pub const PANE_LIFT: f64 = 0.05;

/// The overview fit leaves this much of the NDC square as margin —
/// `fitToBox`'s padding, expressed the way a projection test wants it.
const FIT_MARGIN: f64 = 0.9;
/// `fitToBox`'s binary search, on `log( distance )`.
const FIT_ITERATIONS: usize = 40;

/// How many times [`MapControls::grab_move`] and [`MapControls::dolly`] refine
/// the target. On a flat ground one step is exact; on a sphere the map from
/// "shift the target in ground coordinates" to "shift what is under the
/// cursor" is not the identity, and this closes the gap.
const SOLVE_ITERATIONS: usize = 24;

/// Where the camera is: an orbit target on the ground, and a direction and
/// distance from it.
///
/// `u` and `v` are [`Ground`] coordinates — arc lengths east and north of the
/// ground origin — and name the point the camera orbits and looks at.
/// `distance` is the length of the offset from it, in world units. `azimuth`
/// is radians about the ground normal there, `0` putting the camera south of
/// the target looking north and positive turning it toward east. `polar` is
/// the tilt away from that normal, `0` being straight down. There is no roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub u: f64,
    pub v: f64,
    pub distance: f64,
    pub azimuth: f64,
    pub polar: f64,
}

impl Pose {
    /// The pose with `distance` and `polar` forced into range and `azimuth`
    /// wrapped to `( -π, π ]`.
    pub fn clamped(self) -> Self {
        Self {
            u: self.u,
            v: self.v,
            distance: self.distance.clamp(MIN_DISTANCE, MAX_DISTANCE),
            azimuth: wrap_pi(self.azimuth),
            polar: self.polar.clamp(MIN_POLAR, MAX_POLAR),
        }
    }
}

/// Free movement, or the overview the pane set is fitted into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Free,
    Overview,
}

/// One rectangle lying flat on the ground, face up, for the overview to fit
/// and the pointer to pick.
///
/// Its centre is [`PANE_LIFT`] along the ground normal above
/// [`Ground::point`]`( u, v )`, its width runs along the frame's `east`, its
/// height along `north`, so its top edge is the northern one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pane {
    pub u: f64,
    pub v: f64,
    pub width: f64,
    pub height: f64,
}

/// One `smoothDamp` velocity per damped field. The distance and the ground
/// radius are damped in log space, so their velocities are in log units too.
#[derive(Clone, Copy, Debug, Default)]
struct Velocities {
    u: f64,
    v: f64,
    log_distance: f64,
    azimuth: f64,
    polar: f64,
    log_radius: f64,
}

/// A map camera over a [`Ground`], damped the way `camera-controls` damps.
///
/// Every input method edits [`MapControls::target`] only;
/// [`MapControls::update`] walks [`MapControls::current`] toward it and
/// [`MapControls::apply`] writes the result onto a camera.
#[derive(Clone, Debug)]
pub struct MapControls {
    ground: Ground,
    /// The radius the ground is being damped toward.
    target_radius: f64,
    target: Pose,
    current: Pose,
    velocities: Velocities,
    mode: Mode,
    return_pose: Option<Pose>,
    start: Pose,
    dragging: bool,
    damping: Damping,
    /// Seconds left in which a wheel notch selects `wheel_smooth_time`.
    wheel_window: f64,
    /// The ground coordinates of the point that was under the cursor when the
    /// left button went down.
    grabbed: Option<(f64, f64)>,
}

impl MapControls {
    /// Map controls over `ground`, settled at `start`.
    pub fn new(ground: Ground, start: Pose) -> Self {
        let start = start.clamped();
        Self {
            ground,
            target_radius: ground.radius(),
            target: start,
            current: start,
            velocities: Velocities::default(),
            mode: Mode::Free,
            return_pose: None,
            start,
            dragging: false,
            damping: Damping::default(),
            wheel_window: 0.0,
            grabbed: None,
        }
    }

    pub fn damping(&self) -> Damping {
        self.damping
    }

    pub fn set_damping(&mut self, damping: Damping) {
        self.damping = damping;
    }

    /// The ground as it is *now*, with the radius the damping has reached.
    pub fn ground(&self) -> &Ground {
        &self.ground
    }

    /// The radius the ground is heading for.
    pub fn target_radius(&self) -> f64 {
        self.target_radius
    }

    pub fn target(&self) -> Pose {
        self.target
    }

    pub fn current(&self) -> Pose {
        self.current
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Puts `current` on `target` and the ground on its target radius, with
    /// every velocity zeroed — the settled frame, for a headless render.
    pub fn settle(&mut self) {
        self.current = self.target;
        self.ground.set_radius(self.target_radius);
        self.velocities = Velocities::default();
    }

    // ------------------------------------------------------------ the inputs

    /// Selects the smooth time: `draggingSmoothTime` while a button is held.
    pub fn set_dragging(&mut self, dragging: bool) {
        self.dragging = dragging;
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    /// Takes hold of the ground under normalised device coordinates
    /// `( ndc_x, ndc_y )`, as `MapControls._handleMouseDownPan` does. Reports
    /// whether the cursor ray hit the ground at all; if it did not, the
    /// following [`MapControls::grab_move`] calls do nothing.
    pub fn grab_begin(&mut self, ndc_x: f64, ndc_y: f64, camera: &PerspectiveCamera) -> bool {
        self.grabbed = self.ground_under(&self.target, ndc_x, ndc_y, camera);
        self.grabbed.is_some()
    }

    /// Drags the grabbed point of ground under the cursor, as
    /// `MapControls._handleMouseMovePan` does.
    ///
    /// three.js casts the cursor ray once per move and shifts the target by
    /// minus the difference. That is exact against a plane whose normal is
    /// `camera.up`, because shifting the target there translates the whole ray
    /// field with it. Against a sphere in exponential-map coordinates it is
    /// only the first step of a fixed-point iteration, so this does the rest of
    /// the iteration too, and the grabbed point stays put at any curvature.
    pub fn grab_move(&mut self, ndc_x: f64, ndc_y: f64, camera: &PerspectiveCamera) {
        let Some(grabbed) = self.grabbed else {
            return;
        };
        if let Some((u, v)) = self.solve_target(self.target, grabbed, ndc_x, ndc_y, camera) {
            self.target.u = u;
            self.target.v = v;
        }
    }

    /// Lets go. The next [`MapControls::grab_begin`] takes a fresh hold.
    pub fn grab_end(&mut self) {
        self.grabbed = None;
    }

    /// An orbit drag of `dx`, `dy` pixels on a viewport `height` pixels tall,
    /// at `OrbitControls`' mapping of a full turn per viewport height — for
    /// both axes, which is what makes a diagonal drag feel square.
    ///
    /// Dragging right swings the camera clockwise as seen from above the
    /// target, and dragging down tips it overhead: `_rotateLeft` subtracts
    /// from `theta` and `_rotateUp` subtracts from `phi`, and with the ground
    /// frame's `azimuth` measured toward east those two come out as `+dx` and
    /// `-dy` here. See `a_rightward_drag_orbits_clockwise_from_above`.
    pub fn rotate(&mut self, dx: f64, dy: f64, height: f64) {
        let height = height.max(1.0);
        self.target.azimuth = wrap_pi(self.target.azimuth + TAU * dx / height);
        self.target.polar = (self.target.polar - TAU * dy / height).clamp(MIN_POLAR, MAX_POLAR);
    }

    /// `OrbitControls`' `zoomToCursor`: multiplies the distance by
    /// `0.95^steps` and then slides the target so that the ground under
    /// `( ndc_x, ndc_y )` is where it was. A ray that misses the ground dollies
    /// the distance alone.
    pub fn dolly(&mut self, steps: f64, ndc_x: f64, ndc_y: f64, camera: &PerspectiveCamera) {
        let before = self.ground_under(&self.target, ndc_x, ndc_y, camera);
        self.wheel_window = WHEEL_WINDOW * self.damping.wheel_smooth_time;

        self.target.distance = (self.target.distance * self.damping.dolly_step.powf(steps))
            .clamp(MIN_DISTANCE, MAX_DISTANCE);

        if let Some(anchor) = before {
            if let Some((u, v)) = self.solve_target(self.target, anchor, ndc_x, ndc_y, camera) {
                self.target.u = u;
                self.target.v = v;
            }
        }
    }

    /// The arrow keys: `forward` and `right` are in `[ -1, 1 ]` and move the
    /// camera over the ground at `0.4 * distance` units a second, so the ground
    /// goes by at the same *apparent* speed however far out the camera is.
    ///
    /// **Direction.** The brief says "along azimuth-rotated north and east".
    /// North it is; east is not, because `east = +X`, `north = +Z`,
    /// `normal = +Y` is a *left*-handed triple, so a camera looking north has
    /// its screen-right along `forward × up = +Z × +Y = -X`, i.e. along
    /// *minus* east. Arrow keys that pan the view the way the arrow points win
    /// over the transcription, as they did in the flight model. See the PR.
    pub fn pan(&mut self, forward: f64, right: f64, dt: f64) {
        if forward == 0.0 && right == 0.0 {
            return;
        }

        let speed = PAN_SPEED * self.target.distance * dt;
        let (sin, cos) = self.target.azimuth.sin_cos();

        // Screen-forward in ground coordinates is `( -sin, cos )` and
        // screen-right is `( -cos, -sin )`.
        self.target.u += speed * (forward * -sin + right * -cos);
        self.target.v += speed * (forward * cos + right * -sin);
    }

    /// The radius the ground is damped toward, clamped to the ground's range.
    pub fn set_radius(&mut self, radius: f64) {
        self.target_radius = Ground::new(radius).radius();
    }

    /// Multiplies that radius.
    pub fn scale_radius(&mut self, factor: f64) {
        self.set_radius(self.target_radius * factor);
    }

    /// Back to the pose the controller was built with, leaving the ground
    /// radius alone. Leaves the overview, since the overview pose is the one
    /// being replaced.
    pub fn reset(&mut self) {
        self.target = self.start;
        self.mode = Mode::Free;
    }

    // ------------------------------------------------------------ the damping

    /// Moves every field of `current` one step toward `target` — and the
    /// ground radius toward its own target — and reports whether anything
    /// moved.
    pub fn update(&mut self, dt: f64) -> bool {
        if dt <= 0.0 {
            return false;
        }

        let smooth_time = if self.dragging {
            self.damping.dragging_smooth_time
        } else if self.wheel_window > 0.0 {
            self.damping.wheel_smooth_time
        } else {
            self.damping.smooth_time
        };
        self.wheel_window = (self.wheel_window - dt).max(0.0);

        let mut moved = false;

        moved |= damp(
            &mut self.current.u,
            self.target.u,
            &mut self.velocities.u,
            smooth_time,
            dt,
        );
        moved |= damp(
            &mut self.current.v,
            self.target.v,
            &mut self.velocities.v,
            smooth_time,
            dt,
        );
        moved |= damp_log(
            &mut self.current.distance,
            self.target.distance,
            &mut self.velocities.log_distance,
            smooth_time,
            dt,
        );
        moved |= damp_angle(
            &mut self.current.azimuth,
            self.target.azimuth,
            &mut self.velocities.azimuth,
            smooth_time,
            dt,
        );
        moved |= damp(
            &mut self.current.polar,
            self.target.polar,
            &mut self.velocities.polar,
            smooth_time,
            dt,
        );

        let mut radius = self.ground.radius();
        moved |= damp_log(
            &mut radius,
            self.target_radius,
            &mut self.velocities.log_radius,
            smooth_time,
            dt,
        );
        self.ground.set_radius(radius);

        moved
    }

    /// `CameraControls`' `rest` event: every field is within `restThreshold`
    /// of its target — in log space for the distance and the ground radius,
    /// which is where those two are damped.
    pub fn rested(&self) -> bool {
        (self.current.u - self.target.u).abs() < REST_THRESHOLD
            && (self.current.v - self.target.v).abs() < REST_THRESHOLD
            && (self.current.distance.ln() - self.target.distance.ln()).abs() < REST_THRESHOLD
            && wrap_pi(self.target.azimuth - self.current.azimuth).abs() < REST_THRESHOLD
            && (self.current.polar - self.target.polar).abs() < REST_THRESHOLD
            && (self.ground.radius().ln() - self.target_radius.ln()).abs() < REST_THRESHOLD
    }

    // ------------------------------------------------------------- the camera

    /// Writes `current` onto `camera`: the position is the orbit target's
    /// ground point plus the spherical offset, `up` *is* the ground normal
    /// there, and the camera looks at the target.
    ///
    /// The no-roll invariant falls straight out of this: `Matrix4.lookAt()`
    /// builds the camera's right vector as `up × z`, so with `up` the ground
    /// normal the right vector is perpendicular to the normal by construction.
    pub fn apply(&self, camera: &mut PerspectiveCamera) {
        place(&self.ground, &self.current, camera);
    }

    // ---------------------------------------------------------- the overview

    /// GNOME's Super key: from [`Mode::Free`], saves the target pose and flies
    /// to a fitted, straight-down view of `panes`; from [`Mode::Overview`],
    /// flies back to the saved pose.
    ///
    /// `camera_fov` is the vertical field of view in degrees and `aspect` the
    /// viewport's width over its height — the fit is computed against them and
    /// against the ground as it is now, so it is right at any curvature. The
    /// azimuth is left alone: the overview is the same map, from overhead.
    pub fn toggle_overview(&mut self, panes: &[Pane], camera_fov: f64, aspect: f64) {
        match self.mode {
            Mode::Free => {
                self.return_pose = Some(self.target);

                let (u, v) = centroid(panes).unwrap_or((self.target.u, self.target.v));
                let azimuth = self.target.azimuth;
                let distance = self.fit_distance(panes, u, v, azimuth, camera_fov, aspect);

                self.target = Pose {
                    u,
                    v,
                    distance,
                    azimuth,
                    polar: OVERVIEW_POLAR,
                };
                self.mode = Mode::Overview;
            }
            Mode::Overview => {
                if let Some(pose) = self.return_pose {
                    self.target = pose;
                }
                self.mode = Mode::Free;
            }
        }
    }

    /// The smallest distance in `[ 1, 5000 ]` at which every *visible* pane
    /// projects inside NDC `[ -0.9, 0.9 ]`, found by 40 bisections on
    /// `log( distance )`. `5000` if nothing fits.
    ///
    /// A pane is visible when the camera is above its local horizon,
    /// `dot( ground normal at the pane, camera position - pane centre ) > 0`,
    /// so that on a tight sphere the panes round the back do not make the
    /// search run away to the ceiling. Nothing is rendered.
    pub fn fit_distance(
        &self,
        panes: &[Pane],
        u: f64,
        v: f64,
        azimuth: f64,
        camera_fov: f64,
        aspect: f64,
    ) -> f64 {
        if panes.is_empty() {
            return self.target.distance;
        }

        let mut camera = PerspectiveCamera::new(camera_fov, aspect, 0.1, 1e9);

        let mut fits = |distance: f64| -> bool {
            let pose = Pose {
                u,
                v,
                distance,
                azimuth,
                polar: OVERVIEW_POLAR,
            };
            place(&self.ground, &pose, &mut camera);
            let eye = camera.node.borrow().position;

            panes.iter().all(|pane| {
                let frame = self.ground.frame(pane.u, pane.v);
                let centre = lifted(&frame, PANE_LIFT);
                let mut to_eye = Vector3::ZERO;
                to_eye.sub_vectors(&eye, &centre);
                if to_eye.dot(&frame.normal) <= 0.0 {
                    // Over the horizon; it is not this pane's job to fit.
                    return true;
                }

                corners(&frame, pane)
                    .into_iter()
                    .all(|corner| match project(&mut camera, corner) {
                        Some((x, y, _)) => x.abs() <= FIT_MARGIN && y.abs() <= FIT_MARGIN,
                        None => false,
                    })
            })
        };

        if !fits(MAX_DISTANCE) {
            return MAX_DISTANCE;
        }

        let (mut low, mut high) = (MIN_DISTANCE.ln(), MAX_DISTANCE.ln());
        for _ in 0..FIT_ITERATIONS {
            let middle = 0.5 * (low + high);
            if fits(middle.exp()) {
                high = middle;
            } else {
                low = middle;
            }
        }

        high.exp()
    }

    /// Which pane the point at normalised device coordinates `( ndc_x, ndc_y )`
    /// is over, if any: each pane's four corners are projected and the point
    /// tested against the convex quad they make, nearest first. No raycaster.
    pub fn pick(
        &self,
        panes: &[Pane],
        ndc_x: f64,
        ndc_y: f64,
        camera: &PerspectiveCamera,
    ) -> Option<usize> {
        let mut camera = camera.clone();
        let mut best: Option<(usize, f64)> = None;

        for (index, pane) in panes.iter().enumerate() {
            let frame = self.ground.frame(pane.u, pane.v);

            let mut polygon = [(0.0, 0.0); 4];
            let mut depth = 0.0;
            let mut visible = true;
            for (slot, corner) in corners(&frame, pane).into_iter().enumerate() {
                match project(&mut camera, corner) {
                    Some((x, y, z)) => {
                        polygon[slot] = (x, y);
                        depth += z * 0.25;
                    }
                    None => {
                        visible = false;
                        break;
                    }
                }
            }
            if !visible || !inside_convex(&polygon, ndc_x, ndc_y) {
                continue;
            }

            if best.is_none_or(|(_, best_depth)| depth < best_depth) {
                best = Some((index, depth));
            }
        }

        best.map(|(index, _)| index)
    }

    /// Drops onto one pane: the orbit target moves to the pane's ground point,
    /// straight down with north up, at the distance that fills the view with
    /// the pane — [`MapControls::fit_distance`] of that one pane, against
    /// `camera_fov` and `aspect`. The pose becomes the one a later
    /// [`MapControls::toggle_overview`] would return to, and the controller
    /// flies to it exactly as leaving the overview does.
    pub fn focus_pane(&mut self, pane: &Pane, camera_fov: f64, aspect: f64) {
        let distance = self.fit_distance(&[*pane], pane.u, pane.v, 0.0, camera_fov, aspect);
        let pose = Pose {
            u: pane.u,
            v: pane.v,
            distance,
            azimuth: 0.0,
            polar: OVERVIEW_POLAR,
        }
        .clamped();

        self.return_pose = Some(pose);
        self.target = pose;
        self.mode = Mode::Free;
    }

    /// The four world-space corners of a pane, in polygon order. Public because
    /// the caller that draws the panes wants the same frame the picker used.
    pub fn pane_corners(&self, pane: &Pane) -> [Vector3; 4] {
        corners(&self.ground.frame(pane.u, pane.v), pane)
    }

    /// The world-space centre of a pane.
    pub fn pane_centre(&self, pane: &Pane) -> Vector3 {
        lifted(&self.ground.frame(pane.u, pane.v), PANE_LIFT)
    }

    // ------------------------------------------------------------ the picking

    /// The ground coordinates the cursor ray hits, with the camera put at
    /// `pose` — the camera passed in is only read for its projection, so the
    /// caller may hand over one that is mid-flight.
    ///
    /// `None` when the ray misses the ground entirely, which off the edge of
    /// the horizon it does.
    pub fn ground_under(
        &self,
        pose: &Pose,
        ndc_x: f64,
        ndc_y: f64,
        camera: &PerspectiveCamera,
    ) -> Option<(f64, f64)> {
        let mut scratch = camera.clone();
        place(&self.ground, pose, &mut scratch);

        let (origin, direction) = cursor_ray(&scratch, ndc_x, ndc_y);
        let hit = intersect_sphere(
            origin,
            direction,
            self.ground.centre(),
            self.ground.radius(),
        )?;
        Some(ground_coordinates(
            hit,
            self.ground.centre(),
            self.ground.radius(),
        ))
    }

    /// The orbit target that puts ground coordinate `anchor` under
    /// `( ndc_x, ndc_y )`, starting from `pose` and keeping its distance,
    /// azimuth and polar.
    fn solve_target(
        &self,
        pose: Pose,
        anchor: (f64, f64),
        ndc_x: f64,
        ndc_y: f64,
        camera: &PerspectiveCamera,
    ) -> Option<(f64, f64)> {
        let mut pose = pose;
        let mut best: Option<((f64, f64), f64)> = None;

        for _ in 0..SOLVE_ITERATIONS {
            let (u, v) = self.ground_under(&pose, ndc_x, ndc_y, camera)?;
            let (du, dv) = (u - anchor.0, v - anchor.1);
            let residual = du.hypot(dv);

            if best.is_none_or(|(_, previous)| residual < previous) {
                best = Some(((pose.u, pose.v), residual));
            } else {
                // Not converging — the ray is grazing the limb, where the
                // ground coordinate is a wild function of the pose. Keep the
                // closest pass and stop.
                break;
            }

            if residual < 1e-12 {
                break;
            }
            pose.u -= du;
            pose.v -= dv;
        }

        best.map(|(uv, _)| uv)
    }
}

// ------------------------------------------------------------------ helpers

/// `MapControls::apply` for an arbitrary pose, so the overview's fit and the
/// cursor picking can try one on a scratch camera without disturbing the
/// controller.
fn place(ground: &Ground, pose: &Pose, camera: &mut PerspectiveCamera) {
    let frame = ground.frame(pose.u, pose.v);

    let (sin_azimuth, cos_azimuth) = pose.azimuth.sin_cos();
    let (sin_polar, cos_polar) = pose.polar.sin_cos();

    // The offset from the target to the eye. At `azimuth = 0` its horizontal
    // part is `-north`, which is the camera south of the target looking north.
    let mut offset = Vector3::ZERO;
    offset.add_scaled_vector(&frame.north, -pose.distance * sin_polar * cos_azimuth);
    offset.add_scaled_vector(&frame.east, pose.distance * sin_polar * sin_azimuth);
    offset.add_scaled_vector(&frame.normal, pose.distance * cos_polar);

    let mut position = frame.origin;
    position.add(&offset);

    {
        let mut object = camera.node.borrow_mut();
        object.up = frame.normal;
        object.position = position;
    }
    camera.look_at(&frame.origin);
    camera.update_matrix_world();
}

/// The world-space ray through normalised device coordinates
/// `( ndc_x, ndc_y )`, as `Raycaster.setFromCamera` builds it for a
/// perspective camera. The camera's matrices must be up to date.
fn cursor_ray(camera: &PerspectiveCamera, ndc_x: f64, ndc_y: f64) -> (Vector3, Vector3) {
    let elements = camera.node.borrow().matrix_world.elements;
    let origin = Vector3::new(elements[12], elements[13], elements[14]);

    let mut far = Vector3::new(ndc_x, ndc_y, 0.5);
    far.unproject(camera);

    let mut direction = Vector3::ZERO;
    direction.sub_vectors(&far, &origin);
    direction.normalize();

    (origin, direction)
}

/// The nearer intersection of a ray with a sphere, or `None` if it misses or
/// the sphere is behind.
///
/// The near root is written `c / ( -b + sqrt( disc ) )` rather than
/// `-b - sqrt( disc )`: with `R = 1e7` and a camera 160 units up, `b²` and
/// `disc` agree to eleven digits and the subtraction throws away all of them.
fn intersect_sphere(
    origin: Vector3,
    direction: Vector3,
    centre: Vector3,
    radius: f64,
) -> Option<Vector3> {
    let mut m = Vector3::ZERO;
    m.sub_vectors(&origin, &centre);

    let length = m.length();
    // `|m|² - R²` factored, so that at `R = 1e7` the height above the surface
    // survives instead of being rounded off against `1e14`.
    let c = (length - radius) * (length + radius);
    let b = m.dot(&direction);

    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let t = if b < 0.0 { c / (-b + root) } else { -b - root };
    if t <= 0.0 {
        return None;
    }

    let mut hit = origin;
    hit.add_scaled_vector(&direction, t);
    Some(hit)
}

/// The inverse of [`Ground::point`]: the exponential map read backwards.
///
/// ```text
/// q       = normalize( hit - centre )
/// d       = R * atan2( |q.xz|, q.y )
/// ( u, v ) = d * q.xz / |q.xz|
/// ```
fn ground_coordinates(hit: Vector3, centre: Vector3, radius: f64) -> (f64, f64) {
    let mut q = Vector3::ZERO;
    q.sub_vectors(&hit, &centre);
    q.normalize();

    let horizontal = q.x.hypot(q.z);
    if horizontal == 0.0 {
        return (0.0, 0.0);
    }

    let d = radius * horizontal.atan2(q.y);
    (d * q.x / horizontal, d * q.z / horizontal)
}

/// The ground point at a frame, lifted `distance` along its normal.
fn lifted(frame: &Frame, distance: f64) -> Vector3 {
    let mut point = frame.origin;
    point.add_scaled_vector(&frame.normal, distance);
    point
}

/// A pane's four corners in polygon order: south-west, south-east,
/// north-east, north-west.
fn corners(frame: &Frame, pane: &Pane) -> [Vector3; 4] {
    let centre = lifted(frame, PANE_LIFT);
    let (half_width, half_height) = (pane.width * 0.5, pane.height * 0.5);

    let corner = |along: f64, up: f64| {
        let mut point = centre;
        point.add_scaled_vector(&frame.east, along * half_width);
        point.add_scaled_vector(&frame.north, up * half_height);
        point
    };

    [
        corner(-1.0, -1.0),
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
    ]
}

/// `( ndc.x, ndc.y, depth )` for a world point, or `None` when it is behind
/// the camera — where the perspective divide flips the sign and the NDC square
/// stops meaning anything. The camera's matrices must be up to date.
fn project(camera: &mut PerspectiveCamera, world: Vector3) -> Option<(f64, f64, f64)> {
    let mut view = world;
    view.apply_matrix4(&camera.matrix_world_inverse);
    let depth = -view.z;
    if depth <= 0.0 {
        return None;
    }

    let mut ndc = world;
    ndc.project(camera);
    Some((ndc.x, ndc.y, depth))
}

/// Point in convex polygon: every edge has the point on the same side.
fn inside_convex(polygon: &[(f64, f64); 4], x: f64, y: f64) -> bool {
    let mut positive = false;
    let mut negative = false;

    for i in 0..4 {
        let (ax, ay) = polygon[i];
        let (bx, by) = polygon[(i + 1) % 4];
        let cross = (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        if cross > 0.0 {
            positive = true;
        } else if cross < 0.0 {
            negative = true;
        }
    }

    !(positive && negative)
}

/// The mean of the panes' ground coordinates.
fn centroid(panes: &[Pane]) -> Option<(f64, f64)> {
    if panes.is_empty() {
        return None;
    }
    let count = panes.len() as f64;
    let u = panes.iter().map(|pane| pane.u).sum::<f64>() / count;
    let v = panes.iter().map(|pane| pane.v).sum::<f64>() / count;
    Some((u, v))
}

/// An angle wrapped into `( -π, π ]`.
fn wrap_pi(angle: f64) -> f64 {
    PI - euclidean_modulo(PI - angle, TAU)
}

/// One damped field. Returns whether it moved.
fn damp(current: &mut f64, target: f64, velocity: &mut f64, smooth_time: f64, dt: f64) -> bool {
    if (target - *current).abs() < SNAP_THRESHOLD {
        let moved = *current != target;
        *current = target;
        *velocity = 0.0;
        return moved;
    }

    let next = smooth_damp(*current, target, velocity, smooth_time, f64::INFINITY, dt);
    let moved = next != *current;
    *current = next;
    moved
}

/// A field damped in log space — the distance and the ground radius, where a
/// constant *ratio* per second is what reads as constant speed.
fn damp_log(current: &mut f64, target: f64, velocity: &mut f64, smooth_time: f64, dt: f64) -> bool {
    let mut log_current = current.ln();
    let log_target = target.ln();
    let moved = damp(&mut log_current, log_target, velocity, smooth_time, dt);

    *current = if log_current == log_target {
        target
    } else {
        log_current.exp()
    };

    moved
}

/// An angle damped along the shortest way round, kept in `( -π, π ]`.
fn damp_angle(
    current: &mut f64,
    target: f64,
    velocity: &mut f64,
    smooth_time: f64,
    dt: f64,
) -> bool {
    let delta = wrap_pi(target - *current);
    if delta.abs() < SNAP_THRESHOLD {
        let wrapped = wrap_pi(target);
        let moved = *current != wrapped;
        *current = wrapped;
        *velocity = 0.0;
        return moved;
    }

    // The target is restated as "this far from where we are", so the damping
    // never takes the long way round even when the two straddle ±π.
    let next = smooth_damp(
        *current,
        *current + delta,
        velocity,
        smooth_time,
        f64::INFINITY,
        dt,
    );
    let moved = next != *current;
    *current = wrap_pi(next);
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use three_rs::math::RAD2DEG;

    const FOV: f64 = 60.0;
    const WIDTH: f64 = 1600.0;
    const HEIGHT: f64 = 1000.0;
    const ASPECT: f64 = WIDTH / HEIGHT;

    /// The demo's start pose.
    fn start() -> Pose {
        Pose {
            u: 0.0,
            v: 0.0,
            distance: 160.0,
            azimuth: 0.0,
            polar: 0.0,
        }
    }

    fn camera() -> PerspectiveCamera {
        PerspectiveCamera::new(FOV, ASPECT, 0.1, 1e6)
    }

    /// The demo's 7×7 layout.
    fn demo_panes() -> Vec<Pane> {
        let mut panes = Vec::new();
        for row in 0..7 {
            for column in 0..7 {
                panes.push(Pane {
                    u: (column as f64 - 3.0) * 50.0,
                    v: (row as f64 - 3.0) * 50.0,
                    width: 16.0,
                    height: 9.0,
                });
            }
        }
        panes
    }

    fn poses() -> Vec<Pose> {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..20)
            .map(|_| Pose {
                u: next() * 600.0 - 300.0,
                v: next() * 600.0 - 300.0,
                distance: 1.0 + next() * 400.0,
                azimuth: next() * TAU - PI,
                polar: MIN_POLAR + next() * (MAX_POLAR - MIN_POLAR),
            })
            .collect()
    }

    /// The whole point: the camera's right vector never leaves the ground's
    /// tangent plane at the target, so the horizon never tilts.
    #[test]
    fn the_camera_never_rolls() {
        for radius in [40.0, 300.0, 1e7] {
            let ground = Ground::new(radius);
            for pose in poses() {
                let mut controls = MapControls::new(ground, pose);
                controls.settle();

                let mut camera = camera();
                controls.apply(&mut camera);

                let normal = ground.normal(pose.u, pose.v);
                let elements = camera.node.borrow().matrix_world.elements;
                let right = Vector3::new(elements[0], elements[1], elements[2]);

                let roll = right.dot(&normal);
                assert!(
                    roll.abs() < 1e-9,
                    "R = {radius}, pose {pose:?}: right · normal = {roll}"
                );
            }
        }
    }

    /// `azimuth = 0` is the camera south of the target, looking north.
    #[test]
    fn the_zero_azimuth_looks_north() {
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.settle();

        let mut camera = camera();
        controls.apply(&mut camera);

        let elements = camera.node.borrow().matrix_world.elements;
        let position = Vector3::new(elements[12], elements[13], elements[14]);
        let forward = Vector3::new(-elements[8], -elements[9], -elements[10]);

        assert!(position.z < 0.0, "the camera is not south of the target");
        assert!(position.x.abs() < 1e-9);
        assert!(forward.z > 0.0, "the camera is not looking north");
    }

    /// `OrbitControls`' rotate sense. A rightward drag turns the camera
    /// clockwise about the target's ground normal as seen from above it —
    /// positive rotation about `+Y` carries `+Z` to `+X` and *appears*
    /// anticlockwise from the tip of the axis, so clockwise is the negative
    /// sense and `cross( before, after ) · normal` must come out below zero.
    #[test]
    fn a_rightward_drag_orbits_clockwise_from_above() {
        let ground = Ground::new(1e7);
        // Tilted, so that the orbit has a sense and the tip has room to go.
        let tilted = Pose {
            polar: 45.0 * DEG2RAD,
            ..start()
        };
        let mut controls = MapControls::new(ground, tilted);
        controls.settle();

        let mut camera = camera();
        controls.apply(&mut camera);
        let before = {
            let elements = camera.node.borrow().matrix_world.elements;
            Vector3::new(elements[12], elements[13], elements[14])
        };

        controls.rotate(100.0, 0.0, HEIGHT);
        controls.settle();
        controls.apply(&mut camera);
        let after = {
            let elements = camera.node.borrow().matrix_world.elements;
            Vector3::new(elements[12], elements[13], elements[14])
        };

        let normal = ground.normal(0.0, 0.0);
        let sense = before.crossed(&after).dot(&normal);
        assert!(
            sense < 0.0,
            "a rightward drag orbited anticlockwise from above: {sense}"
        );

        // And a downward drag tips the camera overhead, as `_rotateUp` does.
        let polar = controls.target().polar;
        controls.rotate(0.0, 100.0, HEIGHT);
        assert!(controls.target().polar < polar);
    }

    /// The map invariant: whatever piece of ground was under the cursor when
    /// the button went down is still under it when the drag ends, at any
    /// curvature and any tilt.
    #[test]
    fn grabbing_the_ground_pins_it_under_the_cursor() {
        // 200 px right and 200 px down on a 1600 × 1000 viewport, starting
        // from the middle of the frame. Down, because at `polar = 80°` on a
        // ground 300 units across the top of the frame is sky, and there is
        // nothing up there to grab.
        let (dx, dy) = (2.0 * 200.0 / WIDTH, -2.0 * 200.0 / HEIGHT);
        let (from_x, from_y) = (-0.1, 0.0);

        for radius in [1e7, 300.0] {
            for polar_degrees in [20.0, 45.0, 80.0] {
                for distance in [160.0, 400.0] {
                    let pose = Pose {
                        polar: polar_degrees * DEG2RAD,
                        distance,
                        ..start()
                    };
                    let mut controls = MapControls::new(Ground::new(radius), pose);
                    controls.settle();

                    let mut camera = camera();
                    controls.apply(&mut camera);

                    assert!(
                        controls.grab_begin(from_x, from_y, &camera),
                        "R = {radius}, polar {polar_degrees}°: the press missed the ground"
                    );
                    let anchor = controls
                        .ground_under(&controls.target(), from_x, from_y, &camera)
                        .expect("the press hit");

                    controls.grab_move(from_x + dx, from_y + dy, &camera);
                    controls.settle();
                    controls.apply(&mut camera);

                    let landed = controls
                        .ground_under(&controls.target(), from_x + dx, from_y + dy, &camera)
                        .expect("the ground is still under the cursor");

                    let error = (landed.0 - anchor.0).hypot(landed.1 - anchor.1);
                    assert!(
                        error < 1e-6,
                        "R = {radius}, polar {polar_degrees}°, d = {distance}: \
                         the grabbed point slipped by {error}"
                    );
                }
            }
        }
    }

    /// And it slips the way the hand goes: drag right, the ground goes right.
    #[test]
    fn dragging_right_moves_the_ground_right() {
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.settle();

        let mut camera = camera();
        controls.apply(&mut camera);

        controls.grab_begin(0.0, 0.0, &camera);
        let grabbed = controls
            .ground()
            .point(controls.target().u, controls.target().v);

        controls.grab_move(0.3, 0.0, &camera);
        controls.settle();
        controls.apply(&mut camera);

        let (x, y, _) = project(&mut camera, grabbed).expect("still in shot");
        assert!(
            (x - 0.3).abs() < 1e-6 && y.abs() < 1e-6,
            "the grabbed ground landed at ( {x}, {y} ), not under the cursor"
        );
    }

    /// `zoomToCursor`: the wheel pulls the ground under the pointer toward the
    /// eye and leaves it where it was on screen.
    #[test]
    fn the_wheel_dollies_toward_the_cursor() {
        let (ndc_x, ndc_y) = (0.5, -0.3);

        for radius in [1e7, 300.0] {
            let mut controls = MapControls::new(Ground::new(radius), start());
            controls.settle();

            let mut camera = camera();
            controls.apply(&mut camera);

            let anchor = controls
                .ground_under(&controls.target(), ndc_x, ndc_y, &camera)
                .expect("the pointer is over the ground");

            controls.dolly(3.0, ndc_x, ndc_y, &camera);
            controls.settle();
            controls.apply(&mut camera);

            assert!(
                controls.target().distance < 160.0,
                "the wheel did not zoom in"
            );

            let landed = controls
                .ground_under(&controls.target(), ndc_x, ndc_y, &camera)
                .expect("still over the ground");
            let error = (landed.0 - anchor.0).hypot(landed.1 - anchor.1);
            assert!(error < 1e-4, "R = {radius}: the ground slipped by {error}");
        }
    }

    /// [`ground_coordinates`] is the inverse of [`Ground::point`] over the
    /// whole useful range, out to nine tenths of the way to the far pole.
    ///
    /// The tolerance is the brief's `1e-9` with a relative floor: at
    /// `R = 1e7` a `1e-9` *absolute* error on an arc length of `1.4e7` would
    /// be a sixteenth of a double's precision, which no arithmetic can hold.
    #[test]
    fn the_inverse_exponential_map_round_trips() {
        for radius in [40.0, 300.0, 1000.0, 1e7] {
            let ground = Ground::new(radius);
            let centre = ground.centre();
            let limit = 0.9 * PI * radius / 2.0;

            let mut worst: f64 = 0.0;
            for i in 0..=12 {
                for j in 0..=12 {
                    let u = limit * (i as f64 / 6.0 - 1.0);
                    let v = limit * (j as f64 / 6.0 - 1.0);

                    let (back_u, back_v) = ground_coordinates(ground.point(u, v), centre, radius);
                    let error = (back_u - u).hypot(back_v - v);
                    let tolerance = 1e-9_f64.max(1e-12 * u.hypot(v));
                    assert!(
                        error <= tolerance,
                        "R = {radius}, ( {u}, {v} ) came back as \
                         ( {back_u}, {back_v} ), off by {error}"
                    );
                    worst = worst.max(error);
                }
            }
            assert!(worst.is_finite());
        }
    }

    #[test]
    fn polar_is_clamped_after_a_rotate() {
        let mut controls = MapControls::new(Ground::new(1e7), start());

        for _ in 0..200 {
            controls.rotate(0.0, 100.0, HEIGHT);
        }
        assert_eq!(controls.target().polar, MIN_POLAR);

        for _ in 0..200 {
            controls.rotate(0.0, -100.0, HEIGHT);
        }
        assert_eq!(controls.target().polar, MAX_POLAR);
    }

    #[test]
    fn distance_and_radius_are_clamped() {
        let mut controls = MapControls::new(Ground::new(1e7), start());
        let camera = camera();

        controls.dolly(-1000.0, 0.0, 0.0, &camera);
        assert_eq!(controls.target().distance, MAX_DISTANCE);
        controls.dolly(1000.0, 0.0, 0.0, &camera);
        assert_eq!(controls.target().distance, MIN_DISTANCE);

        controls.set_radius(1.0);
        assert_eq!(controls.target_radius(), 40.0);
        controls.set_radius(1e20);
        assert_eq!(controls.target_radius(), 1e7);
    }

    #[test]
    fn azimuth_damping_takes_the_short_way_round() {
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.target.azimuth = 179.0 * DEG2RAD;
        controls.settle();
        controls.target.azimuth = -179.0 * DEG2RAD;

        let mut crossed_the_back = false;
        for _ in 0..600 {
            controls.update(1.0 / 60.0);
            let azimuth = controls.current().azimuth * RAD2DEG;
            assert!(
                azimuth.abs() > 90.0,
                "the azimuth went the long way round, through {azimuth}°"
            );
            if azimuth.abs() > 179.5 {
                crossed_the_back = true;
            }
            if controls.rested() {
                break;
            }
        }

        assert!(crossed_the_back, "the azimuth never passed ±180°");
        // `rested()` is `restThreshold` = 0.01 rad away from the target, and
        // that last 0.01 rad may sit either side of ±180°, so the comparison
        // has to wrap too.
        let left = wrap_pi(controls.current().azimuth - -179.0 * DEG2RAD).abs();
        assert!(left < 0.011, "{}° short of the target", left * RAD2DEG);
    }

    /// Arrow keys move the view the way the arrow points, and the step scales
    /// with the distance.
    #[test]
    fn the_arrow_keys_pan_the_view() {
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.pan(1.0, 0.0, 1.0);
        assert!((controls.target().v - PAN_SPEED * 160.0).abs() < 1e-12);
        assert!(controls.target().u.abs() < 1e-12);

        // Facing north (`+Z`), screen-right is `-X`, i.e. `-u`: the frame is
        // left-handed, and the arrow is read as "pan the view right".
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.pan(0.0, 1.0, 1.0);
        assert!((controls.target().u - -PAN_SPEED * 160.0).abs() < 1e-12);
    }

    #[test]
    fn the_overview_settles_and_then_stops() {
        let panes = demo_panes();
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.toggle_overview(&panes, FOV, ASPECT);

        assert_eq!(controls.target().polar, OVERVIEW_POLAR);
        assert!(
            !controls.rested(),
            "the overview pose is not where we already are"
        );

        let mut rested_after = None;
        for frame in 1..=180 {
            controls.update(1.0 / 60.0);
            if controls.rested() {
                rested_after = Some(frame);
                break;
            }
        }
        assert!(
            rested_after.is_some(),
            "the overview never rested within 3 seconds"
        );

        let mut stopped = false;
        for _ in 0..300 {
            if !controls.update(1.0 / 60.0) {
                stopped = true;
                break;
            }
        }
        assert!(stopped, "`update` kept reporting movement after the rest");
        assert_eq!(controls.current(), controls.target());
    }

    #[test]
    fn toggling_twice_returns_the_saved_pose_exactly() {
        let panes = demo_panes();
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.rotate(37.0, -11.0, HEIGHT);
        controls.pan(1.0, 0.5, 0.3);
        let saved = controls.target();

        controls.toggle_overview(&panes, FOV, ASPECT);
        assert_eq!(controls.mode(), Mode::Overview);
        assert_ne!(controls.target(), saved);

        controls.toggle_overview(&panes, FOV, ASPECT);
        assert_eq!(controls.mode(), Mode::Free);
        assert_eq!(controls.target(), saved);
    }

    /// On the flat ground the fit has a closed form: the camera looks straight
    /// down from `distance` at a rectangle `extent_u` by `extent_v`, so it must
    /// be far enough back that the half-extent divides by the tangent of the
    /// half field of view — then divided by the `0.9` NDC margin.
    ///
    /// The extents are the pane centres' span plus a whole pane: `6 * 50 + 16`
    /// across, `6 * 50 + 9` along, since the panes lie flat.
    #[test]
    fn the_flat_fit_matches_the_closed_form() {
        let panes = demo_panes();
        let controls = MapControls::new(Ground::new(1e7), start());
        let distance = controls.fit_distance(&panes, 0.0, 0.0, 0.0, FOV, ASPECT);

        let half_v = (FOV * DEG2RAD) * 0.5;
        let half_h = (half_v.tan() * ASPECT).atan();
        let extent_u = 6.0 * 50.0 + 16.0;
        let extent_v = 6.0 * 50.0 + 9.0;
        let closed_form =
            (extent_u / (2.0 * half_h.tan())).max(extent_v / (2.0 * half_v.tan())) / FIT_MARGIN;

        let error = (distance - closed_form).abs() / closed_form;
        assert!(
            error < 0.05,
            "fit {distance} is {:.1}% off the closed form {closed_form}",
            error * 100.0
        );
    }

    /// **Deviation from the brief, on purpose** — carried over from the flight
    /// model, where the brief asked for the `R = 300` fit to come out *above*
    /// the flat one. It comes out below, and the geometry says it must: the
    /// ground is a ball, so a pane `s` units of arc from the centroid sits only
    /// `R sin( s / R ) < s` away horizontally *and* `R ( 1 - cos( s / R ) )`
    /// further down, both of which shrink the angle it subtends at a camera
    /// over the pole. Curling the ground up pulls the layout in.
    #[test]
    fn a_curved_ground_needs_less_distance_than_a_flat_one() {
        let panes = demo_panes();
        let mut previous = f64::INFINITY;

        for radius in [1e7, 3000.0, 1000.0, 600.0, 300.0] {
            let distance = MapControls::new(Ground::new(radius), start())
                .fit_distance(&panes, 0.0, 0.0, 0.0, FOV, ASPECT);
            assert!(
                distance < previous,
                "R = {radius} fit {distance} is not below the previous {previous}"
            );
            previous = distance;
        }
    }

    #[test]
    fn picking_hits_the_pane_the_cursor_is_over() {
        let panes = demo_panes();
        let ground = Ground::new(1e7);

        // Focused on the middle pane, which is index 24 of the 7×7.
        let middle = panes[24];
        let mut controls = MapControls::new(ground, start());
        controls.focus_pane(&middle, FOV, ASPECT);
        controls.settle();

        let mut camera = camera();
        controls.apply(&mut camera);

        let centre = controls.pane_centre(&middle);
        let (x, y, _) = project(&mut camera, centre).expect("the pane is in front");
        assert_eq!(controls.pick(&panes, x, y, &camera), Some(24));
        // And well off to the side there is nothing but ground.
        assert_eq!(controls.pick(&panes, -0.99, -0.99, &camera), None);
    }

    #[test]
    fn focus_pane_fills_the_view_with_the_pane() {
        let panes = demo_panes();
        let pane = panes[10];
        let mut controls = MapControls::new(Ground::new(1e7), start());
        controls.focus_pane(&pane, FOV, ASPECT);
        controls.settle();

        assert_eq!(controls.target().u, pane.u);
        assert_eq!(controls.target().v, pane.v);
        assert_eq!(controls.target().polar, OVERVIEW_POLAR);
        assert_eq!(controls.target().azimuth, 0.0);

        let mut camera = camera();
        controls.apply(&mut camera);

        // The camera looks straight down at the pane's centre — to within the
        // `MIN_POLAR` tilt that keeps `lookAt` out of its degeneracy.
        let centre = controls.pane_centre(&pane);
        let (x, y, _) = project(&mut camera, centre).expect("the pane is in front");
        assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "off axis at ( {x}, {y} )");

        // Every corner is inside the fit margin, and the widest one is on it:
        // the pane fills the view.
        let mut widest = 0.0_f64;
        for corner in controls.pane_corners(&pane) {
            let (x, y, _) = project(&mut camera, corner).expect("in front");
            assert!(
                x.abs() <= FIT_MARGIN + 1e-6 && y.abs() <= FIT_MARGIN + 1e-6,
                "corner at ( {x}, {y} )"
            );
            widest = widest.max(x.abs()).max(y.abs());
        }
        assert!(
            (widest - FIT_MARGIN).abs() < 1e-3,
            "the pane stops at {widest}, not at the margin {FIT_MARGIN}"
        );

        // And north is up: the northern corners are the top ones. (Which side
        // east lands on is the frame's handedness, and is not asserted here.)
        let [_, _, north_east, north_west] = controls.pane_corners(&pane);
        for corner in [north_east, north_west] {
            let (x, y, _) = project(&mut camera, corner).expect("in front");
            assert!(
                y > 0.0,
                "a northern corner at ( {x}, {y} ) is not at the top"
            );
        }
    }

    /// A ray that misses the ground leaves the target where it is.
    #[test]
    fn a_ray_off_the_edge_of_the_world_changes_nothing() {
        let mut controls = MapControls::new(Ground::new(40.0), start());
        controls.settle();

        let mut camera = camera();
        controls.apply(&mut camera);

        // Straight up, away from a ground only 40 units across.
        assert!(!controls.grab_begin(0.0, 0.99, &camera));
        let before = controls.target();
        controls.grab_move(0.5, 0.99, &camera);
        assert_eq!(controls.target(), before);
    }
}
