//! `sss()`, over frames. three lists `webgpu_postprocessing_sss` in its e2e
//! exception list, so there is no reference screenshot and no rung; the SSS
//! shader is gated against three's dump in `tests/nodes_display_wgsl.rs`,
//! and what the frames do is checked here.
//!
//! A unit box standing on a floor, seen from above and in front, with a
//! directional light behind the box, so the rays from the floor in front of
//! it march back and up into the box's front face. The SSS texture itself
//! is the pipeline's output, so the canvas is `1 - occlusion`:
//!
//! * where the quad discards (no geometry: depth 1) the target keeps its
//!   white clear, so the sky is 255;
//! * a band of floor in front of the box is occluded, at `shadowIntensity`
//!   (black at 1, mid-grey at 0.5), and the floor out in the open is not,
//!   but for the odd pixel whose first step lands in its own texel, as in
//!   three;
//! * nothing is NaN (a NaN reads back as 0 in `rgba8unorm`, so with
//!   `shadowIntensity` 0.5 no pixel may be below mid-grey);
//! * a `maxDistance` shorter than a pixel marches no step at all, so
//!   nothing is occluded;
//! * a `thickness` smaller than a step's overshoot takes the box's face to
//!   be a thin surface the ray passes behind, so the band goes;
//! * a finer `quality` occludes more and a coarser one less, and setting
//!   both back gives the first frame back;
//! * temporal filtering and `resolutionScale = 0.5` still put the band in
//!   front of the box;
//! * with `builtinShadowContext`, the beauty of a Phong floor lit by the
//!   (shadow-casting) light is unchanged wherever the SSS is white and
//!   darker in the band; clearing the context gives the plain frame back;
//!   and with the floor not receiving shadows the context has no shadow to
//!   multiply into, so the frame does not change.
//!
//! The test runs with `maxDistance = 1` and `thickness = 0.1` rather than
//! three's 0.1 and 0.01: at 64 pixels a step crosses more depth than 0.01,
//! and a ray of 0.1 is a pixel or two long.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::core::ObjectRef;
use three_rs::geometries::{box_geometry, plane_geometry};
use three_rs::nodes::display::{sss, SssNode};
use three_rs::nodes::tsl::screen_uv;
use three_rs::{
    pass, Color, DirectionalLight, Mesh, MeshBasicNodeMaterial, MeshPhongNodeMaterial, PassNode,
    PerspectiveCamera, RenderPipeline, Renderer, RendererParameters, Scene, Vector3,
};

const SIZE: u32 = 64;

/// The red channel of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<u8> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| p[0]).collect()
}

fn at(red: &[u8], x: u32, y: u32) -> u8 {
    red[(y * SIZE + x) as usize]
}

/// The mean of a `(2r+1)²` block around `(x, y)`.
fn mean(red: &[u8], x: u32, y: u32, r: u32) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for yy in y - r..=y + r {
        for xx in x - r..=x + r {
            sum += at(red, xx, yy) as f64;
            n += 1.0;
        }
    }
    sum / n
}

/// How many pixels are below `threshold`.
fn count_below(red: &[u8], threshold: u8) -> usize {
    red.iter().filter(|&&r| r < threshold).count()
}

struct Setup {
    scene: Rc<RefCell<Scene>>,
    camera: Rc<RefCell<PerspectiveCamera>>,
    light: ObjectRef,
    floor: ObjectRef,
}

/// A unit box on a floor, lit by a shadow-casting directional light behind
/// it; the camera looks down at the box's front foot. The box casts no
/// shadow-map shadow, so the floor in front of it is lit but for the SSS.
fn box_on_floor(material: MeshBasicNodeMaterial) -> Setup {
    let scene = Scene::new();
    let floor = Mesh::new(Rc::new(plane_geometry(20.0, 20.0, 1, 1)), material.clone());
    {
        let mut object = floor.borrow_mut();
        object.set_rotation(-std::f64::consts::FRAC_PI_2, 0.0, 0.0);
        object.receive_shadow = true;
    }
    scene.add(&floor);
    let cube = Mesh::new(Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1)), material);
    cube.borrow_mut().position.y = 0.5;
    scene.add(&cube);

    let light = DirectionalLight::new(Color::from_hex(0xffffff), 3.0);
    {
        let mut object = light.borrow_mut();
        object.position.set(0.0, 4.0, -3.0);
        object.cast_shadow = true;
    }
    scene.add(&light);

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 20.0);
    camera.node.borrow_mut().position = Vector3::new(0.0, 1.6, 2.6);
    camera.look_at(&Vector3::new(0.0, 0.3, 0.0));
    Setup {
        scene: Rc::new(RefCell::new(scene)),
        camera: Rc::new(RefCell::new(camera)),
        light,
        floor,
    }
}

/// The page's depth pre-pass and the SSS node reading it.
fn pre_pass_and_sss(setup: &Setup) -> (PassNode, SssNode) {
    let mut pre_pass = pass(setup.scene.clone(), setup.camera.clone());
    pre_pass.set_transparent(false);
    let _ = pre_pass.texture_node("depth");
    let sss_node = sss(
        &pre_pass.depth_texture(),
        setup.camera.clone(),
        &setup.light,
    );
    sss_node.max_distance.set(vec![1.0]);
    sss_node.thickness.set(vec![0.1]);
    (pre_pass, sss_node)
}

/// The floor just in front of the box, in canvas pixels, found from the SSS
/// frame itself: scanning up the centre column from the bottom of the
/// frame, the first pixel whose 3x3 block is mostly occluded, one row
/// further in; and the centre of a 9x9 patch of open floor in the bottom
/// left corner, clear of the band.
fn contact_and_open(red: &[u8]) -> ((u32, u32), (u32, u32)) {
    let x = SIZE / 2;
    let edge = (SIZE / 2..SIZE - 2)
        .rev()
        .find(|&y| mean(red, x, y, 1) < 64.0)
        .expect("an occluded band in front of the box");
    ((x, edge - 1), (8, SIZE - 6))
}

#[test]
fn sss_shadows_the_floor_in_front_of_the_box() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);
    renderer.shadow_map_enabled = true;

    let mut white = MeshBasicNodeMaterial::new();
    white.color = Color::from_hex(0xffffff);
    let setup = box_on_floor(white);
    let (_pre_pass, sss_node) = pre_pass_and_sss(&setup);

    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(sss_node.node());

    let full = frame(&mut pipeline, &mut renderer);
    assert_eq!(at(&full, 2, 2), 255, "the sky keeps the white clear");
    let (contact, open) = contact_and_open(&full);
    let contact_sss = mean(&full, contact.0, contact.1, 1);
    let open_sss = mean(&full, open.0, open.1, 4);
    let occluded = count_below(&full, 128);
    println!(
        "intensity 1: contact {contact_sss:.1} at row {}, open {open_sss:.1}, {occluded} occluded",
        contact.1
    );
    assert!(
        contact.1 > SIZE / 2 + 8,
        "the band is on the floor below the box (row {})",
        contact.1
    );
    assert!(
        open_sss > 225.0,
        "the open floor is unoccluded ({open_sss:.1})"
    );
    assert!(
        contact_sss < 64.0,
        "the floor in front of the box is occluded ({contact_sss:.1})"
    );

    // `shadowIntensity = 0.5`: occluded is mid-grey, and nothing is below
    // it — a NaN would read back as 0.
    sss_node.shadow_intensity.set(vec![0.5]);
    let half_intensity = frame(&mut pipeline, &mut renderer);
    let darkest = *half_intensity.iter().min().unwrap();
    println!("intensity 0.5: darkest {darkest}");
    assert!(
        (126..=129).contains(&darkest),
        "occluded pixels are at half intensity, and none is NaN ({darkest})"
    );
    assert_eq!(
        count_below(&half_intensity, 130),
        occluded,
        "the same pixels are occluded at half intensity"
    );
    sss_node.shadow_intensity.set(vec![1.0]);

    // A ray shorter than a pixel takes no step.
    sss_node.max_distance.set(vec![0.0001]);
    let short = frame(&mut pipeline, &mut renderer);
    println!("max distance 0.0001: {} occluded", count_below(&short, 255));
    assert!(
        short.iter().all(|&r| r == 255),
        "a sub-pixel ray occludes nothing"
    );
    sss_node.max_distance.set(vec![1.0]);

    // `thickness`: a surface occludes only while it sits in front of the
    // ray by less than `thickness`; one further in front is taken to be
    // thin, with the ray passing behind it. The box's face is a unit deep,
    // but the first step that lands on it is already behind it by up to one
    // step's worth of view depth, about 0.01 here (at `thickness = 0.01`
    // most of the band is already gone). At 0.0001 the face is in front of
    // every step by more than `thickness`, so the band goes: the contact
    // block must read as open floor (the same 225 as the open-floor check)
    // and fewer than one in twenty of the baseline's occluded pixels may
    // remain (0 on the machine this was written on).
    sss_node.thickness.set(vec![0.0001]);
    let thin = frame(&mut pipeline, &mut renderer);
    let contact_thin = mean(&thin, contact.0, contact.1, 1);
    let occluded_thin = count_below(&thin, 128);
    println!("thickness 0.0001: contact {contact_thin:.1}, {occluded_thin} occluded");
    assert!(
        contact_thin > 225.0,
        "a face further in front of the ray than `thickness` does not occlude ({contact_thin:.1})"
    );
    assert!(
        occluded_thin * 20 < occluded,
        "a sub-step `thickness` leaves almost nothing occluded ({occluded_thin} of {occluded})"
    );
    sss_node.thickness.set(vec![0.1]);

    // `quality`: the march takes one step per `1 / quality` pixels, and how
    // far a step overshoots the face scales with its length, so the finer
    // the march the more rays land within `thickness` of the face. From
    // three's 0.5, 1 must occlude more pixels (about a quarter more here)
    // and 0.1 fewer than half as many (about a fifth here).
    sss_node.quality.set(vec![1.0]);
    let fine = frame(&mut pipeline, &mut renderer);
    let occluded_fine = count_below(&fine, 128);
    sss_node.quality.set(vec![0.1]);
    let coarse = frame(&mut pipeline, &mut renderer);
    let occluded_coarse = count_below(&coarse, 128);
    println!("quality 1: {occluded_fine} occluded; quality 0.1: {occluded_coarse} occluded");
    assert!(
        occluded_fine > occluded,
        "a finer march occludes more ({occluded_fine} vs {occluded})"
    );
    assert!(
        occluded_coarse * 2 < occluded,
        "a coarser march occludes less ({occluded_coarse} vs {occluded})"
    );
    // Both are uniforms, so setting them back gives the first frame back.
    sss_node.quality.set(vec![0.5]);
    let restored = frame(&mut pipeline, &mut renderer);
    assert!(
        restored == full,
        "restoring `thickness` and `quality` restores the frame"
    );

    // Temporal filtering: the ray offset moves every frame, the band stays
    // in front of the box.
    sss_node.set_use_temporal_filtering(true);
    for n in 0..4 {
        let temporal = frame(&mut pipeline, &mut renderer);
        let contact_temporal = mean(&temporal, contact.0, contact.1, 1);
        let open_temporal = mean(&temporal, open.0, open.1, 4);
        println!("temporal frame {n}: contact {contact_temporal:.1}, open {open_temporal:.1}");
        assert_eq!(at(&temporal, 2, 2), 255, "the sky is still clear");
        assert!(
            open_temporal > 225.0,
            "the open floor is still unoccluded ({open_temporal:.1})"
        );
        assert!(
            contact_temporal < 128.0,
            "the floor in front of the box is still occluded ({contact_temporal:.1})"
        );
    }
    sss_node.set_use_temporal_filtering(false);

    // Half resolution: the target is resized and sampled linearly.
    sss_node.set_resolution_scale(0.5);
    let half = frame(&mut pipeline, &mut renderer);
    let contact_half = mean(&half, contact.0, contact.1, 1);
    let open_half = mean(&half, open.0, open.1, 4);
    println!("half resolution: contact {contact_half:.1}, open {open_half:.1}");
    assert!(
        open_half > 225.0,
        "the open floor is unoccluded ({open_half:.1})"
    );
    assert!(
        contact_half < 128.0,
        "the floor in front of the box is occluded at half resolution ({contact_half:.1})"
    );

    // `builtinShadowContext`: the beauty, with and without.
    let setup = box_on_floor(MeshPhongNodeMaterial::phong(Color::from_hex(0xffffff)));
    let (_pre_pass, sss_node) = pre_pass_and_sss(&setup);

    let mut sss_only = RenderPipeline::new();
    sss_only.output_node = Some(sss_node.node());
    let occlusion = frame(&mut sss_only, &mut renderer);

    let plain_pass = pass(setup.scene.clone(), setup.camera.clone());
    let mut plain = RenderPipeline::new();
    plain.output_node = Some(plain_pass.texture_node("output"));
    let without = frame(&mut plain, &mut renderer);

    // One context node for the pass's lifetime, as `set_context_shadow`
    // advises: it is part of the receivers' program key.
    let context = sss_node.sample(screen_uv()).x();
    let shadowed_pass = pass(setup.scene.clone(), setup.camera.clone());
    shadowed_pass.set_context_shadow(context.clone(), &setup.light);
    let mut shadowed = RenderPipeline::new();
    shadowed.output_node = Some(shadowed_pass.texture_node("output"));
    let with = frame(&mut shadowed, &mut renderer);

    let contact_without = mean(&without, contact.0, contact.1, 1);
    let contact_with = mean(&with, contact.0, contact.1, 1);
    let unoccluded_changed = (0..occlusion.len())
        .filter(|&i| occlusion[i] == 255 && with[i].abs_diff(without[i]) > 1)
        .count();
    let brightened = (0..with.len()).filter(|&i| with[i] > without[i]).count();
    println!(
        "beauty: contact {contact_without:.1} -> {contact_with:.1}, {unoccluded_changed} unoccluded pixels changed"
    );
    assert_eq!(
        unoccluded_changed, 0,
        "where the SSS is white the shadow context changes nothing"
    );
    assert_eq!(brightened, 0, "the shadow context only darkens");
    assert!(
        contact_with < contact_without - 40.0,
        "the shadow context darkens the floor in front of the box ({contact_without:.1} -> {contact_with:.1})"
    );

    // `scenePass.contextNode = null` — the page's "Scene with Shadow Maps"
    // — gives the plain frame back.
    shadowed_pass.clear_context_shadow();
    let cleared = frame(&mut shadowed, &mut renderer);
    assert!(
        cleared == without,
        "clearing the context restores the frame"
    );

    // A floor that does not receive shadows has no shadow node for the
    // context to multiply into.
    setup.floor.borrow_mut().receive_shadow = false;
    let unreceived_without = frame(&mut plain, &mut renderer);
    shadowed_pass.set_context_shadow(context, &setup.light);
    let unreceived_with = frame(&mut shadowed, &mut renderer);
    let unreceived_contact = mean(&unreceived_with, contact.0, contact.1, 1);
    println!("without receiveShadow: contact {unreceived_contact:.1}");
    assert!(
        unreceived_with == unreceived_without,
        "without receiveShadow the context changes nothing"
    );
}
