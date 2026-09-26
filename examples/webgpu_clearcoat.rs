//! Port of `three.js/examples/webgpu_clearcoat.html`, calling the three-rs API
//! in the same order the page's `init()`, its loader callback and `render()`
//! do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! # What this page is
//!
//! Four `MeshPhysicalMaterial` spheres with `clearcoat = 1` in front of the
//! Pisa HDR cube, which is both `scene.background` (the raw cube) and
//! `scene.environment` (PMREM-filtered), and one `PointLight` riding on a
//! small white sphere. The coat is the second specular lobe of
//! `PhysicalLightingModel`: its indirect half reflects the PMREM, and its
//! direct half — `clearcoatSpecularDirect` — is the sharp highlight a light
//! makes on it, over a base lobe that is rougher or bumpier. Two of the
//! spheres give the coat its own normal map (`clearcoatNormalMap`), so the
//! coat and the base are lit about different normals.
//!
//! Under the pinned clock `timer` is 0 and the light sits at `( 0, 4, 3 )`,
//! above the top edge of the frame: the particle is culled, but the light still
//! reaches all four spheres.
//!
//! # The random sequence
//!
//! `FlakesTexture` draws 20000 values from `Math.random()` — five per flake —
//! and is the only thing on the page that does apart from `new Inspector()`,
//! whose five draws come first: the inspector is built in `init()`, and the
//! flakes only in the HDR loader's callback, after `init()` has returned.
//! [`FlakesTexture`] takes the harness' seeded sequence past those five.

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::textures::FlakesTexture;
use three_rs::geometries::sphere_geometry;
use three_rs::lights::PointLight;
use three_rs::loaders::HdrCubeTextureLoader;
use three_rs::materials::ToneMapping;
use three_rs::math::Vector2;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::objects::Background;
use three_rs::testing::DeterministicRandom;
use three_rs::textures::{Texture, Wrapping};
use three_rs::{
    Color, ColorSpace, Group, Mesh, MeshBasicNodeMaterial, MeshPhysicalNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene, TextureLoader,
};

use std::rc::Rc;

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes — the same five
/// `webgpu_mesh_batch` documents, from the `List` constructors of the
/// inspector's tabs.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub environment: PmremEnvironment,
    /// The page's `particleLight`.
    pub particle_light: three_rs::core::Node,
    /// The page's `group`.
    pub group: three_rs::core::Node,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(27.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 50.0);
    camera.node.borrow_mut().position.z = 10.0;

    let mut scene = Scene::new();

    let group = Group::new();
    scene.add(&group);

    // `renderer = new THREE.WebGPURenderer( { antialias: true } )`. The page
    // builds it after the loader call; here it is first, because the PMREM
    // below takes it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 1.25;

    // `renderer.inspector = new Inspector()`.
    let mut random = DeterministicRandom::new();
    random.skip(INSPECTOR_RANDOM_DRAWS);

    // `new HDRCubeTextureLoader().setPath( 'textures/cube/pisaHDR/' ).load(
    // … )`, synchronous here, so the callback's body follows inline.
    let texture = HdrCubeTextureLoader::new()
        .set_path(examples_dir().join("textures/cube/pisaHDR/"))
        .load(["px.hdr", "nx.hdr", "py.hdr", "ny.hdr", "pz.hdr", "nz.hdr"])
        .unwrap();

    let geometry = Rc::new(sphere_geometry(0.8, 64, 32));

    let texture_loader = TextureLoader::new();
    let load = |path: &str| texture_loader.load(examples_dir().join(path)).unwrap();

    let diffuse = load("textures/carbon/Carbon.png");
    diffuse.set_color_space(ColorSpace::SRGB);
    diffuse.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    diffuse.set_repeat(10.0, 10.0);

    let normal_map = load("textures/carbon/Carbon_Normal.png");
    normal_map.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    normal_map.set_repeat(10.0, 10.0);

    let normal_map2 = load("textures/water/Water_1_M_Normal.jpg");

    // `new THREE.CanvasTexture( new FlakesTexture() )` — 512², drawing from
    // the page's `Math.random()`.
    let normal_map3 = Texture::new(
        512,
        512,
        Some(FlakesTexture::new(512, 512, || random.next())),
    );
    normal_map3.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    normal_map3.set_repeat(10.0, 6.0);
    normal_map3.set_anisotropy(16);

    let normal_map4 = load("textures/golfball.jpg");

    let clearcoat_normal_map = load("textures/pbr/Scratched_gold/Scratched_gold_01_1K_Normal.png");

    // `MeshPhysicalMaterial`'s defaults: white, roughness 1, metalness 0.
    let physical = || MeshPhysicalNodeMaterial::physical(Color::new(1.0, 1.0, 1.0), 1.0, 0.0);

    // car paint
    let mut material = MeshPhysicalNodeMaterial {
        clearcoat: 1.0,
        clearcoat_roughness: 0.1,
        metalness: 0.9,
        roughness: 0.5,
        color: Color::from_hex(0x0000ff),
        normal_map: Some(normal_map3),
        normal_scale: Vector2::new(0.15, 0.15),
        ..physical()
    };
    let mesh = Mesh::new(geometry.clone(), material);
    mesh.borrow_mut().position.x = -1.0;
    mesh.borrow_mut().position.y = 1.0;
    group.add(&mesh);

    // fibers
    material = MeshPhysicalNodeMaterial {
        roughness: 0.5,
        clearcoat: 1.0,
        clearcoat_roughness: 0.1,
        map: Some(diffuse),
        normal_map: Some(normal_map),
        ..physical()
    };
    let mesh = Mesh::new(geometry.clone(), material);
    mesh.borrow_mut().position.x = 1.0;
    mesh.borrow_mut().position.y = 1.0;
    group.add(&mesh);

    // golf
    material = MeshPhysicalNodeMaterial {
        metalness: 0.0,
        roughness: 0.1,
        clearcoat: 1.0,
        normal_map: Some(normal_map4),
        clearcoat_normal_map: Some(clearcoat_normal_map.clone()),
        // "y scale is negated to compensate for normal map handedness."
        clearcoat_normal_scale: Vector2::new(2.0, -2.0),
        ..physical()
    };
    let mesh = Mesh::new(geometry.clone(), material);
    mesh.borrow_mut().position.x = -1.0;
    mesh.borrow_mut().position.y = -1.0;
    group.add(&mesh);

    // clearcoat + normalmap
    material = MeshPhysicalNodeMaterial {
        clearcoat: 1.0,
        metalness: 1.0,
        color: Color::from_hex(0xff0000),
        normal_map: Some(normal_map2),
        normal_scale: Vector2::new(0.15, 0.15),
        clearcoat_normal_map: Some(clearcoat_normal_map),
        clearcoat_normal_scale: Vector2::new(2.0, -2.0),
        ..physical()
    };
    let mesh = Mesh::new(geometry, material);
    mesh.borrow_mut().position.x = 1.0;
    mesh.borrow_mut().position.y = -1.0;
    group.add(&mesh);

    // `scene.background = texture`: the raw HDR cube as the skybox.
    scene.background = Some(Background::CubeTexture(texture.clone()));
    // `scene.environment = texture`: the same cube, PMREM-filtered.
    let mut environment = PmremEnvironment::new(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // LIGHTS

    let particle_light = Mesh::new(
        Rc::new(sphere_geometry(0.05, 8, 8)),
        MeshBasicNodeMaterial {
            color: Color::from_hex(0xffffff),
            ..MeshBasicNodeMaterial::default()
        },
    );
    scene.add(&particle_light);

    particle_light.add(&PointLight::new(Color::from_hex(0xffffff), 30.0, 0.0));

    // EVENTS

    let mut controls = OrbitControls::new(&mut camera);
    // The renderer's canvas stands in for the element's `clientWidth` /
    // `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 3.0;
    controls.max_distance = 30.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        environment,
        particle_light,
        group,
    }
}

/// The page's `animate()`, which is its `render()`.
pub fn animate(app: &mut App) {
    // `Date.now() * 0.00025`.
    let timer = three_rs::utils::date_now_ms() * 0.00025;

    {
        let mut light = app.particle_light.borrow_mut();
        light.position.x = (timer * 7.0).sin() * 3.0;
        light.position.y = (timer * 5.0).cos() * 4.0;
        light.position.z = (timer * 3.0).cos() * 3.0;
    }

    for child in app.group.children() {
        let mut child = child.borrow_mut();
        let rotation = child.rotation;
        child.set_rotation(rotation.x, rotation.y + 0.005, rotation.z);
    }

    app.environment.update(&mut app.renderer).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
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
        .unwrap_or_else(|| "target/webgpu_clearcoat.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
