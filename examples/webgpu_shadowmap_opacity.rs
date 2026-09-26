//! Port of `three.js/examples/webgpu_shadowmap_opacity.html`, calling the
//! three-rs API the way the page calls three.js.
//!
//! Two transmissive dragons from `DragonAttenuation.glb` cast coloured shadows
//! onto the cloth backdrop: `renderer.shadowMap.transmitted = true` makes the
//! receivers sample the shadow pass's colour target, and each dragon's
//! `material.castShadowNode` writes its `attenuationColor` into it (see
//! `docs/nodes.md` §43). The output is `AgXToneMapping` at exposure 1.5.
//!
//! Divergence: the page sets `dirLight.shadow.autoUpdate = false` and
//! `needsUpdate = true`, so three renders the shadow map once. The port has no
//! `autoUpdate` and renders it every frame; nothing in the scene moves, so the
//! map it renders is the same one each time.

use three_rs::addons::controls::OrbitControls;
use three_rs::core::Node;
use three_rs::loaders::GLTFLoader;
use three_rs::materials::ToneMapping;
use three_rs::nodes::tsl::{float, mix, vec3};
use three_rs::nodes::NodeRef;
use three_rs::objects::Background;
use three_rs::{
    AmbientLight, Color, DirectionalLight, PerspectiveCamera, Renderer, RendererParameters, Scene,
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
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
}

/// The page's `customShadow = Fn( ( [ color, opacity = 1 ] ) => mix( 1, color,
/// opacity ) )` — "opacity by color". Called with a `Color`, which TSL turns
/// into a constant, so the colour is baked into the shadow material's WGSL.
fn custom_shadow(color: Color) -> NodeRef {
    mix(vec3(1.0, 1.0, 1.0), NodeRef::from(color), float(1.0))
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 40.0);
    camera.node.borrow_mut().position.set(-4.0, 2.0, 6.0);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AgX;
    renderer.tone_mapping_exposure = 1.5;
    renderer.shadow_map_enabled = true;
    renderer.shadow_map_transmitted = true;

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x9e9eff)));

    // light + shadow

    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 0.5));

    let dir_light = DirectionalLight::new(Color::from_hex(0x6666ff), 10.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(3.0, 5.0, 17.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(50.0);
        shadow.camera.set_bounds(-5.0, 5.0, 5.0, -5.0);
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.radius = 4.0;
    }
    scene.add(&dir_light);

    //

    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/DragonAttenuation.glb"))
        .expect("DragonAttenuation.glb");
    gltf.scene.borrow_mut().position.set(0.0, 0.0, -0.5);

    let children = gltf.scene.children();
    let floor = children[0].clone();
    {
        let mut object = floor.borrow_mut();
        object.scale.x += 4.0;
        object.scale.y += 4.0;
    }

    let dragon = children[1].clone();
    dragon.borrow_mut().position.set(-1.5, -0.8, 1.0);

    // `dragon.clone()` then `dragon2.material = dragon.material.clone()`: the
    // port's `Object3D` clone already copies the material (with a fresh id),
    // which is where the page ends up.
    let dragon2 = Node::new(dragon.borrow().clone());
    {
        let mut object = dragon2.borrow_mut();
        object
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
            .expect("the dragon has a material")
            .attenuation_color = Color::from_hex(0xff0000);
        object.position.x += 4.0;
    }
    gltf.scene.add(&dragon2);

    // apply shadow

    floor.borrow_mut().receive_shadow = true;

    for node in [&dragon, &dragon2] {
        let mut object = node.borrow_mut();
        object.cast_shadow = true;
        object.receive_shadow = true;
        let material = object
            .mesh_mut()
            .and_then(|mesh| mesh.material.as_mut())
            .expect("the dragon has a material");
        material.cast_shadow_node = Some(custom_shadow(material.attenuation_color));
    }

    //

    scene.add(&gltf.scene);

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 0.1;
    controls.max_distance = 10.0;
    controls.target.set(0.0, 0.0, 0.0);
    let _ = controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_shadowmap_opacity.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
