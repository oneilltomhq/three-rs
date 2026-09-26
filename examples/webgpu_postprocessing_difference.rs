//! Port of `three.js/examples/webgpu_postprocessing_difference.html`, calling
//! the three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `timer.getDelta()` is 0. The GUI's `speed` starts at 0 as well, so
//! `mesh.rotation.y += delta * 5 * speed` is `+= 0` twice over.
//!
//! `Math.random` is never called. `OrbitControls`' constructor calls
//! `update()`, which points the camera at its target — the origin — and with
//! `enableDamping` the per-frame `update()` is a no-op while nothing has
//! moved. `new Inspector()` only registers the renderer with the inspector
//! panel; the `resize` listener never fires.
//!
//! The effect is four TSL lines on the pass, not an addon:
//!
//! ```text
//! frameDiff        = previousTexture.sub( currentTexture ).abs()
//! saturationAmount = luminance( frameDiff ).mul( 1000 ).clamp( 0, 3 )
//! outputNode       = saturation( currentTexture, saturationAmount )
//! ```
//!
//! Two things in it are easy to get wrong and invisible when you do:
//!
//! * **`luminance()` of a vec4.** `frameDiff` is a `vec4`, so TSL widens the
//!   `vec3` coefficients and the alpha difference is weighted **1.0**
//!   (`tests/nodes_dot_widening.rs`).
//! * **The "previous" texture is zero at frame 0.** `PassNode.updateBefore()`
//!   toggles the pair *before* rendering, so the graded frame's `frameDiff` is
//!   `| 0 − current |` against a texture nothing has ever drawn into. See
//!   [`three_rs::PassNode::previous_texture_node`].

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::box_geometry_default;
use three_rs::math::Color;
use three_rs::nodes::tsl::{luminance, saturation};
use three_rs::renderer::OUTPUT_ATTACHMENT;
use three_rs::Timer;
use three_rs::{
    ColorSpace, Fog, Mesh, MeshBasicNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline,
    Renderer, RendererParameters, Scene, TextureLoader, ToneMapping,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub mesh: three_rs::Node,
    /// The page's module-level `timer`.
    pub timer: Timer,
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    //

    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 100.0);
    camera.node.borrow_mut().position.set(1.0, 2.0, 3.0);

    let mut scene = Scene::new();
    // `new THREE.Fog( 0x0487e2, 7, 25 )` — linear fog, the same colour as the
    // background, so the box fades into it rather than into black.
    scene.fog = Some(Fog::new(Color::from_hex(0x0487e2), 7.0, 25.0).into());
    scene.set_background(Color::from_hex(0x0487e2));

    // `new THREE.TextureLoader().load( 'textures/crate.gif' )` — a real GIF89a,
    // 256×256, palette + LZW, which the browser decodes through
    // `createImageBitmap`. `generateMipmaps` stays true, so the upload is
    // followed by the eight mipmap blits the dump shows.
    let texture = TextureLoader::new()
        .load(three_rs::testing::three_js_dir().join("examples/textures/crate.gif"))
        .unwrap();
    texture.set_color_space(ColorSpace::SRGB);

    let geometry = box_geometry_default();
    let mut material = MeshBasicNodeMaterial::new();
    material.map = Some(texture);

    let mesh = Mesh::new(Rc::new(geometry), Some(material));
    scene.add(&mesh);

    // post processing

    let scene_pass = PassNode::new();

    let current_texture = scene_pass.texture_node(OUTPUT_ATTACHMENT);
    let previous_texture = scene_pass.previous_texture_node(OUTPUT_ATTACHMENT);

    let frame_diff = previous_texture.sub(current_texture.clone()).abs();

    let saturation_amount = luminance(frame_diff).mul(1000.0).clamp(0.0, 3.0);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(saturation(current_texture, saturation_amount));

    // `const controls = new OrbitControls( camera, renderer.domElement );` —
    // its `update()` points the camera at the target, the origin, and damping
    // leaves it there for the graded frame.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 2.0;
    controls.max_distance = 10.0;
    controls.enable_damping = true;
    controls.damping_factor = 0.01;

    App {
        // `const timer = new THREE.Timer();` — constructed in `init()`, as the
        // page does, so its `_startTime` is the moment the scene was built.
        timer: Timer::new(),
        renderer,
        scene,
        camera,
        controls,
        mesh,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `timer.update(); mesh.rotation.y += timer.getDelta() * 5 * params.speed`
    // — `params.speed` is 0 and the GUI that could raise it is not ported, so
    // the box never turns however long the clock runs.
    app.timer.update();

    // `controls.update();`
    let _ = app.controls.update(&mut app.camera, None);

    let delta = app.timer.get_delta();
    let speed = 0.0;
    {
        let mut mesh = app.mesh.borrow_mut();
        let rotation = mesh.rotation;
        mesh.set_rotation(rotation.x, rotation.y + delta * 5.0 * speed, rotation.z);
    }

    // `PassNode.updateBefore()`, then the output quad — see
    // `docs/postprocessing.md` for why the port fires the pass explicitly.
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` when the
/// page creates none — the signature is the same for every example so the
/// viewer and the browser shell can drive any of them through one call.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
///
/// They are two fields of the same `App`, so borrowing both is sound — but
/// only this module can say so; a host holding `&mut App` and calling
/// [`controls`] and then reaching for the camera cannot. Hence the pair.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so that the frame this writes is the frame the rung
    // grades no matter how long `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_difference.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
