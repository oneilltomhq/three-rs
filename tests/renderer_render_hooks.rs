//! `Object3D.on_before_render` / `on_after_render` and
//! `Scene.on_before_render` / `on_after_render` — three's per-object and
//! per-scene render callbacks (issue #159, `docs/api.md` decision 12).
//!
//! Builds a `Renderer`, so it needs a GPU adapter and is listed in
//! `tests/gpu_only`. One `#[test]`: each `Renderer::new` builds its own device.
//!
//! What is asserted is three's order in `Renderer._renderScene()`: the scene's
//! before-hook, then each drawn object's before-hook, then the draws, then
//! each object's after-hook, then the scene's after-hook — on every render.
//! The draw count `info` holds when each hook runs is the witness for "before"
//! and "after" the draws. Also: the arguments (the object's own node, the
//! scene, the camera, the target the scene is drawn into), that a culled or
//! invisible object's hooks do not run, that a hook can replace itself, and
//! that `LightProbeHelper` follows its probe through its hook alone.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::addons::helpers::LightProbeHelper;
use three_rs::core::ObjectRef;
use three_rs::geometries::plane_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::math::SphericalHarmonics3;
use three_rs::renderer::RenderTarget;
use three_rs::{
    Color, LightProbe, Mesh, OrthographicCamera, RenderCamera, Renderer, RendererParameters, Scene,
};

type Log = Rc<RefCell<Vec<(String, u64)>>>;

fn quad(name: &str, x: f64) -> ObjectRef {
    let geometry = Rc::new(plane_geometry(4.0, 4.0, 1, 1));
    let mut material = MeshBasicNodeMaterial::new();
    material.color = Color::from_hex(0xff0000);
    let mesh = Mesh::new(geometry, material);
    mesh.borrow_mut().name = name.to_string();
    mesh.borrow_mut().position.set(x, 0.0, 0.0);
    mesh
}

fn hook_object(node: &ObjectRef, log: &Log) {
    let name = node.borrow().name.clone();
    let before = (log.clone(), name.clone());
    node.borrow_mut()
        .set_on_before_render(move |node, renderer, _scene, _camera, group| {
            assert_eq!(node.borrow().name, before.1, "the hook gets its own node");
            assert!(group.is_none(), "a single-material object has no group");
            before
                .0
                .borrow_mut()
                .push((format!("before {}", before.1), renderer.info().render.calls));
        });
    let after = (log.clone(), name);
    node.borrow_mut()
        .set_on_after_render(move |_node, renderer, _scene, _camera, _group| {
            after
                .0
                .borrow_mut()
                .push((format!("after {}", after.1), renderer.info().render.calls));
        });
}

#[test]
fn hooks_run_in_three_order_around_the_draws() {
    let mut renderer =
        Renderer::new(RendererParameters::default()).expect("a wgpu adapter and device");
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(32.0, 32.0);
    let mut camera = OrthographicCamera::new(-10.0, 10.0, 10.0, -10.0, 0.1, 100.0);
    camera.object.position.set(0.0, 0.0, 10.0);
    let camera_id = camera.id();

    let log: Log = Rc::default();
    let mut scene = Scene::new();
    let scene_node = scene.node.clone();

    let a = quad("a", -5.0);
    let b = quad("b", 5.0);
    let hidden = quad("hidden", 0.0);
    hidden.borrow_mut().visible = false;
    let culled = quad("culled", 1000.0);
    for node in [&a, &b, &hidden, &culled] {
        hook_object(node, &log);
        scene.add(node);
    }

    let targets: Rc<RefCell<Vec<Option<RenderTarget>>>> = Rc::default();
    {
        let (log, targets) = (log.clone(), targets.clone());
        let scene_node = scene_node.clone();
        scene.set_on_before_render(move |renderer, scene, camera, target| {
            assert!(
                ObjectRef::ptr_eq(&scene.node, &scene_node),
                "the scene itself"
            );
            assert_eq!(camera.id(), camera_id, "the camera rendered with");
            log.borrow_mut()
                .push(("scene before".to_string(), renderer.info().render.calls));
            targets.borrow_mut().push(target.cloned());
        });
    }
    {
        let (log, targets) = (log.clone(), targets.clone());
        scene.set_on_after_render(move |renderer, _scene, _camera, target| {
            log.borrow_mut()
                .push(("scene after".to_string(), renderer.info().render.calls));
            targets.borrow_mut().push(target.cloned());
        });
    }

    renderer.info_mut().auto_reset = false;
    renderer.info_mut().reset();
    let calls = renderer.info().render.calls;

    renderer.render(&mut scene, &mut camera);
    let first = renderer.info().render.calls;
    assert!(first >= calls + 2, "both quads were drawn");
    renderer.render(&mut scene, &mut camera);
    let second = renderer.info().render.calls;

    let expected = |start: u64, end: u64| {
        vec![
            ("scene before".to_string(), start),
            ("before a".to_string(), start),
            ("before b".to_string(), start),
            ("after a".to_string(), end),
            ("after b".to_string(), end),
            ("scene after".to_string(), end),
        ]
    };
    let mut want = expected(calls, first);
    want.extend(expected(first, second));
    assert_eq!(
        *log.borrow(),
        want,
        "scene before, objects before, draws, objects after, scene after; \
         nothing for the invisible or the culled object"
    );

    // The default output colour space is sRGB, so the scene is drawn into the
    // internal framebuffer target, and that is three's `renderTarget`.
    {
        let targets = targets.borrow();
        assert_eq!(targets.len(), 4);
        let id = targets[0]
            .as_ref()
            .expect("the framebuffer target")
            .texture()
            .id();
        for target in targets.iter() {
            assert_eq!(target.as_ref().map(|t| t.texture().id()), Some(id));
        }
    }

    // Into a render target of the caller's, that is the one the hooks get.
    let own = RenderTarget::new(32, 32);
    renderer.set_render_target(Some(own.clone()));
    targets.borrow_mut().clear();
    renderer.render(&mut scene, &mut camera);
    renderer.set_render_target(None);
    for target in targets.borrow().iter() {
        assert_eq!(
            target.as_ref().map(|t| t.texture().id()),
            Some(own.texture().id()),
            "the caller's render target"
        );
    }

    // A hook that installs another keeps the new one; one that does not is
    // put back.
    log.borrow_mut().clear();
    {
        let log = log.clone();
        a.borrow_mut()
            .set_on_before_render(move |node, renderer, _scene, _camera, _group| {
                log.borrow_mut()
                    .push(("first".to_string(), renderer.info().render.calls));
                let log = log.clone();
                node.borrow_mut()
                    .set_on_before_render(move |_, renderer, _, _, _| {
                        log.borrow_mut()
                            .push(("replacement".to_string(), renderer.info().render.calls));
                    });
            });
    }
    renderer.render(&mut scene, &mut camera);
    renderer.render(&mut scene, &mut camera);
    let names: Vec<String> = log
        .borrow()
        .iter()
        .map(|(name, _)| name.clone())
        .filter(|name| name == "first" || name == "replacement")
        .collect();
    assert_eq!(names, vec!["first", "replacement"]);

    // `LightProbeHelper.onBeforeRender()` moves the helper to its probe; no
    // `update()` call is made after the probe moves.
    let probe = LightProbe::new(SphericalHarmonics3::default(), 1.0);
    let helper = LightProbeHelper::new(&probe, 2.0);
    scene.add(&probe);
    scene.add(&helper.node);
    probe.borrow_mut().position.set(3.0, -2.0, 0.0);
    renderer.render(&mut scene, &mut camera);
    let object = helper.node.borrow();
    assert_eq!(
        (object.position.x, object.position.y, object.position.z),
        (3.0, -2.0, 0.0),
        "the helper followed the probe"
    );
    assert_eq!(object.scale.x, 2.0, "scaled to size");
}
