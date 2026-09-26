//! Port of `three.js/examples/webgpu_particles.html`, calling the three-rs API
//! in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is 800 x 500 at a pixel ratio of 1, and
//! `performance.now()` is pinned, so `time` is 0 on the graded frame.
//!
//! **What the graded frame shows.** Every sprite's opacity is
//! `texture.a * ( 1 - life )`, and at `time = 0`
//!
//! ```text
//! lifeTime = ( ( 0 + 5 ) * 0.2 * lifeRange ) mod 1 = lifeRange mod 1 = lifeRange
//! life     = lifeTime / lifeRange                                     = 1
//! ```
//!
//! for every `lifeRange` in `[ 0.1, 1 )`, so all 3000 sprites are drawn with
//! an alpha of 0: the frame is the grid on the `0x333333` background. They are
//! still drawn — 2000 smoke instances through `Mesh.count` and 1000 fire
//! instances through an indexed indirect draw — and `docs/webgpu_particles-progress.md`
//! says what that does and does not prove.
//!
//! `new Inspector()` is ported only as the five `Math.random()` draws its
//! `List`s make before the first render fills the `range()` buffers, exactly
//! as in `webgpu_tsl_galaxy`; its `speed` slider is hidden by `clean-page.js`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::IndirectStorageBufferAttribute;
use three_rs::materials::{instanced_range, Blending, MeshBasicNodeMaterial};
use three_rs::nodes::tsl::{
    float, mix, mod_float, position_local, rotate_uv_about, texture_uv, time, uniform_value, uv,
    vec2, vec3,
};
use three_rs::nodes::Type;
use three_rs::textures::Texture;
use three_rs::{
    plane_geometry, Color, GridHelper, Mesh, PerspectiveCamera, Renderer, RendererParameters,
    Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `smokeInstancedSprite.count = 2000;`
pub const SMOKE_COUNT: usize = 2000;
/// `const fireCount = 1000;`
pub const FIRE_COUNT: usize = 1000;

/// `new Inspector()`'s `Math.random()` draws — see `webgpu_tsl_galaxy`'s
/// `INSPECTOR_RANDOM_DRAWS`: the same five `List`s.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    /// The fire sprite's `IndirectStorageBufferAttribute`.
    pub fire_indirect: IndirectStorageBufferAttribute,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `color( hex )` — a linear-sRGB `vec3` constant.
fn color(hex: u32) -> three_rs::nodes::NodeRef {
    let c = Color::from_hex(hex);
    vec3(c.r, c.g, c.b)
}

/// The page's two `SpriteNodeMaterial`s, from `const lifeRange = range( .1, 1
/// );` down to `fireNodeMaterial.depthWrite = false`. Split out so
/// `tests/nodes_compute_indirect_wgsl.rs` builds the graph the frame draws
/// without a GPU.
pub fn materials(map: &Texture) -> (MeshBasicNodeMaterial, MeshBasicNodeMaterial) {
    // create nodes

    let smoke_count = SMOKE_COUNT;

    // `const lifeRange = range( .1, 1 );` — one buffer, read by both
    // materials. Each `range()` is sized by the object it is first set up on;
    // the smoke mesh is drawn first, with `count = 2000`.
    let life_range = instanced_range(0.1, 1.0, smoke_count);
    // `const offsetRange = range( new THREE.Vector3( - 2, 3, - 2 ), new THREE.Vector3( 2, 5, 2 ) );`
    let offset_range = instanced_range(
        Vector3::new(-2.0, 3.0, -2.0),
        Vector3::new(2.0, 5.0, 2.0),
        smoke_count,
    );

    // `const speed = uniform( .2 );`
    let speed = uniform_value(Type::F32, vec![0.2]);
    // `const scaledTime = time.add( 5 ).mul( speed );`
    let scaled_time = time().add(float(5.0)).mul(speed);

    // `const lifeTime = scaledTime.mul( lifeRange ).mod( 1 );`
    let life_time = mod_float(scaled_time.mul(life_range.clone()), float(1.0));
    let scale_range = instanced_range(0.3, 2.0, smoke_count);
    let rotate_range = instanced_range(0.1, 4.0, smoke_count);

    // `const life = lifeTime.div( lifeRange );`
    let life = life_time.div(life_range);

    // `const fakeLightEffect = positionLocal.y.oneMinus().max( 0.2 );`
    let fake_light_effect = position_local().y().one_minus().max(float(0.2));

    // `const textureNode = texture( map, rotateUV( uv(), scaledTime.mul( rotateRange ) ) );`
    let texture_node = texture_uv(
        map,
        rotate_uv_about(uv(), scaled_time.mul(rotate_range), vec2(0.5, 0.5)),
    );

    // `const opacityNode = textureNode.a.mul( life.oneMinus() );`
    let opacity_node = texture_node.w().mul(life.one_minus());

    // `const smokeColor = mix( color( 0x2c1501 ), color( 0x222222 ), positionLocal.y.mul( 3 ).clamp() );`
    let smoke_color = mix(
        color(0x2c1501),
        color(0x222222),
        position_local()
            .y()
            .mul(float(3.0))
            .clamp(float(0.0), float(1.0)),
    );

    // create particles

    let mut smoke_material = MeshBasicNodeMaterial::sprite();
    smoke_material.color_node = Some(
        mix(
            color(0xf27d0c),
            smoke_color,
            life.mul(float(2.5)).min(float(1.0)),
        )
        .mul(fake_light_effect),
    );
    smoke_material.opacity_node = Some(opacity_node.clone());
    smoke_material.position_node = Some(offset_range.mul(life_time.clone()));
    let scale_node = scale_range.mul(life_time.max(float(0.3)));
    smoke_material.scale_node = Some(scale_node.clone());
    smoke_material.depth_write = false;

    let fire_count = FIRE_COUNT;

    let mut fire_material = MeshBasicNodeMaterial::sprite();
    // `mix( color( 0xb72f17 ), color( 0xb72f17 ), life )`
    fire_material.color_node = Some(mix(color(0xb72f17), color(0xb72f17), life));
    fire_material.position_node = Some(
        instanced_range(
            Vector3::new(-1.0, 1.0, -1.0),
            Vector3::new(1.0, 2.0, 1.0),
            fire_count,
        )
        .mul(life_time),
    );
    fire_material.scale_node = Some(scale_node);
    fire_material.opacity_node = Some(opacity_node.mul(float(0.5)));
    fire_material.blending = Blending::Additive;
    fire_material.transparent = true;
    fire_material.depth_write = false;

    (smoke_material, fire_material)
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 5000.0);
    camera.node.borrow_mut().position.set(1300.0, 500.0, 0.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x333333));

    // textures

    let map = three_rs::TextureLoader::new()
        .load(examples_dir().join("textures/opengameart/smoke1.png"))
        .unwrap();

    let (smoke_material, fire_material) = materials(&map);

    let smoke = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), smoke_material);
    smoke.borrow_mut().scale.set(400.0, 400.0, 400.0);
    smoke.borrow_mut().payload.mesh_mut().unwrap().count = Some(SMOKE_COUNT);
    scene.add(&smoke);

    //

    let mut fire_geometry = plane_geometry(1.0, 1.0, 1, 1);
    let fire_count = FIRE_COUNT;
    // indirect draw ( optional )
    // each indirect draw call is 5 uint32 values for indexes

    let index_count = fire_geometry
        .index
        .as_ref()
        .expect("PlaneGeometry is indexed")
        .count() as u32;
    let fire_indirect = IndirectStorageBufferAttribute::new(
        vec![
            index_count,       // indexCount
            fire_count as u32, // instanceCount
            0,                 // firstIndex
            0,                 // baseVertex
            0,                 // firstInstance
        ],
        5,
    );
    fire_geometry.set_indirect(fire_indirect.clone());

    let fire = Mesh::new(Rc::new(fire_geometry), fire_material);
    {
        let mut fire = fire.borrow_mut();
        fire.scale.set(400.0, 400.0, 400.0);
        fire.payload.mesh_mut().unwrap().count = Some(fire_count);
        fire.position.y = -100.0;
        fire.render_order = 1.0;
    }
    scene.add(&fire);

    //

    let helper = GridHelper::new(
        3000.0,
        40,
        Color::from_hex(0x444444),
        Color::from_hex(0x444444),
    );
    helper.borrow_mut().position.y = -75.0;
    scene.add(&helper);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.inspector = new Inspector();`
    renderer.skip_random_draws(INSPECTOR_RANDOM_DRAWS);

    //

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.max_distance = 2700.0;
    controls.target.set(0.0, 500.0, 0.0);
    controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        controls,
        fire_indirect,
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

/// The controls and the camera at once; see `webgpu_tsl_galaxy`.
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
        .unwrap_or_else(|| "target/webgpu_particles.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
