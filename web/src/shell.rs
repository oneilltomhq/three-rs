//! The graded examples, in a browser, on the browser's own WebGPU.
//!
//! The examples are not reimplemented here. They are included as modules —
//! `#[path = "../../examples/<name>.rs"] mod <name>;` — exactly as
//! `tests/e2e/main.rs` and `src/bin/viewer.rs` include them, so this is the
//! third driver of one corpus and not a second copy of it (issue #128).
//!
//! # The page's flow
//!
//! `?example=<name>` names the example. Then, in order:
//!
//! 1. create a wgpu instance on [`three_rs::BACKENDS`], which is
//!    `BROWSER_WEBGPU` here, and a surface on the page's `<canvas>`;
//! 2. `request_adapter` and `request_device`, both awaited — this is the
//!    asynchrony that cannot happen inside a synchronous `init()`;
//! 3. hand the three handles to [`three_rs::renderer::adopt_device`], so that
//!    the `Renderer::new` inside the example's `init()` finds them;
//! 4. fetch every asset the example's manifest names from a pinned three.js
//!    commit on raw.githubusercontent.com, and
//!    [`three_rs::io::preload`] each one under the path the example will ask
//!    for;
//! 5. call `init()`, then `animate()` once, then blit the renderer's canvas
//!    texture onto the surface — the *graded* frame, 800x500, with the clock
//!    pinned to 0;
//! 6. unpin onto the page's own clock, size the canvas to the window, attach
//!    the input listeners and run `requestAnimationFrame` until the tab
//!    closes.
//!
//! # Held on the graded frame: `&hold`
//!
//! `?example=<name>&hold` stops after step 5. There is no step 6 — no resize
//! to the window, no animation loop, no listeners — so the frame on the canvas
//! stays the graded one. Instead the renderer's own 800x500 canvas texture is
//! read back, the same texture and the same copy `tests/e2e/main.rs` grades
//! natively, and handed to the page's JavaScript:
//!
//! - `window.__three_rs_graded` is `{ width, height, pixels }`, `pixels` a
//!   `Uint8Array` of tightly packed, top-down RGBA8;
//! - `<body data-graded="1">` says it is there;
//! - any failure on the way — no WebGPU, a fetch, a panic — sets
//!   `<body data-error="…">` instead, so a driver polling for one of the two
//!   attributes never waits out its timeout on a page that has already died.
//!
//! This is what `tools/web_gate.mjs` drives: it grades the pixels outside the
//! page with three.js' own `image.js`, exactly as the native ladder does. The
//! pixels are read from the texture rather than the canvas element because a
//! WebGPU canvas' `toDataURL()` after the frame has been presented is allowed
//! to be blank.
//!
//! Step 5 is exactly what it was before this loop existed, and deliberately:
//! the first thing a visitor sees is the frame the ladder grades, and it is
//! reproducible because `now_ms()` and `date_now_ms()` both return 0 while
//! [`three_rs::testing::pin_time`] holds them there, the way three.js' own
//! `deterministic-injection.js` pins `performance.now` and `Date.now`.
//!
//! # The page's clock
//!
//! The loop does not simply unpin. It keeps the clock pinned and advances it
//! itself: frame *n* is pinned to `rAF timestamp - first rAF timestamp`. Two
//! reasons. The graded frame happened at t = 0, and the first animated frame
//! should continue from there rather than jump forward by however long the
//! asset fetches took — a `Timer` created during `init()` recorded
//! `start_time = 0`, and an unpinned `performance.now()` would hand it a
//! multi-second first delta. And the pages that read an absolute time rather
//! than a delta (`Date.now() * 0.0005` and friends) then start where the
//! graded frame left off instead of at an arbitrary phase. Deltas are the
//! real ones either way, so the motion runs at wall-clock speed.
//!
//! # Input
//!
//! The canvas' pointer, wheel and key events are translated into the
//! input-agnostic value types [`three_rs::addons::controls`] defines and handed
//! to the example's own `OrbitControls`. There is no controls implementation
//! here — that is the point of the addon being input-agnostic: this file is a
//! DOM-to-struct adapter and nothing else, and the viewer's winit handler is
//! the same adapter over a different event source.
//!
//! # Nothing of Three's is committed
//!
//! The manifests hold paths relative to a three.js checkout and the assets are
//! fetched at run time from a pinned tag, the same promise `examples/gallery.rs`
//! keeps for the README's thumbnails.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use three_rs::addons::controls::{Key, KeyEvent, MouseButton, OrbitControls, WheelDelta};
use three_rs::PerspectiveCamera;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// An example's three.js name: its module's, unless the table gives one.
/// Three's `webgpu_textures_2d-array_compressed` has a hyphen, which a Rust
/// module name cannot.
macro_rules! example_name {
    ($module:ident) => {
        stringify!($module)
    };
    ($module:ident $name:literal) => {
        $name
    };
}

/// Every graded example, in the README's order, as modules plus the two enums
/// and the dispatch over them.
///
/// The table is hand-maintained, like the viewer's and
/// `examples/web_manifests.rs`', and for the same reason: `#[path]` takes a
/// string literal, so the includes cannot be generated from a list. The
/// freshness gate is `web_manifests`' `#[test]`, which asserts its own copy of
/// the list is the README's; a name here with no committed manifest fails to
/// compile at the `include_str!` below.
macro_rules! examples {
    ( $( $variant:ident , $module:ident , $path:literal $( , $name:literal )? ; )* ) => {
        $(
            #[path = $path]
            #[allow(dead_code)] // the example's own `main()` is unused here
            mod $module;
        )*

        /// Which graded example the page was asked for.
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Which {
            $( $variant, )*
        }

        /// The built example, holding that module's own `App`.
        #[allow(clippy::large_enum_variant)] // one of these exists per page; an indirection buys nothing
        enum Example {
            $( $variant($module::App), )*
        }

        impl Which {
            const ALL: &'static [Which] = &[ $( Which::$variant, )* ];

            fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => example_name!($module $( $name )?), )*
                }
            }

            /// The committed manifest, compiled in. Embedding it rather than
            /// fetching it saves a round trip and, more to the point, makes a
            /// manifest that was never generated a build error instead of a
            /// 404.
            fn manifest(self) -> &'static str {
                match self {
                    $( Self::$variant => include_str!(
                        concat!("../manifests/", example_name!($module $( $name )?), ".json")
                    ), )*
                }
            }

            /// `window.innerWidth`/`innerHeight` as the example itself
            /// declares them: the graded frame's size, which is what the first
            /// frame is rendered at before the window takes over.
            fn graded_size(self) -> (f64, f64) {
                match self {
                    $( Self::$variant => ($module::INNER_WIDTH, $module::INNER_HEIGHT), )*
                }
            }

            /// The example's `DPR` constant — the `setPixelRatio()` its page
            /// calls, pinned to 1 for grading. The canvas' backing store is
            /// sized by this so that the renderer's canvas texture and the
            /// surface agree exactly and the blit is one-to-one.
            fn dpr(self) -> f64 {
                match self {
                    $( Self::$variant => $module::DPR, )*
                }
            }

            fn init(self) -> Example {
                match self {
                    $( Self::$variant => Example::$variant($module::init()), )*
                }
            }
        }

        impl Example {
            fn renderer(&mut self) -> &mut three_rs::Renderer {
                match self {
                    $( Example::$variant(app) => &mut app.renderer, )*
                }
            }

            /// The example's own `animate()`, unchanged and unassisted.
            fn animate(&mut self) {
                match self {
                    $( Example::$variant(app) => $module::animate(app), )*
                }
            }

            /// The example's own `resize()` — its page's `onWindowResize()`.
            fn resize(&mut self, width: f64, height: f64) {
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
            /// so `pointer_move` and friends take it as an argument. They are
            /// two fields of one `App` and borrowing both is sound, but only
            /// the example can say so.
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
LinesFatRaycasting, webgpu_lines_fat_raycasting, "../../examples/webgpu_lines_fat_raycasting.rs";
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
MaterialsTextureManualmipmap, webgpu_materials_texture_manualmipmap, "../../examples/webgpu_materials_texture_manualmipmap.rs";
TslVfxFlames, webgpu_tsl_vfx_flames, "../../examples/webgpu_tsl_vfx_flames.rs";
ProceduralTexture, webgpu_procedural_texture, "../../examples/webgpu_procedural_texture.rs";
PostprocessingSobel, webgpu_postprocessing_sobel, "../../examples/webgpu_postprocessing_sobel.rs";
PostprocessingTransition, webgpu_postprocessing_transition, "../../examples/webgpu_postprocessing_transition.rs";
TslRagingSea, webgpu_tsl_raging_sea, "../../examples/webgpu_tsl_raging_sea.rs";
TslAngularSlicing, webgpu_tsl_angular_slicing, "../../examples/webgpu_tsl_angular_slicing.rs";
Textures2dArrayCompressed, webgpu_textures_2d_array_compressed, "../../examples/webgpu_textures_2d-array_compressed.rs", "webgpu_textures_2d-array_compressed";
ComputeTexture, webgpu_compute_texture, "../../examples/webgpu_compute_texture.rs";
VolumePerlin, webgpu_volume_perlin, "../../examples/webgpu_volume_perlin.rs";
ShadowmapVsm, webgpu_shadowmap_vsm, "../../examples/webgpu_shadowmap_vsm.rs";
ShadowmapPointlight, webgpu_shadowmap_pointlight, "../../examples/webgpu_shadowmap_pointlight.rs";
StructDrawindirect, webgpu_struct_drawindirect, "../../examples/webgpu_struct_drawindirect.rs";
Particles, webgpu_particles, "../../examples/webgpu_particles.rs";
Clearcoat, webgpu_clearcoat, "../../examples/webgpu_clearcoat.rs";
ModifierCurve, webgpu_modifier_curve, "../../examples/webgpu_modifier_curve.rs";
Occlusion, webgpu_occlusion, "../../examples/webgpu_occlusion.rs";
}

/// Where an example's assets come from: three.js at the commit this port is
/// graded against, so a browser frame reads the very bytes the native ladder
/// reads off disk. Pinned, never `main` — an asset changing upstream would
/// silently move a graded frame. 5f610f5 is past r186 for the cube PMREM
/// (#146); it becomes the r187 tag once upstream tags it.
const THREE_JS_REV: &str = "5f610f516730eb11e0166d9fc21dfc34538dcdeb";
const ASSET_BASE: &str = "https://raw.githubusercontent.com/mrdoob/three.js";

/// Our own rendered frame for each example, for the browsers that cannot run
/// one. Same raw-GitHub URLs the README's gallery grid uses.
const GALLERY_BASE: &str =
    "https://raw.githubusercontent.com/oneilltomhq/three-rs/main/docs/gallery";

/// One running example and the browser state it needs per frame.
///
/// Held in an `Rc<RefCell<…>>` because three different callbacks reach it —
/// `requestAnimationFrame`, the canvas' input listeners and the window's
/// `resize` — and a page has one thread, so the borrows never overlap.
struct Page {
    which: Which,
    example: Example,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    canvas: web_sys::HtmlCanvasElement,
    /// CSS pixels, the units the example's `resize()` and the controls'
    /// `set_element_size()` both speak.
    size: (f64, f64),
    /// The first `requestAnimationFrame` timestamp, which becomes the page's
    /// t = 0. See the module header, "The page's clock".
    origin_ms: Option<f64>,
    fps_window_start_ms: f64,
    fps_window_frames: u32,
}

impl Page {
    /// One animated frame: advance the pinned clock, let the example animate
    /// itself, blit.
    fn frame(&mut self, timestamp_ms: f64) {
        let origin = *self.origin_ms.get_or_insert(timestamp_ms);
        three_rs::testing::pin_time(Some(timestamp_ms - origin));

        self.example.animate();

        if let Err(message) = present(self.example.renderer(), &self.surface, self.format) {
            // Not fatal and not reported to the status line: a surface texture
            // can be missing for a frame across a resize, and a line per frame
            // would bury the fps.
            web_sys::console::warn_1(&JsValue::from_str(&message));
        }

        self.fps_window_frames += 1;
        let elapsed = timestamp_ms - self.fps_window_start_ms;
        if elapsed >= 500.0 {
            let fps = f64::from(self.fps_window_frames) * 1000.0 / elapsed;
            self.fps_window_start_ms = timestamp_ms;
            self.fps_window_frames = 0;
            let (width, height) = self.size;
            report(&format!(
                "{} — {}x{} — {fps:.0} fps{}",
                self.which.name(),
                width as u32,
                height as u32,
                if self.has_controls() {
                    " — drag to orbit, right-drag to pan, wheel to dolly, arrows to pan"
                } else {
                    ""
                }
            ));
        }
    }

    fn has_controls(&mut self) -> bool {
        self.example.controls().is_some()
    }

    /// The window changed size: the example's own `onWindowResize()`, the
    /// controls' element size, the canvas' backing store and the surface.
    ///
    /// The three.js pages size their canvas to `window.innerWidth` and
    /// `innerHeight`, so the canvas is full-bleed here too and `clientX`/
    /// `clientY` need no offset — which is exactly the assumption
    /// `OrbitControls`' zoom-to-cursor makes, since the port takes an element
    /// *size* rather than a bounding rect.
    fn resize_to(&mut self, width: f64, height: f64) {
        let width = width.max(1.0);
        let height = height.max(1.0);
        self.size = (width, height);

        self.example.resize(width, height);
        if let Some(controls) = self.example.controls() {
            controls.set_element_size(width, height);
        }

        // The backing store is CSS pixels times the example's own `DPR`, not
        // the browser's `devicePixelRatio`: the example's `init()` called
        // `set_pixel_ratio(DPR)` and its `resize()` passes CSS pixels to
        // `set_size()`, so this is the size the renderer's canvas texture
        // actually has and the blit stays one-to-one. `DPR` is 1 for every
        // graded example, because that is what the grader pins it to; a
        // HiDPI screen therefore shows the page's own resolution rather than
        // the screen's. Deliberate: the browser shows what the ladder grades.
        let dpr = self.which.dpr();
        let pixel_width = (width * dpr).round().max(1.0) as u32;
        let pixel_height = (height * dpr).round().max(1.0) as u32;
        self.canvas.set_width(pixel_width);
        self.canvas.set_height(pixel_height);
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{width}px"));
        let _ = style.set_property("height", &format!("{height}px"));

        self.surface.configure(
            &self.device,
            &surface_configuration(self.format, pixel_width, pixel_height),
        );
    }
}

/// wasm-bindgen's entry point, called by the generated JS glue once the page
/// has its canvas.
///
/// Everything real happens in [`run`], on `spawn_local`: the adapter, the
/// device and every asset fetch are promises, and a page has one thread to
/// wait on them with.
#[wasm_bindgen(start)]
pub fn start() {
    // The readable console trace `console_error_panic_hook` gives, and the
    // same message on `<body data-error>` for a driver to find (see "Held on
    // the graded frame" above): a panic otherwise leaves a page that simply
    // never finishes.
    std::panic::set_hook(Box::new(|info| {
        console_error_panic_hook::hook(info);
        mark_error(&format!("panic: {info}"));
    }));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(message) = run().await {
            report(&message);
            mark_error(&message);
        }
    });
}

/// The `?example=` query parameter, or `None` when the page is the index.
fn requested_example() -> Option<String> {
    let search = window().location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get("example")
}

/// `&hold` in the query: stop on the graded frame and publish its pixels.
fn requested_hold() -> bool {
    let Ok(search) = window().location().search() else {
        return false;
    };
    web_sys::UrlSearchParams::new_with_str(&search)
        .map(|params| params.has("hold"))
        .unwrap_or(false)
}

/// Sets `data-<name>` on `<body>`, the one place a driver polls.
fn mark(name: &str, value: &str) {
    if let Some(body) = document().body() {
        let _ = body.set_attribute(&format!("data-{name}"), value);
    }
}

/// Records a failure where a driver will look for it. Set on every page, held
/// or not; an attribute nobody reads costs nothing.
fn mark_error(message: &str) {
    mark("error", message);
}

fn window() -> web_sys::Window {
    web_sys::window().expect("three-rs-web: no window")
}

fn document() -> web_sys::Document {
    window().document().expect("three-rs-web: no document")
}

/// Puts one line of text in the page's status element. This is the only
/// reporting channel a page has that is not the console, and every failure
/// below ends here rather than in a panic, because a panic in a page is a
/// blank rectangle.
fn report(message: &str) {
    if let Some(status) = document().get_element_by_id("status") {
        status.set_text_content(Some(message));
    }
}

/// The still frame and one line, for a browser with no WebGPU.
///
/// Not a panic and not an empty page: the thumbnail is what this example looks
/// like, and the sentence says why it is a picture rather than a rendering.
fn no_webgpu(name: &str, reason: &str) {
    if let Some(canvas) = document().get_element_by_id("canvas") {
        let _ = canvas
            .dyn_ref::<web_sys::HtmlElement>()
            .map(|element| element.style().set_property("display", "none"));
    }
    if let Some(still) = document().get_element_by_id("still") {
        let _ = still.set_attribute("src", &format!("{GALLERY_BASE}/{name}.jpg"));
        let _ = still
            .dyn_ref::<web_sys::HtmlElement>()
            .map(|element| element.style().set_property("display", "block"));
    }
    report(&format!(
        "This browser has no WebGPU ({reason}), so this is a still of the frame \
         three-rs renders natively."
    ));
    mark_error(&format!("no WebGPU: {reason}"));
}

fn surface_configuration(
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        color_space: wgpu::SurfaceColorSpace::Auto,
        view_formats: vec![format],
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        width,
        height,
        desired_maximum_frame_latency: 2,
        present_mode: wgpu::PresentMode::AutoVsync,
    }
}

async fn run() -> Result<(), String> {
    let name = requested_example().ok_or("no ?example= in the URL")?;
    let which = *Which::ALL
        .iter()
        .find(|candidate| candidate.name() == name)
        .ok_or_else(|| format!("no graded example named {name}"))?;

    let (graded_width, graded_height) = which.graded_size();
    let pixel_width = (graded_width * which.dpr()) as u32;
    let pixel_height = (graded_height * which.dpr()) as u32;

    let canvas: web_sys::HtmlCanvasElement = document()
        .get_element_by_id("canvas")
        .ok_or("the page has no #canvas")?
        .dyn_into()
        .map_err(|_| "#canvas is not a <canvas>")?;
    canvas.set_width(pixel_width);
    canvas.set_height(pixel_height);

    // `navigator.gpu` missing is the common case on Firefox and Safari today,
    // and it is not an error: the page falls back to the still.
    if js_sys::Reflect::get(&window().navigator(), &JsValue::from_str("gpu"))
        .map(|gpu| gpu.is_undefined() || gpu.is_null())
        .unwrap_or(true)
    {
        no_webgpu(which.name(), "navigator.gpu is undefined");
        return Ok(());
    }

    report(&format!("{name}: asking for a WebGPU adapter…"));

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: three_rs::BACKENDS,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|error| format!("cannot create a surface on the canvas: {error}"))?;

    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            no_webgpu(which.name(), &format!("requestAdapter failed: {error}"));
            return Ok(());
        }
    };

    // The same feature request `Renderer::with_instance_async` makes: an
    // `r32float` texture is only sampled through a filtering sampler on a
    // device that asked for it, and a KTX2 texture only transcodes to a
    // compressed format the device asked for. Requested when the adapter has
    // them, exactly as the native path does, so a browser device is not
    // quietly less capable than a desktop one.
    let required_features = adapter.features()
        & (wgpu::Features::FLOAT32_FILTERABLE | three_rs::renderer::COMPRESSION_FEATURES);

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("three-rs device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| format!("cannot create a device: {error}"))?;

    // wgpu reports validation and out-of-memory errors on this callback; in a
    // page they would otherwise only reach the devtools console, and a frame
    // that failed validation is a black canvas with no explanation.
    device.on_uncaptured_error(std::sync::Arc::new(|error| {
        let message = format!("wgpu: {error}");
        web_sys::console::error_1(&JsValue::from_str(&message));
        // Natively an uncaptured validation error panics; a held page is being
        // graded, so it fails here as loudly as the native ladder would.
        if requested_hold() {
            mark_error(&message);
        }
    }));

    let format = {
        let caps = surface.get_capabilities(&adapter);
        let preferred = *caps
            .formats
            .first()
            .ok_or("the canvas surface supports no format")?;
        // Present to a NON-srgb format, exactly as the viewer does: the
        // renderer's canvas texture already holds sRGB-encoded bytes and an
        // `-srgb` view would encode them a second time.
        let linear = preferred.remove_srgb_suffix();
        if caps.formats.contains(&linear) {
            linear
        } else {
            preferred
        }
    };

    surface.configure(
        &device,
        &surface_configuration(format, pixel_width, pixel_height),
    );

    // Everything the example's synchronous `init()` will need, in place before
    // it runs: the device it will "create", and the bytes it will "read". The
    // device is cloned first because reconfiguring the surface on every window
    // resize needs one and `adopt_device` takes ownership.
    let page_device = device.clone();
    three_rs::renderer::adopt_device(adapter, device, queue);
    preload_assets(which).await?;

    // The graded frame, and nothing else: one `animate()` with both clocks
    // pinned to 0, exactly as `tests/e2e/main.rs` runs it.
    report(&format!("{name}: rendering…"));
    three_rs::testing::pin_time(Some(0.0));
    let mut example = which.init();
    example.animate();
    present(example.renderer(), &surface, format)?;
    report(&format!(
        "{name} — the graded frame, {pixel_width}x{pixel_height}, rendered in this browser",
    ));

    if requested_hold() {
        return publish_graded_frame(example.renderer()).await;
    }

    let page = Rc::new(RefCell::new(Page {
        which,
        example,
        surface,
        device: page_device,
        format,
        canvas: canvas.clone(),
        size: (graded_width, graded_height),
        origin_ms: None,
        fps_window_start_ms: 0.0,
        fps_window_frames: 0,
    }));

    // From here the page is live: full-window canvas, input, and a frame per
    // vsync. The class is what `index.html` hangs the full-bleed layout off.
    if let Some(body) = document().body() {
        let _ = body.class_list().add_1("playing");
    }
    page.borrow_mut()
        .resize_to(window_size().0, window_size().1);

    install_input(&page, &canvas);
    install_window_resize(&page);
    start_animation_loop(&page);

    Ok(())
}

/// Reads the renderer's canvas texture back and hands it to the page's
/// JavaScript as `window.__three_rs_graded`, then sets `data-graded`. See
/// "Held on the graded frame" in the module header.
async fn publish_graded_frame(renderer: &mut three_rs::Renderer) -> Result<(), String> {
    let (width, height, pixels) = renderer
        .read_canvas_pixels_async()
        .await
        .map_err(|error| format!("cannot read the graded frame back: {error}"))?;

    let graded = js_sys::Object::new();
    let set = |key: &str, value: &JsValue| {
        js_sys::Reflect::set(&graded, &JsValue::from_str(key), value)
            .map(|_| ())
            .map_err(|error| format!("cannot publish the graded frame: {error:?}"))
    };
    set("width", &JsValue::from(width))?;
    set("height", &JsValue::from(height))?;
    set(
        "pixels",
        &js_sys::Uint8Array::from(pixels.as_slice()).into(),
    )?;
    js_sys::Reflect::set(&window(), &JsValue::from_str("__three_rs_graded"), &graded)
        .map_err(|error| format!("cannot publish the graded frame: {error:?}"))?;

    mark("graded", "1");
    Ok(())
}

/// `window.innerWidth` and `window.innerHeight`, in CSS pixels — the two
/// numbers every one of the ported `onWindowResize()` handlers reads.
fn window_size() -> (f64, f64) {
    let window = window();
    let width = window
        .inner_width()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(800.0);
    let height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(500.0);
    (width, height)
}

/// The `requestAnimationFrame` loop.
///
/// The usual two-`Rc` shape: the closure has to hold a handle to itself in
/// order to re-request, so it is created into a cell it also captures.
fn start_animation_loop(page: &Rc<RefCell<Page>>) {
    type Frame = Closure<dyn FnMut(f64)>;

    let holder: Rc<RefCell<Option<Frame>>> = Rc::new(RefCell::new(None));
    let again = holder.clone();
    let page = page.clone();

    *holder.borrow_mut() = Some(Closure::new(move |timestamp: f64| {
        page.borrow_mut().frame(timestamp);
        if let Some(closure) = again.borrow().as_ref() {
            request_frame(closure);
        }
    }));

    if let Some(closure) = holder.borrow().as_ref() {
        request_frame(closure);
    }
    // The holder outlives this function through the closure's own capture of
    // `again`; leaking it is what keeps the loop alive for the tab's lifetime.
    std::mem::forget(holder);
}

fn request_frame(closure: &Closure<dyn FnMut(f64)>) {
    let _ = window().request_animation_frame(closure.as_ref().unchecked_ref());
}

/// Translates the canvas' DOM events into the controls' input-agnostic value
/// types.
///
/// Which events: the six three.js' own `OrbitControls.connect()` binds
/// (`pointerdown`, `pointermove`, `pointerup`, `pointercancel`, `wheel`,
/// `contextmenu`) plus `keydown`. `contextmenu` is cancelled so that a
/// right-drag pans instead of opening a menu, and `wheel` is bound
/// non-passively so that it can be cancelled at all — Chrome treats a wheel
/// listener as passive by default and would scroll the page under the canvas.
fn install_input(page: &Rc<RefCell<Page>>, canvas: &web_sys::HtmlCanvasElement) {
    // Arrow keys only reach an element that can hold focus.
    let _ = canvas.set_attribute("tabindex", "0");
    let _ = canvas.focus();

    fn listen<E: wasm_bindgen::convert::FromWasmAbi + 'static>(
        target: &web_sys::HtmlCanvasElement,
        name: &str,
        passive: bool,
        mut handler: impl FnMut(E) + 'static,
    ) {
        let closure = Closure::<dyn FnMut(E)>::new(move |event: E| handler(event));
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(passive);
        let _ = target.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            closure.as_ref().unchecked_ref(),
            &options,
        );
        // The listener lives as long as the page does.
        closure.forget();
    }

    {
        let page = page.clone();
        let canvas = canvas.clone();
        listen(
            &canvas.clone(),
            "pointerdown",
            false,
            move |event: web_sys::PointerEvent| {
                // Capture, so that a drag that leaves the canvas keeps arriving —
                // `OrbitControls.js` calls `setPointerCapture` for the same reason.
                let _ = canvas.set_pointer_capture(event.pointer_id());
                let _ = canvas.focus();
                event.prevent_default();
                let mut page = page.borrow_mut();
                if let Some((controls, camera)) = page.example.controls_and_camera() {
                    controls.pointer_down(camera, &pointer_event(&event));
                }
            },
        );
    }

    {
        let page = page.clone();
        listen(
            canvas,
            "pointermove",
            false,
            move |event: web_sys::PointerEvent| {
                event.prevent_default();
                let mut page = page.borrow_mut();
                if let Some((controls, camera)) = page.example.controls_and_camera() {
                    controls.pointer_move(camera, &pointer_event(&event));
                }
            },
        );
    }

    for name in ["pointerup", "pointercancel"] {
        let page = page.clone();
        let canvas_for_release = canvas.clone();
        listen(canvas, name, true, move |event: web_sys::PointerEvent| {
            let _ = canvas_for_release.release_pointer_capture(event.pointer_id());
            let mut page = page.borrow_mut();
            if let Some(controls) = page.example.controls() {
                controls.pointer_up(&pointer_event(&event));
            }
        });
    }

    {
        let page = page.clone();
        listen(canvas, "wheel", false, move |event: web_sys::WheelEvent| {
            event.prevent_default();
            let mut page = page.borrow_mut();
            if let Some((controls, camera)) = page.example.controls_and_camera() {
                controls.wheel(camera, &wheel_event(&event));
            }
        });
    }

    listen(
        canvas,
        "contextmenu",
        false,
        move |event: web_sys::Event| {
            event.prevent_default();
        },
    );

    {
        let page = page.clone();
        listen(
            canvas,
            "keydown",
            false,
            move |event: web_sys::KeyboardEvent| {
                let Some(key) = key_event(&event) else { return };
                event.prevent_default();
                let mut page = page.borrow_mut();
                if let Some((controls, camera)) = page.example.controls_and_camera() {
                    controls.key(camera, &key);
                }
            },
        );
    }
}

/// `window.addEventListener('resize', onWindowResize)`, the line every one of
/// the ported pages has.
fn install_window_resize(page: &Rc<RefCell<Page>>) {
    let page = page.clone();
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
        let (width, height) = window_size();
        page.borrow_mut().resize_to(width, height);
    });
    let _ = window().add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
    closure.forget();
}

fn pointer_event(event: &web_sys::PointerEvent) -> three_rs::addons::controls::PointerEvent {
    three_rs::addons::controls::PointerEvent {
        pointer_id: event.pointer_id(),
        button: match event.button() {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::Other,
        },
        client_x: f64::from(event.client_x()),
        client_y: f64::from(event.client_y()),
        ctrl_key: event.ctrl_key(),
        meta_key: event.meta_key(),
        shift_key: event.shift_key(),
    }
}

fn wheel_event(event: &web_sys::WheelEvent) -> three_rs::addons::controls::WheelEvent {
    three_rs::addons::controls::WheelEvent {
        client_x: f64::from(event.client_x()),
        client_y: f64::from(event.client_y()),
        delta_y: event.delta_y(),
        // `WheelEvent.DOM_DELTA_PIXEL` / `_LINE` / `_PAGE`, which the port
        // scales exactly as `OrbitControls._customWheelEvent` does.
        delta_mode: match event.delta_mode() {
            1 => WheelDelta::Line,
            2 => WheelDelta::Page,
            _ => WheelDelta::Pixel,
        },
        ctrl_key: event.ctrl_key(),
    }
}

/// The four keys `OrbitControls` reacts to. Anything else is left to the page,
/// unprevented.
fn key_event(event: &web_sys::KeyboardEvent) -> Option<KeyEvent> {
    let key = match event.key().as_str() {
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowUp" => Key::ArrowUp,
        "ArrowRight" => Key::ArrowRight,
        "ArrowDown" => Key::ArrowDown,
        _ => return None,
    };
    Some(KeyEvent {
        key,
        ctrl_key: event.ctrl_key(),
        meta_key: event.meta_key(),
        shift_key: event.shift_key(),
    })
}

/// Fetches every asset the manifest names and hands it to the I/O seam under
/// the path the example will ask for.
///
/// The key is `three_js_dir().join(relative)`: on wasm32 `three_js_dir()` is
/// the fixed virtual root `/vendor/three.js`, and the examples build their
/// asset paths through that same function, so host and example agree without
/// either knowing about the other.
async fn preload_assets(which: Which) -> Result<(), String> {
    let assets: Vec<String> = serde_json::from_str(which.manifest())
        .map_err(|error| format!("{}: bad manifest: {error}", which.name()))?;

    let root = three_rs::testing::three_js_dir();

    for (index, asset) in assets.iter().enumerate() {
        report(&format!(
            "{}: fetching asset {} of {} — {asset}",
            which.name(),
            index + 1,
            assets.len()
        ));
        let url = format!("{ASSET_BASE}/{THREE_JS_REV}/{asset}");
        let bytes = fetch(&url).await?;
        three_rs::io::preload(PathBuf::from(&root).join(asset), bytes);
    }

    Ok(())
}

/// `fetch(url)` down to bytes.
///
/// Sequential rather than concurrent on purpose for this first pass: the
/// progress line is readable, a failure names the file that failed, and the
/// heaviest example fetches a couple of dozen files.
async fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let response: web_sys::Response = JsFuture::from(window().fetch_with_str(url))
        .await
        .map_err(|error| format!("fetch {url} failed: {error:?}"))?
        .dyn_into()
        .map_err(|_| format!("fetch {url} did not return a Response"))?;

    if !response.ok() {
        return Err(format!("fetch {url}: HTTP {}", response.status()));
    }

    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|error| format!("{url}: no body: {error:?}"))?,
    )
    .await
    .map_err(|error| format!("{url}: cannot read the body: {error:?}"))?;

    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Blits the renderer's finished canvas texture onto the page's surface.
///
/// The mechanism is the viewer's, unchanged: `Renderer::present` is the
/// fullscreen blit `src/renderer/present.rs` already provides for exactly this
/// — the renderer draws into its own canvas texture and a caller decides where
/// that ends up. There is no second presentation path for the web.
fn present(
    renderer: &mut three_rs::Renderer,
    surface: &wgpu::Surface<'static>,
    format: wgpu::TextureFormat,
) -> Result<(), String> {
    let surface_texture = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(texture)
        | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
        other => return Err(format!("no surface texture to present to: {other:?}")),
    };

    let view = surface_texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            format: Some(format),
            ..Default::default()
        });

    if !renderer.present(&view, format) {
        return Err("the renderer has nothing to present".into());
    }

    renderer.queue().present(surface_texture);
    Ok(())
}
