//! `viewportDepthTexture()` on a renderer with `stencil: true`.
//!
//! The canvas depth buffer is then `depth24plus-stencil8`, and the viewport
//! depth copy takes the same format (a texture-to-texture copy needs it). A
//! combined depth-stencil texture can only be bound as `texture_depth_2d`
//! through a view of its depth aspect alone; a view of both aspects fails
//! bind-group validation, so the frame below would not render.

use std::rc::Rc;

use three_rs::nodes::display::viewport_depth_texture;
use three_rs::nodes::tsl::vec3_join;
use three_rs::{
    plane_geometry, Color, Mesh, MeshBasicNodeMaterial, ObjectRef, OrthographicCamera, Renderer,
    RendererParameters, Scene,
};

const W: u32 = 16;
const H: u32 = 16;

/// A 2x2 plane at `z` with `material`.
fn quad(material: MeshBasicNodeMaterial, z: f64) -> ObjectRef {
    let mesh = Mesh::new(Rc::new(plane_geometry(2.0, 2.0, 1, 1)), material);
    mesh.borrow_mut().position.z = z;
    mesh
}

/// An opaque red quad, then a transparent one in front of it whose colour
/// is the depth the red one left: every pixel is that grey, not red.
#[test]
fn viewport_depth_reads_a_depth_stencil_buffer() {
    let mut parameters = RendererParameters::default();
    parameters.stencil = true;
    let mut renderer = Renderer::new(parameters).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(W as f64, H as f64);

    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 100.0);
    camera.object.position.z = 10.0;
    camera.update_matrix_world();

    let mut scene = Scene::new();
    scene.add(&quad(
        MeshBasicNodeMaterial::line(Color::from_hex(0xff0000)),
        0.0,
    ));
    let mut depth = MeshBasicNodeMaterial::new();
    let d = viewport_depth_texture();
    depth.color_node = Some(vec3_join(vec![d.clone(), d.clone(), d]));
    depth.transparent = true;
    scene.add(&quad(depth, 1.0));

    renderer.render(&mut scene, &mut camera);
    let (w, h, pixels) = renderer.read_canvas_pixels().expect("canvas readback");
    assert_eq!((w, h), (W, H));
    let [r, g, b] = [pixels[0], pixels[1], pixels[2]];
    assert!(
        r == g && g == b && r > 0 && r < 255,
        "pixel 0 is {:?}",
        [r, g, b]
    );
}
