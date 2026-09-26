//! Port of `three.js/examples/webgpu_tsl_angular_slicing.html`, calling the
//! three-rs API in the same order the page's `init()` and its two loader
//! callbacks do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws nothing from
//! `Math.random()` or the clock.
//!
//! # Which page
//!
//! The graded screenshot, `examples/screenshots/webgpu_tsl_angular_slicing.jpg`,
//! was last written by three.js' f8462e3, when the page lit the gears with a
//! plain `DirectionalLight`. r186 briefly swapped that for the `SunLight`
//! addon (#34476) without regenerating the screenshot, and #34586 swapped it
//! back after r186. This port follows the page the screenshot was taken of —
//! the `DirectionalLight` one, which is also three.js' `dev` today.
//!
//! # What this page is
//!
//! The model is `gears.glb`, whose three meshes are all
//! `KHR_draco_mesh_compression` primitives (issue #139). The outer hull gets
//! a material whose `maskNode` discards an angular wedge around the local z
//! axis, and whose `outputNode` paints every back face a flat colour — so the
//! cut shows a solid "inside" through the missing wedge. The mask is carried
//! into the shadow pass too, so the wedge is missing from the shadow.
//!
//! `envMapIntensity: 0.5` on both materials does nothing on this page:
//! `materialEnvIntensity` reads `material.envMap ? material.envMapIntensity :
//! scene.environmentIntensity` (`MaterialProperties.js`), and neither material
//! has an `envMap` of its own — the environment is `scene.environment`, at
//! `environmentIntensity` 1. The port therefore has no field to set.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GLTFLoader, UltraHdrLoader};
use three_rs::materials::{Side, ToneMapping};
use three_rs::math::Vector3;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    block, float, front_facing, if_then, output_property, position_local, to_var, two_pi,
    uniform_value, vec4_join,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::{
    plane_geometry, Color, DirectionalLight, Mesh, MeshPhysicalNodeMaterial,
    MeshStandardNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene,
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
    pub environment: PmremEnvironment,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

/// `inAngle( [ position, angleStart, angleArc ] )`: `Fn()` without a layout,
/// so three inlines it at its one call site.
fn in_angle(position: NodeRef, angle_start: NodeRef, angle_arc: NodeRef) -> NodeRef {
    // `atan( position.y, position.x )` — TSL's two-argument `atan` is WGSL's
    // `atan2( y, x )`.
    let angle = to_var(
        None,
        position
            .y()
            .atan2(position.x())
            .sub(angle_start)
            .modulo(two_pi()),
    );
    angle.greater_than(0.0).and(angle.less_than(angle_arc))
}

/// `new MeshPhysicalNodeMaterial( { metalness: 0.5, roughness: 0.25,
/// envMapIntensity: 0.5, color: '#858080' } )` — see the module docs for why
/// `envMapIntensity` has no counterpart.
fn gear_material() -> MeshPhysicalNodeMaterial {
    MeshPhysicalNodeMaterial::physical(Color::from_hex(0x858080), 0.25, 0.5)
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 35, window.innerWidth / window.innerHeight,
    // 0.1, 100 )`, then `camera.position.set( - 5, 5, 12 )`.
    let mut camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-5.0, 5.0, 12.0);

    let mut scene = Scene::new();

    // `renderer = new THREE.WebGPURenderer( { antialias: true } )`. The page
    // builds it last; it is first here because the environment conversions
    // below take it.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AcesFilmic;
    renderer.tone_mapping_exposure = 1.0;
    renderer.shadow_map_enabled = true;

    // environment

    // `new UltraHDRLoader().load( 'textures/equirectangular/
    // royal_esplanade_2k.hdr.jpg', … )`, resolved synchronously, with
    // `EquirectangularReflectionMapping`: the background is the 512² cube
    // `CubeMapNode` converts it into, the environment its PMREM.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));

    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    // lights

    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 4.0);
    {
        let mut object = directional_light.borrow_mut();
        object.position.set(6.25, 3.0, 4.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(30.0);
        // `top = 8; right = 8; bottom = - 8; left = - 8`.
        shadow.camera.set_bounds(-8.0, 8.0, 8.0, -8.0);
        shadow.normal_bias = 0.05;
    }
    scene.add(&directional_light);

    // materials

    let default_material = gear_material();
    let mut sliced_material = gear_material();
    sliced_material.side = Side::Double;

    // uniforms

    let slice_start = uniform_value(Type::F32, vec![1.75]);
    let slice_arc = uniform_value(Type::F32, vec![1.25]);
    // `uniform( color( '#b62f58' ) )` — the sRGB hex in the working space.
    let slice = Color::from_hex(0xb62f58);
    let slice_color = uniform_value(Type::Vec3, vec![slice.r, slice.g, slice.b]);

    // mask

    sliced_material.mask_node = Some(in_angle(position_local().xy(), slice_start, slice_arc).not());

    // output: `const finalOutput = output; If( frontFacing.not(), () => {
    // finalOutput.assign( vec4( sliceColor, 1 ) ) } ); return finalOutput;`
    sliced_material.output_node = Some(block(
        vec![if_then(
            front_facing().not(),
            vec![output_property().assign(vec4_join(vec![slice_color, float(1.0)]))],
        )],
        output_property(),
    ));

    // model

    let gltf = GLTFLoader::load(examples_dir().join("models/gltf/gears.glb")).expect("gears.glb");
    gltf.scene.traverse(&mut |node| {
        let mut object = node.borrow_mut();
        if !object.is_mesh() {
            return;
        }
        let material = if object.name == "outerHull" {
            sliced_material.clone()
        } else {
            default_material.clone()
        };
        object.mesh_mut().unwrap().material = Some(material);
        object.cast_shadow = true;
        object.receive_shadow = true;
    });
    scene.add(&gltf.scene);

    // plane

    let plane = Mesh::new(
        Rc::new(plane_geometry(10.0, 10.0, 10, 1)),
        // `new THREE.MeshStandardMaterial( { color: '#aaaaaa' } )`: the
        // roughness and metalness are `MeshStandardMaterial`'s 1 and 0.
        MeshStandardNodeMaterial::standard(Color::from_hex(0xaaaaaa), 1.0, 0.0),
    );
    {
        let mut object = plane.borrow_mut();
        object.receive_shadow = true;
        object.position.set(-4.0, -3.0, -4.0);
        object.look_at(&Vector3::new(0.0, 0.0, 0.0));
    }
    scene.add(&plane);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 0.1;
    controls.max_distance = 50.0;

    App {
        renderer,
        scene,
        camera,
        environment,
        controls,
    }
}

/// The page's `animate()`: `controls.update()` and one render.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

    app.environment.update(&mut app.renderer).unwrap();
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

/// The controls and the camera at once — see `webgpu_loader_gltf`'s copy.
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
        .unwrap_or_else(|| "target/webgpu_tsl_angular_slicing.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
