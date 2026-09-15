//! A camera that moves like a helicopter over a [`Ground`]: it never rolls,
//! and "up" is always the ground normal under it.
//!
//! The damping is yomotsu's `camera-controls`
//! ([`smooth_damp`](crate::math::math_utils::smooth_damp), `smoothTime = 0.25`,
//! `draggingSmoothTime = 0.125`, `restThreshold = 0.01`), applied field by
//! field exactly as `CameraControls.update()` does. The *pose* model is not:
//! there is no spherical orbit here, no target point and no roll — a
//! [`Pose`] is a place on the ground, a height above it, and where the pilot is
//! looking.

use crate::cameras::PerspectiveCamera;
use crate::controls::ground::{Frame, Ground};
use crate::math::math_utils::{euclidean_modulo, smooth_damp, DEG2RAD};
use crate::math::Vector3;

use std::f64::consts::{PI, TAU};

/// `CameraControls.smoothTime`.
pub const SMOOTH_TIME: f64 = 0.25;
/// `CameraControls.draggingSmoothTime`.
pub const DRAGGING_SMOOTH_TIME: f64 = 0.125;
/// `CameraControls.restThreshold`.
pub const REST_THRESHOLD: f64 = 0.01;

/// Below this difference a field is put on its target and its velocity zeroed,
/// so a settled controller is bit-stable and [`Helicopter::update`] can report
/// "nothing moved".
const SNAP_THRESHOLD: f64 = 1e-6;

/// The lowest the camera may fly, in world units.
pub const MIN_ALTITUDE: f64 = 1.0;
/// The highest the camera may fly, in world units.
pub const MAX_ALTITUDE: f64 = 5000.0;

/// Pitch floor. Not `-90°`: there `forward` is parallel to `up` and
/// `Matrix4.lookAt()` has to nudge itself out of the degeneracy.
pub const MIN_PITCH: f64 = -89.0 * DEG2RAD;
/// Pitch ceiling — the horizon is meant to stay in shot.
pub const MAX_PITCH: f64 = 20.0 * DEG2RAD;

/// `first-person.html`'s rotate speed.
const LOOK_SPEED: f64 = 0.3;
/// Ground units per second, per unit of altitude, per unit of input.
const MOVE_SPEED: f64 = 1.2;
/// One notch of the wheel, or one press of `Q` / `E`.
const CLIMB_STEP: f64 = 1.15;

/// The pitch the overview looks down at.
const OVERVIEW_PITCH: f64 = MIN_PITCH;
/// The overview fit leaves this much of the NDC square as margin —
/// `fitToBox`'s padding, expressed the way a projection test wants it.
const FIT_MARGIN: f64 = 0.9;
/// `fitToBox`'s binary search, on `log( altitude )`.
const FIT_ITERATIONS: usize = 40;

/// Where the helicopter is and where it is looking.
///
/// `u` and `v` are [`Ground`] coordinates — arc lengths east and north of the
/// ground origin. `altitude` is along the ground normal. `yaw` is radians about
/// that normal with `0` facing north and positive turning toward east; `pitch`
/// is radians above the horizon. There is no roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub u: f64,
    pub v: f64,
    pub altitude: f64,
    pub yaw: f64,
    pub pitch: f64,
}

impl Pose {
    /// The pose with `altitude` and `pitch` forced into range and `yaw` wrapped
    /// to `( -π, π ]`.
    pub fn clamped(self) -> Self {
        Self {
            u: self.u,
            v: self.v,
            altitude: self.altitude.clamp(MIN_ALTITUDE, MAX_ALTITUDE),
            yaw: wrap_pi(self.yaw),
            pitch: self.pitch.clamp(MIN_PITCH, MAX_PITCH),
        }
    }
}

/// Free flight, or the overview the pane set is fitted into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Free,
    Overview,
}

/// One upright rectangle standing on the ground, for the overview to fit and
/// the pointer to pick.
///
/// Its centre is `height / 2` along the ground normal above
/// [`Ground::point`]`( u, v )`, its width runs along the frame's `east` and it
/// faces `-north`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pane {
    pub u: f64,
    pub v: f64,
    pub width: f64,
    pub height: f64,
}

/// One `smoothDamp` velocity per damped field. Altitude and the ground radius
/// are damped in log space, so their velocities are in log units too.
#[derive(Clone, Copy, Debug, Default)]
struct Velocities {
    u: f64,
    v: f64,
    log_altitude: f64,
    yaw: f64,
    pitch: f64,
    log_radius: f64,
}

/// A camera over a [`Ground`], damped the way `camera-controls` damps.
///
/// Every input method edits [`Helicopter::target`] only; [`Helicopter::update`]
/// walks [`Helicopter::current`] toward it and [`Helicopter::apply`] writes the
/// result onto a camera.
#[derive(Clone, Debug)]
pub struct Helicopter {
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
}

impl Helicopter {
    /// A helicopter over `ground`, settled at `start`.
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
        }
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

    /// Flies over the ground: `forward` along the heading, `right` across it,
    /// at `1.2 * altitude` units per second per unit of input — so the ground
    /// goes by at the same *apparent* speed however high the camera is.
    pub fn move_ground(&mut self, forward: f64, right: f64, dt: f64) {
        if forward == 0.0 && right == 0.0 {
            return;
        }

        let speed = MOVE_SPEED * self.target.altitude * dt;
        let (sin, cos) = (self.target.yaw.sin(), self.target.yaw.cos());

        // The heading in ground coordinates is `( sin yaw, cos yaw )`: `+v` is
        // north, `+u` is east. Screen-right is `forward × up`, which with
        // `up = normal` and `forward = north` is `-east` — so `right` runs
        // along `( -cos yaw, sin yaw )`, the heading turned a quarter turn the
        // way the pilot's right hand points.
        self.target.u += speed * (forward * sin - right * cos);
        self.target.v += speed * (forward * cos + right * sin);
    }

    /// Multiplies the target altitude by `1.15^steps`.
    pub fn climb(&mut self, steps: f64) {
        self.target.altitude =
            (self.target.altitude * CLIMB_STEP.powf(steps)).clamp(MIN_ALTITUDE, MAX_ALTITUDE);
    }

    /// A look-around drag of `dx`, `dy` pixels on a viewport `height` pixels
    /// tall, at `first-person.html`'s rotate speed of `0.3`.
    ///
    /// **Sign.** The brief writes `yaw += 0.3 * 2π * dx / height`, with the
    /// stated intent "dragging right turns right, matching first-person.html".
    /// Those two disagree, because `east = +X`, `north = +Z`, `normal = +Y` is
    /// a *left*-handed triple: a camera at the ground origin facing north has
    /// its screen-right along `forward × up = +Z × +Y = -X`, i.e. along
    /// `-east`. Turning right therefore *decreases* a yaw that is positive
    /// toward east, and the intent wins over the transcription. See the PR.
    pub fn look(&mut self, dx: f64, dy: f64, height: f64) {
        let height = height.max(1.0);
        self.target.yaw = wrap_pi(self.target.yaw - LOOK_SPEED * TAU * dx / height);
        self.target.pitch =
            (self.target.pitch - LOOK_SPEED * PI * dy / height).clamp(MIN_PITCH, MAX_PITCH);
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
            DRAGGING_SMOOTH_TIME
        } else {
            SMOOTH_TIME
        };

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
            &mut self.current.altitude,
            self.target.altitude,
            &mut self.velocities.log_altitude,
            smooth_time,
            dt,
        );
        moved |= damp_angle(
            &mut self.current.yaw,
            self.target.yaw,
            &mut self.velocities.yaw,
            smooth_time,
            dt,
        );
        moved |= damp(
            &mut self.current.pitch,
            self.target.pitch,
            &mut self.velocities.pitch,
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
    /// of its target — in log space for the altitude and the ground radius,
    /// which is where those two are damped.
    pub fn rested(&self) -> bool {
        (self.current.u - self.target.u).abs() < REST_THRESHOLD
            && (self.current.v - self.target.v).abs() < REST_THRESHOLD
            && (self.current.altitude.ln() - self.target.altitude.ln()).abs() < REST_THRESHOLD
            && wrap_pi(self.target.yaw - self.current.yaw).abs() < REST_THRESHOLD
            && (self.current.pitch - self.target.pitch).abs() < REST_THRESHOLD
            && (self.ground.radius().ln() - self.target_radius.ln()).abs() < REST_THRESHOLD
    }

    // ------------------------------------------------------------- the camera

    /// Writes `current` onto `camera`: the position is the ground point lifted
    /// along the normal, `up` *is* the normal, and the camera looks one unit
    /// along the heading.
    ///
    /// The no-roll invariant falls straight out of this: `Matrix4.lookAt()`
    /// builds the camera's right vector as `up × z`, so with `up` the ground
    /// normal the right vector is perpendicular to the normal by construction.
    pub fn apply(&self, camera: &mut PerspectiveCamera) {
        place(&self.ground, &self.current, camera);
    }

    // ---------------------------------------------------------- the overview

    /// GNOME's Super key: from [`Mode::Free`], saves the target pose and flies
    /// to a fitted, near-vertical view of `panes`; from [`Mode::Overview`],
    /// flies back to the saved pose.
    ///
    /// `camera_fov` is the vertical field of view in degrees and `aspect` the
    /// viewport's width over its height — the fit is computed against them and
    /// against the ground as it is now, so it is right at any curvature.
    pub fn toggle_overview(&mut self, panes: &[Pane], camera_fov: f64, aspect: f64) {
        match self.mode {
            Mode::Free => {
                self.return_pose = Some(self.target);

                let (u, v) = centroid(panes).unwrap_or((self.target.u, self.target.v));
                let yaw = self.target.yaw;
                let altitude = self.fit_altitude(panes, u, v, yaw, camera_fov, aspect);

                self.target = Pose {
                    u,
                    v,
                    altitude,
                    yaw,
                    pitch: OVERVIEW_PITCH,
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

    /// The smallest altitude in `[ 1, 5000 ]` at which every *visible* pane
    /// projects inside NDC `[ -0.9, 0.9 ]`, found by 40 bisections on
    /// `log( altitude )`. `5000` if nothing fits.
    ///
    /// A pane is visible when the camera is above its local horizon,
    /// `dot( ground normal at the pane, camera position - pane centre ) > 0`,
    /// so that on a tight sphere the panes round the back do not make the
    /// search run away to the ceiling. Nothing is rendered.
    pub fn fit_altitude(
        &self,
        panes: &[Pane],
        u: f64,
        v: f64,
        yaw: f64,
        camera_fov: f64,
        aspect: f64,
    ) -> f64 {
        if panes.is_empty() {
            return self.target.altitude;
        }

        let mut camera = PerspectiveCamera::new(camera_fov, aspect, 0.1, 1e9);

        let mut fits = |altitude: f64| -> bool {
            let pose = Pose {
                u,
                v,
                altitude,
                yaw,
                pitch: OVERVIEW_PITCH,
            };
            place(&self.ground, &pose, &mut camera);
            camera.update_matrix_world();
            let eye = camera.node.borrow().position;

            panes.iter().all(|pane| {
                let frame = self.ground.frame(pane.u, pane.v);
                let centre = lifted(&frame, pane.height * 0.5);
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

        if !fits(MAX_ALTITUDE) {
            return MAX_ALTITUDE;
        }

        let (mut low, mut high) = (MIN_ALTITUDE.ln(), MAX_ALTITUDE.ln());
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

    /// Drops onto one pane: 24 units in front of it, 9 up, looking at its
    /// centre. The pose becomes the one a later [`Helicopter::toggle_overview`]
    /// would return to, and the controller flies to it exactly as leaving the
    /// overview does.
    pub fn focus_pane(&mut self, pane: &Pane) {
        let pose = Pose {
            u: pane.u,
            v: pane.v - 24.0,
            altitude: 9.0,
            yaw: 0.0,
            pitch: -(9.0 - pane.height * 0.5).atan2(24.0),
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
        lifted(&self.ground.frame(pane.u, pane.v), pane.height * 0.5)
    }
}

// ------------------------------------------------------------------ helpers

/// `Helicopter::apply` for an arbitrary pose, so the overview's fit can try one
/// on a scratch camera without disturbing the controller.
fn place(ground: &Ground, pose: &Pose, camera: &mut PerspectiveCamera) {
    let frame = ground.frame(pose.u, pose.v);

    let mut position = frame.origin;
    position.add_scaled_vector(&frame.normal, pose.altitude);

    let (sin_yaw, cos_yaw) = (pose.yaw.sin(), pose.yaw.cos());
    let (sin_pitch, cos_pitch) = (pose.pitch.sin(), pose.pitch.cos());

    let mut forward = Vector3::ZERO;
    forward.add_scaled_vector(&frame.north, cos_pitch * cos_yaw);
    forward.add_scaled_vector(&frame.east, cos_pitch * sin_yaw);
    forward.add_scaled_vector(&frame.normal, sin_pitch);

    let mut look_at = position;
    look_at.add(&forward);

    {
        let mut object = camera.node.borrow_mut();
        object.up = frame.normal;
        object.position = position;
    }
    camera.look_at(&look_at);
    camera.update_matrix_world();
}

/// The ground point at a frame, lifted `distance` along its normal.
fn lifted(frame: &Frame, distance: f64) -> Vector3 {
    let mut point = frame.origin;
    point.add_scaled_vector(&frame.normal, distance);
    point
}

/// A pane's four corners, anticlockwise as seen from its front.
fn corners(frame: &Frame, pane: &Pane) -> [Vector3; 4] {
    let centre = lifted(frame, pane.height * 0.5);
    let (half_width, half_height) = (pane.width * 0.5, pane.height * 0.5);

    let corner = |along: f64, up: f64| {
        let mut point = centre;
        point.add_scaled_vector(&frame.east, along * half_width);
        point.add_scaled_vector(&frame.normal, up * half_height);
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

/// A field damped in log space — altitude and the ground radius, where a
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
    use crate::math::RAD2DEG;

    const FOV: f64 = 60.0;
    const ASPECT: f64 = 1.6;

    pub fn start() -> Pose {
        Pose {
            u: 0.0,
            v: -120.0,
            altitude: 40.0,
            yaw: 0.0,
            pitch: -15.0 * DEG2RAD,
        }
    }

    /// The demo's 7×7 layout.
    pub fn demo_panes() -> Vec<Pane> {
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
                altitude: 1.0 + next() * 400.0,
                yaw: next() * TAU - PI,
                pitch: MIN_PITCH + next() * (MAX_PITCH - MIN_PITCH),
            })
            .collect()
    }

    /// The whole point: the camera's right vector never leaves the ground's
    /// tangent plane, so the horizon never tilts.
    #[test]
    fn the_camera_never_rolls() {
        for radius in [40.0, 300.0, 1e7] {
            let ground = Ground::new(radius);
            for pose in poses() {
                let mut heli = Helicopter::new(ground, pose);
                heli.settle();

                let mut camera = PerspectiveCamera::new(FOV, ASPECT, 0.1, 1e6);
                heli.apply(&mut camera);

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

    #[test]
    fn yaw_damping_takes_the_short_way_round() {
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.target.yaw = 179.0 * DEG2RAD;
        heli.settle();
        heli.target.yaw = -179.0 * DEG2RAD;

        let mut crossed_the_back = false;
        for _ in 0..600 {
            heli.update(1.0 / 60.0);
            let yaw = heli.current().yaw * RAD2DEG;
            assert!(
                yaw.abs() > 90.0,
                "the yaw went the long way round, through {yaw}°"
            );
            if yaw.abs() > 179.5 {
                crossed_the_back = true;
            }
            if heli.rested() {
                break;
            }
        }

        assert!(crossed_the_back, "the yaw never passed ±180°");
        // `rested()` is `restThreshold` = 0.01 rad away from the target, and
        // that last 0.01 rad may sit either side of ±180°, so the comparison
        // has to wrap too.
        let left = wrap_pi(heli.current().yaw - -179.0 * DEG2RAD).abs();
        assert!(left < 0.011, "{}° short of the target", left * RAD2DEG);
    }

    #[test]
    fn pitch_is_clamped_after_a_look() {
        let mut heli = Helicopter::new(Ground::new(1e7), start());

        for _ in 0..200 {
            heli.look(0.0, 100.0, 1000.0);
        }
        assert!((heli.target().pitch - MIN_PITCH).abs() < 1e-12);

        for _ in 0..200 {
            heli.look(0.0, -100.0, 1000.0);
        }
        assert!((heli.target().pitch - MAX_PITCH).abs() < 1e-12);
    }

    /// Dragging right turns right: the heading swings toward where the camera's
    /// own right vector was pointing.
    #[test]
    fn dragging_right_turns_right() {
        let ground = Ground::new(1e7);
        let mut heli = Helicopter::new(ground, start());

        let mut camera = PerspectiveCamera::new(FOV, ASPECT, 0.1, 1e6);
        heli.apply(&mut camera);
        let elements = camera.node.borrow().matrix_world.elements;
        let right = Vector3::new(elements[0], elements[1], elements[2]);

        heli.look(100.0, 0.0, 1000.0);
        heli.settle();
        heli.apply(&mut camera);
        let after = {
            let elements = camera.node.borrow().matrix_world.elements;
            // The camera looks down its own `-Z`.
            Vector3::new(-elements[8], -elements[9], -elements[10])
        };

        assert!(
            after.dot(&right) > 0.0,
            "a rightward drag turned the heading away from the camera's right"
        );
    }

    #[test]
    fn the_overview_settles_and_then_stops() {
        let panes = demo_panes();
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.toggle_overview(&panes, FOV, ASPECT);

        assert!(
            !heli.rested(),
            "the overview pose is not where we already are"
        );

        let mut rested_after = None;
        for frame in 1..=180 {
            heli.update(1.0 / 60.0);
            if heli.rested() {
                rested_after = Some(frame);
                break;
            }
        }
        assert!(
            rested_after.is_some(),
            "the overview never rested within 3 seconds"
        );

        // And once every field has snapped, `update` reports nothing moved.
        let mut stopped = false;
        for _ in 0..300 {
            if !heli.update(1.0 / 60.0) {
                stopped = true;
                break;
            }
        }
        assert!(stopped, "`update` kept reporting movement after the rest");
        assert_eq!(heli.current(), heli.target());
    }

    #[test]
    fn toggling_twice_returns_the_saved_pose_exactly() {
        let panes = demo_panes();
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.look(37.0, -11.0, 900.0);
        heli.move_ground(1.0, 0.5, 0.3);
        let saved = heli.target();

        heli.toggle_overview(&panes, FOV, ASPECT);
        assert_eq!(heli.mode(), Mode::Overview);
        assert_ne!(heli.target(), saved);

        heli.toggle_overview(&panes, FOV, ASPECT);
        assert_eq!(heli.mode(), Mode::Free);
        assert_eq!(heli.target(), saved);
    }

    /// On the flat ground the fit has a closed form: the camera looks straight
    /// down from `altitude` at a rectangle `extent_u` by `extent_v`, so it must
    /// be far enough back that the half-extent divides by the tangent of the
    /// half field of view — then divided by the `0.9` NDC margin.
    ///
    /// The extents are the pane centres' span plus a whole pane: `6 * 50 + 16`
    /// across, `6 * 50 + 9` along — the second because a pane standing `9` tall
    /// is `9` closer to the camera at its top, which costs exactly as much
    /// frame as `9` more ground would.
    #[test]
    fn the_flat_fit_matches_the_closed_form() {
        let panes = demo_panes();
        let heli = Helicopter::new(Ground::new(1e7), start());
        let altitude = heli.fit_altitude(&panes, 0.0, 0.0, 0.0, FOV, ASPECT);

        let half_v = (FOV * DEG2RAD) * 0.5;
        let half_h = (half_v.tan() * ASPECT).atan();
        let extent_u = 6.0 * 50.0 + 16.0;
        let extent_v = 6.0 * 50.0 + 9.0;
        let closed_form =
            (extent_u / (2.0 * half_h.tan())).max(extent_v / (2.0 * half_v.tan())) / FIT_MARGIN;

        let error = (altitude - closed_form).abs() / closed_form;
        assert!(
            error < 0.05,
            "fit {altitude} is {:.1}% off the closed form {closed_form}",
            error * 100.0
        );
    }

    /// **Deviation from the brief, on purpose.** It asks for the `R = 300` fit
    /// to come out *above* the flat one. It comes out below, and the geometry
    /// says it must: the ground is a ball, so a pane `s` units of arc from the
    /// centroid sits only `R sin( s / R ) < s` away horizontally *and*
    /// `R ( 1 - cos( s / R ) )` further down, both of which shrink the angle it
    /// subtends at a camera over the pole. Curling the ground up pulls the
    /// layout in; it does not push it out. Measured across the range:
    ///
    /// ```text
    /// R = 1e7   -> 310.4     R = 600  -> 292.8     R = 150 -> 204.8
    /// R = 3000  -> 307.4     R = 300  -> 268.9     R =  80 ->  96.8
    /// R = 1000  -> 300.7                           R =  40 ->  18.6
    /// ```
    ///
    /// Below `R ≈ 150` the horizon test starts excluding panes, which is what
    /// it is there for, and the fit drops away faster still.
    #[test]
    fn a_curved_ground_needs_less_altitude_than_a_flat_one() {
        let panes = demo_panes();
        let mut previous = f64::INFINITY;

        for radius in [1e7, 3000.0, 1000.0, 600.0, 300.0] {
            let altitude = Helicopter::new(Ground::new(radius), start())
                .fit_altitude(&panes, 0.0, 0.0, 0.0, FOV, ASPECT);
            assert!(
                altitude < previous,
                "R = {radius} fit {altitude} is not below the previous {previous}"
            );
            previous = altitude;
        }
    }

    #[test]
    fn picking_hits_the_pane_under_the_centre_of_the_screen() {
        let panes = demo_panes();
        let ground = Ground::new(1e7);

        // Dead in front of the middle pane, which is index 24 of the 7×7.
        let middle = panes[24];
        let mut heli = Helicopter::new(ground, start());
        heli.focus_pane(&middle);
        heli.settle();

        let mut camera = PerspectiveCamera::new(FOV, ASPECT, 0.1, 1e6);
        heli.apply(&mut camera);

        assert_eq!(heli.pick(&panes, 0.0, 0.0, &camera), Some(24));
        // And well off to the side there is nothing but ground.
        assert_eq!(heli.pick(&panes, -0.99, -0.99, &camera), None);
    }

    #[test]
    fn focus_pane_puts_the_pane_centre_on_the_optical_axis() {
        let panes = demo_panes();
        let pane = panes[10];
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.focus_pane(&pane);
        heli.settle();

        let mut camera = PerspectiveCamera::new(FOV, ASPECT, 0.1, 1e6);
        heli.apply(&mut camera);

        let centre = heli.pane_centre(&pane);
        let (x, y, _) = project(&mut camera, centre).expect("the pane is in front");
        // Not exact: `v - 24` is 24 units of *arc*, and the focus pitch is the
        // angle a flat 24 units would want, so even at `R = 1e7` the centre
        // lands a couple of millionths of an NDC unit off the axis.
        assert!(x.abs() < 1e-4 && y.abs() < 1e-4, "off axis at ( {x}, {y} )");
    }

    #[test]
    fn altitude_and_radius_are_clamped() {
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.climb(1000.0);
        assert_eq!(heli.target().altitude, MAX_ALTITUDE);
        heli.climb(-1000.0);
        assert_eq!(heli.target().altitude, MIN_ALTITUDE);

        heli.set_radius(1.0);
        assert_eq!(heli.target_radius(), 40.0);
        heli.set_radius(1e20);
        assert_eq!(heli.target_radius(), 1e7);
    }

    /// Movement is proportional to altitude, and `D` goes the way the pilot's
    /// right hand points.
    #[test]
    fn moving_scales_with_altitude_and_strafes_right() {
        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.move_ground(1.0, 0.0, 1.0);
        assert!((heli.target().v - (-120.0 + 1.2 * 40.0)).abs() < 1e-12);
        assert!(heli.target().u.abs() < 1e-12);

        let mut heli = Helicopter::new(Ground::new(1e7), start());
        heli.move_ground(0.0, 1.0, 1.0);
        // Facing north (`+Z`), screen-right is `-X`, i.e. `-u`.
        assert!((heli.target().u - (-1.2 * 40.0)).abs() < 1e-12);
    }
}
