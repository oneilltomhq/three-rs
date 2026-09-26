//! Port of `three.js/examples/webgpu_tsl_earth.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is 0: the
//! timer's delta on the graded frame is 0, so the globe has not turned.
//!
//! `OrbitControls` is ported, but with damping and no pointer input the first
//! `controls.update()` only aims the camera at the origin.
//!
//! The page's `Inspector` GUI (the colour and roughness sliders) has no
//! counterpart here; its parameters are the uniforms the port builds.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::Side;
use three_rs::nodes::tsl::{
    bump_map_with, camera_position, float, max, mix, normal_world_geometry, output_property,
    position_world, step, texture, texture_uv, to_var, uniform_value, uv, vec3, vec4_join,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::textures::Texture;
use three_rs::{
    sphere_geometry, Background, Color, ColorSpace, DirectionalLight, Mesh, MeshBasicNodeMaterial,
    MeshStandardNodeMaterial, Node, PerspectiveCamera, Renderer, RendererParameters, Scene,
    TextureLoader, Timer, Vector3,
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
    pub timer: Timer,
    pub globe: Node,
}

/// `uniform( color( hex ) )` — the sRGB hex in the working space.
fn uniform_color(hex: u32) -> NodeRef {
    let color = Color::from_hex(hex);
    uniform_value(Type::Vec3, vec![color.r, color.g, color.b])
}

/// The page's node graph, from `// uniforms` to the atmosphere material:
/// the globe's and the atmosphere's materials. Public so
/// `examples/dump_wgsl.rs` builds the same graph the graded frame does.
pub fn materials(
    day_texture: &Texture,
    night_texture: &Texture,
    bump_roughness_clouds_texture: &Texture,
    sun_position: Vector3,
) -> (MeshBasicNodeMaterial, MeshBasicNodeMaterial) {
    // uniforms

    let atmosphere_day_color = uniform_color(0x4db2ff);
    let atmosphere_twilight_color = uniform_color(0xbc490b);
    let roughness_low = uniform_value(Type::F32, vec![0.25]);
    let roughness_high = uniform_value(Type::F32, vec![0.35]);

    // fresnel

    let view_direction = position_world().sub(camera_position()).normalize();
    let fresnel = to_var(
        None,
        view_direction
            .dot(normal_world_geometry())
            .abs()
            .one_minus(),
    );

    // sun orientation

    // `normalize( sun.position )` reads the `Vector3` once, as a constant.
    let sun_orientation = to_var(
        None,
        normal_world_geometry()
            .dot(vec3(sun_position.x, sun_position.y, sun_position.z).normalize()),
    );

    // atmosphere color

    let atmosphere_color = mix(
        atmosphere_twilight_color,
        atmosphere_day_color,
        sun_orientation.smoothstep(-0.25, 0.75),
    );

    // globe

    let mut globe_material =
        MeshStandardNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);

    let clouds_strength = texture_uv(bump_roughness_clouds_texture, uv())
        .z()
        .smoothstep(0.2, 1.0);

    globe_material.color_node = Some(mix(
        texture(day_texture),
        vec3(1.0, 1.0, 1.0),
        clouds_strength.mul(2.0),
    ));

    let roughness = max(
        texture(bump_roughness_clouds_texture).y(),
        step(0.01, clouds_strength.clone()),
    );
    globe_material.roughness_node = Some(roughness.remap(0.0, 1.0, roughness_low, roughness_high));

    let night = texture(night_texture);
    let day_strength = sun_orientation.smoothstep(-0.25, 0.5);

    let atmosphere_day_strength = sun_orientation.smoothstep(-0.5, 1.0);
    let atmosphere_mix = atmosphere_day_strength
        .mul(fresnel.pow(2.0))
        .clamp(0.0, 1.0);

    let output = output_property();
    let final_output = mix(night.rgb(), output.rgb(), day_strength);
    let final_output = mix(final_output, atmosphere_color.clone(), atmosphere_mix);

    globe_material.output_node = Some(vec4_join(vec![final_output, output.w()]));

    // `bumpMap( max( texture( bumpRoughnessCloudsTexture ).r, cloudsStrength ) )`:
    // `texture( … )` is re-sampled at each of the bump taps' uvs,
    // `cloudsStrength` (sampled at an explicit `uv()`) is not.
    globe_material.normal_node = Some(bump_map_with(
        move |texture| {
            max(
                texture(bump_roughness_clouds_texture).x(),
                clouds_strength.clone(),
            )
        },
        float(1.0),
    ));

    let mut atmosphere_material = MeshBasicNodeMaterial::new();
    atmosphere_material.side = Side::Back;
    atmosphere_material.transparent = true;
    let alpha = fresnel.remap(0.73, 1.0, 1.0, 0.0).pow(3.0);
    let alpha = alpha.mul(sun_orientation.smoothstep(-0.5, 1.0));
    atmosphere_material.output_node = Some(vec4_join(vec![atmosphere_color, alpha]));

    (globe_material, atmosphere_material)
}

pub fn init() -> App {
    let timer = Timer::new();

    let mut camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(4.5, 2.0, 3.0);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x000000)));

    // sun

    let sun = DirectionalLight::new(Color::from_hex(0xffffff), 2.0);
    sun.borrow_mut().position.set(0.0, 0.0, 3.0);
    scene.add(&sun);
    let sun_position = sun.borrow().position;

    // textures

    let texture_loader = TextureLoader::new();
    let load = |path: &str| texture_loader.load(examples_dir().join(path)).unwrap();

    let day_texture = load("textures/planets/earth_day_4096.jpg");
    day_texture.set_color_space(ColorSpace::SRGB);
    day_texture.set_anisotropy(8);

    let night_texture = load("textures/planets/earth_night_4096.jpg");
    night_texture.set_color_space(ColorSpace::SRGB);
    night_texture.set_anisotropy(8);

    let bump_roughness_clouds_texture =
        load("textures/planets/earth_bump_roughness_clouds_4096.jpg");
    bump_roughness_clouds_texture.set_anisotropy(8);

    let (globe_material, atmosphere_material) = materials(
        &day_texture,
        &night_texture,
        &bump_roughness_clouds_texture,
        sun_position,
    );

    let sphere_geometry = Rc::new(sphere_geometry(1.0, 64, 64));
    let globe = Mesh::new(sphere_geometry.clone(), globe_material);
    scene.add(&globe);

    // atmosphere

    let atmosphere = Mesh::new(sphere_geometry, atmosphere_material);
    atmosphere.borrow_mut().scale.set(1.04, 1.04, 1.04);
    scene.add(&atmosphere);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 0.1;
    controls.max_distance = 50.0;

    App {
        renderer,
        scene,
        camera,
        controls,
        timer,
        globe,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta();
    app.globe.borrow_mut().rotation.y += delta * 0.025;

    app.controls.update(&mut app.camera, None);

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

/// The controls and the camera at once; see `webgpu_postprocessing_ca`.
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
        .unwrap_or_else(|| "target/webgpu_tsl_earth.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
