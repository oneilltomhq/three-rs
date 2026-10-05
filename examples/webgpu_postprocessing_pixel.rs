//! Port of `three.js/examples/webgpu_postprocessing_pixel.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Two checkered boxes and a bobbing crystal on a checkered plane, lit by an
//! ambient, a directional and a spot light with `BasicShadowMap` shadows,
//! drawn through [`pixelation_pass`] at a sixth of the drawing buffer with
//! its depth- and normal-edge outlines.
//!
//! Under the e2e harness `window.innerWidth` / `innerHeight` are 800 and 500
//! and the clocks are pinned to zero, so the graded frame has `t = 0`: the
//! crystal at `y = 0.7`, unturned, with `emissiveIntensity = 0.5`.
//!
//! Divergences, none of which changes the graded frame:
//!
//! - **No `OrbitControls`.** The page puts them on its `OrthographicCamera`,
//!   whose branches `OrbitControls` does not port (see its docs). What the
//!   constructor does to the camera — `update()`, which with nothing to apply
//!   only points it at the target, the origin — is done here by
//!   `camera.lookAt( 0, 0, 0 )`. `controls.maxZoom = 2` only bounds a zoom
//!   that no input can now make.
//! - The checker texture loads synchronously, where the page's
//!   `TextureLoader.load()` is asynchronous; three's screenshot is taken once
//!   it is in.
//! - `scene.add( spotLight.target )` is not repeated: the target is the
//!   light's own, at the origin, where three's would also be.
//! - The GUI is not ported; its defaults are (`pixelSize` 6, the two edge
//!   strengths 0.3 and 0.4 as settable uniforms, `pixelAlignedPanning` on).

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::{box_geometry, icosahedron_geometry, plane_geometry};
use three_rs::lights::ShadowMapType;
use three_rs::nodes::display::{pixelation_pass, PixelationPassNode};
use three_rs::nodes::node::{SettableValue, Type};
use three_rs::nodes::tsl::uniform_settable;
use three_rs::objects::Background;
use three_rs::textures::{MinFilter, Texture, TextureFilter, Wrapping};
use three_rs::{
    AmbientLight, Color, ColorSpace, DirectionalLight, Mesh, MeshPhongNodeMaterial, ObjectRef,
    OrthographicCamera, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
    SpotLight, TextureLoader, Timer, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// The page sets no pixel ratio, so the canvas is at CSS size.
pub const DPR: f64 = 1.0;

/// The page's `params`.
pub struct Params {
    pub pixel_aligned_panning: bool,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Rc<RefCell<Scene>>,
    pub camera: Rc<RefCell<OrthographicCamera>>,
    pub crystal_mesh: ObjectRef,
    /// The page's module-level `timer`.
    pub timer: Timer,
    pub params: Params,
    /// `uniform( 0.3 )`, the GUI's "Normal Edge Strength".
    pub normal_edge_strength: SettableValue,
    /// `uniform( 0.4 )`, the GUI's "Depth Edge Strength".
    pub depth_edge_strength: SettableValue,
    pub scene_pass: PixelationPassNode,
    pub render_pipeline: RenderPipeline,
}

/// The page's `pixelTexture( texture )`.
fn pixel_texture(texture: Texture) -> Texture {
    texture.set_min_filter(MinFilter::Nearest);
    texture.set_mag_filter(TextureFilter::Nearest);
    texture.set_generate_mipmaps(false);
    texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);
    texture.set_color_space(ColorSpace::Srgb);
    texture
}

pub fn init() -> App {
    let aspect_ratio = INNER_WIDTH / INNER_HEIGHT;

    let mut camera = OrthographicCamera::new(-aspect_ratio, aspect_ratio, 1.0, -1.0, 0.1, 10.0);
    camera.object.position.y = 2.0 * (PI / 6.0).tan();
    camera.object.position.z = 2.0;

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x151729)));

    // `timer.connect( document )` only listens for visibility changes.
    let timer = Timer::new();

    // textures

    let loader = TextureLoader::new();
    let checker = examples_dir().join("textures/checker.png");
    let tex_checker = pixel_texture(loader.load(&checker).expect("checker.png"));
    let tex_checker2 = pixel_texture(loader.load(&checker).expect("checker.png"));
    tex_checker.set_repeat(3.0, 3.0);
    tex_checker2.set_repeat(1.5, 1.5);

    // meshes

    let mut box_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    box_material.map = Some(tex_checker2);

    let add_box =
        |scene: &Scene, box_side_length: f64, x: f64, z: f64, rotation: f64| -> ObjectRef {
            let mesh = Mesh::new(
                Rc::new(box_geometry(
                    box_side_length,
                    box_side_length,
                    box_side_length,
                    1,
                    1,
                    1,
                )),
                box_material.clone(),
            );
            {
                let mut object = mesh.borrow_mut();
                object.cast_shadow = true;
                object.receive_shadow = true;
                let current = object.rotation;
                object.set_rotation(current.x, rotation, current.z);
                object.position.y = box_side_length / 2.0;
                object.position.set(x, box_side_length / 2.0 + 0.0001, z);
            }
            scene.add(&mesh);
            mesh
        };

    add_box(&scene, 0.4, 0.0, 0.0, PI / 4.0);
    add_box(&scene, 0.5, -0.5, -0.5, PI / 4.0);

    let plane_side_length = 2.0;
    let mut plane_material = MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff));
    plane_material.map = Some(tex_checker);
    let plane_mesh = Mesh::new(
        Rc::new(plane_geometry(plane_side_length, plane_side_length, 1, 1)),
        plane_material,
    );
    {
        let mut object = plane_mesh.borrow_mut();
        object.receive_shadow = true;
        let current = object.rotation;
        object.set_rotation(-PI / 2.0, current.y, current.z);
    }
    scene.add(&plane_mesh);

    let radius = 0.2;
    let geometry = Rc::new(icosahedron_geometry(radius, 0));
    let mut crystal_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x68b7e9));
    crystal_material.emissive = Color::from_hex(0x4f7e8b);
    crystal_material.shininess = 10.0;
    crystal_material.specular = Color::from_hex(0xffffff);
    let crystal_mesh = Mesh::new(geometry, crystal_material);
    {
        let mut object = crystal_mesh.borrow_mut();
        object.receive_shadow = true;
        object.cast_shadow = true;
    }
    scene.add(&crystal_mesh);

    // lights

    scene.add(&AmbientLight::new(Color::from_hex(0x757f8e), 3.0));

    let directional_light = DirectionalLight::new(Color::from_hex(0xfffecd), 1.5);
    {
        let mut object = directional_light.borrow_mut();
        object.position.set(100.0, 100.0, 100.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
    }
    scene.add(&directional_light);

    // `new THREE.SpotLight( 0xffc100, 10, 10, Math.PI / 16, .02, 2 )`.
    let spot_light = SpotLight::new(Color::from_hex(0xffc100), 10.0);
    {
        let mut object = spot_light.borrow_mut();
        object.position.set(2.0, 2.0, 0.0);
        object.cast_shadow = true;
        let light = object.light_mut().unwrap();
        light.distance = 10.0;
        light.angle = PI / 16.0;
        light.penumbra = 0.02;
        light.decay = 2.0;
    }
    scene.add(&spot_light);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;
    renderer.shadow_map_type = ShadowMapType::Basic;

    let scene = Rc::new(RefCell::new(scene));

    let mut render_pipeline = RenderPipeline::new();
    let (normal_edge_strength_node, normal_edge_strength) = uniform_settable(Type::F32, vec![0.3]);
    let (depth_edge_strength_node, depth_edge_strength) = uniform_settable(Type::F32, vec![0.4]);
    let scene_pass = pixelation_pass(6, normal_edge_strength_node, depth_edge_strength_node);
    render_pipeline.output_node = Some(scene_pass.node());

    // `new OrbitControls( camera, renderer.domElement )`: the constructor's
    // `update()`, with no rotation, pan or zoom pending, leaves the camera
    // where it is, looking at the target. See the module docs.
    camera.look_at(&Vector3::ZERO);

    let camera = Rc::new(RefCell::new(camera));
    scene_pass.pass().set_scene(scene.clone(), camera.clone());

    App {
        renderer,
        scene,
        camera,
        crystal_mesh,
        timer,
        params: Params {
            pixel_aligned_panning: true,
        },
        normal_edge_strength,
        depth_edge_strength,
        scene_pass,
        render_pipeline,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    let t = app.timer.get_elapsed();

    {
        let mut crystal = app.crystal_mesh.borrow_mut();
        if let Some(material) = crystal.mesh_mut().and_then(|mesh| mesh.material.as_mut()) {
            material.emissive_intensity = (t * 3.0).sin() * 0.5 + 0.5;
        }
        crystal.position.y = 0.7 + (t * 2.0).sin() * 0.05;
        let current = crystal.rotation;
        crystal.set_rotation(current.x, stop_go_eased(t, 2.0, 4.0) * 2.0 * PI, current.z);
    }

    let (width, height) = app.renderer.size();
    let aspect_ratio = width / height;

    let mut camera = app.camera.borrow_mut();
    if app.params.pixel_aligned_panning {
        let pixel_size = app.scene_pass.pixel_size() as f64;

        pixel_align_frustum(
            &mut camera,
            aspect_ratio,
            (width / pixel_size).floor(),
            (height / pixel_size).floor(),
        );
    } else if camera.left != -aspect_ratio || camera.top != 1.0 {
        // Reset the Camera Frustum if it has been modified
        camera.left = -aspect_ratio;
        camera.right = aspect_ratio;
        camera.top = 1.0;
        camera.bottom = -1.0;
        camera.update_projection_matrix();
    }
    drop(camera);

    app.render_pipeline.render(&mut app.renderer);
}

// Helper functions

fn ease_in_out_cubic(x: f64) -> f64 {
    x.powi(2) * 3.0 - x.powi(3) * 2.0
}

fn linear_step(x: f64, edge0: f64, edge1: f64) -> f64 {
    let w = edge1 - edge0;
    let m = 1.0 / w;
    let y0 = -m * edge0;
    (y0 + m * x).clamp(0.0, 1.0)
}

fn stop_go_eased(x: f64, downtime: f64, period: f64) -> f64 {
    // `( x / period ) | 0`: truncation toward zero.
    let cycle = (x / period).trunc();
    let tween = x - cycle * period;
    let lin_step = ease_in_out_cubic(linear_step(tween, downtime, period));
    cycle + lin_step
}

fn pixel_align_frustum(
    camera: &mut OrthographicCamera,
    aspect_ratio: f64,
    pixels_per_screen_width: f64,
    pixels_per_screen_height: f64,
) {
    // 0. Get Pixel Grid Units
    let world_screen_width = (camera.right - camera.left) / camera.zoom;
    let world_screen_height = (camera.top - camera.bottom) / camera.zoom;
    let pixel_width = world_screen_width / pixels_per_screen_width;
    let pixel_height = world_screen_height / pixels_per_screen_height;

    // 1. Project the current camera position along its local rotation bases
    let cam_pos = camera.object.get_world_position();
    let cam_rot = camera.object.get_world_quaternion();
    let mut cam_right = Vector3::new(1.0, 0.0, 0.0);
    cam_right.apply_quaternion(&cam_rot);
    let mut cam_up = Vector3::new(0.0, 1.0, 0.0);
    cam_up.apply_quaternion(&cam_rot);
    let cam_pos_right = cam_pos.dot(&cam_right);
    let cam_pos_up = cam_pos.dot(&cam_up);

    // 2. Find how far along its position is along these bases in pixel units
    let cam_pos_right_px = cam_pos_right / pixel_width;
    let cam_pos_up_px = cam_pos_up / pixel_height;

    // 3. Find the fractional pixel units and convert to world units
    let fract_x = cam_pos_right_px - js_round(cam_pos_right_px);
    let fract_y = cam_pos_up_px - js_round(cam_pos_up_px);

    // 4. Add fractional world units to the left/right top/bottom to align with the pixel grid
    camera.left = -aspect_ratio - (fract_x * pixel_width);
    camera.right = aspect_ratio - (fract_x * pixel_width);
    camera.top = 1.0 - (fract_y * pixel_height);
    camera.bottom = -1.0 - (fract_y * pixel_height);
    camera.update_projection_matrix();
}

/// `Math.round`, which rounds halves up rather than away from zero.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let aspect_ratio = width / height;
    let mut camera = app.camera.borrow_mut();
    camera.left = -aspect_ratio;
    camera.right = aspect_ratio;
    camera.update_projection_matrix();
    drop(camera);

    app.renderer.set_size(width, height);
}

/// `None`: the page's `OrbitControls` drive an `OrthographicCamera`, which
/// the port's controls do not support (see the module docs).
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// `None`, as [`controls`] is.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_pixel.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
