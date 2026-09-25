//! Port of `three.js/examples/webgpu_tsl_raging_sea.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is 0 — so
//! the `time` uniform is 0 and the sea is caught at its first frame.
//!
//! The waves are the MaterialX library's `mx_noise_float` (#142), summed
//! three times per vertex in the vertex stage and — because the page reads
//! `elevation` again in `emissiveNode` and builds `normalNode` from two more
//! calls — four more times per fragment.
//!
//! `new Inspector()` draws from `Math.random`, but nothing in this page reads
//! the sequence afterwards, so there is no `skip_random_draws` here.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::materialx::mx_noise_float;
use three_rs::nodes::node::FnDef;
use three_rs::nodes::tsl::{
    block, call, camera_view_matrix, float, inline_fn, loop_range_cond, model_normal_matrix,
    position_local, time, to_var, transform_direction, uniform_value, vec3_join,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::{
    plane_geometry, Color, DirectionalLight, Mesh, PerspectiveCamera, Renderer, RendererParameters,
    Scene,
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
}

/// `uniform( color( hex ) )` — `color()` converts the sRGB literal to the
/// working (linear) colour space, as `Color::from_hex` does.
fn uniform_color(hex: u32) -> NodeRef {
    let c = Color::from_hex(hex);
    uniform_value(Type::Vec3, vec![c.r, c.g, c.b])
}

fn uniform_f32(v: f64) -> NodeRef {
    uniform_value(Type::F32, vec![v])
}

/// `node.remap( inLow, inHigh )` — `Remap.js` with its default
/// `outLow = float( 0 )`, `outHigh = float( 1 )` and no clamp:
/// `t.mul( outHigh.sub( outLow ) ).add( outLow )`, the constants left
/// unfolded as three leaves them. Local to the example until `tsl.rs` grows
/// its own.
fn remap(node: NodeRef, in_low: NodeRef, in_high: NodeRef) -> NodeRef {
    let (out_low, out_high) = (float(0.0), float(1.0));
    node.sub(in_low.clone())
        .div(in_high.sub(in_low))
        .mul(out_high.sub(out_low.clone()))
        .add(out_low)
}

/// `transformNormalToView( normal )` — `Normal.js`, without a
/// `modelNormalViewMatrix` in the context:
/// `modelNormalMatrix.mul( normal ).transformNormalByViewMatrix(
/// cameraViewMatrix )`, which prints as three's `transformDirection`.
fn transform_normal_to_view(normal: NodeRef) -> NodeRef {
    transform_direction(camera_view_matrix(), model_normal_matrix().mul(normal))
}

/// The page's uniforms, in declaration order.
struct Waves {
    large_waves_frequency: NodeRef,
    large_waves_speed: NodeRef,
    large_waves_multiplier: NodeRef,
    small_waves_iterations: NodeRef,
    small_waves_frequency: NodeRef,
    small_waves_speed: NodeRef,
    small_waves_multiplier: NodeRef,
}

/// `const wavesElevation = Fn( ( [ position ] ) => { … } )` — no layout, so
/// it is inlined at each of its three call sites.
fn waves_elevation(w: Rc<Waves>) -> Rc<FnDef> {
    inline_fn(1, Type::F32, move |a| {
        let position = a[0].clone();

        // large waves
        //   const elevation = mul(
        //       sin( position.x.mul( largeWavesFrequency.x ).add( time.mul( largeWavesSpeed ) ) ),
        //       sin( position.z.mul( largeWavesFrequency.y ).add( time.mul( largeWavesSpeed ) ) ),
        //       largeWavesMultiplier
        //   ).toVar();
        let elevation = to_var(
            None,
            position
                .x()
                .mul(w.large_waves_frequency.x())
                .add(time().mul(w.large_waves_speed.clone()))
                .sin()
                .mul(
                    position
                        .z()
                        .mul(w.large_waves_frequency.y())
                        .add(time().mul(w.large_waves_speed.clone()))
                        .sin(),
                )
                .mul(w.large_waves_multiplier.clone()),
        );

        //   Loop( { start: float( 1 ), end: smallWavesIterations.add( 1 ) }, ( { i } ) => {
        //       const noiseInput = vec3( position.xz.add( 2 ).mul( smallWavesFrequency ).mul( i ),
        //           time.mul( smallWavesSpeed ) );
        //       const wave = mx_noise_float( noiseInput, 1, 0 )
        //           .mul( smallWavesMultiplier ).div( i ).abs();
        //       elevation.subAssign( wave );
        //   } );
        //
        // `LoopNode` writes a constant bound as an integer literal and
        // converts any other, which is `loop_range_cond`'s contract.
        //
        // The index is an `i32` and three's `format()` converts it to the
        // `f32` the arithmetic wants as `f32( i )` at each use; the port's
        // `wgsl::convert` does not change component type in place, so each
        // use casts it — separately, as a cast shared by both uses would be
        // hoisted into a var three does not have.
        let small = loop_range_cond(
            "i",
            float(1.0),
            w.small_waves_iterations.add(float(1.0)),
            "<",
            |i| {
                let noise_input = vec3_join(vec![
                    position
                        .xz()
                        .add(float(2.0))
                        .mul(w.small_waves_frequency.clone())
                        .mul(i.to(Type::F32)),
                    time().mul(w.small_waves_speed.clone()),
                ]);
                let wave = mx_noise_float(noise_input, float(1.0), float(0.0))
                    .mul(w.small_waves_multiplier.clone())
                    .div(i.to(Type::F32))
                    .abs();
                vec![elevation.sub_assign(wave)]
            },
        );

        //   return elevation;
        block(vec![elevation.clone(), small], elevation)
    })
}

/// The page's material, from `const material = new
/// THREE.MeshStandardNodeMaterial( … )` down to `material.emissiveNode`.
pub fn raging_sea_material() -> MeshBasicNodeMaterial {
    //   const material = new THREE.MeshStandardNodeMaterial( {
    //       color: '#271442',
    //       roughness: 0.15
    //   } );
    let mut material = MeshBasicNodeMaterial::standard(Color::from_hex(0x271442), 0.15, 0.0);

    //   const emissiveColor = uniform( color( '#ff0a81' ) );
    //   const emissiveLow = uniform( - 0.25 );
    //   const emissiveHigh = uniform( 0.2 );
    //   const emissivePower = uniform( 7 );
    let emissive_color = uniform_color(0xff0a81);
    let emissive_low = uniform_f32(-0.25);
    let emissive_high = uniform_f32(0.2);
    let emissive_power = uniform_f32(7.0);

    //   const largeWavesFrequency = uniform( vec2( 3, 1 ) );
    //   … smallWavesMultiplier = uniform( 0.18 );
    //   const normalComputeShift = uniform( 0.01 );
    let waves = Rc::new(Waves {
        large_waves_frequency: uniform_value(Type::Vec2, vec![3.0, 1.0]),
        large_waves_speed: uniform_f32(1.25),
        large_waves_multiplier: uniform_f32(0.15),
        small_waves_iterations: uniform_f32(3.0),
        small_waves_frequency: uniform_f32(2.0),
        small_waves_speed: uniform_f32(0.3),
        small_waves_multiplier: uniform_f32(0.18),
    });
    let normal_compute_shift = uniform_f32(0.01);

    let waves_elevation = waves_elevation(waves);

    // position

    //   const elevation = wavesElevation( positionLocal );
    //   const position = positionLocal.add( vec3( 0, elevation, 0 ) );
    //   material.positionNode = position;
    let elevation = call(&waves_elevation, vec![position_local()]);
    let position = position_local().add(vec3_join(vec![float(0.0), elevation.clone(), float(0.0)]));
    material.position_node = Some(position.clone());

    // normals

    //   let positionA = positionLocal.add( vec3( normalComputeShift, 0, 0 ) );
    //   let positionB = positionLocal.add( vec3( 0, 0, normalComputeShift.negate() ) );
    //   positionA = positionA.add( vec3( 0, wavesElevation( positionA ), 0 ) );
    //   positionB = positionB.add( vec3( 0, wavesElevation( positionB ), 0 ) );
    let position_a = position_local().add(vec3_join(vec![
        normal_compute_shift.clone(),
        float(0.0),
        float(0.0),
    ]));
    let position_b = position_local().add(vec3_join(vec![
        float(0.0),
        float(0.0),
        normal_compute_shift.negate(),
    ]));
    let position_a = position_a.add(vec3_join(vec![
        float(0.0),
        call(&waves_elevation, vec![position_a.clone()]),
        float(0.0),
    ]));
    let position_b = position_b.add(vec3_join(vec![
        float(0.0),
        call(&waves_elevation, vec![position_b.clone()]),
        float(0.0),
    ]));

    //   const toA = positionA.sub( position ).normalize();
    //   const toB = positionB.sub( position ).normalize();
    //   const normal = toA.cross( toB );
    //   material.normalNode = transformNormalToView( normal );
    let to_a = position_a.sub(position.clone()).normalize();
    let to_b = position_b.sub(position).normalize();
    let normal = to_a.cross(to_b);
    material.normal_node = Some(transform_normal_to_view(normal));

    // emissive

    //   const emissive = elevation.remap( emissiveHigh, emissiveLow ).pow( emissivePower );
    //   material.emissiveNode = emissiveColor.mul( emissive );
    let emissive = remap(elevation, emissive_high, emissive_low).pow(emissive_power);
    material.emissive_node = Some(emissive_color.mul(emissive));

    material
}

pub fn init() -> App {
    //   camera = new THREE.PerspectiveCamera( 50, window.innerWidth / window.innerHeight, 0.1, 10 );
    //   camera.position.set( 1.25, 1.25, 1.25 );
    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 10.0);
    camera.node.borrow_mut().position.set(1.25, 1.25, 1.25);

    let scene = Scene::new();

    // lights

    //   const directionalLight = new THREE.DirectionalLight( '#ffffff', 3 );
    //   directionalLight.position.set( - 4, 2, 0 );
    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    directional_light.borrow_mut().position.set(-4.0, 2.0, 0.0);
    scene.add(&directional_light);

    // material

    let material = raging_sea_material();

    // mesh

    //   const geometry = new THREE.PlaneGeometry( 2, 2, 256, 256 );
    //   geometry.rotateX( - Math.PI * 0.5 );
    let mut geometry = plane_geometry(2.0, 2.0, 256, 256);
    geometry.rotate_x(-PI * 0.5);

    let mesh = Mesh::new(Rc::new(geometry), material);
    scene.add(&mesh);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //   controls = new OrbitControls( camera, renderer.domElement );
    //   controls.target.y = - 0.25;
    //   controls.enableDamping = true;
    //   controls.minDistance = 0.1;
    //   controls.maxDistance = 50;
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.y = -0.25;
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
    let _ = app.controls.update(&mut app.camera, None);

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`. The rung harness never calls this.
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
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_tsl_raging_sea.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
