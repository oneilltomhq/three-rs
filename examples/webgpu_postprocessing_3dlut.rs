//! Port of `three.js/examples/webgpu_postprocessing_3dlut.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and the harness pins `time` at 0.
//!
//! The page loads all nine lookup tables up front — five `.CUBE` files through
//! `LUTCubeLoader`, one `.3dl` through `LUT3dlLoader`, three PNG strips
//! through `LUTImageLoader` — and grades `Bourbon 64.CUBE` at full intensity:
//! `lut3D( renderOutput( pass( scene, camera ) ), texture3D( lut ), 32,
//! uniform( 1 ) )` over the coffee mug and its smoke.
//!
//! One divergence, in `animate()`: the page writes `lutPass.lutNode.value =
//! lut.texture3D` every frame, rebinding the table under the same program. A
//! texture is an identity in the port's graph (see
//! `three_rs::nodes::display::Lut3DNode`), so picking another table builds a
//! new `Lut3DNode` and hands it to the pipeline instead; the graded frame never
//! changes table.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::plane_geometry;
use three_rs::loaders::{GltfLoader, Lut3dlLoader, LutCubeLoader, LutImageLoader, TextureLoader};
use three_rs::materials::{render_output, MeshBasicNodeMaterial, Side};
use three_rs::nodes::display::{lut_3d, Lut3DNode};
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::tsl::{
    block, mix, mod_, position_local, rotate_uv_about, smoothstep, texture_3d_sampled, texture_uv,
    time, uniform_settable, uv, vec2, vec2_join, vec3, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::{Data3DTexture, Wrapping};
use three_rs::{
    pass, Color, Mesh, PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters,
    Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's `params`.
pub struct Params {
    /// `params.lut`, a key of `lutMap`.
    pub lut: String,
    /// `params.intensity`, 0 to 1.
    pub intensity: f64,
}

/// One loaded `lutMap` entry: the table and its side
/// (`lut.texture3D.image.width`).
pub struct Lut {
    pub name: &'static str,
    pub texture_3d: Data3DTexture,
    pub size: u32,
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub params: Params,
    /// The page's `lutMap`, in its key order.
    pub luts: Vec<Lut>,
    pub scene_pass: PassNode,
    /// `renderOutput( scenePass )`, the input every `Lut3DNode` grades.
    pub output_pass: three_rs::nodes::NodeRef,
    /// The page's `lutPass`.
    pub lut_pass: Lut3DNode,
    /// The table `lut_pass` was built with.
    pub lut_pass_table: String,
    /// `lutPass.intensityNode`, the page's `uniform( 1 )`.
    pub intensity: SettableValue,
    intensity_node: three_rs::nodes::NodeRef,
    pub render_pipeline: RenderPipeline,
}

/// The page's `lutMap` keys, in order.
pub const LUT_NAMES: [&str; 9] = [
    "Bourbon 64.CUBE",
    "Chemical 168.CUBE",
    "Clayton 33.CUBE",
    "Cubicle 99.CUBE",
    "Remy 24.CUBE",
    "Presetpro-Cinematic.3dl",
    "NeutralLUT",
    "B&WLUT",
    "NightLUT",
];

/// The page's loader loop: `/\.CUBE$/i` through `LUTCubeLoader`, `/\LUT$/i`
/// (`\L` is a plain `L`) through `LUTImageLoader` as `luts/${name}.png`,
/// anything else through `LUT3dlLoader`.
fn load_luts() -> Vec<Lut> {
    let dir = examples_dir().join("luts");
    let cube = LutCubeLoader::new();
    let image = LutImageLoader::new();
    let three_dl = Lut3dlLoader::new();
    LUT_NAMES
        .iter()
        .map(|&name| {
            let upper = name.to_ascii_uppercase();
            let (texture_3d, size) = if upper.ends_with(".CUBE") {
                let lut = cube.load(dir.join(name)).expect("a .CUBE table");
                (lut.texture_3d, lut.size)
            } else if upper.ends_with("LUT") {
                let lut = image
                    .load(dir.join(format!("{name}.png")))
                    .expect("a LUT strip");
                (lut.texture_3d, lut.size)
            } else {
                let lut = three_dl.load(dir.join(name)).expect("a .3dl table");
                (lut.texture_3d, lut.size)
            };
            Lut {
                name,
                texture_3d,
                size,
            }
        })
        .collect()
}

/// The smoke's `MeshBasicNodeMaterial`: a twisting, wind-bent plane whose
/// alpha is perlin noise faded at the edges.
fn smoke_material() -> MeshBasicNodeMaterial {
    // texture

    let noise_texture = TextureLoader::new()
        .load(examples_dir().join("textures/noises/perlin/128x128.png"))
        .expect("128x128.png");
    noise_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // material

    let mut material = MeshBasicNodeMaterial::new();
    material.transparent = true;
    material.side = Side::Double;
    material.depth_write = false;

    // position

    // twist

    let twist_noise_uv = vec2_join(vec![
        three_rs::nodes::tsl::float(0.5),
        mod_(uv().y().mul(0.2).sub(time().mul(0.005)), 1.0),
    ]);
    let twist = texture_uv(&noise_texture, twist_noise_uv).x().mul(10.0);
    let twist_assign = position_local().xz().assign(rotate_uv_about(
        position_local().xz(),
        twist,
        vec2(0.0, 0.0),
    ));

    // wind

    let wind_offset = vec2_join(vec![
        texture_uv(
            &noise_texture,
            mod_(vec2_join(vec![0.25.into(), time().mul(0.01)]), 1.0),
        )
        .x()
        .sub(0.5),
        texture_uv(
            &noise_texture,
            mod_(vec2_join(vec![0.75.into(), time().mul(0.01)]), 1.0),
        )
        .x()
        .sub(0.5),
    ])
    .mul(uv().y().pow(2.0).mul(10.0));
    let wind_assign = position_local().add_assign(wind_offset);

    material.position_node = Some(block(vec![twist_assign, wind_assign], position_local()));

    // color

    // alpha

    let alpha_noise_uv = uv()
        .mul(vec2(0.5, 0.3))
        .add(vec2_join(vec![0.0.into(), time().mul(0.03).negate()]));
    let alpha = texture_uv(&noise_texture, alpha_noise_uv)
        .x()
        .smoothstep(0.4, 1.0)
        // edges fade
        .mul(smoothstep(0.0, 0.1, uv().x()))
        .mul(smoothstep(0.0, 0.1, uv().x().one_minus()))
        .mul(smoothstep(0.0, 0.1, uv().y()))
        .mul(smoothstep(0.0, 0.1, uv().y().one_minus()));

    // color

    let final_color = mix(vec3(0.6, 0.3, 0.2), vec3(1.0, 1.0, 1.0), alpha.pow(3.0));

    material.color_node = Some(vec4_join(vec![final_color, alpha]));
    material
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(8.0, 10.0, 12.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    // LUTs

    let luts = load_luts();

    // baked model

    // `gltfLoader.load()` adds the mug from its callback, after `init()` has
    // added the smoke, so the mug is the scene's second child.
    let gltf =
        GltfLoader::load(examples_dir().join("models/gltf/coffeeMug.glb")).expect("coffeeMug.glb");
    {
        let baked = gltf
            .scene
            .get_object_by_name("baked")
            .expect("the baked mesh");
        let baked = baked.borrow();
        let map = baked
            .material()
            .and_then(|m| m.map.clone())
            .expect("a baked map");
        map.set_anisotropy(8);
    }

    // geometry

    let mut smoke_geometry = plane_geometry(1.0, 1.0, 16, 64);
    smoke_geometry.translate(0.0, 0.5, 0.0);
    smoke_geometry.scale(1.5, 6.0, 1.5);

    // mesh

    let smoke = Mesh::new(Rc::new(smoke_geometry), smoke_material());
    smoke.borrow_mut().position.y = 1.83;
    scene.add(&smoke);
    scene.add(&gltf.scene);

    // renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // post processing

    let mut render_pipeline = RenderPipeline::new();

    // ignore default output color transform ( toneMapping and outputColorSpace )
    // use renderOutput() for control the sequence

    render_pipeline.output_color_transform = false;

    // scene pass

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));
    let scene_pass = pass(scene.clone(), camera.clone());
    let output_pass = render_output(scene_pass.node(), renderer.tone_mapping);

    let params = Params {
        lut: "Bourbon 64.CUBE".to_string(),
        intensity: 1.0,
    };

    let (intensity_node, intensity) = uniform_settable(Type::F32, vec![1.0]);
    let lut = luts
        .iter()
        .find(|l| l.name == params.lut)
        .expect("the default table");
    let lut_pass = lut_3d(
        output_pass.clone(),
        &texture_3d_sampled(&lut.texture_3d),
        f64::from(lut.size),
        intensity_node.clone(),
    );

    render_pipeline.output_node = Some(lut_pass.node());

    // controls

    let controls = {
        let mut camera = camera.borrow_mut();
        let mut controls = OrbitControls::new(&mut camera);
        // The canvas the example renders at, standing in for the element's
        // `clientWidth` / `clientHeight`.
        controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
        controls.enable_damping = true;
        controls.min_distance = 0.1;
        controls.max_distance = 50.0;
        controls.target.y = 3.0;
        controls
    };
    // The page never calls `controls.update()` here; the first `animate()`
    // aims the camera at the raised target.

    App {
        renderer,
        scene,
        camera,
        controls,
        lut_pass_table: params.lut.clone(),
        params,
        luts,
        scene_pass,
        output_pass,
        lut_pass,
        intensity,
        intensity_node,
        render_pipeline,
    }
}

/// The page's `animate()`: `controls.update()`, the intensity and the table
/// written onto `lutPass`, then `renderPipeline.render()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.intensity.set(vec![app.params.intensity]);

    if let Some(lut) = app.luts.iter().find(|l| l.name == app.params.lut) {
        // `lutPass.lutNode.value = lut.texture3D`: a new node for a new table
        // (see the module doc).
        if app.lut_pass_table != app.params.lut {
            app.lut_pass = lut_3d(
                app.output_pass.clone(),
                &texture_3d_sampled(&lut.texture_3d),
                f64::from(lut.size),
                app.intensity_node.clone(),
            );
            app.render_pipeline.output_node = Some(app.lut_pass.node());
            app.lut_pass_table = app.params.lut.clone();
        }
        app.lut_pass.size().set(vec![f64::from(lut.size)]);
    }

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

/// The controls and the camera at once; see `webgpu_postprocessing_ca`.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_3dlut.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
