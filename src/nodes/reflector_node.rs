//! Port of `three.js/src/nodes/utils/ReflectorNode.js`: `reflector()`, a
//! planar mirror.
//!
//! Three splits it in two. `ReflectorNode` is a `TextureNode` whose uv is
//! `screenUV.flipX()`; `ReflectorBaseNode` holds the virtual cameras and render
//! targets and, as an `updateBefore` node of type `RENDER`, renders the scene
//! from the mirrored camera into a target before the object that carries it is
//! drawn, then points the texture node's `value` at that target.
//!
//! The port keeps the split. [`ReflectorNode`] is what the page holds: its
//! [`uv_node`](ReflectorNode::uv_node) is public to be reassigned exactly as
//! the page does (`groundReflector.uvNode = groundReflector.uvNode.add( … )`),
//! and [`ReflectorNode::node`] is the `TextureNode` a material's graph takes.
//! [`Reflector`] is `ReflectorBaseNode`; the renderer runs its
//! `updateBefore()` (`Renderer::update_reflectors`, `src/renderer/reflector.rs`),
//! because that is a nested `renderer.render()`.
//!
//! **Divergence** (`docs/nodes.md` §55): three samples one module-level
//! `_defaultRT.texture` from every reflector and rewrites `textureNode.value`
//! in place; a draw binds whatever `value` is when it is recorded. The port's
//! bindings are resolved from the program, so each reflector has its own
//! default texture, which the renderer finds in this module's registry and
//! replaces, per draw, with the `value` the reflector had when that draw's
//! object came up in the render list. The draw sees the texture three's
//! would.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::core::{Node, Object3D};
use crate::nodes::tsl::{screen_uv, texture_uv};
use crate::nodes::NodeRef;
use crate::renderer::RenderTarget;
use crate::textures::Texture;

/// `reflector( parameters )`'s options. `generateMipmaps` and `depth` are not
/// ported: no graded page sets them.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ReflectorParameters {
    /// `target` — `new Object3D()` when `None`.
    pub target: Option<Node>,
    /// `resolutionScale`.
    pub resolution_scale: f64,
    /// `bounces` — whether this reflector renders while another one is
    /// rendering.
    pub bounces: bool,
    /// `samples` — the internal render target's MSAA sample count.
    pub samples: u32,
}

impl Default for ReflectorParameters {
    fn default() -> Self {
        Self {
            target: None,
            resolution_scale: 1.0,
            bounces: true,
            samples: 0,
        }
    }
}

/// `ReflectorBaseNode`'s state.
pub struct ReflectorBase {
    /// `target` — the object whose world transform is the mirror plane: its
    /// position is a point on the plane and its +Z the normal.
    pub target: Node,
    /// `resolutionScale`.
    pub resolution_scale: f64,
    /// `bounces`.
    pub bounces: bool,
    /// `samples`.
    pub samples: u32,
    /// `forceUpdate`.
    pub force_update: bool,
    /// `hasOutput`.
    pub has_output: bool,
    /// `virtualCameras`, keyed on the id of the camera being mirrored.
    pub(crate) virtual_cameras: HashMap<u32, PerspectiveCamera>,
    /// `renderTargets`, keyed on the id of the *virtual* camera.
    pub(crate) render_targets: HashMap<u32, RenderTarget>,
    /// `_defaultRT` — what the texture node samples before this reflector has
    /// rendered anything. One per reflector here; see the module docs.
    pub(crate) default_render_target: RenderTarget,
    /// `textureNode.value`.
    pub(crate) value: Texture,
}

impl std::fmt::Debug for ReflectorBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflectorBase")
            .field("resolution_scale", &self.resolution_scale)
            .field("bounces", &self.bounces)
            .field("samples", &self.samples)
            .field("virtual_cameras", &self.virtual_cameras.len())
            .finish_non_exhaustive()
    }
}

/// `ReflectorBaseNode` — a shared handle, as a JS object is.
#[derive(Clone, Debug)]
pub struct Reflector(Rc<RefCell<ReflectorBase>>);

impl Reflector {
    /// Borrows the shared [`ReflectorBase`].
    pub fn borrow(&self) -> std::cell::Ref<'_, ReflectorBase> {
        self.0.borrow()
    }

    /// Mutably borrows the shared [`ReflectorBase`].
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, ReflectorBase> {
        self.0.borrow_mut()
    }

    /// Identity, for the renderer's once-per-render bookkeeping.
    pub(crate) fn key(&self) -> usize {
        Rc::as_ptr(&self.0) as *const u8 as usize
    }

    /// The id of the texture the texture node was built over — the binding
    /// the renderer swaps for [`value`](Self::value).
    pub(crate) fn default_texture_id(&self) -> usize {
        self.0.borrow().default_render_target.texture().id()
    }

    /// `textureNode.value`.
    pub fn value(&self) -> Texture {
        self.0.borrow().value.clone()
    }
}

thread_local! {
    /// Default texture id → reflector, so the renderer can tell a reflector's
    /// binding from any other texture's.
    ///
    /// The entries are strong. In three the material graph keeps the
    /// `ReflectorBaseNode` alive through the `ReflectorNode` it holds; here
    /// the graph holds only the texture node over the default texture, so the
    /// registry keeps the reflector until nothing but the reflector itself
    /// holds that texture any more ([`prune`]).
    static REGISTRY: RefCell<HashMap<usize, Rc<RefCell<ReflectorBase>>>> =
        RefCell::new(HashMap::new());
    /// `const _defaultUV = screenUV.flipX()` — one node for every reflector.
    static DEFAULT_UV: NodeRef = screen_uv().flip_x();
}

impl ReflectorBase {
    /// Whether anything outside this reflector — a material graph, a built
    /// program's bindings — still holds its default texture.
    fn is_sampled(&self) -> bool {
        let texture = self.default_render_target.texture();
        // This clone, the render target's own handle, and `value` when it is
        // still the default texture.
        let own = 2 + usize::from(self.value.id() == texture.id());
        texture.handle_count() > own
    }
}

/// Drops every reflector nothing samples any more.
fn prune(registry: &mut HashMap<usize, Rc<RefCell<ReflectorBase>>>) {
    registry.retain(|_, base| base.try_borrow().map_or(true, |base| base.is_sampled()));
}

/// The reflector whose default texture is `texture_id`, if there is one.
pub(crate) fn lookup(texture_id: usize) -> Option<Reflector> {
    REGISTRY.with(|registry| registry.borrow().get(&texture_id).cloned().map(Reflector))
}

/// Whether any reflector is still sampled — the renderer skips the per-item
/// lookup entirely on every page that has none.
pub(crate) fn any() -> bool {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        prune(&mut registry);
        !registry.is_empty()
    })
}

/// `ReflectorNode` — the `TextureNode` half.
#[derive(Clone, Debug)]
pub struct ReflectorNode {
    /// `uvNode` — `screenUV.flipX()` until the page reassigns it.
    pub uv_node: NodeRef,
    reflector: Reflector,
}

impl ReflectorNode {
    /// `reflectorNode.reflector`.
    pub fn reflector(&self) -> &Reflector {
        &self.reflector
    }

    /// `reflectorNode.target` — add it to the mirror's mesh
    /// (`plane.add( groundReflector.target )`).
    pub fn target(&self) -> Node {
        self.reflector.0.borrow().target.clone()
    }

    /// The node itself, as a material graph uses it: a sample of the
    /// reflection at [`uv_node`](Self::uv_node). `setUpdateMatrix( false )`,
    /// so no uv transform.
    pub fn node(&self) -> NodeRef {
        let texture = self.reflector.0.borrow().default_render_target.texture();
        texture_uv(&texture, self.uv_node.clone())
    }
}

impl From<&ReflectorNode> for NodeRef {
    fn from(node: &ReflectorNode) -> Self {
        node.node()
    }
}

/// `reflector( parameters )`.
pub fn reflector(parameters: ReflectorParameters) -> ReflectorNode {
    // `const _defaultRT = new RenderTarget()`; `setup()` resizes it to the
    // drawing buffer before it can be sampled.
    let default_render_target = RenderTarget::new(1, 1);
    let value = default_render_target.texture();
    let texture_id = value.id();

    let base = Rc::new(RefCell::new(ReflectorBase {
        target: parameters
            .target
            .unwrap_or_else(|| Object3D::default().into_node()),
        resolution_scale: parameters.resolution_scale,
        bounces: parameters.bounces,
        samples: parameters.samples,
        force_update: false,
        has_output: false,
        virtual_cameras: HashMap::new(),
        render_targets: HashMap::new(),
        default_render_target,
        value,
    }));

    REGISTRY.with(|registry| {
        registry.borrow_mut().insert(texture_id, base.clone());
    });

    ReflectorNode {
        uv_node: DEFAULT_UV.with(|uv| uv.clone()),
        reflector: Reflector(base),
    }
}
