//! `NodeFrame`'s three update phases, driven through a real `Renderer`
//! (issue #162, `docs/nodes.md` §57).
//!
//! Builds a `Renderer`, so it needs a GPU adapter and is not in CI's no-GPU
//! list. The unit tests in `src/nodes/frame.rs` check the guards on their own;
//! this checks that `draw()` calls them where three's `_renderObjectDirect()`
//! does.
//!
//! One custom node, shared by two meshes, asks for all three phases with a
//! different guard each: `updateBefore` once per frame, `update` once per
//! render, `updateAfter` once per object. Each frame renders the scene twice,
//! into a render target and then to the canvas, and it is the canvas render
//! that ends the frame. Over two frames the log has to read, per frame:
//! before, update, after (first mesh), after (second mesh), then update,
//! after, after for the second render.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::nodes::tsl::{
    custom, on_after_object_update, on_before_frame_update, on_before_material_update,
    on_before_object_update, on_frame_update, on_material_update, on_object_update, vec4,
};
use three_rs::nodes::{CustomNode, NodeBuilder, NodeRef, NodeUpdateType, Type};
use three_rs::{
    box_geometry, Mesh, MeshBasicNodeMaterial, PerspectiveCamera, RenderTarget, Renderer,
    RendererParameters, Scene,
};

type Log = Rc<RefCell<Vec<(&'static str, u64)>>>;

/// A node that returns a constant colour and logs each phase with the frame
/// it ran in.
struct Probe {
    log: Log,
}

impl Probe {
    fn push(&self, phase: &'static str, renderer: &Renderer) -> bool {
        self.log
            .borrow_mut()
            .push((phase, renderer.node_frame().frame_id));
        true
    }
}

impl CustomNode for Probe {
    fn type_name(&self) -> &'static str {
        "Probe"
    }

    fn node_type(&self) -> Type {
        Type::Vec4
    }

    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        vec4(1.0, 0.5, 0.25, 1.0)
    }

    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    fn update_type(&self) -> NodeUpdateType {
        NodeUpdateType::Render
    }

    fn update_after_type(&self) -> NodeUpdateType {
        NodeUpdateType::Object
    }

    fn update_before(&self, renderer: &mut Renderer) -> bool {
        self.push("before", renderer)
    }

    fn update(&self, renderer: &mut Renderer) -> bool {
        self.push("update", renderer)
    }

    fn update_after(&self, renderer: &mut Renderer) -> bool {
        self.push("after", renderer)
    }
}

#[test]
fn two_frames_run_each_phase_behind_its_guard() {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(16.0, 16.0);

    let log: Log = Rc::default();
    let probe = custom(Probe { log: log.clone() });

    let mut scene = Scene::new();
    let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    for x in [-0.75, 0.75] {
        let mut material = MeshBasicNodeMaterial::new();
        material.color_node = Some(probe.clone());
        let mesh = Mesh::new(geometry.clone(), material);
        mesh.borrow_mut().position.x = x;
        scene.add(&mesh);
    }

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 4.0;

    let target = RenderTarget::new(16, 16);
    let first_frame = renderer.node_frame().frame_id + 1;

    for _ in 0..2 {
        renderer.set_render_target(Some(target.clone()));
        renderer.render(&mut scene, &mut camera);
        renderer.set_render_target(None);
        renderer.render(&mut scene, &mut camera);
    }

    let frame = |id: u64| {
        vec![
            ("before", id),
            ("update", id),
            ("after", id),
            ("after", id),
            ("update", id),
            ("after", id),
            ("after", id),
        ]
    };
    let expected: Vec<_> = [frame(first_frame), frame(first_frame + 1)].concat();
    assert_eq!(*log.borrow(), expected);
}

/// The `On*Update` hooks (`EventNode.js`), attached with `bypass()` to one
/// colour shared by two meshes and rendered as above: two frames of two
/// renders of two draws. An `OBJECT` hook runs at every draw, a `RENDER`
/// (material) one once per render, a `FRAME` one once per frame.
#[test]
fn event_hooks_run_at_their_phase() {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(16.0, 16.0);

    let log: Log = Rc::default();
    let hook = |tag: &'static str| {
        let log = log.clone();
        move |renderer: &mut Renderer| {
            log.borrow_mut().push((tag, renderer.node_frame().frame_id));
        }
    };
    let color = vec4(1.0, 0.5, 0.25, 1.0)
        .bypass(on_before_frame_update(hook("before frame")))
        .bypass(on_before_material_update(hook("before material")))
        .bypass(on_before_object_update(hook("before object")))
        .bypass(on_frame_update(hook("frame")))
        .bypass(on_material_update(hook("material")))
        .bypass(on_object_update(hook("object")))
        .bypass(on_after_object_update(hook("after object")));

    let mut scene = Scene::new();
    let geometry = Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1));
    for x in [-0.75, 0.75] {
        let mut material = MeshBasicNodeMaterial::new();
        material.color_node = Some(color.clone());
        let mesh = Mesh::new(geometry.clone(), material);
        mesh.borrow_mut().position.x = x;
        scene.add(&mesh);
    }

    let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 10.0);
    camera.node.borrow_mut().position.z = 4.0;

    let target = RenderTarget::new(16, 16);
    let first_frame = renderer.node_frame().frame_id + 1;

    for _ in 0..2 {
        renderer.set_render_target(Some(target.clone()));
        renderer.render(&mut scene, &mut camera);
        renderer.set_render_target(None);
        renderer.render(&mut scene, &mut camera);
    }

    let log = log.borrow();
    let count = |tag: &str| log.iter().filter(|(t, _)| *t == tag).count();
    // 2 frames x 2 renders x 2 draws.
    assert_eq!(count("before object"), 8, "{log:?}");
    assert_eq!(count("object"), 8, "{log:?}");
    assert_eq!(count("after object"), 8, "{log:?}");
    // 2 frames x 2 renders.
    assert_eq!(count("before material"), 4, "{log:?}");
    assert_eq!(count("material"), 4, "{log:?}");
    // 2 frames.
    assert_eq!(count("before frame"), 2, "{log:?}");
    assert_eq!(count("frame"), 2, "{log:?}");
    // The first draw runs the before phase, then the update phase, then the
    // after phase, as `_renderObjectDirect()` does.
    let first: Vec<_> = log.iter().take(7).map(|(t, f)| (*t, *f)).collect();
    let phase = |t: &str| match t {
        "before frame" | "before material" | "before object" => 0,
        "frame" | "material" | "object" => 1,
        _ => 2,
    };
    assert!(
        first.windows(2).all(|w| phase(w[0].0) <= phase(w[1].0)),
        "{first:?}"
    );
    assert!(first.iter().all(|(_, f)| *f == first_frame), "{first:?}");
}
