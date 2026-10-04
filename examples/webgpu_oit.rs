//! Port of `three.js/examples/webgpu_oit.html`, calling the three-rs API in
//! the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0. `animate()` only updates the controls and renders: nothing in the scene
//! moves, and the spheres "orbit" only because the camera does.
//!
//! `controls.autoRotate` is on, and `update()` runs twice (once from the
//! constructor, once in `animate()`). The constructor's update aims the camera
//! at the origin, which nothing else in the page does; the auto-rotation with
//! `deltaTime` null, damped, moves the camera by a sliver of a pixel. The port
//! runs the real `OrbitControls`, so that sliver is there too.
//!
//! The graded frame is the first, and on it the OIT pass's target is drawn
//! into for the first time: `Renderer._renderScene()` clears a new target's
//! depth whatever `autoClearDepth` says, so on that frame the transparent
//! planes and spheres are *not* occluded by the knot. Three does it and the
//! reference shows it; the port does it too (`docs/nodes.md` §82).
//!
//! The page's `Inspector` GUI is not ported. Its two parameters are
//! [`Params`], and its two `onChange` handlers are [`set_oit`] and
//! [`set_opacity`].

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::{plane_geometry, sphere_geometry, torus_knot_geometry};
use three_rs::materials::Side;
use three_rs::nodes::display::{oit_pass, OitPassNode};
use three_rs::{
    pass, Color, DirectionalLight, Group, HemisphereLight, Mesh, MeshStandardNodeMaterial,
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// The page's `params`.
pub struct Params {
    /// `params.oit` — whether the pipeline's output is the OIT pass or the
    /// plain one.
    pub oit: bool,
    /// `params.opacity` — every transparent material's opacity.
    pub opacity: f64,
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with both passes, which render it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with both passes.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's `spheres` group.
    pub spheres: three_rs::Node,
    /// The meshes of the page's `transparentMaterials`, planes first: the
    /// materials live on the meshes here, so the handler reaches them
    /// through these.
    pub transparent_meshes: Vec<three_rs::Node>,
    pub params: Params,
    /// `scenePassOIT`.
    pub scene_pass_oit: OitPassNode,
    /// `scenePass`.
    pub scene_pass: PassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let params = Params {
        oit: true,
        opacity: 0.5,
    };

    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(8.0, 4.0, 10.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x202020));

    // lights

    let hemi_light =
        HemisphereLight::new(Color::from_hex(0xffffff), Color::from_hex(0x444444), 1.5);
    scene.add(&hemi_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 2.5);
    dir_light.borrow_mut().position.set(4.0, 10.0, 6.0);
    scene.add(&dir_light);

    // opaque center object

    // `new THREE.MeshStandardMaterial( { color: 0xffffff, roughness: 0.2 } )`
    // — `metalness` at its default of 0.
    let torus_knot = Mesh::new(
        Rc::new(torus_knot_geometry(1.0, 0.4, 128, 32, 2.0, 3.0)),
        MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 0.2, 0.0),
    );
    scene.add(&torus_knot);

    // intersecting transparent planes. no draw order can resolve them correctly
    // with sorting-based transparency

    let mut transparent_meshes = Vec::new();

    let plane_geometry = Rc::new(plane_geometry(6.0, 6.0, 1, 1));
    const PLANE_COLORS: [u32; 3] = [0xff4030, 0x30d060, 0x3070ff];

    for (i, &color) in PLANE_COLORS.iter().enumerate() {
        let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(color), 0.5, 0.0);
        material.transparent = true;
        material.opacity = params.opacity;
        material.side = Side::Double;

        let plane = Mesh::new(plane_geometry.clone(), material);
        {
            let mut object = plane.borrow_mut();
            let rotation = object.rotation;
            object.set_rotation(
                rotation.x,
                (i as f64 / PLANE_COLORS.len() as f64) * std::f64::consts::PI,
                rotation.z,
            );
        }
        scene.add(&plane);
        transparent_meshes.push(plane);
    }

    // orbiting transparent spheres. their sort order flips while they rotate
    // which results in popping with sorting-based transparency

    let spheres = Group::new();

    let sphere_geometry = Rc::new(sphere_geometry(0.8, 32, 16));
    const SPHERE_COLORS: [u32; 4] = [0xffc020, 0xd040d0, 0x40c0d0, 0xa0d040];

    for (i, &color) in SPHERE_COLORS.iter().enumerate() {
        let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(color), 0.3, 0.0);
        material.transparent = true;
        material.opacity = params.opacity;

        let sphere = Mesh::new(sphere_geometry.clone(), material);

        let angle = (i as f64 / SPHERE_COLORS.len() as f64) * std::f64::consts::PI * 2.0;
        sphere
            .borrow_mut()
            .position
            .set(angle.cos() * 2.5, 0.0, angle.sin() * 2.5);

        spheres.add(&sphere);
        transparent_meshes.push(sphere);
    }

    scene.add(&spheres);

    //

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let scene_pass_oit = oit_pass(scene.clone(), camera.clone());
    let scene_pass = pass(scene.clone(), camera.clone());

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(scene_pass_oit.node());

    //

    let mut controls = {
        let mut camera = camera.borrow_mut();
        OrbitControls::new(&mut camera)
    };
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.auto_rotate = true;
    controls.enable_damping = true;
    controls.min_distance = 5.0;
    controls.max_distance = 25.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        spheres,
        transparent_meshes,
        params,
        scene_pass_oit,
        scene_pass,
        render_pipeline,
    }
}

/// `gui.add( params, 'oit' ).onChange( … )`: the output node switches between
/// the two passes. `renderPipeline.needsUpdate = true` is implied — the
/// pipeline rebuilds its quad whenever the output node's key changes.
pub fn set_oit(app: &mut App, value: bool) {
    app.params.oit = value;
    app.render_pipeline.output_node = Some(if value {
        app.scene_pass_oit.node()
    } else {
        app.scene_pass.node()
    });
}

/// `gui.add( params, 'opacity', 0, 1 ).onChange( … )`.
pub fn set_opacity(app: &mut App, value: f64) {
    app.params.opacity = value;
    for mesh in &app.transparent_meshes {
        if let Some(material) = mesh.borrow_mut().payload.material_mut() {
            material.opacity = value;
        }
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // `controls.update();`
    app.controls.update(&mut app.camera.borrow_mut(), None);

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
    // does to the page.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_oit.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
