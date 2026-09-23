//! Port of `three.js/examples/webgpu_tsl_galaxy.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is 0 — so the
//! `time` uniform is 0 and the galaxy is unrotated.
//!
//! `OrbitControls` is ported, but it moves nothing here: with `enableDamping`
//! on, `controls.update()` on the first frame leaves the camera exactly where
//! `camera.position.set()` put it, and the page never calls
//! `controls.target.set()`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::{instanced_range, MeshBasicNodeMaterial};
use three_rs::nodes::tsl::{
    float, length, mix, time, two_pi, uniform_value, uv, vec3_join, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::{
    plane_geometry, Color, InstancedMesh, PerspectiveCamera, Renderer, RendererParameters, Scene,
    Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `new THREE.InstancedMesh( new THREE.PlaneGeometry( 1, 1 ), material, 20000 )`.
pub const COUNT: usize = 20000;

/// `const branches = 3;`
const BRANCHES: f64 = 3.0;

/// `Math.random()` draws the page makes during `init()` *after* the material's
/// `range()` nodes are built but *before* the graded frame, which is when
/// `RangeNode.setup()` fills them. `new Inspector()` is the only thing in the
/// page that touches `Math.random`: its tabs each build a
/// `List`, whose constructor ends with
///
/// ```js
/// this.id = `list-${Math.random().toString( 36 ).slice( 2, 11 )}`;
/// ```
///
/// (`examples/jsm/inspector/ui/List.js:11`). three.js' own `generateUUID()`
/// does not count — the harness rewrites it to the unseeded `Math._random`.
/// The count is pinned by `tests/nodes_range_buffers.rs` against the four range
/// buffers three.js actually uploads for this page.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

/// The page's material graph, from `const material = new
/// THREE.SpriteNodeMaterial( … )` down to `material.colorNode = …`. Split out so
/// `examples/dump_wgsl.rs` and `tests/nodes_range_buffers.rs` build the same
/// graph the graded frame does.
pub fn galaxy_material() -> MeshBasicNodeMaterial {
    // galaxy

    //   const material = new THREE.SpriteNodeMaterial( {
    //       depthWrite: false,
    //       blending: THREE.AdditiveBlending
    //   } );
    let mut material = MeshBasicNodeMaterial::sprite();
    material.depth_write = false;
    material.blending = three_rs::materials::Blending::Additive;

    //   const size = uniform( 0.08 );
    //   material.scaleNode = range( 0, 1 ).mul( size );
    let size = uniform_value(Type::F32, vec![0.08]);
    material.scale_node = Some(instanced_range(0.0, 1.0, COUNT).mul(size));

    //   const radiusRatio = range( 0, 1 );
    //   const radius = radiusRatio.pow( 1.5 ).mul( 5 ).toVar();
    let radius_ratio = instanced_range(0.0, 1.0, COUNT);
    let radius = radius_ratio.pow(float(1.5)).mul(float(5.0));
    let radius = three_rs::nodes::tsl::to_var(None, radius);

    //   const branchAngle = range( 0, branches ).floor().mul( TWO_PI.div( branches ) );
    //   const angle = branchAngle.add( time.mul( radiusRatio.oneMinus() ) );
    let branch_angle = instanced_range(0.0, BRANCHES, COUNT)
        .floor()
        .mul(two_pi().div(float(BRANCHES)));
    let angle = branch_angle.add(time().mul(radius_ratio.one_minus()));

    //   const position = vec3( cos( angle ), 0, sin( angle ) ).mul( radius );
    let position = vec3_join(vec![angle.cos(), float(0.0), angle.sin()]).mul(radius);

    //   const randomOffset = range( vec3( - 1 ), vec3( 1 ) ).pow3()
    //       .mul( radiusRatio ).add( 0.2 );
    let random_offset = instanced_range(
        Vector3::new(-1.0, -1.0, -1.0),
        Vector3::new(1.0, 1.0, 1.0),
        COUNT,
    )
    .pow3()
    .mul(radius_ratio.clone())
    .add(float(0.2));

    //   material.positionNode = position.add( randomOffset );
    material.position_node = Some(position.add(random_offset));

    //   const colorInside = uniform( color( '#ffa575' ) );
    //   const colorOutside = uniform( color( '#311599' ) );
    let inside = Color::from_hex(0xffa575);
    let outside = Color::from_hex(0x311599);
    let color_inside = uniform_value(Type::Vec3, vec![inside.r, inside.g, inside.b]);
    let color_outside = uniform_value(Type::Vec3, vec![outside.r, outside.g, outside.b]);

    //   const colorFinal = mix( colorInside, colorOutside,
    //       radiusRatio.oneMinus().pow( 2 ).oneMinus() );
    let color_final = mix(
        color_inside,
        color_outside,
        radius_ratio.one_minus().pow(float(2.0)).one_minus(),
    );

    //   const alpha = float( 0.1 ).div( uv().sub( 0.5 ).length() ).sub( 0.2 );
    let alpha = float(0.1).div(length(uv().sub(float(0.5)))).sub(float(0.2));

    //   material.colorNode = vec4( colorFinal, alpha );
    material.color_node = Some(vec4_join(vec![color_final, alpha]));

    material
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(4.0, 2.0, 5.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x201919));

    let material = galaxy_material();

    let mesh = InstancedMesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material, COUNT);
    scene.add(&mesh);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `renderer.inspector = new Inspector()` — the only effect it has on the
    // graded frame is the shared `Math.random` sequence it advances.
    renderer.skip_random_draws(INSPECTOR_RANDOM_DRAWS);

    // `controls = new OrbitControls( camera, renderer.domElement );` — with the
    // default `target` of ( 0, 0, 0 ), the `update()` its constructor ends with
    // aims the camera at the origin.
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
    }
}

/// The page's `animate()`: `controls.update()` then `renderer.render()`.
pub fn animate(app: &mut App) {
    // `controls.update();`
    let _ = app.controls.update(&mut app.camera, None);

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
        .unwrap_or_else(|| "target/webgpu_tsl_galaxy.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
