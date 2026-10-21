//! The surface a [`MapControls`](crate::MapControls) moves over: a plane, or a
//! sphere either side of it, with a distinguished point at the world origin.
//!
//! The knob is the signed curvature `k`, not the radius. `0` is the plane,
//! exactly; `k > 0` is a planet under the camera (the sphere's centre below the
//! origin, the camera outside it); `k < 0` is the same sphere mirrored in `y`,
//! closing around the camera (the centre above the origin, the camera inside,
//! looking at the grid on the inner surface). The radius `1 / |k|` appears in
//! no formula and no signature: the plane has none, and every term here is
//! written so that it tends to the plane's as `k` tends to `0` from either side.
//!
//! 0.2 carried a radius in `[ 40, 1e7 ]` and stood the plane in with `1e7`,
//! which kept every term finite but could not go past the plane: the radius is
//! `+∞` there and the bowl is on the other side of it. The curvature passes
//! through `0`, so a damped change slides from planet to plane to bowl without
//! a seam.
//!
//! Nothing here is a port of three.js — three.js has no ground — but the frame
//! it hands out is exactly what `Object3D.up` and `Camera.lookAt()` want, so a
//! camera driven from it never rolls.

use three_rs::math::Vector3;

/// The smallest ground radius, either side of the plane. Below this the
/// curvature is so tight that a camera at the altitude ceiling is further from
/// the surface than the surface is wide, and the exponential map stops being a
/// useful coordinate system.
pub const MIN_RADIUS: f64 = 40.0;

/// The tightest curvature either sign: [`Ground::new`] clamps to
/// `[ -MAX_CURVATURE, MAX_CURVATURE ]`.
pub const MAX_CURVATURE: f64 = 1.0 / MIN_RADIUS;

/// The step the east vector is differenced over, in ground units.
const EPSILON: f64 = 1e-3;

/// Below this the series stand in for `sin( x ) / x` and `atan( x ) / x`: the
/// first dropped term is `x⁴ / 120`, under `1e-18`, and above it the quotient
/// is exact to the last bit. The branch is on the angle `k · d`, not on `k`, so
/// it is as good for a tight sphere near the origin as for a flat one far out.
const SERIES: f64 = 1e-4;

/// An orthonormal frame on the ground: where a point is and which way is
/// along-the-surface east, along-the-surface north, and up.
///
/// `east`, `north` and `normal` are unit vectors, mutually perpendicular and
/// right-handed: `east × north = normal`, as `x × y = z` on a map with east
/// along `x` and north along `y`. At the ground origin they are `+X`, `-Z` and
/// `+Y`, whatever the curvature — three.js' own convention, where a camera at
/// `+Z` looking down `-Z` with `up = +Y` has `+X` on its right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The point on the surface, in world space.
    pub origin: Vector3,
    /// Increasing `u`, tangent to the surface.
    pub east: Vector3,
    /// Increasing `v`, tangent to the surface.
    pub north: Vector3,
    /// The camera's side of the surface: away from the centre on a planet,
    /// toward it in a bowl, `+Y` on the plane.
    pub normal: Vector3,
}

/// The surface of signed curvature `k`, touching the world origin with
/// normal `+Y` for every `k` — content authored on a plane keeps its place when
/// the ground is curled up, or down.
///
/// For `k ≠ 0` it is the sphere of radius `1 / |k|` centred at
/// `( 0, -1 / k, 0 )`: below the origin for a planet, above it for a bowl. For
/// `k = 0` it is the plane `y = 0`.
///
/// Ground coordinates `( u, v )` are arc lengths from that origin: the
/// exponential map of the surface at the origin. `u` runs east, `v` runs north,
/// and as `k` tends to `0` from either side `point( u, v )` tends to
/// `( u, 0, -v )`, which is what it is at `0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    curvature: f64,
}

impl Default for Ground {
    /// The plane.
    fn default() -> Self {
        Self::flat()
    }
}

impl Ground {
    /// The plane: curvature `0`.
    pub fn flat() -> Self {
        Self { curvature: 0.0 }
    }

    /// A ground of this curvature, clamped to
    /// `[ -MAX_CURVATURE, MAX_CURVATURE ]`. NaN is the plane.
    pub fn new(curvature: f64) -> Self {
        let mut ground = Self::flat();
        ground.set_curvature(curvature);
        ground
    }

    /// The signed curvature, in `[ -MAX_CURVATURE, MAX_CURVATURE ]`: positive
    /// for a planet, negative for a bowl, `0` for the plane.
    pub fn curvature(&self) -> f64 {
        self.curvature
    }

    /// Sets the curvature, clamped to `[ -MAX_CURVATURE, MAX_CURVATURE ]`.
    /// `-0.0` is stored as `0.0`, so the plane compares equal to the plane.
    pub fn set_curvature(&mut self, k: f64) {
        self.curvature = if k.is_nan() {
            0.0
        } else {
            k.clamp(-MAX_CURVATURE, MAX_CURVATURE) + 0.0
        };
    }

    /// The world-space point at ground coordinate `( u, v )`:
    ///
    /// ```text
    /// d   = sqrt( u² + v² )
    /// a   = k d                                  the angle subtended at the centre
    /// p   = ( d sinc( a ) u / d,
    ///        -d ( a / 2 ) sinc²( a / 2 ),
    ///        -d sinc( a ) v / d )
    /// ```
    ///
    /// which is `sin( k d ) / k` out and `-2 sin²( k d / 2 ) / k` down, the
    /// sphere's exponential map, written so that `k` is never a divisor: at
    /// `k = 0` it is `( u, 0, -v )` term by term. The horizontal part is even in
    /// `k` and the vertical part odd, so `k` and `-k` are mirror images in `y`.
    ///
    /// The vertical term is the `-2 R sin²( d / 2R )` form 0.2 used against
    /// `R cos( d / R ) - R`, for the same reason: no subtraction of two nearly
    /// equal numbers, so a nearly flat ground is flat to the last bit.
    pub fn point(&self, u: f64, v: f64) -> Vector3 {
        let d = u.hypot(v);
        if d == 0.0 {
            return Vector3::ZERO;
        }

        let a = self.curvature * d;
        let horizontal = d * sinc(a);
        let half = sinc(0.5 * a);

        Vector3::new(
            horizontal * (u / d),
            -d * (0.5 * a) * half * half,
            -horizontal * (v / d),
        )
    }

    /// The unit normal at `( u, v )` on the camera's side of the surface:
    /// `( sin( a ) u / d, cos( a ), -sin( a ) v / d )` with `a = k d`.
    ///
    /// This is not the sphere's geometric outward normal. It is `+Y` at the
    /// origin for every `k`, so in a bowl it points toward the centre — the
    /// side the camera is on, and the `up` a camera there wants.
    pub fn normal(&self, u: f64, v: f64) -> Vector3 {
        let d = u.hypot(v);
        if d == 0.0 {
            return Vector3::new(0.0, 1.0, 0.0);
        }

        let a = self.curvature * d;
        let (sin, cos) = a.sin_cos();

        Vector3::new(sin * (u / d), cos, -sin * (v / d))
    }

    /// The orthonormal [`Frame`] at ground coordinate `( u, v )`.
    ///
    /// `east` is the central difference of [`Ground::point`] in `u`, made
    /// perpendicular to the exact normal by Gram-Schmidt; `north` is
    /// `normal × east`, which completes the right-handed triple
    /// (`+Y × +X = -Z` at the origin).
    pub fn frame(&self, u: f64, v: f64) -> Frame {
        let origin = self.point(u, v);
        let normal = self.normal(u, v);

        let ahead = self.point(u + EPSILON, v);
        let behind = self.point(u - EPSILON, v);
        let mut east = Vector3::new(ahead.x - behind.x, ahead.y - behind.y, ahead.z - behind.z);
        east.normalize();
        // Gram-Schmidt against the exact normal: the difference is a chord, not
        // a tangent, so it leans into the surface by O( ε² k ).
        let along = east.dot(&normal);
        east.add_scaled_vector(&normal, -along);
        east.normalize();

        let north = normal.crossed(&east);

        Frame {
            origin,
            east,
            north,
            normal,
        }
    }

    /// The ground coordinates where the ray from `origin` along `direction`
    /// meets the surface, or `None` if it does not.
    ///
    /// `k > 0`: the near hit, for a camera outside the planet. `k < 0`: the far
    /// hit, for a camera inside the bowl — from inside, the only hit ahead is
    /// the wall the ray leaves through. `k = 0`: the ray-plane test.
    ///
    /// All three are one quadratic. Substituting `p = o + t d` into
    /// `|p - c|² = 1 / k²` with `c = ( 0, -1 / k, 0 )` and multiplying through
    /// by `k` gives
    ///
    /// ```text
    /// k |d|² t²  +  2 ( k d·o + d.y ) t  +  ( k |o|² + 2 o.y )  =  0
    /// ```
    ///
    /// which at `k = 0` *is* the plane, `d.y t + o.y = 0`. The roots are taken
    /// as `q / A` and `C / q` with `q = -( B + sign( B ) √disc )`, so that
    /// as `k` tends to `0` the root that becomes the plane's is `C / q`, which
    /// never divides by `k`, and the other runs off to the far side of the
    /// sphere. 0.2's factored `( |m| - R )( |m| + R )`, which kept a camera
    /// 160 units over an `R = 1e7` ball from rounding to the surface, is not
    /// needed: no term here is the size of the radius.
    pub fn intersect(&self, origin: Vector3, direction: Vector3) -> Option<(f64, f64)> {
        let k = self.curvature;
        let a = k * direction.dot(&direction);
        let b = k * direction.dot(&origin) + direction.y;
        let c = k * origin.dot(&origin) + 2.0 * origin.y;

        let t = if a == 0.0 {
            // The plane: one root, or none when the ray runs parallel to it.
            if b == 0.0 {
                return None;
            }
            -0.5 * c / b
        } else {
            let discriminant = b * b - a * c;
            if discriminant < 0.0 {
                return None;
            }
            let q = -(b + discriminant.sqrt().copysign(b));
            let mut roots = [q / a, if q == 0.0 { q / a } else { c / q }];
            roots.sort_by(f64::total_cmp);
            let ahead = roots.into_iter().filter(|t| *t > 0.0 && t.is_finite());
            if k > 0.0 {
                ahead.reduce(f64::min)?
            } else {
                ahead.reduce(f64::max)?
            }
        };
        if t <= 0.0 || !t.is_finite() {
            return None;
        }

        let mut hit = origin;
        hit.add_scaled_vector(&direction, t);
        Some(self.coordinates(hit))
    }

    /// The inverse of [`Ground::point`]: the exponential map read backwards,
    /// for a point on the surface (or off it along the normal, which changes
    /// nothing).
    ///
    /// ```text
    /// h       = |p.xz|
    /// a       = atan2( k h, 1 + k p.y )           the angle at the centre
    /// d       = a / k
    /// ( u, v ) = d ( p.x, -p.z ) / h
    /// ```
    ///
    /// `a / k` divides by `k`, so while the angle is under 60° it is written
    /// `h / ( 1 + k p.y ) · atan( z ) / z` with `z = k h / ( 1 + k p.y )`,
    /// which is the same number and is `h` at `k = 0`. Past 60° `|k| d` is at
    /// least one, so `k` is no longer small and the quotient is safe.
    fn coordinates(&self, p: Vector3) -> (f64, f64) {
        let k = self.curvature;
        let h = p.x.hypot(p.z);
        if h == 0.0 {
            return (0.0, 0.0);
        }

        let (sin, cos) = (k * h, 1.0 + k * p.y);
        let d = if cos > 0.5 {
            let z = sin / cos;
            h / cos * atanc(z)
        } else {
            sin.atan2(cos) / k
        };

        (d * p.x / h, -d * p.z / h)
    }
}

/// `sin( x ) / x`, and `1` at `0`.
fn sinc(x: f64) -> f64 {
    if x.abs() < SERIES {
        1.0 - x * x / 6.0
    } else {
        x.sin() / x
    }
}

/// `atan( x ) / x`, and `1` at `0`.
fn atanc(x: f64) -> f64 {
    if x.abs() < SERIES {
        1.0 - x * x / 3.0
    } else {
        x.atan() / x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// Both signs, the plane, and a curvature too small to see.
    const CURVATURES: [f64; 7] = [-MAX_CURVATURE, -1e-3, -1e-9, 0.0, 1e-9, 1e-3, MAX_CURVATURE];

    /// A sweep over `[ -MAX_CURVATURE, MAX_CURVATURE ]` in 41 even steps, which
    /// lands on `0` exactly.
    fn sweep() -> impl Iterator<Item = f64> {
        (-20..=20).map(|i| MAX_CURVATURE * i as f64 / 20.0)
    }

    fn close(a: f64, b: f64, tolerance: f64, what: &str) {
        assert!((a - b).abs() <= tolerance, "{what}: {a} != {b}");
    }

    fn close_vector(a: Vector3, b: Vector3, tolerance: f64, what: &str) {
        close(a.x, b.x, tolerance, &format!("{what}.x"));
        close(a.y, b.y, tolerance, &format!("{what}.y"));
        close(a.z, b.z, tolerance, &format!("{what}.z"));
    }

    /// A small deterministic sequence, so "ten random points" is the same ten
    /// points on every run.
    fn samples() -> Vec<(f64, f64)> {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..10)
            .map(|_| (next() * 800.0 - 400.0, next() * 800.0 - 400.0))
            .collect()
    }

    #[test]
    fn the_origin_is_the_origin_at_every_curvature() {
        for k in sweep() {
            let p = Ground::new(k).point(0.0, 0.0);
            assert_eq!(p, Vector3::ZERO, "k = {k}");
        }
    }

    #[test]
    fn the_origin_frame_is_the_world_axes_at_every_curvature() {
        for k in sweep().chain(CURVATURES) {
            let frame = Ground::new(k).frame(0.0, 0.0);
            close_vector(frame.origin, Vector3::ZERO, 1e-12, "origin");
            close_vector(frame.east, Vector3::new(1.0, 0.0, 0.0), 1e-9, "east");
            close_vector(frame.north, Vector3::new(0.0, 0.0, -1.0), 1e-9, "north");
            close_vector(frame.normal, Vector3::new(0.0, 1.0, 0.0), 1e-12, "normal");
        }
    }

    #[test]
    fn k_and_minus_k_are_mirror_images_in_y() {
        for k in sweep().filter(|k| *k > 0.0) {
            let (planet, bowl) = (Ground::new(k), Ground::new(-k));
            for (u, v) in samples() {
                let (p, q) = (planet.point(u, v), bowl.point(u, v));
                close(p.x, q.x, 1e-12, "x");
                close(p.y, -q.y, 1e-12, "y");
                close(p.z, q.z, 1e-12, "z");
            }
        }
    }

    #[test]
    fn every_point_is_on_its_sphere() {
        for k in CURVATURES.into_iter().filter(|k| k.abs() >= 1e-3) {
            let ground = Ground::new(k);
            let (radius, centre) = (1.0 / k.abs(), Vector3::new(0.0, -1.0 / k, 0.0));
            for (u, v) in samples() {
                let mut offset = Vector3::ZERO;
                offset.sub_vectors(&ground.point(u, v), &centre);
                let error = (offset.length() - radius).abs() / radius;
                assert!(error <= 1e-12, "k = {k}, ( {u}, {v} ): {error} off");
            }
        }
    }

    #[test]
    fn the_plane_is_the_plane() {
        let ground = Ground::flat();
        for (u, v) in samples() {
            // To rounding: the point is `d` along `( u, -v ) / d`.
            close_vector(ground.point(u, v), Vector3::new(u, 0.0, -v), 1e-9, "point");
            assert_eq!(ground.normal(u, v), Vector3::new(0.0, 1.0, 0.0));
        }
    }

    /// The brief asked for `1e-6` of the plane at `k = ±1e-9`. The drop is the
    /// sphere's own, `k d² / 2` to leading order, and at the corner of a
    /// 400-unit grid that is `1.6e-4`: no correct formula is within `1e-6`
    /// there. So the horizontal terms are held to `1e-6`, the drop to the
    /// leading-order sag, and the whole point to `1e-6` at `k = ±1e-12`, where
    /// the sag is that small.
    #[test]
    fn a_tiny_curvature_is_the_plane_to_within_its_sag() {
        let mut u = -400.0;
        while u <= 400.0 {
            let mut v = -400.0;
            while v <= 400.0 {
                let flat = Vector3::new(u, 0.0, -v);
                let what = format!("( {u}, {v} )");
                for k in [1e-9, -1e-9] {
                    let p = Ground::new(k).point(u, v);
                    close(p.x, flat.x, 1e-6, &what);
                    close(p.z, flat.z, 1e-6, &what);
                    let sag = -0.5 * k * (u * u + v * v);
                    close(p.y, sag, 1e-12, &what);
                }
                for k in [1e-12, -1e-12] {
                    close_vector(Ground::new(k).point(u, v), flat, 1e-6, &what);
                }
                v += 25.0;
            }
            u += 25.0;
        }
    }

    #[test]
    fn the_frame_is_orthonormal() {
        for k in CURVATURES {
            let ground = Ground::new(k);
            for (u, v) in samples() {
                let f = ground.frame(u, v);
                for (name, axis) in [("east", f.east), ("north", f.north), ("normal", f.normal)] {
                    close(axis.length(), 1.0, 1e-9, &format!("|{name}| at k = {k}"));
                }
                close(f.east.dot(&f.north), 0.0, 1e-9, "east · north");
                close(f.east.dot(&f.normal), 0.0, 1e-9, "east · normal");
                close(f.north.dot(&f.normal), 0.0, 1e-9, "north · normal");
                // And right-handed: `east × north = normal`.
                close_vector(f.east.crossed(&f.north), f.normal, 1e-9, "east × north");
            }
        }
    }

    /// In a bowl the normal is the camera's side: it points at the centre.
    #[test]
    fn the_normal_faces_the_camera_side() {
        for k in CURVATURES.into_iter().filter(|k| k.abs() >= 1e-3) {
            let ground = Ground::new(k);
            let centre = Vector3::new(0.0, -1.0 / k, 0.0);
            for (u, v) in samples() {
                let mut to_centre = Vector3::ZERO;
                to_centre.sub_vectors(&centre, &ground.point(u, v));
                let facing = to_centre.dot(&ground.normal(u, v));
                assert!(
                    (k < 0.0) == (facing > 0.0),
                    "k = {k}, ( {u}, {v} ): normal · to-centre = {facing}"
                );
            }
        }
    }

    #[test]
    fn the_curvature_is_clamped() {
        assert_eq!(Ground::new(1.0).curvature(), MAX_CURVATURE);
        assert_eq!(Ground::new(-1.0).curvature(), -MAX_CURVATURE);
        assert_eq!(Ground::new(f64::NAN).curvature(), 0.0);
        assert!(Ground::new(-0.0).curvature().is_sign_positive());
        assert_eq!(Ground::default(), Ground::flat());

        let mut ground = Ground::flat();
        ground.set_curvature(-1e3);
        assert_eq!(ground.curvature(), -MAX_CURVATURE);
        ground.set_curvature(1.0 / 300.0);
        assert_eq!(ground.curvature(), 1.0 / 300.0);
    }

    /// [`Ground::coordinates`] is the inverse of [`Ground::point`] over the
    /// whole useful range, out to nine tenths of the way to the far pole, on
    /// both sides of the plane.
    ///
    /// The tolerance is `1e-9` with a relative floor: on a near-plane ground
    /// the arc lengths run to `1e7`, where a `1e-9` *absolute* error would be
    /// a sixteenth of a double's precision, which no arithmetic can hold.
    #[test]
    fn the_inverse_exponential_map_round_trips() {
        for k in [
            -MAX_CURVATURE,
            -1.0 / 300.0,
            -1e-3,
            -1e-9,
            0.0,
            1e-9,
            1e-3,
            1.0 / 300.0,
            MAX_CURVATURE,
        ] {
            let ground = Ground::new(k);
            let limit = if k.abs() * 1e7 > 0.9 * PI / 2.0 {
                0.9 * PI / 2.0 / k.abs()
            } else {
                1e7
            };

            for i in 0..=12 {
                for j in 0..=12 {
                    let u = limit * (i as f64 / 6.0 - 1.0);
                    let v = limit * (j as f64 / 6.0 - 1.0);

                    let (back_u, back_v) = ground.coordinates(ground.point(u, v));
                    let error = (back_u - u).hypot(back_v - v);
                    let tolerance = 1e-9_f64.max(1e-12 * u.hypot(v));
                    assert!(
                        error <= tolerance,
                        "k = {k}, ( {u}, {v} ) came back as \
                         ( {back_u}, {back_v} ), off by {error}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_plane_intersect_is_a_plane_test() {
        let ground = Ground::flat();
        let above = Vector3::new(3.0, 10.0, -4.0);

        // Straight down lands under the origin of the ray.
        let (u, v) = ground
            .intersect(above, Vector3::new(0.0, -1.0, 0.0))
            .expect("down hits");
        close(u, 3.0, 1e-12, "u");
        close(v, 4.0, 1e-12, "v");

        // Down and north at 45° lands ten units further north.
        let mut slant = Vector3::new(0.0, -1.0, -1.0);
        slant.normalize();
        let (_, v) = ground.intersect(above, slant).expect("slant hits");
        close(v, 14.0, 1e-9, "v");

        // Parallel and upward miss; so does looking down from underneath.
        assert_eq!(ground.intersect(above, Vector3::new(1.0, 0.0, 0.0)), None);
        assert_eq!(ground.intersect(above, Vector3::new(0.0, 1.0, 0.0)), None);
        let below = Vector3::new(0.0, -10.0, 0.0);
        assert_eq!(ground.intersect(below, Vector3::new(0.0, -1.0, 0.0)), None);
    }

    /// From inside a bowl every ray hits, and the hit is the wall ahead: up
    /// from the floor is the far pole, `π / |k|` of arc away.
    #[test]
    fn the_bowl_intersect_is_the_far_wall() {
        let k = -1.0 / 300.0;
        let ground = Ground::new(k);
        let inside = Vector3::new(0.0, 100.0, 0.0);

        let (u, v) = ground
            .intersect(inside, Vector3::new(0.0, -1.0, 0.0))
            .expect("down hits the floor");
        close(u.hypot(v), 0.0, 1e-9, "floor");

        // Up from just off the axis, to keep the far pole's direction defined.
        let off_axis = Vector3::new(1e-3, 100.0, 0.0);
        let (u, v) = ground
            .intersect(off_axis, Vector3::new(0.0, 1.0, 0.0))
            .expect("up hits the far pole");
        close(u.hypot(v), PI * 300.0 - 1e-3, 1e-6, "far pole");

        // Sideways hits the wall, on the side the ray went.
        for (direction, east, north) in [
            (Vector3::new(1.0, 0.0, 0.0), 1.0, 0.0),
            (Vector3::new(0.0, 0.0, -1.0), 0.0, 1.0),
        ] {
            let (u, v) = ground.intersect(inside, direction).expect("sideways hits");
            let hit = ground.point(u, v);
            close(hit.y, 100.0, 1e-9, "level with the ray");
            assert!(u * east + v * north > 0.0, "the wall behind was hit");
        }
    }

    /// A planet is hit on its near side, and missed past the limb.
    #[test]
    fn the_planet_intersect_is_the_near_side() {
        let ground = Ground::new(1.0 / 300.0);
        let above = Vector3::new(0.0, 100.0, 0.0);
        let mut slant = Vector3::new(0.0, -1.0, -0.5);
        slant.normalize();
        let (u, v) = ground.intersect(above, slant).expect("slant hits");
        let hit = ground.point(u, v);
        let mut back = hit;
        back.sub(&above);
        close(
            back.normalized().dot(&slant),
            1.0,
            1e-9,
            "the hit is on the ray",
        );
        // The far side would be hundreds of units further; the near side is
        // within the flat answer, since the surface drops away.
        assert!(back.length() > 100.0 && back.length() < 300.0);

        assert_eq!(ground.intersect(above, Vector3::new(1.0, 0.0, 0.0)), None);
    }
}
