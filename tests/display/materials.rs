//! The display-node quads of #144, built as the three.js pages that three's
//! dumps come from build them. Shared by `tests/nodes_display_wgsl.rs`, which
//! checks them against `tests/fixtures/nodes_display/`, and by
//! `examples/dump_wgsl.rs`, which prints them.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::loaders::LutCubeLoader;
use three_rs::materials::{quad_vertex_node, render_output, MeshBasicNodeMaterial};
use three_rs::nodes::display::{
    after_image, box_blur, dot_screen, fxaa, gaussian_blur, hash_blur_with, lut_3d, motion_blur,
    outline, pixelation_pass, rgb_shift, sobel, traa, viewport_shared_texture_at, BoxBlurOptions,
    GaussianBlurOptions, HashBlurOptions, OutlineParams,
};
use three_rs::nodes::tsl::{
    distance, float, osc_sine, screen_uv, texture_3d_sampled, texture_uv, time, uniform_value, uv,
    vec4_join,
};
use three_rs::nodes::Type;
use three_rs::textures::{DepthTexture, Texture};
use three_rs::{Color, PerspectiveCamera, Scene, ToneMapping};

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

    quads
}
