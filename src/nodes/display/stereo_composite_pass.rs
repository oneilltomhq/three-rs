//! Port of `three.js/examples/jsm/tsl/display/StereoCompositePassNode.js`,
//! the abstract base of [`AnaglyphPassNode`](super::AnaglyphPassNode) and
//! [`ParallaxBarrierPassNode`](super::ParallaxBarrierPassNode). Every stereo
//! pass brackets its render with `RendererUtils.resetRendererState()` and
//! `restoreRendererState()`, which are the renderer's `reset_state()` scope.
//!
//! Three's class is a `PassNode` subclass: its `renderTarget` is the pass's,
//! its `getTextureNode()` the pass's, and `PassNode.setup()` still sets
//! `renderTarget.samples = renderer.samples`. Here the [`PassNode`] is a
//! field, kept for exactly those three things; the update-before registry
//! entry for its textures is pointed at [`CompositeState`] instead, which
//! renders the two eyes and the composite where `PassNode.updateBefore()`
//! would have rendered the scene once.
//!
//! The default `updateStereoCamera()` runs `stereo.update()` with the port's
//! WebGPU-style (`[0, 1]` depth) eye projections, so the parallax barrier's
//! eyes clip near at `near` where three's, copied from a WebGL-style matrix,
//! clip at `2fn / (f + n)`; see [`StereoPassNode`](super::StereoPassNode).
//! The anaglyph override is unaffected: `frameCorners` writes a WebGL-style
//! matrix in both.

use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, StereoCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::CoordinateSystem;
use crate::nodes::frame::register_texture_update;
use crate::nodes::{NodeRef, NodeUpdate, NodeUpdateType};
use crate::objects::QuadMesh;
use crate::renderer::{PassNode, RenderTarget, RenderTargetOptions, Renderer, SceneRef};
use crate::textures::{Texture, TextureFilter, TextureType};

/// The renderer's `coordinateSystem`, which the stereo passes copy onto
/// their eye cameras. A `WebGPURenderer`'s is always WebGPU's.
pub(super) const RENDERER_COORDINATE_SYSTEM: CoordinateSystem = CoordinateSystem::WebGpu;

/// `camera.matrixWorld` as the stereo passes read it — after updating it.
///
/// Three never updates the source camera's world matrix in a stereo pass:
/// the camera is not in the scene and is never rendered itself, so what
/// `stereo.update( camera )` and `updateStereoCamera()` read is whatever the
/// application last left there. On `webgpu_display_stereo` that is
/// `OrbitControls.update()`'s `lookAt()`, which refreshes the world matrix
/// *before* it writes the new quaternion, so three's eyes trail the controls
/// by one update. [`PerspectiveCamera::look_at`] does not touch the world
/// matrix at all, which would leave the eyes wherever the camera was first
/// rendered from, so the port updates it here, the current pose and not the
/// previous one. At rest — the graded frame — the two agree.
///
/// The borrow is unconditional, as three's update is: a caller holding a
/// borrow of the camera across the pipeline's render panics here rather than
/// render from stale eyes. No caller in the crate does — the example's
/// `controls_and_camera()` borrow is tied to `&mut App`, so it cannot be
/// alive across `animate()`.
pub(super) fn update_source_camera(camera: &RefCell<PerspectiveCamera>) {
    camera.borrow_mut().update_matrix_world();
}

/// `AnaglyphPassNode`'s own fields, which turn
/// [`CompositeState::update_stereo_camera`] into its override.
pub(super) struct AnaglyphEyes {
    /// `AnaglyphPassNode.eyeSep`.
    pub(super) eye_sep: Cell<f64>,
    /// `AnaglyphPassNode.planeDistance`.
    pub(super) plane_distance: Cell<f64>,
}

/// What a stereo composite pass shares with the update-before registry.
pub(crate) struct CompositeState {
    /// The `PassNode` this class extends: its target is the composite's, its
    /// node is the pass's output.
    pass: PassNode,
    /// `this.scene`.
    scene: SceneRef,
    /// `this.camera`.
    camera: Rc<RefCell<PerspectiveCamera>>,
    /// `this.stereo`.
    stereo: RefCell<StereoCamera>,
    /// `this._renderTargetL`.
    target_l: RenderTarget,
    /// `this._renderTargetR`.
    target_r: RenderTarget,
    /// `_quadMesh` with `this._material`.
    quad: RefCell<QuadMesh>,
    /// `Some` for an `AnaglyphPassNode`, whose `updateStereoCamera()` frames
    /// a virtual screen instead of calling `stereo.update()`.
    anaglyph: Option<AnaglyphEyes>,
}

impl CompositeState {
    /// `new StereoCompositePassNode( scene, camera )`, then the derived
    /// class's `setup()`: `material` is given the two eye textures
    /// (`this._mapLeft` / `this._mapRight`'s) and builds `this._material`.
    pub(super) fn new(
        scene: SceneRef,
        camera: Rc<RefCell<PerspectiveCamera>>,
        anaglyph: Option<AnaglyphEyes>,
        material: impl FnOnce(&Texture, &Texture) -> MeshBasicNodeMaterial,
    ) -> Rc<Self> {
        // `{ minFilter: LinearFilter, magFilter: NearestFilter, type:
        // HalfFloatType }` — and `RenderTarget`'s own `depthBuffer: true`.
        let options = || RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer: true,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Nearest,
        };
        let target_l = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: a stereo eye target is a colour type");
        let target_r = RenderTarget::new_with_options(1, 1, options())
            .expect("three-rs: a stereo eye target is a colour type");
        let quad = QuadMesh::new(material(&target_l.texture(), &target_r.texture()));

        let pass = PassNode::new();
        let state = Rc::new(Self {
            pass,
            scene,
            camera,
            stereo: RefCell::new(StereoCamera::new()),
            target_l,
            target_r,
            quad: RefCell::new(quad),
            anaglyph,
        });
        // The pass's textures are filled by this state's `updateBefore()`,
        // not by `PassNode`'s: the registration overwrites the pass's own.
        register_texture_update(state.pass.texture().id(), &state);
        register_texture_update(state.pass.depth_texture().id(), &state);
        state
    }

    /// `getTextureNode()` — the composite, for the graph downstream.
    pub(super) fn node(&self) -> NodeRef {
        self.pass.node()
    }

    /// `this.stereo`.
    pub(super) fn stereo(&self) -> RefMut<'_, StereoCamera> {
        self.stereo.borrow_mut()
    }

    /// `this._material`.
    pub(super) fn quad(&self) -> std::cell::Ref<'_, QuadMesh> {
        self.quad.borrow()
    }

    /// `AnaglyphPassNode`'s `eyeSep` and `planeDistance`; `None` for a
    /// `ParallaxBarrierPassNode`.
    pub(super) fn anaglyph(&self) -> Option<&AnaglyphEyes> {
        self.anaglyph.as_ref()
    }

    /// `updateStereoCamera( coordinateSystem )`:
    /// `StereoCompositePassNode`'s, or `AnaglyphPassNode`'s override.
    pub(super) fn update_stereo_camera(&self, coordinate_system: CoordinateSystem) {
        let camera = self.camera.borrow();
        let mut stereo = self.stereo.borrow_mut();
        match &self.anaglyph {
            None => {
                stereo.camera_l.coordinate_system = coordinate_system;
                stereo.camera_r.coordinate_system = coordinate_system;
                stereo.update(&camera);
            }
            Some(eyes) => super::anaglyph_pass::update_stereo_camera(
                &mut stereo,
                &camera,
                eyes.eye_sep.get(),
                eyes.plane_distance.get(),
                coordinate_system,
            ),
        }
    }

    /// `StereoCompositePassNode.setSize( width, height )`.
    fn set_size(&self, width: u32, height: u32) {
        let target = self.pass.render_target();
        target.set_size(width, height);
        let (width, height) = target.size();
        self.target_l.set_size(width, height);
        self.target_r.set_size(width, height);
    }
}

impl NodeUpdate for CompositeState {
    /// `PassNode`'s `updateBeforeType = NodeUpdateType.FRAME`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `StereoCompositePassNode.updateBefore( frame )`.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let mut renderer = renderer.reset_state();

        //

        update_source_camera(&self.camera);
        self.update_stereo_camera(RENDERER_COORDINATE_SYSTEM);

        let (width, height) = renderer.drawing_buffer_size();
        self.set_size(width, height);
        // `PassNode.setup()`: `renderTarget.samples = renderer.samples`.
        self.pass.render_target().set_samples(renderer.samples());

        {
            let scene = self.scene.borrow();
            let mut stereo = self.stereo.borrow_mut();

            // left
            renderer.set_render_target(Some(self.target_l.clone()));
            renderer.render_nested(&scene, &mut stereo.camera_l);

            // right
            renderer.set_render_target(Some(self.target_r.clone()));
            renderer.render_nested(&scene, &mut stereo.camera_r);
        }

        // composite
        renderer.set_render_target(Some(self.pass.render_target().clone()));
        renderer.render_quad(&self.quad.borrow());

        // restore
        drop(renderer);
        true
    }

    /// `PassNode.setup()`'s `renderTarget.samples = renderer.samples`, ahead
    /// of any build that binds the pass's textures (see
    /// [`PassNode`]'s own).
    fn sync_before_build(&self, samples: u32) {
        self.pass.render_target().set_samples(samples);
    }
}
