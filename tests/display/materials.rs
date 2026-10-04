//! The display-node quads of #144, built as the three.js pages that three's
//! dumps come from build them. Shared by `tests/nodes_display_wgsl.rs`, which
//! checks them against `tests/fixtures/nodes_display/`, and by
//! `examples/dump_wgsl.rs`, which prints them.

use three_rs::materials::{quad_vertex_node, render_output, MeshBasicNodeMaterial};
use three_rs::nodes::display::{
    after_image, bilateral_blur, box_blur, depth_aware_blend, dot_screen, fxaa, gaussian_blur,
    godrays, hash_blur_with, lensflare, motion_blur, pixelation_pass, rgb_shift, rtt, sobel, traa,
    viewport_shared_texture_at, BoxBlurOptions, DepthAwareBlendOptions, GaussianBlurOptions,
    HashBlurOptions, LensflareParams,
};
use three_rs::nodes::tsl::{
    distance, float, screen_uv, texture_uv, uniform_value, uv, vec2, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::{DepthTexture, Texture};
use three_rs::{Color, PerspectiveCamera, PointLight, ToneMapping};

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
    let mut resolve = traa.quad_material().clone();
    resolve.vertex_node = Some(quad_vertex_node());
    quads.push(DisplayQuad {
        label: "traa",
        fixture: "webgpu_postprocessing_traa_m05_traa_resolve.wgsl",
        material: resolve,
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

    quads
}
