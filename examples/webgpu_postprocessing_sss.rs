//! Port of `three.js/examples/webgpu_postprocessing_sss.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The Nemetona statue on a grey ground, lit by a hemisphere light and a
//! shadow-casting directional light. A depth + velocity pre-pass feeds
//! [`sss`], which marches screen-space rays towards the directional light
//! for the contact shadows its 1024² shadow map is too coarse to resolve;
//! the scene pass multiplies them into that light's colour
//! ([`PassNode::set_context_shadow`], three's `builtinShadowContext`), and
//! [`traa`] resolves the per-frame ray jitter of the temporal filtering.
//!
//! **There is no rung.** three lists `webgpu_postprocessing_sss` in
//! `test/e2e/puppeteer.js`'s exception list ("Black screen"), so it has no
//! reference screenshot to grade against. What the port checks instead is
//! the SSS quad shader, against three's dump in
//! `tests/nodes_display_wgsl.rs`, and the effect over several frames in
//! `tests/sss_frames.rs`.
//!
//! Differences from the page:
//!
//! * `loader.load( 'models/gltf/nemetona.glb', … )` is asynchronous on the
//!   page and adds the model whenever it arrives; the port loads it
//!   synchronously, so the model is in the first frame.
//! * The inspector GUI is not ported: the example runs in the page's
//!   initial state (output 0, "Scene with Shadow Maps + SSS", with temporal
//!   filtering). The "Scene with Shadow Maps" mode is
//!   [`PassNode::clear_context_shadow`] and the setters the GUI drives are
//!   [`SssNode`]'s public uniforms.
//! * `toInspector( … )` returns its node unchanged and is dropped.

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::plane_geometry;
use three_rs::loaders::GltfLoader;
use three_rs::nodes::display::{sss, traa, SssNode, TraaNode};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::screen_uv;
use three_rs::nodes::velocity::velocity;
use three_rs::{
    pass, Background, Color, DirectionalLight, Fog, HemisphereLight, Mesh, MeshPhongNodeMaterial,
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with both passes, which render it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with both passes, `sss_node` and `traa_node`, which jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's `prePass`.
    pub pre_pass: PassNode,
    /// The page's `scenePass`.
    pub scene_pass: PassNode,
    /// The page's `sssPass`.
    pub sss_node: SssNode,
    /// The page's `traaPass`.
    pub traa_node: TraaNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(1.0, 2.5, -3.5);

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0xa0a0a0)));
    scene.fog = Some(Fog::new(Color::from_hex(0xa0a0a0), 10.0, 50.0).into());

    // lights

    let hemi_light =
        HemisphereLight::new(Color::from_hex(0xffffff), Color::from_hex(0x8d8d8d), 2.0);
    hemi_light.borrow_mut().position.set(0.0, 20.0, 0.0);
    scene.add(&hemi_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(-3.0, 10.0, -10.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        // remove self-shadowing artifacts
        shadow.bias = -0.001;
        shadow.radius = 2.0;
        // `camera.top / bottom / left / right`, as left, right, top, bottom.
        shadow.camera.set_bounds(-4.0, 4.0, 4.0, -4.0);
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(40.0);
        shadow.map_size.x = 1024.0;
        shadow.map_size.y = 1024.0;
    }
    scene.add(&dir_light);

    // ground

    let mut ground_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xcbcbcb));
    ground_material.depth_write = false;
    let mesh = Mesh::new(Rc::new(plane_geometry(100.0, 100.0, 1, 1)), ground_material);
    {
        let mut object = mesh.borrow_mut();
        object.set_rotation(-PI / 2.0, 0.0, 0.0);
        object.receive_shadow = true;
    }
    scene.add(&mesh);

    // model: see the module docs for why it loads synchronously.

    let gltf = GltfLoader::load(examples_dir().join("models/gltf/nemetona.glb"))
        .expect("three-rs: nemetona.glb loads");
    let model = gltf.scene.clone();
    {
        let mut object = model.borrow_mut();
        object.set_rotation(0.0, PI, 0.0);
        object.scale.set_scalar(10.0);
        object.position.y = 0.45;
    }
    scene.add(&model);

    model.traverse(&mut |child| {
        let mut object = child.borrow_mut();
        if object.is_mesh() {
            object.cast_shadow = true;
            object.receive_shadow = true;
            // remove AO to better see the effect of shadows
            let mesh = object.mesh_mut().unwrap();
            if let Some(material) = mesh.material.as_mut() {
                material.ao_map = None;
            }
            for material in &mut mesh.materials {
                material.ao_map = None;
            }
        }
    });

    //

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    // post-processing

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut render_pipeline = RenderPipeline::new();

    // pre-pass

    let mut pre_pass = pass(scene.clone(), camera.clone());
    pre_pass.set_transparent(false);

    pre_pass.set_mrt(mrt(vec![("output", velocity())]));

    // `prePass.getTextureNode( 'depth' )` and `( 'output' )`: besides the
    // nodes, they link both textures to the pass.
    let _ = pre_pass.texture_node("depth");
    let _ = pre_pass.texture_node("output");

    // scene pass

    let scene_pass = pass(scene.clone(), camera.clone());

    // sss

    let sss_node = sss(&pre_pass.depth_texture(), camera.clone(), &dir_light);
    sss_node.max_distance.set(vec![0.2]);
    sss_node.set_use_temporal_filtering(true);

    // scene context: `scenePass.contextNode = builtinShadowContext(
    // sssPass.getTextureNode().sample( screenUV ).r, dirLight )`.

    scene_pass.set_context_shadow(sss_node.sample(screen_uv()).x(), &dir_light);

    // traa: the pre-pass `output` is its velocity.

    let traa_node = traa(
        &scene_pass.texture(),
        &pre_pass.depth_texture(),
        &pre_pass.texture(),
        camera.clone(),
    );
    traa_node.attach(&mut render_pipeline);
    render_pipeline.output_node = Some(traa_node.node());

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 1.0;
    controls.max_distance = 20.0;
    controls.target.set(0.0, 2.0, 0.0);
    controls.enable_damping = true;
    let _ = controls.update(&mut camera.borrow_mut(), None);

    App {
        renderer,
        scene,
        camera,
        controls,
        pre_pass,
        scene_pass,
        sss_node,
        traa_node,
        render_pipeline,
    }
}

pub fn animate(app: &mut App) {
    let _ = app.controls.update(&mut app.camera.borrow_mut(), None);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`. The TRAA history restarts at the new size
/// on the next frame.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    // Pin both clocks to zero, as three.js' `test/e2e/deterministic-injection.js`
    // does to the page, so the frame this writes does not depend on how long
    // `init()` took.
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_sss.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
