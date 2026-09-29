//! Port of `three.js/examples/webgpu_instance_points.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! The `webgpu_lines_fat` spline (a `hilbert3D` curve through a
//! `CatmullRomCurve3`), sampled at 256 points, each drawn as one instance of a
//! screen-space quad by a `PointsNodeMaterial` on a `Sprite`. A kernel
//! writes each point's size into a `StorageInstancedBufferAttribute`, which
//! the material reads back as an instanced attribute for both its size and
//! its brightness. The frame is rendered twice, as in `webgpu_lines_fat`: the
//! main view, then a 125 x 125 inset in the top left, drawn over it with its
//! own background.
//!
//! **The graded time is zero**, so the kernel's `time.add( instanceIndex )` is
//! the instance index alone and every size is fixed by it.
//!
//! The `Inspector` panel is DOM the harness hides; the parameters it would
//! edit are plain uniforms here. The `resize` listener does not fire, but the
//! page calls `onWindowResize()` once from `init()`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::geometry_utils::hilbert_3d_default;
use three_rs::materials::PointsNodeMaterial;
use three_rs::math::ColorSpace;
use three_rs::nodes::node::Type;
use three_rs::nodes::tsl::{
    color, instance_index, instanced_data_attribute, mix, shape_circle, storage_f32, time,
    uniform_value, vec3,
};
use three_rs::nodes::{ComputeFlow, NodeRef};
use three_rs::{
    Background, CatmullRomCurve3, Color, Curve, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Sprite, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `insetWidth = insetHeight = window.innerHeight / 4` — the value the
/// `onWindowResize()` call at the end of `init()` writes.
pub const INSET: f64 = INNER_HEIGHT / 4.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub camera2: PerspectiveCamera,
    pub background_node: NodeRef,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// `computeSize`, dispatched at the top of every `animate()`.
    pub compute_size: ComputeFlow,
    /// `window.innerWidth` / `window.innerHeight`, which `animate()` reads.
    pub inner_width: f64,
    pub inner_height: f64,
    /// The page's module-level `insetWidth` / `insetHeight`.
    pub inset_width: f64,
    pub inset_height: f64,
}

/// The page's `effectController`: three `uniform()`s the (hidden) inspector
/// would edit.
struct EffectController {
    pulse_speed: NodeRef,
    min_width: NodeRef,
    max_width: NodeRef,
}

/// What `init()` builds between `backgroundNode` and `new THREE.Sprite()`.
pub struct InstancedPoints {
    /// `computeSize`.
    pub compute_size: ComputeFlow,
    /// The `PointsNodeMaterial`.
    pub material: PointsNodeMaterial,
    /// `divisions` — the instance count.
    pub divisions: usize,
}

/// The spline's points, the size kernel and the material that reads it.
pub fn instanced_points() -> InstancedPoints {
    let effect_controller = EffectController {
        pulse_speed: uniform_value(Type::F32, vec![6.0]),
        min_width: uniform_value(Type::F32, vec![6.0]),
        max_width: uniform_value(Type::F32, vec![20.0]),
    };

    // Position and THREE.Color Data
    let points = hilbert_3d_default(Vector3::new(0.0, 0.0, 0.0), 20.0, 1);
    let spline = CatmullRomCurve3::new(points.clone());
    // `Math.round( 4 * points.length )` — 4 * 64.
    let divisions = (4.0 * points.len() as f64).round() as usize;
    let mut point_color = Color::default();

    let mut positions: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut colors: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut sizes: Vec<f32> = vec![0.0; divisions];

    for (i, size) in sizes.iter_mut().enumerate() {
        let t = i as f64 / divisions as f64;

        let point = spline.get_point(t);
        positions.extend([point.x as f32, point.y as f32, point.z as f32]);

        point_color.set_hsl(t, 1.0, 0.5, ColorSpace::Srgb);
        colors.extend([
            point_color.r as f32,
            point_color.g as f32,
            point_color.b as f32,
        ]);

        *size = 10.0;
    }

    // Instanced Points
    let position_attribute = instanced_data_attribute(&Rc::new(positions), 3, 0, Type::Vec3);
    let colors_attribute = instanced_data_attribute(&Rc::new(colors), 3, 0, Type::Vec3);

    // `new StorageInstancedBufferAttribute( sizes, 1 )` and `storage( it,
    // 'float', count )`: one GPU buffer, written by the kernel and read by the
    // material as an instanced attribute.
    let instance_size_storage = storage_f32(&sizes, Type::F32);

    let compute_size = {
        let EffectController {
            pulse_speed,
            min_width,
            max_width,
        } = &effect_controller;

        let relative_time = time().add(instance_index().to(Type::F32));
        let size_factor = relative_time
            .mul(pulse_speed.clone())
            .sin()
            .add(1.0)
            .div(2.0);

        ComputeFlow {
            statements: vec![instance_size_storage.element(instance_index()).assign(
                size_factor
                    .mul(max_width.sub(min_width.clone()))
                    .add(min_width.clone()),
            )],
            count: divisions,
            workgroup_size: [64, 1, 1],
            name: None,
            on_init: None,
        }
    };

    // Material / Sprites
    //
    // `instancedBufferAttribute( instanceSizeBufferAttribute )`.
    let attribute_range = instance_size_storage.to_attribute();
    let point_colors = mix(
        vec3(0.0, 0.0, 0.0),
        colors_attribute,
        attribute_range.div(effect_controller.max_width.clone()),
    );

    let mut material = PointsNodeMaterial::points();
    material.color_node = Some(point_colors);
    material.opacity_node = Some(shape_circle());
    material.position_node = Some(position_attribute);
    material.size_node = Some(attribute_range);
    // `vertexColors: true` changes nothing here: `NodeMaterial.setupDiffuseColor()`
    // multiplies by `vertexColor()` only when the geometry has a `color`
    // attribute, and the sprite quad has none. So it is left off.
    material.size_attenuation = false;
    material.alpha_to_coverage = true;

    InstancedPoints {
        compute_size,
        material,
        divisions,
    }
}

pub fn init() -> App {
    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x000000)));

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(-40.0, 0.0, 60.0);

    // `camera2.position.copy( camera.position )`.
    let camera2 = PerspectiveCamera::new(40.0, 1.0, 1.0, 1000.0);
    camera2.node.borrow_mut().position.set(-40.0, 0.0, 60.0);

    let background_node = color(0x222222);

    let InstancedPoints {
        compute_size,
        material,
        divisions,
    } = instanced_points();

    let instanced_points = Sprite::new(material);
    if let three_rs::objects::Payload::Sprite(sprite) = &mut instanced_points.borrow_mut().payload {
        sprite.count = divisions;
    }
    scene.add(&instanced_points);

    // Renderer / Controls
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // The constructor's own `update()` aims the camera at the default target.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 10.0;
    controls.max_distance = 500.0;

    let mut app = App {
        renderer,
        scene,
        camera,
        camera2,
        background_node,
        controls,
        compute_size,
        inner_width: INNER_WIDTH,
        inner_height: INNER_HEIGHT,
        inset_width: INSET,
        inset_height: INSET,
    };
    // `onWindowResize()`, called once at the end of `init()`.
    resize(&mut app, INNER_WIDTH, INNER_HEIGHT);
    app
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // compute
    app.renderer
        .compute(&app.compute_size)
        .expect("three-rs: computeSize dispatches");

    // main scene
    app.renderer
        .set_viewport(0.0, 0.0, app.inner_width, app.inner_height);

    app.controls.update(&mut app.camera, None);

    app.renderer.auto_clear = true;
    // `scene.backgroundNode = null` — back to `scene.background`.
    app.scene.background = Some(Background::Color(Color::from_hex(0x000000)));
    app.renderer.render(&mut app.scene, &mut app.camera);

    // inset scene
    let pos_y = app.inner_height - app.inset_height - 20.0;

    app.renderer.clear_depth(); // important!
    app.renderer.set_scissor_test(true);
    app.renderer
        .set_scissor(20.0, pos_y, app.inset_width, app.inset_height);
    app.renderer
        .set_viewport(20.0, pos_y, app.inset_width, app.inset_height);

    {
        let camera = app.camera.node.borrow();
        let mut camera2 = app.camera2.node.borrow_mut();
        camera2.position = camera.position;
        camera2.quaternion = camera.quaternion;
    }

    app.renderer.auto_clear = false;
    app.scene.background = Some(Background::Node(app.background_node.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera2);

    app.renderer.set_scissor_test(false);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.inner_width = width;
    app.inner_height = height;

    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);

    // `insetWidth = window.innerHeight / 4; // square`
    app.inset_width = height / 4.0;
    app.inset_height = height / 4.0;

    app.camera2.aspect = app.inset_width / app.inset_height;
    app.camera2.update_projection_matrix();
}

/// The example's controls.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
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
        .unwrap_or_else(|| "target/webgpu_instance_points.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
