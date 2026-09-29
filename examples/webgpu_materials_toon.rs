//! Port of `three.js/examples/webgpu_materials_toon.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `Date.now()` is pinned to 0.
//!
//! A cube of `MeshToonNodeMaterial` spheres — hue along x, brightness along z,
//! a darkening along y — each with a `RedFormat` `DataTexture` gradient ramp
//! whose step count grows with the hue index, lit by an ambient light and a
//! point light riding a small white sphere. The whole scene goes through
//! `toonOutlinePass()`, which draws every toon object a second time, back
//! faces pushed out along the clip-space normal, as a black outline.
//!
//! The font arrives through `FontLoader.load()`'s callback, which calls
//! `init()`; the grader waits for the network to settle first, so here the
//! load is synchronous and comes first.
//!
//! `Math.random` is never called. `new Inspector()` only registers the
//! renderer with the inspector panel; the `resize` listener never fires.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::text_geometry::{text_geometry, TextGeometryOptions};
use three_rs::geometries::sphere_geometry;
use three_rs::lights::{AmbientLight, PointLight};
use three_rs::loaders::Font;
use three_rs::loaders::FontLoader;
use three_rs::nodes::display::{toon_outline_pass, ToonOutlinePassNode};
use three_rs::textures::Texture;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, MeshToonNodeMaterial, PerspectiveCamera, RenderPipeline,
    Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    pub render_pipeline: RenderPipeline,
    /// `renderPipeline.outputNode = toonOutlinePass( scene, camera )` — the
    /// pass is fired explicitly before the pipeline, as every ported pass is
    /// (`docs/postprocessing.md`).
    pub scene_pass: ToonOutlinePassNode,
    pub particle_light: three_rs::Node,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    // `loader.load( 'fonts/gentilis_regular.typeface.json', function ( font )
    // { init( font ); } )`.
    let font = FontLoader::new()
        .load(examples_dir().join("fonts/gentilis_regular.typeface.json"))
        .unwrap();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 2500.0);
    camera
        .node
        .borrow_mut()
        .position
        .set(0.0, 400.0, 400.0 * 3.5);

    //

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x444488));

    //

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //

    let scene_pass = toon_outline_pass();
    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass.node());

    // Materials

    let cube_width = 400.0;
    let number_of_spheres_per_side = 5.0;
    let sphere_radius = (cube_width / number_of_spheres_per_side) * 0.8 * 0.5;
    let step_size = 1.0 / number_of_spheres_per_side;

    let geometry = Rc::new(sphere_geometry(sphere_radius, 32, 16));

    // The loops accumulate `stepSize` in doubles exactly as the page's `for`
    // statements do, so the last step lands wherever JS's lands.
    let mut alpha = 0.0;
    let mut alpha_index = 0usize;
    while alpha <= 1.0 {
        let gradient_map = gradient_map(alpha_index);

        let mut beta = 0.0;
        while beta <= 1.0 {
            let mut gamma = 0.0;
            while gamma <= 1.0 {
                // basic monochromatic energy preservation
                let mut diffuse_color = Color::new(0.0, 0.0, 0.0);
                diffuse_color
                    .set_hsl(
                        alpha,
                        0.5,
                        gamma * 0.5 + 0.1,
                        three_rs::math::ColorSpace::LinearSrgb,
                    )
                    .multiply_scalar(1.0 - beta * 0.2);

                let material =
                    MeshToonNodeMaterial::toon(diffuse_color, Some(gradient_map.clone()));

                let mesh = Mesh::new(geometry.clone(), material);
                {
                    let mut mesh = mesh.borrow_mut();
                    mesh.position.x = alpha * 400.0 - 200.0;
                    mesh.position.y = beta * 400.0 - 200.0;
                    mesh.position.z = gamma * 400.0 - 200.0;
                }

                scene.add(&mesh);

                gamma += step_size;
            }
            beta += step_size;
        }
        alpha += step_size;
        alpha_index += 1;
    }

    add_label(
        &scene,
        &font,
        "-gradientMap",
        Vector3::new(-350.0, 0.0, 0.0),
    );
    add_label(&scene, &font, "+gradientMap", Vector3::new(350.0, 0.0, 0.0));

    add_label(&scene, &font, "-diffuse", Vector3::new(0.0, 0.0, -300.0));
    add_label(&scene, &font, "+diffuse", Vector3::new(0.0, 0.0, 300.0));

    let particle_light = Mesh::new(
        Rc::new(sphere_geometry(4.0, 8, 8)),
        MeshBasicNodeMaterial {
            color: Color::from_hex(0xffffff),
            ..MeshBasicNodeMaterial::default()
        },
    );
    scene.add(&particle_light);

    // Lights

    scene.add(&AmbientLight::new(Color::from_hex(0xc1c1c1), 3.0));

    // `new THREE.PointLight( 0xffffff, 2, 800, 0 )` — no decay, so inside its
    // 800-unit cutoff the light only falls off through the window term.
    let point_light = PointLight::new(Color::from_hex(0xffffff), 2.0, 800.0);
    point_light
        .borrow_mut()
        .light_mut()
        .expect("three-rs: a PointLight is a light")
        .decay = 0.0;
    particle_light.add(&point_light);

    //

    let mut controls = OrbitControls::new(&mut camera);
    // The renderer's canvas stands in for the element's `clientWidth` /
    // `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 200.0;
    controls.max_distance = 2000.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        render_pipeline,
        scene_pass,
        particle_light,
    }
}

/// One hue column's ramp:
///
/// ```js
/// const colors = new Uint8Array( alphaIndex + 2 );
/// for ( let c = 0; c <= colors.length; c ++ ) colors[ c ] = ( c / colors.length ) * 256;
/// const gradientMap = new THREE.DataTexture( colors, colors.length, 1, THREE.RedFormat );
/// ```
///
/// The loop runs one past the end; a typed-array store out of bounds is
/// dropped. Every store in bounds is below 256, so `ToUint8`'s wrap never
/// applies and it is a plain truncation.
fn gradient_map(alpha_index: usize) -> Texture {
    let len = alpha_index + 2;
    let colors: Vec<u8> = (0..len)
        .map(|c| ((c as f64 / len as f64) * 256.0) as u8)
        .collect();
    let map = Texture::data_r8(len as u32, 1, &colors);
    map.set_needs_update();
    map
}

/// The page's `addLabel( name, location )`.
fn add_label(scene: &Scene, font: &Font, name: &str, location: Vector3) {
    let mut parameters = TextGeometryOptions::new();
    parameters.size = 20.0;
    parameters.extrude.depth = 1.0;
    parameters.extrude.curve_segments = 1;
    let text_geo = text_geometry(name, font, &parameters);

    let text_material = MeshBasicNodeMaterial::new();
    let text_mesh = Mesh::new(Rc::new(text_geo), text_material);
    text_mesh.borrow_mut().position = location;
    scene.add(&text_mesh);
}

/// The page's `render()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    // `Date.now() * 0.00025`.
    let timer = three_rs::utils::date_now_ms() * 0.00025;

    {
        let mut light = app.particle_light.borrow_mut();
        light.position.x = (timer * 7.0).sin() * 300.0;
        light.position.y = (timer * 5.0).cos() * 400.0;
        light.position.z = (timer * 3.0).cos() * 300.0;
    }

    // `renderPipeline.render()`: the outline pass's `updateBefore()`, then the
    // output quad.
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
        .unwrap_or_else(|| "target/webgpu_materials_toon.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
