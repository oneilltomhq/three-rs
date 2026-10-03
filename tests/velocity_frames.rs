//! `VelocityNode`, over frames: the `velocity` MRT output is zero on the
//! first frame an object and a camera are seen, carries the screen-space
//! motion on the frame after one of them moves, and is zero again once they
//! hold still. A skinned mesh moves through its bones (`positionPrevious`
//! skinned with last frame's bones) and a projection override is what the
//! camera history records.
//!
//! `webgpu_postprocessing_motion_blur`'s graded frame is a first frame, where
//! every velocity is zero, so the motion itself is checked here.
//!
//! One `#[test]`: each `Renderer::new` builds its own device, and cargo runs
//! test functions inside a binary concurrently.

use std::cell::RefCell;
use std::rc::Rc;

use three_rs::cameras::RenderCamera;
use three_rs::core::BufferAttribute;
use three_rs::geometries::plane_geometry;
use three_rs::materials::MeshBasicNodeMaterial;
use three_rs::nodes::mrt;
use three_rs::nodes::tsl::{output_property, texture_uv, uv};
use three_rs::nodes::velocity::velocity;
use three_rs::objects::{Bone, Skeleton, SkinnedMesh};
use three_rs::renderer::{RenderTarget, RenderTargetOptions};
use three_rs::textures::TextureType;
use three_rs::{Mesh, PerspectiveCamera, QuadMesh, Renderer, RendererParameters, Scene, Vector3};

const SIZE: u32 = 8;

/// `1 / ( 5 tan 30° )`: the NDC distance one world unit at `z = 0` spans for
/// a 60° camera five units back.
const ONE_UNIT_NDC: f32 = 0.346_410_16;

struct Rig {
    renderer: Renderer,
    target: RenderTarget,
    velocity: three_rs::Texture,
    scene: Scene,
    camera: PerspectiveCamera,
}

impl Rig {
    /// A fresh scene and camera on `renderer`. Velocity history is keyed by
    /// object and camera, so the new ones start from their first frame.
    fn new(renderer: Renderer, scene: Scene) -> Self {
        let mut options = RenderTargetOptions::default();
        options.texture_type = TextureType::HalfFloat;
        let target = RenderTarget::new_with_options(SIZE, SIZE, options)
            .expect("HalfFloatType is a colour type");
        let velocity = target.add_texture("velocity");

        let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
        camera.node.borrow_mut().position.set(0.0, 0.0, 5.0);
        camera.look_at(&Vector3::ZERO);

        Self {
            renderer,
            target,
            velocity,
            scene,
            camera,
        }
    }

    /// One display frame: `PassNode.updateBefore()` — the scene into the
    /// target with `mrt( { output, velocity } )` — then the velocity
    /// attachment's centre texel, sampled into a probe target, then a quad to
    /// the screen. The port has no animation loop; a render to the screen is
    /// what closes a frame, and the camera history rolls once per frame.
    fn frame(&mut self) -> [f32; 2] {
        self.renderer.set_render_target(Some(self.target.clone()));
        self.renderer.set_mrt(Some(mrt(vec![
            ("output", output_property()),
            ("velocity", velocity()),
        ])));
        self.renderer.render(&mut self.scene, &mut self.camera);
        self.renderer.set_mrt(None);

        let mut options = RenderTargetOptions::default();
        options.texture_type = TextureType::HalfFloat;
        options.depth_buffer = false;
        let probe = RenderTarget::new_with_options(SIZE, SIZE, options)
            .expect("HalfFloatType is a colour type");
        let mut material = MeshBasicNodeMaterial::new();
        material.fragment_node = Some(texture_uv(&self.velocity, uv()));
        material.depth_test = false;
        material.depth_write = false;
        self.renderer.set_render_target(Some(probe.clone()));
        self.renderer.render_quad(&QuadMesh::new(material));
        self.renderer.set_render_target(None);
        self.renderer
            .render_quad(&QuadMesh::new(MeshBasicNodeMaterial::new()));

        let pixels = self
            .renderer
            .read_target_pixels_rgba16f(&probe)
            .expect("the probe is rgba16float")
            .2;
        let i = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
        [pixels[i], pixels[i + 1]]
    }
}

fn plane() -> std::rc::Rc<three_rs::core::BufferGeometry> {
    Rc::new(plane_geometry(20.0, 20.0, 1, 1))
}

fn assert_near(actual: [f32; 2], expected: [f32; 2], what: &str) {
    // Half-float storage: ten mantissa bits.
    let close = |a: f32, b: f32| (a - b).abs() <= 1e-3 * b.abs().max(1.0);
    assert!(
        close(actual[0], expected[0]) && close(actual[1], expected[1]),
        "{what}: velocity {actual:?}, expected {expected:?}"
    );
}

#[test]
fn velocity_follows_the_motion() {
    let mut renderer = Renderer::new(RendererParameters::default()).unwrap();
    renderer.set_pixel_ratio(1.0);
    renderer.set_size(SIZE as f64, SIZE as f64);

    let renderer = an_object_that_moves(Rig::new(renderer, Scene::new()));
    let renderer = a_skinned_mesh_whose_bone_moves(renderer);
    a_projection_override_hides_the_jitter(renderer);
}

/// The model matrix's history: zero, then the one-unit move, then zero.
fn an_object_that_moves(mut rig: Rig) -> Renderer {
    let mesh = Mesh::new(plane(), MeshBasicNodeMaterial::new());
    rig.scene.add(&mesh);

    assert_near(rig.frame(), [0.0, 0.0], "the first frame");

    mesh.borrow_mut().position.x = 1.0;
    assert_near(rig.frame(), [ONE_UNIT_NDC, 0.0], "the frame after the move");

    assert_near(rig.frame(), [0.0, 0.0], "the frame after that");

    // The camera's history: the camera moving up one unit is the scene
    // moving down one.
    rig.camera.node.borrow_mut().position.y = 1.0;
    assert_near(
        rig.frame(),
        [0.0, -ONE_UNIT_NDC],
        "the frame after the camera moves",
    );
    assert_near(rig.frame(), [0.0, 0.0], "the camera at rest");
    rig.renderer
}

/// `positionPrevious` skinned with last frame's bones: the mesh never moves,
/// its one bone does.
fn a_skinned_mesh_whose_bone_moves(renderer: Renderer) -> Renderer {
    let mut geometry = plane_geometry(20.0, 20.0, 1, 1);
    let count = geometry.position().unwrap().count();
    geometry.set_attribute("skinIndex", BufferAttribute::new(vec![0.0; count * 4], 4));
    geometry.set_attribute(
        "skinWeight",
        BufferAttribute::new([1.0, 0.0, 0.0, 0.0].repeat(count), 4),
    );
    let mesh = SkinnedMesh::new(Rc::new(geometry), MeshBasicNodeMaterial::new());
    let bone = Bone::new();
    mesh.add(&bone);
    let skeleton = Rc::new(RefCell::new(Skeleton::new(vec![bone.clone()], None)));
    SkinnedMesh::bind(&mesh, skeleton, None);

    let scene = Scene::new();
    scene.add(&mesh);
    let mut rig = Rig::new(renderer, scene);

    assert_near(rig.frame(), [0.0, 0.0], "the skinned mesh's first frame");

    bone.borrow_mut().position.x = 1.0;
    assert_near(
        rig.frame(),
        [ONE_UNIT_NDC, 0.0],
        "the frame after the bone moves",
    );

    assert_near(rig.frame(), [0.0, 0.0], "the bone at rest");
    rig.renderer
}

/// `velocity.setProjectionMatrix( m )`: with the override held at the
/// original projection, a changed camera projection (TRAA's jitter) does not
/// read as motion.
fn a_projection_override_hides_the_jitter(renderer: Renderer) {
    let scene = Scene::new();
    scene.add(&Mesh::new(plane(), MeshBasicNodeMaterial::new()));
    let mut rig = Rig::new(renderer, scene);

    let original = rig.camera.projection_matrix();
    rig.renderer.set_velocity_projection_matrix(Some(original));
    assert_near(rig.frame(), [0.0, 0.0], "the first frame");

    rig.camera
        .set_view_offset(SIZE as f64, SIZE as f64, 0.5, 0.5, SIZE as f64, SIZE as f64);
    assert_near(
        rig.frame(),
        [0.0, 0.0],
        "a jittered frame under the override",
    );

    // Without the override the same jitter is a velocity: last frame's
    // projection is the override, this frame's the jittered one.
    rig.renderer.set_velocity_projection_matrix(None);
    assert_ne!(rig.frame(), [0.0, 0.0], "the jitter, seen");
}
