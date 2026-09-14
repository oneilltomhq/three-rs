//! #42's gate: a `BatchedText` whose node sits at the origin and whose members
//! sit far away must still be drawn.
//!
//! The failure this pins down is silent. The batch node is one `InstancedMesh`
//! over a unit `PlaneGeometry( 1, 1 )` whose quads come from `positionNode`, so
//! the geometry's bounding sphere is a half-unit blob at the node's origin and
//! nothing about the members' placement reaches the frustum cull. Point a
//! camera at the members, leave the node at the origin, and the renderer drops
//! the whole batch: a black frame, no warning, indistinguishable from a layout
//! bug.
//!
//! Two halves, because only one of them needs a GPU:
//!
//! - `the_bounding_sphere_spans_the_members` is arithmetic — the sphere
//!   `sync()` computes must cover the distant glyphs, and the frustum that
//!   accepts it must reject the sphere the geometry alone would have offered.
//! - `distant_members_still_render` puts the same scene through the renderer and
//!   counts lit pixels. Before the fix it counted zero.

use std::rc::Rc;

use sdf_text::{Anchor, BatchedText, BatchedTextOptions, Text, VectorFont};
use three_rs::math::{Frustum, Matrix4, Sphere, Vector3};
use three_rs::{Color, OrthographicCamera, Renderer, RendererParameters, Scene};

const WIDTH: f64 = 256.0;
const HEIGHT: f64 = 256.0;
/// How far off its own node the member is placed, in world units — far enough
/// that the node's origin is nowhere near the frustum.
const FAR_X: f64 = 40.0;
/// The orthographic view height; the width is the same, the aspect being 1.
const VIEW_H: f64 = 6.0;

fn roboto() -> VectorFont {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/assets/Roboto-Regular.ttf");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    VectorFont::parse(bytes, "tests/assets/Roboto-Regular.ttf").unwrap()
}

/// The batch node stays at the origin; its one member is at `( FAR_X, 0, 0 )`.
fn batch_with_a_distant_member() -> BatchedText {
    let mut batch = BatchedText::new(
        4,
        64,
        BatchedTextOptions {
            outline_width: 0.0,
            outline_color: None,
            atlas_size: 256,
        },
    );
    batch.set_font(Rc::new(roboto()));

    let mut text = Text::new();
    text.set_text("Far");
    text.set_font_size(1.0);
    text.set_anchor_x(Anchor::named("center"));
    text.set_anchor_y(Anchor::named("middle"));
    let id = batch.add_text(text) as usize;
    batch.set_color_at(id, Color::new(1.0, 1.0, 1.0));
    batch
        .member_node(id)
        .expect("the member was just added")
        .borrow_mut()
        .position
        .set(FAR_X, 0.0, 0.0);

    batch.sync();
    batch
}

/// Looking down -z at `( FAR_X, 0 )`, so the batch node's own origin is 40 units
/// to the left of the frustum's left plane.
fn camera_on_the_member() -> OrthographicCamera {
    let half = VIEW_H / 2.0;
    let mut camera = OrthographicCamera::new(-half, half, half, -half, -10.0, 10.0);
    camera.object.position.set(FAR_X, 0.0, 5.0);
    camera.update_projection_matrix();
    camera.update_matrix_world();
    camera
}

#[test]
fn the_bounding_sphere_spans_the_members() {
    let batch = batch_with_a_distant_member();

    let sphere = batch
        .bounding_sphere()
        .expect("sync() packed a glyph, so the batch has a sphere");
    assert!(
        (sphere.center.x - FAR_X).abs() < 1.0,
        "the sphere should sit on the member, not on the node: {sphere:?}"
    );
    assert!(
        sphere.radius > 0.0 && sphere.radius < 3.0,
        "one one-unit label is a small sphere: {sphere:?}"
    );

    let camera = camera_on_the_member();
    let mut proj_screen = Matrix4::identity();
    proj_screen.multiply_matrices(&camera.projection_matrix, &camera.matrix_world_inverse);
    let mut frustum = Frustum::default();
    frustum.set_from_projection_matrix(&proj_screen, camera.coordinate_system, false);

    batch.node().update_matrix_world(true);
    assert!(
        frustum.intersects_object(batch.node()),
        "the batch is inside the frustum its member fills"
    );

    // And the sphere the geometry alone offers — the unit plane at the node's
    // origin — is not, which is exactly why the cull used to drop the batch.
    let half_diagonal = (0.5f64 * 0.5 + 0.5 * 0.5).sqrt();
    assert!(
        !frustum.intersects_sphere(&Sphere::new(Vector3::ZERO, half_diagonal)),
        "the node's origin is far outside this frustum, which is the premise"
    );
}

#[test]
fn distant_members_still_render() {
    let batch = batch_with_a_distant_member();
    let mut camera = camera_on_the_member();

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    scene.add(batch.node());
    batch.node().update_matrix_world(true);

    let mut renderer = Renderer::new(RendererParameters { antialias: false }).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(WIDTH, HEIGHT);
    renderer.render(&mut scene, &mut camera);

    let (w, h, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((w as f64, h as f64), (WIDTH, HEIGHT));

    // White glyphs on black: any non-black pixel is ink.
    let lit = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 8)
        .count();
    assert!(
        lit > 200,
        "the batch was culled: {lit} lit pixels in a {w}x{h} frame"
    );
}
