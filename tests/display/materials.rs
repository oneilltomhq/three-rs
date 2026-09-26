//! The display-node quads of #144, built as the three.js pages that three's
//! dumps come from build them. Shared by `tests/nodes_display_wgsl.rs`, which
//! checks them against `tests/fixtures/nodes_display/`, and by
//! `examples/dump_wgsl.rs`, which prints them.

use three_rs::materials::{quad_vertex_node, render_output, MeshBasicNodeMaterial};
use three_rs::nodes::display::{
    after_image, box_blur, dot_screen, gaussian_blur, hash_blur, pixelation_pass, rgb_shift, sobel,
    BoxBlurOptions, GaussianBlurOptions, HashBlurOptions,
};
use three_rs::nodes::tsl::{float, texture_uv, uniform_value, uv};
use three_rs::nodes::Type;
use three_rs::textures::Texture;
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
    quads.push(quad(
        "box_blur",
        "webgpu_postprocessing_dof_basic_m23_box_blur.wgsl",
        box_blur(
            &input(),
            BoxBlurOptions {
                size: uniform_value(Type::F32, vec![2.0]),
                separation: uniform_value(Type::F32, vec![4.0]),
                premultiplied_alpha: false,
            },
        ),
    ));

    // webgpu_backdrop_area `m08`: `hashBlur( viewportSharedTexture(), .05 )`.
    // Again only the loop is compared; the tap there is a viewport texture.
    quads.push(quad(
        "hash_blur",
        "webgpu_backdrop_area_m08_hash_blur.wgsl",
        hash_blur(&input(), float(0.05), HashBlurOptions::default()),
    ));

    quads
}
