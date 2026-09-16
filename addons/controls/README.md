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

- **`Ground`** — one sphere formula. `R = 1e7` reads as a plane; a small `R` is a
  planet. Ground coordinate `( 0, 0 )` is the world origin at every radius, so
  content authored on a plane keeps its place when the ground is curled up.
  `Ground::frame( u, v )` is an east/north/normal frame at any point.
- **`Pose`** — a target `( u, v )` on the ground, a distance, an azimuth and a
  polar angle. The camera never rolls: `up` is the ground normal.
- **`MapControls`** — left-drag grabs the ground (ray-sphere, iterated to a
  fixed point so what was under the cursor stays there), right-drag or
  ctrl-drag tilts and orbits, the wheel zooms to the cursor, the arrows pan, and
  `Tab` is an overview fitted to a set of `Pane`s, from which a click drops onto
  one. Every field is damped with `smooth_damp`; `Damping` holds the smooth
  times so a demo can A/B the feel.

```
cargo test -p three-rs-controls --lib             # no GPU
cargo run --release -p three-rs-controls --bin heli
```

`heli` is the demo. **left-drag** grabs the ground, **right-drag** (or
**ctrl**-drag) orbits, the **wheel** zooms to the pointer, the **arrows** pan,
**[** **]** curl the ground and **P** flattens it, **Home** resets, **Tab** is
the overview (click a pane to drop onto it), **Esc** quits. `--headless out.png`
renders one frame:

```sh
cargo run --release -p three-rs-controls --bin heli -- \
    --headless shots/heli-sphere-high.png \
    --radius 300 --pose 0,0,800,0,20 [--overview] [--size 1600x1000]
```

`shots/` holds six such frames: two grounds by three poses.
