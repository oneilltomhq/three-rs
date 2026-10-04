//! Port of `three.js/examples/jsm/tsl/display/StereoPassNode.js`.
//!
//! The scene rendered twice into one target, side by side: the left eye of a
//! [`StereoCamera`] (`aspect = 0.5`) into the left half, the right eye into
//! the right half.
//!
//! Three's class is a `PassNode` subclass that overrides `updateBefore()`;
//! here the [`PassNode`] is a field and the update-before registry entry for
//! its textures is pointed at this node's state, as for
//! [`CompositeState`](super::stereo_composite_pass::CompositeState). The
//! bracket around the two renders — sizing the target, `renderTarget.samples
//! = renderer.samples`, toggling previous textures, `cameraNear` /
//! `cameraFar`, `setRenderTarget()` / `setMRT()` — is `PassNode`'s own.
//!
//! # Divergences
//!
//! * The eyes' projection matrices are the source camera's, which in the
//!   port is a WebGPU-style (`[0, 1]` depth) projection by default; on
//!   `webgpu_display_stereo` three's camera keeps the WebGL-style matrix its
//!   constructor built, which `StereoCamera.update()` copies. Both renders
//!   then clip near at the same distance in the port, while three's eyes clip
//!   at roughly twice `near`. Nothing on the page is that close.
//! * `renderTarget.scissorTest = true` and the two `scissor.set()` calls are
//!   not ported: `WebGPURenderer` reads the scissor test from the canvas
//!   target only (`Renderer.js`, `_renderScene()`), so in three they have no
//!   effect, while the port's renderer honours a render target's scissor.
//!   The viewports confine each eye to its half, as they do in three.
//!
//! # Not ported
//!
//! * `RendererUtils.resetRendererState()` saves and restores only what the
//!   pass changes; see `RendererState`.

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, StereoCamera};
use crate::nodes::frame::register_texture_update;
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::renderer::{PassNode, Renderer, SceneRef};

use super::stereo_composite_pass::{
    update_source_camera, RendererState, RENDERER_COORDINATE_SYSTEM,
};

/// `stereoPass( scene, camera )`.
pub fn stereo_pass(scene: SceneRef, camera: Rc<RefCell<PerspectiveCamera>>) -> StereoPassNode {
    StereoPassNode::new(scene, camera)
}

/// What a stereo pass shares with the update-before registry.
struct StereoState {
    /// The `PassNode` this class extends.
    pass: PassNode,
    /// `this.scene`.
    scene: SceneRef,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this.stereo`.
    stereo: RefCell<StereoCamera>,
}

/// `StereoPassNode` — see the module docs.
pub struct StereoPassNode(Rc<StereoState>);

impl StereoPassNode {
    /// `StereoPassNode.type`.
    pub const TYPE: &'static str = "StereoPassNode";

    /// `new StereoPassNode( scene, camera )`.
    pub fn new(scene: SceneRef, camera: Rc<RefCell<PerspectiveCamera>>) -> Self {
        let mut stereo = StereoCamera::new();
        stereo.aspect = 0.5;
        let state = Rc::new(StereoState {
            pass: PassNode::new(),
            scene,
            camera,
            stereo: RefCell::new(stereo),
        });
        // The pass's textures are filled by this node's `updateBefore()`,
        // not by `PassNode`'s: the registration overwrites the pass's own.
        register_texture_update(state.pass.texture().id(), &state);
        register_texture_update(state.pass.depth_texture().id(), &state);
        Self(state)
    }

    /// `getTextureNode()` — both eyes side by side.
    pub fn node(&self) -> NodeRef {
        self.0.pass.node()
    }

    /// `this.stereo` — `aspect` `0.5`, `eyeSep` the page's slider.
    pub fn stereo(&self) -> RefMut<'_, StereoCamera> {
        self.0.stereo.borrow_mut()
    }
}

impl NodeUpdate for StereoState {
    /// `PassNode`'s `updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `StereoPassNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let state = RendererState::reset(renderer);

        //

        update_source_camera(&self.camera);
        let (near, far) = {
            let camera = self.camera.borrow();
            let mut stereo = self.stereo.borrow_mut();
            stereo.camera_l.coordinate_system = RENDERER_COORDINATE_SYSTEM;
            stereo.camera_r.coordinate_system = RENDERER_COORDINATE_SYSTEM;
            stereo.update(&camera);
            (camera.near, camera.far)
        };

        let render_target = self.pass.render_target().clone();
        let scene = self.scene.borrow();
        let mut stereo = self.stereo.borrow_mut();
        // `setSize()`, the previous textures, `_cameraNear` / `_cameraFar`,
        // `setRenderTarget( renderTarget )` and `setMRT( this._mrt )`.
        self.pass.render_with(renderer, near, far, |renderer| {
            renderer.auto_clear = false;
            renderer.clear(true, true);

            let (width, height) = render_target.size();
            let (width, height) = (f64::from(width), f64::from(height));

            render_target.set_viewport(0.0, 0.0, width / 2.0, height);
            renderer.render_nested(&scene, &mut stereo.camera_l);

            render_target.set_viewport(width / 2.0, 0.0, width / 2.0, height);
            renderer.render_nested(&scene, &mut stereo.camera_r);
        });

        // restore
        state.restore(renderer);
        true
    }

    /// `PassNode.setup()`'s `renderTarget.samples = renderer.samples`.
    fn sync_before_build(&self, samples: u32) {
        self.pass.render_target().set_samples(samples);
    }
}
