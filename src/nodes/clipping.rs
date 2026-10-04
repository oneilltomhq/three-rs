//! Port of `three.js/src/renderers/common/ClippingContext.js` and
//! `src/nodes/accessors/ClippingNode.js` — the planes a `ClippingGroup`
//! puts on its descendants, and the three shader fragments that apply them.

use std::rc::Rc;

use crate::math::{Matrix3, Matrix4, Plane};
use crate::objects::ClippingGroup;

/// `ClippingContext` — the clipping planes in force for one render item,
/// already in view space.
///
/// three keeps one context per `ClippingGroup` per render context and
/// re-projects its planes in place every frame. The port builds the contexts
/// afresh on each scene walk ([`ClippingContext::group_context`]) and hands
/// one to every item below the group; a walk with no `ClippingGroup` (or none
/// with planes) hands out none.
///
/// Each plane is stored the way `ClippingContext.projectPlanes()` stores it:
/// the view-space normal negated in `xyz` and the constant in `w`, so that a
/// point is clipped when `dot( positionView, plane.xyz ) > plane.w`.
///
/// `Hash` covers only the two counts and `hardware`: those change the
/// generated WGSL (`RenderObject.getCacheKey()` reads the context's
/// `cacheKey`, `id:intersection:union`), while the plane values are a
/// render-group uniform written per draw.
#[derive(Clone, Debug, Default, PartialEq)]
#[doc(hidden)]
pub struct ClippingContext {
    /// `intersectionPlanes` — from the groups with `clipIntersection`: a
    /// fragment is clipped only when it is behind all of them.
    pub intersection: Vec<[f32; 4]>,
    /// `unionPlanes` — from every other group: a fragment behind any one of
    /// them is clipped.
    pub union: Vec<[f32; 4]>,
    /// `builder.isAvailable( 'clipDistance' )` — the device has the WebGPU
    /// `clip-distances` feature, so up to eight union planes become
    /// `@builtin( clip_distances )` in the vertex stage
    /// (`NodeMaterial.setupHardwareClipping()`) instead of a fragment
    /// discard. Set by the renderer, not by the walk.
    pub hardware: bool,
}

impl std::hash::Hash for ClippingContext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.intersection.len().hash(state);
        self.union.len().hash(state);
        self.hardware.hash(state);
    }
}

/// `ClippingContext.updateGlobal( scene, camera )`: what every context of one
/// walk shares — the camera's view matrix, and whether the walk is a shadow
/// pass (`scene.overrideMaterial.isShadowPassMaterial`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClippingView {
    /// `viewMatrix` — `camera.matrixWorldInverse`.
    pub view_matrix: Matrix4,
    /// `viewNormalMatrix.getNormalMatrix( viewMatrix )`.
    pub view_normal_matrix: Matrix3,
    /// `shadowPass`.
    pub shadow_pass: bool,
}

impl ClippingView {
    pub(crate) fn new(view_matrix: Matrix4, shadow_pass: bool) -> Self {
        let mut view_normal_matrix = Matrix3::default();
        view_normal_matrix.get_normal_matrix(&view_matrix);
        Self {
            view_matrix,
            view_normal_matrix,
            shadow_pass,
        }
    }

    /// `ClippingContext.projectPlanes()` for one plane.
    fn project(&self, plane: &Plane) -> [f32; 4] {
        let mut plane = *plane;
        plane.apply_matrix4(&self.view_matrix, Some(&self.view_normal_matrix));
        [
            -plane.normal.x as f32,
            -plane.normal.y as f32,
            -plane.normal.z as f32,
            plane.constant as f32,
        ]
    }
}

impl ClippingContext {
    /// `parentContext.getGroupContext( clippingGroup )`: the parent's planes
    /// with the group's appended — to the intersection planes when the group
    /// has `clipIntersection`, to the union planes otherwise. In a shadow
    /// pass a group without `clipShadows` leaves its descendants on the
    /// parent's context.
    ///
    /// `None` stands for a context with no planes at all (three's root
    /// context before any group), which clips nothing and changes no program.
    pub(crate) fn group_context(
        parent: &Option<Rc<Self>>,
        group: &ClippingGroup,
        view: &ClippingView,
    ) -> Option<Rc<Self>> {
        if view.shadow_pass && !group.clip_shadows {
            return parent.clone();
        }
        let mut context = parent.as_deref().cloned().unwrap_or_default();
        let planes = group.clipping_planes.iter().map(|p| view.project(p));
        if group.clip_intersection {
            context.intersection.extend(planes);
        } else {
            context.union.extend(planes);
        }
        if context.intersection.is_empty() && context.union.is_empty() {
            return None;
        }
        Some(Rc::new(context))
    }

    /// This context on a device with (or without) the `clip-distances`
    /// feature.
    pub(crate) fn with_hardware(self: &Rc<Self>, hardware: bool) -> Rc<Self> {
        if self.hardware == hardware {
            return self.clone();
        }
        Rc::new(Self {
            hardware,
            ..(**self).clone()
        })
    }

    /// `NodeMaterial.setupHardwareClipping()`'s test: one to eight union
    /// planes on a device that can clip in hardware.
    pub fn hardware_clipping(&self) -> bool {
        self.hardware && (1..=8).contains(&self.union.len())
    }
}
