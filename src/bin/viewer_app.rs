//! An interactive viewer for the ported examples: a winit window, a wgpu
//! surface on the renderer's own adapter, the examples' own `OrbitControls`
//! and real time.
//!
//! ```text
//! cargo run --release --bin viewer -- --list
//! cargo run --release --bin viewer -- webgpu_lights_physical
//! cargo run --release --bin viewer -- lights_phong --headless --frames 60
//! ```
//!
//! Every example the e2e grader runs is here, in the README's order, named on
//! the command line with or without its `webgpu_` prefix or by its index in
//! that table; `--list` prints it. `[` and `]` step to the previous and next
//! example, and Esc quits. Everything else the pointer and the keyboard do is
//! the example's own [`OrbitControls`]: left-drag orbits, right-drag pans, the
//! wheel dollies and the arrow keys pan — on the examples whose page creates
//! controls, and on no others, because the page cannot be dragged either.
//!
//! # What this file is not
//!
//! It is not a second copy of the examples. Each one is included as a module
//! (exactly as `tests/e2e/main.rs` and `web/src/shell.rs` do) and the viewer
//! only ever calls the four functions every example exposes: `init()`,
//! `animate()`, `resize()` and `controls()`. It knows no example's scene
//! graph, no example's clock and no example's camera target. Frames come from
//! [`three_rs::utils::now_ms()`], which is the same clock the browser shell
//! and the pages run on, so the window needs to pin nothing and restate
//! nothing.
//!
//! # Frame time
//!
//! The window prints one line a second to stdout:
//!
//! ```text
//! webgpu_lights_phong — 1000x625 — 59.9 fps — render mean 1.61 ms max 1.79 ms (last 60 frames, after 10 warm-up)
//! ```
//!
//! `render` is the CPU side of a frame — the example's `animate()` from its
//! first line to the command buffer being submitted — which is where a
//! per-frame regression on the CPU (the 250 ms cache-key bug, issue #55) shows
//! up. Under vsync the presented frame time is pinned to the display, so it is
//! not what is reported. `--headless --frames N` renders N frames with no
//! window and no vsync, waits for the GPU after each, and reports the same
//! mean/max over the *whole* frame, CPU and GPU; that is what the e2e
//! harness's steady-frame ceiling is compared against, and what the README's
//! steady-frame column is measured with.
//!
//! The mean and max are over the last [`FrameTimer::WINDOW`] frames once
//! [`FrameTimer::WARMUP`] frames have gone by, so the first frame's program
//! builds and uploads never count.

use std::sync::Arc;
use std::time::{Duration, Instant};

use three_rs::addons::controls::{
    Key as OrbitKey, KeyEvent, MouseButton as OrbitButton, OrbitControls, PointerEvent, WheelDelta,
    WheelEvent,
};
use three_rs::{PerspectiveCamera, Renderer};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

// The SDF text examples are not here: they live in `sdf-text/examples/`,
// because that crate depends on three-rs and three-rs cannot depend back on
// it (`Cargo.toml` explains the publish cycle).

// ---------------------------------------------------------------- the scenes

/// Declares the viewer's whole knowledge of the examples: one line per
/// example, giving its enum name, its module and the file to include.
///
/// Everything else — the [`Which`] table, the [`Example`] enum and every
/// dispatch on it — is generated from that one list, so adding an example to
/// the viewer is adding a line and nothing else. Before the examples grew
/// `resize()` and `controls()` this could not be done: the viewer restated
/// each example's `animate()` and carried a table of their camera targets and
/// renderer options, and each of those was a place to get an example wrong.
macro_rules! examples {
    ( $( $variant:ident , $module:ident , $path:literal ; )* ) => {
        $(
            #[path = $path]
            #[allow(dead_code)] // each module's `main()` is unused here
            mod $module;
        )*

        /// Which ported example is on screen — the README's table, in its
        /// order.
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum Which {
            $( $variant, )*
        }

        /// The built example. One variant per module, holding that module's
        /// own `App`.
        #[allow(clippy::large_enum_variant)] // internal to the viewer binary; not worth an indirection for a debug tool
        enum Example {
            $( $variant($module::App), )*
        }

        impl Which {
            /// Every graded example, in README order.
            const ALL: [Which; [$( stringify!($variant) ),*].len()] = [
                $( Self::$variant, )*
            ];

            fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => stringify!($module), )*
                }
            }
        }

        impl Example {
            /// Builds the example's scene through its own `init()`.
            fn build_raw(which: Which) -> Self {
                match which {
                    $( Which::$variant => Example::$variant($module::init()), )*
                }
            }

            fn renderer(&mut self) -> &mut Renderer {
                match self {
                    $( Example::$variant(app) => &mut app.renderer, )*
                }
            }

            /// The example's own `animate()`, unchanged and unassisted. Every
            /// clock it reads is the real one.
            fn animate(&mut self) {
                match self {
                    $( Example::$variant(app) => $module::animate(app), )*
                }
            }

            /// The example's own `resize()` — its page's `onWindowResize()`.
            fn resize_example(&mut self, width: f64, height: f64) {
                match self {
                    $( Example::$variant(app) => $module::resize(app, width, height), )*
                }
            }

            /// The example's own `OrbitControls`, if its page creates any.
            fn controls(&mut self) -> Option<&mut OrbitControls> {
                match self {
                    $( Example::$variant(app) => $module::controls(app), )*
                }
            }

            /// The controls and the camera at once, which every event handler
            /// needs: the JS holds the camera as `this.object` and Rust cannot,
            /// so `pointer_move` and friends take it as an argument.
            ///
            /// They are two fields of one `App` and borrowing both is
            /// perfectly sound, but only the example can say so — a host
            /// holding `&mut App` and calling two accessors cannot. Hence a
            /// second uniform function on every example rather than a cast
            /// here.
            fn controls_and_camera(
                &mut self,
            ) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
                match self {
                    $( Example::$variant(app) => $module::controls_and_camera(app), )*
                }
            }
        }
    };
}

examples! {
DepthTexture, webgpu_depth_texture, "../../examples/webgpu_depth_texture.rs";
InstanceMesh, webgpu_instance_mesh, "../../examples/webgpu_instance_mesh.rs";
MaterialsBasic, webgpu_materials_basic, "../../examples/webgpu_materials_basic.rs";
Rtt, webgpu_rtt, "../../examples/webgpu_rtt.rs";
LightsPhong, webgpu_lights_phong, "../../examples/webgpu_lights_phong.rs";
Morphtargets, webgpu_morphtargets, "../../examples/webgpu_morphtargets.rs";
Shadowmap, webgpu_shadowmap, "../../examples/webgpu_shadowmap.rs";
LightsPhysical, webgpu_lights_physical, "../../examples/webgpu_lights_physical.rs";
PostprocessingMasking, webgpu_postprocessing_masking, "../../examples/webgpu_postprocessing_masking.rs";
TslGalaxy, webgpu_tsl_galaxy, "../../examples/webgpu_tsl_galaxy.rs";
Skinning, webgpu_skinning, "../../examples/webgpu_skinning.rs";
MeshBatch, webgpu_mesh_batch, "../../examples/webgpu_mesh_batch.rs";
PostprocessingRadialBlur, webgpu_postprocessing_radial_blur, "../../examples/webgpu_postprocessing_radial_blur.rs";
Materials, webgpu_materials, "../../examples/webgpu_materials.rs";
PostprocessingSsaa, webgpu_postprocessing_ssaa, "../../examples/webgpu_postprocessing_ssaa.rs";
PmremCubemap, webgpu_pmrem_cubemap, "../../examples/webgpu_pmrem_cubemap.rs";
PostprocessingBloomSelective, webgpu_postprocessing_bloom_selective, "../../examples/webgpu_postprocessing_bloom_selective.rs";
ComputePoints, webgpu_compute_points, "../../examples/webgpu_compute_points.rs";
LinesFat, webgpu_lines_fat, "../../examples/webgpu_lines_fat.rs";
PmremTest, webgpu_pmrem_test, "../../examples/webgpu_pmrem_test.rs";
PostprocessingDifference, webgpu_postprocessing_difference, "../../examples/webgpu_postprocessing_difference.rs";
PostprocessingDirect, webgpu_postprocessing_direct, "../../examples/webgpu_postprocessing_direct.rs";
FurnaceTest, webgpu_furnace_test, "../../examples/webgpu_furnace_test.rs";
PostprocessingAnamorphic, webgpu_postprocessing_anamorphic, "../../examples/webgpu_postprocessing_anamorphic.rs";
PmremScene, webgpu_pmrem_scene, "../../examples/webgpu_pmrem_scene.rs";
PostprocessingBloom, webgpu_postprocessing_bloom, "../../examples/webgpu_postprocessing_bloom.rs";
MaterialsEnvmaps, webgpu_materials_envmaps, "../../examples/webgpu_materials_envmaps.rs";
MaterialsCubemapMipmaps, webgpu_materials_cubemap_mipmaps, "../../examples/webgpu_materials_cubemap_mipmaps.rs";
PostprocessingBloomEmissive, webgpu_postprocessing_bloom_emissive, "../../examples/webgpu_postprocessing_bloom_emissive.rs";
InstanceUniform, webgpu_instance_uniform, "../../examples/webgpu_instance_uniform.rs";
TslInteroperability, webgpu_tsl_interoperability, "../../examples/webgpu_tsl_interoperability.rs";
PmremEquirectangular, webgpu_pmrem_equirectangular, "../../examples/webgpu_pmrem_equirectangular.rs";
PostprocessingCa, webgpu_postprocessing_ca, "../../examples/webgpu_postprocessing_ca.rs";
LoaderGltf, webgpu_loader_gltf, "../../examples/webgpu_loader_gltf.rs";
Mrt, webgpu_mrt, "../../examples/webgpu_mrt.rs";
CustomFogBackground, webgpu_custom_fog_background, "../../examples/webgpu_custom_fog_background.rs";
LoaderGltfSheen, webgpu_loader_gltf_sheen, "../../examples/webgpu_loader_gltf_sheen.rs";
Deferred, webgpu_deferred, "../../examples/webgpu_deferred.rs";
LoaderGltfAnisotropy, webgpu_loader_gltf_anisotropy, "../../examples/webgpu_loader_gltf_anisotropy.rs";
PostprocessingTransition, webgpu_postprocessing_transition, "../../examples/webgpu_postprocessing_transition.rs";
PostprocessingSobel, webgpu_postprocessing_sobel, "../../examples/webgpu_postprocessing_sobel.rs";
ProceduralTexture, webgpu_procedural_texture, "../../examples/webgpu_procedural_texture.rs";}

impl Which {
    /// The name with or without its `webgpu_` prefix, or its 1-based index in
    /// the table.
    fn parse(name: &str) -> Option<Self> {
        let trimmed = name.trim_start_matches("webgpu_");
        if let Some(index) = name.parse::<usize>().ok().filter(|i| *i >= 1) {
            return Self::ALL.get(index - 1).copied();
        }
        Self::ALL
            .into_iter()
            .find(|which| which.short_name() == trimmed)
    }

    fn short_name(self) -> &'static str {
        self.name().trim_start_matches("webgpu_")
    }

    /// The example after this one, for `]` and `VIEWER_AUTOSWITCH`.
    fn next(self) -> Self {
        let i = Self::ALL.iter().position(|w| *w == self).unwrap();
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// The example before this one, for `[`.
    fn previous(self) -> Self {
        let i = Self::ALL.iter().position(|w| *w == self).unwrap();
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    fn list() -> String {
        Self::ALL
            .iter()
            .enumerate()
            .map(|(index, which)| format!("  {:>2}  {}", index + 1, which.name()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Example {
    /// Builds the example through its own `init()`, then — if the caller has
    /// an instance of its own — moves the renderer onto it.
    ///
    /// `init()` is the graded example's code verbatim, so its `Renderer` is on
    /// an instance of its own making, with no display handle and hence no
    /// surface support. [`Renderer::rebuilt_on`] swaps in one on the viewer's
    /// instance, carrying over everything `init()` set; the scene holds no GPU
    /// state yet, because every buffer and texture is uploaded lazily on the
    /// first render.
    fn build(which: Which, instance: Option<wgpu::Instance>) -> Self {
        let mut scene = Self::build_raw(which);
        if let Some(instance) = instance {
            let renderer = scene
                .renderer()
                .rebuilt_on(instance)
                .expect("three-rs viewer: cannot create the renderer");
            *scene.renderer() = renderer;
        }
        scene
    }

    /// A window resize: the example's own `onWindowResize()`, and then the
    /// element size its controls measure drags against.
    fn set_size(&mut self, width: u32, height: u32) {
        let (w, h) = (width.max(1) as f64, height.max(1) as f64);
        self.resize_example(w, h);
        if let Some(controls) = self.controls() {
            controls.set_element_size(w, h);
        }
    }
}

// ------------------------------------------------------------ frame timer

/// Steady-state frame time: the mean and max of the last [`Self::WINDOW`]
/// samples, reported only once [`Self::WARMUP`] frames have gone by, so the
/// first frames' program builds and texture uploads never count.
struct FrameTimer {
    /// Every sample so far, capped at `WINDOW` in a ring.
    samples: Vec<Duration>,
    next: usize,
    /// How many frames have been recorded in total.
    frames: u64,
}

impl FrameTimer {
    /// Frames ignored before any number is reported.
    const WARMUP: u64 = 10;
    /// Frames the mean and max are over.
    const WINDOW: usize = 60;

    fn new() -> Self {
        Self {
            samples: Vec::with_capacity(Self::WINDOW),
            next: 0,
            frames: 0,
        }
    }

    fn record(&mut self, sample: Duration) {
        self.frames += 1;
        if self.frames <= Self::WARMUP {
            return;
        }
        if self.samples.len() < Self::WINDOW {
            self.samples.push(sample);
        } else {
            self.samples[self.next] = sample;
        }
        self.next = (self.next + 1) % Self::WINDOW;
    }

    /// `(mean, max, count)` over the window, or `None` during the warm-up.
    fn steady(&self) -> Option<(Duration, Duration, usize)> {
        if self.samples.is_empty() {
            return None;
        }
        let total: Duration = self.samples.iter().sum();
        let max = *self.samples.iter().max().unwrap();
        Some((total / self.samples.len() as u32, max, self.samples.len()))
    }

    /// The report line's tail: `mean 1.83 ms max 2.41 ms (last 60 frames, after
    /// 10 warm-up)`, or a note that the warm-up is still running.
    fn report(&self) -> String {
        match self.steady() {
            Some((mean, max, count)) => format!(
                "mean {:.2} ms max {:.2} ms (last {count} frames, after {} warm-up)",
                mean.as_secs_f64() * 1e3,
                max.as_secs_f64() * 1e3,
                Self::WARMUP
            ),
            None => format!("warming up ({} of {} frames)", self.frames, Self::WARMUP),
        }
    }
}

// ---------------------------------------------------- winit to the controls

/// Turns a winit pointer position and button into the [`PointerEvent`] the
/// controls expect.
///
/// The controls take element-relative coordinates with y downwards, which is
/// what winit's `CursorMoved` already reports, so this is only a rename. The
/// pointer id is constant because a mouse is one pointer; the JS counts
/// fingers by distinct ids and this viewer has none.
fn pointer_event(
    cursor: (f64, f64),
    button: Option<MouseButton>,
    modifiers: (bool, bool),
) -> PointerEvent {
    PointerEvent {
        pointer_id: 1,
        button: match button {
            Some(MouseButton::Left) | None => OrbitButton::Left,
            Some(MouseButton::Middle) => OrbitButton::Middle,
            Some(MouseButton::Right) => OrbitButton::Right,
            Some(_) => OrbitButton::Other,
        },
        client_x: cursor.0,
        client_y: cursor.1,
        ctrl_key: modifiers.0,
        meta_key: false,
        shift_key: modifiers.1,
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
    scene: Option<Example>,
    gpu: Option<Gpu>,

    requested_size: (u32, u32),
    frames: u32,
    last_report: Instant,
    /// The CPU side of each frame (`Example::animate()`), for the report line.
    timer: FrameTimer,

    cursor: (f64, f64),
    dragging: Option<MouseButton>,
    /// Ctrl and Shift, which the controls' default bindings read off a
    /// pointer event to turn a left drag into a pan.
    modifiers: (bool, bool),

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
        let instance = self.instance.clone();
        let mut scene = Example::build(which, instance);
        let size = self
            .gpu
            .as_ref()
            .map(|g| g.size)
            .unwrap_or(self.requested_size);
        scene.set_size(size.0, size.1);
        self.scene = Some(scene);
        self.timer = FrameTimer::new();
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
        let Some(scene) = self.scene.as_mut() else {
            return;
        };

        let t0 = Instant::now();
        scene.animate();
        let t_animate = t0.elapsed();
        self.timer.record(t_animate);
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
            println!(
                "{} — {w}x{h} — {fps:.1} fps — render {}",
                self.which.name(),
                self.timer.report()
            );
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
            backends: three_rs::BACKENDS,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
                event_loop.owned_display_handle(),
            ))
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("three-rs viewer: cannot create a surface");

        let mut scene = Example::build(self.which, Some(instance.clone()));

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
        println!(
            "controls: left-drag orbit, right-drag pan, wheel dolly, arrows pan, \
             [ and ] switch example, Esc quit"
        );
        println!("{}", Which::list());

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = (state.control_key(), state.shift_key());
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                // Esc and the two switch keys are the viewer's; every other
                // key the controls bind is the example's.
                match event.logical_key.as_ref() {
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character("[") => self.switch(self.which.previous()),
                    Key::Character("]") => self.switch(self.which.next()),
                    key => {
                        let arrow = match key {
                            Key::Named(NamedKey::ArrowLeft) => Some(OrbitKey::ArrowLeft),
                            Key::Named(NamedKey::ArrowUp) => Some(OrbitKey::ArrowUp),
                            Key::Named(NamedKey::ArrowRight) => Some(OrbitKey::ArrowRight),
                            Key::Named(NamedKey::ArrowDown) => Some(OrbitKey::ArrowDown),
                            _ => None,
                        };
                        let (Some(arrow), Some(scene)) = (arrow, self.scene.as_mut()) else {
                            return;
                        };
                        let event = KeyEvent {
                            key: arrow,
                            ctrl_key: self.modifiers.0,
                            meta_key: false,
                            shift_key: self.modifiers.1,
                        };
                        if let Some((controls, camera)) = scene.controls_and_camera() {
                            controls.key(camera, &event);
                        }
                    }
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
                let pressed = state == ElementState::Pressed;
                self.dragging = pressed.then_some(button);
                let event = pointer_event(self.cursor, Some(button), self.modifiers);
                let Some(scene) = self.scene.as_mut() else {
                    return;
                };
                if let Some((controls, camera)) = scene.controls_and_camera() {
                    if pressed {
                        controls.pointer_down(camera, &event);
                    } else {
                        controls.pointer_up(&event);
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                let event = pointer_event(self.cursor, self.dragging, self.modifiers);
                let Some(scene) = self.scene.as_mut() else {
                    return;
                };
                if let Some((controls, camera)) = scene.controls_and_camera() {
                    controls.pointer_move(camera, &event);
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // winit reports a line count or a pixel count; the DOM reports
                // the same two through `deltaMode`, which the controls already
                // normalise, so each maps straight across. The sign is the
                // same as the DOM's: scrolling away from you is negative.
                let (delta_y, delta_mode) = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (-y as f64, WheelDelta::Line),
                    MouseScrollDelta::PixelDelta(p) => (-p.y, WheelDelta::Pixel),
                };
                let event = WheelEvent {
                    client_x: self.cursor.0,
                    client_y: self.cursor.1,
                    delta_y,
                    delta_mode,
                    ctrl_key: self.modifiers.0,
                };
                let Some(scene) = self.scene.as_mut() else {
                    return;
                };
                if let Some((controls, camera)) = scene.controls_and_camera() {
                    controls.wheel(camera, &event);
                }
            }

            WindowEvent::RedrawRequested => {
                // `VIEWER_AUTOSWITCH=<seconds>` cycles the examples on a timer,
                // which is how the key-switching path gets exercised from a
                // script.
                if let Some(period) = self.autoswitch {
                    if self.last_switch.elapsed().as_secs_f64() >= period {
                        self.last_switch = Instant::now();
                        self.switch(self.which.next());
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

// ------------------------------------------------------------ headless mode

/// The count half of the performance ladder: three more frames of the example,
/// tiled into `target/e2e/<example>/strip.png` with each frame's draw calls and
/// build counts in the gutter under it (issue #68).
///
/// It goes beside the time the caller has just printed, and for the same
/// reason: the time says a frame is cheap, the counts say why — a frame that
/// re-uploads a live geometry or rebuilds a program shows an orange line here
/// while staying well inside the time ceiling.
fn write_strip(which: Which, scene: &mut Example) {
    let strip = three_rs::testing::strip(
        scene,
        |scene| scene.renderer(),
        &mut |scene: &mut Example| scene.animate(),
        &mut [
            ("", &mut |_: &mut Example| {}),
            ("", &mut |_: &mut Example| {}),
        ],
    )
    .expect("three-rs viewer: the strip could not be read back");

    for (index, frame) in strip.frames.iter().enumerate() {
        println!(
            "{}: strip frame {} — {}",
            which.name(),
            index + 1,
            frame.info
        );
    }

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/e2e")
        .join(which.name());
    std::fs::create_dir_all(&dir).expect("three-rs viewer: cannot create the strip directory");
    let path = dir.join("strip.png");
    strip.write_png(path.to_str().expect("three-rs viewer: the path is UTF-8"));
    println!("{}: wrote {}", which.name(), path.display());
}

/// Drives `--orbit` / `--zoom` / `--pan` through the example's own controls,
/// as synthetic events, so that what a flag renders is exactly what the same
/// drag on the window would.
fn drive(scene: &mut Example, size: (u32, u32), orbit: (f64, f64), zoom: f64, pan: (f64, f64)) {
    if orbit == (0.0, 0.0) && zoom == 0.0 && pan == (0.0, 0.0) {
        return;
    }

    let centre = (size.0 as f64 / 2.0, size.1 as f64 / 2.0);
    let Some((controls, camera)) = scene.controls_and_camera() else {
        eprintln!(
            "--orbit/--zoom/--pan ignored: this example's page creates no OrbitControls, \
             so neither does it"
        );
        return;
    };

    let at = |button: OrbitButton, x: f64, y: f64| PointerEvent {
        pointer_id: 1,
        button,
        client_x: x,
        client_y: y,
        ctrl_key: false,
        meta_key: false,
        shift_key: false,
    };

    if orbit != (0.0, 0.0) {
        controls.pointer_down(camera, &at(OrbitButton::Left, centre.0, centre.1));
        controls.pointer_move(
            camera,
            &at(OrbitButton::Left, centre.0 + orbit.0, centre.1 + orbit.1),
        );
        controls.pointer_up(&at(
            OrbitButton::Left,
            centre.0 + orbit.0,
            centre.1 + orbit.1,
        ));
    }

    if zoom != 0.0 {
        // One notch of a mouse wheel is 120 pixels' worth of `deltaY`, and the
        // page's `_getZoomScale` reads it that way; `--zoom N` is N notches
        // towards the scene.
        controls.wheel(
            camera,
            &WheelEvent {
                client_x: centre.0,
                client_y: centre.1,
                delta_y: -120.0 * zoom,
                delta_mode: WheelDelta::Pixel,
                ctrl_key: false,
            },
        );
    }

    if pan != (0.0, 0.0) {
        controls.pointer_down(camera, &at(OrbitButton::Right, centre.0, centre.1));
        controls.pointer_move(
            camera,
            &at(OrbitButton::Right, centre.0 + pan.0, centre.1 + pan.1),
        );
        controls.pointer_up(&at(OrbitButton::Right, centre.0 + pan.0, centre.1 + pan.1));
    }
}

/// Renders `frames` frames headless, reporting the steady-state frame time
/// (`--headless`), and with `path` also writes the canvas as a PNG plus the
/// result of `Renderer::present()` into an off-screen `bgra8unorm` texture, so
/// the blit the window uses is covered too (`--screenshot`).
#[allow(clippy::too_many_arguments)] // internal to the viewer binary
fn screenshot(
    which: Which,
    size: (u32, u32),
    frames: u32,
    path: Option<&str>,
    orbit: (f64, f64),
    zoom: f64,
    pan: (f64, f64),
    pinned: &[(String, f64)],
    // `series` is `--times`: one file per time, named `<stem>_<t>.png`.
    series: bool,
) {
    if !pinned.is_empty() {
        for (index, (label, time)) in pinned.iter().enumerate() {
            // `--time` / `--times`: the page's clock stands still at T, which
            // is exactly what three.js' `deterministic-injection.js` does to
            // the page, except at T instead of 0. The example reads it through
            // `now_ms()` / `date_now_ms()` and so does the renderer's
            // `NodeFrame`, so nothing here has to know which clock the example
            // uses.
            //
            // A fresh scene per time, not just a fresh renderer: `range()`'s
            // draws from the deterministic `Math.random` are consumed per
            // render in this port, so a second render in the same renderer
            // would see a different sequence than three.js' first frame does.
            three_rs::testing::pin_time(Some(time * 1000.0));
            let mut scene = Example::build(which, None);
            if index == 0 {
                println!(
                    "adapter: {:?}",
                    scene.renderer().adapter_info().name.clone()
                );
            }
            scene.set_size(size.0, size.1);
            drive(&mut scene, size, orbit, zoom, pan);

            for _ in 0..frames.max(1) {
                scene.animate();
            }

            let Some(path) = path else { continue };
            let (width, height, pixels) = scene.renderer().read_canvas_pixels().unwrap();
            let out = if !series {
                path.to_string()
            } else {
                format!("{}_{label}.png", path.trim_end_matches(".png"))
            };
            three_rs::testing::write_png(&out, width, height, &pixels);
            println!(
                "wrote {out} ({width}x{height}) at pinned t={time}s \
                 (Date.now() = performance.now() = {}ms)",
                time * 1000.0
            );
        }

        three_rs::testing::pin_time(None);
        return;
    }

    let mut scene = Example::build(which, None);
    println!(
        "adapter: {:?}",
        scene.renderer().adapter_info().name.clone()
    );
    scene.set_size(size.0, size.1);
    drive(&mut scene, size, orbit, zoom, pan);

    // Each frame is timed from the example's `animate()` to the GPU finishing
    // it — there is no vsync here, so this is the whole cost of a frame, the
    // number the e2e harness's steady-frame ceiling is compared against. The
    // clock runs, so the example animates exactly as it does in the window.
    let mut timer = FrameTimer::new();
    for frame in 0..frames.max(1) {
        let t0 = Instant::now();
        scene.animate();
        scene
            .renderer()
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
        let elapsed = t0.elapsed();
        timer.record(elapsed);
        if std::env::var("VIEWER_TRACE").is_ok() {
            eprintln!("frame {frame}: {:.2} ms", elapsed.as_secs_f64() * 1e3);
        }
    }
    println!(
        "{} — {}x{} — {frames} frame(s) headless — frame {}",
        which.name(),
        size.0,
        size.1,
        timer.report()
    );

    write_strip(which, &mut scene);

    let Some(path) = path else { return };

    let (width, height, pixels) = scene.renderer().read_canvas_pixels().unwrap();
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

/// The viewer's entry point; `src/bin/viewer.rs` calls it on every target but
/// wasm32.
pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut which = Which::ALL[0];
    let mut size = (800u32, 500u32);
    let mut frames = 1u32;
    let mut shot: Option<String> = None;
    let mut headless = false;
    let mut orbit = (0.0f64, 0.0f64);
    let mut zoom = 0.0f64;
    let mut pan = (0.0f64, 0.0f64);
    // `--time` / `--times`: the label is kept as typed, for the file name.
    let mut pinned: Vec<(String, f64)> = Vec::new();
    let mut series = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => {
                println!("{}", Which::list());
                return;
            }
            "--headless" => headless = true,
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
                        "usage: viewer <example | index> [--headless] [--screenshot out.png] \
                         [--frames N] [--width W] [--height H] [--orbit DX DY] \
                         [--zoom NOTCHES] [--pan DX DY] [--time T | --times T1,T2,...]\n\
                         \x20      viewer --list\n\nexamples:\n{}",
                        Which::list()
                    );
                    std::process::exit(2);
                }
            },
        }
        i += 1;
    }

    if headless || shot.is_some() {
        screenshot(
            which,
            size,
            frames,
            shot.as_deref(),
            orbit,
            zoom,
            pan,
            &pinned,
            series,
        );
        return;
    }

    let event_loop = EventLoop::new().expect("three-rs viewer: no event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let now = Instant::now();
    let mut viewer = Viewer {
        which,
        instance: None,
        scene: None,
        gpu: None,
        requested_size: size,
        frames: 0,
        last_report: now,
        timer: FrameTimer::new(),
        cursor: (0.0, 0.0),
        dragging: None,
        modifiers: (false, false),
        autoswitch: std::env::var("VIEWER_AUTOSWITCH")
            .ok()
            .and_then(|v| v.parse().ok()),
        last_switch: now,
    };

    event_loop
        .run_app(&mut viewer)
        .expect("three-rs viewer: event loop failed");
}
