//! Port of `three.js/examples/webgpu_multiple_rendertargets_readback.html`,
//! calling the three-rs API in the same order the page's `init()` /
//! `render()` / `readback()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! # What the page shows
//!
//! `webgpu_multiple_rendertargets`' knot and split screen, reached a second
//! way: the MRT is set around the scene render and cleared again
//! (`renderer.setMRT( sceneMRT )` … `renderer.setMRT( null )`), and the
//! composite is a `QuadMesh` with a bare `NodeMaterial`, so the canvas
//! transform runs as its own `outputColorTransform` pass after it rather than
//! inline as in a `RenderPipeline`.
//!
//! The page's Inspector offers `selection: 'mrt' | 'diffuse' | 'normal'`. The
//! last two render into a 512² `readbackTarget`, read one attachment back
//! with `readRenderTargetPixelsAsync()`, and draw it through a `DataTexture`.
//! The graded frame is `'mrt'`, the default, which never takes that branch —
//! the readback objects are still built in `init()`, as the page builds them,
//! and [`App::options`] switches to them.
//!
//! `readbackTarget`'s two textures are never named. `MRTNode.setup()` looks
//! outputs up by texture name, so `normal` never reaches attachment 1 and the
//! `'normal'` readback is the attachment's black clear. In the port attachment
//! 0 always answers to `output`, so `'diffuse'` shows the knot; see
//! `docs/nodes.md` §52.4.
//!
//! `renderer.inspector = new Inspector()` registers a panel and changes
//! nothing about the graded frame. `performance.now()` and the RAF's `time`
//! are 0 under the harness, so the knot's `rotation.y` is 0.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::torus_knot_geometry;
use three_rs::nodes::tsl::{
    mix, normal_world, output_property, screen_uv, step, texture, texture_uv, uv, vec2,
};
use three_rs::nodes::{mrt, MrtNode};
use three_rs::renderer::RenderTargetOptions;
use three_rs::textures::Wrapping;
use three_rs::utils::now_ms;
use three_rs::{
    Color, ColorSpace, Mesh, MeshBasicNodeMaterial, Node, PerspectiveCamera, QuadMesh,
    RenderTarget, Renderer, RendererParameters, Scene, Texture, TextureFilter, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `window.devicePixelRatio`.
pub const DPR: f64 = 1.0;

/// `const size = 512;` — "Be careful with the size! 512 is already big."
const READBACK_SIZE: u32 = 512;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The Inspector's `selection` dropdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Selection {
    /// Both attachments of `renderTarget`, split at `screenUV.x = 0.5`.
    #[default]
    Mrt,
    /// `readbackTarget.textures[ 0 ]`, read back and redrawn.
    Diffuse,
    /// `readbackTarget.textures[ 1 ]`, read back and redrawn.
    Normal,
}

/// The page's `options`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub selection: Selection,
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub torus: Node,
    pub options: Options,
    pub quad_mesh: QuadMesh,
    pub scene_mrt: MrtNode,
    pub render_target: RenderTarget,
    pub readback_target: RenderTarget,
    /// Whichever of the page's `material` / `readbackMaterial` is not in
    /// `quad_mesh.material` right now; see [`animate`].
    pub other_material: MeshBasicNodeMaterial,
    /// Whether `quad_mesh.material` is `readbackMaterial`.
    pub quad_is_readback: bool,
    pub pixel_buffer_texture: Texture,
    /// The page's anonymous `new OrbitControls( camera, renderer.domElement )`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // Create a multi render target with Float buffers

    // `{ count: 2, minFilter: NearestFilter, magFilter: NearestFilter }` —
    // the default `UnsignedByteType`, whatever the comment says.
    let mut options = RenderTargetOptions::default();
    options.min_filter = TextureFilter::Nearest;
    options.mag_filter = TextureFilter::Nearest;
    let render_target = RenderTarget::new_with_options(
        (INNER_WIDTH * DPR) as u32,
        (INNER_HEIGHT * DPR) as u32,
        options,
    )
    .unwrap();
    render_target.set_count(2);

    // Name our G-Buffer attachments for debugging

    render_target.set_texture_name(0, "output");
    render_target.set_texture_name(1, "normal");

    // Init readback render target, readback data texture, readback material

    // `new THREE.RenderTarget( size, size, { count: 2 } )` — never named.
    let readback_target = RenderTarget::new(READBACK_SIZE, READBACK_SIZE);
    readback_target.set_count(2);

    // `new THREE.DataTexture( new Uint8Array( size ** 2 * 4 ).fill( 0 ), size,
    // size )`, `UnsignedByteType`, `RGBAFormat`.
    let pixel_buffer = vec![0u8; (READBACK_SIZE * READBACK_SIZE * 4) as usize];
    let pixel_buffer_texture =
        Texture::data_rgba8(READBACK_SIZE, READBACK_SIZE, pixel_buffer.clone());

    let mut readback_material = MeshBasicNodeMaterial::new();
    readback_material.color_node = Some(texture(&pixel_buffer_texture));

    // MRT

    // `mrt( { output, normal: normalWorld } )`. `normalWorld` is a node
    // object upstream whose `setup()` runs per material, so it is handed over
    // as a closure (`docs/nodes.md` §23).
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    scene_mrt.set_deferred("normal", normal_world);

    // Scene

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x222222));

    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 50.0);
    camera.node.borrow_mut().position.z = 4.0;

    let diffuse = TextureLoader::new()
        .load(examples_dir().join("textures/hardwood2_diffuse.jpg"))
        .unwrap();
    diffuse.set_color_space(ColorSpace::Srgb);
    diffuse.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // `new THREE.NodeMaterial()`: the port's unlit material generates the
    // same program for a bare `colorNode` (`webgpu_multiple_rendertargets`).
    let mut torus_material = MeshBasicNodeMaterial::new();
    torus_material.color_node = Some(texture_uv(&diffuse, uv().mul(vec2(10.0, 4.0))));

    let torus = Mesh::new(
        Rc::new(torus_knot_geometry(1.0, 0.3, 128, 32, 2.0, 3.0)),
        torus_material,
    );
    scene.add(&torus);

    // Output

    let textures = render_target.textures();
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(mix(
        texture(&textures[0]),
        texture(&textures[1]),
        step(0.5, screen_uv().x()),
    ));

    let quad_mesh = QuadMesh::new(material);

    // Controls

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        torus,
        options: Options::default(),
        quad_mesh,
        scene_mrt,
        render_target,
        readback_target,
        other_material: readback_material,
        quad_is_readback: false,
        pixel_buffer_texture,
        controls,
    }
}

/// The page's `render( time )`. Upstream it is `async` and awaits
/// `readback()`; the port's readback blocks, so this is the same order.
pub fn animate(app: &mut App) {
    let selection = app.options.selection;

    // `time` is the RAF timestamp — `performance.now()`.
    let time = now_ms();
    {
        let mut torus = app.torus.borrow_mut();
        let rotation = torus.rotation;
        torus.set_rotation(rotation.x, (time / 1000.0) * 0.4, rotation.z);
    }

    let is_readback = selection != Selection::Mrt;

    // render scene into target
    app.renderer.set_mrt(Some(app.scene_mrt.clone()));
    app.renderer.set_render_target(Some(if is_readback {
        app.readback_target.clone()
    } else {
        app.render_target.clone()
    }));
    app.renderer.render(&mut app.scene, &mut app.camera);

    // render post FX
    app.renderer.set_mrt(None);
    app.renderer.set_render_target(None);

    // `quadMesh.material = isReadback ? readbackMaterial : material`. A JS
    // assignment keeps the object's identity; a Rust `clone()` of a material
    // is a *new* material (`MaterialId::clone`, as `material.clone()` is
    // upstream) and would rebuild its program every frame. So the two
    // materials trade places instead, and each stays the one object it is.
    if is_readback != app.quad_is_readback {
        std::mem::swap(&mut app.quad_mesh.material, &mut app.other_material);
        app.quad_is_readback = is_readback;
    }

    if is_readback {
        readback(app);
    }

    app.renderer.render_quad(&app.quad_mesh);
}

/// The page's `readback()`: `readRenderTargetPixelsAsync()` into the
/// `DataTexture`'s image, then `needsUpdate`.
fn readback(app: &mut App) {
    let (width, height) = app.readback_target.size();

    let texture_index = match app.options.selection {
        Selection::Diffuse => 0, // zero is optional
        Selection::Normal => 1,
        Selection::Mrt => return,
    };
    let pixel_buffer = app
        .renderer
        .read_render_target_pixels(&app.readback_target, 0, 0, width, height, texture_index)
        .unwrap();

    // `pixelBufferTexture.image.data = pixelBuffer; pixelBufferTexture.needsUpdate = true;`
    app.pixel_buffer_texture.set_data(pixel_buffer);
}

/// The page's `onWindowResize()`. Only `renderTarget` follows the canvas;
/// `readbackTarget` stays 512².
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);

    // `const dpr = renderer.getPixelRatio();` — the ratio `init()` set.
    app.render_target
        .set_size((width * DPR) as u32, (height * DPR) as u32);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
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
    // `THREE_RS_SELECTION=diffuse|normal` stands in for the Inspector's
    // dropdown, to look at the readback branch the graded frame never takes.
    app.options.selection = match std::env::var("THREE_RS_SELECTION").as_deref() {
        Ok("diffuse") => Selection::Diffuse,
        Ok("normal") => Selection::Normal,
        _ => Selection::Mrt,
    };
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_multiple_rendertargets_readback.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
