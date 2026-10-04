//! Port of `three.js/examples/webgpu_postprocessing_outline.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and the harness pins `time` at 0.
//!
//! Twenty spheres, a torus, a floor and the `tree.obj` model (through
//! `ObjLoader`) under a shadow-casting directional light, rendered by one
//! `pass( scene, camera )` with `OutlineNode`'s outline added on top:
//! `visibleEdge * visibleEdgeColor + hiddenEdge * hiddenEdgeColor`, times
//! `edgeStrength`, pulsed by `oscSine()` when `pulsePeriod` is above 0.
//!
//! The outline follows the pointer: `onPointerMove()` raycasts the scene and
//! selects the nearest hit ([`pointer_move`]). The graded frame has no
//! pointer, so `selectedObjects` is the empty array the page starts with,
//! `OutlineNode.updateBefore()` returns before drawing anything, and the
//! composite the output reads is the never-rendered 1x1 target — the outline
//! adds 0 and the frame is the lit scene.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]). `new Inspector()` runs before
//! the sphere loop and draws five times (`webgpu_tsl_galaxy`'s
//! `INSPECTOR_RANDOM_DRAWS`); each sphere then takes five draws — hue, x, y,
//! z, scale — in that order.
//!
//! The page loads the tree asynchronously, after `init()` has built the rest
//! of the scene; the port loads it synchronously at the same point in
//! `init()`, so it hangs under `obj3d` exactly as the callback leaves it.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::{Node, Object3D};
use three_rs::geometries::{plane_geometry, sphere_geometry, torus_geometry};
use three_rs::loaders::ObjLoader;
use three_rs::materials::{MeshBasicNodeMaterial, Side};
use three_rs::math::{ColorSpace, Vector2};
use three_rs::nodes::display::{outline, OutlineNode, OutlineParams};
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::{float, osc_sine, time, uniform_settable};
use three_rs::nodes::{NodeRef, Type};
use three_rs::testing::DeterministicRandom;
use three_rs::{
    pass, AmbientLight, Color, DirectionalLight, Group, Mesh, PassNode, PerspectiveCamera,
    Raycaster, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes before the sphere loop — see
/// `webgpu_tsl_galaxy`'s `INSPECTOR_RANDOM_DRAWS`.
const INSPECTOR_RANDOM_DRAWS: usize = 5;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's GUI-bound uniforms.
pub struct Uniforms {
    /// `edgeStrength`, `uniform( 3.0 )`.
    pub edge_strength: SettableValue,
    /// `edgeGlow`, `uniform( 0.0 )`.
    pub edge_glow: SettableValue,
    /// `edgeThickness`, `uniform( 1.0 )`.
    pub edge_thickness: SettableValue,
    /// `pulsePeriod`, `uniform( 0 )`.
    pub pulse_period: SettableValue,
    /// `visibleEdgeColor`, `uniform( new THREE.Color( 0xffffff ) )`.
    pub visible_edge_color: SettableValue,
    /// `hiddenEdgeColor`, `uniform( new THREE.Color( 0x4e3636 ) )`.
    pub hidden_edge_color: SettableValue,
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass` and `outline_pass`, which render it from
    /// `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass` and `outline_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's `group`: everything but the lights.
    pub group: Node,
    pub uniforms: Uniforms,
    /// The page's `outlinePass`.
    pub outline_pass: OutlineNode,
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
    /// The page's `raycaster`.
    pub raycaster: Raycaster,
    /// The page's `mouse`.
    pub mouse: Vector2,
}

/// A colour uniform: `uniform( new THREE.Color( hex ) )`, linear-sRGB.
fn color_uniform(hex: u32) -> (NodeRef, SettableValue) {
    let c = Color::from_hex(hex);
    uniform_settable(Type::Vec3, vec![c.r, c.g, c.b])
}

/// `loader.load( 'models/obj/tree.obj', function ( object ) { … } )`'s
/// callback: centre the geometry, swap in a Phong material, scale the model
/// to its bounding sphere and raise it.
fn load_tree() -> Node {
    let object = ObjLoader::new()
        .load(examples_dir().join("models/obj/tree.obj"))
        .expect("tree.obj")
        .group;

    let mut scale = 1.0;

    object.traverse(&mut |child| {
        let mut child = child.borrow_mut();
        let Some(mesh) = child.mesh_mut() else {
            return;
        };

        let geometry = Rc::make_mut(&mut mesh.geometry);
        geometry.center();
        let bounding_sphere = geometry
            .compute_bounding_sphere()
            .expect("tree.obj has positions");
        scale = 0.2 * bounding_sphere.radius;

        let mut phong_material = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
        phong_material.specular = Color::from_hex(0x111111);
        phong_material.shininess = 5.0;
        mesh.material = Some(phong_material);
        mesh.materials.clear();
        child.receive_shadow = true;
        child.cast_shadow = true;
    });

    {
        let mut object = object.borrow_mut();
        object.position.y = 1.0;
        object.scale.divide_scalar(scale);
    }
    object
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    let width = INNER_WIDTH;
    let height = INNER_HEIGHT;

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(width, height);
    // `renderer.inspector = new Inspector()`.
    random.skip(INSPECTOR_RANDOM_DRAWS);
    renderer.shadow_map_enabled = true;

    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(45.0, width / height, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 8.0);

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(width, height);
    controls.min_distance = 5.0;
    controls.max_distance = 20.0;
    controls.enable_pan = false;
    controls.enable_damping = true;
    controls.damping_factor = 0.05;

    //

    scene.add(&AmbientLight::new(Color::from_hex(0xaaaaaa), 0.6));

    let light = DirectionalLight::new(Color::from_hex(0xddffdd), 2.0);
    {
        let mut object = light.borrow_mut();
        object.position.set(5.0, 5.0, 5.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;

        let d = 10.0;

        shadow.camera.set_bounds(-d, d, d, -d);
        shadow.camera.set_far(25.0);
    }
    scene.add(&light);

    // model

    let obj3d = Object3D::new_node();
    let group = Group::new();

    scene.add(&group);

    group.add(&obj3d);

    //

    let geometry = Rc::new(sphere_geometry(3.0, 48, 24));

    for _ in 0..20 {
        // `material.color.setHSL( Math.random(), 1.0, 0.3 )` in the working
        // (linear-sRGB) colour space.
        let mut color = Color::default();
        color.set_hsl(random.next(), 1.0, 0.3, ColorSpace::LinearSrgb);
        let material = MeshBasicNodeMaterial::lambert(color);

        let mesh = Mesh::new(geometry.clone(), material);
        {
            let mut object = mesh.borrow_mut();
            object.position.x = random.next() * 4.0 - 2.0;
            object.position.y = random.next() * 4.0 - 2.0;
            object.position.z = random.next() * 4.0 - 2.0;
            object.receive_shadow = true;
            object.cast_shadow = true;
            object.scale.multiply_scalar(random.next() * 0.3 + 0.1);
        }
        group.add(&mesh);
    }

    let mut floor_material = MeshBasicNodeMaterial::lambert(Color::from_hex(0xffffff));
    floor_material.side = Side::Double;

    let floor_geometry = Rc::new(plane_geometry(12.0, 12.0, 1, 1));
    let floor_mesh = Mesh::new(floor_geometry, floor_material);
    {
        let mut object = floor_mesh.borrow_mut();
        // `floorMesh.rotation.x -= Math.PI * 0.5`, through the Euler's
        // `onChange` (`set_rotation`) so the quaternion follows.
        let rotation = object.rotation;
        object.set_rotation(
            rotation.x - std::f64::consts::PI * 0.5,
            rotation.y,
            rotation.z,
        );
        object.position.y -= 1.5;
    }
    group.add(&floor_mesh);
    floor_mesh.borrow_mut().receive_shadow = true;

    let torus_geometry = Rc::new(torus_geometry(1.0, 0.3, 16, 100));
    let torus_material = MeshBasicNodeMaterial::phong(Color::from_hex(0xffaaff));
    let torus = Mesh::new(torus_geometry, torus_material);
    torus.borrow_mut().position.z = -4.0;
    group.add(&torus);
    {
        let mut object = torus.borrow_mut();
        object.receive_shadow = true;
        object.cast_shadow = true;
    }

    // The loader's callback (see the module docs).
    obj3d.add(&load_tree());

    // outline pass

    let (edge_strength_node, edge_strength) = uniform_settable(Type::F32, vec![3.0]);
    let (edge_glow_node, edge_glow) = uniform_settable(Type::F32, vec![0.0]);
    let (edge_thickness_node, edge_thickness) = uniform_settable(Type::F32, vec![1.0]);
    let (pulse_period_node, pulse_period) = uniform_settable(Type::F32, vec![0.0]);
    let (visible_edge_color_node, visible_edge_color) = color_uniform(0xffffff);
    let (hidden_edge_color_node, hidden_edge_color) = color_uniform(0x4e3636);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let outline_pass = outline(
        scene.clone(),
        camera.clone(),
        OutlineParams {
            selected_objects: Vec::new(),
            edge_glow: edge_glow_node,
            edge_thickness: edge_thickness_node,
            ..OutlineParams::default()
        },
    );

    let visible_edge = outline_pass.visible_edge();
    let hidden_edge = outline_pass.hidden_edge();

    let period = time().div(pulse_period_node.clone()).mul(float(2.0));
    let osc = osc_sine(period).mul(float(0.5)).add(float(0.5)); // osc [ 0.5, 1.0 ]

    let outline_color = visible_edge
        .mul(visible_edge_color_node)
        .add(hidden_edge.mul(hidden_edge_color_node))
        .mul(edge_strength_node);
    let outline_pulse = pulse_period_node
        .greater_than(float(0.0))
        .select(outline_color.clone().mul(osc), outline_color);

    // postprocessing

    let scene_pass = pass(scene.clone(), camera.clone());

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(outline_pulse.add(scene_pass.node()));

    App {
        renderer,
        scene,
        camera,
        controls,
        group,
        uniforms: Uniforms {
            edge_strength,
            edge_glow,
            edge_thickness,
            pulse_period,
            visible_edge_color,
            hidden_edge_color,
        },
        outline_pass,
        scene_pass,
        render_pipeline,
        raycaster: Raycaster::default(),
        mouse: Vector2::new(0.0, 0.0),
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onPointerMove()` and `checkIntersection()`, for a host with a
/// pointer: the nearest object under it becomes the one selected object; a
/// miss keeps the previous selection, as the page's commented-out `else`
/// does.
pub fn pointer_move(app: &mut App, client_x: f64, client_y: f64, width: f64, height: f64) {
    app.mouse.x = (client_x / width) * 2.0 - 1.0;
    app.mouse.y = -(client_y / height) * 2.0 + 1.0;

    check_intersection(app);
}

/// `checkIntersection()`.
fn check_intersection(app: &mut App) {
    app.raycaster
        .set_from_camera(&app.mouse, &*app.camera.borrow());

    let root = app.scene.borrow().node.clone();
    let intersects = app.raycaster.intersect_object(&root, true);

    if let Some(intersection) = intersects.first() {
        // `addSelectedObject( selectedObject )`, then
        // `outlinePass.selectedObjects = selectedObjects`.
        app.outline_pass
            .set_selected_objects(vec![intersection.object.clone()]);
    } else {
        // outlinePass.selectedObjects = [];
    }
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

/// The controls and the camera at once; see `webgpu_postprocessing_ca`.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_outline.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
