//! The surface a [`MapControls`](crate::MapControls) moves over: a
//! sphere with a distinguished point at the world origin.
//!
//! Nothing here is a port of three.js — three.js has no ground — but the frame
//! it hands out is exactly what `Object3D.up` and `Camera.lookAt()` want, so a
//! camera driven from it never rolls.

use three_rs::math::Vector3;

/// The smallest ground radius. Below this the curvature is so tight that a
/// camera at the altitude ceiling is further from the surface than the surface
/// is wide, and the exponential map stops being a useful coordinate system.
pub const MIN_RADIUS: f64 = 40.0;

/// The largest ground radius. At `1e7` a sphere is a plane to the eye — the
/// drop over the 800-unit demo grid is a centimetre and a half — while every
/// term below stays finite, which is why there is no separate plane case.
pub const MAX_RADIUS: f64 = 1e7;

/// The step the east vector is differenced over, in ground units.
const EPSILON: f64 = 1e-3;

/// An orthonormal frame on the ground: where a point is and which way is
/// along-the-surface east, along-the-surface north, and up.
///
/// `east`, `north` and `normal` are unit vectors and mutually perpendicular.
/// At the ground origin they are `+X`, `+Z` and `+Y`, whatever the radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The point on the surface, in world space.
    pub origin: Vector3,
    /// Increasing `u`, tangent to the surface.
    pub east: Vector3,
    /// Increasing `v`, tangent to the surface.
    pub north: Vector3,
    /// Away from the ground's centre.
    pub normal: Vector3,
}

/// A sphere of radius `radius` centred at `( 0, -radius, 0 )`, so that ground
/// coordinate `( 0, 0 )` is the world origin with normal `+Y` for every radius
/// — content authored on a plane keeps its place when the ground is curled up.
///
/// Ground coordinates `( u, v )` are arc lengths from that origin: the
/// exponential map of the sphere at the origin. `u` runs east, `v` runs north,
/// and as the radius grows `point( u, v )` tends to `( u, 0, v )`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    radius: f64,
}

impl Default for Ground {
    fn default() -> Self {
        Self::new(MAX_RADIUS)
    }
}

impl Ground {
    /// A ground of this radius, clamped to `[ MIN_RADIUS, MAX_RADIUS ]`.
    pub fn new(radius: f64) -> Self {
        Self {
            radius: radius.clamp(MIN_RADIUS, MAX_RADIUS),
        }
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Sets the radius, clamped to `[ MIN_RADIUS, MAX_RADIUS ]`.
    pub fn set_radius(&mut self, radius: f64) {
        self.radius = radius.clamp(MIN_RADIUS, MAX_RADIUS);
    }

    /// Multiplies the radius, clamped the same way.
    pub fn scale_radius(&mut self, factor: f64) {
        self.set_radius(self.radius * factor);
    }

    /// The sphere's centre, `( 0, -radius, 0 )`.
    pub fn centre(&self) -> Vector3 {
        Vector3::new(0.0, -self.radius, 0.0)
    }

    /// The world-space point at ground coordinate `( u, v )`:
    ///
    /// ```text
    /// d   = sqrt( u² + v² )
    /// dir = ( u, v ) / d
    /// p   = centre + R * ( sin( d / R ) * ( dir.u, 0, dir.v )
    ///                    + cos( d / R ) * ( 0, 1, 0 ) )
    /// ```
    ///
    /// The `y` term is written `-2 R sin²( d / 2R )` rather than
    /// `R cos( d / R ) - R`: the two are the same number, but the first is
    /// computed without subtracting `1e7` from `1e7`, which is what keeps the
    /// flat ground flat to the millimetre instead of to the metre.
    pub fn point(&self, u: f64, v: f64) -> Vector3 {
        let d = u.hypot(v);
        if d == 0.0 {
            return Vector3::ZERO;
        }

        let r = self.radius;
        let a = d / r;
        let horizontal = r * a.sin();
        let half = (a * 0.5).sin();

        Vector3::new(
            horizontal * (u / d),
            -2.0 * r * half * half,
            horizontal * (v / d),
        )
    }

    /// The outward unit normal at `( u, v )`, i.e.
    /// `normalize( point( u, v ) - centre )` — written out analytically, since
    /// that difference is the one place the `1e7` cancellation bites.
    pub fn normal(&self, u: f64, v: f64) -> Vector3 {
        let d = u.hypot(v);
        if d == 0.0 {
            return Vector3::new(0.0, 1.0, 0.0);
        }

        let a = d / self.radius;
        let (sin, cos) = (a.sin(), a.cos());

        Vector3::new(sin * (u / d), cos, sin * (v / d))
    }

    /// The orthonormal [`Frame`] at ground coordinate `( u, v )`.
    ///
    /// `east` is the central difference of [`Ground::point`] in `u`, made
    /// perpendicular to the exact normal by Gram-Schmidt; `north` is
    /// `east × normal`, which is the order that comes out `+Z` at the origin
    /// (`+X × +Y = +Z`).
    pub fn frame(&self, u: f64, v: f64) -> Frame {
        let origin = self.point(u, v);
        let normal = self.normal(u, v);

        let ahead = self.point(u + EPSILON, v);
        let behind = self.point(u - EPSILON, v);
        let mut east = Vector3::new(ahead.x - behind.x, ahead.y - behind.y, ahead.z - behind.z);
        east.normalize();
        // Gram-Schmidt against the exact normal: the difference is a chord, not
        // a tangent, so it leans into the surface by O( ε² / R ).
        let along = east.dot(&normal);
        east.add_scaled_vector(&normal, -along);
        east.normalize();

        let north = east.crossed(&normal);

        Frame {
            origin,
            east,
            north,
            normal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RADII: [f64; 3] = [40.0, 1000.0, MAX_RADIUS];

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
    fn the_origin_frame_is_the_world_axes_at_every_radius() {
        for radius in RADII {
            let frame = Ground::new(radius).frame(0.0, 0.0);
            close_vector(frame.origin, Vector3::ZERO, 1e-12, "origin");
            close_vector(frame.east, Vector3::new(1.0, 0.0, 0.0), 1e-9, "east");
            close_vector(frame.north, Vector3::new(0.0, 0.0, 1.0), 1e-9, "north");
            close_vector(frame.normal, Vector3::new(0.0, 1.0, 0.0), 1e-12, "normal");
        }
    }

    #[test]
    fn every_point_is_on_the_sphere() {
        for radius in RADII {
            let ground = Ground::new(radius);
            let centre = ground.centre();
            for (u, v) in samples() {
                let p = ground.point(u, v);
                let mut offset = Vector3::ZERO;
                offset.sub_vectors(&p, &centre);
                let error = (offset.length() - radius).abs() / radius;
                assert!(error <= 1e-6, "R = {radius}, ( {u}, {v} ): {error} off");
            }
        }
    }

    #[test]
    fn the_largest_radius_is_a_plane_to_a_tenth_of_a_unit() {
        let ground = Ground::new(MAX_RADIUS);
        let mut u = -500.0;
        while u <= 500.0 {
            let mut v = -500.0;
            while v <= 500.0 {
                let p = ground.point(u, v);
                close_vector(p, Vector3::new(u, 0.0, v), 0.1, &format!("( {u}, {v} )"));
                v += 25.0;
            }
            u += 25.0;
        }
    }

    #[test]
    fn the_frame_is_orthonormal() {
        for radius in RADII {
            let ground = Ground::new(radius);
            for (u, v) in samples() {
                let f = ground.frame(u, v);
                for (name, axis) in [("east", f.east), ("north", f.north), ("normal", f.normal)] {
                    close(
                        axis.length(),
                        1.0,
                        1e-9,
                        &format!("|{name}| at R = {radius}"),
                    );
                }
                close(f.east.dot(&f.north), 0.0, 1e-9, "east · north");
                close(f.east.dot(&f.normal), 0.0, 1e-9, "east · normal");
                close(f.north.dot(&f.normal), 0.0, 1e-9, "north · normal");
                // And right-handed in the order that made `north` come out `+Z`.
                close_vector(f.east.crossed(&f.normal), f.north, 1e-9, "east × normal");
            }
        }
    }

    #[test]
    fn the_radius_is_clamped() {
        assert_eq!(Ground::new(1.0).radius(), MIN_RADIUS);
        assert_eq!(Ground::new(1e12).radius(), MAX_RADIUS);

        let mut ground = Ground::new(100.0);
        ground.scale_radius(0.001);
        assert_eq!(ground.radius(), MIN_RADIUS);
        ground.scale_radius(1e9);
        assert_eq!(ground.radius(), MAX_RADIUS);
    }
}
