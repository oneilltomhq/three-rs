//! Port of `three.js/src/renderers/common/ClippingContext.js` and
//! `src/nodes/accessors/ClippingNode.js` — the planes a `ClippingGroup`
//! puts on its descendants, and the three shader fragments that apply them.

use std::rc::Rc;

use crate::math::{Matrix3, Matrix4, Plane};
use crate::nodes::node::{BufferId, BufferNode, BufferSource, Node, NodeRef, Type};
use crate::nodes::tsl::{
    boolean, diffuse_color, discard, float, fwidth, if_then, int, loop_n, position_view,
    smoothstep, to_var,
};
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
    /// discard. Taken from the walk's [`ClippingView`].
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
/// walk shares — the camera's view matrix, whether the walk is a shadow pass
/// (`scene.overrideMaterial.isShadowPassMaterial`), and whether the device
/// clips in hardware.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClippingView {
    /// `viewMatrix` — `camera.matrixWorldInverse`.
    pub view_matrix: Matrix4,
    /// `viewNormalMatrix.getNormalMatrix( viewMatrix )`.
    pub view_normal_matrix: Matrix3,
    /// `shadowPass`.
    pub shadow_pass: bool,
    /// `builder.isAvailable( 'clipDistance' )`, copied into every context
    /// the walk builds ([`ClippingContext::hardware`]).
    pub hardware: bool,
}

impl ClippingView {
    pub(crate) fn new(view_matrix: Matrix4, shadow_pass: bool, hardware: bool) -> Self {
        let mut view_normal_matrix = Matrix3::default();
        view_normal_matrix.get_normal_matrix(&view_matrix);
        Self {
            view_matrix,
            view_normal_matrix,
            shadow_pass,
            hardware,
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
        context.hardware = view.hardware;
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

    /// `NodeMaterial.setupHardwareClipping()`'s test: one to eight union
    /// planes on a device that can clip in hardware.
    pub fn hardware_clipping(&self) -> bool {
        self.hardware && (1..=8).contains(&self.union.len())
    }
}

/// `uniformArray( planes ).setGroup( renderGroup )`: one `vec4` per plane,
/// read by the draw from its context (`source` says which list). `element`
/// is `clippingPlanes.element( i )`, sharing the one buffer node.
fn plane_buffer(source: BufferSource, count: usize) -> impl Fn(&NodeRef) -> NodeRef {
    let buffer = Rc::new(BufferNode {
        id: BufferId::next(),
        source,
        element_ty: Type::Vec4,
        count,
    });
    move |index| {
        NodeRef::new(Node::BufferElement {
            buffer: buffer.clone(),
            index: index.clone(),
        })
    }
}

/// `clipping()` — `ClippingNode.setupDefault()`: a fragment behind any union
/// plane is discarded, and one behind every intersection plane is too. The
/// union planes are left out when the vertex stage clips them in hardware
/// ([`hardware_clipping`]).
///
/// `NodeMaterial.setupClipping()` pushes it first in the fragment flow — ahead
/// of `setupDepth()`, the diffuse colour and the mask discard — whenever the
/// material does not take [`clipping_alpha`] instead.
pub fn clipping(context: &ClippingContext) -> Vec<NodeRef> {
    let mut statements = Vec::new();
    let union = context.union.len();
    if !context.hardware_clipping() && union > 0 {
        let planes = plane_buffer(BufferSource::ClippingUnion, union);
        statements.push(loop_n("i", int(union as i64), |i| {
            let plane = planes(i);
            vec![if_then(
                position_view().dot(plane.xyz()).greater_than(plane.w()),
                vec![discard()],
            )]
        }));
    }
    let intersection = context.intersection.len();
    if intersection > 0 {
        let planes = plane_buffer(BufferSource::ClippingIntersection, intersection);
        let clipped = to_var(Some("clipped"), boolean(true));
        statements.push(clipped.clone());
        statements.push(loop_n("i", int(intersection as i64), |i| {
            let plane = planes(i);
            vec![clipped.assign(
                position_view()
                    .dot(plane.xyz())
                    .greater_than(plane.w())
                    .and(clipped.clone()),
            )]
        }));
        statements.push(if_then(clipped, vec![discard()]));
    }
    statements
}

/// `clippingAlpha()` — `ClippingNode.setupAlphaToCoverage()`: each plane
/// fades the diffuse alpha out over one `fwidth` of the distance to it, so
/// the edge is antialiased by alpha-to-coverage, and a fully clipped fragment
/// is discarded.
///
/// `NodeMaterial.setupClipping()` returns it instead of pushing
/// [`clipping`] when the material has `alphaToCoverage` and the target is
/// multisampled, and `setup()` adds it to the flow after `setupLighting()` —
/// once the diffuse alpha is final. A material with a `fragmentNode` drops it.
pub fn clipping_alpha(context: &ClippingContext) -> Vec<NodeRef> {
    let distance = to_var(Some("distanceToPlane"), float(0.0));
    let gradient = to_var(Some("distanceToGradient"), float(0.0));
    let clip_opacity = to_var(Some("clipOpacity"), float(1.0));
    let mut statements = vec![distance.clone(), gradient.clone(), clip_opacity.clone()];
    // `distanceToPlane` and `distanceToGradient` for one plane.
    let fade = |plane: &NodeRef| {
        vec![
            distance.assign(position_view().dot(plane.xyz()).negate().add(plane.w())),
            gradient.assign(fwidth(distance.clone()).div(2.0)),
        ]
    };
    let step = || smoothstep(gradient.negate(), gradient.clone(), distance.clone());

    let union = context.union.len();
    if !context.hardware_clipping() && union > 0 {
        let planes = plane_buffer(BufferSource::ClippingUnion, union);
        statements.push(loop_n("i", int(union as i64), |i| {
            let mut body = fade(&planes(i));
            body.push(clip_opacity.mul_assign(step()));
            body
        }));
    }
    let intersection = context.intersection.len();
    if intersection > 0 {
        let planes = plane_buffer(BufferSource::ClippingIntersection, intersection);
        let intersection_opacity = to_var(Some("intersectionClipOpacity"), float(1.0));
        statements.push(intersection_opacity.clone());
        statements.push(loop_n("i", int(intersection as i64), |i| {
            let mut body = fade(&planes(i));
            body.push(intersection_opacity.mul_assign(step().one_minus()));
            body
        }));
        statements.push(clip_opacity.mul_assign(intersection_opacity.one_minus()));
    }
    statements.push(diffuse_color().w().mul_assign(clip_opacity));
    statements.push(if_then(diffuse_color().w().equal(0.0), vec![discard()]));
    statements
}

/// `hardwareClipping()` — `ClippingNode.setupHardwareClipping()`: the vertex
/// stage writes each union plane's signed distance to `@builtin(
/// clip_distances )`, and the rasteriser clips. Only for a context whose
/// [`hardware_clipping`](ClippingContext::hardware_clipping) holds; the
/// count goes in [`MaterialFlow::clip_distances`](crate::nodes::builder::MaterialFlow::clip_distances),
/// which declares the builtin.
pub fn hardware_clipping(context: &ClippingContext) -> NodeRef {
    let union = context.union.len();
    let planes = plane_buffer(BufferSource::ClippingUnion, union);
    loop_n("i", int(union as i64), |i| {
        let plane = planes(i);
        // `builtin( builder.getClipDistance() ).element( i )`.
        let target = NodeRef::new(Node::Expression {
            snippet: "varyings.hw_clip_distances[ i ]".into(),
            ty: Type::F32,
        });
        vec![target.assign(position_view().dot(plane.xyz()).sub(plane.w()).negate())]
    })
}

/// The clipping a material's setup adds, by `NodeMaterial.setup()`'s rules:
/// the fragment statements that go first (`clipping()`), the ones that go
/// after the lighting setup (`clippingAlpha()`), and the vertex loop with its
/// clip-distance count (`hardwareClipping()`).
#[derive(Default)]
pub(crate) struct MaterialClipping {
    /// [`clipping`]'s statements, or none.
    pub first: Vec<NodeRef>,
    /// [`clipping_alpha`]'s statements, or none.
    pub alpha: Vec<NodeRef>,
    /// [`hardware_clipping`]'s loop and the number of planes it writes.
    pub hardware: Option<(NodeRef, usize)>,
}

impl MaterialClipping {
    /// `setupHardwareClipping()` and `setupClipping()` for a material with
    /// `alphaToCoverage` on a multisampled target (`alpha_to_coverage`) or
    /// without.
    pub(crate) fn new(context: Option<&ClippingContext>, alpha_to_coverage: bool) -> Self {
        let Some(context) = context else {
            return Self::default();
        };
        let hardware = context
            .hardware_clipping()
            .then(|| (hardware_clipping(context), context.union.len()));
        let any = !context.union.is_empty() || !context.intersection.is_empty();
        let (first, alpha) = match (any, alpha_to_coverage) {
            (false, _) => (Vec::new(), Vec::new()),
            (true, false) => (clipping(context), Vec::new()),
            (true, true) => (Vec::new(), clipping_alpha(context)),
        };
        Self {
            first,
            alpha,
            hardware,
        }
    }
}
