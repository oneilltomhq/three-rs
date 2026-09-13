//! An interactive viewer for the ported examples: a winit window, a wgpu
//! surface on the renderer's own adapter, orbit controls and real time.
//!
//! ```text
//! cargo run --release --bin viewer -- webgpu_depth_texture
//! cargo run --release --bin viewer -- webgpu_instance_mesh
//! cargo run --release --bin viewer -- webgpu_materials_basic
//! ```
//!
//! Left-drag orbits, right-drag (or middle-drag) pans, the wheel zooms, 1/2/3
//! switch examples and Esc quits.
//!
//! The scenes themselves are *not* reimplemented here: the graded examples are
//! included as modules (exactly as `tests/e2e/main.rs` does) and their `init()`
//! builds the scene. Only the per-frame `animate()` math is restated, because
//! the graded copies pin `Date.now()` to 0 — here it runs on the wall clock.
//! Nothing in this file is reachable from the e2e harness.

use std::sync::Arc;
use std::time::Instant;

use three_rs::nodes::tsl;
use three_rs::{DepthTexture, MeshBasicNodeMaterial, Object3D,
    PerspectiveCamera, QuadMesh, RenderTarget, Renderer, RendererParameters, TextureType,
    Vector3};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[path = "../../examples/webgpu_depth_texture.rs"]
#[allow(dead_code)]
mod webgpu_depth_texture;

#[path = "../../examples/webgpu_instance_mesh.rs"]
#[allow(dead_code)]
mod webgpu_instance_mesh;

#[path = "../../examples/webgpu_materials_basic.rs"]
#[allow(dead_code)]
mod webgpu_materials_basic;

// ---------------------------------------------------------------- the scenes

/// Which ported example is on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Which {
    DepthTexture,
    InstanceMesh,
    MaterialsBasic,
}

impl Which {
    fn parse(name: &str) -> Option<Self> {
        match name.trim_start_matches("webgpu_") {
            "depth_texture" | "depth" | "1" => Some(Self::DepthTexture),
            "instance_mesh" | "instance" | "2" => Some(Self::InstanceMesh),
            "materials_basic" | "materials" | "3" => Some(Self::MaterialsBasic),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::DepthTexture => "webgpu_depth_texture",
            Self::InstanceMesh => "webgpu_instance_mesh",
            Self::MaterialsBasic => "webgpu_materials_basic",
        }
    }
}

enum Scene {
    DepthTexture(webgpu_depth_texture::App),
    InstanceMesh(webgpu_instance_mesh::App),
    MaterialsBasic(webgpu_materials_basic::App),
}

impl Scene {
    /// Builds the example's scene through its own `init()`, on an instance the
    /// caller already created (so a surface can live on the same one).
    fn build(which: Which, instance: Option<wgpu::Instance>) -> Self {
        let mut scene = match which {
            Which::DepthTexture => Scene::DepthTexture(webgpu_depth_texture::init()),
            Which::InstanceMesh => Scene::InstanceMesh(webgpu_instance_mesh::init()),
            Which::MaterialsBasic => Scene::MaterialsBasic(webgpu_materials_basic::init()),
        };

        // `init()` is the graded example's code verbatim, so its `Renderer` is
        // on an instance of its own making, with no display handle and hence no
        // surface support. Swap in one on the viewer's instance; the scene holds
        // no GPU state yet (every buffer and texture is uploaded lazily on the
        // first render, and `set_size()` below rebuilds the FX render target).
        if let Some(instance) = instance {
            // The `RendererParameters` each example passes to `Renderer::new()`.
            let antialias = match which {
                Which::DepthTexture | Which::InstanceMesh => true,
                Which::MaterialsBasic => false,
            };
            *scene.renderer() = Renderer::with_instance(
                RendererParameters { antialias },
                instance,
            );
        }

        scene
    }

    fn renderer(&mut self) -> &mut Renderer {
        match self {
            Scene::DepthTexture(app) => &mut app.renderer,
            Scene::InstanceMesh(app) => &mut app.renderer,
            Scene::MaterialsBasic(app) => &mut app.renderer,
        }
    }

    fn camera(&mut self) -> &mut PerspectiveCamera {
        match self {
            Scene::DepthTexture(app) => &mut app.camera,
            Scene::InstanceMesh(app) => &mut app.camera,
            Scene::MaterialsBasic(app) => &mut app.camera,
        }
    }

    fn set_size(&mut self, width: u32, height: u32) {
        let (w, h) = (width.max(1) as f64, height.max(1) as f64);
        self.camera().aspect = w / h;
        self.camera().update_projection_matrix();
        self.renderer().set_size(w, h);

        if let Scene::DepthTexture(app) = self {
            // `RenderTarget::set_size()` drops the colour texture but cannot
            // resize an attached `DepthTexture`, so the FX chain is rebuilt the
            // way `init()` built it.
            let depth_texture = DepthTexture::new();
            depth_texture.set_type(TextureType::Float);
            let render_target = RenderTarget::new(width.max(1), height.max(1));
            render_target.set_depth_texture(depth_texture.clone());
            let mut material_fx = MeshBasicNodeMaterial::new();
            material_fx.color_node = Some(tsl::depth_texture(&depth_texture));
            app.render_target = render_target;
            app.quad = QuadMesh::new(material_fx);
        }
    }

    /// The example's `animate()` with `Date.now()` running, and without the
    /// camera moves the page does (the orbit controls own the camera here).
    ///
    /// `time` is the example's own clock (`Date.now() * 0.001`); `node_time` is
    /// `NodeFrame.time`, which the window feeds from the same wall clock but
    /// `--time` pins (see `PINNED_NODE_TIME`).
    fn animate(&mut self, time: f64, node_time: f64) {
        match self {
            Scene::DepthTexture(app) => {
                // Nothing in this scene moves; only the camera does.
                app.renderer.set_time(node_time);
                app.renderer
                    .set_render_target(Some(app.render_target.clone()));
                app.renderer.render(&mut app.scene, &mut app.camera);
                app.renderer.set_render_target(None);
                app.renderer.render_quad(&app.quad);
            }
            Scene::InstanceMesh(app) => {
                // `const time = Date.now() * 0.001;`
                app.renderer.set_time(node_time);
                // `const amount = … || 10;`
                const AMOUNT: usize = 10;

                for child in app.scene.children() {
                    if !child.borrow().is_instanced_mesh() {
                        continue;
                    }

                    let mut mesh = child.borrow_mut();

                    let rotation_z = mesh.rotation.z;
                    mesh.set_rotation((time / 4.0).sin(), (time / 2.0).sin(), rotation_z);

                    let mut dummy = Object3D::default();
                    let mut i = 0usize;
                    let offset = (AMOUNT as f64 - 1.0) / 2.0;

                    for x in 0..AMOUNT {
                        for y in 0..AMOUNT {
                            for z in 0..AMOUNT {
                                dummy.position.set(
                                    offset - x as f64,
                                    offset - y as f64,
                                    offset - z as f64,
                                );

                                let rotation_y = (x as f64 / 4.0 + time).sin()
                                    + (y as f64 / 4.0 + time).sin()
                                    + (z as f64 / 4.0 + time).sin();

                                dummy.set_rotation(dummy.rotation.x, rotation_y, rotation_y * 2.0);
                                dummy.update_matrix();
                                mesh.set_matrix_at(i, &dummy.matrix);
                                i += 1;
                            }
                        }
                    }
                }

                app.renderer.render(&mut app.scene, &mut app.camera);
            }
            Scene::MaterialsBasic(app) => {
                app.renderer.set_time(node_time);
                // `const timer = 0.0001 * Date.now();`
                let timer = 0.1 * time;

                for (i, sphere) in app.scene.children().iter().enumerate() {
                    let mut sphere = sphere.borrow_mut();
                    sphere.position.x = 5.0 * (timer + i as f64).cos();
                    sphere.position.y = 5.0 * (timer + i as f64 * 1.1).sin();
                }

                app.renderer.render(&mut app.scene, &mut app.camera);
            }
        }
    }
}

// ------------------------------------------------------------ orbit controls

/// A port of the spherical half of `examples/jsm/controls/OrbitControls.js`:
/// `rotateLeft`/`rotateUp` on a left drag, `dollyIn`/`dollyOut` on the wheel,
/// `panLeft`/`panUp` on a right drag, then `update()`'s
/// `position = target + spherical` plus `lookAt( target )`.
struct OrbitControls {
    target: Vector3,
    radius: f64,
    /// Polar angle from +Y, in radians.
    phi: f64,
    /// Azimuthal angle, in radians.
    theta: f64,
    min_distance: f64,
    max_distance: f64,
    zoom_speed: f64,
}

impl OrbitControls {
    fn new(camera: &PerspectiveCamera, target: Vector3) -> Self {
        let mut offset = Vector3::ZERO;
        offset.sub_vectors(&camera.node.borrow().position, &target);
        let radius = offset.length().max(1e-6);
        // `Spherical.setFromVector3()`.
        let theta = offset.x.atan2(offset.z);
        let phi = (offset.y / radius).clamp(-1.0, 1.0).acos();

        Self {
            target,
            radius,
            phi,
            theta,
            min_distance: 0.01,
            max_distance: 1000.0,
            zoom_speed: 1.0,
        }
    }

    fn rotate(&mut self, dx: f64, dy: f64, height: f64) {
        self.theta -= 2.0 * std::f64::consts::PI * dx / height;
        self.phi -= 2.0 * std::f64::consts::PI * dy / height;
        // `OrbitControls` clamps phi into ( EPS, PI - EPS ).
        const EPS: f64 = 1e-6;
        self.phi = self.phi.clamp(EPS, std::f64::consts::PI - EPS);
    }

    fn dolly(&mut self, steps: f64) {
        let scale = 0.95f64.powf(self.zoom_speed * steps);
        self.radius = (self.radius * scale).clamp(self.min_distance, self.max_distance);
    }

    /// `pan()` for a perspective camera: the drag is scaled by the distance to
    /// the target at the camera's field of view.
    fn pan(&mut self, dx: f64, dy: f64, camera: &PerspectiveCamera, height: f64) {
        let fov = camera.fov * std::f64::consts::PI / 180.0;
        let target_distance = self.radius * (fov / 2.0).tan();
        let object = camera.node.borrow();
        let m = &object.matrix_world;
        let e = m.elements;
        // `panLeft`: matrix column 0; `panUp`: column 1.
        let mut left = Vector3::new(e[0], e[1], e[2]);
        let mut up = Vector3::new(e[4], e[5], e[6]);
        left.multiply_scalar(-2.0 * dx * target_distance / height);
        up.multiply_scalar(2.0 * dy * target_distance / height);
        self.target.add(&left);
        self.target.add(&up);
    }

    fn apply(&self, camera: &mut PerspectiveCamera) {
        let sin_phi = self.phi.sin();
        camera.node.borrow_mut().position.set(
            self.target.x + self.radius * sin_phi * self.theta.sin(),
            self.target.y + self.radius * self.phi.cos(),
            self.target.z + self.radius * sin_phi * self.theta.cos(),
        );
        camera.look_at(&self.target);
    }
}

// ------------------------------------------------------------------ the app

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

struct Viewer {
    which: Which,
    instance: Option<wgpu::Instance>,
    scene: Option<Scene>,
    controls: Option<OrbitControls>,
    gpu: Option<Gpu>,

    requested_size: (u32, u32),
    start: Instant,
    frames: u32,
    last_report: Instant,

    cursor: (f64, f64),
    dragging: Option<MouseButton>,

    autoswitch: Option<f64>,
    last_switch: Instant,
}

impl Viewer {
    fn configure_surface(&mut self) {
        let Some(gpu) = &mut self.gpu else { return };
        let scene = self.scene.as_mut().unwrap();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![gpu.format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: gpu.size.0.max(1),
            height: gpu.size.1.max(1),
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        gpu.surface.configure(scene.renderer().device(), &config);
    }

    fn switch(&mut self, which: Which) {
        if which == self.which {
            return;
        }
        println!("switching to {}", which.name());
        self.which = which;
        // Drop the old scene (and its device) before the new one is built.
        self.scene = None;
        self.controls = None;
        let instance = self.instance.clone();
        let mut scene = Scene::build(which, instance);
        let size = self.gpu.as_ref().map(|g| g.size).unwrap_or(self.requested_size);
        scene.set_size(size.0, size.1);
        self.controls = Some(OrbitControls::new(scene.camera(), Vector3::ZERO));
        self.scene = Some(scene);
        self.configure_surface();
        println!("{}", self.adapter_line());
    }

    fn adapter_line(&mut self) -> String {
        let info = self.scene.as_mut().unwrap().renderer().adapter_info();
        format!(
            "adapter: {} ({:?}, {:?}) — {}",
            info.name, info.device_type, info.backend, info.driver
        )
    }

    fn redraw(&mut self) {
        let Some(scene) = self.scene.as_mut() else { return };
        let time = self.start.elapsed().as_secs_f64();

        if let Some(controls) = &self.controls {
            controls.apply(scene.camera());
        }

        let t0 = Instant::now();
        // The window runs on the wall clock, so `NodeFrame.time` tracks it.
        scene.animate(time, time);
        let t_animate = t0.elapsed();
        let t1 = Instant::now();

        if let Some(gpu) = &mut self.gpu {
            let surface_texture = match gpu.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(texture) => texture,
                wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                    drop(texture);
                    self.configure_surface();
                    return;
                }
                wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout => {
                    return
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.configure_surface();
                    return;
                }
                wgpu::CurrentSurfaceTexture::Lost => {
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
                    panic!("three-rs viewer: surface validation error")
                }
            };

            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    format: Some(gpu.format),
                    ..Default::default()
                });

            // The canvas holds sRGB-encoded bytes, so `gpu.format` is the
            // non-srgb view of the surface and the blit writes them verbatim.
            let presented = scene.renderer().present(&view, gpu.format);
            assert!(presented, "three-rs viewer: nothing to present");

            gpu.window.pre_present_notify();
            scene.renderer().queue().present(surface_texture);
        }

        if std::env::var("VIEWER_TRACE").is_ok() {
            eprintln!(
                "frame {}: animate {:?} present {:?}",
                self.frames,
                t_animate,
                t1.elapsed()
            );
        }

        self.frames += 1;
        if self.last_report.elapsed().as_secs_f64() >= 1.0 {
            let fps = self.frames as f64 / self.last_report.elapsed().as_secs_f64();
            let (w, h) = self.gpu.as_ref().map(|g| g.size).unwrap_or((0, 0));
            println!("{} — {w}x{h} — {fps:.1} fps", self.which.name());
            self.frames = 0;
            self.last_report = Instant::now();
        }
    }
}

impl ApplicationHandler for Viewer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(format!("three-rs — {}", self.which.name()))
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.requested_size.0,
                            self.requested_size.1,
                        )),
                )
                .expect("three-rs viewer: cannot create a window"),
        );

        // A Vulkan surface needs the instance to carry the display handle, so
        // the instance is created here and handed to the renderer.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                event_loop.owned_display_handle(),
            ))
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("three-rs viewer: cannot create a surface");

        let mut scene = Scene::build(self.which, Some(instance.clone()));

        let caps = surface.get_capabilities(scene.renderer().adapter());
        let preferred = caps.formats[0];
        // Present to a NON-srgb format: the canvas texture already holds
        // sRGB-encoded bytes, and an `-srgb` view would encode them twice.
        let linear = preferred.remove_srgb_suffix();
        let format = if caps.formats.contains(&linear) {
            linear
        } else {
            preferred
        };
        println!("surface formats: {:?} — using {format:?}", caps.formats);

        let size = window.inner_size();
        let size = (size.width.max(1), size.height.max(1));
        scene.set_size(size.0, size.1);
        self.controls = Some(OrbitControls::new(scene.camera(), Vector3::ZERO));

        self.instance = Some(instance);
        self.scene = Some(scene);
        self.gpu = Some(Gpu {
            window: window.clone(),
            surface,
            format,
            size,
        });
        self.requested_size = size;
        self.configure_surface();

        println!("{}", self.adapter_line());
        println!("controls: left-drag orbit, right-drag pan, wheel zoom, 1/2/3 switch, Esc quit");

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character("1") => self.switch(Which::DepthTexture),
                    Key::Character("2") => self.switch(Which::InstanceMesh),
                    Key::Character("3") => self.switch(Which::MaterialsBasic),
                    _ => {}
                }
            }

            WindowEvent::Resized(size) => {
                let size = (size.width.max(1), size.height.max(1));
                if let Some(gpu) = &mut self.gpu {
                    gpu.size = size;
                }
                if let Some(scene) = self.scene.as_mut() {
                    scene.set_size(size.0, size.1);
                }
                self.configure_surface();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.dragging = (state == ElementState::Pressed).then_some(button);
            }

            WindowEvent::CursorMoved { position, .. } => {
                let last = self.cursor;
                self.cursor = (position.x, position.y);
                let (dx, dy) = (self.cursor.0 - last.0, self.cursor.1 - last.1);
                let height = self.gpu.as_ref().map(|g| g.size.1).unwrap_or(1).max(1) as f64;

                let (Some(controls), Some(scene)) = (&mut self.controls, self.scene.as_mut()) else {
                    return;
                };

                match self.dragging {
                    Some(MouseButton::Left) => controls.rotate(dx, dy, height),
                    Some(MouseButton::Right) | Some(MouseButton::Middle) => {
                        controls.pan(dx, dy, scene.camera(), height)
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(controls) = &mut self.controls {
                    let steps = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f64,
                        MouseScrollDelta::PixelDelta(p) => p.y / 100.0,
                    };
                    controls.dolly(steps);
                }
            }

            WindowEvent::RedrawRequested => {
                // `VIEWER_AUTOSWITCH=<seconds>` cycles the examples on a timer,
                // which is how the 1/2/3 path gets exercised from a script.
                if let Some(period) = self.autoswitch {
                    if self.last_switch.elapsed().as_secs_f64() >= period {
                        self.last_switch = Instant::now();
                        self.switch(match self.which {
                            Which::DepthTexture => Which::InstanceMesh,
                            Which::InstanceMesh => Which::MaterialsBasic,
                            Which::MaterialsBasic => Which::DepthTexture,
                        });
                    }
                }
                self.redraw();
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

// ------------------------------------------------------------- screenshot mode

/// Renders `frames` frames headless and writes the canvas as a PNG, plus the
/// result of `Renderer::present()` into an off-screen `bgra8unorm` texture so
/// the blit the window uses is covered too.
/// `NodeFrame.time` when the page's clock is pinned.
///
/// `NodeFrame.update()` (three.js/src/nodes/core/NodeFrame.js) does:
///
/// ```js
/// if ( this.lastTime === undefined ) this.lastTime = performance.now();
/// this.deltaTime = ( performance.now() - this.lastTime ) / 1000;
/// this.lastTime = performance.now();
/// this.time += this.deltaTime;
/// ```
///
/// `time` starts at 0 and only ever accumulates deltas. On the first frame
/// `lastTime` is set to `performance.now()` immediately before it is read, so
/// `deltaTime` is 0 whatever `performance.now()` returns — and with the clock
/// pinned every later frame has a 0 delta too. So a pinned `performance.now()`
/// of `T * 1000` leaves `NodeFrame.time` at **0**, for any `T`, on every frame.
/// This is also why the e2e harness' single frame sees `time === 0`.
const PINNED_NODE_TIME: f64 = 0.0;

/// Puts a brand-new `Renderer` under an already-built scene, which resets both
/// `NodeFrame.time` and the deterministic `Math.random` the port draws from.
fn fresh_renderer(scene: &mut Scene, which: Which, size: (u32, u32)) {
    // The `RendererParameters` each example passes to `Renderer::new()`.
    let antialias = match which {
        Which::DepthTexture | Which::InstanceMesh => true,
        Which::MaterialsBasic => false,
    };
    *scene.renderer() = Renderer::new(RendererParameters { antialias });
    scene.set_size(size.0, size.1);
}

fn screenshot(
    which: Which,
    size: (u32, u32),
    frames: u32,
    path: &str,
    orbit: (f64, f64),
    zoom: f64,
    pan: (f64, f64),
    pinned: &[(String, f64)],
    // `series` is `--times`: one file per time, named `<stem>_<t>.png`.
    series: bool,
) {
    let mut scene = Scene::build(which, None);
    println!(
        "adapter: {:?}",
        scene.renderer().adapter_info().name.clone()
    );
    scene.set_size(size.0, size.1);

    // The same controls the window drives, so `--orbit`/`--zoom` render exactly
    // what a drag would put on screen.
    let mut controls = OrbitControls::new(scene.camera(), Vector3::ZERO);
    controls.rotate(orbit.0, orbit.1, size.1 as f64);
    controls.dolly(zoom);
    if pan != (0.0, 0.0) {
        // `pan()` reads the camera's world matrix, which `render()` normally
        // leaves behind from the previous frame.
        controls.apply(scene.camera());
        scene.camera().update_matrix_world();
        controls.pan(pan.0, pan.1, scene.camera(), size.1 as f64);
    }

    if !pinned.is_empty() {
        // `--time` / `--times`: the page's clock stands still at T, which is
        // `Date.now() === performance.now() === T * 1000`. The example's own
        // `animate()` maths gets T; `NodeFrame.time` gets `PINNED_NODE_TIME`.
        for (index, (label, time)) in pinned.iter().enumerate() {
            if index > 0 {
                // A fresh renderer per time: `range()`'s draws from the
                // deterministic `Math.random` are consumed per render in this
                // port, so a second render in the same renderer would see a
                // different random sequence than three.js' first frame does.
                fresh_renderer(&mut scene, which, size);
            }

            for _ in 0..frames.max(1) {
                controls.apply(scene.camera());
                scene.animate(*time, PINNED_NODE_TIME);
            }

            let (width, height, pixels) = scene.renderer().read_canvas_pixels();
            let out = if !series {
                path.to_string()
            } else {
                format!("{}_{label}.png", path.trim_end_matches(".png"))
            };
            three_rs::testing::write_png(&out, width, height, &pixels);
            println!(
                "wrote {out} ({width}x{height}) at pinned t={time}s (Date.now() = {}ms, NodeFrame.time = {PINNED_NODE_TIME})",
                time * 1000.0
            );
        }

        return;
    }

    // Frame n is drawn at t = n / 60 s, so the animation is exercised without
    // depending on how fast this machine renders.
    for frame in 0..frames.max(1) {
        let t = frame as f64 / 60.0;
        controls.apply(scene.camera());
        scene.animate(t, t);
    }

    let (width, height, pixels) = scene.renderer().read_canvas_pixels();
    three_rs::testing::write_png(path, width, height, &pixels);
    println!("wrote {path} ({width}x{height}) after {frames} frame(s)");

    // And the present path, into the format a Wayland surface hands out.
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let device = scene.renderer().device().clone();
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("viewer present check"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&Default::default());
    assert!(scene.renderer().present(&view, format));

    let bytes_per_row = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("viewer present readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    scene.renderer().queue().submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let padded = slice.get_mapped_range().unwrap().to_vec();
    buffer.unmap();

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * bytes_per_row) as usize;
        for px in 0..width as usize {
            let p = start + px * 4;
            rgba.extend_from_slice(&[padded[p + 2], padded[p + 1], padded[p], padded[p + 3]]);
        }
    }

    let present_path = path.replace(".png", "") + ".present.png";
    three_rs::testing::write_png(&present_path, width, height, &rgba);
    println!("wrote {present_path} (the blit Renderer::present() does)");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut which = Which::DepthTexture;
    let mut size = (800u32, 500u32);
    let mut frames = 1u32;
    let mut shot: Option<String> = None;
    let mut orbit = (0.0f64, 0.0f64);
    let mut zoom = 0.0f64;
    let mut pan = (0.0f64, 0.0f64);
    // `--time` / `--times`: the label is kept as typed, for the file name.
    let mut pinned: Vec<(String, f64)> = Vec::new();
    let mut series = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                i += 1;
                shot = Some(args[i].clone());
            }
            "--orbit" => {
                i += 1;
                orbit.0 = args[i].parse().expect("--orbit takes two numbers");
                i += 1;
                orbit.1 = args[i].parse().expect("--orbit takes two numbers");
            }
            "--time" => {
                i += 1;
                let label = args[i].clone();
                let time = label.parse().expect("--time takes a number of seconds");
                pinned = vec![(label, time)];
                series = false;
            }
            "--times" => {
                i += 1;
                series = true;
                pinned = args[i]
                    .split(',')
                    .map(|part| {
                        let label = part.trim().to_string();
                        let time = label
                            .parse()
                            .expect("--times takes a comma-separated list of seconds");
                        (label, time)
                    })
                    .collect();
            }
            "--pan" => {
                i += 1;
                pan.0 = args[i].parse().expect("--pan takes two numbers");
                i += 1;
                pan.1 = args[i].parse().expect("--pan takes two numbers");
            }
            "--zoom" => {
                i += 1;
                zoom = args[i].parse().expect("--zoom takes a number");
            }
            "--frames" => {
                i += 1;
                frames = args[i].parse().expect("--frames takes a number");
            }
            "--width" => {
                i += 1;
                size.0 = args[i].parse().expect("--width takes a number");
            }
            "--height" => {
                i += 1;
                size.1 = args[i].parse().expect("--height takes a number");
            }
            other => match Which::parse(other) {
                Some(w) => which = w,
                None => {
                    eprintln!(
                        "usage: viewer <webgpu_depth_texture|webgpu_instance_mesh|\
                         webgpu_materials_basic> [--screenshot out.png [--frames N]] \
                         [--width W] [--height H] [--orbit DX DY] [--zoom STEPS] \
                         [--pan DX DY] [--time T | --times T1,T2,...]"
                    );
                    std::process::exit(2);
                }
            },
        }
        i += 1;
    }

    if let Some(path) = shot {
        screenshot(which, size, frames, &path, orbit, zoom, pan, &pinned, series);
        return;
    }

    let event_loop = EventLoop::new().expect("three-rs viewer: no event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let now = Instant::now();
    let mut viewer = Viewer {
        which,
        instance: None,
        scene: None,
        controls: None,
        gpu: None,
        requested_size: size,
        start: now,
        frames: 0,
        last_report: now,
        cursor: (0.0, 0.0),
        dragging: None,
        autoswitch: std::env::var("VIEWER_AUTOSWITCH")
            .ok()
            .and_then(|v| v.parse().ok()),
        last_switch: now,
    };

    event_loop
        .run_app(&mut viewer)
        .expect("three-rs viewer: event loop failed");
}
