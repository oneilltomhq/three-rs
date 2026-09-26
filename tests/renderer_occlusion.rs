//! The occlusion queries behind `webgpu_occlusion` (`docs/nodes.md` §36),
//! checked where the graded frame cannot see them.
//!
//! The graded frame is the first, and a query's answer is at least two frames
//! away (three maps the *previous* `finishRender()`'s buffer), so the plane is
//! blue on it whether or not the queries work. These run the page's own frame
//! loop on and read the plane's centre pixel:
//!
//! - with the sphere behind the plane, the plane turns green once the answer
//!   lands, and stays green;
//! - with the sphere moved in front of the plane, it never does — which is what
//!   tells a query that counted samples from one that was never recorded.

#[path = "../examples/webgpu_occlusion.rs"]
#[allow(dead_code)]
mod example;

/// The plane's centre, which the sphere covers only when it is in front.
fn centre(app: &mut example::App) -> [u8; 4] {
    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    let i = ((height / 2 * width + width / 2) * 4) as usize;
    pixels[i..i + 4].try_into().unwrap()
}

fn is_blue(px: [u8; 4]) -> bool {
    px[2] > 100 && px[1] < 40 && px[0] < 40
}

fn is_green(px: [u8; 4]) -> bool {
    px[1] > 100 && px[2] < 40 && px[0] < 40
}

/// The corner of the plane, clear of the sphere wherever it sits.
fn plane_corner(app: &mut example::App) -> [u8; 4] {
    let (width, height, pixels) = app.renderer.read_canvas_pixels().unwrap();
    // The plane spans roughly 320..480 x 170..330 at 800 x 500.
    let (x, y) = (width / 2 - 70, height / 2 - 70);
    let i = ((y * width + x) * 4) as usize;
    pixels[i..i + 4].try_into().unwrap()
}

#[test]
fn a_hidden_sphere_turns_the_plane_green_after_the_query_lands() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = example::init();

    example::animate(&mut app);
    let first = centre(&mut app);
    assert!(is_blue(first), "frame 1 is not blue: {first:?}");

    example::animate(&mut app);
    let second = centre(&mut app);
    assert!(
        is_blue(second),
        "frame 2 already has an answer, before three's could: {second:?}"
    );

    for frame in 3..=6 {
        example::animate(&mut app);
        let px = centre(&mut app);
        assert!(is_green(px), "frame {frame} is not green: {px:?}");
    }
}

#[test]
fn a_visible_sphere_leaves_the_plane_blue() {
    three_rs::testing::pin_time(Some(0.0));
    let mut app = example::init();

    // In front of the plane, where every one of its fragments passes.
    let sphere = app.scene.node.borrow().children[3].clone();
    assert!(sphere.borrow().occlusion_test);
    sphere.borrow_mut().position.z = 1.0;

    for frame in 1..=6 {
        example::animate(&mut app);
        let px = plane_corner(&mut app);
        assert!(is_blue(px), "frame {frame}'s plane is not blue: {px:?}");
    }
}
