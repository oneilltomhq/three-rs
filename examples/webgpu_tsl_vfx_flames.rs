//! Port of `three.js/examples/webgpu_tsl_vfx_flames.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is 800 x 500 at a device pixel ratio of
//! 1 and `performance.now()` is pinned to 0, so the `time` uniform is 0 on the
//! graded frame.
//!
//! Two stand-ins, both for things the port does not have yet and neither of
//! which changes a pixel:
//!
//! - **`THREE.Sprite` is a `Mesh` over the sprite's own quad.** Both flames
//!   set a `vertexNode` (`billboarding()`), and `NodeMaterial.setupVertex()`
//!   returns a `vertexNode` as it stands, so `SpriteNodeMaterial`'s own
//!   position-view seam — the only place `Sprite.center` is read — never runs.
//!   What is left of a `Sprite` is its shared geometry, which
//!   [`sprite_geometry`] rebuilds vertex for vertex.
//! - **The gradient `CanvasTexture` is filled in Rust.** The page draws a
//!   128 x 1 `createLinearGradient( 0, 0, 128, 0 )` through five sRGB colour
//!   stops; [`gradient_texture`] evaluates the same gradient at each pixel
//!   centre, interpolating in sRGB as the canvas does.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::core::{BufferAttribute, BufferGeometry};
use three_rs::materials::{MeshBasicNodeMaterial, Side};
use three_rs::nodes::tsl::{
    billboarding, block, float, mix, spherize_uv, texture_level, texture_uv, time, to_var, two_pi,
    uv, vec2, vec2_join, vec3, vec4_join, Billboarding,
};
use three_rs::nodes::NodeRef;
use three_rs::{
    Color, ColorSpace, Mesh, PerspectiveCamera, Renderer, RendererParameters, Scene, Texture,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

/// `Sprite`'s shared geometry (`src/objects/Sprite.js`): a unit quad in the
/// XY plane, `( -0.5, -0.5 )` to `( 0.5, 0.5 )`, indexed `0, 1, 2, 0, 2, 3`.
pub fn sprite_geometry() -> BufferGeometry {
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(
            vec![
                -0.5, -0.5, 0.0, //
                0.5, -0.5, 0.0, //
                0.5, 0.5, 0.0, //
                -0.5, 0.5, 0.0,
            ],
            3,
        ),
    );
    geometry.set_attribute(
        "uv",
        BufferAttribute::new(vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0], 2),
    );
    geometry.set_index(&[0, 1, 2, 0, 2, 3]);
    geometry
}

/// `gradient.colors`.
const GRADIENT_COLORS: [u32; 5] = [0x090033, 0x5f1f93, 0xe02e96, 0xffbd80, 0xfff0db];

/// The page's `gradient.texture`: a 128 x 1 canvas filled by
/// `createLinearGradient( 0, 0, 128, 0 )` with a stop at `i / 4` for each of
/// [`GRADIENT_COLORS`], `colorSpace = SRGBColorSpace`.
///
/// A canvas gradient interpolates the stops' sRGB bytes linearly and paints
/// each pixel with its value at the pixel centre, `x + 0.5`.
pub fn gradient_texture() -> Texture {
    const WIDTH: usize = 128;
    let stops = GRADIENT_COLORS.len() - 1;
    let mut data = Vec::with_capacity(WIDTH * 4);
    for x in 0..WIDTH {
        let t = (x as f64 + 0.5) / WIDTH as f64 * stops as f64;
        let i = (t.floor() as usize).min(stops - 1);
        let f = t - i as f64;
        let (a, b) = (GRADIENT_COLORS[i], GRADIENT_COLORS[i + 1]);
        for shift in [16, 8, 0] {
            let ca = ((a >> shift) & 0xff) as f64;
            let cb = ((b >> shift) & 0xff) as f64;
            data.push((ca + (cb - ca) * f).round() as u8);
        }
        data.push(255);
    }
    let texture = Texture::new(WIDTH as u32, 1, Some(data));
    texture.set_color_space(ColorSpace::SRGB);
    texture
}

/// `flame1Material.colorNode = Fn( () => { … } )()`.
pub fn flame1_color(cellular: &Texture, gradient: &Texture) -> NodeRef {
    let mut s = Vec::new();

    // main UV
    //   const mainUv = uv().toVar();
    let main_uv = to_var(None, uv());
    s.push(main_uv.clone());
    //   mainUv.assign( spherizeUV( mainUv, 10 ).mul( 0.6 ).add( 0.2 ) ); // spherize
    s.push(main_uv.assign(spherize_uv(main_uv.clone(), 10.0).mul(0.6).add(0.2)));
    //   mainUv.assign( mainUv.pow( vec2( 1, 2 ) ) ); // stretch
    s.push(main_uv.assign(main_uv.pow(vec2(1.0, 2.0))));
    //   mainUv.assign( mainUv.mul( 2, 1 ).sub( vec2( 0.5, 0 ) ) ); // scale
    s.push(main_uv.assign(main_uv.mul(2.0).mul(1.0).sub(vec2(0.5, 0.0))));

    // gradients
    //   const gradient1 = sin( time.mul( 10 ).sub( mainUv.y.mul( TWO_PI ).mul( 2 ) ) ).toVar();
    let gradient1 = to_var(
        None,
        time()
            .mul(10.0)
            .sub(main_uv.y().mul(two_pi()).mul(2.0))
            .sin(),
    );
    s.push(gradient1.clone());
    //   const gradient2 = mainUv.y.smoothstep( 0, 1 ).toVar();
    let gradient2 = to_var(None, main_uv.y().smoothstep(0.0, 1.0));
    s.push(gradient2.clone());
    //   mainUv.x.addAssign( gradient1.mul( gradient2 ).mul( 0.2 ) );
    s.push(
        main_uv
            .x()
            .add_assign(gradient1.mul(gradient2.clone()).mul(0.2)),
    );

    // cellular noise
    //   const cellularUv = mainUv.mul( 0.5 ).add( vec2( 0, time.negate().mul( 0.5 ) ) ).mod( 1 );
    let cellular_uv = main_uv
        .mul(0.5)
        .add(vec2_join(vec![float(0.0), time().negate().mul(0.5)]))
        .mod_(1.0);
    //   const cellularNoise = texture( cellularTexture, cellularUv, 0 ).r.oneMinus().smoothstep( 0, 0.5 ).oneMinus();
    //   cellularNoise.mulAssign( gradient2 );
    // Assigning to an operator node makes three declare it as a var in place;
    // the port asks for the var.
    let cellular_noise = to_var(
        None,
        texture_level(cellular, cellular_uv, float(0.0))
            .x()
            .one_minus()
            .smoothstep(0.0, 0.5)
            .one_minus(),
    );
    s.push(cellular_noise.clone());
    s.push(cellular_noise.mul_assign(gradient2));

    // shape
    //   const shape = mainUv.sub( 0.5 ).mul( vec2( 3, 2 ) ).length().oneMinus().toVar();
    let shape = to_var(
        None,
        main_uv.sub(0.5).mul(vec2(3.0, 2.0)).length().one_minus(),
    );
    s.push(shape.clone());
    //   shape.assign( shape.sub( cellularNoise ) );
    s.push(shape.assign(shape.sub(cellular_noise)));

    // gradient color
    //   const gradientColor = texture( gradient.texture, vec2( shape.remap( 0, 1, 0, 1 ), 0 ) );
    let gradient_color = texture_uv(
        gradient,
        vec2_join(vec![shape.remap(0.0, 1.0, 0.0, 1.0), float(0.0)]),
    );

    // output
    //   const color = mix( gradientColor, vec3( 1 ), shape.step( 0.8 ) );
    //   const alpha = shape.smoothstep( 0, 0.3 );
    //   return vec4( color.rgb, alpha );
    let color = mix(gradient_color, vec3(1.0, 1.0, 1.0), shape.step(0.8));
    let alpha = shape.smoothstep(0.0, 0.3);
    block(s, vec4_join(vec![color.rgb(), alpha]))
}

/// `flame2Material.colorNode = Fn( () => { … } )()`.
pub fn flame2_color(cellular: &Texture, perlin: &Texture) -> NodeRef {
    let mut s = Vec::new();

    // main UV
    let main_uv = to_var(None, uv());
    s.push(main_uv.clone());
    //   mainUv.assign( spherizeUV( mainUv, 10 ).mul( 0.6 ).add( 0.2 ) ); // spherize
    s.push(main_uv.assign(spherize_uv(main_uv.clone(), 10.0).mul(0.6).add(0.2)));
    //   mainUv.assign( mainUv.abs().pow( vec2( 1, 3 ) ).mul( mainUv.sign() ) ); // stretch
    s.push(
        main_uv.assign(
            main_uv
                .abs()
                .pow(vec2(1.0, 3.0))
                .mul(three_rs::nodes::tsl::sign(main_uv.clone())),
        ),
    );
    //   mainUv.assign( mainUv.mul( 2, 1 ).sub( vec2( 0.5, 0 ) ) ); // scale
    s.push(main_uv.assign(main_uv.mul(2.0).mul(1.0).sub(vec2(0.5, 0.0))));

    // perlin noise
    //   const perlinUv = mainUv.add( vec2( 0, time.negate().mul( 1 ) ) ).mod( 1 );
    //   const perlinNoise = texture( perlinTexture, perlinUv, 0 ).sub( 0.5 ).mul( 1 );
    //   mainUv.x.addAssign( perlinNoise.x.mul( 0.5 ) );
    let perlin_uv = main_uv
        .add(vec2_join(vec![float(0.0), time().negate().mul(1.0)]))
        .mod_(1.0);
    let perlin_noise = texture_level(perlin, perlin_uv, float(0.0))
        .sub(0.5)
        .mul(1.0);
    s.push(main_uv.x().add_assign(perlin_noise.x().mul(0.5)));

    // gradients
    //   const gradient1 = sin( time.mul( 10 ).sub( mainUv.y.mul( TWO_PI ).mul( 2 ) ) );
    //   const gradient2 = mainUv.y.smoothstep( 0, 1 );
    //   const gradient3 = oneMinus( mainUv.y ).smoothstep( 0, 0.3 );
    //   mainUv.x.addAssign( gradient1.mul( gradient2 ).mul( 0.2 ) );
    let gradient1 = time()
        .mul(10.0)
        .sub(main_uv.y().mul(two_pi()).mul(2.0))
        .sin();
    let gradient2 = main_uv.y().smoothstep(0.0, 1.0);
    let gradient3 = main_uv.y().one_minus().smoothstep(0.0, 0.3);
    s.push(main_uv.x().add_assign(gradient1.mul(gradient2).mul(0.2)));

    // displaced perlin noise
    //   const displacementPerlinUv = mainUv.mul( 0.5 ).add( vec2( 0, time.negate().mul( 0.25 ) ) ).mod( 1 );
    //   const displacementPerlinNoise = texture( perlinTexture, displacementPerlinUv, 0 ).sub( 0.5 ).mul( 1 );
    //   const displacedPerlinUv = mainUv.add( vec2( 0, time.negate().mul( 0.5 ) ) ).add( displacementPerlinNoise ).mod( 1 );
    //   const displacedPerlinNoise = texture( perlinTexture, displacedPerlinUv, 0 ).sub( 0.5 ).mul( 1 );
    //   mainUv.x.addAssign( displacedPerlinNoise.mul( 0.5 ) );
    let displacement_uv = main_uv
        .mul(0.5)
        .add(vec2_join(vec![float(0.0), time().negate().mul(0.25)]))
        .mod_(1.0);
    let displacement_noise = texture_level(perlin, displacement_uv, float(0.0))
        .sub(0.5)
        .mul(1.0);
    // `vec2 + vec4` is a `vec4` (`OperatorNode.getNodeType()`), so the mod is
    // `tsl_mod_vec4` and the sample reads its `.xy`.
    let displaced_uv = main_uv
        .add(vec2_join(vec![float(0.0), time().negate().mul(0.5)]))
        .add(displacement_noise)
        .mod_(1.0);
    let displaced_noise = texture_level(perlin, displaced_uv.xy(), float(0.0))
        .sub(0.5)
        .mul(1.0);
    // `x += vec4` is `x = ( vec4( x ) + … ).x` in three's dump.
    s.push(
        main_uv
            .x()
            .assign(main_uv.x().add(displaced_noise.mul(0.5)).x()),
    );

    // cellular noise
    //   const cellularUv = mainUv.add( vec2( 0, time.negate().mul( 1.5 ) ) ).mod( 1 );
    //   const cellularNoise = texture( cellularTexture, cellularUv, 0 ).r.oneMinus().smoothstep( 0.25, 1 );
    let cellular_uv = main_uv
        .add(vec2_join(vec![float(0.0), time().negate().mul(1.5)]))
        .mod_(1.0);
    let cellular_noise = texture_level(cellular, cellular_uv, float(0.0))
        .x()
        .one_minus()
        .smoothstep(0.25, 1.0);

    // shape
    //   const shape = step( mainUv.sub( 0.5 ).mul( vec2( 6, 1 ) ).length(), 0.5 );
    //   shape.assign( shape.mul( cellularNoise ) );
    //   shape.mulAssign( gradient3 );
    //   shape.assign( step( 0.01, shape ) );
    let shape = to_var(
        None,
        three_rs::nodes::tsl::step(main_uv.sub(0.5).mul(vec2(6.0, 1.0)).length(), 0.5),
    );
    s.push(shape.clone());
    s.push(shape.assign(shape.mul(cellular_noise)));
    s.push(shape.mul_assign(gradient3));
    s.push(shape.assign(three_rs::nodes::tsl::step(0.01, shape.clone())));

    // output
    //   return vec4( vec3( 1 ), shape );
    block(s, vec4_join(vec![vec3(1.0, 1.0, 1.0), shape]))
}

/// `new THREE.SpriteNodeMaterial( { side: THREE.DoubleSide } )` with the
/// page's `colorNode` and `vertexNode = billboarding( { horizontalRotation:
/// true } )`.
fn flame_material(color_node: NodeRef) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::sprite();
    material.side = Side::Double;
    material.color_node = Some(color_node);
    material.vertex_node = Some(billboarding(Billboarding {
        horizontal_rotation: true,
        ..Billboarding::default()
    }));
    material
}

pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(1.0, 1.0, 3.0);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x201919));

    // textures

    let loader = three_rs::TextureLoader::new();
    let cellular_texture = loader
        .load(examples_dir().join("textures/noises/voronoi/grayscale-256x256.png"))
        .unwrap();
    let perlin_texture = loader
        .load(examples_dir().join("textures/noises/perlin/rgb-256x256.png"))
        .unwrap();

    // gradient canvas

    let gradient = gradient_texture();

    // flame materials

    let flame1_material = flame_material(flame1_color(&cellular_texture, &gradient));
    let flame2_material = flame_material(flame2_color(&cellular_texture, &perlin_texture));

    // meshes

    let geometry = Rc::new(sprite_geometry());

    //   const flame1 = new THREE.Sprite( flame1Material );
    //   flame1.center.set( 0.5, 0 );
    //   flame1.scale.x = 0.5; // optional
    //   flame1.position.x = - 0.5;
    let flame1 = Mesh::new(geometry.clone(), flame1_material);
    flame1.borrow_mut().scale.x = 0.5;
    flame1.borrow_mut().position.x = -0.5;
    scene.add(&flame1);

    //   const flame2 = new THREE.Sprite( flame2Material );
    //   flame2.center.set( 0.5, 0 );
    //   flame2.position.x = 0.5;
    let flame2 = Mesh::new(geometry, flame2_material);
    flame2.borrow_mut().position.x = 0.5;
    scene.add(&flame2);

    // renderer

    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    //   controls = new OrbitControls( camera, renderer.domElement );
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 0.1;
    controls.max_distance = 50.0;

    App {
        renderer,
        scene,
        camera,
        controls,
    }
}

/// The page's `animate()`: `controls.update()` then `renderer.render()`.
pub fn animate(app: &mut App) {
    let _ = app.controls.update(&mut app.camera, None);

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

/// The controls and the camera at once; see `webgpu_tsl_galaxy`.
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
        .unwrap_or_else(|| "target/webgpu_tsl_vfx_flames.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
