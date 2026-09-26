//! Port of `three.js/examples/webgpu_compute_texture.html`, calling the
//! three-rs API in the same order the page's `init()` / `render()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A kernel fills a 512² `StorageTexture` once, in `init()`, with the
//! shadertoy plasma; a plane samples it. Between the two, the renderer rebuilds
//! the texture's mip chain from the level the kernel wrote
//! (`Bindings._update()`'s `needsMipmap`) — the plane covers about 500² pixels,
//! so the sample reads mips 0 and 1 and a stale chain would show.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::{
    float, instance_index, storage_texture, texture, texture_store, to_const, uint, vec2_join,
    vec4_join,
};
use three_rs::nodes::ComputeFlow;
use three_rs::{
    plane_geometry, Color, Mesh, MeshBasicNodeMaterial, OrthographicCamera, PerspectiveCamera,
    Renderer, RendererParameters, Scene, Texture,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const width = 512, height = 512;`
pub const WIDTH: u32 = 512;
pub const HEIGHT: u32 = 512;

/// `ComputeNode`'s default `workgroupSize = [ 64 ]`, padded to three components.
pub const WORKGROUP_SIZE: [u32; 3] = [64, 1, 1];

/// `computeTexture( { storageTexture } ).compute( width * height )`. Split out
/// so `tests/nodes_texture_wgsl.rs` builds exactly the kernel the example
/// runs.
pub fn compute_texture(storage: &Texture) -> ComputeFlow {
    let storage = storage_texture(storage);

    // Every intermediate below is a `let nodeConstN` in three's dump because
    // each is used more than once; `to_const` is what gives the port the same
    // single evaluation (and the same `let`) rather than a repeated inline.
    //
    // `const posX = instanceIndex.mod( width );`
    let pos_x = to_const(None, instance_index().modulo(uint(WIDTH)));
    // `const posY = instanceIndex.div( width );`
    let pos_y = to_const(None, instance_index().div(uint(WIDTH)));
    // `const indexUV = uvec2( posX, posY );`
    let index_uv = vec2_join(vec![pos_x.clone(), pos_y.clone()]);

    // `const x = float( posX ).div( 50.0 );`
    let x = to_const(None, float_of(&pos_x).div(50.0));
    let y = to_const(None, float_of(&pos_y).div(50.0));

    let v1 = x.sin();
    let v2 = y.sin();
    let v3 = x.add(&y).sin();
    let v4 = x.mul(&x).add(y.mul(&y)).sqrt().add(5.0).sin();
    // `v1.add( v2, v3, v4 )` — the variadic add folds left.
    let v = to_const(None, v1.add(v2).add(v3).add(v4));

    let r = v.sin();
    let g = v.add(std::f64::consts::PI).sin();
    let b = v.add(std::f64::consts::PI).sub(0.5).sin();

    // `textureStore( storageTexture, indexUV, vec4( r, g, b, 1 ) ).toWriteOnly()`
    let store = texture_store(&storage, index_uv, vec4_join(vec![r, g, b, float(1.0)]));

    ComputeFlow {
        statements: vec![store],
        count: (WIDTH * HEIGHT) as usize,
        workgroup_size: WORKGROUP_SIZE,
        name: None,
        on_init: None,
    }
}

/// `float( posX )` — a `u32` converted.
fn float_of(v: &three_rs::nodes::NodeRef) -> three_rs::nodes::NodeRef {
    v.to(three_rs::nodes::Type::F32)
}

/// The plane's material: `new MeshBasicNodeMaterial( { color: 0x00ff00 } )`
/// with `colorNode = texture( storageTexture )`. The colour never reaches the
/// shader — `colorNode` replaces `materialColor` — but it is what the page
/// sets.
pub fn material(storage: &Texture) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0x00ff00);
    material.color_node = Some(texture(storage));
    material
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: OrthographicCamera,
    pub storage_texture: Texture,
}

pub fn init() -> App {
    let aspect = INNER_WIDTH / INNER_HEIGHT;
    let mut camera = OrthographicCamera::new(-aspect, aspect, 1.0, -1.0, 0.0, 2.0);
    camera.object.position.z = 1.0;
    camera.update_matrix_world();

    let scene = Scene::new();

    // texture

    // `new THREE.StorageTexture( width, height )`
    let storage_texture = Texture::storage(WIDTH, HEIGHT);

    // compute

    let compute_node = compute_texture(&storage_texture);

    let material = material(&storage_texture);

    let plane = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    scene.add(&plane);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `renderer.compute( computeNode )` — once, in `init()`.
    renderer.compute(&compute_node).unwrap();

    App {
        renderer,
        scene,
        camera,
        storage_texture,
    }
}

/// The page's `render()`.
pub fn animate(app: &mut App) {
    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`: the frustum keeps its height of 2 and
/// widens to the new aspect.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.renderer.set_size(width, height);

    let aspect = width / height;
    let frustum_height = app.camera.top - app.camera.bottom;
    app.camera.left = -frustum_height * aspect / 2.0;
    app.camera.right = frustum_height * aspect / 2.0;
    app.camera.update_projection_matrix();
}

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
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
        .unwrap_or_else(|| "target/webgpu_compute_texture.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
