//! Typed vertex attributes on the GPU (issue #294): a `color` attribute that
//! is not a `Float32Array` reaches the fragment stage as the floats three
//! would read.
//!
//! Three quads side by side, each in its own colour layout, through one
//! `MeshBasicNodeMaterial` with `vertexColors`:
//!
//! * a normalized `Uint8BufferAttribute` of item size 3 — three bytes per
//!   vertex, padded to a 4-byte stride on upload and read as `unorm8x4`;
//! * a normalized `InterleavedBufferAttribute` over a `Uint8Array` of stride
//!   4, offset 0 — the bytes go up as they are, `unorm8x4`, stride 4;
//! * a normalized `Uint16BufferAttribute` of item size 4 (`unorm16x4`), which
//!   `vertexColor()` reads as RGBA.
//!
//! The camera is the pixel camera of `renderer_vertex_colors.rs`, and every
//! colour is a saturated primary (0 or the kind's maximum), so each channel is
//! exactly 0 or 1 and stays 0 or 255 through the sRGB transfer.

use std::rc::Rc;

use three_rs::core::{
    ArrayKind, BufferAttribute, BufferGeometry, InterleavedBuffer, StorageBufferAttribute,
    TypedArray,
};
use three_rs::nodes::tsl::storage;
use three_rs::nodes::Type;
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, OrthographicCamera, Renderer, RendererParameters, Scene,
};

const W: usize = 96;
const H: usize = 32;
const THIRD: f32 = (W / 3) as f32;

fn rgb(pixels: &[u8], column: usize, row: usize) -> [u8; 3] {
    let at = (row * W + column) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// The quad covering the `slot`-th third of the canvas, as an indexed pair of
/// triangles, with `color` as its colour attribute.
fn quad(slot: usize, color: BufferAttribute) -> Rc<BufferGeometry> {
    let x0 = THIRD * slot as f32;
    let x1 = x0 + THIRD;
    let h = H as f32;
    let mut geometry = BufferGeometry::new();
    geometry.set_attribute(
        "position",
        BufferAttribute::new(
            vec![
                x0, 0.0, 0.0, //
                x1, 0.0, 0.0, //
                x1, h, 0.0, //
                x0, h, 0.0,
            ],
            3,
        ),
    );
    geometry.set_attribute("color", color);
    geometry.set_index(&[0, 1, 2, 0, 2, 3]);
    Rc::new(geometry)
}

fn material() -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    material.vertex_colors = true;
    material
}

/// The middle pixel of the `slot`-th third.
fn middle(pixels: &[u8], slot: usize) -> [u8; 3] {
    rgb(pixels, slot * W / 3 + W / 6, H / 2)
}

struct Fixture {
    scene: Scene,
    camera: OrthographicCamera,
    renderer: Renderer,
    interleaved: Rc<InterleavedBuffer>,
}

impl Fixture {
    fn new() -> Self {
        let red = BufferAttribute::uint8([255, 0, 0].repeat(4), 3, true);

        // RGBA bytes, the fourth unused: the view reads three of each four.
        let interleaved = Rc::new(InterleavedBuffer::new([0u8, 255, 0, 7].repeat(4), 4));
        let green = BufferAttribute::interleaved(interleaved.clone(), 3, 0, true);

        let blue = BufferAttribute::uint16([0, 0, 65535, 65535].repeat(4), 4, true);

        let mut scene = Scene::new();
        scene.set_background(Color::from_hex(0x000000));
        for (slot, color) in [red, green, blue].into_iter().enumerate() {
            scene.add(&Mesh::new(quad(slot, color), material()));
        }

        let camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0);
        let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
        renderer.set_pixel_ratio(1.0);
        renderer.set_size(W as f64, H as f64);
        Self {
            scene,
            camera,
            renderer,
            interleaved,
        }
    }

    fn render(&mut self) -> Vec<u8> {
        self.renderer.render(&mut self.scene, &mut self.camera);
        let (width, height, pixels) = self.renderer.read_canvas_pixels().unwrap();
        assert_eq!((width as usize, height as usize), (W, H));
        pixels
    }
}

#[test]
fn normalized_integer_colours_read_as_floats() {
    let mut fixture = Fixture::new();
    let pixels = fixture.render();
    assert_eq!(middle(&pixels, 0), [255, 0, 0], "Uint8 item size 3, padded");
    assert_eq!(
        middle(&pixels, 1),
        [0, 255, 0],
        "interleaved Uint8, stride 4"
    );
    assert_eq!(middle(&pixels, 2), [0, 0, 255], "Uint16 RGBA");
}

/// `interleavedBuffer.needsUpdate = true` re-uploads the shared bytes.
#[test]
fn an_interleaved_buffer_update_reaches_the_gpu() {
    let mut fixture = Fixture::new();
    let pixels = fixture.render();
    assert_eq!(middle(&pixels, 1), [0, 255, 0]);

    for vertex in 0..4 {
        fixture
            .interleaved
            .set(&[255.0, 0.0, 255.0], vertex * fixture.interleaved.stride());
    }
    fixture.interleaved.set_needs_update();

    let pixels = fixture.render();
    assert_eq!(middle(&pixels, 1), [255, 0, 255], "the new bytes are drawn");
    assert_eq!(
        middle(&pixels, 0),
        [255, 0, 0],
        "the neighbours are untouched"
    );
}

/// A `StorageBufferAttribute` over a non-normalized `Uint16Array`, read as a
/// vertex attribute: `createAttribute()` widens it to `Uint32Array` like any
/// other (so the format is `uint32x4` / `uint32x3`), and pads item size 3 to
/// a `vec4` stride, so the buffer is 16 bytes per item either way. The
/// colours are 0 or 1, which `vertexColor()` converts to the floats 0 and 1.
#[test]
fn a_uint16_storage_attribute_is_widened_on_upload() {
    let green =
        StorageBufferAttribute::new(TypedArray::U16([0, 1, 0, 1].repeat(4)), 4, ArrayKind::U16);
    let blue = StorageBufferAttribute::new(TypedArray::U16([0, 0, 1].repeat(4)), 3, ArrayKind::U16);

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    let geometries: Vec<Rc<BufferGeometry>> = [&green, &blue]
        .into_iter()
        .enumerate()
        .map(|(slot, color)| quad(slot, color.clone().into()))
        .collect();
    for geometry in &geometries {
        scene.add(&Mesh::new(geometry.clone(), material()));
    }
    let mut camera = OrthographicCamera::new(0.0, W as f64, H as f64, 0.0, -1.0, 1.0);
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);

    renderer.render(&mut scene, &mut camera);
    let (_, _, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!(
        middle(&pixels, 0),
        [0, 255, 0],
        "Uint16 storage, item size 4"
    );
    assert_eq!(
        middle(&pixels, 1),
        [0, 0, 255],
        "Uint16 storage, item size 3"
    );
    assert_eq!(middle(&pixels, 2), [0, 0, 0], "nothing drawn");

    // What the draw uploaded: one `u32` per element, item size 3 padded to 4.
    let words = renderer
        .read_storage_buffer_u32(&storage(&green, Type::UVec4, 4))
        .unwrap();
    assert_eq!(words, [0, 1, 0, 1].repeat(4));
    let words = renderer
        .read_storage_buffer_u32(&storage(&blue, Type::UVec3, 4))
        .unwrap();
    assert_eq!(words, [0, 0, 1, 0].repeat(4));

    // A frame with nothing changed writes nothing (`info().build` is per
    // frame).
    renderer.render(&mut scene, &mut camera);
    assert_eq!(renderer.info().build.buffers_written, 0);

    // `attribute.needsUpdate = true` writes the array over the buffer, once.
    let color = geometries[0].get_attribute("color").unwrap();
    *color.data_mut() = TypedArray::U16([1, 0, 0, 1].repeat(4));
    color.set_needs_update();
    renderer.render(&mut scene, &mut camera);
    assert_eq!(renderer.info().build.buffers_written, 1);
    let (_, _, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!(
        middle(&pixels, 0),
        [255, 0, 0],
        "the new contents are drawn"
    );
    assert_eq!(
        middle(&pixels, 1),
        [0, 0, 255],
        "the neighbour is untouched"
    );
}
