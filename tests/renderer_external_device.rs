//! The compositor-embedding seam, drawn: a renderer built on a device the test
//! owns, sampling a `wgpu::Texture` the test created and filled, and then a
//! CPU-backed texture whose bytes change between two frames.
//!
//! Each half is a silent-wrong-output failure if it regresses:
//!
//! - **`Renderer::with_device` really adopts the device.** The external texture
//!   is created on the test's device. If the renderer had made one of its own,
//!   binding that texture would be a wgpu validation error, not a wrong pixel —
//!   so this half fails loudly either way, which is the point.
//! - **`Texture::external` samples the foreign texture as it stands.** Four
//!   distinct texels, read back at the four corners of the frame. A renderer
//!   that tried to re-upload it (`own_gpu` leaking back to true) would panic on
//!   the missing image data; one that ignored the handle would draw black.
//! - **`set_data` reaches the GPU on the next frame.** The second frame must
//!   show the new bytes in the texture that was already uploaded. Without the
//!   version in the cache key the first upload is what the cache returns
//!   forever, and the frame is silently stale.
//! - **The update is in place.** The `wgpu::Texture` behind the CPU texture is
//!   the same one before and after, so the allocation and every bind group
//!   built from it survive the change.
//!
//! The camera is `renderer_lines.rs`' pixel camera: `OrthographicCamera( 0, W,
//! H, 0, -1, 1 )` at the origin makes one world unit one pixel. The quad is a
//! `PlaneGeometry` the size of the frame, moved to its centre, so UV `( 0, 0 )`
//! is the bottom-left pixel and `( 1, 1 )` the top-right.
//!
//! One `#[test]`, run on one device: cargo runs test functions in a binary
//! concurrently and the GPU is shared.

use std::rc::Rc;

use three_rs::geometries::plane_geometry;
use three_rs::nodes::tsl::texture;
use three_rs::textures::ColorSpace;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, OrthographicCamera, Renderer, RendererParameters, Scene,
    Texture,
};

const W: u32 = 64;
const H: u32 = 64;

/// Pure channels only, so the assertion holds whatever transfer function the
/// output pass applies: 0 stays 0 and 1 stays 255 under any of them.
const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

/// `new OrthographicCamera( 0, W, H, 0, -1, 1 )`.
fn pixel_camera() -> OrthographicCamera {
    OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0)
}

/// A full-frame quad textured with `map`, on a black background.
///
/// The sample goes through `colorNode = texture( map )` rather than
/// `material.map`, which only the lit materials read in this port: the point
/// here is the texture reaching the GPU, and an unlit quad puts the texel
/// straight into the framebuffer with nothing else in the way.
fn scene_with(map: Texture) -> Scene {
    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));

    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(texture(&map));

    let quad = Mesh::new(Rc::new(plane_geometry(W as f64, H as f64, 1, 1)));
    quad.borrow_mut().mesh_mut().unwrap().material = Some(material);
    quad.borrow_mut()
        .position
        .set(W as f64 / 2.0, H as f64 / 2.0, 0.0);
    scene.add(&quad);

    scene
}

/// The RGB of framebuffer pixel `( column, row )`, row 0 at the top.
fn rgb(pixels: &[u8], column: u32, row: u32) -> [u8; 3] {
    let at = ((row * W + column) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

#[track_caller]
fn assert_rgb(pixels: &[u8], column: u32, row: u32, expected: [u8; 4], what: &str) {
    assert_eq!(
        rgb(pixels, column, row),
        [expected[0], expected[1], expected[2]],
        "{what}: pixel ({column}, {row})"
    );
}

/// An `rgba8unorm` 2×2 texture on the test's own device, filled with four
/// distinct texels: `[ RED, GREEN ]` on row 0 and `[ BLUE, WHITE ]` on row 1.
fn filled_2x2(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let gpu = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("the compositor's imported dmabuf"),
        size: wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let mut texels = Vec::with_capacity(16);
    for texel in [RED, GREEN, BLUE, WHITE] {
        texels.extend_from_slice(&texel);
    }

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &gpu,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &texels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
    );

    gpu
}

/// A solid `width × height` RGBA8 image.
fn solid(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    color
        .iter()
        .copied()
        .cycle()
        .take((width * height * 4) as usize)
        .collect()
}

#[test]
fn adopted_device_external_texture_and_in_place_update() {
    // The host's device: what a Smithay compositor would already have, with
    // its imported dmabufs and its scanout surfaces on it.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    // The same adapter the rest of the suite grades on, chosen the same way.
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    let adapter = adapters
        .iter()
        .find(|a| a.get_info().device_type == wgpu::DeviceType::IntegratedGpu)
        .or_else(|| adapters.first())
        .expect("no wgpu adapter")
        .clone();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("the host's device"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("no wgpu device");

    let mut renderer = Renderer::with_device(
        RendererParameters { antialias: false },
        adapter,
        device.clone(),
        queue.clone(),
    );
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);

    // -- the external texture --------------------------------------------

    let external = Texture::external(filled_2x2(&device, &queue), ColorSpace::NoColorSpace);
    assert_eq!(external.size(), (2, 2), "size comes off the wgpu texture");
    assert_eq!(external.format(), wgpu::TextureFormat::Rgba8Unorm);

    let mut scene = scene_with(external.clone());
    let mut camera = pixel_camera();
    renderer.render(&mut scene, &mut camera);
    let (width, height, pixels) = renderer.read_canvas_pixels();
    assert_eq!((width, height), (W, H));

    // Texture row 0 is at `v = 0`, which the camera puts at the bottom of the
    // frame — an external texture has no `flipY` upload to turn it over. The
    // four sample points are inside the first and last half-texel, where a
    // linear sampler clamps and the texel's own colour comes back exactly.
    assert_rgb(&pixels, 2, H - 3, RED, "external, texel (0, 0)");
    assert_rgb(&pixels, W - 3, H - 3, GREEN, "external, texel (1, 0)");
    assert_rgb(&pixels, 2, 2, BLUE, "external, texel (0, 1)");
    assert_rgb(&pixels, W - 3, 2, WHITE, "external, texel (1, 1)");

    // -- the in-place update ---------------------------------------------

    let map = Texture::new(4, 4, Some(solid(4, 4, RED)));
    let mut scene = scene_with(map.clone());
    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels();
    assert_rgb(&pixels, W / 2, H / 2, RED, "first frame");

    let before = map.with_gpu(|gpu| gpu.clone());

    assert_eq!(map.version(), 0);
    map.set_data(solid(4, 4, BLUE));
    assert_eq!(map.version(), 1, "set_data is the needsUpdate too");

    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels();
    assert_rgb(&pixels, W / 2, H / 2, BLUE, "after set_data");

    // The same allocation, written into — not a new texture. A re-created one
    // would draw the right pixels and quietly throw away the bind groups.
    let after = map.with_gpu(|gpu| gpu.clone());
    assert!(
        before == after,
        "set_data must write into the existing GPU texture"
    );

    // A frame with no version change must not re-upload anything, and must not
    // go stale either.
    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels();
    assert_rgb(&pixels, W / 2, H / 2, BLUE, "unchanged third frame");
}
