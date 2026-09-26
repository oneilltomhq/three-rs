//! The texture caches let go of what the consumer drops (issue #158).
//!
//! three.js frees a texture's GPU side from `texture.dispose()`; the port has
//! no dispose event, and reads the handle's strong count instead, the way the
//! geometry cache has since #58. Each step below swaps the one mesh's material
//! onto a new texture, drops the old one, and checks that the renderer's
//! entries for it are gone on the second render after the drop: the first
//! still finds it held by the material's built program, which is replaced
//! during that render's draw, and the sweep runs before the draw.
//!
//! Every assertion compares two consecutive renders, well inside the view and
//! bind-group caches' age-out window (`CACHE_GRACE_FRAMES`, four frames), so
//! an entry that goes here went because its texture was dropped, not because
//! it aged.
//!
//! One test function: each renderer builds its own device, and cargo runs test
//! functions inside a binary concurrently.

use std::rc::Rc;

use three_rs::nodes::tsl::{cube_texture, normal_world, texture};
use three_rs::nodes::NodeRef;
use three_rs::textures::Image;
use three_rs::{
    plane_geometry, Color, CubeTexture, Mesh, MeshBasicNodeMaterial, Node, OrthographicCamera,
    RenderTarget, Renderer, RendererParameters, Scene, Texture,
};

const SIZE: u32 = 32;

/// `new OrthographicCamera( -1, 1, 1, -1, 0.1, 100 )` at `z = 10`: the 2x2
/// plane at the origin fills the canvas.
fn camera() -> OrthographicCamera {
    let mut camera = OrthographicCamera::new(-1.0, 1.0, 1.0, -1.0, 0.1, 100.0);
    camera.object.position.z = 10.0;
    camera.update_matrix_world();
    camera
}

/// A 2x2 plane whose material's `colorNode` the steps swap.
fn plane(material: MeshBasicNodeMaterial) -> Node {
    Mesh::new(Rc::new(plane_geometry(2.0, 2.0, 1, 1)), material)
}

/// `mesh.material.colorNode = node; mesh.material.needsUpdate = true`.
fn set_color_node(mesh: &Node, node: Option<NodeRef>) {
    let mut object = mesh.borrow_mut();
    let material = object
        .mesh_mut()
        .and_then(|mesh| mesh.material.as_mut())
        .expect("three-rs: the plane has a material");
    material.color_node = node;
    material.set_needs_update();
}

fn solid_texture(rgba: [u8; 4]) -> Texture {
    Texture::new(2, 2, Some(rgba.repeat(4)))
}

fn solid_cube(rgba: [u8; 4]) -> CubeTexture {
    CubeTexture::new(vec![Image::rgba8(1, 1, rgba.to_vec()); 6])
}

/// The canvas' centre pixel, RGB.
fn centre(renderer: &mut Renderer) -> [u8; 3] {
    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
    let at = ((h as usize / 2) * w as usize + w as usize / 2) * 4;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// One render, and what it left resident: `( textures, views, bind groups )`.
fn frame(
    renderer: &mut Renderer,
    scene: &mut Scene,
    camera: &mut OrthographicCamera,
) -> (usize, usize, usize) {
    renderer.render(scene, camera);
    let (_, views, bind_groups) = renderer.binding_cache_lens();
    (renderer.info().memory.textures, views, bind_groups)
}

/// The render after a drop against the render of the drop: one texture-map
/// entry fewer (`textures` of them — a render target's texture is not in the
/// maps), one view fewer and one bind group fewer, and nothing built.
///
/// One render apart, so nothing else can have aged out between the two: every
/// entry the second has lost, it lost to the liveness sweep.
#[track_caller]
fn assert_swept(
    renderer: &Renderer,
    at_drop: (usize, usize, usize),
    after: (usize, usize, usize),
    textures: usize,
    what: &str,
) {
    assert_eq!(
        after,
        (at_drop.0 - textures, at_drop.1 - 1, at_drop.2 - 1),
        "{what}: ( textures, views, bind groups ) at the drop and on the next render"
    );
    assert_eq!(
        renderer.info().build.total(),
        0,
        "{what}: a render whose sweep only drops entries builds nothing"
    );
}

#[test]
fn dropped_textures_leave_the_caches() {
    let mut renderer =
        Renderer::new(RendererParameters { antialias: false }).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);
    let mut camera = camera();

    // -- 2D ----------------------------------------------------------------
    let red = solid_texture([255, 0, 0, 255]);
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(texture(&red));
    let mesh = plane(material);
    let mut scene = Scene::new();
    scene.add(&mesh);

    let first = frame(&mut renderer, &mut scene, &mut camera);
    assert!(first.0 >= 1, "the red texture should be resident");
    assert_eq!(centre(&mut renderer), [255, 0, 0]);
    // A second frame, so the baseline is a steady one.
    let baseline = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(baseline, first);
    assert_eq!(
        renderer.info().build.total(),
        0,
        "the steady frame built nothing"
    );

    let green = solid_texture([0, 255, 0, 255]);
    set_color_node(&mesh, Some(texture(&green)));
    drop(red);
    // The material's cached program still holds `red` when this render
    // sweeps; its draw rebuilds the program and uploads `green` beside it.
    let at_drop = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(
        at_drop.0,
        baseline.0 + 1,
        "red is still held by the material's old program, green is uploaded"
    );
    assert_eq!(centre(&mut renderer), [0, 255, 0]);
    // Now nothing holds `red`, and this render's sweep drops its texture, its
    // view and the bind group built from that view.
    let after = frame(&mut renderer, &mut scene, &mut camera);
    assert_swept(&renderer, at_drop, after, 1, "a dropped 2D texture");
    assert_eq!(
        (after.0, after.1),
        (baseline.0, baseline.1),
        "one texture and one view resident again, green's"
    );

    // -- cube ----------------------------------------------------------------
    let blue = solid_cube([0, 0, 255, 255]);
    set_color_node(&mesh, Some(cube_texture(&blue, normal_world())));
    drop(green);
    let at_drop = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(centre(&mut renderer), [0, 0, 255]);
    let after = frame(&mut renderer, &mut scene, &mut camera);
    assert_swept(
        &renderer,
        at_drop,
        after,
        1,
        "a 2D texture dropped for a cube",
    );

    // -- render target -------------------------------------------------------
    // A yellow plane drawn into a target, and the target's texture sampled.
    let target = RenderTarget::new(SIZE, SIZE);
    {
        let mut target_scene = Scene::new();
        let mut yellow = MeshBasicNodeMaterial::new();
        yellow.color = Color::from_hex(0xffff00);
        target_scene.add(&plane(yellow));
        renderer.set_render_target(Some(target.clone()));
        renderer.render(&mut target_scene, &mut camera);
        renderer.set_render_target(None);
    }
    set_color_node(&mesh, Some(texture(&target.texture())));
    drop(blue);
    let at_drop = frame(&mut renderer, &mut scene, &mut camera);
    assert_eq!(centre(&mut renderer), [255, 255, 0]);
    let after = frame(&mut renderer, &mut scene, &mut camera);
    assert_swept(
        &renderer,
        at_drop,
        after,
        1,
        "a cube dropped for a render target",
    );
    assert_eq!(
        after.0,
        baseline.0 - 1,
        "a render target's texture is not in the renderer's texture maps"
    );

    // Back to a flat colour, and the target dropped: its view and the bind
    // group sampling it go now, not when they age out.
    set_color_node(&mesh, None);
    drop(target);
    let at_drop = frame(&mut renderer, &mut scene, &mut camera);
    let after = frame(&mut renderer, &mut scene, &mut camera);
    assert_swept(&renderer, at_drop, after, 0, "a dropped render target");
}
