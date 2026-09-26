//! Port of `three.js/examples/webgpu_lines_fat.html`, calling the three-rs API
//! in the same order the page's `init()` / `onWindowResize()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. `insetWidth` and `insetHeight` are
//! `innerHeight / 4` — 125 each.
//!
//! # The two things this frame is really testing
//!
//! **The fat line itself.** 64 `hilbert3D` corners become a 768-point
//! `CatmullRomCurve3` sample, which `LineGeometry` turns into 767 segments.
//! Each segment is one instance of an eight-vertex quad that the vertex shader
//! expands into a screen-space ribbon 5 pixels wide, with round caps cut by a
//! `discard` in the fragment stage. See [`three_rs::addons::lines`] and
//! [`three_rs::materials::line2`].
//!
//! **The inset.** The page renders the scene twice: once full-frame with the
//! main camera, then again into a 125x125 box at the bottom left with a second
//! camera that copies the first's position and orientation, after
//! `clearDepth()` and with `autoClear` off so the main frame survives
//! underneath. The inset is about 15 600 pixels of the 400 000 the grader
//! compares — 39 times its whole 0.1% budget — so getting the rectangle wrong
//! by so much as a scanline fails the rung outright.
//!
//! `posY` is `innerHeight - insetHeight - 20`, i.e. 355, and it goes straight
//! into `setViewport`. three.js' `setViewport` takes **bottom-left** origin
//! coordinates, so 355 puts the box 355 pixels up from the bottom of a
//! 500-tall frame — 20 pixels from the *top*. The page's own comment
//! ("`window.innerHeight - insetHeight - 20`") reads as if it meant the
//! bottom, and the rendered page shows it at the top left. There is no y-flip
//! anywhere in the port; see `docs/webgpu_lines_fat-progress.md`.
//!
//! # Not ported, deliberately
//!
//! * `line.computeLineDistances()` and the `dashed` GUI branch — the material
//!   has `dashed: false` and nothing reads `instanceDistanceStart`.
//! * `Stats` and the GUI.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::geometry_utils::hilbert_3d_default;
use three_rs::addons::lines::{Line2, LineGeometry};
use three_rs::core::BufferAttribute;
use three_rs::math::ColorSpace;
use three_rs::{
    Background, BufferGeometry, CatmullRomCurve3, Color, Curve, Line, Line2NodeMaterial,
    LineBasicNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `insetWidth = insetHeight = window.innerHeight / 4` — a square. The page
/// writes the pair in `onWindowResize()`, which `init()` calls once before the
/// first frame; this is that first value, and [`resize`] recomputes it.
pub const INSET: f64 = INNER_HEIGHT / 4.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub camera2: PerspectiveCamera,
    pub background_node: three_rs::nodes::NodeRef,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// `window.innerWidth` / `window.innerHeight`, which `animate()` reads for
    /// the main viewport and for the inset's `posY` just as `onWindowResize()`
    /// does. A page gets them from the window; an example that owns its own
    /// resize has to remember them.
    pub inner_width: f64,
    pub inner_height: f64,
    /// The page's module-level `insetWidth` / `insetHeight`, written by
    /// `onWindowResize()` and read by `animate()`.
    pub inset_width: f64,
    pub inset_height: f64,
}

pub fn init() -> App {
    // `new THREE.WebGPURenderer( { antialias: true } )` — four samples. It has
    // teeth here: the fat line is nothing *but* edges, and a single-sampled
    // frame differs from three's along every one of them (1415 pixels, 14x the
    // budget, when this was first run with `antialias: false`).
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_clear_color(Color::from_hex(0x000000), 1.0);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(-40.0, 0.0, 60.0);

    // `camera2.position.copy( camera.position )`, and `onWindowResize()` then
    // sets its aspect to `insetWidth / insetHeight` — 1.
    let camera2 = PerspectiveCamera::new(40.0, 1.0, 1.0, 1000.0);
    camera2.node.borrow_mut().position.set(-40.0, 0.0, 60.0);

    // `const controls = new OrbitControls( camera, renderer.domElement );`
    // The constructor's own `update()` aims the camera at the default
    // `( 0, 0, 0 )` target from where the line above put it; `animate()`'s
    // `controls.update()` then repeats that every frame, damping and all,
    // and with no pointer input moves nothing.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 10.0;
    controls.max_distance = 500.0;

    // `backgroundNode = color( 0x222222 )` — assigned to `scene.backgroundNode`
    // for the inset render only, so the inset's own background paints over the
    // main frame inside the scissor box.
    let background_node: three_rs::nodes::NodeRef = Color::from_hex(0x222222).into();

    // Position and Color Data
    let points = hilbert_3d_default(Vector3::new(0.0, 0.0, 0.0), 20.0, 1);
    let spline = CatmullRomCurve3::new(points.clone());
    // `Math.round( 12 * points.length )` — 12 * 64.
    let divisions = (12.0 * points.len() as f64).round() as usize;

    let mut positions: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut colors: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut line_color = Color::default();
    for i in 0..divisions {
        let t = i as f64 / divisions as f64;
        let point = spline.get_point(t);
        positions.push(point.x as f32);
        positions.push(point.y as f32);
        positions.push(point.z as f32);

        // `lineColor.setHSL( t, 1.0, 0.5, THREE.SRGBColorSpace )` — the sRGB
        // overload, so the hsl triple is decoded into the working space.
        line_color.set_hsl(t, 1.0, 0.5, ColorSpace::SRGB);
        colors.push(line_color.r as f32);
        colors.push(line_color.g as f32);
        colors.push(line_color.b as f32);
    }

    // Line2 ( LineGeometry, Line2NodeMaterial )
    let mut geometry = LineGeometry::new();
    geometry.set_positions(&positions);
    geometry.set_colors(&colors);

    let mut mat_line = Line2NodeMaterial::line2(Color::from_hex(0xffffff));
    // "in world units with size attenuation, pixels otherwise" — and world
    // units are off, so this is 5 screen pixels.
    mat_line.linewidth = 5.0;
    mat_line.vertex_colors = true;
    mat_line.alpha_to_coverage = false;

    let line = Line2::new(&geometry, mat_line);
    // `line.computeLineDistances()` is skipped: `dashed` is false.
    // `line.scale.set( 1, 1, 1 )` is the identity.
    scene.add(&line);

    // Line ( BufferGeometry, LineBasicNodeMaterial ) — `line-strip`. The GUI's
    // "line type" switches to it; on the graded frame it is invisible, and it
    // is here because the render list still walks it.
    let mut geo = BufferGeometry::new();
    geo.set_attribute("position", BufferAttribute::new(positions.clone(), 3));
    geo.set_attribute("color", BufferAttribute::new(colors.clone(), 3));

    let mut mat_line_basic = LineBasicNodeMaterial::line(Color::from_hex(0xffffff));
    mat_line_basic.vertex_colors = true;

    let line1 = Line::new(Rc::new(geo), mat_line_basic);
    line1.borrow_mut().visible = false;
    scene.add(&line1);

    App {
        renderer,
        scene,
        camera,
        camera2,
        background_node,
        controls,
        // The page ends `init()` with a bare `onWindowResize()` call, which is
        // where `insetWidth` / `insetHeight` and `camera2.aspect` — 1, the
        // value it was constructed with — first get written.
        inner_width: INNER_WIDTH,
        inner_height: INNER_HEIGHT,
        inset_width: INSET,
        inset_height: INSET,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // --- main scene
    app.renderer.set_clear_color(Color::from_hex(0x000000), 1.0);
    app.renderer
        .set_viewport(0.0, 0.0, app.inner_width, app.inner_height);

    // `controls.update();`
    app.controls.update(&mut app.camera, None);

    app.renderer.auto_clear = true;
    app.scene.background = None;
    app.renderer.render(&mut app.scene, &mut app.camera);

    // --- inset scene
    let pos_y = app.inner_height - app.inset_height - 20.0;
    // "important!" — without it the inset's geometry loses the depth test
    // against the main frame it is drawing over.
    app.renderer.clear_depth();
    app.renderer.set_scissor_test(true);
    app.renderer
        .set_scissor(20.0, pos_y, app.inset_width, app.inset_height);
    app.renderer
        .set_viewport(20.0, pos_y, app.inset_width, app.inset_height);

    // `camera2.position.copy( camera.position ); camera2.quaternion.copy(
    // camera.quaternion );` — after `controls.update()`, so it picks up the
    // orientation `lookAt` just wrote.
    {
        let camera = app.camera.node.borrow();
        let mut camera2 = app.camera2.node.borrow_mut();
        camera2.position = camera.position;
        camera2.quaternion = camera.quaternion;
    }

    app.renderer.auto_clear = false;
    app.scene.background = Some(Background::Node(app.background_node.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera2);

    app.renderer.set_scissor_test(false);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
///
/// This one does more than the usual three lines: it also rewrites the
/// page's module-level `insetWidth` / `insetHeight` and the inset camera's
/// aspect, which is why [`App`] carries them as fields rather than consts.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.inner_width = width;
    app.inner_height = height;

    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);

    // `insetWidth = window.innerHeight / 4; // square`
    app.inset_width = height / 4.0;
    app.inset_height = height / 4.0;

    app.camera2.aspect = app.inset_width / app.inset_height;
    app.camera2.update_projection_matrix();
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
        .unwrap_or_else(|| "target/webgpu_lines_fat.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
