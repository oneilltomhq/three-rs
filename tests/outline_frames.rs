//! `outline()`, over frames. The page's graded frame has nothing selected, so
//! its e2e rung never draws `OutlineNode`'s passes; its shaders are gated
//! against three's dump in `tests/nodes_display_wgsl.rs`, and what the passes
//! draw is checked here, reading the outline node straight to the canvas
//! (`outputColorTransform = false`):
//!
//! * with nothing selected the composite is the zeroed target: no outline;
//! * a selected box in the open gets a ring of *visible* edge (red, the
//!   `visibleEdge` channel) just outside its silhouette, and nothing inside
//!   it — `mask.r` is 0 where the selection drew — or far from it. The
//!   scene's background is not drawn into the mask, or the ring would vanish;
//! * the same box behind a larger unselected one gets a *hidden* edge (green)
//!   ring instead, with no red;
//! * an orthographic camera takes `prepareMask`'s other branch and gives the
//!   same visible ring;
//! * clearing the selection clears the composite on the next frame.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::cameras::OrthographicCamera;
use three_rs::geometries::box_geometry;
use three_rs::nodes::display::{outline, OutlineNode, OutlineParams};
use three_rs::{
    Color, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderPipeline, Renderer,
    RendererParameters, Scene,
};

const SIZE: u32 = 64;

/// The red and green channels of every canvas pixel.
fn frame(pipeline: &mut RenderPipeline, renderer: &mut Renderer) -> Vec<(u8, u8)> {
    pipeline.render(renderer);
    let (width, height, pixels) = renderer.read_canvas_pixels().unwrap();
    assert_eq!((width, height), (SIZE, SIZE));
    pixels.chunks(4).map(|p| (p[0], p[1])).collect()
}

/// A white unlit box.
fn white_box(size: f64) -> three_rs::Node {
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xffffff);
    Mesh::new(Rc::new(box_geometry(size, size, size, 1, 1, 1)), material)
}

/// The scene drawn plainly, white on black: where the selected box is.
fn silhouette(
    renderer: &mut Renderer,
    scene: &Rc<RefCell<Scene>>,
    camera: &mut dyn three_rs::RenderCamera,
) -> Vec<bool> {
    renderer.render(&mut scene.borrow_mut(), camera);
    let (_, _, pixels) = renderer.read_canvas_pixels().unwrap();
    pixels.chunks(4).map(|p| p[0] > 128).collect()
}

/// Distance in pixels (Chebyshev) from `(x, y)` to the nearest pixel of
/// `inside`, up to `limit`.
fn distance_to(inside: &[bool], x: i32, y: i32, limit: i32) -> Option<i32> {
    let size = SIZE as i32;
    (0..=limit).find(|&r| {
        (-r..=r).any(|dy| {
            (-r..=r).any(|dx| {
                let (px, py) = (x + dx, y + dy);
                (0..size).contains(&px)
                    && (0..size).contains(&py)
                    && inside[(py * size + px) as usize]
            })
        })
    })
}

/// Every lit pixel is outside the silhouette and within a few pixels of it;
/// there are enough of them to go round it. Returns the lit pixels'
/// channels.
fn assert_ring(pixels: &[(u8, u8)], inside: &[bool], what: &str) -> Vec<(u8, u8)> {
    let size = SIZE as i32;
    let lit: Vec<(usize, (u8, u8))> = pixels
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, (r, g))| *r > 2 || *g > 2)
        .collect();
    assert!(lit.len() >= 40, "{what}: only {} outline pixels", lit.len());
    for &(i, (r, g)) in &lit {
        let (x, y) = (i as i32 % size, i as i32 / size);
        assert!(
            !inside[i],
            "{what}: ({x}, {y}) = ({r}, {g}) is inside the selection"
        );
        assert!(
            distance_to(inside, x, y, 6).is_some(),
            "{what}: ({x}, {y}) = ({r}, {g}) is far from the selection"
        );
    }
    lit.into_iter().map(|(_, c)| c).collect()
}

fn pipeline_for(outline_pass: &OutlineNode) -> RenderPipeline {
    let mut pipeline = RenderPipeline::new();
    pipeline.output_color_transform = false;
    pipeline.output_node = Some(outline_pass.node());
    pipeline
}

#[test]
fn outline_draws_visible_and_hidden_edges() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(f64::from(SIZE), f64::from(SIZE));

    let mut scene = Scene::new();
    scene.set_background(Color::from_hex(0x000000));
    let selected = white_box(1.0);
    selected.borrow_mut().set_rotation(0.3, 0.4, 0.0);
    scene.add(&selected);
    let scene = Rc::new(RefCell::new(scene));

    let camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 4.0;
    let camera = Rc::new(RefCell::new(camera));

    let inside = silhouette(&mut renderer, &scene, &mut *camera.borrow_mut());
    assert!(inside.iter().filter(|&&p| p).count() > 100);

    let outline_pass = outline(scene.clone(), camera.clone(), OutlineParams::default());
    let mut pipeline = pipeline_for(&outline_pass);

    // Nothing selected: the composite was never drawn.
    let empty = frame(&mut pipeline, &mut renderer);
    assert!(empty.iter().all(|&(r, g)| r == 0 && g == 0));

    // Selected, in the open: a visible (red) ring.
    outline_pass.set_selected_objects(vec![selected.clone()]);
    let visible = frame(&mut pipeline, &mut renderer);
    let ring = assert_ring(&visible, &inside, "visible");
    assert!(
        ring.iter().all(|&(_, g)| g == 0),
        "a visible edge has no green"
    );
    assert!(
        ring.iter().any(|&(r, _)| r > 40),
        "the visible edge is faint"
    );

    // Behind a larger unselected box: a hidden (green) ring.
    let blocker = white_box(1.6);
    blocker.borrow_mut().position.z = 1.5;
    scene.borrow().add(&blocker);
    let hidden = frame(&mut pipeline, &mut renderer);
    let ring = assert_ring(&hidden, &inside, "hidden");
    assert!(
        ring.iter().all(|&(r, _)| r == 0),
        "a hidden edge has no red"
    );
    assert!(
        ring.iter().any(|&(_, g)| g > 40),
        "the hidden edge is faint"
    );
    scene.borrow().remove(&blocker);

    // Deselected: the composite is cleared once, and stays clear.
    outline_pass.set_selected_objects(Vec::new());
    let cleared = frame(&mut pipeline, &mut renderer);
    assert!(cleared.iter().all(|&(r, g)| r == 0 && g == 0));
    let still = frame(&mut pipeline, &mut renderer);
    assert!(still.iter().all(|&(r, g)| r == 0 && g == 0));

    // An orthographic camera: `orthographicDepthToViewZ()`.
    let mut ortho = OrthographicCamera::new(-1.2, 1.2, 1.2, -1.2, 0.1, 10.0);
    ortho.object.position.z = 4.0;
    let ortho_inside = silhouette(&mut renderer, &scene, &mut ortho);
    let ortho = Rc::new(RefCell::new(ortho));
    let ortho_outline = outline(
        scene.clone(),
        ortho.clone(),
        OutlineParams {
            selected_objects: vec![selected.clone()],
            ..OutlineParams::default()
        },
    );
    let mut pipeline = pipeline_for(&ortho_outline);
    let visible = frame(&mut pipeline, &mut renderer);
    let ring = assert_ring(&visible, &ortho_inside, "orthographic");
    assert!(
        ring.iter().all(|&(_, g)| g == 0),
        "a visible edge has no green"
    );
}
