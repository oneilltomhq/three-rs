//! Port of `three.js/examples/webgpu_lines_fat_raycasting.html`, calling the
//! three-rs API in the same order the page's `init()` / `onWindowResize()` /
//! `animate()` do.
//!
//! Under the e2e harness the viewport is 800 x 500 with a device pixel ratio
//! of 1, and `performance.now()` is pinned at 0.
//!
//! # What the graded frame is testing
//!
//! **The world-units fat line.** 300 `CatmullRomCurve3` samples, taken in
//! pairs by `LineSegmentsGeometry.setPositions()` — 150 separate segments, one
//! world unit wide, drawn by `Line2NodeMaterial`'s `worldUnits` branch: a
//! view-space box per segment and a fragment stage that measures the distance
//! from each pixel's view ray to the segment (`closestLineToLine`) and turns it
//! into coverage with `fwidth`. `alphaToCoverage` is on, so that coverage goes
//! to the multisample mask rather than a `discard`. See
//! [`three_rs::materials::line2`].
//!
//! **The raycast.** `animate()` casts a ray from `pointer` every frame. The
//! pointer starts at `( Infinity, Infinity )` and the harness never moves it,
//! so `setFromCamera()` makes a NaN ray; `LineSegments2.raycast()` then fails
//! its bounding-sphere test (every comparison with NaN is false) and the two
//! marker spheres stay hidden. The call is still made, as on the page.
//!
//! # Not ported, deliberately
//!
//! * `computeLineDistances()` — the material has `dashed: false` and nothing
//!   reads `instanceDistanceStart`.
//! * The `Inspector` panel and its GUI; `trackTimestamp`.
//! * `timer.connect( document )`; see [`three_rs::Timer`].

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::lines::{Line2, LineGeometry, LineSegments2, LineSegmentsGeometry};
use three_rs::core::Node;
use three_rs::geometries::sphere_geometry;
use three_rs::math::ColorSpace;
use three_rs::{
    CatmullRomCurve3, Color, Curve, Line2NodeMaterial, Mesh, MeshBasicNodeMaterial,
    PerspectiveCamera, Raycaster, Renderer, RendererParameters, Scene, Timer, Vector2, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
    pub timer: Timer,
    pub raycaster: Raycaster,
    /// `new THREE.Vector2( Infinity, Infinity )`, moved by `onPointerMove`.
    pub pointer: Vector2,
    pub line: Node,
    pub threshold_line: Node,
    pub segments: Node,
    pub threshold_segments: Node,
    pub sphere_inter: Node,
    pub sphere_on_line: Node,
    /// `params.animate`.
    pub animate: bool,
}

pub fn init() -> App {
    // `const raycaster = new THREE.Raycaster(); raycaster.params.Line2 = {
    // threshold: 0 }`.
    let mut raycaster = Raycaster::default();
    raycaster.params.line2.threshold = 0.0;

    // `matLine`: `linewidth: 1` — "in world units with size attenuation,
    // pixels otherwise".
    let mut mat_line = Line2NodeMaterial::line2(Color::from_hex(0xffffff));
    mat_line.linewidth = 1.0;
    mat_line.world_units = true;
    mat_line.vertex_colors = true;
    mat_line.alpha_to_coverage = true;

    // `matThresholdLine`: the translucent halo the GUI's "visualize threshold"
    // shows; `visible: false` keeps it out of the render list.
    let mut mat_threshold_line = Line2NodeMaterial::line2(Color::from_hex(0xffffff));
    mat_threshold_line.linewidth = mat_line.linewidth;
    mat_threshold_line.world_units = true;
    mat_threshold_line.transparent = true;
    mat_threshold_line.opacity = 0.2;
    mat_threshold_line.depth_test = false;
    mat_threshold_line.visible = false;

    let timer = Timer::new();

    // `new THREE.WebGPURenderer( { antialias: true, alpha: true } )` — four
    // samples, which is also what lets `alphaToCoverage` reach the pipeline.
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_clear_color(Color::from_hex(0x000000), 1.0);

    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 1.0, 1000.0);
    camera.node.borrow_mut().position.set(-40.0, 0.0, 60.0);

    // The constructor's own `update()` aims the camera at the default
    // `( 0, 0, 0 )` target; `animate()` never calls `controls.update()`.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.min_distance = 10.0;
    controls.max_distance = 500.0;

    let sphere_geometry = Rc::new(sphere_geometry(0.25, 8, 4));
    let mut sphere_inter_material = MeshBasicNodeMaterial::new();
    sphere_inter_material.color = Color::from_hex(0xff0000);
    sphere_inter_material.depth_test = false;
    let mut sphere_on_line_material = MeshBasicNodeMaterial::new();
    sphere_on_line_material.color = Color::from_hex(0x00ff00);
    sphere_on_line_material.depth_test = false;

    let sphere_inter = Mesh::new(sphere_geometry.clone(), sphere_inter_material);
    let sphere_on_line = Mesh::new(sphere_geometry, sphere_on_line_material);
    for sphere in [&sphere_inter, &sphere_on_line] {
        let mut object = sphere.borrow_mut();
        object.visible = false;
        object.render_order = 10.0;
    }
    scene.add(&sphere_inter);
    scene.add(&sphere_on_line);

    // Position and THREE.Color Data
    let mut points = Vec::with_capacity(100);
    for i in -50..50 {
        let t = i as f64 / 3.0;
        points.push(Vector3::new(t * (2.0 * t).sin(), t, t * (2.0 * t).cos()));
    }

    let spline = CatmullRomCurve3::new(points.clone());
    let divisions = (3.0 * points.len() as f64).round() as usize;
    let mut positions: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut colors: Vec<f32> = Vec::with_capacity(divisions * 3);
    let mut color = Color::default();
    for i in 0..divisions {
        let t = i as f64 / divisions as f64;

        let point = spline.get_point(t);
        positions.extend([point.x as f32, point.y as f32, point.z as f32]);

        color.set_hsl(t, 1.0, 0.5, ColorSpace::SRGB);
        colors.extend([color.r as f32, color.g as f32, color.b as f32]);
    }

    let mut line_geometry = LineGeometry::new();
    line_geometry.set_positions(&positions);
    line_geometry.set_colors(&colors);

    let mut segments_geometry = LineSegmentsGeometry::new();
    segments_geometry.set_positions(positions.clone());
    segments_geometry.set_colors(colors.clone());

    // `segments.computeLineDistances()` and `.scale.set( 1, 1, 1 )` are
    // skipped for all four: no dashes, and the identity.
    let segments = LineSegments2::new(&segments_geometry, mat_line.clone());
    scene.add(&segments);

    let threshold_segments = LineSegments2::new(&segments_geometry, mat_threshold_line.clone());
    scene.add(&threshold_segments);

    let line = Line2::new(&line_geometry, mat_line);
    scene.add(&line);

    let threshold_line = Line2::new(&line_geometry, mat_threshold_line);
    scene.add(&threshold_line);

    // The page also builds a plain `BufferGeometry` (`geo`) from the same
    // arrays and never uses it.

    let mut app = App {
        renderer,
        scene,
        camera,
        controls,
        timer,
        raycaster,
        pointer: Vector2::new(f64::INFINITY, f64::INFINITY),
        line,
        threshold_line,
        segments,
        threshold_segments,
        sphere_inter,
        sphere_on_line,
        animate: true,
    };

    // `switchLine( params[ 'line type' ] )` — 1, the segments.
    switch_line(&mut app, 1);

    // `onWindowResize()`.
    resize(&mut app, INNER_WIDTH, INNER_HEIGHT);

    app
}

/// The page's `switchLine( val )`: 0 shows the `Line2`, 1 the
/// `LineSegments2`, each with its threshold twin.
pub fn switch_line(app: &mut App, val: u32) {
    let show_line = val == 0;
    app.line.borrow_mut().visible = show_line;
    app.threshold_line.borrow_mut().visible = show_line;
    app.segments.borrow_mut().visible = !show_line;
    app.threshold_segments.borrow_mut().visible = !show_line;
}

/// The page's `animate()`, run once by the harness's single RAF.
pub fn animate(app: &mut App) {
    app.timer.update();

    let delta = app.timer.get_delta();

    let obj = if app.line.borrow().visible {
        app.line.clone()
    } else {
        app.segments.clone()
    };
    copy_placement(&app.line, &app.threshold_line);
    copy_placement(&app.segments, &app.threshold_segments);

    if app.animate {
        let rotation_y = app.line.borrow().rotation.y + delta * 0.1;
        {
            let mut line = app.line.borrow_mut();
            let rotation = line.rotation;
            line.set_rotation(rotation.x, rotation_y, rotation.z);
        }
        let mut segments = app.segments.borrow_mut();
        let rotation = segments.rotation;
        segments.set_rotation(rotation.x, rotation_y, rotation.z);
    }

    // `raycaster.setFromCamera( pointer, camera )` reads the camera's world
    // matrices as the previous frame's `render()` left them — on the first
    // frame, as the constructor left them — exactly as the page does.
    app.raycaster.set_from_camera(&app.pointer, &app.camera);

    let intersects = app.raycaster.intersect_object(&obj, true);

    if let Some(hit) = intersects.first() {
        app.sphere_inter.borrow_mut().visible = true;
        app.sphere_on_line.borrow_mut().visible = true;

        app.sphere_inter.borrow_mut().position = hit.point;
        if let Some(point_on_line) = hit.point_on_line {
            app.sphere_on_line.borrow_mut().position = point_on_line;
        }

        // `color.fromBufferAttribute( instanceColorStart, faceIndex )`.
        let mut color = Color::default();
        if let (Some(index), Some(colors)) = (
            hit.face_index,
            obj.borrow()
                .payload
                .line_segments()
                .and_then(|segments| segments.colors.clone()),
        ) {
            let start = &colors[index * 6..index * 6 + 3];
            color = Color::new(start[0] as f64, start[1] as f64, start[2] as f64);
        }

        set_sphere_color(&app.sphere_inter, &color, 0.3);
        set_sphere_color(&app.sphere_on_line, &color, 0.7);
    } else {
        app.sphere_inter.borrow_mut().visible = false;
        app.sphere_on_line.borrow_mut().visible = false;
    }

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// `threshold.position.copy( line.position ); threshold.quaternion.copy(
/// line.quaternion )`.
fn copy_placement(from: &Node, to: &Node) {
    let (position, quaternion) = {
        let from = from.borrow();
        (from.position, from.quaternion)
    };
    to.borrow_mut().position = position;
    to.borrow_mut().set_rotation_from_quaternion(&quaternion);
}

/// `sphere.material.color.copy( color ).offsetHSL( h, 0, 0 )`.
fn set_sphere_color(sphere: &Node, color: &Color, h: f64) {
    let mut sphere = sphere.borrow_mut();
    if let Some(material) = sphere.mesh_mut().and_then(|mesh| mesh.material.as_mut()) {
        material.color.copy(color).offset_hsl(h, 0.0, 0.0);
    }
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The page's `onPointerMove()`, for a host with a pointer: client
/// coordinates to normalised device coordinates.
pub fn pointer_move(app: &mut App, client_x: f64, client_y: f64, width: f64, height: f64) {
    app.pointer.x = (client_x / width) * 2.0 - 1.0;
    app.pointer.y = -(client_y / height) * 2.0 + 1.0;
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once; see `webgpu_lines_fat.rs`.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
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
        .unwrap_or_else(|| "target/webgpu_lines_fat_raycasting.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
