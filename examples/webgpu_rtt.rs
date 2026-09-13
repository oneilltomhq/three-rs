//! Port of `three.js/examples/webgpu_rtt.html`, calling the three-rs API in the
//! same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so the image is always
//! present for the graded frame; here it is decoded synchronously.
//!
//! `renderer.inspector = new Inspector()` and `.toInspector( 'Scene Pass' )`
//! only register the node for the inspector panel — `toInspector()` returns the
//! node unchanged and changes nothing about the graded frame. Neither the
//! `mousemove` nor the `resize` listener fires, so `mouse` stays (0, 0).

use std::rc::Rc;

use three_rs::nodes::tsl::{hue, saturation, texture, uniform_value};
use three_rs::nodes::Type;
use three_rs::{
    box_geometry, Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, QuadMesh, RenderTarget,
    Renderer, RendererParameters, Scene, Vector2,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `const dpr = window.devicePixelRatio;`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub quad_mesh: QuadMesh,
    pub render_target: RenderTarget,
}

fn examples_dir() -> std::path::PathBuf {
    let three = match std::env::var("THREE_JS_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => {
            std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("src/vendor/three.js")
        }
    };
    three.join("examples")
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 3.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x0066FF));

    // textured mesh

    let uv_texture = three_rs::TextureLoader::new().load(examples_dir().join("textures/uv_grid_opengl.jpg"));

    let geometry_box = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    let mut material_box = MeshBasicNodeMaterial::new();
    material_box.color_node = Some(texture(&uv_texture));

    //

    let boxed = Mesh::new(geometry_box);
    boxed.borrow_mut().mesh_mut().unwrap().material = Some(material_box);
    scene.add(&boxed);

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let render_target = RenderTarget::new((INNER_WIDTH * DPR) as u32, (INNER_HEIGHT * DPR) as u32);

    // FX

    // modulate the final color based on the mouse position

    let mouse = Vector2::new(0.0, 0.0);
    let screen_fx_node = uniform_value(Type::Vec2, vec![mouse.x, mouse.y]);

    let mut material_fx = MeshBasicNodeMaterial::new();

    let scene_pass_texture = texture(&render_target.texture());
    material_fx.color_node = Some(hue(
        saturation(scene_pass_texture.rgb(), screen_fx_node.x().one_minus()),
        screen_fx_node.y(),
    ));

    let quad_mesh = QuadMesh::new(material_fx);

    App {
        renderer,
        scene,
        camera,
        quad_mesh,
        render_target,
    }
}

/// The page's `animate()`. `setAnimationLoop` runs it once under the harness,
/// so the box has been rotated exactly one step by the time the frame is read.
pub fn animate(app: &mut App) {
    let boxed = app.scene.children()[0].clone();
    let rotation = boxed.borrow().rotation;
    boxed
        .borrow_mut()
        .set_rotation(rotation.x + 0.01, rotation.y + 0.02, rotation.z);

    app.renderer
        .set_render_target(Some(app.render_target.clone()));
    app.renderer.render(&mut app.scene, &mut app.camera);

    app.renderer.set_render_target(None);
    app.renderer.render_quad(&app.quad_mesh);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_rtt.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
