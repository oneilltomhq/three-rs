//! Port of `three.js/examples/webgpu_postprocessing_anamorphic.html`, calling
//! the three-rs API in the same order the page's top-level script does.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and `performance.now()` is pinned to
//! 0 — so `time` is 0 in the graded frame and the spheres' bob is
//! `sin( instanceIndex * 0.5 * 0.5 ) * 5`, which is **not** zero.
//!
//! `Math.random` is the harness's seeded sequence
//! ([`three_rs::testing::DeterministicRandom`]), drawn **four** times per
//! instance in this order: `position.x`, `.y`, `.z`, then the colour — 800
//! draws for the 200 instances. Nothing draws from it before the loop.
//!
//! The effect is a [`BloomNode`] with two things swapped, which is the whole point
//! of the example: `setResolutionScale( 0.25 )`, and a high pass that is not a
//! high pass at all but a **horizontal streak**. The bright pass proper —
//! `mix( vec4( 0 ), input, smoothstep( … ) )`, i.e. exactly
//! [`luminosity_high_pass`] — is wrapped in an [`rtt`], so it is computed once
//! into an 800×500 half-float target; the high-pass material then reads that
//! target eighty times along x, weighted by
//! `pow( 1 - |i| / halfSamples, 2 )`. Without the `rtt()` those eighty taps
//! would each be a re-evaluation of the bright pass.
//!
//! The `rtt()` target is `MirroredRepeatWrapping` on both axes and that is
//! load-bearing: the loop reads `uv.x ± 4i / width` with `i` up to ±40, which
//! is up to 0.2 outside `[0,1]`, so the streak folds back at the screen edge
//! instead of smearing the edge column. See
//! `docs/webgpu_postprocessing_anamorphic-progress.md` for the dump evidence.

use std::rc::Rc;

use three_rs::geometries::sphere_geometry;
use three_rs::nodes::display::{luminosity_high_pass, rtt, BloomNode, RttNode};
use three_rs::nodes::tsl::{
    block, distance, float, instance_index, loop_range, mix, position_local, screen_uv, time,
    to_var, uniform_value, uv, vec2, vec2_join, vec3_join, vec4, viewport,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::objects::Background;
use three_rs::testing::DeterministicRandom;
use three_rs::textures::Wrapping;
use three_rs::{
    Color, InstancedMesh, MeshBasicNodeMaterial, Object3D, PassNode, PerspectiveCamera,
    RenderPipeline, Renderer, RendererParameters, Scene, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `const maxCount = 200`.
const MAX_COUNT: usize = 200;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub scene_pass: PassNode,
    /// The `rtt()` inside the custom high pass. The port renders it explicitly,
    /// like every other node that owns a target; see `docs/postprocessing.md`.
    pub bright_pass: RttNode,
    pub bloom_pass: BloomNode,
    pub render_pipeline: RenderPipeline,
}

/// The page's replacement `bloomPass.highPassFn`.
///
/// `args` is what [`BloomNode`] hands any high pass: the input texture node and
/// the node's own `threshold` / `smoothWidth` uniforms. The bright pass built
/// from them is the default one; what this adds is the `rtt()` around it and
/// the horizontal loop over it.
pub fn anamorphic_high_pass(
    args: three_rs::nodes::display::HighPassInput,
    samples: NodeRef,
) -> (RttNode, NodeRef) {
    // `const brightPass = rtt( mix( vec4( 0 ), input, alpha ), null, null,
    // { wrapS: MirroredRepeatWrapping, wrapT: MirroredRepeatWrapping } )`.
    let bright_pass = rtt(luminosity_high_pass(args));
    bright_pass.set_wrapping(Wrapping::MirroredRepeat, Wrapping::MirroredRepeat);

    // `const total = vec4( 0 )`, written by the loop, so a var.
    let total = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    // `const halfSamples = samples.div( 2 )` — read three times (the negated
    // start, the end, and the softness divisor), so a var too.
    let half_samples = to_var(None, samples.div(float(2.0)));
    // `const invSize = vec2( 1.0 ).div( viewportSize )`. `viewportSize` is
    // `viewport.zw`, not `screenSize`: three's dump reads `render.nodeUniformN.zw`
    // off the four-component viewport uniform.
    let inv_size = vec2(1.0, 1.0).div(viewport().zw());

    // `Loop( { start: halfSamples.negate(), end: halfSamples }, … )`. The
    // bounds are nodes, so they are written into the loop header as they
    // build: `i32( ( - nodeVar1 ) )` and `i32( nodeVar1 )`.
    let loop_node = loop_range(
        "i",
        half_samples.negate().to(Type::I32),
        half_samples.to(Type::I32),
        |i| {
            // `float( i )` — the index is an `i32`, every use of it an `f32`.
            let index = i.to(Type::F32);
            // `let softness = float( i ).abs().div( halfSamples ).oneMinus();
            //  softness = softness.pow( 2.0 );`
            let softness = index
                .abs()
                .div(half_samples.clone())
                .one_minus()
                .pow(float(2.0));
            // `vec2( uv().x.add( invSize.x.mul( i ).mul( 4.0 ) ), uv().y )`.
            let shifted_uv = vec2_join(vec![
                uv().x().add(inv_size.x().mul(index).mul(float(4.0))),
                uv().y(),
            ]);
            vec![total.add_assign(bright_pass.sample(shifted_uv).mul(softness))]
        },
    );

    // `return total.div( samples.div( 3.0 ) )`.
    let node = block(
        vec![total.clone(), half_samples.clone(), loop_node],
        total.div(samples.div(float(3.0))),
    );

    (bright_pass, node)
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 250.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 20.0);
    // `new OrbitControls( camera, … )` with the default target and one
    // `controls.update()` a frame: the camera keeps its position and looks at
    // the origin.
    camera.look_at(&Vector3::ZERO);

    let mut scene = Scene::new();
    // `scene.backgroundNode = Fn( () => mix( color( 0x111111 ),
    // color( 0x000000 ), screenUV.distance( 0.5 ).mul( 2.0 ) ) )()`.
    let dist = distance(screen_uv(), float(0.5)).mul(float(2.0));
    scene.background = Some(Background::Node(mix(
        Color::from_hex(0x111111),
        Color::from_hex(0x000000),
        dist,
    )));

    // `const timeScale = uniform( 0.5 )`.
    let time_scale = uniform_value(Type::F32, vec![0.5]);

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    // `positionLocal.add( vec3( 0, time.add( float( instanceIndex ).mul( 0.5 ) )
    // .mul( timeScale ).sin().mul( 5.0 ), 0 ) )`.
    let bob = time()
        .add(instance_index().to(Type::F32).mul(float(0.5)))
        .mul(time_scale)
        .sin()
        .mul(float(5.0));
    material.position_node =
        Some(position_local().add(vec3_join(vec![float(0.0), bob, float(0.0)])));

    let geometry = Rc::new(sphere_geometry(0.1, 32, 32));
    let instanced_mesh = InstancedMesh::new(geometry, material, MAX_COUNT);

    let mut random = DeterministicRandom::new();
    {
        let mut mesh = instanced_mesh.borrow_mut();
        let mut dummy = Object3D::default();
        for i in 0..MAX_COUNT {
            dummy.position.x = (random.next() - 0.5) * 20.0;
            dummy.position.y = (random.next() - 0.5) * 20.0;
            dummy.position.z = (random.next() - 0.5) * 20.0;
            dummy.update_matrix();
            mesh.set_matrix_at(i, &dummy.matrix);

            // `_color.setHex( Math.random() * 0xffffff )` — `setHex` floors,
            // and its default colour space is sRGB, so the instance colours
            // reach the buffer converted into the working space.
            let color = Color::from_hex((random.next() * 0xffffff as f64).floor() as u32);
            mesh.set_color_at(i, &color);
        }
    }

    scene.add(&instanced_mesh);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::Neutral;

    // post-processing

    let scene_pass = PassNode::new();

    let tint_color = {
        let c = Color::from_hex(0x7a8aff);
        uniform_value(Type::Vec3, vec![c.r, c.g, c.b])
    };
    // `const samples = uniform( 80 )` — a float uniform, which is why the loop
    // bound is `object.nodeUniform0 / 2.0` and not an integer division.
    let samples = uniform_value(Type::F32, vec![80.0]);

    // `bloom( scenePass.getTextureNode(), intensity, radius, threshold )` —
    // `toInspector( 'Color' )` names the node for the inspector panel and
    // returns it unchanged.
    let mut bright_pass = None;
    let mut bloom_pass = BloomNode::with_high_pass(scene_pass.texture_node("output"), |args| {
        let (node, high_pass) = anamorphic_high_pass(args, samples);
        bright_pass = Some(node);
        high_pass
    });
    // `bloom( …, intensity, radius, threshold )` with
    // `intensity = uniform( 5.0 )`, `radius = uniform( 0.0 )` and
    // `threshold = uniform( 0.3 )`.
    bloom_pass.strength.set(vec![5.0]);
    bloom_pass.radius.set(vec![0.0]);
    bloom_pass.threshold.set(vec![0.3]);
    bloom_pass.set_resolution_scale(0.25);
    let bright_pass = bright_pass.expect("three-rs: the high pass built its rtt()");

    let mut render_pipeline = RenderPipeline::new();
    // `renderPipeline.outputNode = scenePass.add( bloomPass.mul( tintColor ) )`.
    // `bloomPass.mul( tintColor )` is a `vec4` times a `vec3`, which widens the
    // tint with `w = 1.0` — three's dump reads
    // `( nodeVar3 * vec4<f32>( object.nodeUniform2, 1.0 ) )`.
    render_pipeline.output_node = Some(scene_pass.node().add(bloom_pass.node().mul(tint_color)));

    App {
        renderer,
        scene,
        camera,
        scene_pass,
        bright_pass,
        bloom_pass,
        render_pipeline,
    }
}

/// The page's `render()`: `renderPipeline.render()`, which fires all three
/// `updateBefore()`s on its way.
///
/// The order is the one three's dump *submits* in — the scene, then the
/// `rtt()`, then the bloom's twelve quads — even though three records the
/// bloom's high-pass descriptor before the RTT pass it nests inside. See
/// `docs/postprocessing.md` for why the port fires them explicitly.
pub fn animate(app: &mut App) {
    app.scene_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.bright_pass.render(&mut app.renderer);
    app.bloom_pass.render(&mut app.renderer);
    app.render_pipeline.render(&mut app.renderer);
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_anamorphic.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
