//! MRT, drawn: that a second colour attachment reaches the pass descriptor and
//! the pipeline, that each material's `mrtNode` overrides the pass's default,
//! and that a draw with no MRT set is untouched.
//!
//! This is the renderer half of the first sitting of
//! `webgpu_postprocessing_bloom_selective`. The WGSL half — the `OutputType`
//! struct and the two `output.mN` lines — is the `bloom_selective_scene`
//! section of `examples/dump_wgsl.rs`, diffed against three's own dump.
//!
//! The failure this guards against is silent: with a one-attachment pipeline
//! bound to a two-attachment pass, wgpu refuses the draw outright — but with
//! the *members* laid out by dictionary order rather than by attachment index,
//! or with the material's MRT losing to the pass's, every pixel is a plausible
//! wrong colour. So each assertion reads a value that could only have come
//! from the right attachment.
//!
//! Attachment 1 is read by sampling it into a second target: the readback API
//! takes a `RenderTarget` and gives attachment 0, and a quad that samples the
//! texture proves the attachment is bindable as well as writable, which is what
//! `BloomNode` will do with it.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::tsl::{float, output_property, texture_uv, uniform_value, uv};
use three_rs::nodes::{mrt, Type};
use three_rs::renderer::{RenderTarget, RenderTargetOptions};
use three_rs::textures::TextureType;
use three_rs::{
    Color, Mesh, PerspectiveCamera, QuadMesh, Renderer, RendererParameters, Scene, TextureFilter,
    Vector3,
};

const SIZE: u32 = 8;

/// `PassNode`'s target: `rgba16float`, with a depth buffer, since the scene
/// pass of `webgpu_postprocessing_bloom_selective` is the only one of its
/// fourteen passes that has one.
fn pass_target() -> RenderTarget {
    RenderTarget::new_with_options(
        SIZE,
        SIZE,
        RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: true,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )
    .expect("HalfFloatType is a colour type")
}

fn texel(pixels: &[f32], x: u32, y: u32) -> [f32; 4] {
    let i = ((y * SIZE + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// The texel at the centre, which every scene here covers.
fn centre(pixels: &[f32]) -> [f32; 4] {
    texel(pixels, SIZE / 2, SIZE / 2)
}

/// A camera and a scene of one plane filling the view, with `mrt_node` on the
/// plane's material.
fn scene_with(mrt_node: Option<three_rs::nodes::MrtNode>) -> (Scene, PerspectiveCamera) {
    let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    camera.node.borrow_mut().position.set(0.0, 0.0, 5.0);
    camera.look_at(&Vector3::ZERO);

    let scene = Scene::new();
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::new(0.25, 0.5, 0.75);
    material.mrt_node = mrt_node;
    let mesh = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), material);
    scene.add(&mesh);

    (scene, camera)
}

/// The pass's own MRT — `mrt( { output, bloomIntensity: float( 0 ) } )`, the
/// default each material overrides.
fn pass_mrt() -> three_rs::nodes::MrtNode {
    mrt(vec![
        ("output", output_property()),
        ("bloomIntensity", float(0.0)),
    ])
}

/// Render `scene` into `target` with `mrt` set, restoring both after, as
/// `PassNode.updateBefore()` does.
fn render_into(
    renderer: &mut Renderer,
    target: &RenderTarget,
    mrt_node: Option<three_rs::nodes::MrtNode>,
    scene: &mut Scene,
    camera: &mut PerspectiveCamera,
) {
    renderer.set_render_target(Some(target.clone()));
    renderer.set_mrt(mrt_node);
    renderer.render(scene, camera);
    renderer.set_mrt(None);
    renderer.set_render_target(None);
}

/// One colour attachment sampled into a fresh single-attachment target and read
/// back — the only way to see attachment 1, and a proof that it binds.
fn read_attachment(renderer: &mut Renderer, texture: &three_rs::Texture) -> Vec<f32> {
    let probe = RenderTarget::new_with_options(
        SIZE,
        SIZE,
        RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: false,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )
    .expect("HalfFloatType is a colour type");

    let mut material = MeshBasicNodeMaterial::new();
    material.fragment_node = Some(texture_uv(texture, uv()));
    material.depth_test = false;
    material.depth_write = false;

    renderer.set_render_target(Some(probe.clone()));
    renderer.render_quad(&QuadMesh::new(material));
    renderer.set_render_target(None);

    renderer
        .read_target_pixels_rgba16f(&probe)
        .expect("the probe is rgba16float")
        .2
}

#[test]
fn mrt_writes_every_attachment() {
    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    the_material_mrt_beats_the_pass_default(&mut renderer);
    a_pass_with_no_mrt_writes_only_the_first_attachment(&mut renderer);
    an_attachment_is_created_only_by_asking_for_it(&mut renderer);
}

/// The whole of selective bloom's scene pass: attachment 0 gets the material's
/// colour and attachment 1 gets `bloomIntensity`, which is the material's
/// `uniform( 0.75 )` and *not* the pass's `float( 0 )`.
///
/// 0.75 is exactly representable in binary16 and is neither of the values a
/// mistake would produce: 0 is the pass default winning the merge, and the
/// clear is `( 0, 0, 0, 0 )`.
fn the_material_mrt_beats_the_pass_default(renderer: &mut Renderer) {
    let target = pass_target();
    let bloom_texture = target.add_texture("bloomIntensity");
    assert_eq!(target.attachment_names(), ["output", "bloomIntensity"]);
    assert_eq!(target.textures().len(), 2);

    let (mut scene, mut camera) = scene_with(Some(mrt(vec![(
        "bloomIntensity",
        uniform_value(Type::F32, vec![0.75]),
    )])));
    render_into(renderer, &target, Some(pass_mrt()), &mut scene, &mut camera);

    // Attachment 0 is the material's colour, untouched by the MRT: three's own
    // `output.m0 = Output` is the standard flow's result.
    let colour = renderer
        .read_target_pixels_rgba16f(&target)
        .expect("the pass target is rgba16float")
        .2;
    assert_eq!(centre(&colour), [0.25, 0.5, 0.75, 1.0]);

    // Attachment 1 is `vec4<f32>( bloomIntensity )` — the scalar splat
    // `NodeBuilder.format()` puts on an `f32` member of a `vec4` output.
    let bloom = read_attachment(renderer, &bloom_texture);
    assert_eq!(centre(&bloom), [0.75, 0.75, 0.75, 0.75]);
}

/// The other half of the gate, and the one the whole ladder depends on: with no
/// MRT set the fragment stage is the old `OutputStruct { color }` and the
/// second attachment is never written, so it still reads as the clear.
fn a_pass_with_no_mrt_writes_only_the_first_attachment(renderer: &mut Renderer) {
    let target = pass_target();
    let bloom_texture = target.add_texture("bloomIntensity");

    let (mut scene, mut camera) = scene_with(Some(mrt(vec![(
        "bloomIntensity",
        uniform_value(Type::F32, vec![0.75]),
    )])));
    // The material still carries an `mrtNode`; `NodeMaterial.setup()`'s MRT
    // branch is guarded on the *renderer's*, so it is inert.
    render_into(renderer, &target, None, &mut scene, &mut camera);

    let colour = renderer
        .read_target_pixels_rgba16f(&target)
        .expect("the pass target is rgba16float")
        .2;
    assert_eq!(centre(&colour), [0.25, 0.5, 0.75, 1.0]);

    let bloom = read_attachment(renderer, &bloom_texture);
    assert_eq!(centre(&bloom), [0.0, 0.0, 0.0, 0.0]);
}

/// `MRTNode.setup()` drops an output with no attachment of that name
/// (`index === -1`), so a pass whose MRT names an output nothing ever asked
/// `getTextureNode()` for renders single-attachment — in three.js too.
fn an_attachment_is_created_only_by_asking_for_it(renderer: &mut Renderer) {
    let target = pass_target();
    assert_eq!(target.attachment_names(), ["output"]);

    let (mut scene, mut camera) = scene_with(Some(mrt(vec![(
        "bloomIntensity",
        uniform_value(Type::F32, vec![0.75]),
    )])));
    render_into(renderer, &target, Some(pass_mrt()), &mut scene, &mut camera);

    let colour = renderer
        .read_target_pixels_rgba16f(&target)
        .expect("the pass target is rgba16float")
        .2;
    assert_eq!(centre(&colour), [0.25, 0.5, 0.75, 1.0]);
}
