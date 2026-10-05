//! Port of `three.js/examples/webgpu_camera.html`, calling the three-rs API in
//! the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! One scene drawn twice per frame, side by side, through
//! `setScissorTest( true )` and a `setScissor` / `setViewport` pair per half:
//! the left half through the active camera (`cameraPerspective`, riding on
//! `cameraRig` and pointed at the big sphere), the right half through an
//! overview camera, where the active camera's `CameraHelper` shows its
//! frustum. The spheres are `wireframe` `MeshBasicMaterial`s; the ten thousand
//! stars a `Points` with a legacy `PointsMaterial`.
//!
//! `Date.now()` is 0 on the graded frame (the harness pins it), so `r = 0`:
//! the big sphere sits at `( 700, 0, 0 )`, the small one at `( 70, 150, 0 )`
//! inside it, and `cameraPerspective.fov` is 35.
//!
//! The stars use `THREE.MathUtils.randFloatSpread`, i.e. the harness'
//! deterministic `Math.random()`, three draws per star, and this page makes no
//! other draws (it has no `Inspector`).
//!
//! **What is not ported.** The `O` / `P` `keydown` handler (switching the
//! active camera to `cameraOrtho`) — the graded frame is the untouched default,
//! and none of the port's hosts forwards key presses to an example. And
//! `cameraOrtho` is not attached to `cameraRig`: the port's
//! `OrthographicCamera` is not a scene-graph node. Neither changes a pixel:
//! with `cameraPerspective` active, `cameraOrthoHelper.visible` is set to
//! `false` every frame before either render.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::{BufferAttribute, BufferGeometry, ObjectRef};
use three_rs::geometries::sphere_geometry;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    CameraHelper, Color, Group, Mesh, MeshBasicNodeMaterial, OrthographicCamera, PerspectiveCamera,
    Points, PointsNodeMaterial, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const frustumSize = 600`.
const FRUSTUM_SIZE: f64 = 600.0;

/// `activeCamera` / `activeHelper` — which of the two rig cameras the left
/// half draws through.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Active {
    Perspective,
    Orthographic,
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    /// The overview camera the right half draws through.
    pub camera: PerspectiveCamera,
    pub camera_perspective: PerspectiveCamera,
    pub camera_ortho: OrthographicCamera,
    pub camera_perspective_helper: CameraHelper,
    pub camera_ortho_helper: CameraHelper,
    pub camera_rig: ObjectRef,
    pub mesh: ObjectRef,
    /// `mesh.children[ 0 ]`.
    pub mesh2: ObjectRef,
    pub active: Active,
    pub screen_width: f64,
    pub screen_height: f64,
}

/// `THREE.MathUtils.randFloatSpread( range )` — `range * ( 0.5 - Math.random() )`.
fn rand_float_spread(random: &mut DeterministicRandom, range: f64) -> f64 {
    range * (0.5 - random.next())
}

/// `new THREE.MeshBasicMaterial( { color, wireframe: true } )`.
fn wireframe_material(hex: u32) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(hex);
    material.wireframe = true;
    material
}

pub fn init() -> App {
    let mut random = DeterministicRandom::new();

    let screen_width = INNER_WIDTH;
    let screen_height = INNER_HEIGHT;
    let aspect = screen_width / screen_height;

    let scene = Scene::new();

    //

    let camera = PerspectiveCamera::new(50.0, 0.5 * aspect, 1.0, 10000.0);
    camera.node.borrow_mut().position.z = 2500.0;

    let camera_perspective = PerspectiveCamera::new(50.0, 0.5 * aspect, 150.0, 1000.0);

    let camera_perspective_helper = CameraHelper::new(&camera_perspective);
    scene.add(&camera_perspective_helper.node);

    //
    let mut camera_ortho = OrthographicCamera::new(
        0.5 * FRUSTUM_SIZE * aspect / -2.0,
        0.5 * FRUSTUM_SIZE * aspect / 2.0,
        FRUSTUM_SIZE / 2.0,
        FRUSTUM_SIZE / -2.0,
        150.0,
        1000.0,
    );

    let camera_ortho_helper = CameraHelper::new(&camera_ortho);
    scene.add(&camera_ortho_helper.node);

    //

    let active = Active::Perspective;

    // counteract different front orientation of cameras vs rig

    camera_ortho.object.set_rotation(0.0, PI, 0.0);
    camera_perspective
        .node
        .borrow_mut()
        .set_rotation(0.0, PI, 0.0);

    let camera_rig = Group::new();

    camera_rig.add(&camera_perspective.node);
    // `cameraRig.add( cameraOrtho )` — not portable, see the module docs.

    scene.add(&camera_rig);

    //

    let mesh = Mesh::new(
        Rc::new(sphere_geometry(100.0, 16, 8)),
        wireframe_material(0xffffff),
    );
    scene.add(&mesh);

    let mesh2 = Mesh::new(
        Rc::new(sphere_geometry(50.0, 16, 8)),
        wireframe_material(0x00ff00),
    );
    mesh2.borrow_mut().position.y = 150.0;
    mesh.add(&mesh2);

    let mesh3 = Mesh::new(
        Rc::new(sphere_geometry(5.0, 16, 8)),
        wireframe_material(0x0000ff),
    );
    mesh3.borrow_mut().position.z = 150.0;
    camera_rig.add(&mesh3);

    //

    let mut geometry = BufferGeometry::new();
    let mut vertices: Vec<f32> = Vec::with_capacity(30000);

    for _ in 0..10000 {
        vertices.push(rand_float_spread(&mut random, 2000.0) as f32); // x
        vertices.push(rand_float_spread(&mut random, 2000.0) as f32); // y
        vertices.push(rand_float_spread(&mut random, 2000.0) as f32); // z
    }

    geometry.set_attribute("position", BufferAttribute::new(vertices, 3));

    // `new THREE.PointsMaterial( { color: 0xffffff } )`. `NodeLibrary.fromMaterial()`
    // turns it into a `PointsNodeMaterial` and then copies every property of
    // the legacy material over it — `transparent: false` included, which
    // undoes the `true` `PointsNodeMaterial` inherits from `SpriteNodeMaterial`
    // (three's dump of this pipeline has no blend state).
    let mut points_material = PointsNodeMaterial::points();
    points_material.color = Color::from_hex(0xffffff);
    points_material.transparent = false;

    let particles = Points::new(Rc::new(geometry), points_material);
    scene.add(&particles);

    //

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(screen_width, screen_height);

    renderer.set_scissor_test(true);
    renderer.set_clear_color(Color::from_hex(0x000000), 1.0);

    App {
        renderer,
        scene,
        camera,
        camera_perspective,
        camera_ortho,
        camera_perspective_helper,
        camera_ortho_helper,
        camera_rig,
        mesh,
        mesh2,
        active,
        screen_width,
        screen_height,
    }
}

/// The page's `animate()`, which is its `render()`.
pub fn animate(app: &mut App) {
    let r = three_rs::utils::date_now_ms() * 0.0005;

    let position = {
        let mut mesh = app.mesh.borrow_mut();
        mesh.position.x = 700.0 * r.cos();
        mesh.position.z = 700.0 * r.sin();
        mesh.position.y = 700.0 * r.sin();
        mesh.position
    };

    {
        let mut mesh2 = app.mesh2.borrow_mut();
        mesh2.position.x = 70.0 * (2.0 * r).cos();
        mesh2.position.z = 70.0 * r.sin();
    }

    if app.active == Active::Perspective {
        app.camera_perspective.fov = 35.0 + 30.0 * (0.5 * r).sin();
        app.camera_perspective.far = position.length();
        app.camera_perspective.update_projection_matrix();

        app.camera_perspective_helper
            .update(&app.camera_perspective);
        app.camera_perspective_helper.node.borrow_mut().visible = true;

        app.camera_ortho_helper.node.borrow_mut().visible = false;
    } else {
        app.camera_ortho.far = position.length();
        app.camera_ortho.update_projection_matrix();

        app.camera_ortho_helper.update(&app.camera_ortho);
        app.camera_ortho_helper.node.borrow_mut().visible = true;

        app.camera_perspective_helper.node.borrow_mut().visible = false;
    }

    app.camera_rig.look_at(&position);

    //

    let active_helper = match app.active {
        Active::Perspective => app.camera_perspective_helper.node.clone(),
        Active::Orthographic => app.camera_ortho_helper.node.clone(),
    };

    let (w, h) = (app.screen_width, app.screen_height);

    active_helper.borrow_mut().visible = false;

    app.renderer.set_clear_color(Color::from_hex(0x000000), 1.0);
    app.renderer.set_scissor(0.0, 0.0, w / 2.0, h);
    app.renderer.set_viewport(0.0, 0.0, w / 2.0, h);
    match app.active {
        Active::Perspective => app
            .renderer
            .render(&mut app.scene, &mut app.camera_perspective),
        Active::Orthographic => app.renderer.render(&mut app.scene, &mut app.camera_ortho),
    }

    //

    active_helper.borrow_mut().visible = true;

    app.renderer.set_clear_color(Color::from_hex(0x111111), 1.0);
    app.renderer.set_scissor(w / 2.0, 0.0, w / 2.0, h);
    app.renderer.set_viewport(w / 2.0, 0.0, w / 2.0, h);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.screen_width = width;
    app.screen_height = height;
    let aspect = width / height;

    app.renderer.set_size(width, height);

    app.camera.aspect = 0.5 * aspect;
    app.camera.update_projection_matrix();

    app.camera_perspective.aspect = 0.5 * aspect;
    app.camera_perspective.update_projection_matrix();

    app.camera_ortho.left = -0.5 * FRUSTUM_SIZE * aspect / 2.0;
    app.camera_ortho.right = 0.5 * FRUSTUM_SIZE * aspect / 2.0;
    app.camera_ortho.top = FRUSTUM_SIZE / 2.0;
    app.camera_ortho.bottom = -FRUSTUM_SIZE / 2.0;
    app.camera_ortho.update_projection_matrix();
}

/// The page creates no controls.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The page creates no controls.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
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
        .unwrap_or_else(|| "target/webgpu_camera.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
