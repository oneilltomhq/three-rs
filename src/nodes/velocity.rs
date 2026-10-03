//! Port of `three.js/src/nodes/accessors/VelocityNode.js`: `velocity`, the
//! screen-space motion of a fragment since the last frame, in NDC units.
//!
//! The node is the shader half. The bookkeeping half — three's `_objectData`
//! `WeakMap`, which holds each object's previous `matrixWorld` and each
//! camera's previous view and projection — is [`VelocityState`], which lives in
//! the renderer-owned [`NodeFrameState`](crate::nodes::NodeFrameState)
//! (issue #154, decision 1, option C: the frame owns the history, not the
//! node).
//!
//! # When the history moves
//!
//! Three's `VelocityNode` is an `OBJECT` node in both `update` and
//! `updateAfter`. The port runs the same two steps from the renderer's draw
//! loop, and only for a program that binds one of the velocity uniforms
//! ([`NodeProgram::reads_velocity`](crate::nodes::NodeProgram)), so a frame
//! that draws nothing into a `velocity` attachment never touches the store:
//!
//! * **before the draw's bindings are written** ([`VelocityState::update`]):
//!   the object's stored matrix becomes `previousModelWorldMatrix` (seeded with
//!   the current one the first time the object is seen), and — once per
//!   `frameId` per camera — the camera's current view and projection rotate
//!   into its previous ones. On a camera's first frame previous = current.
//! * **after the draw** ([`VelocityState::update_after`]): the object's current
//!   `matrixWorld` is stored for the next frame.
//!
//! So on the first frame everything is at rest: velocity is zero wherever
//! nothing moved *within* the frame, which is every pixel of a graded t = 0
//! frame.
//!
//! Skinned meshes keep a second history, last frame's bone matrices
//! ([`VelocityState::rotate_bones`] / [`VelocityState::previous_bones`]): see
//! [`crate::nodes::skinning`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::core::{Node as SceneNode, WeakNode};
use crate::math::Matrix4;
use crate::nodes::node::{Type, UniformGroup, UniformSource};
use crate::nodes::tsl::{
    float, model_view_matrix, position_local, position_previous, to_const, uniform, vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::Skeleton;

/// `velocity` — `nodeImmutable( VelocityNode )`, a `vec2`.
///
/// `VelocityNode.setup()`:
///
/// ```js
/// const previousModelViewMatrix = this.previousCameraViewMatrix.mul( this.previousModelWorldMatrix );
/// const clipPositionCurrent = this.currentProjectionMatrix.mul( modelViewMatrix ).mul( positionLocal );
/// const clipPositionPrevious = this.previousProjectionMatrix.mul( previousModelViewMatrix ).mul( positionPrevious );
/// return ndcPositionCurrent.sub( ndcPositionPrevious );
/// ```
///
/// Built fresh per call, like the rest of the TSL: the uniforms resolve by
/// source, so two calls key the same program. Read in the fragment stage —
/// which is where an MRT member is — `positionLocal` and `positionPrevious`
/// arrive as varyings, as three's dump of `webgpu_postprocessing_motion_blur`
/// shows.
pub fn velocity() -> NodeRef {
    let previous_model_world = uniform(
        UniformSource::PreviousModelWorldMatrix,
        Type::Mat4,
        UniformGroup::Object,
        None,
    );
    let current_projection = uniform(
        UniformSource::VelocityProjectionMatrix,
        Type::Mat4,
        UniformGroup::Render,
        None,
    );
    let previous_projection = uniform(
        UniformSource::PreviousProjectionMatrix,
        Type::Mat4,
        UniformGroup::Render,
        None,
    );
    let previous_camera_view = uniform(
        UniformSource::PreviousCameraViewMatrix,
        Type::Mat4,
        UniformGroup::Render,
        None,
    );

    let previous_model_view = previous_camera_view.mul(previous_model_world);

    // `mat4.mul( vec3 )` is the `vec4( v, 1 )` `NodeBuilder.format()` pads
    // with; the port spells the padding. Each clip position is read twice
    // (`.xy` and `.w`), which three's `TempNode` turns into a `let`.
    let clip_current = to_const(
        None,
        current_projection
            .mul(model_view_matrix())
            .mul(vec4_join(vec![position_local(), float(1.0)])),
    );
    let clip_previous = to_const(
        None,
        previous_projection
            .mul(previous_model_view)
            .mul(vec4_join(vec![position_previous(), float(1.0)])),
    );

    let ndc_current = clip_current.xy().div(clip_current.w());
    let ndc_previous = clip_previous.xy().div(clip_previous.w());

    ndc_current.sub(ndc_previous)
}

/// The four matrices one velocity draw binds.
#[derive(Clone, Copy, Debug, Default)]
#[doc(hidden)]
pub struct VelocityUniforms {
    pub(crate) previous_model_world: Matrix4,
    pub(crate) current_projection: Matrix4,
    pub(crate) previous_projection: Matrix4,
    pub(crate) previous_camera_view: Matrix4,
}

/// `getData( camera )` — one camera's matrices, rotated once per frame.
#[derive(Clone, Copy, Debug)]
struct CameraHistory {
    /// `cameraData.frameId`.
    frame_id: u64,
    previous_projection: Matrix4,
    previous_view: Matrix4,
    current_projection: Matrix4,
    current_view: Matrix4,
}

/// Three's `_objectData` `WeakMap` and `Skinning.js`'
/// `_previousBoneMatricesData`, owned by the renderer's frame.
///
/// Entries are keyed by id and dropped when the object or skeleton they
/// belong to is, which is what a `WeakMap` does. Camera entries are kept by
/// recency instead (a camera is not always a scene-graph node), and dropped
/// after the same grace the renderer's caches use.
#[derive(Debug, Default)]
pub struct VelocityState {
    /// `getPreviousMatrix( object )`, by `object.id`.
    objects: HashMap<u32, (WeakNode, Matrix4)>,
    /// `getData( camera )`, by camera id, with the frame it was last read.
    cameras: HashMap<u32, (CameraHistory, u64)>,
    /// `_previousBoneMatricesData`, by skeleton address.
    bones: HashMap<usize, (Weak<RefCell<Skeleton>>, Vec<f32>)>,
    /// `VelocityNode.projectionMatrix` — what `setProjectionMatrix()` set.
    /// Three's `velocity` is one shared node, so one override serves every
    /// draw; `TRAANode` sets it to the unjittered projection for the length of
    /// its scene pass.
    projection_override: Option<Matrix4>,
}

impl VelocityState {
    /// `velocity.setProjectionMatrix( matrix )`.
    pub(crate) fn set_projection_matrix(&mut self, matrix: Option<Matrix4>) {
        self.projection_override = matrix;
    }

    /// `VelocityNode.update( { frameId, camera, object } )`: the matrices this
    /// draw binds. `object` is `None` for a draw with no scene object, which
    /// is then treated as never having moved.
    pub(crate) fn update(
        &mut self,
        frame_id: u64,
        object: Option<&SceneNode>,
        model_world: Matrix4,
        camera_id: u32,
        projection: Matrix4,
        view: Matrix4,
    ) -> VelocityUniforms {
        let previous_model_world = match object {
            Some(node) => {
                let id = node.borrow().id;
                self.objects
                    .entry(id)
                    .or_insert_with(|| (node.downgrade(), model_world))
                    .1
            }
            None => model_world,
        };

        let projection = self.projection_override.unwrap_or(projection);
        let (camera, touched) = self.cameras.entry(camera_id).or_insert_with(|| {
            (
                CameraHistory {
                    // Not this frame, so the rotation below runs.
                    frame_id: frame_id.wrapping_sub(1),
                    previous_projection: projection,
                    previous_view: view,
                    current_projection: projection,
                    current_view: view,
                },
                frame_id,
            )
        });
        *touched = frame_id;
        if camera.frame_id != frame_id {
            camera.frame_id = frame_id;
            camera.previous_projection = camera.current_projection;
            camera.previous_view = camera.current_view;
            camera.current_projection = projection;
            camera.current_view = view;
        }

        VelocityUniforms {
            previous_model_world,
            current_projection: camera.current_projection,
            previous_projection: camera.previous_projection,
            previous_camera_view: camera.previous_view,
        }
    }

    /// `VelocityNode.updateAfter( { object } )`:
    /// `getPreviousMatrix( object ).copy( object.matrixWorld )`.
    pub(crate) fn update_after(&mut self, object: Option<&SceneNode>, model_world: Matrix4) {
        if let Some(node) = object {
            let id = node.borrow().id;
            self.objects
                .entry(id)
                .or_insert_with(|| (node.downgrade(), model_world))
                .1 = model_world;
        }
    }

    /// The skinning `OnObjectUpdate`'s `previousBoneMatrices.set(
    /// skeleton.boneMatrices )`, run right before `skeleton.update()` — and
    /// only for a skeleton some velocity draw has asked for, so the check is
    /// one lookup in an empty map everywhere else.
    pub(crate) fn rotate_bones(&mut self, skeleton: &Rc<RefCell<Skeleton>>) {
        if self.bones.is_empty() {
            return;
        }
        let key = Rc::as_ptr(skeleton) as *const u8 as usize;
        if let Some((_, previous)) = self.bones.get_mut(&key) {
            previous.clone_from(&skeleton.borrow().bone_matrices);
        }
    }

    /// `getPreviousSkinnedPosition()`'s `previousBoneMatrices`: last frame's
    /// bones, seeded with this frame's on first sight. Three seeds at build
    /// time, after a `skeleton.update()`; the port's skeleton has already been
    /// updated this frame by the time any draw asks, so it is the same array.
    pub(crate) fn previous_bones(&mut self, skeleton: &Rc<RefCell<Skeleton>>) -> Vec<f32> {
        let key = Rc::as_ptr(skeleton) as *const u8 as usize;
        self.bones
            .entry(key)
            .or_insert_with(|| {
                (
                    Rc::downgrade(skeleton),
                    skeleton.borrow().bone_matrices.clone(),
                )
            })
            .1
            .clone()
    }

    /// Drop the history of objects and skeletons that are gone, and of cameras
    /// nothing has drawn through for `grace` frames.
    pub(crate) fn sweep(&mut self, frame_id: u64, grace: u64) {
        if !self.objects.is_empty() {
            self.objects.retain(|_, (node, _)| node.upgrade().is_some());
        }
        if !self.bones.is_empty() {
            self.bones
                .retain(|_, (skeleton, _)| skeleton.strong_count() > 0);
        }
        if !self.cameras.is_empty() {
            let cutoff = frame_id.saturating_sub(grace);
            self.cameras.retain(|_, (_, touched)| *touched >= cutoff);
        }
    }

    /// How many objects have a stored previous matrix — for tests.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Object3D;

    fn translation(x: f64) -> Matrix4 {
        let mut m = Matrix4::identity();
        m.elements[12] = x;
        m
    }

    #[test]
    fn the_first_frame_is_at_rest_and_the_second_sees_the_move() {
        let mut state = VelocityState::default();
        let object = Object3D::new_node();
        let projection = translation(0.5);
        let view = Matrix4::identity();

        let first = state.update(1, Some(&object), translation(1.0), 7, projection, view);
        assert_eq!(first.previous_model_world, translation(1.0));
        assert_eq!(first.previous_projection, projection);
        state.update_after(Some(&object), translation(1.0));

        let moved_view = translation(-2.0);
        let second = state.update(
            2,
            Some(&object),
            translation(3.0),
            7,
            projection,
            moved_view,
        );
        assert_eq!(second.previous_model_world, translation(1.0));
        assert_eq!(second.previous_camera_view, view);
        state.update_after(Some(&object), translation(3.0));

        // A second draw through the same camera in the same frame does not
        // rotate the camera again.
        let again = state.update(
            2,
            Some(&object),
            translation(3.0),
            7,
            projection,
            moved_view,
        );
        assert_eq!(again.previous_camera_view, view);
        assert_eq!(again.previous_model_world, translation(3.0));
    }

    #[test]
    fn the_projection_override_is_what_the_camera_history_records() {
        let mut state = VelocityState::default();
        let jittered = translation(0.25);
        let unjittered = translation(0.0);
        state.set_projection_matrix(Some(unjittered));
        let uniforms = state.update(
            1,
            None,
            Matrix4::identity(),
            1,
            jittered,
            Matrix4::identity(),
        );
        assert_eq!(uniforms.current_projection, unjittered);
        assert_eq!(uniforms.previous_projection, unjittered);
    }

    #[test]
    fn a_dropped_object_loses_its_history() {
        let mut state = VelocityState::default();
        let object = Object3D::new_node();
        state.update_after(Some(&object), Matrix4::identity());
        assert_eq!(state.object_count(), 1);
        drop(object);
        state.sweep(10, 4);
        assert_eq!(state.object_count(), 0);
    }
}
