//! Port of `three.js/examples/jsm/tsl/display/ParallaxBarrierPassNode.js`.
//!
//! The two eyes of a [`StereoCamera`] interleaved by row, for a parallax
//! barrier display: a fragment whose `screenCoordinate.y` is odd
//! (`mod( y, 2 ) > 1` at the pixel centre `y + 0.5`) shows the left eye, an
//! even one the right.
//!
//! # Not ported
//!
//! * `material.contextNode = context( builder.getSharedContext() )`: the
//!   port's quad materials build in their own context, as every other
//!   display node's do.
//! * `dispose()`: the targets and the material are dropped with the node.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::cameras::{PerspectiveCamera, StereoCamera};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::CoordinateSystem;
use crate::nodes::tsl::{
    block, if_else, mod_float, screen_coordinate, texture_sample, to_var, uv, vec4,
};
use crate::nodes::NodeRef;
use crate::objects::QuadMesh;
use crate::renderer::SceneRef;
use crate::textures::Texture;

use super::stereo_composite_pass::CompositeState;

/// `parallaxBarrierPass( scene, camera )`.
pub fn parallax_barrier_pass(
    scene: SceneRef,
    camera: Rc<RefCell<PerspectiveCamera>>,
) -> ParallaxBarrierPassNode {
    ParallaxBarrierPassNode::new(scene, camera)
}

/// `ParallaxBarrierPassNode` — see the module docs.
pub struct ParallaxBarrierPassNode {
    state: Rc<CompositeState>,
}

impl ParallaxBarrierPassNode {
    /// `ParallaxBarrierPassNode.type`.
    pub const TYPE: &'static str = "ParallaxBarrierPassNode";

    /// `new ParallaxBarrierPassNode( scene, camera )`, with the material
    /// `setup()` builds — an unnamed `NodeMaterial`, as three's is.
    pub fn new(scene: SceneRef, camera: Rc<RefCell<PerspectiveCamera>>) -> Self {
        let state = CompositeState::new(scene, camera, None, |map_left, map_right| {
            let mut material = MeshBasicNodeMaterial::new();
            material.fragment_node = Some(parallax_barrier_node(map_left, map_right));
            material
        });
        Self { state }
    }

    /// `getTextureNode()` — the interleaved image, for the graph downstream.
    pub fn node(&self) -> NodeRef {
        self.state.node()
    }

    /// `this.stereo` — its `eyeSep` is the page's slider; its `aspect` is
    /// `1`, as each eye fills the whole target.
    pub fn stereo(&self) -> RefMut<'_, StereoCamera> {
        self.state.stereo()
    }

    /// `updateStereoCamera( coordinateSystem )` — what `updateBefore()`
    /// calls with the renderer's coordinate system.
    pub fn update_stereo_camera(&self, coordinate_system: CoordinateSystem) {
        self.state.update_stereo_camera(coordinate_system);
    }

    /// The quad material, for `examples/dump_wgsl.rs` and the dump gate.
    #[doc(hidden)]
    pub fn quad_material(&self) -> MeshBasicNodeMaterial {
        let quad: Ref<'_, QuadMesh> = self.state.quad();
        quad.material.clone()
    }
}

/// `ParallaxBarrierPassNode.setup()`'s `parallaxBarrier` `Fn()`.
fn parallax_barrier_node(map_left: &Texture, map_right: &Texture) -> NodeRef {
    let uv_node = uv();
    // `vec4().toVar()` — `Vector4`'s default, `( 0, 0, 0, 1 )`.
    let color = to_var(None, vec4(0.0, 0.0, 0.0, 1.0));
    block(
        vec![
            color.clone(),
            if_else(
                mod_float(screen_coordinate().y(), 2.0).greater_than(1.0),
                vec![color.assign(texture_sample(map_left, uv_node.clone()))],
                vec![color.assign(texture_sample(map_right, uv_node))],
            ),
        ],
        color,
    )
}
