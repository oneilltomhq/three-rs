//! Port of `three.js/examples/webgpu_materials_arrays.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Sixteen "paper model" polyhedra, each one `Mesh( geometry, materials )`
//! with an *array* of six materials: `makeHoleyGeometry()` rebuilds every
//! face as a ring of quads around a hole and adds one `geometry.groups` entry
//! per face with `materialIndex = face % 6`, so a single mesh draws once per
//! face, each draw with its own colour. That is the multi-material path of
//! `Renderer._projectObject()` and `RenderObject.getDrawParameters()` — see
//! `docs/nodes.md` §51.
//!
//! Under the e2e harness the viewport is 800 x 500 at `devicePixelRatio` 1.
//! The page never calls `Math.random()`.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::BufferAttribute;
use three_rs::geometries::{
    box_geometry, dodecahedron_geometry, icosahedron_geometry, octahedron_geometry,
    tetrahedron_geometry,
};
use three_rs::materials::Side;
use three_rs::math::Box3;
use three_rs::{
    BufferGeometry, Color, DirectionalLight, HemisphereLight, Mesh, MeshStandardNodeMaterial,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Vector3,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// `const palette`, one `MeshStandardMaterial` per paper colour.
const PALETTE: [u32; 6] = [0xe4002b, 0xff7f11, 0xffd100, 0x00a651, 0x0072ce, 0x8a2be2];

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
}

pub fn init() -> App {
    // renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.set_pixel_ratio(DPR);
    renderer.shadow_map_enabled = true;

    // scene

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x222222));

    // camera

    let mut camera = PerspectiveCamera::new(40.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(-6.0, 7.0, 14.0);

    // controls

    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.target.set(0.0, 0.5, 0.0);
    controls.min_distance = 5.0;
    controls.max_distance = 50.0;
    controls.enable_damping = true;
    controls.update(&mut camera, None);

    // environment

    let hemi_light =
        HemisphereLight::new(Color::from_hex(0xffffff), Color::from_hex(0x0e696c), 1.0);
    scene.add(&hemi_light);

    // lights

    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 6.0);
    {
        let mut object = dir_light.borrow_mut();
        object.position.set(5.0, 10.0, 6.0);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        // `shadow.camera.left / right / top / bottom = ∓10`, `far = 20`.
        shadow.camera.set_bounds(-10.0, 10.0, 10.0, -10.0);
        shadow.camera.set_far(20.0);
        shadow.map_size.x = 2048.0;
        shadow.map_size.y = 2048.0;
        shadow.radius = 10.0;
    }
    scene.add(&dir_light);

    // materials, one per paper color, shared across all solids

    let materials: Vec<MeshStandardNodeMaterial> = PALETTE
        .iter()
        .map(|&color| {
            // `new THREE.MeshStandardMaterial( { color, roughness: 0.8, side:
            // THREE.DoubleSide } )` — `metalness` stays 0.
            let mut material = MeshStandardNodeMaterial::standard(Color::from_hex(color), 0.8, 0.0);
            material.side = Side::Double;
            material
        })
        .collect();

    let geometries = [
        make_holey_geometry(tetrahedron_geometry(1.2, 0), 3, materials.len()), // 4 faces, 1 triangle each
        make_holey_geometry(box_geometry(1.5, 1.5, 1.5, 1, 1, 1), 6, materials.len()), // 6 faces, 2 triangles each (indexed)
        make_holey_geometry(octahedron_geometry(1.2, 0), 3, materials.len()), // 8 faces, 1 triangle each
        make_holey_geometry(dodecahedron_geometry(1.1, 0), 9, materials.len()), // 12 faces, 3 triangles each
        make_holey_geometry(icosahedron_geometry(1.1, 0), 3, materials.len()), // 20 faces, 1 triangle each
    ]
    .map(Rc::new);

    // table

    let table = Mesh::new(
        Rc::new(box_geometry(17.0, 0.5, 17.0, 1, 1, 1)),
        MeshStandardNodeMaterial::standard(Color::from_hex(0x0e696c), 0.5, 0.0),
    );
    {
        let mut object = table.borrow_mut();
        object.position.y = -0.25;
        object.receive_shadow = true;
    }
    scene.add(&table);

    // grid

    const ORDER: [usize; 16] = [0, 1, 2, 3, 3, 4, 0, 1, 1, 2, 3, 4, 4, 0, 1, 2];

    for (i, &order) in ORDER.iter().enumerate() {
        let row = (i / 4) as f64;
        let col = (i % 4) as f64;

        let mesh = Mesh::with_materials(geometries[order].clone(), materials.clone());
        {
            let mut object = mesh.borrow_mut();
            object.cast_shadow = true;

            object.scale.set_scalar(0.5 + row * 0.25);
            object
                .position
                .set((col - 1.5) * 3.5, 0.0, (1.5 - row) * 3.5);
        }

        // `mesh.position.y = - new THREE.Box3().setFromObject( mesh, true
        // ).min.y - 0.01` — `setFromObject` updates the mesh's world matrix
        // first, with the y still 0.
        let min_y = Box3::default().set_from_object(&mesh, true).min.y;
        mesh.borrow_mut().position.y = -min_y - 0.01;

        scene.add(&mesh);
    }

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// `v.toArray().join( ',' )` as a key: two corners are the same when their
/// three numbers print the same, and `String( -0 )` is `"0"`, so a negative
/// zero is folded into a positive one first.
fn corner_key(v: &Vector3) -> [u64; 3] {
    [v.x, v.y, v.z].map(|c| (c + 0.0).to_bits())
}

/// `makeHoleyGeometry( geometry, verticesPerFace, holeScale = 0.6 )` —
/// rebuilds a polyhedron with a hole cut into each polygon face, like an open
/// paper model. `material_count` is the page's `materials.length`, which the
/// function reads from module scope.
fn make_holey_geometry(
    geometry: BufferGeometry,
    vertices_per_face: usize,
    material_count: usize,
) -> BufferGeometry {
    let hole_scale = 0.6;

    let geometry = if geometry.index.is_some() {
        geometry.to_non_indexed()
    } else {
        geometry
    };

    let position = geometry
        .get_attribute("position")
        .expect("a polyhedron has a position attribute");

    let mut positions: Vec<f32> = Vec::new();
    let mut holey = BufferGeometry::new();

    for face in 0..position.count() / vertices_per_face {
        // collect the unique corner vertices of the face

        let mut corners: Vec<Vector3> = Vec::new();
        let mut keys: Vec<[u64; 3]> = Vec::new();

        for i in 0..vertices_per_face {
            let index = face * vertices_per_face + i;
            let v = Vector3::new(
                position.get_x(index),
                position.get_y(index),
                position.get_z(index),
            );
            let key = corner_key(&v);

            if !keys.contains(&key) {
                keys.push(key);
                corners.push(v);
            }
        }

        // sort the corners counterclockwise around the face centroid

        let mut centroid = Vector3::new(0.0, 0.0, 0.0);
        for v in &corners {
            centroid.add(v);
        }
        centroid.divide_scalar(corners.len() as f64);

        let mut a = Vector3::default();
        let mut b = Vector3::default();
        a.sub_vectors(&corners[1], &corners[0]);
        b.sub_vectors(&corners[2], &corners[0]);
        let mut normal = Vector3::default();
        normal.cross_vectors(&a, &b).normalize();
        let mut tangent = Vector3::default();
        tangent.sub_vectors(&corners[0], &centroid).normalize();
        let mut bitangent = Vector3::default();
        bitangent.cross_vectors(&normal, &tangent);

        let angle_of = |v: &Vector3| {
            let mut b = Vector3::default();
            b.sub_vectors(v, &centroid);
            b.dot(&bitangent).atan2(b.dot(&tangent))
        };
        // `Array.prototype.sort` with a numeric comparator is stable, as
        // `sort_by` is.
        corners.sort_by(|p, q| {
            (angle_of(p) - angle_of(q))
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // bridge each outer edge with its scaled-down inner edge

        let inner: Vec<Vector3> = corners
            .iter()
            .map(|v| {
                let mut w = Vector3::default();
                w.lerp_vectors(&centroid, v, hole_scale);
                w
            })
            .collect();

        holey.add_group(
            positions.len() / 3,
            corners.len() * 6,
            face % material_count,
        );

        for i in 0..corners.len() {
            let j = (i + 1) % corners.len();

            for v in [
                &corners[i],
                &corners[j],
                &inner[j],
                &corners[i],
                &inner[j],
                &inner[i],
            ] {
                positions.extend([v.x as f32, v.y as f32, v.z as f32]);
            }
        }
    }

    holey.set_attribute("position", BufferAttribute::new(positions, 3));
    holey.compute_vertex_normals();

    holey
}

/// The page's `animate()`: `controls.update()` then `renderer.render()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera, None);

    app.renderer.render(&mut app.scene, &mut app.camera);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();

    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
pub fn controls_and_camera(app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    Some((&mut app.controls, &mut app.camera))
}

fn main() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = init();
    println!("adapter: {:?}", app.renderer.adapter_info());
    animate(&mut app);

    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/webgpu_materials_arrays.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
