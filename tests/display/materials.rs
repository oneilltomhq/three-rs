//! The display-node quads of #144, built as the three.js pages that three's
//! dumps come from build them. Shared by `tests/nodes_display_wgsl.rs`, which
//! checks them against `tests/fixtures/nodes_display/`, and by
//! `examples/dump_wgsl.rs`, which prints them.

use three_rs::materials::{quad_vertex_node, render_output, MeshBasicNodeMaterial};
use three_rs::nodes::display::{
    after_image, box_blur, dof, dot_screen, fxaa, gaussian_blur, hash_blur_with, motion_blur,
    pixelation_pass, rgb_shift, sobel, traa, viewport_shared_texture_at, BoxBlurOptions,
    GaussianBlurOptions, HashBlurOptions,
};
use three_rs::nodes::tsl::{
    distance, float, pass_depth_texture, perspective_depth_to_view_z, screen_uv, texture_uv,
    uniform_value, uv, vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::{DepthTexture, Texture};
use three_rs::ToneMapping;

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

    quads
}
