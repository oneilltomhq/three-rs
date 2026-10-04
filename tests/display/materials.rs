//! The display-node quads of #144, built as the three.js pages that three's
//! dumps come from build them. Shared by `tests/nodes_display_wgsl.rs`, which
//! checks them against `tests/fixtures/nodes_display/`, and by
//! `examples/dump_wgsl.rs`, which prints them.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::loaders::LutCubeLoader;
use three_rs::materials::{quad_vertex_node, render_output, MeshBasicNodeMaterial};
use three_rs::nodes::display::convert_to_texture;
use three_rs::nodes::display::{
    after_image, anaglyph_pass, ao, barrel_uv, bayer_dither, bilateral_blur, bleach, box_blur,
    circle, color_bleeding, depth_aware_blend, dof, dot_screen, film, fxaa, gaussian_blur, godrays,
    hash_blur_with, lensflare, lut_3d, motion_blur, outline, parallax_barrier_pass,
    pixelation_pass, recurrent_denoise, retro_pass, rgb_shift, rtt, scanlines, sepia, smaa, sobel,
    ssgi, ssr, sss, temporal_reproject, traa, viewport_shared_texture_at, BoxBlurOptions,
    DenoiseAlphaSource, DenoiseMode, DepthAwareBlendOptions, EnvironmentLobe, Fsr1Node,
    GaussianBlurOptions, HashBlurOptions, ImportanceSampledEnvironment, LensflareParams,
    OutlineParams, RecurrentDenoiseOptions, RetroPassOptions, SampleFn, SharpenNode, SsrOptions,
    TemporalReprojectMode, TemporalReprojectOptions,
};
use three_rs::nodes::tsl::{
    bind_analytic_noise, d_gtr, distance, equirect_dir_pdf, equirect_uv_to_dir, f_schlick, float,
    geometry_term, get_specular_dominant_factor, ggx_reflection_sample, ggx_reflection_struct, int,
    mis_power_heuristic, osc_sine, pass_depth_texture, perspective_depth_to_view_z, posterize,
    replace_default_uv, screen_size, screen_uv, smith_g, struct_get, texture_3d_sampled,
    texture_uv, time, uniform_value, uv, vec2, vec2_join, vec3, vec3_join, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::{DepthTexture, MinFilter, Texture, TextureFilter};
use three_rs::{Color, Matrix4, PerspectiveCamera, PointLight, Scene, ToneMapping};

/// One quad: the name the gate reports it by, the three.js dump file it is
/// checked against, and the material.
pub struct DisplayQuad {
    pub label: &'static str,
    pub fixture: &'static str,
    pub material: MeshBasicNodeMaterial,
}

fn quad(
    label: &'static str,
    fixture: &'static str,
    fragment: three_rs::nodes::NodeRef,
) -> DisplayQuad {
    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(fragment);
    material.vertex_node = Some(quad_vertex_node());
    DisplayQuad {
        label,
        fixture,
        material,
    }
}

/// A half-float render target's texture, which is what three's
/// `convertToTexture()` / `PassNode` hand every one of these nodes.
fn input() -> Texture {
    Texture::render_target(256, 256, wgpu::TextureFormat::Rgba16Float)
}

pub fn display_quads() -> Vec<DisplayQuad> {
    let mut quads = Vec::new();

    // webgpu_procedural_texture `m02`: `gaussianBlur( proceduralToTexture,
    // uniform( .5 ), 20 )`. The vertical pass (`m03`) is the same module with
    // the other direction.
    let blur = gaussian_blur(
        &input(),
        Some(uniform_value(Type::F32, vec![0.5])),
        20,
        GaussianBlurOptions::default(),
    );
    let mut horizontal = blur.quad_materials()[0].clone();
    horizontal.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "gaussian_blur_horizontal",
        fixture: "webgpu_procedural_texture_m02_gaussian_blur_horizontal.wgsl",
        material: horizontal,
    });
    let mut vertical = blur.quad_materials()[1].clone();
    vertical.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "gaussian_blur_vertical",
        fixture: "webgpu_procedural_texture_m03_gaussian_blur_vertical.wgsl",
        material: vertical,
    });

    // webgpu_postprocessing_sobel `m18`: `sobel( renderOutput( scenePass ) )`
    // — the operator over the `RTT` of the render output.
    quads.push(quad(
        "sobel",
        "webgpu_postprocessing_sobel_m18_sobel.wgsl",
        sobel(&input()).node(),
    ));

    // webgpu_postprocessing `m03`: `dotScreen( scenePassColor, 1.57, 0.3 )`,
    // converted to a texture for the rgbShift after it.
    quads.push(quad(
        "dot_screen",
        "webgpu_postprocessing_m03_dot_screen.wgsl",
        dot_screen(texture_uv(&input(), uv()), float(1.57), float(0.3)),
    ));

    // …`m05`: `rgbShift( dotScreenPass, 0.001 )` as the `RenderPipeline`'s
    // output, so under `renderOutput()` with no tone mapping.
    quads.push(quad(
        "rgb_shift",
        "webgpu_postprocessing_m05_rgb_shift.wgsl",
        render_output(
            rgb_shift(&input(), float(0.001), float(0.0)),
            ToneMapping::None,
        ),
    ));

    // webgpu_postprocessing_fxaa `m05`: `fxaa( renderOutput( scenePass ) )`
    // as the `RenderPipeline`'s output with `outputColorTransform = false`,
    // so the quad's fragment is the node itself.
    quads.push(quad(
        "fxaa",
        "webgpu_postprocessing_fxaa_m05_fxaa.wgsl",
        fxaa(&input()).node(),
    ));

    // webgpu_postprocessing_afterimage `m04`: `afterImage( scenePass, 0.96 )`.
    let after = after_image(&input(), uniform_value(Type::F32, vec![0.96]));
    let mut after_quad = after.quad_material().clone();
    after_quad.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "after_image",
        fixture: "webgpu_postprocessing_afterimage_m04_after_image.wgsl",
        material: after_quad,
    });

    // webgpu_postprocessing_pixel `m09`: `pixelationPass( scene, camera, 6,
    // uniform( 0.3 ), uniform( 0.4 ) )` as the `RenderPipeline`'s output.
    let pixel = pixelation_pass(
        6,
        uniform_value(Type::F32, vec![0.3]),
        uniform_value(Type::F32, vec![0.4]),
    );
    quads.push(quad(
        "pixelation",
        "webgpu_postprocessing_pixel_m09_pixelation.wgsl",
        render_output(pixel.node(), ToneMapping::None),
    ));

    // webgpu_postprocessing_dof_basic `m23`: `boxBlur( scenePassColor, {
    // size: blurSize, separation: blurSpread } )`. Only its loops are
    // compared: the rest of that module is the depth-of-field mix.
    let mut options = BoxBlurOptions::default();
    options.size = uniform_value(Type::F32, vec![2.0]);
    options.separation = uniform_value(Type::F32, vec![4.0]);
    quads.push(quad(
        "box_blur",
        "webgpu_postprocessing_dof_basic_m23_box_blur.wgsl",
        box_blur(&input(), options),
    ));

    // webgpu_backdrop_area `m08`: `hashBlur( viewportSharedTexture(), .05 )`.
    // Again only the loop is compared: three dumps it inside the backdrop
    // material, `hashBlur`'s tap is the viewport texture's nearest load.
    quads.push(quad(
        "hash_blur",
        "webgpu_backdrop_area_m08_hash_blur.wgsl",
        hash_blur_with(
            viewport_shared_texture_at,
            screen_uv(),
            float(0.05),
            HashBlurOptions::default(),
        ),
    ));

    // webgpu_postprocessing_motion_blur `m14`: the page's whole output node,
    // `motionBlur( beauty, velocity.mul( blurAmount ) )` under a vignette, as
    // the `RenderPipeline`'s output with the sRGB output transform. The two
    // inputs are the scene pass's `output` and `velocity` attachments.
    let beauty = input();
    let velocity = texture_uv(&input(), uv()).mul(uniform_value(Type::F32, vec![1.0]));
    let m_blur = motion_blur(&beauty, velocity, 16);
    let vignette = distance(screen_uv(), float(0.5))
        .remap(0.6, 1.0, 0.0, 1.0)
        .mul(2.0)
        .clamp(0.0, 1.0)
        .one_minus();
    quads.push(quad(
        "motion_blur",
        "webgpu_postprocessing_motion_blur_m14_motion_blur.wgsl",
        render_output(
            vec4_join(vec![m_blur.mul(vignette).xyz(), m_blur.w()]),
            ToneMapping::None,
        ),
    ));

    // webgpu_postprocessing_traa `m05`: `traa( scenePass.getTextureNode(
    // 'output' ), …( 'depth' ), …( 'velocity' ), camera )`'s resolve quad,
    // `TRAA.resolve`, whose `colorNode` is the whole resolve.
    let traa = traa(
        &input(),
        &DepthTexture::new(),
        &input(),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            70.0, 1.0, 0.1, 10.0,
        ))),
    );
    let mut resolve = traa.quad_material();
    resolve.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "traa",
        fixture: "webgpu_postprocessing_traa_m05_traa_resolve.wgsl",
        material: resolve,
    });

    // webgpu_display_stereo `m06` with the effect set to Anaglyph:
    // `anaglyphPass( scene, camera )`'s composite quad, which mixes the two
    // eye targets with the default Dubois red/cyan matrices. `stereoPass` has
    // no quad of its own (it renders both eyes straight into its target).
    let stereo_camera = || {
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            60.0, 1.6, 0.1, 100.0,
        )))
    };
    let stereo_scene = || std::rc::Rc::new(std::cell::RefCell::new(three_rs::Scene::new()));
    let mut anaglyph = anaglyph_pass(stereo_scene(), stereo_camera()).quad_material();
    anaglyph.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "anaglyph",
        fixture: "webgpu_display_stereo_anaglyph_m06_anaglyph.wgsl",
        material: anaglyph,
    });

    // webgpu_display_stereo `m06` with the effect set to ParallaxBarrier:
    // `parallaxBarrierPass( scene, camera )`'s interleaving quad.
    let mut barrier = parallax_barrier_pass(stereo_scene(), stereo_camera()).quad_material();
    barrier.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "parallax_barrier",
        fixture: "webgpu_display_stereo_parallax_barrier_m06_parallax_barrier.wgsl",
        material: barrier,
    });

    // webgpu_postprocessing_3dlut `m06`: `lut3D( renderOutput( scenePass ),
    // texture3D( lut.texture3D ), lut.texture3D.image.width, uniform( 1 ) )`
    // as the `RenderPipeline`'s output with `outputColorTransform = false`.
    // The table is a 2³ identity `.CUBE`; its size is a uniform either way.
    let identity = (0..8)
        .map(|i| format!("{} {} {}\n", i & 1, (i >> 1) & 1, i >> 2))
        .collect::<String>();
    let lut = LutCubeLoader::new()
        .parse(&format!("LUT_3D_SIZE 2\n{identity}"))
        .expect("a 2³ table");
    quads.push(quad(
        "lut_3d",
        "webgpu_postprocessing_3dlut_m06_lut_3d.wgsl",
        lut_3d(
            render_output(texture_uv(&input(), uv()), ToneMapping::None),
            &texture_3d_sampled(&lut.texture_3d),
            f64::from(lut.size),
            uniform_value(Type::F32, vec![1.0]),
        )
        .node(),
    ));

    // tools/dump-pages/outline_selected.html — webgpu_postprocessing_outline
    // with the torus selected before the first frame — `m01` to `m11`:
    // `OutlineNode`'s depth and prepare-mask scene materials and its quads,
    // with the page's `edgeGlow` and `edgeThickness` uniforms. The sprite
    // variants (`m02`, `m04`) are not in the dump: the page has no sprites.
    // The X and Y draws of each blur are one module in three; the port's Y
    // materials differ only in the texture and the baked direction.
    let outline_pass = outline(
        Rc::new(RefCell::new(Scene::new())),
        Rc::new(RefCell::new(PerspectiveCamera::new(45.0, 1.6, 0.1, 100.0))),
        OutlineParams {
            selected_objects: Vec::new(),
            edge_glow: uniform_value(Type::F32, vec![0.0]),
            edge_thickness: uniform_value(Type::F32, vec![1.0]),
            ..OutlineParams::default()
        },
    );
    let materials = outline_pass.materials();
    for (label, fixture, material, is_quad) in [
        (
            "outline_depth",
            "outline_selected_m01_outline_depth.wgsl",
            materials[0],
            false,
        ),
        (
            "outline_prepare_mask",
            "outline_selected_m03_outline_prepare_mask.wgsl",
            materials[2],
            false,
        ),
        (
            "outline_copy",
            "outline_selected_m05_outline_copy.wgsl",
            materials[4],
            true,
        ),
        (
            "outline_edge_detection",
            "outline_selected_m07_outline_edge_detection.wgsl",
            materials[5],
            true,
        ),
        (
            "outline_separable_blur",
            "outline_selected_m09_outline_separable_blur.wgsl",
            materials[6],
            true,
        ),
        (
            "outline_separable_blur2",
            "outline_selected_m10_outline_separable_blur2.wgsl",
            materials[8],
            true,
        ),
        (
            "outline_composite",
            "outline_selected_m11_outline_composite.wgsl",
            materials[10],
            true,
        ),
    ] {
        let mut material = material.clone();
        if is_quad {
            material.vertex_node = Some(quad_vertex_node());
        }
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // webgpu_postprocessing_sss `m08`: `sss( prePassDepth, camera, dirLight
    // )`'s quad, `SSS`. The page turns temporal filtering on before its first
    // frame, and three bakes the frame id the material is built on into the
    // shader: 2 in the dump.
    let sss_node = sss(
        &DepthTexture::new(),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            45.0, 1.0, 0.1, 100.0,
        ))),
        &three_rs::DirectionalLight::new(three_rs::Color::from_hex(0xffffff), 3.0),
    );
    let mut sss_quad = sss_node.quad_material(2);
    sss_quad.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "sss",
        fixture: "webgpu_postprocessing_sss_m08_sss.wgsl",
        material: sss_quad,
    });

    // webgpu_postprocessing_ao `m18`: `ao( prePassDepth, prePassNormal,
    // camera )`'s quad, `GTAO`, at the page's 16 samples (three slices of
    // six steps). The depth and normal are the pre-pass's attachments.
    let gtao = ao(
        &DepthTexture::new(),
        &input(),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            45.0, 1.0, 0.1, 50.0,
        ))),
    );
    let mut gtao_quad = gtao.quad_material();
    gtao_quad.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "gtao",
        fixture: "webgpu_postprocessing_ao_m18_gtao.wgsl",
        material: gtao_quad,
    });

    // webgpu_postprocessing_godrays `m09`: `godrays( scenePassDepth, camera,
    // pointLight )`'s raymarch quad, over the shadow-casting point light's
    // cube depth map.
    let camera = std::rc::Rc::new(std::cell::RefCell::new(PerspectiveCamera::new(
        60.0, 1.6, 0.1, 1000.0,
    )));
    let point_light = PointLight::new(Color::from_hex(0xf6287d), 10000.0, 0.0);
    point_light.borrow_mut().cast_shadow = true;
    let scene_depth = DepthTexture::new();
    let godrays_pass = godrays(&scene_depth, camera.clone(), &point_light);
    let mut raymarch = godrays_pass.quad_material().clone();
    raymarch.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "godrays",
        fixture: "webgpu_postprocessing_godrays_m09_godrays.wgsl",
        material: raymarch,
    });

    // …`m11`: `bilateralBlur( godraysPassColor, 4, 0.1 )`. Three caches one
    // module for both directions — the direction is a uniform — so `m11` is
    // the horizontal and vertical pass alike.
    let bilateral = bilateral_blur(&godrays_pass.texture(), None, 4, 0.1);
    for (label, material) in ["bilateral_blur_horizontal", "bilateral_blur_vertical"]
        .into_iter()
        .zip(bilateral.quad_materials())
    {
        let mut material = material.clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture: "webgpu_postprocessing_godrays_m11_bilateral_blur.wgsl",
            material,
        });
    }

    // …`m13`: the `RenderPipeline`'s output, `depthAwareBlend(
    // scenePassColor, blurPassColor, scenePassDepth, camera, { blendColor,
    // edgeRadius, edgeStrength } )` under `renderOutput()` with no tone
    // mapping.
    let mut options = DepthAwareBlendOptions::default();
    options.blend_color = uniform_value(Type::Vec3, vec![1.0, 0.0, 0.5]);
    options.edge_radius = uniform_value(Type::F32, vec![2.0]);
    options.edge_strength = uniform_value(Type::F32, vec![2.0]);
    quads.push(quad(
        "depth_aware_blend",
        "webgpu_postprocessing_godrays_m13_depth_aware_blend.wgsl",
        render_output(
            depth_aware_blend(
                &input(),
                &bilateral.texture(),
                &scene_depth,
                &camera,
                options,
            ),
            ToneMapping::None,
        ),
    ));

    // webgpu_postprocessing_lensflare `m24`: `convertToTexture( bloomPass )`,
    // the `RTTNode` quad — the input sampled and written out as it is.
    let bloom_texture = rtt(texture_uv(&input(), uv()));
    let mut copy = bloom_texture.quad_material().clone();
    copy.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "rtt",
        fixture: "webgpu_postprocessing_lensflare_m24_rtt.wgsl",
        material: copy,
    });

    // …`m25`: `lensflare( bloomPass, { threshold, ghostAttenuationFactor,
    // ghostSpacing } )`, the three overridden params uniforms.
    let mut params = LensflareParams::default();
    params.threshold = uniform_value(Type::F32, vec![0.5]);
    params.ghost_attenuation_factor = uniform_value(Type::F32, vec![25.0]);
    params.ghost_spacing = uniform_value(Type::F32, vec![0.25]);
    let flare = lensflare(&bloom_texture.texture(), params);
    let mut ghosts = flare.quad_material().clone();
    ghosts.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "lensflare",
        fixture: "webgpu_postprocessing_lensflare_m25_lensflare.wgsl",
        material: ghosts,
    });

    // …`m26`/`m27`: `gaussianBlur( flarePass, 8 )` — the plain number `8`,
    // which `vec2( directionNode )` makes the constant `vec2( 8, 8 )`, and the
    // default sigma of 4, so not the procedural_texture modules.
    let flare_blur = gaussian_blur(
        &input(),
        Some(vec2(8.0, 8.0)),
        4,
        GaussianBlurOptions::default(),
    );
    for ((label, fixture), material) in [
        (
            "lensflare_gaussian_blur_horizontal",
            "webgpu_postprocessing_lensflare_m26_gaussian_blur_horizontal.wgsl",
        ),
        (
            "lensflare_gaussian_blur_vertical",
            "webgpu_postprocessing_lensflare_m27_gaussian_blur_vertical.wgsl",
        ),
    ]
    .into_iter()
    .zip(flare_blur.quad_materials())
    {
        let mut material = material.clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // …`m29`: the `RenderPipeline`'s output, `outputPass.add( bloomPass
    // ).add( blurPass )` under `renderOutput()` with ACES filmic.
    quads.push(quad(
        "lensflare_composite",
        "webgpu_postprocessing_lensflare_m29_composite.wgsl",
        render_output(
            texture_uv(&input(), uv())
                .add(texture_uv(&input(), uv()))
                .add(texture_uv(&input(), uv())),
            ToneMapping::AcesFilmic,
        ),
    ));

    // webgpu_postprocessing_dof: `dof( scenePassColor, scenePassViewZ,
    // uniform( 500 ), uniform( 200 ), uniform( 10 ) )`'s quads, in draw order
    // (`m04`, the near field's `gaussianBlur( _CoCTextureNode, 1, 2 )`
    // horizontal pass `m06` — `float` taps through the CoC texture's uv
    // matrix, summed into a `vec4` splat — and its vertical pass `m07`, which
    // reads the horizontal pass's RGBA target: `vec4` taps, no uv matrix, a
    // `( 0, 1 )` direction; then `m08`, `m10`, `m11`, `m12`;
    // `m10` is the blur64 module both fields share). The viewZ is the scene pass's
    // `perspectiveDepthToViewZ( depth, near, far )`.
    let view_z = perspective_depth_to_view_z(
        pass_depth_texture(&DepthTexture::new()),
        uniform_value(Type::F32, vec![1.0]),
        uniform_value(Type::F32, vec![3500.0]),
    );
    let dof = dof(
        &input(),
        view_z,
        uniform_value(Type::F32, vec![500.0]),
        uniform_value(Type::F32, vec![200.0]),
        uniform_value(Type::F32, vec![10.0]),
    );
    let dof_quads = [
        ("dof_coc", "webgpu_postprocessing_dof_m04_coc.wgsl", 0),
        (
            "dof_coc_gaussian_horizontal",
            "webgpu_postprocessing_dof_m06_coc_gaussian_horizontal.wgsl",
            1,
        ),
        (
            "dof_coc_gaussian_vertical",
            "webgpu_postprocessing_dof_m07_coc_gaussian_vertical.wgsl",
            2,
        ),
        (
            "dof_coc_blurred",
            "webgpu_postprocessing_dof_m08_coc_blurred.wgsl",
            3,
        ),
        ("dof_blur64", "webgpu_postprocessing_dof_m10_blur64.wgsl", 4),
        ("dof_blur16", "webgpu_postprocessing_dof_m11_blur16.wgsl", 6),
        (
            "dof_composite",
            "webgpu_postprocessing_dof_m12_composite.wgsl",
            7,
        ),
    ];
    for (label, fixture, index) in dof_quads {
        let mut material = dof.quad_materials()[index].clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // webgpu_postprocessing_ssr `m21`, `m23`, `m24`: `ssr( scenePassColor,
    // scenePassDepth, sceneNormal, { metalnessNode: metalRoughness.r,
    // roughnessNode: metalRoughness.g } )`'s three passes, with `sceneNormal`
    // the page's `sample( ( uv ) => unpackRGBToNormal( normal.sample( uv ) ) )`
    // and `blurQuality = 1` as `updateParameters()` sets it.
    let normal = input();
    let metal_rough = input();
    let scene_color = input();
    let ssr_node = ssr(
        &scene_color,
        &DepthTexture::new(),
        std::rc::Rc::new(move |coord| texture_uv(&normal, coord).mul(2.0).sub(1.0)),
        SsrOptions::new(
            texture_uv(&metal_rough, uv()).x(),
            Some(texture_uv(&metal_rough, uv()).y()),
        ),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            35.0, 1.6, 0.1, 50.0,
        ))),
    );
    ssr_node.set_blur_quality(1);
    for (label, fixture, mut material) in [
        (
            "ssr",
            "webgpu_postprocessing_ssr_m21_ssr.wgsl",
            ssr_node.quad_material(),
        ),
        (
            "ssr_copy",
            "webgpu_postprocessing_ssr_m23_ssr_copy.wgsl",
            ssr_node.copy_material().clone(),
        ),
        (
            "ssr_blur",
            "webgpu_postprocessing_ssr_m24_ssr_blur.wgsl",
            ssr_node.blur_material(),
        ),
    ] {
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // webgpu_postprocessing_ssr `m26`: the page's `RTT`, `scenePassColor.add(
    // ssrPass.rgb )` — the blur chain at `clamp( roughness² · 4, 0, 4 )`, its
    // `rgb` widened to `vec4( rgb, 1 )` and added to the beauty pass.
    quads.push(quad(
        "ssr_resolve",
        "webgpu_postprocessing_ssr_m26_ssr_resolve.wgsl",
        texture_uv(&scene_color, uv()).add(vec4_join(vec![ssr_node.node().rgb(), float(1.0)])),
    ));

    // tools/dump-pages/ssr_stochastic.html `m10`, `m14`, `m16`: three
    // `SSRNode.SSR` passes over the page's MRT, with `sceneNormal` its
    // `sample( ( uv ) => unpackRGBToNormal( normal.sample( uv ).rgb ) )`,
    // `metalnessNode: diffuseColor.a` and `roughnessNode: normal.a`.
    let normal = input();
    let diffuse = input();
    let hdr = equirect_hdr();
    let sampled_normal = {
        let normal = normal.clone();
        move || -> three_rs::nodes::display::SampleFn {
            let normal = normal.clone();
            Rc::new(move |coord| texture_uv(&normal, coord).rgb().mul(2.0).sub(1.0))
        }
    };
    let sampled_diffuse = || -> three_rs::nodes::display::SampleFn {
        let diffuse = diffuse.clone();
        Rc::new(move |coord| texture_uv(&diffuse, coord))
    };
    let options = || {
        SsrOptions::new(
            texture_uv(&diffuse, uv()).w(),
            Some(texture_uv(&normal, uv()).w()),
        )
    };
    let camera = || Rc::new(RefCell::new(PerspectiveCamera::new(50.0, 1.6, 0.1, 50.0)));
    // A: `{ stochastic: true, diffuseNode, environmentNode: hdr,
    // envImportanceSampling: false, binaryRefine: false }`.
    let ssr_a = ssr(
        &scene_color,
        &DepthTexture::new(),
        sampled_normal(),
        options()
            .with_stochastic(true)
            .with_diffuse(sampled_diffuse())
            .with_environment(&hdr),
        camera(),
    );
    // B: the page's `envImportanceSampling: true, binaryRefine: true`, its
    // `stepExponent = 3`, and `setHistory( A's target, velocity )`.
    let ssr_b = ssr(
        &scene_color,
        &DepthTexture::new(),
        sampled_normal(),
        options()
            .with_stochastic(true)
            .with_diffuse(sampled_diffuse())
            .with_environment(&hdr)
            .with_env_importance_sampling(true)
            .with_binary_refine(true),
        camera(),
    );
    ssr_b.set_step_exponent(3.0);
    ssr_b.set_history(&ssr_a.render_target().texture(), &input());
    // C: `{ stochastic: false, reflectNonMetals: true }`.
    let ssr_c = ssr(
        &scene_color,
        &DepthTexture::new(),
        sampled_normal(),
        options().with_reflect_non_metals(true),
        camera(),
    );
    for (label, fixture, node) in [
        (
            "ssr_stochastic",
            "ssr_stochastic_m10_ssr_stochastic.wgsl",
            &ssr_a,
        ),
        (
            "ssr_stochastic_refine",
            "ssr_stochastic_m14_ssr_stochastic_refine.wgsl",
            &ssr_b,
        ),
        (
            "ssr_reflect_non_metals",
            "ssr_stochastic_m16_ssr_reflect_non_metals.wgsl",
            &ssr_c,
        ),
    ] {
        let mut material = node.quad_material();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // webgpu_postprocessing_ssr `m28`, `m30`, `m32`: `smaa( … )`'s edges,
    // weights and blend passes over the page's `RTT`.
    let smaa_node = smaa(&input());
    for (label, fixture, material) in [
        (
            "smaa_edges",
            "webgpu_postprocessing_ssr_m28_smaa_edges.wgsl",
            smaa_node.edges_material(),
        ),
        (
            "smaa_weights",
            "webgpu_postprocessing_ssr_m30_smaa_weights.wgsl",
            smaa_node.weights_material(),
        ),
        (
            "smaa_blend",
            "webgpu_postprocessing_ssr_m32_smaa_blend.wgsl",
            smaa_node.blend_material(),
        ),
    ] {
        let mut material = material.clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }

    // webgpu_postprocessing_ssgi `m07`: `ssgi( scenePassColor,
    // scenePassDepth, sceneNormal, camera )` with `sliceCount = 2` and
    // `stepCount = 8` (uniforms, so the shader is the default's), whose
    // `colorNode` is the `gi` `Fn()` and whose `outputNode` is
    // `outputStruct( aoField, giField )`.
    let camera = std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
        40.0, 1.0, 0.1, 100.0,
    )));
    let scene_color = input();
    let scene_diffuse = input();
    let ssgi_node = ssgi(&scene_color, &DepthTexture::new(), &input(), camera.clone());
    let mut gi = ssgi_node.quad_material();
    gi.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "ssgi",
        fixture: "webgpu_postprocessing_ssgi_m07_ssgi.wgsl",
        material: gi,
    });

    // `m09`: the `RTT` quad `traa()`'s `convertToTexture()` wraps the page's
    // composite in — `vec4( add( scenePassColor.rgb.mul( ao ),
    // scenePassDiffuse.rgb.mul( gi.rgb ) ), scenePassColor.a )`.
    let color = texture_uv(&scene_color, uv());
    let composite = vec4_join(vec![
        color.xyz().mul(ssgi_node.ao_node()).add(
            texture_uv(&scene_diffuse, uv())
                .xyz()
                .mul(ssgi_node.gi_node()),
        ),
        color.w(),
    ]);
    let rtt = convert_to_texture(composite);
    let mut composite = rtt.quad_material().clone();
    composite.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "ssgi_composite",
        fixture: "webgpu_postprocessing_ssgi_m09_rtt.wgsl",
        material: composite,
    });

    // `m11`: the page's `TRAA.resolve`, the same material as the TRAA
    // page's — checked against this page's dump too.
    let ssgi_traa =
        three_rs::nodes::display::traa(&rtt.texture(), &DepthTexture::new(), &input(), camera);
    let mut resolve = ssgi_traa.quad_material().clone();
    resolve.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "ssgi_traa",
        fixture: "webgpu_postprocessing_ssgi_m11_traa_resolve.wgsl",
        material: resolve,
    });

    // webgpu_postprocessing_outline `m09`: the page's `renderPipeline.outputNode
    // = outlinePulse.add( scenePass )`, through the default output transform.
    let color_uniform = |hex| {
        let c = Color::from_hex(hex);
        uniform_value(Type::Vec3, vec![c.r, c.g, c.b])
    };
    let pulse_period = uniform_value(Type::F32, vec![0.0]);
    let period = time().div(pulse_period.clone()).mul(float(2.0));
    let osc = osc_sine(period).mul(float(0.5)).add(float(0.5));
    let outline_color = outline_pass
        .visible_edge()
        .mul(color_uniform(0xffffff))
        .add(outline_pass.hidden_edge().mul(color_uniform(0x4e3636)))
        .mul(uniform_value(Type::F32, vec![3.0]));
    let outline_pulse = pulse_period
        .greater_than(float(0.0))
        .select(outline_color.clone().mul(osc), outline_color);
    quads.push(quad(
        "outline_output",
        "webgpu_postprocessing_outline_m09_output.wgsl",
        render_output(
            outline_pulse.add(texture_uv(&input(), uv())),
            ToneMapping::None,
        ),
    ));

    // webgpu_oit `m06`: `oitPass( scene, camera )` as the `RenderPipeline`'s
    // output, so under `renderOutput()` with no tone mapping — the composite
    // of the beauty, `accum` and `revealage` textures.
    let oit = three_rs::nodes::display::oit_pass(
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::Scene::new())),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            45.0, 1.6, 0.1, 100.0,
        ))),
    );
    quads.push(quad(
        "oit_composite",
        "webgpu_oit_m06_composite.wgsl",
        render_output(oit.node(), ToneMapping::None),
    ));

    // webgpu_postprocessing_retro `m08` and `m10`: the page's pipeline with
    // its initial uniforms. `m08` is the `RTT` that `colorBleeding()`'s
    // `convertToTexture()` makes of the retro pass read through
    // `replaceDefaultUV( barrelUV( curvature ) )`: an unfilterable (nearest)
    // texture, so a clamped `textureLoad`. `m10` is the `RenderPipeline`'s
    // output: the bleed taps of that texture, Bayer, posterize, vignette and
    // scanlines, under the sRGB output transform.
    let f32_uniform = |value: f64| uniform_value(Type::F32, vec![value]);
    let curvature = f32_uniform(0.02);
    let color_depth_steps = f32_uniform(32.0);
    let retro = retro_pass(
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::Scene::new())),
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            25.0, 1.6, 0.1, 100.0,
        ))),
        RetroPassOptions::default().affine_distortion(f32_uniform(0.0)),
    );
    let distorted = replace_default_uv(barrel_uv(curvature.clone(), uv()), retro.node());
    quads.push(quad(
        "retro_barrel",
        "webgpu_postprocessing_retro_m08_retro_barrel.wgsl",
        distorted.clone(),
    ));
    let distorted_delta = circle(curvature.add(0.1).mul(10.0), float(1.0), uv())
        .mul(curvature)
        .mul(0.05);
    let mut crt = color_bleeding(distorted, f32_uniform(0.001).add(distorted_delta));
    crt = bayer_dither(crt, color_depth_steps.clone());
    crt = posterize(crt, color_depth_steps);
    crt = three_rs::nodes::display::vignette(crt, f32_uniform(0.3), float(0.6), uv());
    crt = scanlines(
        crt,
        f32_uniform(0.3),
        screen_size().y().mul(f32_uniform(1.0)),
        f32_uniform(0.0),
        uv(),
    );
    quads.push(quad(
        "retro_crt",
        "webgpu_postprocessing_retro_m10_retro_crt.wgsl",
        render_output(crt, ToneMapping::None),
    ));

    // `tools/dump-pages/film_sepia_bleach.html` `m03`, `m05`, `m07` and
    // `m09`: `bleach( scenePass, uniform( 0.8 ) )`, `sepia()` of its texture
    // and `film()` of that, with no intensity, each converted to a texture,
    // then `film( …, uniform( 0.5 ) )` as the `RenderPipeline`'s output.
    quads.push(quad(
        "bleach_bypass",
        "film_sepia_bleach_m03_bleach_bypass.wgsl",
        bleach(texture_uv(&input(), uv()), f32_uniform(0.8)),
    ));
    quads.push(quad(
        "sepia",
        "film_sepia_bleach_m05_sepia.wgsl",
        sepia(texture_uv(&input(), uv())),
    ));
    quads.push(quad(
        "film_no_intensity",
        "film_sepia_bleach_m07_film_no_intensity.wgsl",
        film(texture_uv(&input(), uv()), None, None),
    ));
    quads.push(quad(
        "film",
        "film_sepia_bleach_m09_film.wgsl",
        render_output(
            film(texture_uv(&input(), uv()), Some(f32_uniform(0.5)), None),
            ToneMapping::None,
        ),
    ));

    // tools/dump-pages/temporal_reproject.html: `m03` is the seed quad
    // (`TemporalReproject.seed`) of the first node, `convertToTexture(
    // temporalReproject( scenePassColor, depth, normal, velocity, camera ) )`
    // with the defaults, and `m05` its resolve quad; `m09` is the resolve of
    // the page's configuration, `{ mode: 'specular', accumulate: false }`
    // with `setHistoryTexture()` given another texture.
    let reproject_camera = || {
        std::rc::Rc::new(std::cell::RefCell::new(three_rs::PerspectiveCamera::new(
            70.0, 1.0, 0.1, 10.0,
        )))
    };
    let diffuse = temporal_reproject(
        &input(),
        &DepthTexture::new(),
        &input(),
        &input(),
        reproject_camera(),
        TemporalReprojectOptions::default(),
    );
    let mut seed = diffuse.seed_material();
    seed.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "temporal_reproject_seed",
        fixture: "temporal_reproject_m03_seed.wgsl",
        material: seed,
    });
    let mut resolve = diffuse.quad_material();
    resolve.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "temporal_reproject_resolve",
        fixture: "temporal_reproject_m05_resolve.wgsl",
        material: resolve,
    });
    let specular = temporal_reproject(
        &input(),
        &DepthTexture::new(),
        &input(),
        &input(),
        reproject_camera(),
        TemporalReprojectOptions {
            mode: TemporalReprojectMode::Specular,
            accumulate: false,
            ..TemporalReprojectOptions::default()
        },
    );
    specular.set_history_texture(Some(&input()));
    let mut resolve = specular.quad_material();
    resolve.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "temporal_reproject_resolve_specular",
        fixture: "temporal_reproject_m09_resolve_specular.wgsl",
        material: resolve,
    });
    // `tools/dump-pages/sharpen.html` `m03` and `m06`: `sharpen( scenePass,
    // 0.2 )`'s RCAS quad, then `sharpen( a, 0.5, true )`'s over the `RTT`
    // three's `convertToTexture()` makes of the first.
    for (label, fixture, sharpness, denoise) in [
        ("sharpen_rcas", "sharpen_m03_rcas.wgsl", 0.2, false),
        (
            "sharpen_rcas_denoise",
            "sharpen_m06_rcas_denoise.wgsl",
            0.5,
            true,
        ),
    ] {
        let mut material = SharpenNode::new(&input(), float(sharpness), denoise)
            .quad_material()
            .clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }
    recurrent_denoise_quads(&mut quads);
    specular_helpers_quads(&mut quads);
    fsr1_quads(&mut quads);

    quads
}

/// `webgpu_upscaling_fsr1` `m36` and `m38`: `fsr1( scenePass )`'s EASU quad
/// over the half-resolution pass, then its RCAS quad over the EASU target,
/// at the default sharpness 0.2 without `denoise`.
fn fsr1_quads(quads: &mut Vec<DisplayQuad>) {
    let fsr1 = Fsr1Node::new(&input(), float(Fsr1Node::DEFAULT_SHARPNESS), false);
    for (label, fixture, material) in [
        (
            "fsr1_easu",
            "webgpu_upscaling_fsr1_m36_easu.wgsl",
            fsr1.easu_material(),
        ),
        (
            "fsr1_rcas",
            "webgpu_upscaling_fsr1_m38_rcas.wgsl",
            fsr1.rcas_material(),
        ),
    ] {
        let mut material = material.clone();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }
}

/// `tools/dump-pages/recurrent_denoise.html` `m09` and `m05`: the diffuse
/// denoiser over the scene pass (`alphaSource = 'ao'`, the beauty as its own
/// raw input) and the page's specular configuration reading the first's
/// output (`alphaSource = 'raylength'`, `accumulate`, every G-buffer bound).
fn recurrent_denoise_quads(quads: &mut Vec<DisplayQuad>) {
    let camera = Rc::new(RefCell::new(PerspectiveCamera::new(35.0, 1.6, 0.1, 50.0)));
    let scene_color = input();
    let depth = DepthTexture::new();
    let normal_tex = input();
    let diffuse_tex = input();
    let normal: SampleFn = {
        let normal_tex = normal_tex.clone();
        Rc::new(move |coord| texture_uv(&normal_tex, coord))
    };

    let diffuse_denoise = recurrent_denoise(
        &scene_color,
        camera.clone(),
        RecurrentDenoiseOptions {
            depth: Some(depth.clone()),
            normal: Some(normal.clone()),
            raw: Some(scene_color.clone()),
            ..Default::default()
        },
    );
    diffuse_denoise.set_alpha_source(DenoiseAlphaSource::Ao);

    let metal_roughness: SampleFn = {
        let (diffuse_tex, normal_tex) = (diffuse_tex.clone(), normal_tex.clone());
        Rc::new(move |coord: three_rs::nodes::NodeRef| {
            vec2_join(vec![
                texture_uv(&diffuse_tex, coord.clone()).w(),
                texture_uv(&normal_tex, coord).w(),
            ])
        })
    };
    let diffuse: SampleFn = {
        let diffuse_tex = diffuse_tex.clone();
        Rc::new(move |coord| texture_uv(&diffuse_tex, coord))
    };
    let specular_denoise = recurrent_denoise(
        &diffuse_denoise.texture(),
        camera,
        RecurrentDenoiseOptions {
            depth: Some(depth),
            normal: Some(normal),
            metal_roughness: Some(metal_roughness),
            diffuse: Some(diffuse),
            raw: Some(scene_color),
            mode: DenoiseMode::Specular,
            accumulate: true,
        },
    );
    specular_denoise.set_alpha_source(DenoiseAlphaSource::RayLength);

    for (label, fixture, node) in [
        (
            "recurrent_denoise_diffuse",
            "recurrent_denoise_m09_diffuse.wgsl",
            &diffuse_denoise,
        ),
        (
            "recurrent_denoise_specular",
            "recurrent_denoise_m05_specular.wgsl",
            &specular_denoise,
        ),
    ] {
        let mut material = node.quad_material();
        material.vertex_node = Some(quad_vertex_node());
        quads.push(DisplayQuad {
            label,
            fixture,
            material,
        });
    }
}

/// `tools/dump-pages/specular_helpers.html`: one `convertToTexture()` quad
/// per helper group of `SpecularHelpers.js`, `RNoise.js` and
/// `ImportanceSampledEnvironment.js`.
fn specular_helpers_quads(quads: &mut Vec<DisplayQuad>) {
    // `m01`: `ggxReflectionSample( N, V, p.x, p.y, vec3( 0.9, 0.6, 0.3 ),
    // vec4( p, p.yx ) )`, every member of the result read.
    let p = uv();
    let n = vec3_join(vec![p.sub(0.5), float(1.0)]).normalize();
    let v = vec3_join(vec![p.swizzle("yx").sub(0.5), float(1.0)]).normalize();
    let sample = ggx_reflection_sample(
        n,
        v,
        p.x(),
        p.y(),
        vec3(0.9, 0.6, 0.3),
        vec4_join(vec![p.clone(), p.swizzle("yx")]),
    );
    let layout = ggx_reflection_struct();
    let get = |name: &str| struct_get(&sample, &layout, name);
    quads.push(quad(
        "specular_ggx_reflection_sample",
        "specular_helpers_m01_ggx_reflection_sample.wgsl",
        vec4_join(vec![
            get("reflectDir").add(get("sampleWeight")).add(get("f0")),
            get("pdf").add(get("NdotV")).add(get("alpha")),
        ]),
    ));

    // `m03`: the scalar BRDF terms and the equirect helpers in one colour.
    let p = uv();
    let dir = equirect_uv_to_dir(p.clone());
    let pdf = equirect_dir_pdf(dir.clone());
    let w = mis_power_heuristic(pdf, p.x());
    let d = d_gtr(p.x(), p.y(), float(2.0));
    let g1 = smith_g(p.y(), p.x());
    let g = geometry_term(p.x(), p.y(), float(0.5));
    let f = f_schlick(vec3(0.04, 0.04, 0.04), p.y());
    let sdf = get_specular_dominant_factor(p.y(), p.x());
    quads.push(quad(
        "specular_helpers",
        "specular_helpers_m03_specular_helpers.wgsl",
        vec4_join(vec![dir.mul(w).add(f), d.add(g1).add(g).add(sdf)]),
    ));

    // `m04`: `bindAnalyticNoise( uniform( vec2( 800, 500 ) ), 47 )( uv(),
    // int( 3 ) )`.
    let noise = bind_analytic_noise(uniform_value(Type::Vec2, vec![800.0, 500.0]), 47);
    quads.push(quad(
        "analytic_noise",
        "specular_helpers_m04_analytic_noise.wgsl",
        noise(uv(), int(3)),
    ));

    environment_quads(quads);
}

/// The 8×4 float equirect HDR of `tools/dump-pages/specular_helpers.html`
/// and `ssr_stochastic.html`: a gradient with one hot texel at (2, 2).
fn equirect_hdr() -> Texture {
    let (width, height) = (8, 4);
    let mut pixels = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let hot = if x == 2 && y == 2 { 20.0 } else { 0.0 };
            pixels.extend([
                0.2 + 0.05 * x as f32 + hot,
                0.3 + 0.1 * y as f32 + hot * 0.8,
                0.5 + hot * 0.4,
                1.0,
            ]);
        }
    }
    let hdr = Texture::data_rgba32float(width as u32, height as u32, &pixels);
    // `new DataTexture()`'s `NearestFilter` default (`data_rgba32float`
    // picks linear), which the map's clone keeps: three reads it with
    // `textureLoad`.
    hdr.set_min_filter(MinFilter::Nearest);
    hdr.set_mag_filter(TextureFilter::Nearest);
    hdr
}

/// Quads D–F of `specular_helpers.html`: an 8×4 float equirect with a hot
/// texel at ( 2, 2 ), looked up through `ImportanceSampledEnvironment`.
fn environment_quads(quads: &mut Vec<DisplayQuad>) {
    let hdr = equirect_hdr();
    let camera_world_matrix = uniform_value(Type::Mat4, Matrix4::identity().elements.to_vec());
    let view_reflect_dir = || vec3_join(vec![uv().sub(0.5), float(-1.0)]).normalize();
    let lobe = || EnvironmentLobe {
        camera_world_matrix: camera_world_matrix.clone(),
        view_reflect_dir: view_reflect_dir(),
        n: vec3_join(vec![uv().swizzle("yx").sub(0.5), float(1.0)]).normalize(),
        v: vec3(0.0, 0.0, 1.0),
        alpha: uv().x().mul(uv().x()),
        f0: vec3(0.04, 0.04, 0.04),
    };

    // `m05` (D) and `m06` (E): `new ImportanceSampledEnvironment( false )`.
    let mut environment = ImportanceSampledEnvironment::new(false);
    environment.update_from(&hdr);
    quads.push(quad(
        "env_sample_reflect",
        "specular_helpers_m05_env_sample_reflect.wgsl",
        vec4_join(vec![
            environment.sample_reflect(&camera_world_matrix, view_reflect_dir(), None),
            float(1.0),
        ]),
    ));
    quads.push(quad(
        "env_sample_brdf",
        "specular_helpers_m06_env_sample_brdf.wgsl",
        vec4_join(vec![
            environment.sample_environment_brdf(&lobe()),
            float(1.0),
        ]),
    ));

    // `m08` (F): `new ImportanceSampledEnvironment( true )`, the MIS path.
    let mut importance = ImportanceSampledEnvironment::new(true);
    importance.update_from(&hdr);
    let xi2 = vec4_join(vec![uv(), uv().swizzle("yx")]);
    quads.push(quad(
        "env_sample_mis",
        "specular_helpers_m08_env_sample_mis.wgsl",
        vec4_join(vec![
            importance.sample_environment_mis(&lobe(), xi2),
            float(1.0),
        ]),
    ));
}
