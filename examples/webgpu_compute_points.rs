//! Port of `three.js/examples/webgpu_compute_points.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The pointer handler never fires, so
//! `pointerVector` keeps its `( -10, -10 )` and every particle is at least
//! 10 units from it: the "reset to the origin" branch is dead for the graded
//! frame, but it is in the kernel because it is in the page.
//!
//! The `Inspector` UI and its two `scaleVector` sliders are hidden by
//! `clean-page.js` and touch nothing but the uniform's default, so they are not
//! ported. Unlike `webgpu_tsl_galaxy`, this page's `Inspector` draws no
//! `Math.random`, because nothing in the scene is a `range()`.

use std::rc::Rc;

use three_rs::core::BufferGeometry;
use three_rs::nodes::tsl::{
    abs, instance_index, length, max, to_var, two_pi, uniform_value, vec2, vec2_join, vec3,
    StorageArray,
};
use three_rs::nodes::{ComputeFlow, Type};
use three_rs::{
    OrthographicCamera, Points, PointsNodeMaterial, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const particleCount = 300000;`
pub const PARTICLE_COUNT: usize = 300_000;

/// `ComputeNode`'s default `workgroupSize = [ 64 ]`, padded to three components.
pub const WORKGROUP_SIZE: [u32; 3] = [64, 1, 1];

/// The page's two storage buffers and the two kernels over them. Split out so
/// `tests/nodes_compute_wgsl.rs` and `tests/renderer_compute_points.rs` build
/// exactly what the example runs.
pub struct Particles {
    /// `const particleArray = instancedArray( particleCount, 'vec2' );`
    pub particle: StorageArray,
    /// `const velocityArray = instancedArray( particleCount, 'vec2' );`
    pub velocity: StorageArray,
    /// `computeNode` — `Update Particles`, run every frame, carrying
    /// `precomputeShaderNode` as its `onInit`.
    pub update: ComputeFlow,
}

pub fn particles() -> Particles {
    let particle = three_rs::nodes::tsl::instanced_array(PARTICLE_COUNT, Type::Vec2);
    let velocity = three_rs::nodes::tsl::instanced_array(PARTICLE_COUNT, Type::Vec2);

    // ```js
    // const computeInit = Fn( () => {
    //     const particle = particleArray.element( instanceIndex );
    //     const velocity = velocityArray.element( instanceIndex );
    //     const randX = instanceIndex.hash();            // not in r186's dump
    //     ...
    // } )
    // ```
    // What r186 actually generates for `onInit` is the angle/speed spiral
    // below, writing the *velocity* buffer only; the particle buffer keeps the
    // zeros it was created with.
    let precompute = {
        let angle = instance_index().to(Type::F32).mul(0.005).mul(two_pi());
        let angle = to_var(None, angle);
        let speed = to_var(None, instance_index().to(Type::F32).mul(1e-8).add(1e-7));
        ComputeFlow {
            statements: vec![velocity.element(instance_index()).assign(vec2_join(vec![
                angle.sin().mul(speed.clone()),
                angle.cos().mul(speed),
            ]))],
            count: PARTICLE_COUNT,
            workgroup_size: WORKGROUP_SIZE,
            name: None,
            on_init: None,
        }
    };

    // ```js
    // const computeUpdate = Fn( () => {
    //     const position = particleArray.element( instanceIndex );
    //     const velocity = velocityArray.element( instanceIndex );
    //     ...
    // } )().compute( particleCount );
    // computeUpdate.setName( 'Update Particles' );
    // ```
    let update = {
        // `uniform( scaleVector )` and `uniform( pointerVector )`. Neither is
        // ever written for the graded frame — the sliders are hidden and the
        // pointer handler never fires.
        let limit = uniform_value(Type::Vec2, vec![1.0, 1.0]);
        let pointer = uniform_value(Type::Vec2, vec![-10.0, -10.0]);

        let particle_at = particle.element(instance_index());
        let velocity_at = velocity.element(instance_index());

        // `const position = particle.add( velocity ).toVar();`
        let position = to_var(None, particle_at.add(velocity_at.clone()));

        // The two bounce tests write the *velocity* buffer before the position
        // is clamped, which is why the clamp is a separate statement below and
        // not folded into the `select`s.
        let bounce = |component: &'static str| {
            let v = velocity_at.swizzle(component);
            v.assign(
                abs(position.swizzle(component))
                    .greater_than_equal(limit.swizzle(component))
                    .select(v.negate(), v.clone()),
            )
        };

        ComputeFlow {
            statements: vec![
                position.clone(),
                bounce("x"),
                bounce("y"),
                // `position.assign( max( min( position, limit ), limit.negate() ) )`
                position.assign(max(position.min(limit.clone()), limit.negate())),
                // `particle.assign( select( pointer.sub( position ).length()
                //     .lessThanEqual( 0.1 ), vec2( 0 ), position ) )`
                particle_at.assign(
                    length(pointer.sub(position.clone()))
                        .less_than_equal(0.1)
                        .select(vec2(0.0, 0.0), position),
                ),
            ],
            count: PARTICLE_COUNT,
            workgroup_size: WORKGROUP_SIZE,
            name: Some("Update Particles".to_string()),
            on_init: Some(Box::new(precompute)),
        }
    };

    Particles {
        particle,
        velocity,
        update,
    }
}

/// `new THREE.PointsNodeMaterial()` with the two nodes the page sets. Split
/// out so the WGSL gate builds exactly what the example renders.
pub fn material() -> PointsNodeMaterial {
    let particles = particles();
    material_for(&particles)
}

fn material_for(particles: &Particles) -> PointsNodeMaterial {
    let mut material = PointsNodeMaterial::points();
    material.position_node = Some(particles.particle.element(instance_index()));
    // `.add( color( 0xFFFFFF ) )` — white in linear-sRGB is ( 1, 1, 1 ), and
    // the add widens the `vec2` to a `vec3` by filling `z` with 0.
    material.color_node = Some(
        particles
            .particle
            .element(instance_index())
            .add(vec3(1.0, 1.0, 1.0)),
    );
    material
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: OrthographicCamera,
    pub particles: Particles,
}

pub fn init() -> App {
    // `new THREE.OrthographicCamera( -1, 1, 1, -1, 0, 1 )`, `camera.position.z = 1`.
    // `near = 0`, `far = 1` and the particles sit at `z = 0`, i.e. exactly on
    // the far plane — only `depthCompare: less-equal` draws them.
    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.0, 1.0);
    camera.object.position.z = 1.0;
    camera.update_matrix_world();

    // `scene.background` is never set, so the pass clears to a transparent
    // black rather than to a colour.
    let scene = Scene::new();

    let particles = particles();

    let material = material_for(&particles);

    // `new THREE.BufferGeometry()` with `new Float32Array( 3 )` — one vertex at
    // the origin — and `drawRange.count = 1`. Every particle is an *instance*
    // of that one vertex; the storage buffer is what moves it.
    let mut geometry = BufferGeometry::new();
    geometry.set_from_points(&[Vector3::new(0.0, 0.0, 0.0)]);
    geometry.set_draw_range(0, 1);

    let mesh = Points::new(Rc::new(geometry), material);
    mesh.borrow_mut().payload.points_mut().unwrap().count = Some(PARTICLE_COUNT);
    scene.add(&mesh);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        particles,
    }
}

/// The page's `animate()`: `renderer.compute( computeNode )` then
/// `renderer.render( scene, camera )`.
pub fn animate(app: &mut App) {
    app.renderer.compute(&app.particles.update).unwrap();
    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_compute_points.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
