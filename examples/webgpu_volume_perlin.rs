//! Port of `three.js/examples/webgpu_volume_perlin.html`, calling the three-rs
//! API in the same order the page's `init()` / `animate()` do.
//!
//! Under the e2e harness the viewport is `400 * 2` x `250 * 2` with no
//! `deviceScaleFactor`, so `window.innerWidth` / `innerHeight` /
//! `devicePixelRatio` are 800, 500 and 1.
//!
//! A 128³ `Data3DTexture` of Perlin noise, filled on the CPU by
//! `ImprovedNoise`, is raymarched inside a unit box drawn `BackSide`; the
//! first step whose value crosses `threshold` is refined by four bisection
//! steps and shaded with the volume's gradient. No `Math.random` is drawn by
//! anything the graded frame depends on — `ImprovedNoise` has a fixed
//! permutation table — and the `Inspector` and its three sliders only change
//! uniforms the port holds at their defaults.

use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::addons::improved_noise::ImprovedNoise;
use three_rs::addons::raymarching::raymarching_box;
use three_rs::materials::Side;
use three_rs::nodes::tsl::{
    boolean, break_loop, float, if_then, int, loop_n, texture_3d, to_const, to_var, uniform_value,
    vec3, vec4, wgsl_select, Texture3DNode,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::{
    box_geometry, Data3DTexture, Mesh, MeshBasicNodeMaterial, MinFilter, PerspectiveCamera,
    Renderer, RendererParameters, Scene, TextureFilter,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`
pub const DPR: f64 = 1.0;

/// `const REFINEMENT_STEPS = 4;`
pub const REFINEMENT_STEPS: i64 = 4;

/// `const size = 128;`
pub const SIZE: u32 = 128;

/// The page's volume: `perlin.noise( x / size * 6.5, … ) * 128 + 128` per
/// texel, x fastest. The store into a `Uint8Array` is ECMAScript's
/// `ToUint8` — truncate toward zero, then modulo 256 — so a noise value of
/// exactly 1 wraps 256 to 0 rather than saturating; reproduced as is.
pub fn volume_data() -> Vec<u8> {
    let size = SIZE as usize;
    let mut data = Vec::with_capacity(size * size * size);
    let perlin = ImprovedNoise::new();
    let s = f64::from(SIZE);
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                // `vector.set( x, y, z ).divideScalar( size )` — a multiply by
                // `1 / size` in `Vector3.divideScalar()`, which for a power of
                // two is the same as the divide.
                let (vx, vy, vz) = (
                    x as f64 * (1.0 / s),
                    y as f64 * (1.0 / s),
                    z as f64 * (1.0 / s),
                );
                let d = perlin.noise(vx * 6.5, vy * 6.5, vz * 6.5);
                let v = (d * 128.0 + 128.0).trunc() as i64;
                data.push(v.rem_euclid(256) as u8);
            }
        }
    }
    data
}

/// `new THREE.Data3DTexture( data, size, size, size )` with `RedFormat`
/// (`r8unorm`) and `LinearFilter` both ways.
pub fn volume_texture() -> Data3DTexture {
    let texture = Data3DTexture::new(
        volume_data(),
        SIZE,
        SIZE,
        SIZE,
        wgpu::TextureFormat::R8Unorm,
    );
    texture.set_min_filter(MinFilter::Linear);
    texture.set_mag_filter(TextureFilter::Linear);
    // `texture.unpackAlignment = 1` is a WebGL row-alignment hint; WebGPU's
    // `writeTexture` takes the tight `bytesPerRow` it is given.
    texture.set_needs_update();
    texture
}

/// `opaqueRaymarchingTexture( { texture, steps, threshold, refine } )` — the
/// page's `Fn()`, which has no layout, so three inlines it into the fragment
/// flow; `block` does the same here. Returns `finalColor`.
pub fn opaque_raymarching_texture(
    texture: &Texture3DNode,
    steps: &NodeRef,
    threshold: &NodeRef,
    refine: &NodeRef,
) -> NodeRef {
    // `const finalColor = vec4( 0 ).toVar();`
    let final_color = to_var(None, vec4(0.0, 0.0, 0.0, 0.0));
    // `const positionPrev = vec3( 0 ).toVar();`
    let position_prev = to_var(None, vec3(0.0, 0.0, 0.0));
    // `const hasPrev = bool( false ).toVar();`
    let has_prev = to_var(None, boolean(false));

    let mut statements = vec![final_color.clone(), position_prev.clone(), has_prev.clone()];

    statements.extend(raymarching_box(steps, |position_ray, _step_size| {
        // `texture.sample( positionRay.add( 0.5 ) ).r.toVar()`
        let map_value = to_var(None, texture.sample_r(position_ray.add(0.5)));

        // `const surfacePos = positionRay.toVar();`
        let surface_pos = to_var(None, position_ray.clone());

        let refine_block = {
            let p0 = to_var(None, position_prev.clone());
            let p1 = to_var(None, position_ray.clone());
            let bisect = loop_n("i", int(REFINEMENT_STEPS), |_| {
                // `p0.add( p1 ).mul( 0.5 ).toConst()`
                let pm = to_const(None, p0.add(&p1).mul(0.5));
                // `texture.sample( pm.add( 0.5 ) ).r.toConst()`
                let dm = to_const(None, texture.sample_r(pm.add(0.5)));
                // `dm.greaterThan( threshold )`, used twice, so a `let`.
                let is_greater = to_const(None, dm.greater_than(threshold));
                vec![
                    // `select( isGreater, pm, p1 )` is WGSL `select( p1, pm,
                    // isGreater )` — false value first.
                    p1.assign(wgsl_select(p1.clone(), pm.clone(), is_greater.clone())),
                    p0.assign(wgsl_select(pm, p0.clone(), is_greater)),
                ]
            });
            if_then(
                refine.and(&has_prev),
                vec![p0.clone(), p1.clone(), bisect, surface_pos.assign(&p1)],
            )
        };

        // `const p = vec3( surfacePos ).add( 0.5 );` — read seven times by
        // `normal()`, so three's dump has it as a `let`; see
        // `raymarching_box` for why the port asks for that by hand.
        let p = to_const(None, surface_pos.add(0.5));

        let hit = if_then(
            map_value.greater_than(threshold),
            vec![
                surface_pos.clone(),
                refine_block,
                // `finalColor.rgb.assign( texture.normal( p ).mul( 0.5 ).add(
                //     surfacePos.mul( 1.5 ).add( 0.25 ) ) )`
                final_color.rgb().assign(
                    texture
                        .normal(p)
                        .mul(0.5)
                        .add(surface_pos.mul(1.5).add(0.25)),
                ),
                // `finalColor.a.assign( 1 )`
                final_color.a().assign(float(1.0)),
                break_loop(),
            ],
        );

        vec![
            map_value.clone(),
            hit,
            position_prev.assign(position_ray),
            has_prev.assign(boolean(true)),
        ]
    }));

    three_rs::nodes::tsl::block(statements, final_color)
}

/// The page's `NodeMaterial`: `colorNode` the raymarch, `BackSide`,
/// `transparent`. A plain `NodeMaterial` and a `MeshBasicNodeMaterial` build
/// the same fragment once `colorNode` replaces `materialColor` — unlit, no
/// maps — so the port's basic material stands in for it.
pub fn material(texture: &Data3DTexture) -> MeshBasicNodeMaterial {
    // `uniform( 0.6 )`, `uniform( 200 )`, `uniform( true )`.
    let threshold = uniform_value(Type::F32, vec![0.6]);
    let steps = uniform_value(Type::F32, vec![200.0]);
    let refine = uniform_value(Type::Bool, vec![1.0]);

    let mut material = MeshBasicNodeMaterial::new();
    // `texture3D( texture, null, 0 )`
    let volume = texture_3d(texture, float(0.0));
    material.color_node = Some(opaque_raymarching_texture(
        &volume, &steps, &threshold, &refine,
    ));
    material.side = Side::Back;
    material.transparent = true;
    material
}

pub struct App {
    pub renderer: Renderer,
    pub scene: Scene,
    pub camera: PerspectiveCamera,
    pub controls: OrbitControls,
    pub mesh: three_rs::Node,
}

pub fn init() -> App {
    let mut renderer = Renderer::new(RendererParameters { antialias: true }).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    let scene = Scene::new();

    let mut camera = PerspectiveCamera::new(60.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 2.0);

    // `new OrbitControls( camera, renderer.domElement )` — its constructor's
    // `update()` looks the camera at the origin, which it already faces.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);

    // Texture

    let texture = volume_texture();

    // Shader

    let material = material(&texture);

    let mesh = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    scene.add(&mesh);

    App {
        renderer,
        scene,
        camera,
        controls,
        mesh,
    }
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
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
        .unwrap_or_else(|| "target/webgpu_volume_perlin.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
