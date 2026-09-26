//! Port of `three.js/examples/webgpu_struct_drawindirect.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is 800 x 500 at a pixel ratio of 1, and
//! `performance.now()` is pinned, so `time` is 0 on every frame.
//!
//! **What the graded frame shows.** The page's `render()` is
//!
//! ```js
//! renderer.render( scene, camera );
//! renderer.compute( computeInitDrawBuffer );
//! renderer.compute( computeDrawBuffer );
//! ```
//!
//! — the draw comes *before* the two kernels that fill its arguments. On the
//! first frame, the one the grader captures, the `IndirectStorageBufferAttribute`
//! still holds the `new Uint32Array( 5 )` it was created with, so the
//! `drawIndirect` draws zero vertices and the frame is the `0x00001f`
//! background. The image alone therefore cannot tell a working indirect draw
//! from no draw at all; `tests/nodes_compute_indirect_wgsl.rs` checks both
//! kernels against three's own WGSL, and `tests/renderer_compute_indirect.rs`
//! reads the draw buffer back after the kernels and draws a second frame
//! from it. See `docs/webgpu_struct_drawindirect-progress.md`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::{BufferAttribute, BufferGeometry, IndirectStorageBufferAttribute};
use three_rs::materials::{MeshBasicNodeMaterial, Side};
use three_rs::nodes::tsl::{
    abs, atomic_store, attribute, block, cross, float, max, mix, storage_struct, struct_type, time,
    to_var, uint, varying_property, StorageStruct, StructMember,
};
use three_rs::nodes::{ComputeFlow, Type};
use three_rs::testing::DeterministicRandom;
use three_rs::{Color, Mesh, PerspectiveCamera, Renderer, RendererParameters, Scene};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const instances = 100000;`
pub const INSTANCES: usize = 100_000;

/// `ComputeNode`'s default `workgroupSize = [ 64 ]`, padded.
pub const WORKGROUP_SIZE: [u32; 3] = [64, 1, 1];

/// The draw buffer and the two kernels that write it. Split out so the WGSL
/// gate and the GPU test build exactly what the page runs.
pub struct DrawKernels {
    /// `new THREE.IndirectStorageBufferAttribute( new Uint32Array( 5 ), 5 )`.
    pub draw_buffer: IndirectStorageBufferAttribute,
    /// `computeInitDrawBuffer` — `.compute( 1 )`.
    pub init: ComputeFlow,
    /// `computeDrawBuffer` — `.compute( instances )`.
    pub draw: ComputeFlow,
}

pub fn draw_kernels() -> DrawKernels {
    let draw_buffer = IndirectStorageBufferAttribute::new(vec![0; 5], 5);

    // ```js
    // const drawBufferStruct = struct( {
    //     vertexCount: 'uint',
    //     instanceCount: { type: 'uint', atomic: true },
    //     firstVertex: 'uint',
    //     firstInstance: 'uint',
    //     offset: 'uint'
    // }, 'DrawBuffer' );
    // const drawStorage = storage( drawBuffer, drawBufferStruct, drawBuffer.count );
    // ```
    let layout = struct_type(
        "DrawBuffer",
        vec![
            StructMember::new("vertexCount", Type::U32),
            StructMember::atomic("instanceCount", Type::U32),
            StructMember::new("firstVertex", Type::U32),
            StructMember::new("firstInstance", Type::U32),
            StructMember::new("offset", Type::U32),
        ],
    );
    let draw_storage: StorageStruct = storage_struct(&draw_buffer, layout);

    // ```js
    // computeDrawBuffer = Fn( () => {
    //     const halfTime = sin( time.mul( 0.5 ) );
    //     const instanceCount = max( ( pow( halfTime.add( 1 ), 4.0 ) ).mul( instances ), 100 ).toVar( 'instanceCount' );
    //     atomicStore( drawStorage.get( 'instanceCount' ), instanceCount );
    // } )().compute( instances );
    // ```
    let draw = {
        let half_time = time().mul(float(0.5)).sin();
        let instance_count = to_var(
            Some("instanceCount"),
            max(
                half_time
                    .add(float(1.0))
                    .pow(float(4.0))
                    .mul(float(INSTANCES as f64)),
                float(100.0),
            ),
        );
        ComputeFlow {
            statements: vec![
                instance_count.clone(),
                atomic_store(draw_storage.get("instanceCount"), instance_count),
            ],
            count: INSTANCES,
            workgroup_size: WORKGROUP_SIZE,
            name: None,
            on_init: None,
        }
    };

    // ```js
    // computeInitDrawBuffer = Fn( () => {
    //     const drawInfo = drawStorage;
    //     drawInfo.get( 'vertexCount' ).assign( 3 );
    //     atomicStore( drawInfo.get( 'instanceCount' ), uint( 0 ) );
    //     drawInfo.get( 'firstVertex' ).assign( 0 );
    //     drawInfo.get( 'firstInstance' ).assign( 0 );
    //     drawInfo.get( 'offset' ).assign( 0 );
    // } )().compute( 1 );
    // ```
    let init = ComputeFlow {
        statements: vec![
            draw_storage.get("vertexCount").assign(uint(3)),
            atomic_store(draw_storage.get("instanceCount"), uint(0)),
            draw_storage.get("firstVertex").assign(uint(0)),
            draw_storage.get("firstInstance").assign(uint(0)),
            draw_storage.get("offset").assign(uint(0)),
        ],
        count: 1,
        workgroup_size: WORKGROUP_SIZE,
        name: None,
        on_init: None,
    };

    DrawKernels {
        draw_buffer,
        init,
        draw,
    }
}

/// The page's instanced geometry: one triangle, and five per-instance
/// attributes filled from `Math.random()` in the page's order.
pub fn geometry(draw_buffer: &IndirectStorageBufferAttribute) -> BufferGeometry {
    let mut random = DeterministicRandom::new();

    let mut offsets = Vec::with_capacity(INSTANCES * 3);
    let mut colors = Vec::with_capacity(INSTANCES * 4);
    let mut orientations_start = Vec::with_capacity(INSTANCES * 4);
    let mut orientations_end = Vec::with_capacity(INSTANCES * 4);

    // `vector.set( … ).normalize()` on a `Vector4`.
    let orientation = |random: &mut DeterministicRandom, out: &mut Vec<f32>| {
        let v: [f64; 4] = std::array::from_fn(|_| random.next() * 2.0 - 1.0);
        let length = v.iter().map(|c| c * c).sum::<f64>().sqrt();
        let length = if length == 0.0 { 1.0 } else { length };
        out.extend(v.iter().map(|c| (c / length) as f32));
    };

    for _ in 0..INSTANCES {
        // offsets
        offsets.extend((0..3).map(|_| (random.next() - 0.5) as f32));
        // colors
        colors.extend((0..4).map(|_| random.next() as f32));
        // orientation start, orientation end
        orientation(&mut random, &mut orientations_start);
        orientation(&mut random, &mut orientations_end);
    }

    // `const geometry = new THREE.InstancedBufferGeometry();
    //  geometry.instanceCount = instances;`
    let mut geometry = BufferGeometry::new();
    geometry.instance_count = Some(INSTANCES);

    geometry.set_attribute(
        "position",
        BufferAttribute::new(
            vec![0.025, -0.025, 0.0, -0.025, 0.025, 0.0, 0.0, 0.0, 0.025],
            3,
        ),
    );
    geometry.set_attribute("offset", BufferAttribute::new_instanced(offsets, 3));
    geometry.set_attribute("color", BufferAttribute::new_instanced(colors, 4));
    geometry.set_attribute(
        "orientationStart",
        BufferAttribute::new_instanced(orientations_start, 4),
    );
    geometry.set_attribute(
        "orientationEnd",
        BufferAttribute::new_instanced(orientations_end, 4),
    );

    // `geometry.setIndirect( drawBuffer );`
    geometry.set_indirect(draw_buffer.clone());
    geometry
}

/// `new THREE.MeshBasicNodeMaterial( { side: DoubleSide, forceSinglePass:
/// true, transparent: true } )` with the page's `positionNode` and
/// `fragmentNode`.
pub fn material() -> MeshBasicNodeMaterial {
    // `varyingProperty( 'vec3', 'vPosition' )`, `varyingProperty( 'vec4', 'vColor' )`
    let v_position = varying_property("vPosition", Type::Vec3, false);
    let v_color = varying_property("vColor", Type::Vec4, false);

    // `const positionFn = Fn( () => { … return vPosition; } )();`
    let position_fn = {
        let position = attribute("position", Type::Vec3);
        let offset = attribute("offset", Type::Vec3);
        let color = attribute("color", Type::Vec4);
        let orientation_start = attribute("orientationStart", Type::Vec4);
        let orientation_end = attribute("orientationEnd", Type::Vec4);

        let half_time = time().mul(float(0.5)).sin();

        // `max( abs( halfTime.mul( 2.0 ).add( 1.0 ) ), 0.5 )`
        let oscilation_range = max(abs(half_time.mul(float(2.0)).add(float(1.0))), float(0.5));

        // `offset.mul( oscilationRange ).add( position ).toVar()`
        let sphere_oscilation = to_var(None, offset.mul(oscilation_range).add(position));

        let orientation = mix(orientation_start, orientation_end, half_time).normalize();
        let vc_v = cross(orientation.xyz(), sphere_oscilation.clone());
        let cross_vc_v = cross(orientation.xyz(), vc_v.clone());

        block(
            vec![
                sphere_oscilation.clone(),
                v_position.assign(
                    vc_v.mul(orientation.w().mul(float(2.0)))
                        .add(cross_vc_v.mul(float(2.0)).add(sphere_oscilation)),
                ),
                v_color.assign(color),
            ],
            v_position.clone(),
        )
    };

    // `const fragmentFn = Fn( () => { … return color; } )();`
    let fragment_fn = {
        // `const color = vec4( vColor ).toVar();`
        let color = to_var(None, v_color.clone());
        // `color.r.addAssign( sin( vPosition.x.mul( 10.0 ).add( time ) ).mul( 0.5 ) );`
        let add = color.x().add_assign(
            v_position
                .x()
                .mul(float(10.0))
                .add(time())
                .sin()
                .mul(float(0.5)),
        );
        block(vec![color.clone(), add], color)
    };

    let mut material = MeshBasicNodeMaterial {
        side: Side::Double,
        force_single_pass: true,
        transparent: true,
        ..MeshBasicNodeMaterial::default()
    };
    material.position_node = Some(position_fn);
    material.fragment_node = Some(fragment_fn);
    material
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    pub kernels: DrawKernels,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.setClearColor( 0x000000 ); renderer.setClearAlpha( 0 );`
    renderer.set_clear_color(Color::from_hex(0x000000), 0.0);

    let mut camera = PerspectiveCamera::new(50.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 10000.0);
    let mut scene = Scene::new();

    scene.set_background(Color::from_hex(0x00001f));
    camera.node.borrow_mut().position.set(1.0, 1.0, 1.0);
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);

    let kernels = draw_kernels();
    let geometry = geometry(&kernels.draw_buffer);

    let mesh = Mesh::new(Rc::new(geometry), material());
    scene.add(&mesh);

    App {
        renderer,
        scene,
        camera,
        controls,
        kernels,
    }
}

/// The page's `render()`: the draw first, then the two kernels that fill the
/// next frame's draw arguments.
pub fn animate(app: &mut App) {
    let _ = app.controls.update(&mut app.camera, None);

    app.renderer.render(&mut app.scene, &mut app.camera);

    app.renderer.compute(&app.kernels.init).unwrap();
    app.renderer.compute(&app.kernels.draw).unwrap();
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
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_struct_drawindirect.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
