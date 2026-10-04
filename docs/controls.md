# Controls

`three_rs::addons::controls` ports three.js' `examples/jsm/controls/` classes
that the WebGPU example pages need: `OrbitControls`, `FirstPersonControls` and
`FlyControls`. `TransformControls` (3 pages) is not ported yet. The
`three-rs-controls` workspace crate (`addons/controls/`) is a different thing,
a map camera over a ground that is not a port; see the README's "Addons".

## The shape every port shares

- **Same state, same defaults, same `update()`.** The public fields are the
  JS properties in snake case with the JS defaults, and `update` performs the
  same floating-point operations in the same order, so the camera ends a
  frame where three's would to the last few bits.
- **Input is method calls, not DOM listeners.** The JS classes register
  `pointerdown`, `keydown` and the rest on an element in `connect()`. Here the
  host owns its event source (winit, the browser shell, a test) and calls one
  method per handler, passing the fields the handler reads. `connect()`,
  `disconnect()`, `dispose()` and the `contextmenu` suppression have no
  counterpart because they are listener bookkeeping.
- **Element sizes are setters.** Where a handler reads the element's size
  (`clientWidth` for Orbit, `offsetWidth` for Fly), the host calls
  `set_element_size(width, height)`. Pointer coordinates are element-relative
  with y down, so the host subtracts the element's offset that the JS reads
  off the DOM.
- **The camera is an argument.** The camera belongs to the application and
  the controls sit beside it, so the methods that touch it take it. Orbit
  takes `&mut PerspectiveCamera` because it reads `fov`; FirstPerson and Fly
  only need an `Object3D` and take a `&Node` (`&camera.node`).
- **Events are return values.** There is no `EventDispatcher`. Where the JS
  dispatches `change`, `update` returns the condition it dispatches on.
- **Keys.** FirstPerson and Fly share `KeyCode`; OrbitControls keeps its own
  four-arrow `Key` (inside a `KeyEvent` with modifiers). The split is
  deliberate, and Orbit's API is left as it is.

## FirstPersonControls

`examples/jsm/controls/FirstPersonControls.js` as of the pinned 5f610f5
(r187dev); the damped, world-axis version landed in r186 (#33874, f88964a,
#34485). Movement and look are velocities eased by `dampingFactor`, the look
velocity comes from the drag's offset from where it *started*, keys move along
world axes from the camera's yaw, and the pointer moves along the look
direction. The element's size is not read, there is no `activeLook`, and
`handleResize()` has been a deprecated no-op since r184.

Routing: the JS captures the pointer on `pointerdown` and listens for
`pointermove` / `pointerup` on the element's `ownerDocument`, so the host
should keep sending moves and ups after the pointer leaves the element (until
the button is released), not only while it is over it.

| JS | port |
|---|---|
| `new FirstPersonControls( camera, el )` | `FirstPersonControls::new(&camera.node)` |
| `update( delta )` | `update(&camera.node, delta)` |
| `lookAt( x, y, z )` / `lookAt( v )` | `look_at(&camera.node, &v)` |
| `pointerdown` (mouse, pen) | `pointer_down(button, x, y)` |
| `pointerdown` (`pointerType === 'touch'`) | `touch_start(x, y)` |
| `pointerup` / `pointercancel` | `pointer_up(button)` / `touch_end()` |
| `pointermove` | `pointer_move(x, y)` |
| `keydown` / `keyup` | `key_down(KeyCode)` / `key_up(KeyCode)` |
| `mouseDragOn` (read-only) | `mouse_drag_on()` |

`look_at` and `update` point the object with `Node::look_at`, which is
three's `Object3D.lookAt` in full, so a camera inside a moved and turned group
behaves as it does in three (the gate has a scenario for it).

## FlyControls

`examples/jsm/controls/FlyControls.js`: `_moveState` is the public
`move_state: MoveState`, and `update_movement_vector()` /
`update_rotation_vector()` are public so that a host that writes the state
directly can rebuild the two vectors the way the handlers do.

| JS | port |
|---|---|
| `new FlyControls( camera, el )` | `FlyControls::new()` plus `set_element_size(w, h)` |
| `update( delta )`, dispatching `change` | `update(&camera.node, delta) -> bool` |
| `keydown` (ignored with Alt) / `keyup` | `key_down(KeyCode, alt_key)` / `key_up(KeyCode)` |
| `pointerdown` / `pointerup` | `pointer_down(button)` / `pointer_up(button)` |
| `pointermove` | `pointer_move(x, y)`, element-relative |
| `pointercancel` | `pointer_cancel()` |

Shift sets `movement_speed_multiplier` to 0.1 as in the JS, and, as in the JS,
nothing reads it.

Routing: the JS listens only on the element itself (no pointer capture, no
`ownerDocument`), so the host sends input only while the pointer is over it.
`onPointerDown` reads `event.button` for touch as well, where browsers report
0, so send a touch press as `MouseButton::Left`.

## The gate

Each port is checked against three's own class, run under node:

| class | reference script | fixture | test | tolerance |
|---|---|---|---|---|
| `OrbitControls` | `tools/orbit_controls_reference.mjs` | (run live) | `tests/addons_orbit_controls.rs` | 1e-9 |
| `FirstPersonControls` | `tools/first_person_controls_reference.mjs` | `tests/fixtures/first_person_controls.json` | `tests/addons_first_person_controls.rs` | 1e-9 |
| `FlyControls` | `tools/fly_controls_reference.mjs` | `tests/fixtures/fly_controls.json` | `tests/addons_fly_controls.rs` | 1e-9 |

A script loads the class out of the three.js checkout (`THREE_JS_DIR`, at the
revision CI pins), constructs it with no element so it registers no listeners,
stubs the few element properties the handlers read, calls the bound handlers
(`_onPointerDown`, `_onKeyDown`, …) with plain event objects through scripted
scenarios, and records the camera's position and quaternion (and, for Fly,
whether `change` fired) after every step. The test hard-codes the same
scenarios, replays them through the port, and compares every number.

The FirstPerson and Fly fixtures are committed, so their tests compare
everywhere, node or not. Where a checkout and node are present, each test also
reruns its script into a scratch file and compares the port against that, so
a fixture that no longer matches the pinned three.js fails; with `CI` set, a
missing checkout or node fails the test instead of skipping it, and the
scripts refuse a checkout whose `REVISION` is not the pinned one. After changing a
script, regenerate its fixture with `node tools/<name>_reference.mjs`.
