//! Port of `three.js/examples/webgpu_postprocessing_ssgi.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! A Cornell box — red and green side walls, white floor, back wall, ceiling
//! and two white boxes — lit by one shadow-casting point light under a
//! light-disc mesh. The scene pass writes `output`, `diffuseColor`, packed
//! view normals and `velocity`; [`ssgi`] turns colour, depth and normals into
//! an AO and a one-bounce GI texture, the page composites them as `color · ao
//! + diffuse · gi`, and [`traa`] resolves the per-frame noise.
//!
//! **There is no rung.** three lists `webgpu_postprocessing_ssgi` in
//! `test/e2e/puppeteer.js`'s exception list, so it has no reference
//! screenshot to grade against. What the port checks instead is the SSGI and
//! composite shaders, against three's dump in `tests/nodes_display_wgsl.rs`,
//! and the AO / GI targets over several frames in `tests/ssgi_frames.rs`.
//!
//! The frame `main()` writes is the page's first: the TRAA history is a copy
//! of that frame, so it is one frame's SSGI noise, unblended. The noise
//! converges over the frames after it, in the viewer.
//!
//! The page's inspector GUI (output view, the SSGI uniforms, the temporal
//! filtering toggle) is not ported; the uniforms are public fields of
//! [`SsgiNode`].

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::{box_geometry, cylinder_geometry, plane_geometry};
use three_rs::lights::PointLight;
use three_rs::nodes::display::{convert_to_texture, ssgi, traa, RttNode, SsgiNode, TraaNode};
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{
    diffuse_color, normal_view, output_property, pack_normal_to_rgb, vec4_join,
};
use three_rs::nodes::velocity::velocity;
use three_rs::objects::Background;
use three_rs::textures::TextureType;
use three_rs::{
    pass, AmbientLight, Color, Mesh, MeshBasicNodeMaterial, MeshPhysicalNodeMaterial, PassNode,
    PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// The page leaves `renderer.setPixelRatio( window.devicePixelRatio )`
/// commented out ("probably too costly for most hardware").
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `scene_pass`, `gi_pass` and `traa_node`, which jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub scene_pass: PassNode,
    pub gi_pass: SsgiNode,
    /// `compositePass`, which `traa()` converts to a texture.
    pub composite: RttNode,
    pub traa_node: TraaNode,
    pub render_pipeline: RenderPipeline,
}

/// `new THREE.MeshPhysicalMaterial( { color } )`: roughness 1, metalness 0.
fn physical(hex: u32) -> MeshPhysicalNodeMaterial {
    MeshPhysicalNodeMaterial::physical(Color::from_hex(hex), 1.0, 0.0)
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 10.0, 30.0);
    let camera = Rc::new(RefCell::new(camera));

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0xaaaaaa)));
    // Shared with `scene_pass`, which renders it; the meshes are added
    // through the shared handle, as the page adds them after `pass()`.
    let scene = Rc::new(RefCell::new(scene));

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    //

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 7.0, 0.0);
    controls.enable_pan = true;
    controls.min_distance = 1.0;
    controls.max_distance = 100.0;
    let _ = controls.update(&mut camera.borrow_mut(), None);

    //

    let scene_pass = pass(scene.clone(), camera.clone());
    // `mrt( { output, diffuseColor, normal: packNormalToRGB( normalView ),
    // velocity } )`. `normalView` is per material upstream, so it is handed
    // over as a closure (`docs/nodes.md` §23).
    let mut scene_mrt = mrt(vec![
        ("output", output_property()),
        ("diffuseColor", diffuse_color()),
    ]);
    scene_mrt.set_deferred("normal", || pack_normal_to_rgb(normal_view()));
    scene_mrt.set("velocity", velocity());
    scene_pass.set_mrt(scene_mrt);
    // `bandwidth optimization`: the diffuse and normal attachments are
    // `UnsignedByteType`; `output` and `velocity` stay half float.
    for name in ["diffuseColor", "normal"] {
        scene_pass
            .texture_named(name)
            .set_texture_type(TextureType::UnsignedByte);
    }

    // `getTextureNode( name )` for each input: besides the node, it is what
    // adds the attachment and links it to the pass. `toInspector( … )`
    // returns its node unchanged.
    let scene_pass_color = scene_pass.texture_node("output");
    let scene_pass_diffuse = scene_pass.texture_node("diffuseColor");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("normal");
    let _ = scene_pass.texture_node("velocity");

    // gi — `sceneNormal` is `sample( ( uv ) => unpackRGBToNormal( … ) )`;
    // [`ssgi`] takes the packed texture and unpacks it itself.
    let gi_pass = ssgi(
        &scene_pass.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("normal"),
        camera.clone(),
    );
    gi_pass.slice_count.set(vec![2.0]);
    gi_pass.step_count.set(vec![8.0]);

    // composite
    let ao = gi_pass.ao_node();
    let gi = gi_pass.gi_node();
    let composite_pass = vec4_join(vec![
        scene_pass_color
            .xyz()
            .mul(ao)
            .add(scene_pass_diffuse.xyz().mul(gi)),
        scene_pass_color.w(),
    ]);
    // `traa( compositePass, … )`'s `convertToTexture( beautyNode )`, made
    // here so the node can take the texture.
    let composite = convert_to_texture(composite_pass);

    // traa
    let traa_node = traa(
        &composite.texture(),
        &scene_pass.depth_texture(),
        &scene_pass.texture_named("velocity"),
        camera.clone(),
    );
    let mut render_pipeline = RenderPipeline::new();
    traa_node.attach(&mut render_pipeline);
    render_pipeline.output_node = Some(traa_node.node());

    // Cornell Box inspired scene

    // Walls
    let wall_geometry = Rc::new(plane_geometry(1.0, 1.0, 1, 1));

    // Left wall - red
    let left_wall = Mesh::new(wall_geometry.clone(), physical(0xff0000));
    {
        let mut object = left_wall.borrow_mut();
        object.scale.set(20.0, 15.0, 1.0);
        object.set_rotation(0.0, std::f64::consts::PI * 0.5, 0.0);
        object.position.set(-10.0, 7.5, 0.0);
        object.receive_shadow = true;
    }
    scene.borrow().add(&left_wall);

    // Right wall - green
    let right_wall = Mesh::new(wall_geometry.clone(), physical(0x00ff00));
    {
        let mut object = right_wall.borrow_mut();
        object.scale.set(20.0, 15.0, 1.0);
        object.set_rotation(0.0, std::f64::consts::PI * -0.5, 0.0);
        object.position.set(10.0, 7.5, 0.0);
        object.receive_shadow = true;
    }
    scene.borrow().add(&right_wall);

    // White walls and boxes — one material shared by all five meshes.
    let white_material = physical(0xffffff);

    // Floor
    let floor = Mesh::new(wall_geometry.clone(), white_material.clone());
    {
        let mut object = floor.borrow_mut();
        object.scale.set(20.0, 20.0, 1.0);
        object.set_rotation(std::f64::consts::PI * -0.5, 0.0, 0.0);
        object.receive_shadow = true;
    }
    scene.borrow().add(&floor);

    // Back wall
    let back_wall = Mesh::new(wall_geometry.clone(), white_material.clone());
    {
        let mut object = back_wall.borrow_mut();
        object.scale.set(15.0, 20.0, 1.0);
        object.set_rotation(0.0, 0.0, std::f64::consts::PI * -0.5);
        object.position.set(0.0, 7.5, -10.0);
        object.receive_shadow = true;
    }
    scene.borrow().add(&back_wall);

    // Ceiling
    let ceiling = Mesh::new(wall_geometry, white_material.clone());
    {
        let mut object = ceiling.borrow_mut();
        object.scale.set(20.0, 20.0, 1.0);
        object.set_rotation(std::f64::consts::PI * 0.5, 0.0, 0.0);
        object.position.set(0.0, 15.0, 0.0);
        object.receive_shadow = true;
    }
    scene.borrow().add(&ceiling);

    // Boxes
    let tall_box = Mesh::new(
        Rc::new(box_geometry(5.0, 7.0, 5.0, 1, 1, 1)),
        white_material.clone(),
    );
    {
        let mut object = tall_box.borrow_mut();
        object.set_rotation(0.0, std::f64::consts::PI * 0.25, 0.0);
        object.position.set(-3.0, 3.5, -2.0);
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.borrow().add(&tall_box);

    let short_box = Mesh::new(
        Rc::new(box_geometry(4.0, 4.0, 4.0, 1, 1, 1)),
        white_material,
    );
    {
        let mut object = short_box.borrow_mut();
        object.set_rotation(0.0, std::f64::consts::PI * -0.1, 0.0);
        object.position.set(4.0, 2.0, 4.0);
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.borrow().add(&short_box);

    // Light source geometry
    let light_source = Mesh::new(
        Rc::new(cylinder_geometry(2.5, 2.5, 1.0, 64)),
        MeshBasicNodeMaterial::new(),
    );
    light_source.borrow_mut().position.y = 15.0;
    scene.borrow().add(&light_source);

    // Point light — `new THREE.PointLight( '#ffffff', 100 )` with `distance =
    // 100` set afterwards.
    let point_light = PointLight::new(Color::from_hex(0xffffff), 100.0, 100.0);
    {
        let mut object = point_light.borrow_mut();
        object.position.set(0.0, 13.0, 0.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.map_size.x = 1024.0;
        shadow.map_size.y = 1024.0;
    }
    scene.borrow().add(&point_light);

    // Ambient light
    scene
        .borrow()
        .add(&AmbientLight::new(Color::from_hex(0x0c0c0c), 1.0));

    App {
        renderer,
        scene,
        camera,
        controls,
        scene_pass,
        gi_pass,
        composite,
        traa_node,
        render_pipeline,
    }
}

/// The page's `animate()`: `controls.update()`, then the pipeline.
pub fn animate(app: &mut App) {
    let _ = app.controls.update(&mut app.camera.borrow_mut(), None);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`. The TRAA history restarts at the new size
/// on the next frame.
pub fn resize(app: &mut App, width: f64, height: f64) {
    {
        let mut camera = app.camera.borrow_mut();
        camera.aspect = width / height;
        camera.update_projection_matrix();
    }
    app.renderer.set_size(width, height);
    app.controls.set_element_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_ssgi.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
