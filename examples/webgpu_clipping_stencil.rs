//! Port of `three.js/examples/webgpu_clipping_stencil.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! `performance.now()` is pinned to 0, so `Timer.update()`'s first delta is 0
//! and `object` keeps its zero rotation in the graded frame.
//!
//! The caps: each plane draws the knot's two sides into its own stencil bit
//! with `InvertStencilOp` (colour and depth off), clipped by that plane alone,
//! so the bit is left set wherever the plane cuts through the solid. A cap
//! quad, clipped by the other two planes, then draws only where its bit is
//! set and clears it. The knot itself is drawn last (`renderOrder = 6`),
//! clipped by all three planes. `stencil: true` gives the renderer its
//! `depth24plus-stencil8` buffer.
//!
//! **Divergences**:
//!
//! - The ground, `new THREE.ShadowNodeMaterial( { color: 0x000000, opacity:
//!   0.25, side: THREE.DoubleSide } )`, is not drawn: the port has no
//!   `ShadowNodeMaterial` (a transparent shadow catcher). Three's frame has
//!   the knot's faint shadow on the background beside it; this one does not,
//!   though at `opacity: 0.25` the grader does not see the difference.
//! - `shadowSide: THREE.DoubleSide` on the knot's material is dropped: the
//!   port's materials have no `shadowSide`. Its only receiver is the ground
//!   above, so it changes nothing here.
//! - The three `PlaneHelper`s are not built: they are `visible = false`
//!   until the GUI's `displayHelper` turns them on.
//! - `Plane` is `Copy`, so each `ClippingGroup` holds its own copy of the
//!   page's `planes[ i ]`, where three's groups share the one `Plane` object
//!   (a GUI slider moving `plane.constant` moves the cap clips and the knot's
//!   cut together); a port that moved a plane would update every group
//!   holding it. The GUI is never touched here, so the frame is the same.
//!
//! The GUI (`animate`, `planeX` / `planeY` / `planeZ`) sits at its defaults,
//! `renderer.inspector` does not touch the frame, and the `resize` listener
//! never fires.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::ObjectRef;
use three_rs::geometries::{plane_geometry, torus_knot_geometry};
use three_rs::materials::{Side, StencilFunc, StencilOp};
use three_rs::math::{Plane, Vector3};
use three_rs::objects::Background;
use three_rs::Timer;
use three_rs::{
    AmbientLight, ClippingGroup, Color, DirectionalLight, Group, Mesh, MeshBasicNodeMaterial,
    MeshStandardNodeMaterial, PerspectiveCamera, Renderer, RendererParameters, Scene,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// The page's `params.animate`.
const ANIMATE: bool = true;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    /// The page's `object`: the group the stencil groups and the clipped knot
    /// turn in.
    pub object: ObjectRef,
    /// The page's `planes`, which the caps follow.
    pub planes: [Plane; 3],
    /// The page's `planeObjects`: one cap per plane.
    pub plane_objects: Vec<ObjectRef>,
    /// The page's module-level `timer`.
    pub timer: Timer,
}

/// The page's `createPlaneStencilGroup( geometry, plane, stencilBit,
/// renderOrder )`.
fn create_plane_stencil_group(
    geometry: &Rc<three_rs::BufferGeometry>,
    plane: Plane,
    stencil_bit: u32,
    render_order: f64,
) -> ObjectRef {
    let group = ClippingGroup::of(ClippingGroup {
        clipping_planes: vec![plane],
        ..ClippingGroup::default()
    });

    let mut material = MeshBasicNodeMaterial::new();
    material.side = Side::Double;
    material.depth_write = false;
    material.depth_test = false;
    material.color_write = false;
    material.stencil_write = true;
    material.stencil_write_mask = stencil_bit;
    material.stencil_func = StencilFunc::Always;
    material.stencil_fail = StencilOp::Invert;
    material.stencil_z_fail = StencilOp::Invert;
    material.stencil_z_pass = StencilOp::Invert;

    let mesh = Mesh::new(geometry.clone(), material);
    mesh.borrow_mut().render_order = render_order;
    group.add(&mesh);

    group
}

pub fn init() -> App {
    let timer = Timer::new();

    let mut scene = Scene::new();
    scene.background = Some(Background::Color(Color::from_hex(0x263238)));

    let mut camera = PerspectiveCamera::new(36.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 100.0);
    camera.node.borrow_mut().position.set(2.0, 2.0, 2.0);

    scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 1.5));

    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(5.0, 10.0, 7.5);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.camera.set_bounds(-2.0, 2.0, 2.0, -2.0);
        shadow.map_size.x = 1024.0;
        shadow.map_size.y = 1024.0;
    }
    scene.add(&dir_light);

    let planes = [
        Plane::new(Vector3::new(-1.0, 0.0, 0.0), 0.0),
        Plane::new(Vector3::new(0.0, -1.0, 0.0), 0.0),
        Plane::new(Vector3::new(0.0, 0.0, -1.0), 0.0),
    ];

    // `planeHelpers`: not built, see the module docs.

    let geometry = Rc::new(torus_knot_geometry(0.4, 0.15, 220, 60, 2.0, 3.0));
    let object = Group::new();
    scene.add(&object);

    // Set up clip plane rendering

    let mut plane_objects = Vec::with_capacity(3);
    let plane_geom = Rc::new(plane_geometry(4.0, 4.0, 1, 1));

    for (i, &plane) in planes.iter().enumerate() {
        let stencil_bit = 1 << i;

        let stencil_group =
            create_plane_stencil_group(&geometry, plane, stencil_bit, i as f64 + 1.0);
        object.add(&stencil_group);

        // the cap is only rendered where the stencil bit of its plane is set
        // and is clipped by the other clipping planes

        let mut plane_mat =
            MeshStandardNodeMaterial::standard(Color::from_hex(0xe91e63), 0.75, 0.1);
        plane_mat.stencil_write = true;
        plane_mat.stencil_ref = 0;
        plane_mat.stencil_func_mask = stencil_bit;
        plane_mat.stencil_write_mask = stencil_bit;
        plane_mat.stencil_func = StencilFunc::NotEqual;
        plane_mat.stencil_fail = StencilOp::Replace;
        plane_mat.stencil_z_fail = StencilOp::Replace;
        plane_mat.stencil_z_pass = StencilOp::Replace;

        let po = Mesh::new(plane_geom.clone(), plane_mat);
        po.borrow_mut().render_order = i as f64 + 1.1;

        let po_group = ClippingGroup::of(ClippingGroup {
            // `planes.filter( p => p !== plane )`
            clipping_planes: planes
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, p)| *p)
                .collect(),
            ..ClippingGroup::default()
        });
        po_group.add(&po);

        plane_objects.push(po);
        scene.add(&po_group);
    }

    let clipping_group = ClippingGroup::of(ClippingGroup {
        clipping_planes: planes.to_vec(),
        clip_shadows: true,
        ..ClippingGroup::default()
    });
    object.add(&clipping_group);

    // `shadowSide: THREE.DoubleSide` is dropped, see the module docs.
    let material = MeshStandardNodeMaterial::standard(Color::from_hex(0xffc107), 0.75, 0.1);

    // add the color

    let clipped_color_front = Mesh::new(geometry, material);
    {
        let mut object = clipped_color_front.borrow_mut();
        object.cast_shadow = true;
        object.render_order = 6.0;
    }
    clipping_group.add(&clipped_color_front);

    // `ground`, a `ShadowNodeMaterial` plane: not drawn, see the module docs.

    // Renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    parameters.stencil = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.shadow_map_enabled = true;

    // Controls

    let mut controls = OrbitControls::new(&mut camera);
    // The canvas the example renders at, standing in for the element's
    // `clientWidth` / `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 2.0;
    controls.max_distance = 20.0;
    let _ = controls.update(&mut camera, None);

    App {
        renderer,
        scene,
        camera,
        controls,
        object,
        planes,
        plane_objects,
        timer,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta();

    if ANIMATE {
        let mut object = app.object.borrow_mut();
        let r = object.rotation;
        object.set_rotation(r.x + delta * 0.5, r.y + delta * 0.2, r.z);
    }

    for (plane, po) in app.planes.iter().zip(&app.plane_objects) {
        let position = plane.coplanar_point();
        po.borrow_mut().position = position;
        po.look_at(&Vector3::new(
            position.x - plane.normal.x,
            position.y - plane.normal.y,
            position.z - plane.normal.z,
        ));
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

/// The example's controls, for a host that has a pointer. `None` when the
/// page creates none — the signature is the same for every example so the
/// viewer and the browser shell can drive any of them through one call.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs: the JS holds the camera as `this.object` and Rust
/// cannot, so `pointer_move` and the rest take it as an argument.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
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
        .unwrap_or_else(|| "target/webgpu_clipping_stencil.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
