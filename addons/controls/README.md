# `three-rs-controls`

A map camera over a ground with curvature, for [three-rs](../../README.md).

This is an addon in the sense of three.js' `examples/jsm/controls/`: it depends
on `three-rs` the way any user would, and it is not a port of anything in
three.js' `src/`. `MapControls` is in the spirit of three.js' addon of that
name — left-drag pans, right-drag orbits, the wheel zooms to the pointer — but
it is this crate's own design, and its `smooth_damp` is
[camera-controls](https://github.com/yomotsu/camera-controls)' rather than
three.js'. Its constants are not camera-controls' defaults, which read as hefty;
`map_controls.rs` records what was tried and why.

- **`Ground`** — one formula over a signed curvature `k`. `k = 0` is the plane,
  exactly; `k > 0` is a planet of radius `1 / k`; `k < 0` is a bowl, the sphere
  closing round the camera, with the grid on its inside. No term divides by
  `k`, so the ground slides from planet to bowl through the plane without a
  seam. Ground coordinate `( 0, 0 )` is the world origin at every curvature, so
  content authored on a plane keeps its place when the ground bends.
  `Ground::frame( u, v )` is an east/north/normal frame at any point, the normal
  on the camera's side; `Ground::intersect` is the ray test, near side of a
  planet, far wall of a bowl.
- **`Pose`** — a target `( u, v )` on the ground, a distance, an azimuth and a
  polar angle. The camera never rolls: `up` is the ground normal. In a bowl the
  distance is capped at the radius, and at the cap the camera is the sphere's
  centre, looking round at the wall: the panopticon.
- **`MapControls`** — left-drag grabs the ground (the ray test, solved by
  Newton's method so what was under the cursor stays there), right-drag or
  ctrl-drag tilts and orbits, the wheel zooms to the cursor, the arrows pan, and
  `Tab` is an overview fitted to a set of `Pane`s, from which a click drops onto
  one. Every field is damped with `smooth_damp`, the curvature included, so
  `set_curvature` morphs the ground; `Damping` holds the smooth times, for an
  app that wants a different feel.

## Usage

```rust,no_run
use three_rs::cameras::PerspectiveCamera;
use three_rs_controls::{Ground, MapControls, Pose};

let mut controls = MapControls::new(
    Ground::flat(),
    Pose {
        u: 0.0,
        v: 0.0,
        distance: 160.0,
        azimuth: 0.0,
        polar: 0.0,
    },
);

let mut camera = PerspectiveCamera::new(60.0, 16.0 / 9.0, 1.0, 50_000.0);
controls.update(1.0 / 60.0);
controls.apply(&mut camera);
```

```
cargo test -p three-rs-controls --lib             # no GPU
cargo run --release -p three-rs-controls --bin heli
```

`heli` is the demo, and its key map is on screen. It opens on a planet of
radius 500 (`k = 1/500`): tilt with a right-drag and the horizon is in shot;
left-drag and what was under the cursor stays there as the ball turns under
you. **left-drag** grabs the ground, **right-drag** (or **ctrl**-drag) orbits,
the **wheel** zooms to the pointer, the **arrows** pan, **[** bends the ground
toward a bowl and **]** toward a planet, a step at a time through the plane,
**P** flattens it, **O** closes the bowl round the camera (the panopticon),
**Home** resets, **Tab** is the overview (click a pane to drop onto it), **Esc**
quits. `--headless out.png` renders one frame, `--no-legend` without the key
map, and `--curvature` sets the ground, negative for a bowl and a fraction if
that reads better:

```sh
cargo run --release -p three-rs-controls --bin heli -- \
    --headless shots/heli-panopticon.png \
    --curvature -1/160 --pose 0,0,160,0,90 [--overview] [--size 1600x1000] [--no-legend]
```

`shots/` holds eight such frames: the plane and the `1/500` planet by three
poses, then a `-1/300` bowl from 160 out at 60°, and the panopticon above,
looking level from the centre of a `-1/160` bowl.
