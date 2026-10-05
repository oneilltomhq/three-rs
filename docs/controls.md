# Controls

`three_rs::addons::controls` ports three.js' `examples/jsm/controls/` classes
that the WebGPU example pages need: `OrbitControls`, `FirstPersonControls`,
`FlyControls` and `TransformControls`. The
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
  only need an `Object3D` and take a `&ObjectRef` (`&camera.node`). Transform
  takes any `TransformCamera` (perspective or orthographic), since it
  raycasts through it.
- **Events are return values.** There is no `EventDispatcher`. Where the JS
  dispatches `change`, `update` returns the condition it dispatches on.
  TransformControls dispatches from many methods, so each of them returns the
  `Vec<TransformControlsEvent>` it would have dispatched, in order.
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

`look_at` and `update` point the object with `ObjectRef::look_at`, which is
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

## TransformControls

`examples/jsm/controls/TransformControls.js`: the translate, rotate and scale
gizmo, with its picker meshes, helper lines and drag plane. The port builds
the same graph three builds (the same names, handle order, geometry,
transforms, `renderOrder` of infinity and material colours and opacities) under
the node `get_helper()` returns, which the host adds to its scene. The drag
math is the JS's, operation for operation, including the JS's own quirks:
snaps tested for truthiness, `Math.round`, the `|| scaleSnap` that turns a
scale rounded to 0 back into the snap, and the substring tests on axis names.

| JS | port |
|---|---|
| `new TransformControls( camera, el )` | `TransformControls::new()` plus `set_element_size(w, h)` |
| `getHelper()` | `get_helper() -> &ObjectRef` |
| `attach( object )` / `detach()` / `reset()` | the same, each returning its events |
| `disconnect()` | `disconnect()`: stops moves from dragging |
| `setMode` / `setSpace` / `setSize` / `setTranslationSnap` / `setRotationSnap` / `setScaleSnap` | `set_mode(Mode)`, `set_space(Space)`, `set_size`, `set_*_snap(Option<f64>)` |
| `setColors( x, y, z, active )` | `set_colors(Color, Color, Color, Color)` |
| `getRaycaster()` | `get_raycaster()` |
| `getMode()`, `axis`, `dragging`, `worldPosition`, `eye`, … | getters of the same names in snake case |
| `enabled`, `showX` … `showE`, `minX` … `maxZ` | public fields |
| `pointerdown` / `pointermove` / `pointerup` | `on_pointer_down(&e, camera)` / `on_pointer_move(&e, camera)` / `on_pointer_up(&e)` with a `TransformPointerEvent` |
| `pointerHover` / `pointerDown` / `pointerMove` / `pointerUp` (`{ x, y, button }` in NDC) | `pointer_hover` / `pointer_down` / `pointer_move` / `pointer_up` with `Option<&Pointer>` |
| the helper's `updateMatrixWorld` overrides, run by the renderer | `update(camera)` |
| `addEventListener( 'change' \| 'objectChange' \| 'mouseDown' \| 'mouseUp' \| '<prop>-changed', … )` | the `Vec<TransformControlsEvent>` each call returns |
| `viewport`, `document.pointerLockElement` | the `viewport: Option<Vector4>` and `pointer_locked` fields |

`TransformCamera` is `RenderCamera` plus the gizmo's size factor, and is
implemented for `PerspectiveCamera` and `OrthographicCamera`.

Routing:

- **Capture on down.** The JS captures the pointer on `pointerdown` (unless the
  pointer is locked), so send moves and the up until the button is released,
  even after the pointer leaves the element.
- **Touch has no hover.** Hover listens for mouse and pen only, so a touch
  first picks a handle in its own `pointerdown`. Pass the right
  `PointerType` and the port does the same.
- **Call `update(camera)` once a frame, before rendering.** In three the
  helper's matrix update runs inside `renderer.render`; here the host calls it,
  after the attached object and the camera are where they will be drawn.
  Pointer methods raycast against the gizmo as the last `update` left it, as
  three's do against the last render.

Differences, all in the struct's doc comment:

- **`toneMapped: false` is dropped.** The port's materials have no
  tone-mapping switch, so under a tone-mapped output the gizmo's colours are
  tone mapped too.
- **Plain fields send nothing.** `enabled`, the `show*` flags and the `min*` /
  `max*` limits are `defineProperty` properties in the JS, whose assignment
  dispatches `<name>-changed` and `change`. They are fields here and send no
  events; the setters (`set_mode` and the rest) do.
- **Per-handle materials.** Three's pickers share one invisible material; the
  port gives every handle its own. Picker colours differ, but pickers are
  never drawn.
- **Per-instance working state.** The module-level `_dirVector` and raycaster
  belong to each instance, so `get_raycaster()` returns this instance's
  raycaster where three's `getRaycaster()` returns the one all instances
  share.
- **No `camera` property.** The methods that read the camera take it as an
  argument, so there is no `camera` field and no `camera-changed` event.
- **`set_colors` takes `Color`s**, not any argument `Color.set()` accepts.
- **An object with no parent.** Three's root override logs a
  `console.error` and carries on, and so does the port; three's
  `pointerDown` then throws on `object.parent.updateMatrixWorld()`, where the
  port skips that call.
- **`reset()` after `detach()` mid-drag.** `dragging` is still true, so three
  throws on `this.object.position` before dispatching anything; the port
  returns no events and changes nothing.
- **No listeners.** `connect()`, `dispose()` and pointer capture are the
  host's: it decides which `on_pointer_*` calls to make. `disconnect()` is
  kept for the one removal the port can see, the drag listener's: after it,
  `on_pointer_move` no longer drags, so a host can stop a drag from outside.
  As in three, `dragging` and `axis` stay as they are until the next up.

## The gate

Each port is checked against three's own class, run under node:

| class | reference script | fixture | test | tolerance |
|---|---|---|---|---|
| `OrbitControls` | `tools/orbit_controls_reference.mjs` | (run live) | `tests/addons_orbit_controls.rs` | 1e-9 |
| `FirstPersonControls` | `tools/first_person_controls_reference.mjs` | `tests/fixtures/first_person_controls.json` | `tests/addons_first_person_controls.rs` | 1e-9 |
| `FlyControls` | `tools/fly_controls_reference.mjs` | `tests/fixtures/fly_controls.json` | `tests/addons_fly_controls.rs` | 1e-9 |
| `TransformControls` | `tools/transform_controls_reference.mjs` | `tests/fixtures/transform_controls.json` | `tests/addons_transform_controls.rs` | 1e-9 |

A script loads the class out of the three.js checkout (`THREE_JS_DIR`, at the
revision CI pins), constructs it with no element so it registers no listeners,
stubs the few element properties the handlers read, calls the bound handlers
(`_onPointerDown`, `_onKeyDown`, …) with plain event objects through scripted
scenarios, and records the camera's position and quaternion (and, for Fly,
whether `change` fired) after every step. The test hard-codes the same
scenarios, replays them through the port, and compares every number.

TransformControls records more per step: the events dispatched, the object's
transform, `axis`, `mode`, `dragging`, `rotationAngle`, which handles are
visible and highlighted, and the plane's orientation, plus on marked steps
every handle's transform and colour, the world matrices and the working
vectors. The fixture also holds the whole gizmo graph, which a separate test
compares node for node (names, types, transforms, materials, and geometry by
vertex and index counts and coordinate sums). Its test module lists the
branches no scenario reaches.

The FirstPerson, Fly and Transform fixtures are committed, so their tests compare
everywhere, node or not. Where a checkout and node are present, each test also
reruns its script into a scratch file and compares the port against that, so
a fixture that no longer matches the pinned three.js fails; with `CI` set, a
missing checkout or node fails the test instead of skipping it, and the
scripts refuse a checkout whose `REVISION` is not the pinned one. After changing a
script, regenerate its fixture with `node tools/<name>_reference.mjs`.
