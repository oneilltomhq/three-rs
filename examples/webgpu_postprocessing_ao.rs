//! Port of `three.js/examples/webgpu_postprocessing_ao.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! A gallery room — checkerboard floor, two walls, pedestals, a torus knot,
//! columns, a vase, an armchair, a side table, a rug, five picture frames and
//! the Tennyson bust — lit by three wall-mounted spot lights and a dim
//! `RoomEnvironment`. A normal + velocity pre-pass feeds [`ao`] (GTAO), whose
//! result darkens the beauty pass's ambient term through the pass's AO
//! context, and [`traa`] resolves GTAO's temporal noise.
//!
//! The page starts in its `GTAO` mode; the inspector's `SSAO` switch, the
//! `aoOnly` view and the transparent-mesh toggles are GUI, and the port
//! builds the page's initial state only.
//!
//! `scene.environmentIntensity = 0.3` has no field in the port. Three reads
//! it as `materialEnvIntensity` (every material here has no `envMap` of its
//! own), a plain factor on both environment reads, so the port sets
//! `scene.environmentNode` to the same PMREM read scaled by 0.3 — the same
//! product, as an [`EnvironmentNode`].

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::extras::{Path, Shape};
use three_rs::geometries::{
    box_geometry, cylinder_geometry, extrude_geometry, lathe_geometry, plane_geometry,
    rounded_box_geometry, torus_knot_geometry, ExtrudeGeometryOptions,
};
use three_rs::loaders::GltfLoader;
use three_rs::materials::environment::EnvironmentNode;
use three_rs::math::{Box3, ColorSpace};
use three_rs::nodes::display::{ao, traa, GtaoNode, TraaNode};
use three_rs::nodes::mrt;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{float, normal_view, pack_normal_to_rgb, screen_uv};
use three_rs::nodes::velocity::velocity;
use three_rs::textures::{Texture, TextureType, Wrapping};
use three_rs::{
    pass, Background, Color, Group, Mesh, MeshBasicNodeMaterial, MeshStandardNodeMaterial,
    PassNode, PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, RoomEnvironment,
    Scene, SpotLight, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// The page's `params`, as far as the initial `GTAO` state reads them.
const SAMPLES: u32 = 16;
const RADIUS: f64 = 0.4;
const RESOLUTION_SCALE: f64 = 0.5;
const SCALE: f64 = 0.8;
const THICKNESS: f64 = 1.0;
const TEMPORAL_FILTERING: bool = true;
const TRANSPARENT_OPACITY: f64 = 0.3;

pub struct App {
    pub renderer: Renderer,
    /// Shared with both passes, which render it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with both passes, `ao_node` and `traa_node`, which jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub pre_pass: PassNode,
    pub scene_pass: PassNode,
    /// The page's `aoPass`.
    pub ao_node: GtaoNode,
    /// The page's `traaPass`.
    pub traa_node: TraaNode,
    /// The page's `transparentMesh`, hidden until the GUI shows it.
    pub transparent_mesh: three_rs::core::Node,
    pub render_pipeline: RenderPipeline,
}

/// `new THREE.MeshStandardMaterial( { color, roughness, metalness } )`.
fn standard(hex: u32, roughness: f64, metalness: f64) -> MeshBasicNodeMaterial {
    MeshStandardNodeMaterial::standard(Color::from_hex(hex), roughness, metalness)
}

/// The page's `floorCanvas`: `tilesX` x `tilesZ` tiles of 32 px, `'#d8d0c8'`
/// where `( x + z ) % 2 === 0` and `'#b8b0a8'` elsewhere, as RGBA rows from the
/// top of the canvas down.
fn checkerboard_pixels(tiles_x: usize, tiles_z: usize) -> (u32, u32, Vec<u8>) {
    let width = tiles_x * 32;
    let height = tiles_z * 32;
    let mut pixels = Vec::with_capacity(width * height * 4);
    for py in 0..height {
        let z = py / 32;
        for px in 0..width {
            let x = px / 32;
            let rgb: [u8; 3] = if (x + z) % 2 == 0 {
                [0xd8, 0xd0, 0xc8]
            } else {
                [0xb8, 0xb0, 0xa8]
            };
            pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
    (width as u32, height as u32, pixels)
}

/// The page's `addSpotLight( position, targetPosition )`.
fn add_spot_light(scene: &Scene, position: Vector3, target_position: Vector3) {
    let light = SpotLight::new(Color::from_hex(0xffe09e), 40.0);
    let target = {
        let mut object = light.borrow_mut();
        object.position = position;
        let light = object.light_mut().unwrap();
        light.angle = 0.6;
        light.penumbra = 1.0;
        light.distance = 6.5;
        light.decay = 2.0;
        let target = light.target.clone().expect("a SpotLight has a target");
        target.borrow_mut().position = target_position;
        target
    };
    scene.add(&light);
    scene.add(&target);
}

/// The page's `addColumn( x, z )`.
fn add_column(scene: &Scene, column_mat: &MeshBasicNodeMaterial, x: f64, z: f64) {
    let column_group = Group::new();

    let base = Mesh::new(
        Rc::new(box_geometry(0.6, 0.2, 0.6, 1, 1, 1)),
        column_mat.clone(),
    );
    base.borrow_mut().position.y = -1.9;
    column_group.add(&base);

    let shaft = Mesh::new(
        Rc::new(cylinder_geometry(0.18, 0.22, 5.0, 16)),
        column_mat.clone(),
    );
    shaft.borrow_mut().position.y = 0.7;
    column_group.add(&shaft);

    let capital = Mesh::new(
        Rc::new(box_geometry(0.55, 0.25, 0.55, 1, 1, 1)),
        column_mat.clone(),
    );
    capital.borrow_mut().position.y = 3.35;
    column_group.add(&capital);

    {
        let mut object = column_group.borrow_mut();
        object.scale.set_scalar(1.5);
        object.position.set(x, 1.0, z);
    }
    scene.add(&column_group);
}

/// The page's `addFrame( x, y, z, width, height, paintColor, rotY = 0 )`.
#[allow(clippy::too_many_arguments)]
fn add_frame(
    scene: &Scene,
    x: f64,
    y: f64,
    z: f64,
    width: f64,
    height: f64,
    paint: u32,
    rot_y: f64,
) {
    let frame_group = Group::new();
    let t = 0.1;

    let outer_w = width + t * 2.0;
    let outer_h = height + t * 2.0;
    let mut frame_shape = Shape::new();
    frame_shape
        .move_to(-outer_w / 2.0, -outer_h / 2.0)
        .line_to(outer_w / 2.0, -outer_h / 2.0)
        .line_to(outer_w / 2.0, outer_h / 2.0)
        .line_to(-outer_w / 2.0, outer_h / 2.0)
        .close_path();

    let mut hole = Path::new();
    hole.move_to(-width / 2.0, -height / 2.0)
        .line_to(width / 2.0, -height / 2.0)
        .line_to(width / 2.0, height / 2.0)
        .line_to(-width / 2.0, height / 2.0)
        .close_path();
    frame_shape.holes.push(hole);

    let frame_mat = standard(0x8b6840, 0.7, 0.0);
    let mut options = ExtrudeGeometryOptions::default();
    options.depth = 0.12;
    options.bevel_enabled = true;
    options.bevel_thickness = 0.02;
    options.bevel_size = Some(0.02);
    options.bevel_segments = 2;
    let geo = extrude_geometry(&[frame_shape], &options);
    let frame = Mesh::new(Rc::new(geo), frame_mat);
    frame_group.add(&frame);

    let paint_mat = standard(paint, 0.95, 0.0);
    let canvas = Mesh::new(Rc::new(plane_geometry(width, height, 1, 1)), paint_mat);
    canvas.borrow_mut().position.z = 0.001;
    frame_group.add(&canvas);

    {
        let mut object = frame_group.borrow_mut();
        object.position.set(x, y, z);
        // `if ( rotY ) frameGroup.rotation.y = rotY`.
        if rot_y != 0.0 {
            object.set_rotation(0.0, rot_y, 0.0);
        }
    }
    scene.add(&frame_group);
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 50.0);
    camera.node.borrow_mut().position.set(1.0, 3.0, 7.0);

    let scene = Rc::new(RefCell::new(Scene::new()));

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.tone_mapping = ToneMapping::Neutral;
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let camera = Rc::new(RefCell::new(camera));

    // controls

    // `controls = new OrbitControls( camera, renderer.domElement )`; the
    // target the page sets afterwards takes effect on `animate()`'s
    // `controls.update()`.
    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 2.0;
    controls.max_distance = 16.0;
    controls.target.set(0.0, 1.2, 0.0);

    // environment

    let mut room = RoomEnvironment::new();
    let environment = PmremEnvironment::from_scene(&mut renderer, &mut room, 0.04).unwrap();
    {
        let mut scene = scene.borrow_mut();
        scene.background = Some(Background::Color(Color::from_hex(0x666666)));
        scene.environment = Some(environment.handle());
        // `scene.environmentIntensity = 0.3` — see the module docs.
        let handle = environment.handle();
        scene.environment_node = Some(EnvironmentNode::new(move |uv, level| {
            handle.sample(uv, level).mul(float(0.3))
        }));
    }

    // post-processing

    let mut render_pipeline = RenderPipeline::new();

    // pre-pass

    // `toInspector( 'Normal', … )` returns its node unchanged.
    let mut pre_pass = pass(scene.clone(), camera.clone());
    pre_pass.set_transparent(false);

    pre_pass.set_mrt(mrt(vec![
        ("output", pack_normal_to_rgb(normal_view())),
        ("velocity", velocity()),
    ]));

    // `getTextureNode( name )` for each of the three: besides the node, it
    // is what adds the `velocity` attachment and links all three to the
    // pass. The page's `prePassNormal` — `unpackRGBToNormal(
    // prePass.getTextureNode().sample( uv ) )` — is the unpack [`ao`] does
    // on the `output` texture it is handed.
    let _ = pre_pass.texture_node("output");
    let _ = pre_pass.texture_node("depth");
    let _ = pre_pass.texture_node("velocity");

    // pre-pass - bandwidth optimization

    pre_pass
        .texture()
        .set_texture_type(TextureType::UnsignedByte);

    // scene pass

    let scene_pass = pass(scene.clone(), camera.clone());

    // traa ( resolves the temporal noise of GTAO )

    let traa_node = traa(
        &scene_pass.texture(),
        &pre_pass.depth_texture(),
        &pre_pass.texture_named("velocity"),
        camera.clone(),
    );
    traa_node.set_use_subpixel_correction(false);
    traa_node.attach(&mut render_pipeline);

    // ao: `createAO()` with `params.aoType === 'GTAO'`.

    let ao_node = ao(
        &pre_pass.depth_texture(),
        &pre_pass.texture(),
        camera.clone(),
    );

    // `updateParameters()`, GTAO branch.
    ao_node.set_samples(SAMPLES);
    ao_node.radius.set(vec![RADIUS]);
    ao_node.set_resolution_scale(RESOLUTION_SCALE);
    ao_node.scale.set(vec![SCALE]);
    ao_node.thickness.set(vec![THICKNESS]);
    ao_node.set_use_temporal_filtering(TEMPORAL_FILTERING);

    // `scenePass.contextNode = builtinAOContext( aoPass.getTextureNode()
    // .sample( screenUV ).r )`.
    scene_pass.set_context_ao(ao_node.sample(screen_uv()).x());

    // `updateOutput()`: `scenePass.options.samples = useTRAA ? 0 : 4` — the
    // port's `PassNode` has no per-pass sample count; it takes the
    // renderer's, which is 0 here (no `antialias`), the value GTAO wants.
    render_pipeline.output_node = Some(traa_node.node());

    // models

    let transparent_mesh;
    {
        let scene = scene.borrow();

        // wall-mounted spotlights

        add_spot_light(
            &scene,
            Vector3::new(-2.5, 5.0, -4.8),
            Vector3::new(-2.5, -2.0, -4.8),
        );
        add_spot_light(
            &scene,
            Vector3::new(2.5, 5.0, -4.8),
            Vector3::new(2.5, -2.0, -4.8),
        );
        add_spot_light(
            &scene,
            Vector3::new(-5.3, 5.0, 0.0),
            Vector3::new(-5.3, -2.0, 0.0),
        );

        // checkerboard floor

        let tiles_x = 16;
        let tiles_z = 16;
        // `new THREE.CanvasTexture( floorCanvas )`.
        let (width, height, pixels) = checkerboard_pixels(tiles_x, tiles_z);
        let floor_texture = Texture::new(width, height, Some(pixels));
        floor_texture.set_color_space(ColorSpace::Srgb);
        floor_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

        let mut floor_mat = standard(0xffffff, 0.7, 0.05);
        floor_mat.map = Some(floor_texture);
        let floor = Mesh::new(
            Rc::new(plane_geometry(tiles_x as f64, tiles_z as f64, 1, 1)),
            floor_mat,
        );
        {
            let mut object = floor.borrow_mut();
            object.set_rotation(-PI / 2.0, 0.0, 0.0);
            object.position.y = -2.0;
        }
        scene.add(&floor);

        // walls

        let wall_mat = standard(0xe0d8d0, 0.9, 0.0);

        let back_wall = Mesh::new(Rc::new(plane_geometry(16.0, 10.0, 1, 1)), wall_mat.clone());
        back_wall.borrow_mut().position.set(0.0, 3.0, -5.0);
        scene.add(&back_wall);

        let left_wall = Mesh::new(Rc::new(plane_geometry(16.0, 10.0, 1, 1)), wall_mat);
        {
            let mut object = left_wall.borrow_mut();
            object.set_rotation(0.0, PI / 2.0, 0.0);
            object.position.set(-5.5, 3.0, 0.0);
        }
        scene.add(&left_wall);

        // central pedestal

        let pedestal_mat = standard(0xf0ece8, 0.4, 0.05);

        let pedestal_base = Mesh::new(
            Rc::new(cylinder_geometry(0.9, 1.0, 0.2, 32)),
            pedestal_mat.clone(),
        );
        pedestal_base.borrow_mut().position.set(0.0, -1.9, 0.0);
        scene.add(&pedestal_base);

        let pedestal_shaft = Mesh::new(
            Rc::new(cylinder_geometry(0.55, 0.65, 1.5, 32)),
            pedestal_mat.clone(),
        );
        pedestal_shaft.borrow_mut().position.set(0.0, -1.05, 0.0);
        scene.add(&pedestal_shaft);

        let pedestal_top = Mesh::new(
            Rc::new(cylinder_geometry(0.8, 0.7, 0.25, 32)),
            pedestal_mat.clone(),
        );
        pedestal_top.borrow_mut().position.set(0.0, -0.18, 0.0);
        scene.add(&pedestal_top);

        // torus knot

        let knot_mat = standard(0xc0a060, 0.45, 0.0);
        let torus_knot = Mesh::new(
            Rc::new(torus_knot_geometry(0.5, 0.17, 128, 32, 2.0, 3.0)),
            knot_mat,
        );
        torus_knot.borrow_mut().position.set(0.0, 0.82, 0.0);
        scene.add(&torus_knot);

        // columns

        let column_mat = standard(0xe8e4de, 0.5, 0.0);

        add_column(&scene, &column_mat, -5.05, -4.55);
        add_column(&scene, &column_mat, 4.5, -4.55);
        add_column(&scene, &column_mat, -5.05, 3.0);

        // vase on its own pedestal

        let vase_profile = [
            (0.0, 0.5),
            (0.12, 0.5),
            (0.18, 0.7),
            (0.28, 0.9),
            (0.32, 1.0),
            (0.3, 1.1),
            (0.22, 1.15),
            (0.2, 1.2),
            (0.22, 1.25),
            (0.0, 1.25),
        ];

        let vase_mat = standard(0xd4806a, 0.6, 0.02);

        let vase_pedestal_base = Mesh::new(
            Rc::new(cylinder_geometry(0.6, 0.7, 0.15, 32)),
            pedestal_mat.clone(),
        );
        vase_pedestal_base
            .borrow_mut()
            .position
            .set(-5.0, -1.925, -2.0);
        scene.add(&vase_pedestal_base);

        let vase_pedestal_shaft = Mesh::new(
            Rc::new(cylinder_geometry(0.35, 0.42, 1.0, 32)),
            pedestal_mat.clone(),
        );
        vase_pedestal_shaft
            .borrow_mut()
            .position
            .set(-5.0, -1.35, -2.0);
        scene.add(&vase_pedestal_shaft);

        let vase_pedestal_top = Mesh::new(
            Rc::new(cylinder_geometry(0.55, 0.48, 0.15, 32)),
            pedestal_mat.clone(),
        );
        vase_pedestal_top
            .borrow_mut()
            .position
            .set(-5.0, -0.775, -2.0);
        scene.add(&vase_pedestal_top);

        let vase = Mesh::new(Rc::new(lathe_geometry(&vase_profile, 24)), vase_mat);
        {
            let mut object = vase.borrow_mut();
            object.position.set(-5.0, -1.6, -2.0);
            object.scale.set_scalar(1.8);
        }
        scene.add(&vase);

        // armchair

        let wood_mat = standard(0x8b6840, 0.8, 0.0);
        let fabric_mat = standard(0x8b3a3a, 0.9, 0.0);

        let chair_group = Group::new();

        let seat = Mesh::new(
            Rc::new(rounded_box_geometry(0.9, 0.25, 0.8, 4, 0.06)),
            fabric_mat.clone(),
        );
        seat.borrow_mut().position.set(0.0, -1.35, 0.0);
        chair_group.add(&seat);

        let backrest = Mesh::new(
            Rc::new(rounded_box_geometry(0.9, 0.6, 0.12, 4, 0.04)),
            fabric_mat,
        );
        {
            let mut object = backrest.borrow_mut();
            object.position.set(0.0, -0.95, -0.4);
            object.set_rotation(-0.3, 0.0, 0.0);
        }
        chair_group.add(&backrest);

        for side in [-1.0, 1.0] {
            let armrest = Mesh::new(
                Rc::new(box_geometry(0.1, 0.25, 0.7, 1, 1, 1)),
                wood_mat.clone(),
            );
            armrest.borrow_mut().position.set(side * 0.5, -1.2, 0.0);
            chair_group.add(&armrest);

            let arm_top = Mesh::new(
                Rc::new(box_geometry(0.14, 0.06, 0.8, 1, 1, 1)),
                wood_mat.clone(),
            );
            arm_top.borrow_mut().position.set(side * 0.5, -1.07, 0.0);
            chair_group.add(&arm_top);
        }

        let leg_positions = [(-0.38, -0.32), (-0.38, 0.32), (0.38, -0.32), (0.38, 0.32)];
        for (lx, lz) in leg_positions {
            let leg = Mesh::new(
                Rc::new(cylinder_geometry(0.03, 0.035, 0.2, 8)),
                wood_mat.clone(),
            );
            leg.borrow_mut().position.set(lx, -1.575, lz);
            chair_group.add(&leg);
        }

        {
            let mut object = chair_group.borrow_mut();
            object.scale.set_scalar(1.76);
            object.position.set(4.0, 0.948, -2.5);
            object.set_rotation(0.0, -0.6, 0.0);
        }
        scene.add(&chair_group);

        // side table with cup

        let table_group = Group::new();

        let table_top = Mesh::new(
            Rc::new(cylinder_geometry(0.4, 0.4, 0.05, 24)),
            wood_mat.clone(),
        );
        table_top.borrow_mut().position.y = -1.05;
        table_group.add(&table_top);

        let table_leg = Mesh::new(
            Rc::new(cylinder_geometry(0.04, 0.06, 0.9, 8)),
            wood_mat.clone(),
        );
        table_leg.borrow_mut().position.y = -1.5;
        table_group.add(&table_leg);

        let table_base = Mesh::new(Rc::new(cylinder_geometry(0.25, 0.28, 0.06, 24)), wood_mat);
        table_base.borrow_mut().position.y = -1.92;
        table_group.add(&table_base);

        let cup_mat = standard(0xf0ece0, 0.4, 0.05);
        let cup_body = Mesh::new(Rc::new(cylinder_geometry(0.1, 0.08, 0.2, 16)), cup_mat);
        {
            let mut object = cup_body.borrow_mut();
            object.scale.set_scalar(1.0 / 2.2);
            object.position.set(0.15, -0.98, 0.0);
        }
        table_group.add(&cup_body);

        {
            let mut object = table_group.borrow_mut();
            object.scale.set_scalar(2.2);
            object.position.set(2.0, 2.29, -4.0);
        }
        scene.add(&table_group);

        // rug

        let rug_mat = standard(0xc8a0a8, 0.95, 0.0);
        let rug = Mesh::new(Rc::new(box_geometry(6.0, 0.02, 5.0, 1, 1, 1)), rug_mat);
        rug.borrow_mut().position.set(0.0, -1.99, 0.5);
        scene.add(&rug);

        let rug_border_mat = standard(0xd4b880, 0.95, 0.0);
        let rug_border = Mesh::new(
            Rc::new(box_geometry(6.3, 0.015, 5.3, 1, 1, 1)),
            rug_border_mat,
        );
        rug_border.borrow_mut().position.set(0.0, -1.9925, 0.5);
        scene.add(&rug_border);

        // picture frames

        add_frame(&scene, -3.2, 2.0, -4.9, 1.8, 1.2, 0xe8a8a0, 0.0);
        add_frame(&scene, -0.5, 2.6, -4.9, 1.1, 1.6, 0xa0c0e0, 0.0);
        add_frame(&scene, 2.0, 1.8, -4.9, 2.0, 1.4, 0xa0d0a8, 0.0);
        add_frame(&scene, -5.4, 2.2, -3.0, 1.5, 1.1, 0xd0b0d8, PI / 2.0);
        add_frame(&scene, -5.4, 1.8, 1.0, 1.8, 1.3, 0xe0c8a0, PI / 2.0);

        // bust pedestal

        let bust_x = -3.0;
        let bust_z = -3.2;

        let bust_base = Mesh::new(
            Rc::new(cylinder_geometry(0.6, 0.7, 0.15, 32)),
            pedestal_mat.clone(),
        );
        bust_base.borrow_mut().position.set(bust_x, -1.925, bust_z);
        scene.add(&bust_base);

        let bust_shaft = Mesh::new(
            Rc::new(cylinder_geometry(0.35, 0.42, 1.0, 32)),
            pedestal_mat.clone(),
        );
        bust_shaft.borrow_mut().position.set(bust_x, -1.35, bust_z);
        scene.add(&bust_shaft);

        let bust_top = Mesh::new(
            Rc::new(cylinder_geometry(0.55, 0.48, 0.15, 32)),
            pedestal_mat,
        );
        bust_top.borrow_mut().position.set(bust_x, -0.775, bust_z);
        scene.add(&bust_top);

        // Transparent plane for testing

        // `new THREE.MeshStandardNodeMaterial( { transparent: true, opacity:
        // params.transparentOpacity } )`: white, roughness 1, metalness 0.
        let mut transparent_mat = standard(0xffffff, 1.0, 0.0);
        transparent_mat.transparent = true;
        transparent_mat.opacity = TRANSPARENT_OPACITY;
        transparent_mesh = Mesh::new(Rc::new(plane_geometry(1.8, 2.0, 1, 1)), transparent_mat);
        {
            let mut object = transparent_mesh.borrow_mut();
            object.visible = false;
            object.position.set(0.0, 1.2, 1.5);
        }
        scene.add(&transparent_mesh);

        // `updateParameters()` again — the same values `createAO()` set.

        // bust GLB

        // `loader.setDRACOLoader( dracoLoader )`: the port's loader decodes
        // `KHR_draco_mesh_compression` itself.
        let gltf = GltfLoader::load(examples_dir().join("models/gltf/tennyson-bust.glb"))
            .expect("tennyson-bust.glb loads");
        let bust = gltf.scene;
        bust.borrow_mut().set_rotation(0.0, PI, 0.0);

        let target_height = 2.64;
        let mut size_box = Box3::default();
        size_box.set_from_object(&bust, false);
        let size = size_box.get_size();
        bust.borrow_mut().scale.set_scalar(target_height / size.y);

        let mut fit_box = Box3::default();
        fit_box.set_from_object(&bust, false);
        let center = fit_box.get_center();
        bust.borrow_mut().position.set(
            bust_x - center.x,
            -0.7 - fit_box.min.y,
            bust_z - center.z + 0.1,
        );
        scene.add(&bust);
    }

    App {
        renderer,
        scene,
        camera,
        controls,
        pre_pass,
        scene_pass,
        ao_node,
        traa_node,
        transparent_mesh,
        render_pipeline,
    }
}

/// The page's `animate()`: `controls.update()` and one pipeline render.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_ao.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
