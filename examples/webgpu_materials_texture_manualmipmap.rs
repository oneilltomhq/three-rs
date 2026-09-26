//! Port of `three.js/examples/webgpu_materials_texture_manualmipmap.html`,
//! calling the three-rs API in the same order the page's `init()` /
//! `animate()` do.
//!
//! Two scenes side by side through the scissor, each under `new THREE.Fog(
//! 0x000000, 1500, 4000 )`: a floor whose texture carries eight
//! **hand-painted mip levels** — one colour per level, so the level the GPU
//! picks is visible — and a painting on a frame with a translucent shadow. The
//! left half samples with the defaults (`LinearMipmapLinear`, linear), the
//! right half with `NearestMipmapNearest` / `Nearest` and a `0xffccaa` tint.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and the mouse never moves.
//!
//! The page's `init()` is `async` and awaits the painting before it creates
//! the renderer, so the graded frame is the first `animate()` after
//! everything is in place; here the load is synchronous.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::math::Vector3;
use three_rs::textures::Image;
use three_rs::{
    plane_geometry, Color, ColorSpace, Fog, Mesh, MeshBasicNodeMaterial, MinFilter,
    PerspectiveCamera, Renderer, RendererParameters, Scene, Texture, TextureFilter, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene1: Scene,
    pub scene2: Scene,
    pub camera: PerspectiveCamera,
    /// `mouseX` / `mouseY` — `onDocumentMouseMove()`'s offsets from the
    /// window centre. The harness never moves the mouse, so both stay 0.
    pub mouse_x: f64,
    pub mouse_y: f64,
    /// `SCREEN_WIDTH` / `SCREEN_HEIGHT`: the page reads `window.innerWidth`
    /// once, at load, and never updates them — there is no resize handler.
    screen_width: f64,
    screen_height: f64,
}

/// The page's `mipmap( size, color )`: a `size`² canvas filled `#444`, with
/// the top-left and bottom-right quarters painted `color`.
///
/// `fillRect` on a canvas is exact for whole pixels. The one-pixel level is
/// not: each quarter-rectangle covers a quarter of the pixel, and the 2D
/// context blends it in by that coverage (in the canvas' sRGB bytes, as
/// Chrome's canvas does), one `fillRect` after the other.
fn mipmap(size: u32, color: [u8; 3]) -> Image {
    const GREY: [f64; 3] = [68.0, 68.0, 68.0]; // '#444'
    let n = size as usize;
    let mut data = vec![0u8; n * n * 4];

    if size == 1 {
        let mut px = GREY;
        for _ in 0..2 {
            for (c, p) in px.iter_mut().enumerate() {
                *p = 0.25 * f64::from(color[c]) + 0.75 * *p;
            }
        }
        data.copy_from_slice(&[
            px[0].round() as u8,
            px[1].round() as u8,
            px[2].round() as u8,
            255,
        ]);
        return Image::rgba8(size, size, data);
    }

    let half = n / 2;
    for y in 0..n {
        for x in 0..n {
            let painted = (x < half && y < half) || (x >= half && y >= half);
            let rgb = if painted {
                color
            } else {
                [GREY[0] as u8, GREY[1] as u8, GREY[2] as u8]
            };
            let i = (y * n + x) * 4;
            data[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    Image::rgba8(size, size, data)
}

/// `new THREE.Scene()` with a black background and `new THREE.Fog( 0x000000,
/// 1500, 4000 )`.
fn fogged_scene() -> Scene {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.fog = Some(Fog::new(Color::from_hex(0x000000), 1500.0, 4000.0).into());
    scene
}

pub fn init() -> App {
    let screen_width = INNER_WIDTH;
    let screen_height = INNER_HEIGHT;

    let camera = PerspectiveCamera::new(35.0, screen_width / screen_height, 1.0, 5000.0);
    camera.node.borrow_mut().position.z = 1500.0;

    let scene1 = fogged_scene();
    let scene2 = fogged_scene();

    // GROUND

    // `const canvas = mipmap( 128, '#f00' ); new THREE.CanvasTexture( canvas )`
    // and the eight `textureCanvas1.mipmaps[ i ] = mipmap( … )` lines. A
    // `CanvasTexture` is a `Texture` over the canvas' pixels; level 0 is the
    // same canvas as the image.
    let levels = [
        (128, [0xff, 0x00, 0x00]), // '#f00'
        (64, [0x00, 0xff, 0x00]),  // '#0f0'
        (32, [0x00, 0x00, 0xff]),  // '#00f'
        (16, [0x44, 0x00, 0x00]),  // '#400'
        (8, [0x00, 0x44, 0x00]),   // '#040'
        (4, [0x00, 0x00, 0x44]),   // '#004'
        (2, [0x00, 0x44, 0x44]),   // '#044'
        (1, [0x44, 0x00, 0x44]),   // '#404'
    ];
    let mipmaps: Vec<Image> = levels
        .iter()
        .map(|&(size, color)| mipmap(size, color))
        .collect();

    let texture_canvas1 = Texture::new(128, 128, Some(mipmaps[0].data.clone()));
    texture_canvas1.set_mipmaps(mipmaps);
    texture_canvas1.set_color_space(ColorSpace::SRGB);
    texture_canvas1.set_repeat(1000.0, 1000.0);
    texture_canvas1.set_wrapping(
        three_rs::textures::Wrapping::Repeat,
        three_rs::textures::Wrapping::Repeat,
    );

    let texture_canvas2 = texture_canvas1.clone_texture();
    texture_canvas2.set_mag_filter(TextureFilter::Nearest);
    texture_canvas2.set_min_filter(MinFilter::NearestMipmapNearest);

    let mut material_canvas1 = MeshBasicNodeMaterial::new();
    material_canvas1.map = Some(texture_canvas1);
    let mut material_canvas2 = MeshBasicNodeMaterial::new();
    material_canvas2.color = Color::from_hex(0xffccaa);
    material_canvas2.map = Some(texture_canvas2);

    let geometry = Rc::new(plane_geometry(100.0, 100.0, 1, 1));

    let mesh_canvas1 = Mesh::new(geometry.clone(), Some(material_canvas1));
    {
        let mut mesh = mesh_canvas1.borrow_mut();
        mesh.set_rotation(-std::f64::consts::PI / 2.0, 0.0, 0.0);
        mesh.scale.set(1000.0, 1000.0, 1000.0);
    }

    let mesh_canvas2 = Mesh::new(geometry.clone(), Some(material_canvas2));
    {
        let mut mesh = mesh_canvas2.borrow_mut();
        mesh.set_rotation(-std::f64::consts::PI / 2.0, 0.0, 0.0);
        mesh.scale.set(1000.0, 1000.0, 1000.0);
    }

    scene1.add(&mesh_canvas1);
    scene2.add(&mesh_canvas2);

    // PAINTING

    // `await new THREE.TextureLoader().loadAsync( 'textures/758px-…jpg' )`,
    // cloned before either copy's settings change.
    let texture_painting1 = TextureLoader::new()
        .load(
            three_rs::testing::three_js_dir()
                .join("examples/textures/758px-Canestra_di_frutta_(Caravaggio).jpg"),
        )
        .unwrap();
    let texture_painting2 = texture_painting1.clone_texture();

    texture_painting1.set_color_space(ColorSpace::SRGB);
    texture_painting2.set_color_space(ColorSpace::SRGB);

    texture_painting1.set_min_filter(MinFilter::Linear);
    texture_painting1.set_mag_filter(TextureFilter::Linear);
    texture_painting2.set_min_filter(MinFilter::Nearest);
    texture_painting2.set_mag_filter(TextureFilter::Nearest);

    let (image_width, image_height) = texture_painting1.size();
    let (image_width, image_height) = (f64::from(image_width), f64::from(image_height));

    let mut material_painting1 = MeshBasicNodeMaterial::new();
    material_painting1.color = Color::from_hex(0xffffff);
    material_painting1.map = Some(texture_painting1);
    let mut material_painting2 = MeshBasicNodeMaterial::new();
    material_painting2.color = Color::from_hex(0xffccaa);
    material_painting2.map = Some(texture_painting2);

    let geometry_painting = Rc::new(plane_geometry(100.0, 100.0, 1, 1));
    let mesh1 = Mesh::new(geometry_painting.clone(), Some(material_painting1));
    let mesh2 = Mesh::new(geometry_painting, Some(material_painting2));

    // `addPainting( zscene, zmesh )`.
    let add_painting = |zscene: &Scene, zmesh: &three_rs::Node| {
        {
            let mut mesh = zmesh.borrow_mut();
            mesh.scale.x = image_width / 100.0;
            mesh.scale.y = image_height / 100.0;
        }
        zscene.add(zmesh);

        let mut frame_material = MeshBasicNodeMaterial::new();
        frame_material.color = Color::from_hex(0x000000);
        let mesh_frame = Mesh::new(geometry.clone(), Some(frame_material));
        {
            let mut mesh = mesh_frame.borrow_mut();
            mesh.position.z = -10.0;
            mesh.scale.x = 1.1 * image_width / 100.0;
            mesh.scale.y = 1.1 * image_height / 100.0;
        }
        zscene.add(&mesh_frame);

        let mut shadow_material = MeshBasicNodeMaterial::new();
        shadow_material.color = Color::from_hex(0x000000);
        shadow_material.opacity = 0.75;
        shadow_material.transparent = true;
        let mesh_shadow = Mesh::new(geometry.clone(), Some(shadow_material));
        {
            let mut mesh = mesh_shadow.borrow_mut();
            mesh.position.y = -1.1 * image_height / 2.0;
            mesh.position.z = -1.1 * image_height / 2.0;
            mesh.set_rotation(-std::f64::consts::PI / 2.0, 0.0, 0.0);
            mesh.scale.x = 1.1 * image_width / 100.0;
            mesh.scale.y = 1.1 * image_height / 100.0;
        }
        zscene.add(&mesh_shadow);

        let floor_height = -1.117 * image_height / 2.0;
        mesh_canvas1.borrow_mut().position.y = floor_height;
        mesh_canvas2.borrow_mut().position.y = floor_height;
    };

    add_painting(&scene1, &mesh1);
    add_painting(&scene2, &mesh2);

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(screen_width, screen_height);
    renderer.auto_clear = false;

    App {
        renderer,
        scene1,
        scene2,
        camera,
        mouse_x: 0.0,
        mouse_y: 0.0,
        screen_width,
        screen_height,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    animate_cpu_only(app);

    let (width, height) = (app.screen_width, app.screen_height);

    app.renderer.clear(true, true);
    app.renderer.set_scissor_test(true);

    app.renderer
        .set_scissor(0.0, 0.0, width / 2.0 - 2.0, height);
    app.renderer.render(&mut app.scene1, &mut app.camera);

    app.renderer
        .set_scissor(width / 2.0, 0.0, width / 2.0 - 2.0, height);
    app.renderer.render(&mut app.scene2, &mut app.camera);

    app.renderer.set_scissor_test(false);
}

/// The camera half of `animate()`: ease towards the mouse and look at the
/// origin. With the mouse at rest the first frame lifts the camera to y = 10.
pub fn animate_cpu_only(app: &mut App) {
    let mut node = app.camera.node.borrow_mut();
    node.position.x += (app.mouse_x - node.position.x) * 0.05;
    node.position.y += (-(app.mouse_y - 200.0) - node.position.y) * 0.05;
    drop(node);

    // `camera.lookAt( scene1.position )` — the scene root never moves.
    app.camera.look_at(&Vector3::ZERO);
}

/// The page has no `onWindowResize()`: `SCREEN_WIDTH` / `SCREEN_HEIGHT` are
/// read once. A host that resizes the canvas still needs the drawing buffer
/// to follow, so this does what a resize handler would, and the scissor
/// halves follow the new width.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.screen_width = width;
    app.screen_height = height;
    app.camera.aspect = width / height;
    app.camera.update_projection_matrix();
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer. `None` here: the
/// page creates none (it follows the mouse through `mousemove` instead).
pub fn controls(_app: &mut App) -> Option<&mut OrbitControls> {
    None
}

/// The controls and the camera at once, for a host delivering pointer events.
/// `None` here: the page creates no controls.
pub fn controls_and_camera(_app: &mut App) -> Option<(&mut OrbitControls, &mut PerspectiveCamera)> {
    None
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
        .unwrap_or_else(|| "target/webgpu_materials_texture_manualmipmap.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
