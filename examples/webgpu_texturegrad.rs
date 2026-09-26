//! Port of `three.js/examples/webgpu_texturegrad.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1, and the clock is pinned to zero, so
//! `time` is 0 in the graded frame.
//!
//! A unit plane under an orthographic camera, textured with
//! `uv_grid_opengl.jpg` through four `textureSampleGrad` taps whose explicit
//! gradients come from a `cos()` of the uv: the top half keeps them, the bottom
//! half zeroes them (mip 0), and a white line marks the seam.
//!
//! # Two canvases, one renderer
//!
//! The page calls `init()` twice — `init()` for a WebGPU backend and `init(
//! true )` for a WebGL one — and each makes its own `WebGPURenderer` with an
//! `innerWidth / 2` canvas, the WebGL one styled to sit on the right half
//! with a darker background. The port has no WebGL backend and one canvas, so
//! it keeps both `init()`s' scenes and cameras and draws them into the two
//! halves of one 800 x 500 canvas through the viewport and the scissor, both
//! with the WGSL the WebGPU backend generates. The two halves of three's own
//! frame differ only by the background and by whatever the GLSL path rounds
//! differently.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::tsl::{
    block, float, if_then, texture_grad, time, to_const, to_var, uv, vec2_join, vec4,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::{
    plane_geometry, Color, Mesh, OrthographicCamera, PerspectiveCamera, Renderer,
    RendererParameters, Scene, Texture, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// What one `init( forceWebGL )` call makes: its scene and camera, and where
/// its canvas sits on the page.
pub struct Canvas {
    pub scene: Scene,
    pub camera: OrthographicCamera,
    /// `forceWebGL`: the canvas styled `left: 50%`.
    pub force_webgl: bool,
}

pub struct App {
    pub renderer: Renderer,
    /// `init()` then `init( true )`.
    pub canvases: [Canvas; 2],
    width: f64,
    height: f64,
}

/// `material.colorNode = Fn( () => { … } )()` — no layout, so three inlines
/// it into the fragment flow; `block` does the same here.
pub fn color_node(map: &Texture) -> NodeRef {
    // `const color = vec4( 1. ).toVar();`
    let color = to_var(None, vec4(1.0, 1.0, 1.0, 1.0));

    // `const vuv = uv().toVar();`
    let vuv = to_var(None, uv());
    // `pow( float( 0.0625 ).sub( cos( vuv.x.mul( 20.0 ).add( time ) ) ).mul( 0.0625 ), 2.0 )`.
    // Read nine times, so three's builder promotes it to `let nodeConst0`
    // with no `toConst()` on the page; the port asks for the `let` itself
    // (`docs/nodes.md` §8, "Usage-promoted temps").
    let blur = to_const(
        None,
        float(0.0625)
            .sub(vuv.x().mul(20.0).add(time()).cos())
            .mul(0.0625)
            .pow(2.0),
    );

    // `const grad = vec2( blur ).toVar();`
    let grad = to_var(None, blur.to(Type::Vec2));

    // `texture( map, vuv.add( vec2( x, y ).mul( 0.5 ) ) ).grad( grad, grad )`.
    // Three's dump names each tap's uv in a `let nodeConstN` just before the
    // tap — `TextureNode.setup()` wraps the uv in an inline `Fn()`, and the
    // builder counts the sum as read twice — so the port asks for the `let`
    // here too (`docs/nodes.md` §36).
    let tap = |x: NodeRef, y: NodeRef| {
        texture_grad(
            map,
            to_const(None, vuv.add(vec2_join(vec![x, y]).mul(0.5))),
            grad.clone(),
            grad.clone(),
        )
    };

    let sum = tap(blur.clone(), blur.clone())
        .mul(0.25)
        .add(tap(blur.clone(), blur.negate()).mul(0.25))
        .add(tap(blur.negate(), blur.clone()).mul(0.25))
        .add(tap(blur.negate(), blur.negate()).mul(0.25));

    block(
        vec![
            color.clone(),
            vuv.clone(),
            grad.clone(),
            // `If( vuv.y.greaterThan( 0.5 ), () => { grad.assign( 0 ); } );`
            if_then(vuv.y().greater_than(0.5), vec![grad.assign(float(0.0))]),
            color.assign(sum),
            // `If( vuv.y.greaterThan( 0.497 ).and( vuv.y.lessThan( 0.503 ) ), … )`
            if_then(
                vuv.y().greater_than(0.497).and(vuv.y().less_than(0.503)),
                vec![color.assign(float(1.0))],
            ),
        ],
        color,
    )
}

/// `await new THREE.TextureLoader().loadAsync( 'textures/uv_grid_opengl.jpg' )`
/// — left in `NoColorSpace`, as the page leaves it.
pub fn load_map() -> Texture {
    TextureLoader::new()
        .load(three_rs::testing::three_js_dir().join("examples/textures/uv_grid_opengl.jpg"))
        .unwrap()
}

/// The scene half of one `init( forceWebGL )`.
fn init_canvas(map: &Texture, force_webgl: bool) -> Canvas {
    let aspect = (INNER_WIDTH / 2.0) / INNER_HEIGHT;
    // `new THREE.OrthographicCamera( - aspect, aspect )`: top 1, bottom -1,
    // near 0.1, far 2000 are the constructor's defaults.
    let mut camera = OrthographicCamera::new(-aspect, aspect, 1.0, -1.0, 0.1, 2000.0);
    camera.object.position.z = 2.0;

    let mut scene = Scene::new();

    // texture

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.color_node = Some(color_node(map));

    let plane = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    scene.add(&plane);

    scene.set_background(Color::from_hex(if force_webgl {
        0x212121
    } else {
        0x313131
    }));

    Canvas {
        scene,
        camera,
        force_webgl,
    }
}

pub fn init() -> App {
    // Both `init()`s load the same file; one decoded texture serves both.
    let map = load_map();

    let canvases = [init_canvas(&map, false), init_canvas(&map, true)];

    // `new THREE.WebGPURenderer( { antialias: false } )`, sized to the whole
    // window rather than to half of it: see the module docs.
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    App {
        renderer,
        canvases,
        width: INNER_WIDTH,
        height: INNER_HEIGHT,
    }
}

/// Both canvases' `animate()`: each `renderer.render( scene, camera )` into
/// its own half of the canvas.
pub fn animate(app: &mut App) {
    let half = app.width / 2.0;
    app.renderer.set_scissor_test(true);
    for canvas in &mut app.canvases {
        let left = if canvas.force_webgl { half } else { 0.0 };
        app.renderer.set_viewport(left, 0.0, half, app.height);
        app.renderer.set_scissor(left, 0.0, half, app.height);
        app.renderer.render(&mut canvas.scene, &mut canvas.camera);
    }
    app.renderer.set_scissor_test(false);
}

/// The page's `onWindowResize()`, once per canvas: each keeps its frustum
/// height and takes half the window's aspect.
pub fn resize(app: &mut App, width: f64, height: f64) {
    app.width = width;
    app.height = height;
    app.renderer.set_size(width, height);

    let aspect = (width / 2.0) / height;
    for canvas in &mut app.canvases {
        let camera = &mut canvas.camera;
        let frustum_height = camera.top - camera.bottom;
        camera.left = -frustum_height * aspect / 2.0;
        camera.right = frustum_height * aspect / 2.0;
        camera.update_projection_matrix();
    }
}

/// The example's controls, for a host that has a pointer. `None` here:
/// the page creates none.
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
        .unwrap_or_else(|| "target/webgpu_texturegrad.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
