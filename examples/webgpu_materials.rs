//! Port of `three.js/examples/webgpu_materials.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `Date.now()` is 0 — which puts
//! the camera at `( cos( 0 ) * 1000, 200, sin( 0 ) * 1000 )`.
//!
//! `TextureLoader.load()` is asynchronous on the page, but the harness only
//! fires its single RAF once the network is idle, so both images are present
//! for the graded frame; here they are decoded synchronously.
//!
//! `renderer.inspector = new Inspector()` only registers the renderer with the
//! inspector panel and changes nothing about the graded frame, and the `resize`
//! listener never fires.
//!
//! Seventeen teapots, sixteen of them `MeshBasicNodeMaterial` with a different
//! `colorNode` and one `MeshNormalMaterial`. The generated WGSL for every one
//! of them is in `examples/dump_wgsl.rs` under `materials_*`, diffed against
//! three's own dump.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::nodes::tsl::{
    call_wgsl, camera_projection_matrix, code, float, inline_fn, loop_index, loop_statement,
    normal_local, normal_world, osc_sine, position_local, position_world, screen_uv, texture,
    texture_uv, time, to_var, triplanar_texture, uv, vec2_join, vec3, vec4, wgsl_fn,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::testing::DeterministicRandom;
use three_rs::textures::Wrapping;
use three_rs::utils::date_now_ms;
use three_rs::{
    teapot_geometry, Color, GridHelper, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `objects` array — the teapots, without the grid helper.
    pub objects: Vec<three_rs::core::Node>,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's `normalView` accessor is not re-exported at the crate root, so
/// this is the one place that reaches into the node module for it.
fn normal_view() -> NodeRef {
    three_rs::nodes::tsl::normal_view()
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 2000.0);
    camera.node.borrow_mut().position.set(0.0, 200.0, 800.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    // Grid

    let helper = GridHelper::new(
        1000.0,
        40,
        Color::from_hex(0x303030),
        Color::from_hex(0x303030),
    );
    helper.borrow_mut().position.y = -75.0;
    scene.add(&helper);

    // Materials

    let loader = three_rs::TextureLoader::new();

    let uv_texture = loader
        .load(examples_dir().join("textures/uv_grid_opengl.jpg"))
        .unwrap();
    uv_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let opacity_texture = loader
        .load(examples_dir().join("textures/alphaMap.jpg"))
        .unwrap();
    opacity_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let mut materials: Vec<MeshBasicNodeMaterial> = Vec::new();

    let basic = |color_node: NodeRef| {
        let mut material = MeshBasicNodeMaterial::new();
        material.color_node = Some(color_node);
        material
    };

    //
    //	BASIC
    //

    materials.push(basic(position_local()));
    materials.push(basic(position_world()));
    materials.push(basic(normal_local()));
    materials.push(basic(normal_world()));
    materials.push(basic(normal_view()));
    materials.push(basic(texture(&uv_texture)));

    // Opacity
    let mut material = basic(Color::from_hex(0x0099FF).into());
    material.opacity_node = Some(texture(&uv_texture));
    material.transparent = true;
    materials.push(material);

    // AlphaTest
    let mut material = basic(texture(&uv_texture));
    material.opacity_node = Some(texture(&opacity_texture));
    material.alpha_test_node = Some(float(0.5));
    materials.push(material);

    // camera
    materials.push(basic(camera_projection_matrix().mul(position_local())));

    // Normal
    let mut material = MeshBasicNodeMaterial::normal();
    material.opacity = 0.5;
    material.transparent = true;
    materials.push(material);

    //
    //	ADVANCED
    //

    // Custom ShaderNode ( desaturate filter )
    let desaturate_shader_node = inline_fn(1, Type::F32, |input| {
        vec3(0.299, 0.587, 0.114).dot(input[0].clone().xyz())
    });
    materials.push(basic(three_rs::nodes::tsl::call(
        &desaturate_shader_node,
        vec![texture(&uv_texture)],
    )));

    // Custom ShaderNode(no inputs) > Approach 2
    //
    // Three writes this as a second `Fn()` that captures the texture; an
    // unlaid-out `Fn()` contributes nothing to the graph but its body, so in
    // Rust the capture is just the expression. The two materials generate
    // byte-identical WGSL and share one pipeline, which is what three does too
    // and what `tests/e2e` asserts through `renderer.info()`.
    materials.push(basic(
        vec3(0.299, 0.587, 0.114).dot(texture(&uv_texture).xyz()),
    ));

    // Custom WGSL ( desaturate filter ) — the source is three's, verbatim,
    // tabs and all, because `wgslFn` copies everything after the declaration
    // line straight into the shader.
    let desaturate_wgsl_fn = wgsl_fn(DESATURATE_WGSL, vec![]);

    // include example
    let some_wgsl_fn = wgsl_fn(SOME_WGSL, vec![code(&desaturate_wgsl_fn)]);

    materials.push(basic(call_wgsl(
        &some_wgsl_fn,
        vec![("color", texture(&uv_texture).xyz())],
    )));

    // Custom WGSL
    let get_wgsl_texture_sample = wgsl_fn(GET_WGSL_TEXTURE_SAMPLE, vec![]);

    let texture_node = texture(&uv_texture);

    materials.push(basic(call_wgsl(
        &get_wgsl_texture_sample,
        vec![
            ("tex", texture_node.clone()),
            ("tex_sampler", texture_node),
            ("uv", uv()),
        ],
    )));

    // Triplanar Texture Mapping
    materials.push(basic(triplanar_texture(
        &uv_texture,
        None,
        None,
        float(0.01),
    )));

    // Screen Projection Texture
    materials.push(basic(texture_uv(&uv_texture, screen_uv().flip_y())));

    // Loop
    //
    // `LoopNode.generate()` returns an empty snippet, so `vec4( colorNode )`
    // in `setupDiffuseColor()` casts nothing: three emits
    // `DiffuseColor = vec4<f32>(  );`, the loop's result is thrown away and
    // this teapot renders opaque black. **Reproduced on purpose** — see
    // `docs/nodes.md` §8. The body is still built, because three still builds
    // it and its texture bindings are still in the layout.
    const LOOP_COUNT: usize = 10;
    let i = loop_index();
    let output = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    let scale = osc_sine(time()).mul(0.09);
    let scale_i = scale.mul(i.to(Type::F32));
    // `scaleI.negate()` is read twice. `Node::Neg` is not a kind `needs_var`
    // promotes, so three's usage-counted temp has to be asked for here.
    let scale_i_neg = to_var(None, scale_i.clone().negate());
    let tap =
        |offset: NodeRef| output.assign(output.add(texture_uv(&uv_texture, uv().add(offset))));
    materials.push(basic(loop_statement(
        LOOP_COUNT,
        i,
        vec![
            tap(vec2_join(vec![scale_i.clone(), float(0.0)])),
            tap(vec2_join(vec![scale_i_neg.clone(), float(0.0)])),
            tap(vec2_join(vec![float(0.0), scale_i])),
            tap(vec2_join(vec![float(0.0), scale_i_neg])),
        ],
    )));

    //
    // Geometry
    //

    let geometry = Rc::new(teapot_geometry(50.0, 18));

    // `Math.random()` is the harness' deterministic sequence: three draws per
    // mesh, one per Euler angle, in material-creation order.
    let mut random = DeterministicRandom::new();
    let mut objects = Vec::new();
    for material in materials {
        let mesh = Mesh::new(geometry.clone(), material);
        {
            let mut object = mesh.borrow_mut();
            object.position.x = (objects.len() % 4) as f64 * 200.0 - 400.0;
            object.position.z = (objects.len() / 4) as f64 * 200.0 - 200.0;
            object.set_rotation(
                random.next() * 200.0 - 100.0,
                random.next() * 200.0 - 100.0,
                random.next() * 200.0 - 100.0,
            );
        }
        objects.push(mesh.clone());
        scene.add(&mesh);
    }

    //

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        scene,
        camera,
        objects,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    // `const timer = 0.0001 * Date.now();`
    let timer = 0.0001 * date_now_ms();

    {
        let mut camera_object = app.camera.node.borrow_mut();
        camera_object.position.x = timer.cos() * 1000.0;
        camera_object.position.z = timer.sin() * 1000.0;
    }

    // `camera.lookAt( scene.position )` — the scene sits at the origin.
    app.camera.look_at(&Vector3::ZERO);

    for object in &app.objects {
        let mut object = object.borrow_mut();
        let (x, y, z) = (object.rotation.x, object.rotation.y, object.rotation.z);
        object.set_rotation(x + 0.01, y + 0.005, z);
    }

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
///
/// The rung harness never calls this — the graded frame is always
/// 800 x 500 — but the viewer and the browser shell do, so the example
/// owns its own reaction to a resized canvas instead of the host
/// guessing at one.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
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

const DESATURATE_WGSL: &str = "
\t\t\t\t\tfn desaturate( color:vec3<f32> ) -> vec3<f32> {

\t\t\t\t\t\tlet lum = vec3<f32>( 0.299, 0.587, 0.114 );

\t\t\t\t\t\treturn vec3<f32>( dot( lum, color ) );

\t\t\t\t\t}
\t\t\t\t";

const SOME_WGSL: &str = "
\t\t\t\t\tfn someFn( color:vec3<f32> ) -> vec3<f32> {

\t\t\t\t\t\treturn desaturate( color );

\t\t\t\t\t}
\t\t\t\t";

const GET_WGSL_TEXTURE_SAMPLE: &str = "
\t\t\t\t\tfn getWGSLTextureSample( tex: texture_2d<f32>, tex_sampler: sampler, uv:vec2<f32> ) -> vec4<f32> {

\t\t\t\t\t\treturn textureSample( tex, tex_sampler, uv ) * vec4<f32>( 0.0, 1.0, 0.0, 1.0 );

\t\t\t\t\t}
\t\t\t\t";

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
        .unwrap_or_else(|| "target/webgpu_materials.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
