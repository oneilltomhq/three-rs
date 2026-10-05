//! Port of `three.js/examples/webgpu_skinning_points.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! Every vertex of Michelle becomes one instance of a screen-space quad: a
//! kernel skins the vertex (`computeSkinning( child )`), moves it to world
//! space (`objectWorldMatrix( child )`) and writes it and its speed into two
//! `instancedArray`s, which the `PointsNodeMaterial` on a `Sprite` reads back
//! as instanced attributes (`.toAttribute()`). The kernel is the material's
//! `positionNode`, so the renderer dispatches it once per frame before the
//! pass; its `onInit` runs a second, identical kernel once before that.
//!
//! **The graded time is zero**, as in `webgpu_skinning`: `mixer.update( 0 )`
//! poses the first keyframe. The `onInit` kernel writes the skinned position
//! with a speed of `position − 0`; the per-frame kernel then runs on the same
//! pose and writes a speed of exactly zero. So every point is `0x0066ff` and
//! `exp( 0 ) * 5 + 1` = 6 pixels across.
//!
//! `GLTFLoader.load()` is asynchronous on the page; the harness fires its RAF
//! only once the network is idle, so the load is synchronous here. The
//! `resize` listener does not fire.

use std::f64::consts::PI;

use three_rs::addons::controls::OrbitControls;
use three_rs::animation::AnimationMixer;
use three_rs::loaders::GltfLoader;
use three_rs::materials::PointsNodeMaterial;
use three_rs::nodes::node::Type;
use three_rs::nodes::skinning::compute_skinning;
use three_rs::nodes::tsl::{
    color, compute_node, exp, float, instance_index, instanced_array, object_world_matrix,
    shape_circle, vec4_join, StorageArray,
};
use three_rs::nodes::{ComputeFlow, NodeRef};
use three_rs::Timer;
use three_rs::{
    AmbientLight, Background, Color, ObjectRef, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Sprite,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's module-level `mixer`.
    pub mixer: AnimationMixer,
    /// The page's module-level `timer`.
    pub timer: Timer,
}

/// `updateSkinningPoints` — `Fn( () => { … }, 'void' )`. Three builds the
/// body afresh at every call, so `computeSkinning( child )` (the
/// `skinningPosition` the closure captured) is the *same* node in both
/// kernels; only the storage copies it makes are per build. The port builds
/// the node once per call of this function, which the page does twice (the
/// per-frame kernel and its `onInit`) — see `compute_skinning()`.
fn update_skinning_points(
    child: &ObjectRef,
    skinning_position: &NodeRef,
    point_position_array: &StorageArray,
    point_speed_array: &StorageArray,
) -> Vec<NodeRef> {
    let point_position = point_position_array.element(instance_index());
    let point_speed = point_speed_array.element(instance_index());

    // `objectWorldMatrix( child ).mul( skinningPosition )` — a `mat4` times a
    // `vec3`, which `NodeBuilder.format()` pads with 1.0.
    let skinning_world_position =
        object_world_matrix(child).mul(vec4_join(vec![skinning_position.clone(), float(1.0)]));

    // `skinningWorldPosition.sub( pointPosition )`: a `vec4` minus a `vec3`,
    // the `vec3` padded the same way.
    let skinning_speed =
        skinning_world_position.sub(vec4_join(vec![point_position.clone(), float(1.0)]));

    vec![
        point_speed.assign(skinning_speed.xyz()),
        point_position.assign(skinning_world_position.xyz()),
    ]
}

/// One mesh's point cloud: the two `instancedArray`s, the material that reads
/// them, and the kernels (as its `positionNode`) that write them — the body of
/// the page's `traverse` callback between `countOfPoints` and `new Sprite()`.
pub fn point_cloud_material(child: &ObjectRef, count_of_points: usize) -> PointsNodeMaterial {
    let (update, point_position_array, point_speed_array) = kernels(child, count_of_points);
    material(update, &point_position_array, &point_speed_array)
}

/// The per-frame kernel (with its `onInit` twin) and the arrays it writes.
pub fn kernels(
    child: &ObjectRef,
    count_of_points: usize,
) -> (ComputeFlow, StorageArray, StorageArray) {
    // `.setPBO( true )` is a WebGL-backend hint; WebGPU ignores it.
    let point_position_array = instanced_array(count_of_points, Type::Vec3);
    let point_speed_array = instanced_array(count_of_points, Type::Vec3);

    // `onInit( () => renderer.compute( updateSkinningPoints().compute(
    // countOfPoints ) ) )` — the same body as its own kernel.
    let on_init = ComputeFlow::new(
        update_skinning_points(
            child,
            &compute_skinning(child),
            &point_position_array,
            &point_speed_array,
        ),
        count_of_points,
    );
    // `Fn( () => { updateSkinningPoints(); return
    // pointPositionArray.toAttribute(); } )().compute( countOfPoints )`.
    let mut update = ComputeFlow::new(
        update_skinning_points(
            child,
            &compute_skinning(child),
            &point_position_array,
            &point_speed_array,
        ),
        count_of_points,
    );
    update.on_init = Some(Box::new(on_init));
    (update, point_position_array, point_speed_array)
}

/// `materialPoints`, with `update` as its `positionNode`.
pub fn material(
    update: ComputeFlow,
    point_position_array: &StorageArray,
    point_speed_array: &StorageArray,
) -> PointsNodeMaterial {
    let point_speed_attribute = point_speed_array.to_attribute();

    let mut material_points = PointsNodeMaterial::points();
    material_points.color_node = Some(
        point_speed_attribute
            .mul(0.6)
            .mix(color(0x0066ff), color(0xff9000)),
    );
    material_points.opacity_node = Some(shape_circle());
    material_points.size_node = Some(
        exp(point_speed_attribute.length())
            .min(5.0)
            .mul(5.0)
            .add(1.0),
    );
    material_points.size_attenuation = false;
    material_points.alpha_test = 0.5;

    material_points.position_node = Some(compute_node(update, point_position_array.to_attribute()));
    material_points
}

pub fn init() -> App {
    let three = three_rs::testing::three_js_dir();

    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 300.0, -85.0);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x111111)));
    camera.look_at(&three_rs::Vector3::new(0.0, 0.0, -85.0));

    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 10.0));

    let gltf = GltfLoader::load(three.join("examples/models/gltf/Michelle.glb"))
        .expect("three-rs: Michelle.glb loads");
    let object = gltf.scene.clone();

    let mut mixer = AnimationMixer::new(Box::new(gltf.scene_resolver()));
    let action = mixer.clip_action(&gltf.animations[0], None, None);
    mixer.play(action);

    let mut meshes = Vec::new();
    object.traverse(&mut |child| {
        if child.borrow().is_mesh() {
            meshes.push(child.clone());
        }
    });
    for child in meshes {
        child.borrow_mut().visible = false;

        let count_of_points = match &child.borrow().payload {
            three_rs::objects::Payload::SkinnedMesh(skinned) => skinned
                .mesh
                .geometry
                .get_attribute("position")
                .expect("three-rs: Michelle has positions")
                .count(),
            _ => panic!("three-rs: Michelle's meshes are skinned"),
        };

        let material_points = point_cloud_material(&child, count_of_points);

        let point_cloud = Sprite::new(material_points);
        if let three_rs::objects::Payload::Sprite(sprite) = &mut point_cloud.borrow_mut().payload {
            sprite.count = count_of_points;
        }
        scene.add(&point_cloud);
    }

    object.borrow_mut().scale.set(100.0, 100.0, 100.0);
    object.borrow_mut().set_rotation(-PI / 2.0, 0.0, 0.0);

    scene.add(&object);

    // renderer
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        timer: Timer::new(),
        renderer,
        scene,
        camera,
        mixer,
    }
}

/// The page's animation loop. `timer.getDelta()` is 0 on the graded frame and
/// on every steady frame after it, so the pose — and so the kernel's output —
/// never changes.
pub fn animate(app: &mut App) {
    app.timer.update();
    let delta = app.timer.get_delta();

    app.mixer.update(delta);
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls. `None` here: the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once. `None` here: the page creates none.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_skinning_points.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
