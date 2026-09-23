//! Port of `three.js/examples/webgpu_postprocessing_direct.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. `performance.now()` is pinned to 0,
//! but nothing here reads a clock: `animate()` adds a fixed step to
//! `object.rotation` **before** the render, so the graded frame has
//! `object.rotation = ( 0.005, 0.01, 0 )` and not zero.
//!
//! `Math.random()` is the grader's seeded sequence
//! ([`DeterministicRandom`](three_rs::testing::DeterministicRandom)). The
//! `Inspector` is constructed first and takes the five draws its lists make,
//! then the mesh loop takes nine each: the colour, three position components,
//! the `multiplyScalar` factor, three rotation angles and the scale.
//!
//! The post-processing is three lines and no addon:
//!
//! ```text
//! saturationFactor          = uniform( 0 )
//! renderPipeline            = new DirectRenderPipeline( renderer )
//! renderPipeline.outputNode = vec4( saturation( output.rgb, saturationFactor ), output.a )
//! ```
//!
//! `saturationFactor` is **0**, so the graded image is fully desaturated — the
//! flat-shaded spheres come out as their own luminances. What makes the rung
//! worth having is not the effect but where it happens: there is no
//! intermediate colour target and no output quad anywhere in the frame. The
//! transform is inlined at the end of *every* material's fragment shader,
//! including the background quad's, which is why the solid background is
//! substituted for a `uniform( color )` node rather than left as a clear
//! colour. See `docs/postprocessing.md`.

use std::rc::Rc;

use three_rs::core::Node;
use three_rs::nodes::node::Type;
use three_rs::nodes::tsl::{output_property, saturation, uniform_value, vec4_join};
use three_rs::testing::DeterministicRandom;
use three_rs::{
    sphere_geometry, AmbientLight, Color, DirectRenderPipeline, DirectionalLight, Mesh,
    MeshPhongNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene, ToneMapping,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `Math.random()` draws `new Inspector()` makes before the mesh loop — the
/// same five `webgpu_mesh_batch` and `webgpu_tsl_galaxy` document, from the
/// `List` constructors of the inspector's tabs.
pub const INSPECTOR_RANDOM_DRAWS: usize = 5;

/// `for ( let i = 0; i < 100; i ++ )`.
pub const COUNT: usize = 100;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `object`, the parent every sphere hangs off and the only
    /// thing `animate()` moves.
    pub object: Node,
    pub render_pipeline: DirectRenderPipeline,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    // `renderer.inspector = new Inspector()` — it draws from the shared
    // `Math.random` sequence and nothing else that reaches the frame.
    let mut random = DeterministicRandom::new();
    random.skip(INSPECTOR_RANDOM_DRAWS);

    //

    let camera = PerspectiveCamera::new(70.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.z = 400.0;

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let object = three_rs::core::Object3D::new_node();
    scene.add(&object);

    let geometry = Rc::new(sphere_geometry(1.0, 4, 4));

    for _ in 0..COUNT {
        // `new MeshPhongMaterial( { color: Math.random() * 0xffffff,
        // flatShading: true } )` — `Color.setHex()` floors, and the value is
        // sRGB, so it goes through the same conversion as any hex literal.
        let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(
            (random.next() * 0xffffff as f64).floor() as u32,
        ));
        material.flat_shading = true;

        let mesh = Mesh::new(geometry.clone(), material);
        {
            let mut mesh = mesh.borrow_mut();
            mesh.position
                .set(
                    random.next() - 0.5,
                    random.next() - 0.5,
                    random.next() - 0.5,
                )
                .normalize()
                .multiply_scalar(random.next() * 400.0);
            mesh.set_rotation(
                random.next() * 2.0,
                random.next() * 2.0,
                random.next() * 2.0,
            );
            let scale = random.next() * 50.0;
            mesh.scale.set(scale, scale, scale);
        }
        object.add(&mesh);
    }

    scene.add(&AmbientLight::new(Color::from_hex(0xcccccc), 1.0));

    let light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    light.borrow_mut().position.set(1.0, 1.0, 1.0);
    scene.add(&light);

    // direct post-processing

    // `uniform( 0 )`, which the GUI would drive between 0 and 1. The graded
    // frame is the constructed value.
    let saturation_factor = uniform_value(Type::F32, vec![0.0]);

    let mut render_pipeline = DirectRenderPipeline::new();
    // `vec4( saturation( output.rgb, saturationFactor ), output.a )` — `output`
    // is the material's own result, read back out of the `Output` property the
    // hook has just assigned it to.
    render_pipeline.output_node = Some(vec4_join(vec![
        saturation(output_property().rgb(), saturation_factor),
        output_property().a(),
    ]));

    App {
        renderer,
        scene,
        camera,
        object,
        render_pipeline,
    }
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    {
        let mut object = app.object.borrow_mut();
        let rotation = object.rotation;
        object.set_rotation(rotation.x + 0.005, rotation.y + 0.01, rotation.z);
    }

    app.render_pipeline
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_direct.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
