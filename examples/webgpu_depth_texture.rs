//! Port of `three.js/examples/webgpu_depth_texture.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! The page reads `window.innerWidth`, `window.innerHeight` and
//! `window.devicePixelRatio`. Under the e2e harness
//! (`test/e2e/puppeteer.js`) the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so those are 800, 500 and 1.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl;
use three_rs::testing::DeterministicRandom;
use three_rs::{
    torus_knot_geometry, Color, DepthTexture, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    QuadMesh, RenderTarget, Renderer, RendererParameters, Scene, TextureType,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `const dpr = window.devicePixelRatio;`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub quad: QuadMesh,
    pub render_target: RenderTarget,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 20.0);
    camera.node.borrow_mut().position.z = 4.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x222222));
    scene.override_material = Some(MeshBasicNodeMaterial::new());

    //

    let geometry = Rc::new(torus_knot_geometry(1.0, 0.3, 128, 64, 2.0, 3.0));

    let count = 50;
    let scale = 5.0;

    // `Math.random()` is the harness' deterministic sequence; the call order
    // below (r, z, then three rotation values, per mesh) is what fixes the scene.
    let mut random = DeterministicRandom::new();

    for _ in 0..count {
        let r = random.next() * 2.0 * std::f64::consts::PI;
        let z = (random.next() * 2.0) - 1.0;
        let z_scale = (1.0 - z * z).sqrt() * scale;

        let mesh = Mesh::new(geometry.clone(), None);
        {
            let mut object = mesh.borrow_mut();
            object
                .position
                .set(r.cos() * z_scale, r.sin() * z_scale, z * scale);
            object.set_rotation(random.next(), random.next(), random.next());
        }
        scene.add(&mesh);
    }

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let depth_texture = DepthTexture::new();
    depth_texture.set_type(TextureType::Float).unwrap();

    let render_target = RenderTarget::new((INNER_WIDTH * DPR) as u32, (INNER_HEIGHT * DPR) as u32);
    render_target.set_depth_texture(depth_texture.clone());

    // FX

    let mut material_fx = MeshBasicNodeMaterial::new();
    // `materialFX.colorNode = texture( depthTexture )`
    material_fx.color_node = Some(tsl::depth_texture(&depth_texture));

    let quad = QuadMesh::new(material_fx);

    //

    // `controls = new OrbitControls( camera, renderer.domElement );` The
    // constructor's own `update()` rebuilds the camera position from spherical
    // coordinates — which gives back (0, 2.45e-16, 4), a y below f32
    // precision — and then runs `object.lookAt( target )`.
    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    // `controls.enableDamping = true;` The page never calls `controls.update()`
    // in `animate()`, so the damping never runs; that is the page's own
    // inconsistency, transcribed as it stands.
    controls.enable_damping = true;

    App {
        renderer,
        scene,
        camera,
        quad,
        render_target,
        controls,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.renderer
        .set_render_target(Some(app.render_target.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera);

    app.renderer.set_render_target(None);
    app.renderer.render_quad(&app.quad);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
///
/// The last line is the page's `renderTarget.setSize( window.innerWidth * dpr,
/// window.innerHeight * dpr )`: the target the scene pass draws into is sized
/// in device pixels, not CSS ones, as it was in `init()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
    app.render_target
        .set_size((width * DPR) as u32, (height * DPR) as u32);
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
        .unwrap_or_else(|| "target/webgpu_depth_texture.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
