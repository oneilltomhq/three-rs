//! Port of `three.js/examples/webgpu_shadowmap_vsm.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `performance.now()` is pinned to 0, so `Timer.update()`'s first delta is 0:
//! the torus knot keeps its zero rotation, `dirGroup` stays unrotated, and
//! `dirLight.position.z` is `17 + sin( 0 ) * 5 = 17`. Nothing in `animate()`
//! moves the graded frame.
//!
//! `renderer.shadowMap.type = VSMShadowMap`: each shadow-casting light renders
//! its depth map, then `ShadowNode.vsmPass()` blurs it vertically and
//! horizontally into two `RGFormat` targets of `( mean, stdDev )`, and the lit
//! materials read the second one through `VSMShadowFilter`.
//!
//! The `config` GUI (`radius`, `samples`, `animate`) sits at its defaults —
//! `radius = 4` is set on both shadows below and `blurSamples` keeps
//! `LightShadow`'s 8. `renderer.inspector = new Inspector()` does not touch
//! the frame, and the `resize` listener never fires.
//!
//! **Divergence** (`docs/nodes.md`): `scene.fog = new THREE.Fog( … )` is
//! spelt as the `fog( color, rangeFogFactor( near, far ) )` node the port has,
//! whose colour and range are constants in the WGSL where three's `Fog`
//! reads them from uniforms — the same stand-in `webgpu_shadowmap` uses.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::geometries::{cylinder_geometry, plane_geometry, torus_knot_geometry};
use three_rs::lights::ShadowMapType;
use three_rs::nodes::tsl::{fog, range_fog_factor};
use three_rs::objects::Background;
use three_rs::utils::now_ms;
use three_rs::Timer;
use three_rs::{
    AmbientLight, Color, DirectionalLight, Group, Mesh, MeshPhongNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene, SpotLight,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub torus_knot: Node,
    pub dir_group: Node,
    pub dir_light: Node,
    /// The page's module-level `timer`.
    pub timer: Timer,
}

pub fn init() -> App {
    // `initScene()`

    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 10.0, 30.0);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x222244)));
    scene.fog_node = Some(fog(
        Color::from_hex(0x222244),
        range_fog_factor(50.0, 100.0),
    ));

    // Lights

    scene.add(&AmbientLight::new(Color::from_hex(0x444444), 1.0));

    let spot_light = SpotLight::new(Color::from_hex(0xff8888), 400.0);
    {
        let mut object = spot_light.borrow_mut();
        object.position.set(8.0, 10.0, 5.0);
        object.cast_shadow = true;
        let light = object.light_mut().unwrap();
        light.angle = PI / 5.0;
        light.penumbra = 0.3;
        let shadow = light.shadow.as_mut().unwrap();
        shadow.camera.set_near(8.0);
        shadow.camera.set_far(200.0);
        shadow.map_size.x = 256.0;
        shadow.map_size.y = 256.0;
        shadow.bias = -0.002;
        shadow.radius = 4.0;
    }
    scene.add(&spot_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0x8888ff), 3.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(3.0, 12.0, 17.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(500.0);
        shadow.camera.set_bounds(-17.0, 17.0, 17.0, -17.0);
        shadow.map_size.x = 512.0;
        shadow.map_size.y = 512.0;
        shadow.radius = 4.0;
        shadow.bias = -0.0005;
    }

    let dir_group = Group::new();
    dir_group.add(&dir_light);
    scene.add(&dir_group);

    // Geometry

    let geometry = Rc::new(torus_knot_geometry(25.0, 8.0, 75, 20, 2.0, 3.0));
    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0x999999));
    material.shininess = 0.0;
    material.specular = Color::from_hex(0x222222);

    let torus_knot = Mesh::new(geometry, material.clone());
    {
        let mut object = torus_knot.borrow_mut();
        object.scale.multiply_scalar(1.0 / 18.0);
        object.position.y = 3.0;
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.add(&torus_knot);

    let cylinder_geometry = Rc::new(cylinder_geometry(0.75, 0.75, 7.0, 32));

    // `pillar1.clone()` shares the geometry and the material and copies
    // `castShadow` / `receiveShadow`; only the position differs.
    for (x, z) in [(8.0, 8.0), (8.0, -8.0), (-8.0, 8.0), (-8.0, -8.0)] {
        let pillar = Mesh::new(cylinder_geometry.clone(), material.clone());
        {
            let mut object = pillar.borrow_mut();
            object.position.set(x, 3.5, z);
            object.cast_shadow = true;
            object.receive_shadow = true;
        }
        scene.add(&pillar);
    }

    let plane_geometry = Rc::new(plane_geometry(200.0, 200.0, 1, 1));
    let mut plane_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x999999));
    plane_material.shininess = 0.0;
    plane_material.specular = Color::from_hex(0x111111);

    let ground = Mesh::new(plane_geometry, plane_material);
    {
        let mut object = ground.borrow_mut();
        object.set_rotation(-PI / 2.0, 0.0, 0.0);
        object.scale.multiply_scalar(3.0);
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.add(&ground);

    // `initMisc()`

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;
    renderer.shadow_map_type = ShadowMapType::Vsm;

    // Mouse control
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 2.0, 0.0);
    let _ = controls.update(&mut camera, None);

    App {
        // `timer = new THREE.Timer()` — constructed at the end of
        // `initMisc()`, as the page does.
        timer: Timer::new(),
        renderer,
        scene,
        camera,
        controls,
        torus_knot,
        dir_group,
        dir_light,
    }
}

/// The page's `animate( time )` with `time = 0` and `Timer`'s first delta of 0.
/// `config.animate` is `true`.
pub fn animate(app: &mut App) {
    app.timer.update();
    let delta = app.timer.get_delta();
    // `animate( time )`'s argument is `requestAnimationFrame`'s, which is
    // `performance.now()` at the start of the frame.
    let time = now_ms();

    {
        let mut object = app.torus_knot.borrow_mut();
        let r = object.rotation;
        object.set_rotation(r.x + 0.25 * delta, r.y + 0.5 * delta, r.z + 1.0 * delta);
    }
    {
        let mut object = app.dir_group.borrow_mut();
        let r = object.rotation;
        object.set_rotation(r.x, r.y + 0.7 * delta, r.z);
    }
    app.dir_light.borrow_mut().position.z = 17.0 + (time * 0.001).sin() * 5.0;

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `resize()` — the same three lines the other examples spell
/// `onWindowResize()`.
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
        .unwrap_or_else(|| "target/webgpu_shadowmap_vsm.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
