//! Port of `three.js/examples/webgpu_water.html`, calling the three-rs API in
//! the same order the page's `init()` does.
//!
//! The page is in three.js' e2e exception list (`'webgpu_water', // 1 min`),
//! so it has no rung: it is gated by `tests/nodes_water_wgsl.rs` (the water's
//! WGSL against three's own) and `tests/water2_frames.rs` (frames on the GPU).
//! The constants below are still the harness's: `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws from `Math.random()`
//! not at all.
//!
//! # What this page is
//!
//! "The Night Pool" (`pool.glb`, Draco-compressed with WebP textures) at a
//! tenth of its size, with a [`Water2Mesh`] filling the basin 0.2 above the
//! floor and four dark grey planes around it. The water mixes a mirror of the
//! scene (a `reflector()`) with what is behind it on screen (a
//! `viewportSharedTexture()` read), weighted by a Fresnel term from two
//! scrolling normal maps. The light comes from `moonless_golf_2k.hdr.jpg`,
//! which is both the background and `scene.environment`. The frame goes
//! through a `RenderPipeline`: the scene pass writes `output` and `emissive`
//! (MRT), the emissive attachment is bloomed at strength 2 and added back,
//! `renderOutput()` tone-maps (ACES Filmic, exposure 0.5) and converts to
//! sRGB, and FXAA runs last — so the floodlight and the windows glow.
//!
//! # Order and loading
//!
//! The page starts `UltraHDRLoader.load()` without awaiting it, then awaits
//! the glTF and both normal maps. In the browser the HDR has landed before the
//! first frame (three's own screenshot has the sky), so the port loads it
//! synchronously at the same place in `init()`: the background is the cube
//! `CubeMapNode` converts the equirectangular texture into, the environment
//! its PMREM, as in `webgpu_tsl_angular_slicing`.
//!
//! The renderer is created first rather than after the floors because the
//! environment conversions take it.
//!
//! The water's mirror target is added to the water during the first render
//! ([`Water2Mesh`]'s docs), as upstream adds it inside the material's
//! `setup()`. The water's `flowConfig` advances by the frame's delta time,
//! which is 0 on the first frame, so the first frame shows `flowConfig =
//! (0, halfCycle, halfCycle)`.
//!
//! # Not ported
//!
//! `renderer.inspector = new Inspector()` and its "Water" parameters panel
//! draw nothing into the canvas. The panel's four controls are
//! [`set_color`], [`set_scale`], [`set_flow_x`] and [`set_flow_y`] here.

use std::cell::{RefCell, RefMut};
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::objects::{Water2Mesh, Water2MeshOptions};
use three_rs::geometries::plane_geometry;
use three_rs::loaders::{GltfLoader, TextureLoader, UltraHdrLoader};
use three_rs::materials::{render_output, Side};
use three_rs::nodes::display::{bloom, convert_to_texture, fxaa, BloomNode, FxaaNode, RttNode};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{emissive_color, output_property};
use three_rs::objects::{Background, Mesh};
use three_rs::renderer::cube_render_target;
use three_rs::textures::Wrapping;
use three_rs::{
    pass, Color, MeshStandardNodeMaterial, PassNode, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene, ToneMapping, Vector2,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params`: the GUI's values.
#[derive(Clone, Debug)]
pub struct Params {
    /// `'#99e0ff'`.
    pub color: u32,
    pub scale: f64,
    pub flow_x: f64,
    pub flow_y: f64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            color: 0x99e0ff,
            scale: 2.0,
            flow_x: 1.0,
            flow_y: 1.0,
        }
    }
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub controls: OrbitControls,
    pub water: Water2Mesh,
    pub params: Params,
    /// `scene.environment`'s PMREM of the HDR.
    pub environment: PmremEnvironment,
    pub scene_pass: PassNode,
    pub bloom_pass: BloomNode,
    /// The `RTTNode` `fxaa()` makes of `renderOutput( … )`.
    pub fxaa_input: RttNode,
    pub fxaa_pass: FxaaNode,
    pub render_pipeline: RenderPipeline,
}

fn examples_dir() -> PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_pixel_ratio(DPR);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 0.5;

    let mut scene = Scene::new();

    // `new UltraHDRLoader().load( 'textures/equirectangular/
    // moonless_golf_2k.hdr.jpg', … )` with `EquirectangularReflectionMapping`,
    // resolved synchronously — see the module docs.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/moonless_golf_2k.hdr.jpg"))
        .expect("moonless_golf_2k.hdr.jpg");
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // camera

    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 200.0);
    camera.node.borrow_mut().position.set(-20.0, 6.0, -30.0);

    // asset loading

    let gltf = GltfLoader::load(examples_dir().join("models/gltf/pool.glb")).expect("pool.glb");
    let texture_loader = TextureLoader::new();
    let normal_map0 = texture_loader
        .load(examples_dir().join("textures/water/Water_1_M_Normal.jpg"))
        .expect("Water_1_M_Normal.jpg");
    let normal_map1 = texture_loader
        .load(examples_dir().join("textures/water/Water_2_M_Normal.jpg"))
        .expect("Water_2_M_Normal.jpg");

    {
        let mut pool = gltf.scene.borrow_mut();
        pool.position.z = 2.0;
        pool.scale.set_scalar(0.1);
    }
    scene.add(&gltf.scene);

    // water

    normal_map0.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    normal_map1.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let water_geometry = Rc::new(plane_geometry(30.0, 40.0, 1, 1));

    let params = Params::default();
    let mut options = Water2MeshOptions::new(normal_map0, normal_map1);
    options.color = Color::from_hex(params.color);
    options.scale = params.scale;
    options.flow_direction = Vector2::new(params.flow_x, params.flow_y);
    let water = Water2Mesh::new(water_geometry, options);

    {
        let mut object = water.mesh.borrow_mut();
        object.position.set(0.0, 0.2, -2.0);
        // `water.rotation.x = Math.PI * - 0.5` — the port's `rotation` has no
        // `onChange` into the quaternion, so it is set through `set_rotation`.
        object.set_rotation(PI * -0.5, 0.0, 0.0);
        object.render_order = f64::INFINITY;
    }
    scene.add(&water.mesh);

    // floor

    let mut floor_geometry = plane_geometry(1.0, 1.0, 1, 1);
    floor_geometry.rotate_x(-PI * 0.5);
    let floor_geometry = Rc::new(floor_geometry);
    let mut floor_material =
        MeshStandardNodeMaterial::standard(Color::from_hex(0x444444), 1.0, 0.0);
    floor_material.side = Side::Double;

    for (position, scale) in [
        ([20.0, 0.0, 0.0], [15.0, 1.0, 80.0]),
        ([-20.0, 0.0, 0.0], [15.0, 1.0, 80.0]),
        ([0.0, 0.0, 30.0], [30.0, 1.0, 20.0]),
        ([0.0, 0.0, -30.0], [30.0, 1.0, 20.0]),
    ] {
        let floor = Mesh::new(floor_geometry.clone(), floor_material.clone());
        {
            let mut object = floor.borrow_mut();
            object.position.set(position[0], position[1], position[2]);
            object.scale.set(scale[0], scale[1], scale[2]);
        }
        scene.add(&floor);
    }

    // postprocessing

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_color_transform = false;

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());
    // `mrt( { output, emissive } )`.
    scene_pass.set_mrt(mrt(vec![
        ("output", output_property()),
        ("emissive", emissive_color()),
    ]));

    let beauty_pass = scene_pass.texture_node("output");
    let emissive_pass = scene_pass.texture_node("emissive");

    // `bloom( emissivePass, 2 )` — radius and threshold left at 0.
    let bloom_pass = bloom(emissive_pass);
    bloom_pass.strength.set(vec![2.0]);

    let output_pass = render_output(beauty_pass.add(bloom_pass.node()), renderer.tone_mapping);

    let fxaa_input = convert_to_texture(output_pass);
    let fxaa_pass = fxaa(&fxaa_input.texture());
    render_pipeline.output_node = Some(fxaa_pass.node());

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.target.set(0.0, 0.0, -5.0);
    controls.update(&mut camera.borrow_mut(), None);

    App {
        renderer,
        scene,
        camera,
        controls,
        water,
        params,
        environment,
        scene_pass,
        bloom_pass,
        fxaa_input,
        fxaa_pass,
        render_pipeline,
    }
}

/// The GUI's `color` control: `waterNode.color.value.set( value )`.
pub fn set_color(app: &mut App, color: u32) {
    app.params.color = color;
    let c = Color::from_hex(color);
    app.water.color.set(vec![c.r, c.g, c.b]);
}

/// The GUI's `scale` control (1 to 10): `waterNode.scale.value = value`.
pub fn set_scale(app: &mut App, scale: f64) {
    app.params.scale = scale;
    app.water.scale.set(vec![scale]);
}

/// The water's current `flowDirection` uniform.
fn flow_direction(app: &App) -> Vector2 {
    let value = app.water.flow_direction.get();
    Vector2::new(value[0], value[1])
}

/// The GUI's `flowX` control (-1 to 1): `waterNode.flowDirection.value.x =
/// value; waterNode.flowDirection.value.normalize()` — the other component
/// is the uniform's (already normalised) one, not `params.flowY`.
pub fn set_flow_x(app: &mut App, value: f64) {
    app.params.flow_x = value;
    let mut direction = flow_direction(app);
    direction.x = value;
    direction.normalize();
    app.water.flow_direction.set(vec![direction.x, direction.y]);
}

/// The GUI's `flowY` control, as [`set_flow_x`].
pub fn set_flow_y(app: &mut App, value: f64) {
    app.params.flow_y = value;
    let mut direction = flow_direction(app);
    direction.y = value;
    direction.normalize();
    app.water.flow_direction.set(vec![direction.x, direction.y]);
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    // The environment's PMREM is (re)built on demand, as the renderer does
    // for `scene.environment = texture`.
    app.environment.update(&mut app.renderer).unwrap();

    // `FXAANode` is not itself a `NodeUpdate` node, so its `invSize` uniform
    // and its input's size are set by hand — `webgpu_postprocessing_fxaa`.
    let (width, height) = app.renderer.drawing_buffer_size();
    app.fxaa_input.set_size(width, height);
    app.fxaa_pass.update();

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once — see `webgpu_ocean`'s copy.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_water.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
