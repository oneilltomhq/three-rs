//! Port of `three.js/examples/webgpu_postprocessing_ssr_denoise.html`,
//! calling the three-rs API in the same order the page's `init()` /
//! `animate()` do.
//!
//! The Warkarma dungeon, lit by a shadowed sun and the quarry HDR, drawn into
//! an `output` + `diffuseColor` + `normal` + `velocity` MRT. Stochastic
//! [`ssr`] traces one jittered ray per pixel, [`temporal_reproject`]
//! reprojects the denoised history onto it, [`recurrent_denoise`] filters
//! the result and feeds it back, both as the next frame's history and as
//! the SSR's own multi-bounce input. The denoised reflections are added to
//! the beauty, graded (AgX, contrast, saturation, gamma) and resolved by
//! [`traa`] and [`sharpen`].
//!
//! **There is no rung.** three lists `webgpu_postprocessing_ssr_denoise` in
//! `test/e2e/puppeteer.js`'s exception list, so it has no reference
//! screenshot to grade against. What the port checks instead is the floor
//! material, the SSR, denoise, sharpen and grading shaders, against three's
//! dump in `tests/nodes_display_wgsl.rs`, and the whole chain over frames in
//! `tests/ssr_denoise_frames.rs`.
//!
//! **The lighting patch.** The page replaces
//! `PhysicalLightingModel.prototype.indirectSpecular` so that the
//! environment's radiance (and clearcoat radiance) is zero for every PBR
//! material: SSR supplies the specular reflections instead. The port sets
//! [`environment_specular`](three_rs::MeshBasicNodeMaterial::environment_specular)
//! to `false` on every material the model brings, which is the same switch
//! scoped to those materials (`docs/nodes.md` §89).
//!
//! Differences from the page:
//!
//! - The model and the HDR load synchronously, where the callbacks would
//!   run; three's first frames, before they arrive, are not reproduced.
//! - `updateOutputNode()` rebuilds `applyPostProcessing( combinedOutputNode )`
//!   at the end of `init()`, so the page's first TRAA / sharpen pair is never
//!   rendered. The port builds the pair once.
//! - `directionalLight.shadow.autoUpdate = false` has no counterpart: the
//!   shadow map is re-rendered every frame, to the same result.
//! - The GUI and the compare modes are not ported; the defaults are, through
//!   the page's own `applyParams()` and `applyPost()`.
//!
//! The frame `main()` writes is the page's first: one stochastic ray per
//! pixel, before the history has had time to converge.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::controls::OrbitControls;
use three_rs::loaders::{GltfLoader, HdrLoader};
use three_rs::materials::{render_output, ToneMapping};
use three_rs::nodes::display::{
    convert_to_texture, recurrent_denoise, sharpen, ssr, temporal_reproject, traa,
    DenoiseAlphaSource, DenoiseMode, RecurrentDenoiseNode, RecurrentDenoiseOptions, RttNode,
    SampleFn, SharpenNode, SsrNode, SsrOptions, TemporalReprojectMode, TemporalReprojectNode,
    TemporalReprojectOptions, TraaNode,
};
use three_rs::nodes::mrt;
use three_rs::nodes::node::SettableValue;
use three_rs::nodes::pmrem_node::PmremEnvironment;
use three_rs::nodes::tsl::{
    diffuse_color, float, material_metalness_value, material_roughness_value, normal_view,
    output_property, pack_normal_to_rgb, saturation, texture_uv, to_var, uniform_settable,
    vec2_join, vec4_join,
};
use three_rs::nodes::velocity::velocity;
use three_rs::nodes::Type;
use three_rs::objects::Background;
use three_rs::renderer::cube_render_target;
use three_rs::textures::TextureType;
use three_rs::{
    pass, Color, DirectionalLight, PassNode, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

/// `window.innerWidth` of three's e2e harness.
pub const INNER_WIDTH: f64 = 800.0;
/// `window.innerHeight` of three's e2e harness.
pub const INNER_HEIGHT: f64 = 500.0;
/// The page never calls `renderer.setPixelRatio()`, so the canvas is at CSS
/// size.
pub const DPR: f64 = 1.0;

/// `params.ssr`, the GUI's starting values. `resolutionScale` is 1, the
/// SSR's own scale, so it is not listed.
pub struct SsrParams {
    /// `params.ssr.quality`.
    pub quality: f64,
    /// `params.ssr.mirrorBias`.
    pub mirror_bias: f64,
    /// `params.ssr.maxDistance`.
    pub max_distance: f64,
    /// `params.ssr.intensity`.
    pub intensity: f64,
    /// `params.ssr.thickness`.
    pub thickness: f64,
    /// `params.ssr.maxLuminance`.
    pub max_luminance: f64,
    /// `params.ssr.binaryRefine`.
    pub binary_refine: bool,
    /// `params.ssr.stepExponent`.
    pub step_exponent: f64,
    /// `params.ssr.envImportanceSampling`.
    pub env_importance_sampling: bool,
    /// `params.ssr.screenEdgeFade`.
    pub screen_edge_fade: f64,
    /// `params.ssr.screenEdgeFadeBlack`.
    pub screen_edge_fade_black: bool,
    /// `params.ssr.environmentIntensity`.
    pub environment_intensity: f64,
}

/// `params.temporalReproject`.
pub struct TemporalReprojectParams {
    /// `params.temporalReproject.maxFrames`.
    pub max_frames: f64,
    /// `params.temporalReproject.clampIntensity`.
    pub clamp_intensity: f64,
    /// `params.temporalReproject.flickerSuppression`.
    pub flicker_suppression: f64,
    /// `params.temporalReproject.hitPointReprojection`.
    pub hit_point_reprojection: bool,
}

/// `params.denoise`.
pub struct DenoiseParams {
    /// `params.denoise.enabled`.
    pub enabled: bool,
    /// `params.denoise.lumaPhi`.
    pub luma_phi: f64,
    /// `params.denoise.depthPhi`.
    pub depth_phi: f64,
    /// `params.denoise.normalPhi`.
    pub normal_phi: f64,
    /// `params.denoise.roughnessPhi`.
    pub roughness_phi: f64,
    /// `params.denoise.radius`.
    pub radius: f64,
    /// `params.denoise.alphaPhi`.
    pub alpha_phi: f64,
    /// `params.denoise.strength`.
    pub strength: f64,
    /// `params.denoise.adapt`.
    pub adapt: f64,
    /// `params.denoise.smoothDisocclusions`.
    pub smooth_disocclusions: bool,
    /// `params.denoise.flickerSuppression`.
    pub flicker_suppression: f64,
    /// `params.denoise.adaptiveTrust`.
    pub adaptive_trust: f64,
}

/// `params.post.grading`. `toneMapping` is always `'AgX'`.
pub struct GradingParams {
    /// `params.post.grading.exposure`.
    pub exposure: f64,
    /// `params.post.grading.gamma`.
    pub gamma: f64,
    /// `params.post.grading.contrast`.
    pub contrast: f64,
    /// `params.post.grading.saturation`.
    pub saturation: f64,
}

/// The page's `params`. `output` is 0 (`Combined`), the only output ported.
pub struct Params {
    /// `params.roughness`.
    pub roughness: f64,
    /// `params.ssr`.
    pub ssr: SsrParams,
    /// `params.temporalReproject`.
    pub temporal_reproject: TemporalReprojectParams,
    /// `params.denoise`.
    pub denoise: DenoiseParams,
    /// `params.grading`.
    pub grading: GradingParams,
}

/// The page's `params`, as the GUI starts.
// `environmentIntensity: 3.14` is the page's literal, not π.
#[allow(clippy::approx_constant)]
pub const PARAMS: Params = Params {
    roughness: 0.3,
    ssr: SsrParams {
        quality: 0.25,
        mirror_bias: 0.5,
        max_distance: 0.4,
        intensity: 1.0,
        thickness: 0.1,
        max_luminance: 35.0,
        binary_refine: false,
        step_exponent: 3.0,
        env_importance_sampling: false,
        screen_edge_fade: 0.2,
        screen_edge_fade_black: false,
        environment_intensity: 3.14,
    },
    temporal_reproject: TemporalReprojectParams {
        max_frames: 16.0,
        clamp_intensity: 0.25,
        flicker_suppression: 1.0,
        hit_point_reprojection: true,
    },
    denoise: DenoiseParams {
        enabled: true,
        luma_phi: 0.75,
        depth_phi: 20.0,
        normal_phi: 0.3,
        roughness_phi: 100.0,
        radius: 1.5,
        alpha_phi: 5.0,
        strength: 0.725,
        adapt: 0.5,
        smooth_disocclusions: true,
        flicker_suppression: 1.0,
        adaptive_trust: 1.0,
    },
    grading: GradingParams {
        exposure: 1.57,
        gamma: 0.89,
        contrast: 1.31,
        saturation: 1.0,
    },
};

fn examples_dir() -> std::path::PathBuf {
    three_rs::testing::three_js_dir().join("examples")
}

fn flag(value: bool) -> Vec<f64> {
    vec![if value { 1.0 } else { 0.0 }]
}

/// Everything `init()` builds and `animate()` uses.
pub struct App {
    /// The page's `WebGPURenderer`.
    pub renderer: Renderer,
    /// Shared with `scene_pass`, which renders it from `updateBefore()`.
    pub scene: Rc<RefCell<Scene>>,
    /// Shared with the pass and every node of the chain; `traa_node`
    /// jitters it.
    pub camera: Rc<RefCell<PerspectiveCamera>>,
    /// The page's `OrbitControls`.
    pub controls: OrbitControls,
    /// `scene.environment`, the HDR PMREM-filtered.
    pub environment: PmremEnvironment,
    /// `scenePass`, the MRT scene pass.
    pub scene_pass: PassNode,
    /// `ssrNode`, the stochastic SSR.
    pub ssr_node: SsrNode,
    /// `temporalReprojectNode`, in `'specular'` mode.
    pub temporal_reproject_node: TemporalReprojectNode,
    /// `denoiseNode`, the recurrent denoiser, fed back as both histories.
    pub denoise_node: RecurrentDenoiseNode,
    /// `applyGrading( combinedOutputNode )` as a texture, TRAA's beauty.
    pub graded: RttNode,
    /// `traaNode`, which owns the view offset.
    pub traa_node: TraaNode,
    /// `sharpenNode`, the output node.
    pub sharpen_node: SharpenNode,
    /// `gammaUniform`.
    pub gamma: SettableValue,
    /// `contrastUniform`.
    pub contrast: SettableValue,
    /// `saturationUniform`.
    pub saturation: SettableValue,
    /// `renderPipeline`.
    pub render_pipeline: RenderPipeline,
}

/// `applyGrading( source )`: AgX through `renderOutput()`, then contrast
/// about mid-grey, saturation and gamma.
#[inline(never)]
fn apply_grading(
    source: three_rs::nodes::NodeRef,
    contrast: three_rs::nodes::NodeRef,
    saturation_adjustment: three_rs::nodes::NodeRef,
    gamma: three_rs::nodes::NodeRef,
) -> three_rs::nodes::NodeRef {
    let rgb = source.rgb();
    let rgb = render_output(vec4_join(vec![rgb, float(1.0)]), ToneMapping::AgX).rgb();
    let rgb = rgb.sub(0.5).mul(contrast).add(0.5);
    let rgb = saturation(rgb, saturation_adjustment);
    let rgb = rgb.max(0.0).pow(float(1.0).div(gamma));
    vec4_join(vec![rgb, float(1.0)])
}

/// The page's `init()`.
pub fn init() -> App {
    let mut camera = PerspectiveCamera::new(35.0, INNER_WIDTH / INNER_HEIGHT, 0.1, 8.0);
    camera.node.borrow_mut().position.set(
        0.9210513838983053,
        0.16195074025403253,
        0.431687274895316,
    );

    let mut scene = Scene::new();

    // `loader.load( 'models/gltf/dungeon_warkarma.glb', … )`. The callback
    // scales the model down, makes everything cast and receive shadows,
    // flattens the roughness and drops the normal maps. The port also turns
    // off each material's environment specular: the page's
    // `indirectSpecular` patch (see the module docs).
    let gltf = GltfLoader::load(examples_dir().join("models/gltf/dungeon_warkarma.glb"))
        .expect("dungeon_warkarma.glb");
    gltf.scene.borrow_mut().scale.multiply_scalar(0.1);
    gltf.scene.traverse(&mut |child| {
        let mut object = child.borrow_mut();
        let Some(mesh) = object.mesh_mut() else {
            return;
        };
        if mesh.material.is_none() && mesh.materials.is_empty() {
            return;
        }
        for material in mesh.material.iter_mut().chain(mesh.materials.iter_mut()) {
            material.roughness = PARAMS.roughness;
            material.normal_map = None;
            material.environment_specular = false;
        }
        object.cast_shadow = true;
        object.receive_shadow = true;
    });
    scene.add(&gltf.scene);

    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(DPR);
    renderer.set_size(INNER_WIDTH, INNER_HEIGHT);
    renderer.tone_mapping = ToneMapping::AgX;
    renderer.tone_mapping_exposure = PARAMS.grading.exposure;
    renderer.shadow_map_enabled = true;

    // `new HDRLoader().setPath( 'textures/equirectangular/' ).loadAsync(
    // 'quarry_01_1k.hdr' )`, the background, the environment and (below) the
    // SSR's fallback for rays that leave the screen.
    let hdr_texture = HdrLoader::new()
        .load(examples_dir().join("textures/equirectangular/quarry_01_1k.hdr"))
        .expect("quarry_01_1k.hdr");
    hdr_texture.set_generate_mipmaps(true);
    let background = cube_render_target::from_equirectangular_texture(&mut renderer, &hdr_texture)
        .expect("the equirectangular background converts to a cube");
    scene.background = Some(Background::CubeTexture(background));
    let mut environment = PmremEnvironment::from_equirectangular(&hdr_texture);
    environment.update(&mut renderer).unwrap();
    scene.environment = Some(environment.handle());

    let directional_light = DirectionalLight::new(Color::from_hex(0xffffff), 20.0);
    {
        let mut object = directional_light.borrow_mut();
        object.position.set(-10.9, 2.2, 10.75);
        object.cast_shadow = true;
        let shadow = object.light_mut().unwrap().shadow.as_mut().unwrap();
        shadow.map_size.x = 4096.0;
        shadow.map_size.y = 4096.0;
        shadow.camera.set_bounds(-1.75, 1.75, 1.75, -1.75);
        shadow.camera.set_near(0.1);
        shadow.camera.set_far(50.0);
        shadow.bias = -0.0005;
    }
    scene.add(&directional_light);

    scene.environment_intensity = 1.0;

    // The controls are made before the pipeline here, where the camera is
    // still unshared; nothing in between reads its orientation. Then the
    // page's initial camera transform: `animate()`'s first
    // `controls.update()` turns the camera to the new target, so the
    // rotation set here is overridden, as on the page.
    let mut controls = OrbitControls::new(&mut camera);
    controls.set_element_size(INNER_WIDTH, INNER_HEIGHT);
    controls.enable_damping = true;
    controls.update(&mut camera, None);
    {
        let mut node = camera.node.borrow_mut();
        node.position
            .set(1.259878548682251, 0.5391287340899181, -0.27217301481427114);
        node.set_rotation(
            -0.3158233106804791,
            0.26820684188431526,
            0.08637696823742165,
        );
    }
    controls
        .target
        .set(1.0258536154689288, 0.2746440590977971, -1.0815876858987743);

    let scene = Rc::new(RefCell::new(scene));
    let camera = Rc::new(RefCell::new(camera));

    let mut render_pipeline = RenderPipeline::new();

    let scene_pass = pass(scene.clone(), camera.clone());
    let mut scene_mrt = mrt(vec![("output", output_property())]);
    // `diffuseColor: vec4( diffuseColor.rgb, materialMetalness )` and
    // `normal: vec4( packNormalToRGB( normalView ).rgb, materialRoughness )`
    // read per-material values, so they are deferred (`docs/nodes.md` §23).
    scene_mrt.set_deferred("diffuseColor", || {
        vec4_join(vec![diffuse_color().rgb(), material_metalness_value()])
    });
    scene_mrt.set_deferred("normal", || {
        vec4_join(vec![
            pack_normal_to_rgb(normal_view()).rgb(),
            material_roughness_value(),
        ])
    });
    scene_mrt.set("velocity", velocity());
    scene_pass.set_mrt(scene_mrt);

    // `getTextureNode()` in the page's order, which is the attachments'
    // order; `toInspector()` returns its node unchanged.
    let scene_pass_color = scene_pass.texture_node("output");
    let scene_pass_normal = scene_pass.texture_node("normal");
    let _ = scene_pass.texture_node("depth");
    let _ = scene_pass.texture_node("velocity");
    let scene_pass_diffuse = scene_pass.texture_node("diffuseColor");

    let normal_texture = scene_pass.texture_named("normal");
    normal_texture.set_texture_type(TextureType::UnsignedByte);
    let diffuse_texture = scene_pass.texture_named("diffuseColor");
    diffuse_texture.set_texture_type(TextureType::UnsignedByte);
    let velocity_texture = scene_pass.texture_named("velocity");
    let depth_texture = scene_pass.depth_texture();

    // `sample( ( uv ) => unpackRGBToNormal( scenePassNormal.sample( uv ).rgb ) )`.
    let scene_normal: SampleFn = {
        let normal_texture = normal_texture.clone();
        Rc::new(move |coord| texture_uv(&normal_texture, coord).rgb().mul(2.0).sub(1.0))
    };
    // Metalness in `diffuseColor.a`, roughness in `normal.a`.
    let scene_metal_rough: SampleFn = {
        let (diffuse_texture, normal_texture) = (diffuse_texture.clone(), normal_texture.clone());
        Rc::new(move |coord: three_rs::nodes::NodeRef| {
            vec2_join(vec![
                texture_uv(&diffuse_texture, coord.clone()).w(),
                texture_uv(&normal_texture, coord).w(),
            ])
        })
    };
    let scene_diffuse: SampleFn = {
        let diffuse_texture = diffuse_texture.clone();
        Rc::new(move |coord| texture_uv(&diffuse_texture, coord))
    };
    // The packed normal, as `temporalReproject` and `recurrentDenoise` take
    // `scenePassNormal`.
    let scene_normal_packed: SampleFn = {
        let normal_texture = normal_texture.clone();
        Rc::new(move |coord| texture_uv(&normal_texture, coord))
    };

    let ssr_node = ssr(
        &scene_pass.texture(),
        &depth_texture,
        scene_normal,
        SsrOptions::new(scene_pass_diffuse.w(), Some(scene_pass_normal.w()))
            .with_stochastic(true)
            .with_diffuse(scene_diffuse)
            .with_environment(&hdr_texture)
            .with_env_importance_sampling(PARAMS.ssr.env_importance_sampling)
            .with_binary_refine(PARAMS.ssr.binary_refine),
        camera.clone(),
    );

    let temporal_reproject_node = temporal_reproject(
        &ssr_node.render_target().texture(),
        &depth_texture,
        &normal_texture,
        &velocity_texture,
        camera.clone(),
        TemporalReprojectOptions {
            mode: TemporalReprojectMode::Specular,
            accumulate: false,
            ..TemporalReprojectOptions::default()
        },
    );

    let denoise_node = recurrent_denoise(
        &temporal_reproject_node.texture(),
        camera.clone(),
        RecurrentDenoiseOptions {
            depth: Some(depth_texture.clone()),
            normal: Some(scene_normal_packed),
            raw: Some(ssr_node.render_target().texture()),
            metal_roughness: Some(scene_metal_rough),
            mode: DenoiseMode::Specular,
            accumulate: true,
            ..RecurrentDenoiseOptions::default()
        },
    );
    // SSR alpha channel contains ray length.
    denoise_node.set_alpha_source(DenoiseAlphaSource::RayLength);

    // Feed the denoised result and velocity back into SSR for multi-bounce
    // reflections, and into the reprojection as its history.
    ssr_node.set_history(&denoise_node.texture(), &velocity_texture);
    temporal_reproject_node.set_history_texture(Some(&denoise_node.texture()));

    // `vec4( denoiseNode.rgb, ssrNode.a.greaterThan( 0 ).toVar() )`.
    let denoise_pass_blend = vec4_join(vec![
        denoise_node.node().rgb(),
        to_var(None, ssr_node.node().w().greater_than(0.0)),
    ]);

    let (gamma_uniform, gamma) = uniform_settable(Type::F32, vec![PARAMS.grading.gamma]);
    let (contrast_uniform, contrast) = uniform_settable(Type::F32, vec![PARAMS.grading.contrast]);
    let (saturation_uniform, saturation) =
        uniform_settable(Type::F32, vec![PARAMS.grading.saturation]);

    let lit_color = scene_pass_color.rgb().add(denoise_pass_blend.rgb());
    let combined_output_node = vec4_join(vec![lit_color, float(1.0)]);

    // `applyPostProcessing( source )`: `sharpen( traa( applyGrading( source ),
    // scenePassDepth, scenePassVelocity, camera ), 0 )`. TRAA's beauty is a
    // texture here, so `convertToTexture()` is explicit.
    let graded = convert_to_texture(apply_grading(
        combined_output_node,
        contrast_uniform,
        saturation_uniform,
        gamma_uniform,
    ));
    let traa_node = traa(
        &graded.texture(),
        &depth_texture,
        &velocity_texture,
        camera.clone(),
    );
    // TRAA claims the pipeline's view offset first, as it does on the page,
    // where its setup runs before the reprojection's; the reprojection's
    // hooks then stand down and the velocity reads TRAA's jitter.
    traa_node.attach(&mut render_pipeline);
    temporal_reproject_node.attach(&mut render_pipeline);
    let sharpen_node = sharpen(traa_node.node(), 0.0, false);

    render_pipeline.output_node = Some(sharpen_node.node());
    render_pipeline.output_color_transform = false;

    let app = App {
        renderer,
        scene,
        camera,
        controls,
        environment,
        scene_pass,
        ssr_node,
        temporal_reproject_node,
        denoise_node,
        graded,
        traa_node,
        sharpen_node,
        gamma,
        contrast,
        saturation,
        render_pipeline,
    };
    let mut app = app;
    apply_params(&app);
    apply_post(&mut app);
    app
}

/// The page's `applyParams()`, with `params` as the GUI leaves them.
pub fn apply_params(app: &App) {
    let ssr = &PARAMS.ssr;
    let node = &app.ssr_node;
    node.quality().set(vec![ssr.quality]);
    node.mirror_bias().set(vec![ssr.mirror_bias]);
    node.set_step_exponent(ssr.step_exponent);
    node.set_binary_refine(ssr.binary_refine);
    node.max_distance().set(vec![ssr.max_distance]);
    node.intensity().set(vec![ssr.intensity]);
    node.thickness().set(vec![ssr.thickness]);
    node.max_luminance().set(vec![ssr.max_luminance]);
    node.screen_edge_fade().set(vec![ssr.screen_edge_fade]);
    node.set_screen_edge_fade_black(ssr.screen_edge_fade_black);
    node.environment_intensity()
        .set(vec![ssr.environment_intensity]);

    let reproject = &PARAMS.temporal_reproject;
    let node = &app.temporal_reproject_node;
    node.max_frames().set(vec![reproject.max_frames]);
    node.clamp_intensity().set(vec![reproject.clamp_intensity]);
    node.flicker_suppression()
        .set(vec![reproject.flicker_suppression]);
    node.hit_point_reprojection()
        .set(flag(reproject.hit_point_reprojection));

    let denoise = &PARAMS.denoise;
    let node = &app.denoise_node;
    node.luma_phi().set(vec![denoise.luma_phi]);
    node.depth_phi().set(vec![denoise.depth_phi]);
    node.normal_phi().set(vec![denoise.normal_phi]);
    node.roughness_phi().set(vec![denoise.roughness_phi]);
    node.radius()
        .set(vec![if denoise.enabled { denoise.radius } else { 0.0 }]);
    node.alpha_phi().set(vec![denoise.alpha_phi]);
    node.strength().set(vec![denoise.strength]);
    node.adapt().set(vec![denoise.adapt]);
    node.smooth_disocclusions()
        .set(flag(denoise.smooth_disocclusions));
    node.flicker_suppression()
        .set(vec![denoise.flicker_suppression]);
    node.adaptive_trust().set(vec![denoise.adaptive_trust]);
}

/// The page's `applyPost()`.
pub fn apply_post(app: &mut App) {
    let grading = &PARAMS.grading;
    app.gamma.set(vec![grading.gamma]);
    app.contrast.set(vec![grading.contrast]);
    app.saturation.set(vec![grading.saturation]);
    app.renderer.tone_mapping = ToneMapping::AgX;
    app.renderer.tone_mapping_exposure = grading.exposure;
}

/// The page's `animate()`.
pub fn animate(app: &mut App) {
    app.controls.update(&mut app.camera.borrow_mut(), None);
    app.render_pipeline.render(&mut app.renderer);
}

/// The page's `onWindowResize()`. Every history restarts at the new size on
/// the next frame.
pub fn resize(app: &mut App, width: f64, height: f64) {
    let mut camera = app.camera.borrow_mut();
    camera.aspect = width / height;
    camera.update_projection_matrix();
    app.renderer.set_size(width, height);
    app.controls.set_element_size(width, height);
}

/// The orbit controls, for a host delivering pointer events.
pub fn controls(app: &mut App) -> Option<&mut OrbitControls> {
    Some(&mut app.controls)
}

/// The controls and the camera at once, for a host delivering pointer events.
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
        .unwrap_or_else(|| "target/webgpu_postprocessing_ssr_denoise.png".to_string());
    three_rs::testing::write_png(&path, width, height, &pixels);
    println!("wrote {path} ({width}x{height})");
}
