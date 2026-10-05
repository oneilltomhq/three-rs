//! Port of `three.js/examples/webgpu_multiple_rendertargets.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! # What the page shows
//!
//! A hardwood-textured torus knot drawn once into a two-attachment
//! `RenderTarget` — `output` (the material's colour) and `normal`
//! (`normalWorld`) — and a `RenderPipeline` that shows the first on the left
//! half of the screen and the second on the right, split by `step( 0.5,
//! screenUV.x )`.
//!
//! Unlike `webgpu_mrt`, there is no `pass()` node: the page builds the target
//! itself (`{ count: 2 }`), names its textures, and sets the MRT on the
//! *renderer* (`renderer.setMRT()`), once, in `init()`. Both attachments are
//! the target's default `UnsignedByteType` (`rgba8unorm`) and single-sampled —
//! `antialias` does not reach a user render target — and `NearestFilter` on
//! both sides, so the composite reads them with `textureLoad`. The normal is
//! written as `vec4( normalWorld, 1.0 )` and lands in `rgba8unorm` clamped to
//! `[ 0, 1 ]`, which is why the right half is black wherever a component is
//! negative. See `docs/nodes.md` §52.
//!
//! `performance.now()` and the RAF's `time` are 0 under the harness, so the
//! knot's `rotation.y` is 0 for the graded frame.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::torus_knot_geometry;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    mix, normal_world, output_property, screen_uv, step, texture, texture_uv, uv, vec2,
};
use three_rs::renderer::RenderTargetOptions;
use three_rs::textures::Wrapping;
use three_rs::utils::now_ms;
use three_rs::{
    Color, ColorSpace, Mesh, MeshBasicNodeMaterial, ObjectRef, PerspectiveCamera, RenderPipeline,
    RenderTarget, Renderer, RendererParameters, Scene, TextureFilter, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `window.devicePixelRatio`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub torus: ObjectRef,
    pub render_pipeline: RenderPipeline,
    pub render_target: RenderTarget,
    /// The page's anonymous `new OrbitControls( camera, renderer.domElement )`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // Create a multi render target with Float buffers

    // `new THREE.RenderTarget( w * dpr, h * dpr, { count: 2, minFilter:
    // NearestFilter, magFilter: NearestFilter } )`. The comment is the page's;
    // the type is the default `UnsignedByteType`, so both are `rgba8unorm`.
    let mut options = RenderTargetOptions::default();
    options.min_filter = TextureFilter::Nearest;
    options.mag_filter = TextureFilter::Nearest;
    let render_target = RenderTarget::new_with_options(
        (INNER_WIDTH * DPR) as u32,
        (INNER_HEIGHT * DPR) as u32,
        options,
    )
    .unwrap();
    render_target.set_count(2);

    // Name our G-Buffer attachments for debugging

    render_target.set_texture_name(0, "output");
    render_target.set_texture_name(1, "normal");

    // Scene

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x222222));

    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 50.0);
    camera.node.borrow_mut().position.z = 4.0;

    let diffuse = TextureLoader::new()
        .load(examples_dir().join("textures/hardwood2_diffuse.jpg"))
        .unwrap();
    diffuse.set_color_space(ColorSpace::Srgb);
    diffuse.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // `new THREE.NodeMaterial()`. With a `colorNode` and no lighting model
    // the port's unlit material generates the same program, statement for
    // statement against three's `m02` (as in `webgpu_textures_2d-array_compressed`).
    let mut torus_material = MeshBasicNodeMaterial::new();
    torus_material.color_node = Some(texture_uv(&diffuse, uv().mul(vec2(10.0, 4.0))));

    let torus = Mesh::new(
        Rc::new(torus_knot_geometry(1.0, 0.3, 128, 32, 2.0, 3.0)),
        torus_material,
    );
    scene.add(&torus);

    // MRT

    // `renderer.setMRT( mrt( { output, normal: normalWorld } ) )`.
    // `normalWorld` is a node object upstream whose `setup()` runs per
    // material, so it is handed over as a closure (`docs/nodes.md` §23).
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("normal", normal_world);
    renderer.set_mrt(Some(scene_mrt));

    // Post Processing

    let textures = render_target.textures();
    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(mix(
        texture(&textures[0]),
        texture(&textures[1]),
        step(0.5, screen_uv().x()),
    ));

    // Controls

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        torus,
        render_pipeline,
        render_target,
        controls,
    }
}

/// The page's `render( time )`.
pub fn animate(app: &mut App) {
    // `time` is the RAF timestamp — `performance.now()`.
    let time = now_ms();
    {
        let mut torus = app.torus.borrow_mut();
        let rotation = torus.rotation;
        torus.set_rotation(rotation.x, (time / 1000.0) * 0.4, rotation.z);
    }

    // render scene into target
    app.renderer
        .set_render_target(Some(app.render_target.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera);

    // render post FX
    app.renderer.set_render_target(None);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);

    // `const dpr = renderer.getPixelRatio();` — the ratio `init()` set.
    app.render_target
        .set_size((width * DPR) as u32, (height * DPR) as u32);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
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
        .unwrap_or_else(|| "target/webgpu_multiple_rendertargets.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
