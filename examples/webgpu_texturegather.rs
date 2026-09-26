//! Port of `three.js/examples/webgpu_texturegather.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1. The page draws nothing from
//! `Math.random()` or the clock.
//!
//! `init()` renders a red `MeshStandardMaterial` cube once into a 100 x 100
//! render target with a `DepthTexture` (`compareFunction =
//! LessEqualCompare`). A unit plane then reads it back two ways: the top half
//! is `textureGather` of the colour target's red channel, tiled ten times
//! through `RepeatWrapping`; the bottom half is `textureGatherCompare` of the
//! depth against 1 — white where the target was cleared, black where the cube
//! is, with a coloured fringe where the four gathered texels disagree. Both
//! taps carry the page's `ivec2( 0, 7 )` texel offset.
//!
//! The page's two canvases (a WebGPU backend and a `forceWebGL` one) are the
//! two halves of one canvas here, as in `webgpu_texturegrad`; see that
//! example's module docs and `docs/nodes.md` §44.
//!
//! The page asks the target for `generateMipmaps: true` and
//! `minFilter: LinearMipmapLinearFilter`. `textureGather` reads mip level 0
//! and ignores the filters, so neither reaches a pixel, and the port's render
//! target, which has no mip chain, leaves both out.

use std::f64::consts::PI;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::tsl::{
    block, depth_texture_gather_compare, float, if_else, int, ivec2, texture_gather, to_var, uv,
    vec4,
};
use three_rs::nodes::NodeRef;
use three_rs::textures::Wrapping;
use three_rs::{
    box_geometry, plane_geometry, AmbientLight, Color, DepthTexture, DirectionalLight, Mesh,
    MeshStandardNodeMaterial, OrthographicCamera, PerspectiveCamera, RenderTarget, Renderer,
    RendererParameters, Scene, Texture,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

/// What one `init( forceWebGL )` call makes: its scene and camera, the render
/// target its plane reads, and where its canvas sits on the page.
pub struct Canvas {
    pub scene: Scene,
    pub camera: OrthographicCamera,
    pub rt: RenderTarget,
    pub rt_scene: Scene,
    pub rt_camera: PerspectiveCamera,
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

/// `material.colorNode = Fn( () => { … } )()` over `colorNode.value =
/// rt.texture` and `depthNode.value = rt.depthTexture` — no layout, so three
/// inlines it into the fragment flow; `block` does the same here.
pub fn color_node(color: &Texture, depth: &DepthTexture) -> NodeRef {
    // `const color = vec4( 1 ).toVar();`
    let result = to_var(None, vec4(1.0, 1.0, 1.0, 1.0));

    // `const vuv = uv().toVar();`
    let vuv = to_var(None, uv());

    block(
        vec![
            result.clone(),
            vuv.clone(),
            if_else(
                vuv.y().greater_than(0.5),
                // `colorNode.sample( vuv.mul( 10 ) ).offset( ivec2( 0, 7 ) ).gather( 0 )`
                vec![result.assign(texture_gather(
                    color,
                    vuv.mul(10.0),
                    int(0),
                    Some(ivec2(int(0), int(7))),
                ))],
                // `depthNode.sample( vuv ).offset( ivec2( 0, 7 ) ).gather( 0 ).compare( 1 )`
                // — `gather( 0 )`'s channel is not in the WGSL of a depth
                // gather (`generateTextureGatherCompare()`).
                vec![result.assign(depth_texture_gather_compare(
                    depth,
                    vuv.clone(),
                    float(1.0),
                    Some(ivec2(int(0), int(7))),
                ))],
            ),
        ],
        result,
    )
}

/// The scene half of one `init( forceWebGL )`, up to `renderer.render(
/// rtScene, rtCamera )`, which [`init`] does once the renderer exists.
fn init_canvas(force_webgl: bool) -> Canvas {
    let aspect = (INNER_WIDTH / 2.0) / INNER_HEIGHT;
    // `new THREE.OrthographicCamera( - aspect, aspect )`: top 1, bottom -1,
    // near 0.1, far 2000 are the constructor's defaults.
    let mut camera = OrthographicCamera::new(-aspect, aspect, 1.0, -1.0, 0.1, 2000.0);
    camera.object.position.z = 2.0;

    let mut scene = Scene::new();

    scene.set_background(Color::from_hex(if force_webgl {
        0x212121
    } else {
        0x313131
    }));

    //

    // `new THREE.DepthTexture()` with `compareFunction = LessEqualCompare`:
    // the comparison sampler `depth_texture_gather_compare` binds.
    let depth_texture = DepthTexture::new();
    let rt = RenderTarget::new(100, 100);
    rt.set_depth_texture(depth_texture.clone());
    rt.texture()
        .set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    let camera_z = 2.5;
    let mut rt_scene = Scene::new();
    rt_scene.set_background(Color::from_hex(0x808080));
    let rt_camera = PerspectiveCamera::new(
        50.0,
        1.0,
        camera_z - 0.5 * 3f64.sqrt(),
        camera_z + 0.5 * 3f64.sqrt(),
    );
    rt_camera.node.borrow_mut().position.z = camera_z;

    // `new THREE.DirectionalLight()`: white, intensity 1, target at the origin.
    let dir_light = DirectionalLight::new(Color::from_hex(0xffffff), 1.0);
    dir_light.borrow_mut().position.set(1.0, 1.0, 0.0);
    rt_scene.add(&dir_light);
    rt_scene.add(&AmbientLight::new(Color::from_hex(0xffffff), 0.1));

    // `new THREE.MeshStandardNodeMaterial( { color: 0xff0000 } )` — roughness
    // 1 and metalness 0 are the defaults.
    let cube = Mesh::new(
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)),
        MeshStandardNodeMaterial::standard(Color::from_hex(0xff0000), 1.0, 0.0),
    );
    cube.borrow_mut().set_rotation(PI / 4.0, PI / 4.0, 0.0);
    rt_scene.add(&cube);

    // texture

    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.color_node = Some(color_node(&rt.texture(), &depth_texture));

    let plane = Mesh::new(Rc::new(plane_geometry(1.0, 1.0, 1, 1)), material);
    scene.add(&plane);

    Canvas {
        scene,
        camera,
        rt,
        rt_scene,
        rt_camera,
        force_webgl,
    }
}

pub fn init() -> App {
    let mut canvases = [init_canvas(false), init_canvas(true)];

    // `new THREE.WebGPURenderer( { antialias: false } )`, sized to the whole
    // window rather than to half of it: see the module docs.
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // `renderer.setRenderTarget( rt ); renderer.render( rtScene, rtCamera );
    // renderer.setRenderTarget( null );`, once per `init()`.
    for canvas in &mut canvases {
        renderer.set_render_target(Some(canvas.rt.clone()));
        renderer.render(&mut canvas.rt_scene, &mut canvas.rt_camera);
        renderer.set_render_target(None);
    }

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
        .unwrap_or_else(|| "target/webgpu_texturegather.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
