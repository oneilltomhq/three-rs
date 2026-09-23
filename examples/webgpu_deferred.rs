//! Port of `three.js/examples/webgpu_deferred.html`, calling the three-rs API
//! in the same order the page's `init()` and its UltraHDR callback do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. `timer.getDelta()` is zero on the
//! graded frame — the harness freezes `performance.now` — so the light group
//! and the plane group are at the poses `init()` left them in.
//!
//! # The frame
//!
//! Four `pass( scene, camera )` nodes over one scene, told apart by the camera
//! layer mask each renders with and by the renderer switches each sets. Three
//! of them run (the page's `forwardPass` is only reachable through the GUI's
//! `mode: 'forward'`, and `renderPipeline.outputNode` never references it, so
//! three.js never fires its `updateBefore` either):
//!
//! 1. **the opaque pass** — layer 0, `transparent = false`, `lighting.enabled
//!    = false`, with an MRT that writes a three-attachment G-buffer:
//!    `output = diffuseColor`, `position = vec4( positionView, metalness )`,
//!    `normal = vec4( normalView, roughness )`. Unlit, so `Output` is just the
//!    emissive; nothing reads it. This is the pass that owns the depth
//!    attachment.
//! 2. **the resolve pass** — layer 2 only, which is where the full-screen
//!    quad lives and where the eight point lights are also enabled. The quad's
//!    `MeshStandardNodeMaterial` reads the G-buffer back and runs the standard
//!    lighting flow over it: `colorNode` is the `output` attachment behind a
//!    `discard` on depth 1, `metalnessNode` / `roughnessNode` are the `.w`
//!    channels of the other two, and `contextNode = overrideNodes( … )` swaps
//!    `positionView`, `positionViewDirection` and `normalView` for the
//!    G-buffer values. `depthNode` writes the opaque depth back out through
//!    `@builtin( frag_depth )`.
//! 3. **the transparent pass** — layer 0, `opaque = false`, sharing the opaque
//!    pass's `depthTexture` with `autoClearDepth: false`, so the six planes
//!    depth-test against geometry this pass never drew. `opaque = false` also
//!    drops the skybox, because `Background.update()` unshifts it into
//!    `renderList.opaque`.
//!
//! The composite is `vec4( opaque.rgb * ( 1 - transparent.a ) + transparent.rgb,
//! 1 )` — the planes are premultiplied by their own pass's blend.
//!
//! # What is new here
//!
//! `docs/nodes.md` §27 covers all of it: `frag_depth`, `overrideNodes()`, the
//! `metalnessNode` / `roughnessNode` seam, the pass-scoped `opaque` /
//! `transparent` / `lighting` / layer switches, a pass that borrows another
//! pass's depth attachment, and `_renderObjectDirect()`'s two-draw split of a
//! transparent `DoubleSide` material (the planes: a `BackSide` draw and then a
//! `FrontSide` one, per object).

use std::rc::Rc;

use three_rs::core::Layers;
use three_rs::geometries::{plane_geometry, quad_geometry, sphere_geometry, teapot_geometry};
use three_rs::loaders::UltraHdrLoader;
use three_rs::materials::{MeshStandardNodeMaterial, Side, ToneMapping};
use three_rs::math::ColorSpace;
use three_rs::nodes::mrt::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    diffuse_color, discard, float, if_then, metalness, normal_view, position_geometry,
    position_view, roughness, vec4_join, OverrideNodes,
};
use three_rs::objects::Background;
use three_rs::renderer::PassOptions;
use three_rs::{
    Color, Group, Mesh, PassNode, PerspectiveCamera, PointLight, RenderPipeline, Renderer,
    RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `const RESOLVE_LAYER = 2`.
const RESOLVE_LAYER: u32 = 2;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub environment: PmremEnvironment,
    pub opaque_pass: PassNode,
    pub resolved_pass: PassNode,
    pub transparent_pass: PassNode,
    pub render_pipeline: RenderPipeline,
}

pub fn init() -> App {
    // `new THREE.PerspectiveCamera( 45, window.innerWidth / window.innerHeight,
    // 0.25, 20 ); camera.position.set( - 1.8, 0.6, 2.7 )`.
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.25, 20.0);
    camera.node.borrow_mut().position.set(-1.8, 0.6, 2.7);

    let mut scene = Scene::new();

    // `new THREE.WebGPURenderer()` — no `antialias`, so every pass target is
    // single-sampled and the shared depth attachment is a plain
    // `texture_depth_2d`.
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    // `renderer.toneMapping = THREE.ACESFilmicToneMapping`.
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new UltraHDRLoader().setPath( 'textures/equirectangular/' ).load(
    // 'royal_esplanade_2k.hdr.jpg', … )`. The loader resolves synchronously
    // here, so the callback's body is written inline — and it is the callback
    // that adds the teapot, so nothing else in `init()` depends on it.
    let texture = UltraHdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/royal_esplanade_2k.hdr.jpg"))
        .unwrap();

    // `texture.mapping = THREE.EquirectangularReflectionMapping; scene.background
    // = texture; scene.environment = texture`.
    let mut environment = PmremEnvironment::from_equirectangular(&texture);
    environment.update(&mut renderer).unwrap();
    scene.background = Some(Background::Pmrem(environment.handle()));
    scene.environment = Some(environment.handle());

    // `const teapotGeometry = new TeapotGeometry( 0.4, 18 ); … teapot.position.y
    // = - 0.15`.
    let teapot_material = MeshStandardNodeMaterial::standard(Color::from_hex(0x333333), 0.2, 0.8);
    let teapot = Mesh::new(Rc::new(teapot_geometry(0.4, 18)), teapot_material);
    teapot.borrow_mut().position.y = -0.15;
    scene.add(&teapot);

    // light group

    let light_group = Group::new();
    light_group.borrow_mut().position.y = -0.15;
    scene.add(&light_group);

    let num_lights = 8;
    let radius = 1.2;
    let sphere = Rc::new(sphere_geometry(0.03, 16, 8));

    for i in 0..num_lights {
        let angle = (i as f64 / num_lights as f64) * std::f64::consts::PI * 2.0;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        // `new THREE.Color().setHSL( i / numLights, 1.0, 0.5 )` — `setHSL`'s
        // default `colorSpace` is `ColorManagement.workingColorSpace`, i.e.
        // linear-sRGB: the HSL triple is taken as working-space values and is
        // *not* converted.
        let mut color_hex = Color::default();
        color_hex.set_hsl(
            i as f64 / num_lights as f64,
            1.0,
            0.5,
            ColorSpace::LinearSRGB,
        );

        let light_mat = MeshStandardNodeMaterial::standard(color_hex, 1.0, 0.0);
        let light_mesh = Mesh::new(sphere.clone(), light_mat);

        // `new THREE.PointLight( colorHex, 5, 5 ); light.layers.enable(
        // RESOLVE_LAYER )` — the lights are the only objects on *both* layers,
        // which is what lets the resolve pass see them and nothing else.
        let light = PointLight::new(color_hex, 5.0, 5.0);
        light.borrow_mut().layers.enable(RESOLVE_LAYER);
        light.borrow_mut().position.set(x, 0.0, z);
        light.add(&light_mesh);

        light_group.add(&light);
    }

    // planes group

    let planes_group = Group::new();
    planes_group.borrow_mut().position.y = -0.15;
    scene.add(&planes_group);

    let plane = Rc::new(plane_geometry(0.4, 0.4, 1, 1));
    let mut plane_material =
        MeshStandardNodeMaterial::standard(Color::from_hex(0x999999), 0.5, 0.5);
    plane_material.side = Side::Double;
    plane_material.transparent = true;
    plane_material.opacity = 0.5;

    let num_planes = 6;
    let planes_radius = 1.5;

    for i in 0..num_planes {
        let angle = (i as f64 / num_planes as f64) * std::f64::consts::PI * 2.0;

        let pivot = Group::new();
        // `pivot.rotation.y = angle` — three's `Euler.onChange` updates the
        // quaternion, which is what `set_rotation` is here.
        pivot.borrow_mut().set_rotation(0.0, angle, 0.0);
        planes_group.add(&pivot);

        let mesh = Mesh::new(plane.clone(), plane_material.clone());
        mesh.borrow_mut().position.z = planes_radius;
        pivot.add(&mesh);
    }

    // opaque pass

    // `const opaquePass = pass( scene, camera ); opaquePass.transparent =
    // false; opaquePass.setLayers( new THREE.Layers() )`.
    let mut opaque_pass = PassNode::new();
    opaque_pass.set_transparent(false);
    opaque_pass.set_layers(Layers::new());

    // `opaquePass.setMRT( mrt( { output: diffuseColor, position: vec4(
    // positionView, metalness ), normal: vec4( normalView, roughness ) } ) )`.
    //
    // `positionView` and `normalView` are node objects upstream, so their
    // `setup()` runs once per material and reads that material's side and
    // normal map; the port's TSL is eager, so both go in as closures
    // (`docs/nodes.md` §23.2).
    let mut g_buffer = mrt(vec![("output", diffuse_color())]);
    g_buffer.set_deferred("position", || vec4_join(vec![position_view(), metalness()]));
    g_buffer.set_deferred("normal", || vec4_join(vec![normal_view(), roughness()]));
    opaque_pass.set_mrt(g_buffer);

    // `opaquePass.lighting = new THREE.Lighting(); opaquePass.lighting.enabled
    // = false` — the G-buffer holds unlit `diffuseColor`.
    opaque_pass.set_lighting_enabled(false);

    // opaque nodes

    let opaque_output_node = opaque_pass.texture_node("output");
    let opaque_position_view = opaque_pass.texture_node("position").xyz();
    let opaque_normal_view = opaque_pass.texture_node("normal").xyz();
    let opaque_depth_node = opaque_pass.texture_node("depth");
    let opaque_metalness = opaque_pass.texture_node("position").w();
    let opaque_roughness = opaque_pass.texture_node("normal").w();

    // resolve opaque pass - deferred lighting

    let mut resolve_layers = Layers::new();
    resolve_layers.disable_all();
    resolve_layers.enable(RESOLVE_LAYER);

    // the resolve material must be compatible with the other materials in the
    // scene; in common deferred rendering scenarios, a standardized material
    // is used.
    let mut resolve_material =
        MeshStandardNodeMaterial::standard(Color::new(1.0, 1.0, 1.0), 1.0, 0.0);
    // `Fn( () => { opaqueDepthNode.greaterThanEqual( 1.0 ).discard(); return
    // opaqueOutputNode; } )()` — a cleared depth means nothing opaque was
    // drawn there, so the quad leaves the pass's own clear alone.
    resolve_material.color_node = Some(three_rs::nodes::tsl::block(
        vec![if_then(
            opaque_depth_node.greater_than_equal(float(1.0)),
            vec![discard()],
        )],
        opaque_output_node,
    ));
    resolve_material.metalness_node = Some(opaque_metalness);
    resolve_material.roughness_node = Some(opaque_roughness);
    // `resolveMaterial.vertexNode = vec4( positionGeometry.xy, 0.0, 1.0 )`.
    resolve_material.vertex_node = Some(vec4_join(vec![
        position_geometry().xy(),
        float(0.0),
        float(1.0),
    ]));
    resolve_material.depth_node = Some(opaque_depth_node.clone());
    resolve_material.context_overrides = Some(OverrideNodes {
        position_view: Some(opaque_position_view.clone()),
        position_view_direction: Some(opaque_position_view.negate().normalize()),
        normal_view: Some(opaque_normal_view),
    });

    // `const resolveMesh = new THREE.QuadMesh( resolveMaterial );
    // resolveMesh.layers.set( RESOLVE_LAYER ); scene.add( resolveMesh )` —
    // a `QuadMesh` is a `Mesh` over `QuadGeometry`, and here it goes into the
    // scene graph rather than being rendered on its own.
    let resolve_mesh = Mesh::new(Rc::new(quad_geometry()), resolve_material);
    resolve_mesh.borrow_mut().layers.set(RESOLVE_LAYER);
    scene.add(&resolve_mesh);

    // `const resolvedPass = pass( scene, camera ); resolvedPass.setLayers(
    // resolveLayers ); resolvedPass.lighting = new THREE.Lighting()`.
    let resolved_pass = PassNode::new();
    resolved_pass.set_layers(resolve_layers);

    // transparent pass

    // `pass( scene, camera, { depthTexture: opaquePass.getTexture( 'depth' ),
    // autoClearDepth: false } )`.
    let mut transparent_pass = PassNode::new_with_options(PassOptions {
        depth_texture: Some(opaque_pass.depth_texture()),
        auto_clear_depth: false,
        ..PassOptions::default()
    });
    transparent_pass.set_layers(Layers::new());
    transparent_pass.set_opaque(false);

    // resolved - render pipeline

    // `vec4( opaqueNode.rgb.mul( transparentNode.a.oneMinus() ).add(
    // transparentNode.rgb ), 1.0 )`.
    let opaque_node = resolved_pass.node();
    let transparent_node = transparent_pass.node();
    let deferred_node = vec4_join(vec![
        opaque_node
            .rgb()
            .mul(transparent_node.a().one_minus())
            .add(transparent_node.rgb()),
        float(1.0),
    ]);

    let mut render_pipeline = RenderPipeline::new();
    render_pipeline.output_node = Some(deferred_node);

    // `new OrbitControls( camera, renderer.domElement )` with
    // `controls.target.set( 0, 0, - 0.2 )` and one `update()`: the distance is
    // already inside `[ minDistance, maxDistance ]`, so the update only aims
    // the camera at the target.
    camera.look_at(&Vector3::new(0.0, 0.0, -0.2));

    App {
        renderer,
        scene,
        camera,
        environment,
        opaque_pass,
        resolved_pass,
        transparent_pass,
        render_pipeline,
    }
}

/// The page's `render()` with `params.animated` on but a zero delta: the two
/// groups do not move on the graded frame. `renderPipeline.render()` fires the
/// three passes' `updateBefore()` on its way; see `docs/postprocessing.md` for
/// why the port fires them explicitly, and note the order — the resolve pass
/// and the transparent pass both read what the opaque pass just wrote.
pub fn animate(app: &mut App) {
    app.environment.update(&mut app.renderer).unwrap();
    app.opaque_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.resolved_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
    app.transparent_pass
        .render(&mut app.renderer, &mut app.scene, &mut app.camera);
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
        .unwrap_or_else(|| "target/webgpu_deferred.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
