//! Port of `three.js/examples/webgpu_postprocessing_godrays.html`, calling the
//! three-rs API in the same order the page's top-level script does.
//!
//! A shadow-casting point light hangs between concrete pillars; `godrays()`
//! ray-marches its cube shadow map at half resolution, `bilateralBlur()`
//! softens the march without bleeding across edges, and `depthAwareBlend()`
//! lays the rays over the beauty pass in the light's colour.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The page's GUI (raymarch steps, density, max density, distance
//! attenuation, edge radius and strength, the blur toggle) is the inspector
//! panel, which the port does not have; every value it drives is a public
//! uniform on [`App`], and `output_raw` is the graph the blur toggle swaps in.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::loaders::GltfLoader;
use three_rs::materials::Side;
use three_rs::nodes::display::{
    bilateral_blur, depth_aware_blend, godrays, BilateralBlurNode, DepthAwareBlendOptions,
    GodraysNode,
};
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::uniform_settable;
use three_rs::nodes::{NodeRef, Type};
use three_rs::{
    pass, plane_geometry, sphere_geometry, AmbientLight, Color, Mesh, MeshBasicNodeMaterial,
    MeshStandardNodeMaterial, PassNode, PerspectiveCamera, PointLight, RenderPipeline, Renderer,
    RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass` and the godrays and blend nodes.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub controls: OrbitControls,
    pub point_light: Node,
    pub scene_pass: PassNode,
    pub godrays_pass: GodraysNode,
    pub blur_pass: BilateralBlurNode,
    /// The composite folder's `edgeRadius`.
    pub edge_radius: SettableValue,
    /// The composite folder's `edgeStrength`.
    pub edge_strength: SettableValue,
    /// `outputBlurred`, the pipeline's output while `params.enabledBlur`.
    pub output_blurred: NodeRef,
    /// `outputRaw`, the output with the blur disabled.
    pub output_raw: NodeRef,
    pub render_pipeline: RenderPipeline,
}

/// `setupBackdrop()`: five black, double-sided walls 200 units from the
/// origin, so the rays have something to end on.
fn setup_backdrop(scene: &Scene) {
    let backdrop_distance = 200.0;
    let backdrop_geometry = Rc::new(plane_geometry(400.0, 200.0, 1, 1));
    let mut backdrop_material = MeshBasicNodeMaterial::new();
    backdrop_material.color = Color::from_hex(0x000000);
    backdrop_material.side = Side::Double;

    let wall = |x: f64, y: f64, z: f64| {
        let mesh = Mesh::new(backdrop_geometry.clone(), backdrop_material.clone());
        mesh.borrow_mut().position.set(x, y, z);
        mesh
    };

    let backdrop_left = wall(-backdrop_distance, 100.0, 0.0);
    backdrop_left.borrow_mut().rotate_y(PI / 2.0);
    scene.add(&backdrop_left);

    let backdrop_right = wall(backdrop_distance, 100.0, 0.0);
    backdrop_right.borrow_mut().rotate_y(PI / 2.0);
    scene.add(&backdrop_right);

    let backdrop_front = wall(0.0, 100.0, -backdrop_distance);
    scene.add(&backdrop_front);

    let backdrop_back = wall(0.0, 100.0, backdrop_distance);
    scene.add(&backdrop_back);

    let backdrop_top = wall(0.0, 200.0, 0.0);
    {
        let mut object = backdrop_top.borrow_mut();
        object.rotate_x(PI / 2.0);
        object.scale.set(3.0, 6.0, 1.0);
    }
    scene.add(&backdrop_top);
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 1000.0);
    camera.node.borrow_mut().position.set(-175.0, 50.0, 0.0);

    let mut scene = Scene::new();
    scene.background = Some(three_rs::objects::Background::Color(Color::from_hex(
        0x000000,
    )));

    // asset

    // `new GLTFLoader().loadAsync( 'models/gltf/godrays_demo.glb' )`: a
    // floor slab (`base`) and the pillars (`concrete`).
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/godrays_demo.glb"))
        .expect("godrays_demo.glb");
    scene.add(&gltf.scene);

    let pillars = gltf
        .scene
        .get_object_by_name("concrete")
        .expect("the pillars");
    pillars.borrow_mut().mesh_mut().unwrap().material = Some(MeshStandardNodeMaterial::standard(
        Color::from_hex(0x333333),
        1.0,
        0.0,
    ));

    let base = gltf.scene.get_object_by_name("base").expect("the floor");
    let mut base_material = MeshStandardNodeMaterial::standard(Color::from_hex(0x333333), 1.0, 0.0);
    base_material.side = Side::Double;
    base.borrow_mut().mesh_mut().unwrap().material = Some(base_material);

    // lights

    let light_pos = Vector3::new(0.0, 50.0, 0.0);
    let mut light_sphere_material = MeshBasicNodeMaterial::new();
    light_sphere_material.color = Color::from_hex(0xffffff);
    let light_sphere = Mesh::new(Rc::new(sphere_geometry(0.5, 16, 16)), light_sphere_material);
    light_sphere.borrow_mut().position = light_pos;
    scene.add(&light_sphere);

    scene.add(&AmbientLight::new(Color::from_hex(0xcccccc), 0.4));

    let point_light = PointLight::new(Color::from_hex(0xf6287d), 10000.0, 0.0);
    {
        let mut object = point_light.borrow_mut();
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.bias = -0.00001;
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        object.position = light_pos;
    }
    scene.add(&point_light);

    setup_backdrop(&scene);

    // shadow setup

    scene.node.traverse(&mut |node| {
        let mut object = node.borrow_mut();
        if object.mesh().is_some() {
            object.cast_shadow = true;
            object.receive_shadow = true;
        }
    });

    {
        let mut object = light_sphere.borrow_mut();
        object.cast_shadow = false;
        object.receive_shadow = false;
    }

    // renderer

    // `new THREE.WebGPURenderer()` — no `antialias`.
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    // post processing

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let mut render_pipeline = RenderPipeline::new();

    // beauty

    let scene_pass = pass(scene.clone(), camera.clone());
    let scene_pass_color = scene_pass.texture();
    let scene_pass_depth = scene_pass.depth_texture();
    let _ = scene_pass.texture_node("output");
    let _ = scene_pass.texture_node("depth");

    // godrays

    let godrays_pass = godrays(&scene_pass_depth, camera.clone(), &point_light);
    let godrays_pass_color = godrays_pass.texture();

    // blur

    // `bilateralBlur( godraysPassColor )` — direction `vec2( 1 )`, sigma 4,
    // sigmaColor 0.1.
    let blur_pass = bilateral_blur(&godrays_pass_color, None, 4, 0.1);
    let blur_pass_color = blur_pass.texture();

    // composite

    // `uniform( color( 0xf6287d ) )`, `uniform( int( 2 ) )` and `uniform(
    // float( 2 ) )` — the second declared `f32` by three's WGSL.
    let blend = Color::from_hex(0xf6287d);
    let (blend_color, _) = uniform_settable(Type::Vec3, vec![blend.r, blend.g, blend.b]);
    let (edge_radius_node, edge_radius) = uniform_settable(Type::F32, vec![2.0]);
    let (edge_strength_node, edge_strength) = uniform_settable(Type::F32, vec![2.0]);
    let mut options = DepthAwareBlendOptions::default();
    options.blend_color = blend_color;
    options.edge_radius = edge_radius_node;
    options.edge_strength = edge_strength_node;

    let output_blurred = depth_aware_blend(
        &scene_pass_color,
        &blur_pass_color,
        &scene_pass_depth,
        &camera,
        options.clone(),
    );
    let output_raw = depth_aware_blend(
        &scene_pass_color,
        &godrays_pass_color,
        &scene_pass_depth,
        &camera,
        options,
    );

    render_pipeline.output_node = Some(output_blurred.clone());

    // `new OrbitControls( camera, renderer.domElement )` with `target.set( 0,
    // 0.5, 0 )`, damping and one `update()`.
    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 0.5, 0.0);
    controls.enable_damping = true;
    controls.max_distance = 200.0;
    controls.update(&mut camera.borrow_mut(), None);

    App {
        renderer,
        scene,
        camera,
        controls,
        point_light,
        scene_pass,
        godrays_pass,
        blur_pass,
        edge_radius,
        edge_strength,
        output_blurred,
        output_raw,
        render_pipeline,
    }
}

/// The blur folder's `enabled` toggle: `renderPipeline.outputNode =
/// outputBlurred` or `outputRaw`. The port's pipeline rebuilds its quad when
/// the output node changes, which is what three's `needsUpdate = true` asks.
pub fn set_blur_enabled(app: &mut App, enabled: bool) {
    app.render_pipeline.output_node = Some(if enabled {
        app.output_blurred.clone()
    } else {
        app.output_raw.clone()
    });
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    // `renderPipeline.render()`: the output quad samples the blur's texture,
    // whose update renders the godrays target, whose update renders the
    // scene pass (and with it the point light's cube shadow map).
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

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_godrays.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
