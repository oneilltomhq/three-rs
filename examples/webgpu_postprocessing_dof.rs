//! Port of `three.js/examples/webgpu_postprocessing_dof.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! A 14 × 9 × 14 grid of spheres, one `InstancedMesh` whose colour is the
//! Swedish Royal Castle cube map pulsed by `oscSine( positionWorld / 1000 +
//! time * .2 )`, rendered through [`dof`] — `DepthOfFieldNode`, with its
//! focus 500 units in front of the camera, a 200-unit focal length and a
//! bokeh scale of 10.
//!
//! **There is no rung.** three lists `webgpu_postprocessing_dof` in
//! `test/e2e/puppeteer.js`'s exception list, so it has no reference
//! screenshot to grade against. What the port checks instead is every quad
//! `DepthOfFieldNode` draws, against three's dump, in
//! `tests/nodes_display_wgsl.rs` (`dof_*`).
//!
//! `new CubeTextureLoader().load( urls )` is asynchronous on the page; here
//! the six faces are decoded synchronously. The inspector GUI (focus
//! distance, focal length, bokeh scale) is not ported; the uniforms keep
//! their defaults.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::sphere_geometry;
use three_rs::math::Matrix4;
use three_rs::nodes::display::{dof, DepthOfFieldNode};
use three_rs::nodes::tsl::{
    cube_texture, float, material_env_rotation, osc_sine, position_world, reflect_vector, time,
    uniform_value, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::{
    pass, CubeTextureLoader, InstancedMesh, MeshBasicNodeMaterial, PassNode, PerspectiveCamera,
    RenderPipeline, Renderer, RendererParameters, Scene,
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
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub scene_pass: PassNode,
    /// `dof( … )`, the pipeline's `outputNode`.
    pub dof_pass: DepthOfFieldNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 3500.0);
    camera.node.borrow_mut().position.z = 200.0;

    let scene = Scene::new();

    let path = examples_dir().join("textures/cube/SwedishRoyalCastle/");
    let format = ".jpg";
    let urls =
        ["px", "nx", "py", "ny", "pz", "nz"].map(|face| path.join(format!("{face}{format}")));

    let (xgrid, ygrid, zgrid) = (14, 9, 14);
    let count = xgrid * ygrid * zgrid;

    let texture_cube = CubeTextureLoader::new().load(urls).unwrap();
    // `cubeTexture( textureCube )` with no uv: `CubeTextureNode.getDefaultUV()`
    // is `reflectVector` for a `CubeReflectionMapping`, which `setupUV()`
    // rotates by `materialEnvRotation` (the identity here).
    let cube_texture_node = cube_texture(
        &texture_cube,
        material_env_rotation().mul(vec4_join(vec![reflect_vector(), float(1.0)])),
    );
    let osc_pos = osc_sine(
        position_world()
            .div(float(1000.0)) // scene distance
            .add(time().mul(float(0.2))),
    );

    let geometry = Rc::new(sphere_geometry(60.0, 20, 10));
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(cube_texture_node.mul(osc_pos));

    let mesh = InstancedMesh::new(geometry, material, count);
    scene.add(&mesh);

    let mut index = 0;
    {
        let mut mesh = mesh.borrow_mut();
        for i in 0..xgrid {
            for j in 0..ygrid {
                for k in 0..zgrid {
                    let x = 200.0 * (i as f64 - xgrid as f64 / 2.0);
                    let y = 200.0 * (j as f64 - ygrid as f64 / 2.0);
                    let z = 200.0 * (k as f64 - zgrid as f64 / 2.0);

                    // `matrix.identity().setPosition( x, y, z )`.
                    let mut matrix = Matrix4::identity();
                    matrix.set_position(x, y, z);
                    mesh.set_matrix_at(index, &matrix);
                    index += 1;
                }
            }
        }
    }

    // renderer

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `effectController`
    let focus_distance = uniform_value(Type::F32, vec![500.0]);
    let focal_length = uniform_value(Type::F32, vec![200.0]);
    let bokeh_scale = uniform_value(Type::F32, vec![10.0]);

    // post processing

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut render_pipeline = RenderPipeline::new();
    let scene_pass = pass(scene.clone(), camera.clone());
    // `toInspector( 'Color' )` returns its node unchanged.
    let scene_pass_color = scene_pass.texture();
    let scene_pass_view_z = scene_pass.view_z_node("depth");

    let dof_pass = dof(
        &scene_pass_color,
        scene_pass_view_z,
        focus_distance,
        focal_length,
        bokeh_scale,
    );
    render_pipeline.output_node = Some(dof_pass.node());

    // controls

    let mut controls = {
        let mut camera = camera.borrow_mut();
        let mut controls = OrbitControls::new(&mut camera);
        // The canvas the example renders at, standing in for the element's
        // `clientWidth` / `clientHeight`.
        controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
        controls
    };
    controls.enable_damping = true;

    App {
        renderer,
        scene,
        camera,
        controls,
        scene_pass,
        dof_pass,
        render_pipeline,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
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
/// event handlers needs.
pub fn controls_and_camera(
    app: &mut App,
) -> Option<(&mut OrbitControls, std::cell::RefMut<'_, PerspectiveCamera>)> {
    Some((&mut app.controls, app.camera.borrow_mut()))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_postprocessing_dof.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
