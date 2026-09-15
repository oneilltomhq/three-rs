//! The demo for [`three_rs::controls`]: fly a helicopter over a ground that
//! can be flat or a small planet, with a wall of panes standing on it.
//!
//! ```text
//! cargo run --release --bin heli
//! cargo run --release --bin heli -- --headless shots/heli-plane-low.png
//! cargo run --release --bin heli -- --headless shots/x.png --radius 300 --overview
//! ```
//!
//! `W` `A` `S` `D` fly, `Q` / `E` and the wheel climb and descend, either mouse
//! button drags the view round, `[` and `]` curl the ground up and flatten it
//! again, `P` snaps it flat, `Home` returns to the start pose, `Tab` is the
//! overview and `Esc` quits. In the overview a click picks a pane and drops
//! onto it.
//!
//! The window, the surface, the event loop and the headless path are lifted
//! from `src/bin/viewer.rs`; the scene and the controls are this file's own.
//! Nothing here is reachable from the e2e harness.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use three_rs::controls::{Ground, Helicopter, Mode, Pane, Pose};
use three_rs::core::{BufferAttribute, BufferGeometry};
use three_rs::materials::Side;
use three_rs::math::math_utils::{DEG2RAD, RAD2DEG};
use three_rs::{
    plane_geometry, Color, LineSegments, Matrix4, Mesh, MeshBasicNodeMaterial, Node,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
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
        v: -120.0,
        altitude: 40.0,
        yaw: 0.0,
        pitch: -15.0 * DEG2RAD,
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
    heli: Helicopter,
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
        let heli = Helicopter::new(ground, start_pose());
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
            heli,
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
        let radius = self.heli.ground().radius();
        if (radius - self.grid_radius).abs() > 1e-6 * self.grid_radius {
            let positions = grid_positions(self.heli.ground());
            self.grid
                .borrow()
                .line()
                .expect("the grid is a LineSegments")
                .set_positions(&positions);
            self.grid_radius = radius;
        }

        for (pane, node) in self.panes.iter().zip(&self.pane_nodes) {
            let frame = self.heli.ground().frame(pane.u, pane.v);
            let mut position = frame.origin;
            position.add_scaled_vector(&frame.normal, pane.height * 0.5);

            // Local `+Y` is the ground normal and the front faces `-north`, so
            // local `+Z` is `-north` and local `+X` is `normal × -north`.
            let mut back = frame.north;
            back.negate();
            let side = frame.normal.crossed(&back);
            let mut basis = Matrix4::identity();
            basis.make_basis(&side, &frame.normal, &back);

            let mut object = node.borrow_mut();
            object.position = position;
            object.set_rotation_from_matrix(&basis);
        }

        self.heli.apply(&mut self.camera);
    }

    fn render(&mut self) {
        self.sync();
        self.renderer.render(&mut self.scene, &mut self.camera);
    }

    /// The window title, which is also what documents a screenshot.
    fn title(&self) -> String {
        let pose = self.heli.current();
        format!(
            "heli — {} — R {:.0} — u {:.0} v {:.0} alt {:.0} — yaw {:.0}° pitch {:.0}°{}",
            match self.heli.mode() {
                Mode::Free => "free",
                Mode::Overview => "overview",
            },
            self.heli.ground().radius(),
            pose.u,
            pose.v,
            pose.altitude,
            pose.yaw * RAD2DEG,
            pose.pitch * RAD2DEG,
            if self.heli.rested() { " — rest" } else { "" },
        )
    }
}

// ------------------------------------------------------------------ the window

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
}

/// A press, so a release can tell a click from a drag.
struct Press {
    button: MouseButton,
    at: (f64, f64),
    moved: f64,
}

struct Heli {
    instance: Option<wgpu::Instance>,
    app: Option<App>,
    gpu: Option<Gpu>,
    requested_size: (u32, u32),

    held: HashSet<char>,
    cursor: (f64, f64),
    press: Option<Press>,
    last_frame: Instant,
    /// Set by a resize, and by the first frame, so the redraw is not skipped
    /// just because nothing was damped.
    dirty: bool,
}

impl Heli {
    fn new(size: (u32, u32)) -> Self {
        Self {
            instance: None,
            app: None,
            gpu: None,
            requested_size: size,
            held: HashSet::new(),
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
            desired_maximum_frame_latency: 2,
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
        if app.heli.mode() != Mode::Overview {
            return;
        }

        let (width, height) = (app.size.0 as f64, app.size.1 as f64);
        let ndc_x = at.0 / width * 2.0 - 1.0;
        let ndc_y = 1.0 - at.1 / height * 2.0;

        if let Some(index) = app.heli.pick(&app.panes, ndc_x, ndc_y, &app.camera) {
            let pane = app.panes[index];
            app.heli.focus_pane(&pane);
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
        let app = App::build(Some(instance.clone()), size);

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

        println!(
            "heli — W A S D fly, Q/E and the wheel climb, drag to look, \
             [ ] curl the ground, P flattens it, Home resets, Tab is the overview, Esc quits"
        );

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
                        app.heli.toggle_overview(&panes, FOV, aspect);
                    }
                    Key::Named(NamedKey::Home) if pressed => app.heli.reset(),
                    Key::Character(text) => {
                        for character in text.chars().flat_map(char::to_lowercase) {
                            match character {
                                'w' | 'a' | 's' | 'd' | 'q' | 'e' => {
                                    if pressed {
                                        self.held.insert(character);
                                    } else {
                                        self.held.remove(&character);
                                    }
                                }
                                '[' if pressed => app.heli.scale_radius(1.25),
                                ']' if pressed => app.heli.scale_radius(1.0 / 1.25),
                                'p' if pressed => app.heli.set_radius(1e7),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }

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
                    self.press = Some(Press {
                        button,
                        at: self.cursor,
                        moved: 0.0,
                    });
                    if let Some(app) = self.app.as_mut() {
                        app.heli.set_dragging(true);
                    }
                }
                ElementState::Released => {
                    if let Some(app) = self.app.as_mut() {
                        app.heli.set_dragging(false);
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

                let dragging = matches!(
                    self.press.as_ref().map(|press| press.button),
                    Some(MouseButton::Left) | Some(MouseButton::Right)
                );
                if dragging {
                    if let Some(app) = self.app.as_mut() {
                        app.heli.look(dx, dy, height);
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 100.0,
                };
                if let Some(app) = self.app.as_mut() {
                    app.heli.climb(steps);
                }
            }

            WindowEvent::RedrawRequested => self.redraw(),

            _ => {}
        }
    }

    /// The held keys are applied here, with the real elapsed time, so flying is
    /// frame-rate independent; and the frame is only drawn when the controller
    /// says something moved.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let dt = self.last_frame.elapsed().as_secs_f64().min(0.1);
        self.last_frame = Instant::now();

        let Some(app) = self.app.as_mut() else { return };

        let axis = |positive: char, negative: char| {
            f64::from(self.held.contains(&positive)) - f64::from(self.held.contains(&negative))
        };
        let forward = axis('w', 's');
        let right = axis('d', 'a');
        let climb = axis('e', 'q');

        app.heli.move_ground(forward, right, dt);
        if climb != 0.0 {
            // One notch a second while the key is down.
            app.heli.climb(climb * dt * 8.0);
        }

        let moved = app.heli.update(dt);
        if moved || self.dirty {
            self.dirty = false;
            if let Some(gpu) = &self.gpu {
                gpu.window.request_redraw();
            }
        }
    }
}

// ---------------------------------------------------------------- headless

/// One settled frame, written as a PNG. Nothing is damped: the controller is
/// put on its target pose and rendered once.
fn headless(path: &str, size: (u32, u32), pose: Option<Pose>, radius: Option<f64>, overview: bool) {
    let mut app = App::build(None, size);

    if let Some(radius) = radius {
        app.heli.set_radius(radius);
    }
    if let Some(pose) = pose {
        app.heli = Helicopter::new(Ground::new(app.heli.target_radius()), pose);
    }
    app.heli.settle();

    if overview {
        let aspect = app.aspect();
        let panes = app.panes.clone();
        app.heli.toggle_overview(&panes, FOV, aspect);
        app.heli.settle();
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

const USAGE: &str = "usage: heli [--headless out.png] [--pose u,v,h,yaw_deg,pitch_deg] \
                     [--radius R] [--overview] [--size WxH]";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut out: Option<String> = None;
    let mut pose: Option<Pose> = None;
    let mut radius: Option<f64> = None;
    let mut overview = false;
    let mut size = (1600u32, 1000u32);

    while let Some(arg) = args.next() {
        match arg.as_str() {
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
                    altitude: numbers[2],
                    yaw: numbers[3] * DEG2RAD,
                    pitch: numbers[4] * DEG2RAD,
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
        .run_app(&mut Heli::new(size))
        .expect("three-rs heli: the event loop failed");
}
