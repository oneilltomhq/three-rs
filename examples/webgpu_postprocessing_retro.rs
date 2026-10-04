//! Port of `three.js/examples/webgpu_postprocessing_retro.html`, calling the
//! three-rs API in the same order the page's `init()` / `animate()` do.
//!
//! The coffee mug and its smoke under a PS1 night sky, drawn by a
//! [`RetroPassNode`] — a quarter-resolution, nearest-filtered scene pass with
//! snapped vertices and level-0 textures — and then through the CRT stack:
//! [`barrel_uv`] read through [`replace_default_uv`], [`color_bleeding`],
//! [`bayer_dither`], `posterize`, [`vignette`] and [`scanlines`].
//!
//! **The rung is ignored.** three's own render of this page misses its
//! reference screenshot (`docs/webgpu_postprocessing_retro-progress.md` has
//! the scores), so there is nothing a passing port could be graded against.
//! What the port checks instead is the two post-processing shaders, against
//! three's dump in `tests/nodes_display_wgsl.rs`, and the pass's own frame in
//! `tests/retro_frames.rs`.
//!
//! The page's GUI (model, curvature, colour depth, … the "Retro Pipeline"
//! toggle) is not ported; its initial values are the uniforms below. The
//! Damaged Helmet model and its `venice_sunset_1k.hdr` environment are only
//! reachable from that GUI.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::geometries::plane_geometry;
use three_rs::loaders::GltfLoader;
use three_rs::materials::Side;
use three_rs::nodes::display::{
    barrel_uv, bayer_dither, circle, color_bleeding, retro_pass, scanlines, vignette,
    RetroPassNode, RetroPassOptions,
};
use three_rs::nodes::tsl::{
    block, call, float, floor, fract, inline_fn, length, mix, mod_, normal_world, position_local,
    posterize, replace_default_uv, rotate_uv_about, screen_size, smoothstep, step, texture_uv,
    time, uniform_value, uv, vec2, vec2_join, vec3, vec4_join,
};
use three_rs::nodes::{NodeRef, Type};
use three_rs::objects::Background;
use three_rs::textures::Wrapping;
use three_rs::{
    AmbientLight, Color, DirectionalLight, Mesh, MeshBasicNodeMaterial, PerspectiveCamera,
    PointLight, RenderPipeline, Renderer, RendererParameters, Scene, TextureLoader,
};

pub const INNER_WIDTH: f64 = 800.0;
pub const INNER_HEIGHT: f64 = 500.0;
/// `renderer.setPixelRatio( window.devicePixelRatio )`.
pub const DPR: f64 = 1.0;

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

pub struct App {
    pub renderer: Renderer,
    /// Shared with `retro`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with `retro`.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    pub retro: RetroPassNode,
    pub render_pipeline: RenderPipeline,
    /// The page's `controls`.
    pub controls: OrbitControls,
}

/// `ps1Background`: a three-stop night-sky gradient over `normalWorld.y`
/// with a hashed star field in longitude / latitude cells.
fn ps1_background() -> NodeRef {
    // Flip Y coordinate for correct orientation
    let flipped_y = normal_world().y().negate();
    let sky_uv = flipped_y.mul(0.5).add(0.5);

    // Simple gradient sky (dark blue at top to purple/orange at horizon)
    let top_color: NodeRef = Color::from_hex(0x000033).into(); // dark blue night sky
    let mid_color: NodeRef = Color::from_hex(0x330066).into(); // purple
    let horizon_color: NodeRef = Color::from_hex(0x663322).into(); // warm orange/brown horizon

    // Two-step gradient (inverted - top is dark, horizon is warm)
    let sky_gradient = mix(
        horizon_color,
        mix(mid_color, top_color, smoothstep(0.4, 0.9, sky_uv.clone())),
        smoothstep(0.0, 0.4, sky_uv),
    );

    // PS1-style "stars" using spherical coordinates
    let longitude = normal_world().x().atan2(normal_world().z());
    let latitude = flipped_y.asin(); // Use flipped Y for latitude too

    // More stars with smaller scale
    let star_scale = float(50.0);
    let star_uv = vec2_join(vec![
        longitude.mul(star_scale.clone()),
        latitude.mul(star_scale),
    ]);
    let star_cell = floor(star_uv.clone());

    // Hash for randomness
    let cell_hash = fract(
        three_rs::nodes::tsl::dot(star_cell, vec2(12.9898, 78.233))
            .sin()
            .mul(43758.5453),
    );

    // Position within cell (0-1)
    let cell_uv = fract(star_uv);
    let to_center = cell_uv.sub(0.5);

    // Gemini-style star: bright center with soft glow + cross flare
    let dist_to_center = length(to_center.clone());

    // Core (small bright center)
    let core = smoothstep(float(0.08), float(0.0), dist_to_center.clone());

    // Soft glow around
    let glow = smoothstep(float(0.25), float(0.0), dist_to_center).mul(0.4);

    // Cross/diamond flare effect
    let cross_x = smoothstep(float(0.15), float(0.0), to_center.x().abs()).mul(smoothstep(
        float(0.4),
        float(0.0),
        to_center.y().abs(),
    ));
    let cross_y = smoothstep(float(0.15), float(0.0), to_center.y().abs()).mul(smoothstep(
        float(0.4),
        float(0.0),
        to_center.x().abs(),
    ));
    let cross = cross_x.add(cross_y).mul(0.3);

    // Combine star shape
    let star_shape = core.add(glow).add(cross);

    // More stars (lower threshold = more stars)
    let is_star = step(0.85, cell_hash.clone());

    // Show stars from horizon up
    let above_horizon = smoothstep(float(-0.2), float(0.1), flipped_y);

    // Star brightness varies + twinkle color
    let star_intensity = is_star
        .mul(above_horizon)
        .mul(star_shape)
        .mul(cell_hash.clone().mul(0.6).add(0.4));

    // Slight color variation (white to light blue)
    let star_color = mix(vec3(1.0, 1.0, 0.95), vec3(0.8, 0.9, 1.0), cell_hash);

    // Combine sky and stars
    mix(sky_gradient, star_color, star_intensity.clamp(0.0, 1.0))
}

pub fn init() -> App {
    let camera = PerspectiveCamera::new(25.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 100.0);
    camera.node.borrow_mut().position.set(8.0, 5.0, 20.0);

    let mut scene = Scene::new();

    // PS1-style background: gradient sky with simple stars
    // `Fn( … )()`: the body runs inside the skybox material's build, as every
    // TSL `Fn` does, so its `normalWorld` is that `BackSide` material's.
    scene.background = Some(Background::Node(call(
        &inline_fn(0, Type::Vec3, |_| ps1_background()),
        Vec::new(),
    )));

    // Model

    // `loadModel( 'Coffee Mug' )`. The loader resolves synchronously here, so
    // the callback's body is written inline: the mug at the origin, the smoke
    // left visible.
    let gltf =
        GltfLoader::load(examples_dir().join("models/gltf/coffeeMug.glb")).expect("coffeeMug.glb");
    gltf.scene.borrow_mut().position.set(0.0, 0.0, 0.0);
    scene.add(&gltf.scene);

    // lighting

    let ambient_light = AmbientLight::new(Color::from_hex(0x404040), 2.0);
    scene.add(&ambient_light);

    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    directional_light.borrow_mut().position.set(5.0, 10.0, 5.0);
    scene.add(&directional_light);

    let point_light = PointLight::new(Color::from_hex(0xff6600), 5.0, 20.0);
    point_light.borrow_mut().position.set(-3.0, 3.0, 2.0);
    scene.add(&point_light);

    // geometry

    let mut smoke_geometry = plane_geometry(1.0, 1.0, 16, 64);
    smoke_geometry.translate(0.0, 0.5, 0.0);
    smoke_geometry.scale(1.5, 6.0, 1.5);

    // texture

    let noise_texture = TextureLoader::new()
        .load(examples_dir().join("textures/noises/perlin/128x128.png"))
        .expect("perlin/128x128.png");
    noise_texture.set_wrapping(Wrapping::Repeat, Wrapping::Repeat);

    // material

    let mut smoke_material = MeshBasicNodeMaterial::new();
    smoke_material.transparent = true;
    smoke_material.side = Side::Double;
    smoke_material.depth_write = false;

    // position

    smoke_material.position_node = Some({
        // twist

        let twist_noise_uv = vec2_join(vec![
            float(0.5),
            mod_(uv().y().mul(0.2).sub(time().mul(0.005)), float(1.0)),
        ]);
        let twist = texture_uv(&noise_texture, twist_noise_uv).x().mul(10.0);
        let assign_twist = position_local().xz().assign(rotate_uv_about(
            position_local().xz(),
            twist,
            vec2(0.0, 0.0),
        ));

        // wind

        let wind_offset = vec2_join(vec![
            texture_uv(
                &noise_texture,
                mod_(vec2_join(vec![float(0.25), time().mul(0.01)]), float(1.0)),
            )
            .x()
            .sub(0.5),
            texture_uv(
                &noise_texture,
                mod_(vec2_join(vec![float(0.75), time().mul(0.01)]), float(1.0)),
            )
            .x()
            .sub(0.5),
        ])
        .mul(uv().y().pow(2.0).mul(10.0));
        let add_wind = position_local().add_assign(wind_offset);

        block(vec![assign_twist, add_wind], position_local())
    });

    // color

    smoke_material.color_node = Some({
        // alpha

        let alpha_noise_uv = uv()
            .mul(vec2(0.5, 0.3))
            .add(vec2_join(vec![float(0.0), time().mul(0.03).negate()]));
        let alpha = smoothstep(0.4, 1.0, texture_uv(&noise_texture, alpha_noise_uv).x())
            // edges fade
            .mul(smoothstep(0.0, 0.1, uv().x()))
            .mul(smoothstep(0.0, 0.1, uv().x().one_minus()))
            .mul(smoothstep(0.0, 0.1, uv().y()))
            .mul(smoothstep(0.0, 0.1, uv().y().one_minus()));

        // color

        let final_color = mix(vec3(0.6, 0.3, 0.2), vec3(1.0, 1.0, 1.0), alpha.pow(3.0));

        vec4_join(vec![final_color, alpha])
    });

    // mesh

    let smoke = Mesh::new(Rc::new(smoke_geometry), smoke_material);
    smoke.borrow_mut().position.y = 1.83;
    scene.add(&smoke);

    // renderer

    let mut parameters = RendererParameters::default();
    parameters.antialias = true;
    let mut renderer = Renderer::new(parameters).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);

    // uniforms

    // PS1-style: 15-bit color (32 levels per channel)
    let color_depth_steps = uniform_value(Type::F32, vec![32.0]);

    // CRT effect parameters (subtle for PS1 look)
    let scanline_intensity = uniform_value(Type::F32, vec![0.3]); // subtle scanlines
    let scanline_density = uniform_value(Type::F32, vec![1.0]); // normalized scanline density
    let scanline_speed = uniform_value(Type::F32, vec![0.0]); // no scanline movement
    let vignette_intensity = uniform_value(Type::F32, vec![0.3]); // subtle vignette
    let bleeding = uniform_value(Type::F32, vec![0.001]); // minimal bleeding
    let curvature = uniform_value(Type::F32, vec![0.02]); // subtle curve
    let affine_distortion = uniform_value(Type::F32, vec![0.0]); // no affine distortion

    // render pipeline

    let mut render_pipeline = RenderPipeline::new();

    // retro pipeline

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let distorted_uv = barrel_uv(curvature.clone(), uv());
    let distorted_delta = circle(curvature.add(0.1).mul(10.0), float(1.0), uv())
        .mul(curvature.clone())
        .mul(0.05);

    let retro = retro_pass(
        scene.clone(),
        camera.clone(),
        RetroPassOptions::default().affine_distortion(affine_distortion),
    );

    let mut retro_pipeline = retro.node();
    retro_pipeline = replace_default_uv(distorted_uv, retro_pipeline);
    retro_pipeline = color_bleeding(retro_pipeline, bleeding.add(distorted_delta));
    retro_pipeline = bayer_dither(retro_pipeline, color_depth_steps.clone());
    retro_pipeline = posterize(retro_pipeline, color_depth_steps);
    retro_pipeline = vignette(retro_pipeline, vignette_intensity, float(0.6), uv());
    retro_pipeline = scanlines(
        retro_pipeline,
        scanline_intensity,
        screen_size().y().mul(scanline_density),
        scanline_speed,
        uv(),
    );

    render_pipeline.output_node = Some(retro_pipeline);

    // controls

    let mut controls = OrbitControls::new(&mut camera.borrow_mut());
    // The renderer's canvas stands in for the element's `clientWidth` /
    // `clientHeight`.
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.min_distance = 0.1;
    controls.max_distance = 50.0;
    controls.target.y = 1.0;

    App {
        renderer,
        scene,
        camera,
        retro,
        render_pipeline,
        controls,
    }
}

pub fn animate(app: &mut App) {
    // `controls.update();`
    app.controls.update(&mut app.camera.borrow_mut(), None);

    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`.
pub fn resize(app: &mut App, width: f64, height: f64) {
    {
        let mut camera = app.camera.borrow_mut();
        camera.aspect = width / height;
        camera.update_projection_matrix();
    }
    app.renderer.set_size(width, height);
}

/// The example's controls, for a host that has a pointer.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, which every one of the controls'
/// event handlers needs.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_retro.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
