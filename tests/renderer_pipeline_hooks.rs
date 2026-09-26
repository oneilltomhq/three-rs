//! `RenderPipeline::on_before_render` / `on_after_render` — three's
//! `OnBeforeRenderPipeline` / `OnAfterRenderPipeline` (issue #154 decision 2).
//!
//! Builds a `Renderer`, so it needs a GPU adapter and is not in CI's no-GPU
//! list. One `#[test]`: each `Renderer::new` builds its own device.
//!
//! What is asserted is the order three's `RenderPipeline.render()` runs
//! things in: every before-hook, in registration order, then the quad's one
//! draw, then every after-hook, in registration order — on every render. The
//! draw count the renderer's `info` holds when each hook runs is the witness
//! for "before" and "after" the draw. The jitter hooks are the shape TRAA
//! will use: a `set_view_offset` through `&mut dyn RenderCamera` before, and
//! `clear_view_offset` after, leaving the camera as it was.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::nodes::tsl::vec4;
use three_rs::{OrthographicCamera, RenderCamera, RenderPipeline, Renderer, RendererParameters};

#[test]
fn hooks_run_in_order_around_the_draw() {
    let mut renderer =
        Renderer::new(RendererParameters { antialias: false }).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(16.0, 16.0);

    let log: Rc<RefCell<Vec<(&'static str, u64)>>> = Rc::default();
    let camera: Rc<RefCell<dyn RenderCamera>> = Rc::new(RefCell::new(OrthographicCamera::new(
        -1.0, 1.0, 1.0, -1.0, 0.1, 10.0,
    )));
    let unjittered = camera.borrow().projection_matrix();
    let jittered: Rc<RefCell<Vec<bool>>> = Rc::default();

    let mut pipeline = RenderPipeline::new();
    pipeline.output_node = Some(vec4(1.0, 0.0, 0.0, 1.0));

    for name in ["before 0", "before 1"] {
        let log = log.clone();
        pipeline.on_before_render(Box::new(move |renderer: &mut Renderer| {
            log.borrow_mut().push((name, renderer.info().render.calls));
        }));
    }
    {
        let camera = camera.clone();
        pipeline.on_before_render(Box::new(move |renderer: &mut Renderer| {
            let (width, height) = renderer.drawing_buffer_size();
            let (width, height) = (width as f64, height as f64);
            camera
                .borrow_mut()
                .set_view_offset(width, height, 0.25, -0.25, width, height);
        }));
    }
    for name in ["after 0", "after 1"] {
        let log = log.clone();
        pipeline.on_after_render(Box::new(move |renderer: &mut Renderer| {
            log.borrow_mut().push((name, renderer.info().render.calls));
        }));
    }
    {
        let camera = camera.clone();
        let jittered = jittered.clone();
        pipeline.on_after_render(Box::new(move |_: &mut Renderer| {
            // Still jittered here: this is the first thing to undo it.
            jittered
                .borrow_mut()
                .push(camera.borrow().projection_matrix() != unjittered);
            camera.borrow_mut().clear_view_offset();
        }));
    }

    renderer.info_mut().auto_reset = false;
    renderer.info_mut().reset();
    let calls = renderer.info().render.calls;

    pipeline.render(&mut renderer);
    pipeline.render(&mut renderer);

    assert_eq!(
        *log.borrow(),
        vec![
            ("before 0", calls),
            ("before 1", calls),
            ("after 0", calls + 1),
            ("after 1", calls + 1),
            ("before 0", calls + 1),
            ("before 1", calls + 1),
            ("after 0", calls + 2),
            ("after 1", calls + 2),
        ]
    );
    assert_eq!(*jittered.borrow(), vec![true, true]);
    assert_eq!(camera.borrow().projection_matrix(), unjittered);
}
