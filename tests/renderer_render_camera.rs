//! `Renderer::render` takes its camera however the page holds it
//! (`docs/api.md` decision 14): `&mut` a camera the page owns, or the
//! `Rc<RefCell<_>>` a page shares with `pass( scene, camera )`. A shared
//! camera is updated when its cell is free, and a cell that something other
//! than an outer render holds is a caller bug that `render()` names.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::cameras::{PerspectiveCamera, RenderCamera};
use three_rs::renderer::CameraRef;
use three_rs::{Renderer, RendererParameters, Scene};

fn renderer() -> Renderer {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(16.0, 16.0);
    renderer
}

#[test]
fn a_shared_camera_is_updated_when_its_cell_is_free() {
    let mut renderer = renderer();
    let scene = Scene::new();
    let camera = Rc::new(RefCell::new(PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0)));

    camera.borrow().node.borrow_mut().position.z = 3.0;
    renderer.render(&scene, &camera);
    let first = camera.borrow().matrix_world_inverse;

    camera.borrow().node.borrow_mut().position.z = 5.0;
    renderer.render(&scene, &camera);
    let second = camera.borrow().matrix_world_inverse;

    assert_ne!(first, second, "render() updates a shared camera's matrices");
    assert_eq!(
        second.elements[14], -5.0,
        "the view matrix follows the new position"
    );
}

#[test]
fn every_spelling_of_the_camera_renders() {
    let mut renderer = renderer();
    let scene = Scene::new();

    let mut owned = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
    renderer.render(&scene, &mut owned);
    renderer.render(&scene, &mut owned as &mut dyn RenderCamera);

    let cell = RefCell::new(PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0));
    renderer.render(&scene, &cell);
    renderer.render(&scene, &cell as &RefCell<dyn RenderCamera>);

    let shared: CameraRef = Rc::new(RefCell::new(PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0)));
    renderer.render(&scene, &shared);
    renderer.render(&scene, &*shared);
}

#[test]
#[should_panic(expected = "the camera is borrowed, and not by an outer render")]
fn a_shared_camera_held_by_the_caller_is_named_at_render() {
    let mut renderer = renderer();
    let scene = Scene::new();
    let camera = Rc::new(RefCell::new(PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0)));

    let held = camera.borrow();
    renderer.render(&scene, &camera);
    drop(held);
}
