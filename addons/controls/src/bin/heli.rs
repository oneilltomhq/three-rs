//! The demo for [`three_rs_controls`]: a map camera over a ground that can be
//! flat or a small planet, with a grid of panes lying on it.
//!
//! ```text
//! cargo run --release -p three-rs-controls --bin heli
//! cargo run --release -p three-rs-controls --bin heli -- --headless shots/heli-plane-low.png
//! cargo run --release -p three-rs-controls --bin heli -- --headless shots/x.png --radius 300 --overview
//! ```
//!
//! Left-drag grabs the ground and pulls it under the cursor, right-drag (or
//! ctrl and left-drag) orbits, the wheel zooms toward the pointer, the arrow
//! keys pan, `[` and `]` curl the ground up and flatten it again, `P` snaps it
//! flat, `Home` returns to the start pose, `Tab` is the overview and `Esc`
//! quits. In the overview a click picks a pane and drops onto it.
//!
//! The window, the surface, the event loop and the headless path are lifted
//! from `src/bin/viewer.rs`; the scene and the controls are this file's own.
//! Nothing here is reachable from the e2e harness.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use three_rs_controls::{Damping, Ground, MapControls, Mode, Pane, Pose};
use three_rs::core::{BufferAttribute, BufferGeometry};
use three_rs::materials::Side;
use three_rs::math::math_utils::{DEG2RAD, RAD2DEG};
use three_rs::{
    plane_geometry, Color, LineSegments, Matrix4, Mesh, MeshBasicNodeMaterial, Node,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, Modifiers, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

// ------------------------------------------------------------------ the scene

/// The vertical field of view the fit and the projection both use.
const FOV: f64 = 60.0;

/// The grid runs over `[ -GRID_EXTENT, GRID_EXTENT ]` in both ground
/// coordinates, with an iso-line every `GRID_STEP` and a vertex every
/// `GRID_STEP` along each line, so a line bends with the ground.
const GRID_EXTENT: f64 = 400.0;
const GRID_STEP: f64 = 10.0;

const PANE_COLUMNS: i32 = 7;
const PANE_ROWS: i32 = 7;
const PANE_SPACING: f64 = 50.0;
const PANE_WIDTH: f64 = 16.0;
const PANE_HEIGHT: f64 = 9.0;

/// Eight colours, cycling by pane index.
const PANE_COLOURS: [u32; 8] = [
    0xe06c75, 0xe5c07b, 0x98c379, 0x56b6c2, 0x61afef, 0xc678dd, 0xd19a66, 0xabb2bf,
];

fn start_pose() -> Pose {
    Pose {
        u: 0.0,
        v: 0.0,
        distance: 160.0,
        azimuth: 0.0,
        polar: 0.0,
    }
}

fn demo_panes() -> Vec<Pane> {
    let mut panes = Vec::new();
    for row in 0..PANE_ROWS {
        for column in 0..PANE_COLUMNS {
            panes.push(Pane {
                u: (column as f64 - (PANE_COLUMNS as f64 - 1.0) * 0.5) * PANE_SPACING,
                v: (row as f64 - (PANE_ROWS as f64 - 1.0) * 0.5) * PANE_SPACING,
                width: PANE_WIDTH,
                height: PANE_HEIGHT,
            });
        }
    }
    panes
}

/// The flat `[ x, y, z, … ]` the grid's `position` attribute holds: every
/// iso-line of `u` and of `v`, as a `line-list` of short chords that follow the
/// surface.
fn grid_positions(ground: &Ground) -> Vec<f32> {
    let steps = (2.0 * GRID_EXTENT / GRID_STEP).round() as i32;
    let mut positions = Vec::with_capacity((steps as usize + 1) * steps as usize * 12);

    let mut push = |a: Vector3, b: Vector3| {
        positions.extend_from_slice(&[a.x as f32, a.y as f32, a.z as f32]);
        positions.extend_from_slice(&[b.x as f32, b.y as f32, b.z as f32]);
    };

    for line in 0..=steps {
        let fixed = -GRID_EXTENT + line as f64 * GRID_STEP;
        for segment in 0..steps {
            let from = -GRID_EXTENT + segment as f64 * GRID_STEP;
            let to = from + GRID_STEP;
            // An iso-line of `u`, then an iso-line of `v`.
            push(ground.point(fixed, from), ground.point(fixed, to));
            push(ground.point(from, fixed), ground.point(to, fixed));
        }
    }

    positions
}

struct App {
    renderer: Renderer,
    scene: Scene,
    camera: PerspectiveCamera,
    controls: MapControls,
    panes: Vec<Pane>,
    grid: Node,
    pane_nodes: Vec<Node>,
    /// The radius the grid's vertices were last built for.
    grid_radius: f64,
    size: (u32, u32),
}

impl App {
    /// Builds the scene on `instance` — the window's, so the surface and the
    /// renderer share an adapter — or on the renderer's own for the headless
    /// path.
    fn build(instance: Option<wgpu::Instance>, size: (u32, u32)) -> Self {
        let ground = Ground::new(1e7);
        let controls = MapControls::new(ground, start_pose());
        let panes = demo_panes();

        let mut scene = Scene::new();
        scene.set_background(Color::from_hex(0x111111));

        // The grid: one `LineSegments`, rewritten in place when the radius
        // changes, never rebuilt.
        let grid = {
            let mut geometry = BufferGeometry::new();
            let positions = grid_positions(&ground);
            geometry.set_attribute("position", BufferAttribute::new(positions, 3));
            LineSegments::new(
                Rc::new(geometry),
                MeshBasicNodeMaterial::line(Color::from_hex(0x3a3a3a)),
            )
        };
        scene.add(&grid);

        // The panes: one shared geometry, one material each.
        let pane_geometry = Rc::new(plane_geometry(PANE_WIDTH, PANE_HEIGHT, 1, 1));
        let pane_nodes: Vec<Node> = panes
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let mut material = MeshBasicNodeMaterial::new();
                material.color = Color::from_hex(PANE_COLOURS[index % PANE_COLOURS.len()]);
                material.side = Side::Double;
                let node = Mesh::new(pane_geometry.clone(), material);
                scene.add(&node);
                node
            })
            .collect();

        let camera =
            PerspectiveCamera::new(FOV, size.0 as f64 / size.1.max(1) as f64, 1.0, 50_000.0);

        let renderer = match instance {
            Some(instance) => {
                Renderer::with_instance(RendererParameters { antialias: true }, instance)
            }
            None => Renderer::new(RendererParameters { antialias: true }),
        }
        .expect("three-rs heli: cannot create the renderer");

        let mut app = Self {
            renderer,
            scene,
            camera,
            controls,
            panes,
            grid,
            pane_nodes,
            grid_radius: ground.radius(),
            size,
        };
        app.set_size(size.0, size.1);
        app
    }

    fn set_size(&mut self, width: u32, height: u32) {
        let (width, height) = (width.max(1), height.max(1));
        self.size = (width, height);
        self.camera.aspect = width as f64 / height as f64;
        self.camera.update_projection_matrix();
        self.renderer.set_size(width as f64, height as f64);
    }

    fn aspect(&self) -> f64 {
        self.camera.aspect
    }

    /// Writes the controller's current pose onto the camera and the scene:
    /// the grid's vertices if the ground curled since the last frame, then
    /// every pane's transform, then the camera.
    fn sync(&mut self) {
        let radius = self.controls.ground().radius();
        if (radius - self.grid_radius).abs() > 1e-6 * self.grid_radius {
            let positions = grid_positions(self.controls.ground());
            self.grid
                .borrow()
                .line()
                .expect("the grid is a LineSegments")
                .set_positions(&positions);
            self.grid_radius = radius;
        }

        for (pane, node) in self.panes.iter().zip(&self.pane_nodes) {
            let position = self.controls.pane_centre(pane);

            // The plane geometry lies in local `XY` and faces `+Z`. Flat on the
            // ground, face up, top edge north: local `+Z` is the normal, local
            // `+Y` is `north`, and local `+X` is `north × normal` so that the
            // basis stays right-handed and the face is not mirrored.
            let frame = self.controls.ground().frame(pane.u, pane.v);
            let side = frame.north.crossed(&frame.normal);
            let mut basis = Matrix4::identity();
            basis.make_basis(&side, &frame.north, &frame.normal);

            let mut object = node.borrow_mut();
            object.position = position;
            object.set_rotation_from_matrix(&basis);
        }

        self.controls.apply(&mut self.camera);
    }

    fn render(&mut self) {
        self.sync();
        self.renderer.render(&mut self.scene, &mut self.camera);
    }

    /// The window title, which is also what documents a screenshot.
    fn title(&self) -> String {
        let pose = self.controls.current();
        format!(
            "heli — {} — R {:.0} — u {:.0} v {:.0} d {:.0} — az {:.0}° polar {:.0}°{}",
            match self.controls.mode() {
                Mode::Free => "free",
                Mode::Overview => "overview",
            },
            self.controls.ground().radius(),
            pose.u,
            pose.v,
            pose.distance,
            pose.azimuth * RAD2DEG,
            pose.polar * RAD2DEG,
            if self.controls.rested() {
                " — rest"
            } else {
                ""
            },
        )
    }

    /// Pixel coordinates in the window to normalised device coordinates.
    fn ndc(&self, at: (f64, f64)) -> (f64, f64) {
        let (width, height) = (self.size.0 as f64, self.size.1.max(1) as f64);
        (at.0 / width * 2.0 - 1.0, 1.0 - at.1 / height * 2.0)
    }
}

// ------------------------------------------------------------------ the window

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
}

/// A press, so a release can tell a click from a drag, and a move can tell a
/// grab from an orbit.
struct Press {
    button: MouseButton,
    /// `true` when the press orbits: the right button, or the left with ctrl.
    rotating: bool,
    at: (f64, f64),
    moved: f64,
}

/// The arrow keys currently down.
#[derive(Default)]
struct Held {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

struct Heli {
    instance: Option<wgpu::Instance>,
    app: Option<App>,
    gpu: Option<Gpu>,
    requested_size: (u32, u32),
    /// The feel, from the command line: three smooth times and the swapchain's
    /// frame latency, so the numbers can be A/B'd without a rebuild.
    damping: Damping,
    frame_latency: u32,

    held: Held,
    modifiers: Modifiers,
    cursor: (f64, f64),
    press: Option<Press>,
    last_frame: Instant,
    /// Set by a resize, and by the first frame, so the redraw is not skipped
    /// just because nothing was damped.
    dirty: bool,
}

impl Heli {
    fn new(size: (u32, u32), damping: Damping, frame_latency: u32) -> Self {
        Self {
            instance: None,
            app: None,
            gpu: None,
            requested_size: size,
            damping,
            frame_latency,
            held: Held::default(),
            modifiers: Modifiers::default(),
            cursor: (0.0, 0.0),
            press: None,
            last_frame: Instant::now(),
            dirty: true,
        }
    }

    fn configure_surface(&mut self) {
        let (Some(gpu), Some(app)) = (&self.gpu, &mut self.app) else {
            return;
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![gpu.format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: app.size.0.max(1),
            height: app.size.1.max(1),
            desired_maximum_frame_latency: self.frame_latency,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        gpu.surface.configure(app.renderer.device(), &config);
    }

    fn redraw(&mut self) {
        let Some(app) = self.app.as_mut() else { return };
        app.render();

        let Some(gpu) = &self.gpu else { return };
        gpu.window.set_title(&app.title());

        let surface_texture = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                let gpu = self.gpu.as_mut().unwrap();
                gpu.surface = self
                    .instance
                    .as_ref()
                    .unwrap()
                    .create_surface(gpu.window.clone())
                    .unwrap();
                self.configure_surface();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                panic!("three-rs heli: surface validation error")
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(gpu.format),
                ..Default::default()
            });

        let presented = app.renderer.present(&view, gpu.format);
        assert!(presented, "three-rs heli: nothing to present");

        gpu.window.pre_present_notify();
        app.renderer.queue().present(surface_texture);
    }

    /// The click that lands on a pane in the overview.
    fn click(&mut self, at: (f64, f64)) {
        let Some(app) = self.app.as_mut() else { return };
        if app.controls.mode() != Mode::Overview {
            return;
        }

        let (ndc_x, ndc_y) = app.ndc(at);

        if let Some(index) = app.controls.pick(&app.panes, ndc_x, ndc_y, &app.camera) {
            let pane = app.panes[index];
            let (fov, aspect) = (app.camera.fov, app.aspect());
            app.controls.focus_pane(&pane, fov, aspect);
        }
    }
}

impl ApplicationHandler for Heli {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("heli")
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.requested_size.0,
                            self.requested_size.1,
                        )),
                )
                .expect("three-rs heli: cannot create a window"),
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                event_loop.owned_display_handle(),
            ))
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("three-rs heli: cannot create a surface");

        let size = window.inner_size();
        let size = (size.width.max(1), size.height.max(1));
        let mut app = App::build(Some(instance.clone()), size);
        app.controls.set_damping(self.damping);

        let caps = surface.get_capabilities(app.renderer.adapter());
        let preferred = caps.formats[0];
        // The canvas holds sRGB-encoded bytes; an `-srgb` view would encode
        // them twice.
        let linear = preferred.remove_srgb_suffix();
        let format = if caps.formats.contains(&linear) {
            linear
        } else {
            preferred
        };

        self.instance = Some(instance);
        self.app = Some(app);
        self.gpu = Some(Gpu {
            window: window.clone(),
            surface,
            format,
        });
        self.requested_size = size;
        self.configure_surface();
        self.last_frame = Instant::now();

        println!("feel: {}", feel(&self.damping, self.frame_latency));
        println!(
            "heli — drag the ground, right-drag (or ctrl-drag) to orbit, \
             the wheel zooms to the pointer, the arrows pan, [ ] curl the ground, \
             P flattens it, Home resets, Tab is the overview, Esc quits"
        );
        println!("{PRESETS}");

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                let Some(app) = self.app.as_mut() else { return };

                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) if pressed => event_loop.exit(),
                    Key::Named(NamedKey::Tab) if pressed && !event.repeat => {
                        let aspect = app.aspect();
                        let panes = app.panes.clone();
                        app.controls.toggle_overview(&panes, FOV, aspect);
                    }
                    Key::Named(NamedKey::Home) if pressed => app.controls.reset(),
                    Key::Named(NamedKey::ArrowUp) => self.held.up = pressed,
                    Key::Named(NamedKey::ArrowDown) => self.held.down = pressed,
                    Key::Named(NamedKey::ArrowLeft) => self.held.left = pressed,
                    Key::Named(NamedKey::ArrowRight) => self.held.right = pressed,
                    Key::Character(text) => {
                        for character in text.chars().flat_map(char::to_lowercase) {
                            match character {
                                '[' if pressed => app.controls.scale_radius(1.25),
                                ']' if pressed => app.controls.scale_radius(1.0 / 1.25),
                                'p' if pressed => app.controls.set_radius(1e7),
                                c if pressed && !event.repeat => {
                                    if let Some((what, damping)) = preset(c, app.controls.damping())
                                    {
                                        app.controls.set_damping(damping);
                                        self.damping = damping;
                                        println!("{what}: {}", feel(&damping, self.frame_latency));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers,

            WindowEvent::Resized(size) => {
                let size = (size.width.max(1), size.height.max(1));
                if let Some(app) = self.app.as_mut() {
                    app.set_size(size.0, size.1);
                }
                self.configure_surface();
                self.dirty = true;
            }

            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => {
                    let rotating = button == MouseButton::Right
                        || (button == MouseButton::Left && self.modifiers.state().control_key());
                    self.press = Some(Press {
                        button,
                        rotating,
                        at: self.cursor,
                        moved: 0.0,
                    });
                    let cursor = self.cursor;
                    if let Some(app) = self.app.as_mut() {
                        app.controls.set_dragging(true);
                        if !rotating && button == MouseButton::Left {
                            let (x, y) = app.ndc(cursor);
                            app.controls.grab_begin(x, y, &app.camera);
                        }
                    }
                }
                ElementState::Released => {
                    if let Some(app) = self.app.as_mut() {
                        app.controls.set_dragging(false);
                        app.controls.grab_end();
                    }
                    if let Some(press) = self.press.take() {
                        if press.button == MouseButton::Left && press.moved < 4.0 {
                            self.click(press.at);
                        }
                    }
                }
            },

            WindowEvent::CursorMoved { position, .. } => {
                let last = self.cursor;
                self.cursor = (position.x, position.y);
                let (dx, dy) = (self.cursor.0 - last.0, self.cursor.1 - last.1);

                let height = self.app.as_ref().map(|app| app.size.1).unwrap_or(1).max(1) as f64;
                if let Some(press) = self.press.as_mut() {
                    press.moved += dx.hypot(dy);
                }

                let Some((button, rotating)) = self
                    .press
                    .as_ref()
                    .map(|press| (press.button, press.rotating))
                else {
                    return;
                };
                let cursor = self.cursor;
                let Some(app) = self.app.as_mut() else { return };

                if rotating {
                    app.controls.rotate(dx, dy, height);
                } else if button == MouseButton::Left {
                    let (x, y) = app.ndc(cursor);
                    app.controls.grab_move(x, y, &app.camera);
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 100.0,
                };
                let cursor = self.cursor;
                if let Some(app) = self.app.as_mut() {
                    let (x, y) = app.ndc(cursor);
                    app.controls.dolly(steps, x, y, &app.camera);
                }
            }

            WindowEvent::RedrawRequested => self.redraw(),

            _ => {}
        }
    }

    /// The held arrow keys are applied here, with the real elapsed time, so
    /// panning is frame-rate independent; and the frame is only drawn when the controller
    /// says something moved.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let dt = self.last_frame.elapsed().as_secs_f64().min(0.1);
        self.last_frame = Instant::now();

        let Some(app) = self.app.as_mut() else { return };

        let axis = |positive: bool, negative: bool| f64::from(positive) - f64::from(negative);
        let forward = axis(self.held.up, self.held.down);
        let right = axis(self.held.right, self.held.left);
        app.controls.pan(forward, right, dt);

        let moved = app.controls.update(dt);
        if moved || self.dirty {
            self.dirty = false;
            if let Some(gpu) = &self.gpu {
                gpu.window.request_redraw();
            }
        }
    }
}

// ---------------------------------------------------------------- the feel

const PRESETS: &str = "presets, stock first — \
    drag smooth: 1 0.125  2 0.06  3 0.02 | release glide: a 0.25  s 0.15  d 0.08 | \
    wheel smooth: 8 0.25  9 0.12  0 0.05 | wheel step: h 0.95  j 0.85  k 0.75";

/// The command line that reproduces `damping`, for pasting back.
fn feel(damping: &Damping, frame_latency: u32) -> String {
    format!(
        "--smooth {} --drag-smooth {} --wheel-smooth {} --dolly-step {} --frame-latency {}",
        damping.smooth_time,
        damping.dragging_smooth_time,
        damping.wheel_smooth_time,
        damping.dolly_step,
        frame_latency
    )
}

/// One preset key applied to `damping`: what it changed, and the result.
fn preset(key: char, mut damping: Damping) -> Option<(&'static str, Damping)> {
    let what = match key {
        '1' | '2' | '3' => {
            damping.dragging_smooth_time = match key {
                '1' => 0.125,
                '2' => 0.06,
                _ => 0.02,
            };
            "drag smooth"
        }
        'a' | 's' | 'd' => {
            damping.smooth_time = match key {
                'a' => 0.25,
                's' => 0.15,
                _ => 0.08,
            };
            "release glide"
        }
        '8' | '9' | '0' => {
            damping.wheel_smooth_time = match key {
                '8' => 0.25,
                '9' => 0.12,
                _ => 0.05,
            };
            "wheel smooth"
        }
        'h' | 'j' | 'k' => {
            damping.dolly_step = match key {
                'h' => 0.95,
                'j' => 0.85,
                _ => 0.75,
            };
            "wheel step"
        }
        _ => return None,
    };
    Some((what, damping))
}

// ---------------------------------------------------------------- headless

/// One settled frame, written as a PNG. Nothing is damped: the controller is
/// put on its target pose and rendered once.
fn headless(path: &str, size: (u32, u32), pose: Option<Pose>, radius: Option<f64>, overview: bool) {
    let mut app = App::build(None, size);

    if let Some(radius) = radius {
        app.controls.set_radius(radius);
    }
    if let Some(pose) = pose {
        app.controls = MapControls::new(Ground::new(app.controls.target_radius()), pose);
    }
    app.controls.settle();

    if overview {
        let aspect = app.aspect();
        let panes = app.panes.clone();
        app.controls.toggle_overview(&panes, FOV, aspect);
        app.controls.settle();
    }

    app.render();
    app.renderer
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    three_rs::testing::write_png(path, width, height, &pixels);
    println!("wrote {path} ({width}x{height}) — {}", app.title());
}

const USAGE: &str = "usage: heli [--headless out.png] \
                     [--pose u,v,distance,azimuth_deg,polar_deg] \
                     [--radius R] [--overview] [--size WxH] \
                     [--smooth S] [--drag-smooth S] [--wheel-smooth S] \
                     [--dolly-step F] [--frame-latency N]\n\
       the feel: smooth times in seconds (camera-controls' 0.25 / 0.125 / 0.25), \
                     the distance factor per wheel notch (0.95, smaller is more sensitive) \
                     and the swapchain's frame latency (2)";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut out: Option<String> = None;
    let mut pose: Option<Pose> = None;
    let mut radius: Option<f64> = None;
    let mut overview = false;
    let mut size = (1600u32, 1000u32);
    let mut damping = Damping::default();
    let mut frame_latency = 2u32;

    while let Some(arg) = args.next() {
        let mut number = |what: &str| -> f64 {
            args.next()
                .and_then(|text| text.parse().ok())
                .unwrap_or_else(|| panic!("three-rs heli: {what} takes a number\n{USAGE}"))
        };
        match arg.as_str() {
            "--smooth" => damping.smooth_time = number("--smooth"),
            "--drag-smooth" => damping.dragging_smooth_time = number("--drag-smooth"),
            "--wheel-smooth" => damping.wheel_smooth_time = number("--wheel-smooth"),
            "--dolly-step" => damping.dolly_step = number("--dolly-step"),
            "--frame-latency" => frame_latency = number("--frame-latency") as u32,
            "--headless" => {
                out = Some(args.next().unwrap_or_else(|| panic!("{USAGE}")));
            }
            "--pose" => {
                let text = args.next().unwrap_or_else(|| panic!("{USAGE}"));
                let numbers: Vec<f64> = text
                    .split(',')
                    .map(|field| {
                        field
                            .trim()
                            .parse()
                            .expect("three-rs heli: --pose is five numbers")
                    })
                    .collect();
                assert_eq!(numbers.len(), 5, "{USAGE}");
                pose = Some(Pose {
                    u: numbers[0],
                    v: numbers[1],
                    distance: numbers[2],
                    azimuth: numbers[3] * DEG2RAD,
                    polar: numbers[4] * DEG2RAD,
                });
            }
            "--radius" => {
                radius = Some(
                    args.next()
                        .and_then(|text| text.parse().ok())
                        .unwrap_or_else(|| panic!("{USAGE}")),
                );
            }
            "--overview" => overview = true,
            "--size" => {
                let text = args.next().unwrap_or_else(|| panic!("{USAGE}"));
                let (width, height) = text.split_once('x').unwrap_or_else(|| panic!("{USAGE}"));
                size = (
                    width.parse().expect("three-rs heli: --size is WxH"),
                    height.parse().expect("three-rs heli: --size is WxH"),
                );
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return;
            }
            other => panic!("three-rs heli: unknown argument {other}\n{USAGE}"),
        }
    }

    if let Some(out) = out {
        headless(&out, size, pose, radius, overview);
        return;
    }

    let event_loop = EventLoop::new().expect("three-rs heli: cannot create an event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run_app(&mut Heli::new(size, damping, frame_latency))
        .expect("three-rs heli: the event loop failed");
}
