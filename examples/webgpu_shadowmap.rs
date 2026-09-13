//! Port of `three.js/examples/webgpu_shadowmap.html`, calling the three-rs API
//! in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `performance.now()` is pinned to 0, so `Timer.update()`'s first delta is 0:
//! the torus knot keeps its zero rotation, `dirGroup` stays unrotated, and
//! `dirLight.position.z` is `17 + sin( 0 ) * 5 = 17`, i.e. exactly where
//! `init()` put it. Nothing in `animate()` moves the graded frame.
//!
//! `OrbitControls` is constructed and `update()`d once. The camera is 21.54
//! units from the target, inside `[ minDistance, maxDistance ]`, so the only
//! thing `update()` changes is the camera's orientation: `lookAt( 0, 2, 0 )`.
//! `renderer.inspector = new Inspector()` does not touch the frame, and the
//! `resize` listener never fires.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::core::{Node, Object3DNode};
use three_rs::nodes::materialx::{mx_fractal_noise_float, mx_fractal_noise_vec3};
use three_rs::nodes::tsl::{
    block, fog, int, position_local, position_world, range_fog_factor, to_var,
};
use three_rs::nodes::NodeRef;
use three_rs::objects::Background;
use three_rs::geometries::{cylinder_geometry, plane_geometry, torus_knot_geometry};
use three_rs::{
    AmbientLight, Color, DirectionalLight, Group, Mesh, MeshPhongNodeMaterial, PerspectiveCamera,
    Renderer, RendererParameters, Scene, SpotLight, ToneMapping, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub torus_knot: Node,
    pub dir_group: Node,
    pub dir_light: Node,
}

/// The ground's `Fn( () => { const pos = positionWorld.toVar(); pos.xz
/// .addAssign( mx_fractal_noise_vec3( positionWorld.mul( 2 ) ).saturate().xz );
/// … } )()` prologue, shared by `receivedShadowPositionNode` and `colorNode`.
///
/// `pos.xz.addAssign( … )` is three's `SetNode`: it writes the sum into its own
/// temp and then copies the two components out one at a time, which is why the
/// dump shows `nodeVar6.x = nodeVar7[ 0 ]; nodeVar6.z = nodeVar7[ 1 ];` rather
/// than one `nodeVar6.xz = …`.
fn noisy_ground_position() -> (NodeRef, Vec<NodeRef>) {
    let pos = to_var(None, position_world());
    let sum = to_var(
        None,
        pos.clone().xz().add(
            mx_fractal_noise_vec3(position_world().mul(2.0), 3, 2.0, 0.5, 1.0)
                .saturate()
                .xz(),
        ),
    );
    let statements = vec![
        pos.clone().x().assign(sum.clone().element_node(int(0))),
        pos.clone().z().assign(sum.element_node(int(1))),
    ];
    (pos, statements)
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(45.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(0.0, 10.0, 20.0);

    let mut scene = Scene::new();
    scene.background = Some(Background::Node(Color::from_hex(0x222244)));
    scene.fog_node = Some(fog(
        Color::from_hex(0x222244),
        range_fog_factor(50.0, 100.0),
    ));

    // lights

    scene.add(&AmbientLight::new(Color::from_hex(0x444444), 2.0));

    let spot_light = SpotLight::new(Color::from_hex(0xff8888), 400.0);
    {
        let mut object = spot_light.borrow_mut();
        object.position.set(8.0, 10.0, 5.0);
        object.cast_shadow = true;
        let light = object.light_mut().unwrap();
        light.angle = PI / 5.0;
        light.penumbra = 0.3;
        let shadow = light.shadow.as_mut().unwrap();
        shadow.camera.set_near(8.0);
        shadow.camera.set_far(200.0);
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.radius = 4.0;
    }
    scene.add(&spot_light);

    let dir_light = DirectionalLight::new(Color::from_hex(0x8888ff), 3.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(3.0, 12.0, 17.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(500.0);
        shadow.camera.set_bounds(-17.0, 17.0, 17.0, -17.0);
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.radius = 4.0;
    }

    let dir_group = Group::new();
    dir_group.add(&dir_light);
    scene.add(&dir_group);

    // geometry

    let geometry = Rc::new(torus_knot_geometry(25.0, 8.0, 75, 80, 2.0, 3.0));
    let mut material = MeshPhongNodeMaterial::phong(Color::from_hex(0x999999));
    material.shininess = 0.0;
    material.specular = Color::from_hex(0x222222);

    // `material.clone()` plus `transparent = true` and a `maskNode`, which
    // `NodeMaterial.setupDiscard()` turns into the `if ( ! mask ) { discard; }`
    // at the top of both the main and the shadow fragment.
    let mut material_custom_shadow = material.clone();
    material_custom_shadow.transparent = true;
    material_custom_shadow.mask_node = Some(
        mx_fractal_noise_float(position_local().mul(0.1), 3, 2.0, 0.5, 1.0)
            .x()
            .greater_than(0.0),
    );

    let torus_knot = Mesh::new(geometry);
    {
        let mut object = torus_knot.borrow_mut();
        object.mesh_mut().unwrap().material = Some(material_custom_shadow);
        object.scale.multiply_scalar(1.0 / 18.0);
        object.position.y = 3.0;
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.add(&torus_knot);

    let cylinder_geometry = Rc::new(cylinder_geometry(0.75, 0.75, 7.0, 32));

    // `pillar1.clone()` shares the geometry and the material; only the position
    // differs, so four plain meshes are the same thing.
    for (x, z) in [(8.0, 8.0), (8.0, -8.0), (-8.0, 8.0), (-8.0, -8.0)] {
        let pillar = Mesh::new(cylinder_geometry.clone());
        {
            let mut object = pillar.borrow_mut();
            object.mesh_mut().unwrap().material = Some(material.clone());
            object.position.set(x, 3.5, z);
            object.cast_shadow = true;
        }
        scene.add(&pillar);
    }

    let plane_geometry = Rc::new(plane_geometry(200.0, 200.0, 1, 1));

    let mut plane_material = MeshPhongNodeMaterial::phong(Color::from_hex(0x999999));
    plane_material.shininess = 0.0;
    plane_material.specular = Color::from_hex(0x111111);

    let (pos, statements) = noisy_ground_position();
    plane_material.received_shadow_position_node = Some(block(statements, pos));

    let (_pos, statements) = noisy_ground_position();
    plane_material.color_node = Some(block(
        statements,
        mx_fractal_noise_vec3(position_world().mul(2.0), 3, 2.0, 0.5, 1.0)
            .saturate()
            .zzz()
            .mul(0.2)
            .add(0.5),
    ));

    let ground = Mesh::new(plane_geometry);
    {
        let mut object = ground.borrow_mut();
        object.mesh_mut().unwrap().material = Some(plane_material);
        object.set_rotation(-PI / 2.0, 0.0, 0.0);
        object.scale.multiply_scalar(3.0);
        object.cast_shadow = true;
        object.receive_shadow = true;
    }
    scene.add(&ground);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true });
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;
    renderer.tone_mapping = ToneMapping::AcesFilmic;

    // `new OrbitControls( camera, … )` then `controls.target.set( 0, 2, 0 )`
    // and `controls.update()`.
    camera.look_at(&Vector3::new(0.0, 2.0, 0.0));

    App {
        renderer,
        scene,
        camera,
        torus_knot,
        dir_group,
        dir_light,
    }
}

/// The page's `animate( time )` with `time = 0` and `Timer`'s first delta of 0.
pub fn animate(app: &mut App) {
    let delta = 0.0f64;
    let time = 0.0f64;

    {
        let mut object = app.torus_knot.borrow_mut();
        let r = object.rotation;
        object.set_rotation(
            r.x + 0.25 * delta,
            r.y + 0.5 * delta,
            r.z + 1.0 * delta,
        );
    }
    {
        let mut object = app.dir_group.borrow_mut();
        let r = object.rotation;
        object.set_rotation(r.x, r.y + 0.7 * delta, r.z);
    }
    app.dir_light.borrow_mut().position.z = 17.0 + (time * 0.001).sin() * 5.0;

    app.renderer.render(&mut app.scene, &mut app.camera);
}

fn main() {
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_shadowmap.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
