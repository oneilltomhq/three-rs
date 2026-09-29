//! Port of `three.js/examples/webgpu_tsl_halftone.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is 0: the
//! timer's elapsed time is 0, so the cyan halftone's direction is
//! `( cos 0, sin 0, -0.2 )` on the graded frame. The harness fires its single
//! RAF once the network is idle, so `Michelle.glb` is always in the scene; the
//! port loads it synchronously where the page's callback would have run.
//!
//! `OrbitControls` is ported, but with damping and no pointer input the first
//! `controls.update()` only aims the camera at the origin.
//!
//! Both halftones are the page's `Fn()`s without a layout, so every call is
//! inlined: `halftones( output )` is a block of two `Output.rgb.assign( … )`
//! statements ending in `Output`, built once per material, as three builds it
//! once per `outputNode`.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::GltfLoader;
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::{
    block, call, float, frag_coord, inline_fn, mix, mod_, normal_world, output_property, rotate,
    uniform_settable, uniform_value, vec4_join, viewport_size,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::{
    sphere_geometry, torus_knot_geometry, AmbientLight, Color, DirectionalLight, Mesh,
    MeshStandardNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene, Timer,
    Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// One entry of the page's `halftoneSettings`.
struct HalftoneSettings {
    count: f64,
    color: u32,
    direction: Vector3,
    start: f64,
    end: f64,
    mix_low: f64,
    mix_high: f64,
    radius: f64,
}

/// `settings.uniforms`.
#[derive(Clone)]
struct HalftoneUniforms {
    count: NodeRef,
    color: NodeRef,
    direction: NodeRef,
    start: NodeRef,
    end: NodeRef,
    mix_low: NodeRef,
    mix_high: NodeRef,
    radius: NodeRef,
}

/// The page's `halftoneSettings` with their `uniforms`, and the
/// `halftones( output )` built from them. Public so `examples/dump_wgsl.rs`
/// builds the same graph the graded frame does.
pub struct Halftones {
    uniforms: Vec<HalftoneUniforms>,
    /// `halftoneSettings[ 1 ].uniforms.direction`.
    pub cyan_direction: SettableValue,
}

impl Halftones {
    pub fn new() -> Self {
        let halftone_settings = [
            // purple shade
            HalftoneSettings {
                count: 140.0,
                color: 0xfb00ff,
                direction: Vector3::new(-0.4, -1.0, 0.5),
                start: 1.0,
                end: 0.0,
                mix_low: 0.0,
                mix_high: 0.5,
                radius: 0.8,
            },
            // cyan highlight
            HalftoneSettings {
                count: 180.0,
                color: 0x94ffd1,
                direction: Vector3::new(0.5, 0.5, -0.2),
                start: 0.55,
                end: 0.2,
                mix_low: 0.5,
                mix_high: 1.0,
                radius: 0.8,
            },
        ];

        let mut directions = Vec::new();
        let uniforms: Vec<HalftoneUniforms> = halftone_settings
            .iter()
            .map(|settings| {
                // `uniform( color( settings.color ) )` — the sRGB hex in the
                // working space.
                let color = Color::from_hex(settings.color);
                let d = settings.direction;
                let (direction, direction_value) =
                    uniform_settable(Type::Vec3, vec![d.x, d.y, d.z]);
                directions.push(direction_value);
                HalftoneUniforms {
                    count: uniform_value(Type::F32, vec![settings.count]),
                    color: uniform_value(Type::Vec3, vec![color.r, color.g, color.b]),
                    direction,
                    start: uniform_value(Type::F32, vec![settings.start]),
                    end: uniform_value(Type::F32, vec![settings.end]),
                    mix_low: uniform_value(Type::F32, vec![settings.mix_low]),
                    mix_high: uniform_value(Type::F32, vec![settings.mix_high]),
                    radius: uniform_value(Type::F32, vec![settings.radius]),
                }
            })
            .collect();

        Self {
            uniforms,
            cyan_direction: directions.pop().expect("two halftones"),
        }
    }

    /// `halftones( output )` — a fresh call per material, as the page makes.
    pub fn output_node(&self) -> NodeRef {
        halftones(&self.uniforms)
    }
}

impl Default for Halftones {
    fn default() -> Self {
        Self::new()
    }
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's `timer`.
    pub timer: Timer,
    /// `halftoneSettings[ 1 ].uniforms.direction`, which `animate()` turns.
    pub cyan_direction: SettableValue,
}

/// The page's `halftone` `Fn`, inlined.
fn halftone(u: &HalftoneUniforms) -> NodeRef {
    // grid pattern

    let grid_uv = frag_coord()
        .xy()
        .div(viewport_size().swizzle("yy"))
        .mul(u.count.clone());
    let grid_uv = mod_(rotate(grid_uv, float(PI * 0.25)), float(1.0));

    // orientation strength

    let orientation_strength = normal_world().dot(u.direction.normalize()).remap_clamp(
        u.end.clone(),
        u.start.clone(),
        float(0.0),
        float(1.0),
    );

    // mask

    let mask = orientation_strength
        .mul(u.radius.clone())
        .mul(float(0.5))
        .step(grid_uv.sub(float(0.5)).length())
        .mul(mix(
            u.mix_low.clone(),
            u.mix_high.clone(),
            orientation_strength,
        ));

    vec4_join(vec![u.color.clone(), mask])
}

/// The page's `halftones( output )`.
fn halftones(settings: &[HalftoneUniforms]) -> NodeRef {
    // `Fn( ( [ input ] ) => … )( output )`: the body is held back and run by
    // `NodeMaterial` setup in each material's own context, so `normalWorld`
    // reads that material's (normal-mapped, `DoubleSide`) `normalView`.
    let settings = settings.to_vec();
    call(
        &inline_fn(0, Type::Vec4, move |_| {
            let halftones_output = output_property();

            let statements = settings
                .iter()
                .map(|settings| {
                    let half_tone_output = halftone(settings);
                    halftones_output.xyz().assign(mix(
                        halftones_output.xyz(),
                        half_tone_output.xyz(),
                        half_tone_output.w(),
                    ))
                })
                .collect();

            block(statements, halftones_output)
        }),
        Vec::new(),
    )
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(6.0, 3.0, 10.0);

    let scene = Scene::new();

    let timer = Timer::new();

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_clear_color(Color::from_hex(0x000000), 1.0);

    // lights

    let ambient_light = AmbientLight::new(Color::from_hex(0xffffff), 3.0);
    scene.add(&ambient_light);

    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 8.0);
    directional_light.borrow_mut().position.set(4.0, 3.0, 1.0);
    scene.add(&directional_light);

    // halftone settings

    let halftones = Halftones::new();

    // default material

    let mut default_material =
        MeshStandardNodeMaterial::standard(Color::from_hex(0xff622e), 1.0, 0.0);
    default_material.output_node = Some(halftones.output_node());

    // objects

    let torus_knot = Mesh::new(
        Rc::new(torus_knot_geometry(0.6, 0.25, 128, 32, 2.0, 3.0)),
        default_material.clone(),
    );
    torus_knot.borrow_mut().position.x = 3.0;
    scene.add(&torus_knot);

    let sphere = Mesh::new(Rc::new(sphere_geometry(1.0, 64, 64)), default_material);
    sphere.borrow_mut().position.x = -3.0;
    scene.add(&sphere);

    let gltf = GltfLoader::load(examples_dir().join("models/gltf/Michelle.glb"))
        .expect("three-rs: Michelle.glb loads");
    let model = gltf.scene.clone();
    {
        let mut object = model.borrow_mut();
        object.position.y = -2.0;
        object.scale.set(2.5, 2.5, 2.5);
    }
    model.traverse(&mut |child| {
        let mut object = child.borrow_mut();
        if !object.is_mesh() {
            return;
        }
        if let Some(material) = object.mesh_mut().and_then(|mesh| mesh.material.as_mut()) {
            material.output_node = Some(halftones.output_node());
        }
    });
    scene.add(&model);

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
        cyan_direction: halftones.cyan_direction,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    app.controls.update(&mut app.camera, None);

    // `halftoneSettings[ 1 ].uniforms.direction.value.x = Math.cos( time )`
    // and `.y = Math.sin( time )`, keeping `z`.
    let time = app.timer.get_elapsed();
    let mut direction = app.cyan_direction.get();
    direction[0] = time.cos();
    direction[1] = time.sin();
    app.cyan_direction.set(direction);

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
        .unwrap_or_else(|| "target/webgpu_tsl_halftone.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
