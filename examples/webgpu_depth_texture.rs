//! Port of `three.js/examples/webgpu_depth_texture.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! The page reads `window.innerWidth`, `window.innerHeight` and
//! `window.devicePixelRatio`. Under the e2e harness
//! (`test/e2e/puppeteer.js`) the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so those are 800, 500 and 1.

use std::rc::Rc;

use three_rs::testing::DeterministicRandom;
use three_rs::{
    torus_knot_geometry, Color, ColorNode, DepthTexture, Mesh, MeshBasicNodeMaterial,
    PerspectiveCamera, QuadMesh, RenderTarget, Renderer, RendererParameters, Scene, TextureType,
    Vector3,
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
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 20.0);
    camera.object.position.z = 4.0;

    let mut scene = Scene::new();
    scene.background = Some(Color::from_hex(0x222222));
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

        let mut mesh = Mesh::new(geometry.clone());
        mesh.object
            .position
            .set(r.cos() * z_scale, r.sin() * z_scale, z * scale);
        mesh.object
            .set_rotation(random.next(), random.next(), random.next());
        scene.add(mesh);
    }

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let depth_texture = DepthTexture::new();
    depth_texture.set_type(TextureType::Float);

    let render_target = RenderTarget::new(
        (INNER_WIDTH * DPR) as u32,
        (INNER_HEIGHT * DPR) as u32,
    );
    render_target.set_depth_texture(depth_texture.clone());

    // FX

    let mut material_fx = MeshBasicNodeMaterial::new();
    material_fx.color_node = Some(ColorNode::DepthTexture(depth_texture));

    let quad = QuadMesh::new(material_fx);

    //

    // `new OrbitControls( camera, renderer.domElement )` calls `update()` in its
    // constructor. With no input and the default target, the only thing that
    // update() does to the camera is rebuild its position from spherical
    // coordinates — which gives back (0, 2.45e-16, 4), a y below f32 precision —
    // and then `object.lookAt( target )`.
    camera.look_at(&Vector3::ZERO);

    App {
        renderer,
        scene,
        camera,
        quad,
        render_target,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.renderer.set_render_target(Some(app.render_target.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera);

    app.renderer.set_render_target(None);
    app.renderer.render_quad(&app.quad);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_depth_texture.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
