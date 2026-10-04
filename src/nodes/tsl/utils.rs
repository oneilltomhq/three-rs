//! The utils family of TSL (sweep 4): the context helpers from
//! `ContextNode.js` (`uniformFlow`, `setName`, `label`), `BypassNode`,
//! `VaryingNode`'s `vertexStage`, the leftover `Packing.js` functions,
//! `ExpressionNode`, `DebugNode`, `SampleNode`, `CodeNode`'s raw `wgsl()` and
//! the `EventNode` hooks. Each item's doc comment names the JS it ports; the
//! WGSL the shader-emitting ones produce is pinned against three's own dump in
//! `tests/nodes_tsl_batch.rs`.

use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{block, context, custom, dot, float, pack_normal_to_rgb, sqrt, to_varying, uv};
use crate::nodes::code::CodeDef;
use crate::nodes::node::{ContextValue, CustomNode, DebugCallback, Node, NodeRef, Type};
use crate::nodes::{NodeBuilder, NodeUpdateType};
use crate::renderer::Renderer;

// ---------------------------------------------------------------------------
// context helpers (`ContextNode.js`)
// ---------------------------------------------------------------------------

/// `uniformFlow( node )` — `context( node, { uniformFlow: true } )`. Inside
/// it a two-branch [`select`](NodeRef::select) is built as WGSL's `select( else,
/// if, cond )`, both branches evaluated, instead of an `if`/`else` writing a
/// var — what a derivative or texture sample needs to stay in uniform control
/// flow. Three still declares the var the `if` form would have used and
/// leaves it unassigned; so does the port, which keeps the numbering of every
/// later var the same as three's.
pub fn uniform_flow(node: impl Into<NodeRef>) -> NodeRef {
    context(node, ContextValue::default().uniform_flow(true))
}

/// `setName( node, name )` — `context( node, { nodeName: name } )`. The first
/// `uniform()` built inside without a name of its own takes `name`
/// (`UniformNode.getUniformName()`), and the build consumes the key, as
/// three's `delete builder.context.nodeName` does: any later uniform in the
/// same context is numbered as usual.
pub fn set_name(node: impl Into<NodeRef>, name: &'static str) -> NodeRef {
    context(node, ContextValue::default().node_name(name))
}

/// `label( node, name )` — deprecated in three (r179) for [`set_name`], which
/// it calls.
#[deprecated(note = "three.js r179 renamed `label()` to `setName()`; use `set_name`")]
pub fn label(node: impl Into<NodeRef>, name: &'static str) -> NodeRef {
    set_name(node, name)
}

// ---------------------------------------------------------------------------
// BypassNode, vertexStage
// ---------------------------------------------------------------------------

/// `bypass( outputNode, callNode )` — `BypassNode.generate()`: `callNode` is
/// built as a statement (`build( builder, 'void' )`, its line added to the
/// flow when it has one), and the value is `outputNode`'s. A second read in
/// the same scope is the cached snippet, not a second run of the call.
pub fn bypass(output: impl Into<NodeRef>, call: impl Into<NodeRef>) -> NodeRef {
    block(vec![call.into()], output.into())
}

/// `vertexStage( node )` — `varying( node )`: `node` computed in the vertex
/// stage and read through an unnamed varying.
pub fn vertex_stage(node: impl Into<NodeRef>) -> NodeRef {
    to_varying(None, node.into())
}

// ---------------------------------------------------------------------------
// Packing.js
// ---------------------------------------------------------------------------

/// `unpackNormal( xy )` — the `z` of a unit normal whose `xy` a two-channel
/// normal map stores: `vec3( xy, sqrt( saturate( 1 - dot( xy, xy ) ) ) )`.
pub fn unpack_normal(xy: impl Into<NodeRef>) -> NodeRef {
    let xy = xy.into();
    let z = sqrt(float(1.0).sub(dot(xy.clone(), xy.clone())).saturate());
    super::vec3_join(vec![xy, z])
}

/// `directionToColor( node )` — deprecated in three (r185) for
/// [`pack_normal_to_rgb`], which it calls.
#[deprecated(note = "three.js r185 renamed `directionToColor()` to `packNormalToRGB()`")]
pub fn direction_to_color(node: impl Into<NodeRef>) -> NodeRef {
    pack_normal_to_rgb(node.into())
}

/// `colorToDirection( node )` — deprecated in three (r185) for
/// `unpackRGBToNormal()`: `node * 2 - 1`.
#[deprecated(note = "three.js r185 renamed `colorToDirection()` to `unpackRGBToNormal()`")]
pub fn color_to_direction(node: impl Into<NodeRef>) -> NodeRef {
    node.into().mul(2.0).sub(1.0)
}

// ---------------------------------------------------------------------------
// ExpressionNode, DebugNode
// ---------------------------------------------------------------------------

/// `expression( snippet, type )` — `ExpressionNode`: a value of type `ty` is
/// `snippet` itself, verbatim; a [`Type::Void`] one is a statement, the
/// snippet as a flow line ending in `;` (nothing for an empty snippet). Never
/// cached: every read is the snippet again.
pub fn expression(snippet: &str, ty: Type) -> NodeRef {
    NodeRef::new(Node::Expression {
        snippet: snippet.into(),
        ty,
    })
}

/// `debug( node, callback )` — `DebugNode`: `node`'s value, unchanged. When it
/// is generated its snippet is handed to `callback` along with the stage and
/// the flow emitted so far ([`DebugInfo`](crate::nodes::DebugInfo)); with no callback the same thing
/// is written to stderr in three's `// #--- TSL debug - … shader ---#`
/// format, where three's goes to `console.log`.
pub fn debug(node: impl Into<NodeRef>, callback: Option<DebugCallback>) -> NodeRef {
    NodeRef::new(Node::Debug {
        node: node.into(),
        callback,
    })
}

// ---------------------------------------------------------------------------
// SampleNode
// ---------------------------------------------------------------------------

type SampleCallback = Rc<dyn Fn(NodeRef) -> NodeRef>;

/// `SampleNode` — a node that is a function of a uv: in a graph it is the
/// callback at `uv()`, and [`sample`](Self::sample) calls it at any other.
/// What a `NodeMaterial` hands a post-processing pass as "the thing to
/// sample".
///
/// Three's `sample( callback, uv )` also takes a uv, stores it as `uvNode`
/// and never reads it; the port leaves it out.
#[derive(Clone)]
pub struct SampleNode {
    callback: SampleCallback,
    node: NodeRef,
}

impl std::fmt::Debug for SampleNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SampleNode").finish_non_exhaustive()
    }
}

struct SampleSetup {
    callback: SampleCallback,
    ty: Type,
}

impl CustomNode for SampleSetup {
    fn type_name(&self) -> &'static str {
        "SampleNode"
    }
    fn node_type(&self) -> Type {
        self.ty
    }
    fn is_cacheable(&self) -> bool {
        false
    }
    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        (self.callback)(uv())
    }
}

impl SampleNode {
    /// `sampleNode.sample( uv )`: the callback at `uv`.
    pub fn sample(&self, uv: impl Into<NodeRef>) -> NodeRef {
        (self.callback)(uv.into())
    }

    /// The node itself: `setup()`'s `this.sample( uv() )`.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }
}

impl From<&SampleNode> for NodeRef {
    fn from(node: &SampleNode) -> Self {
        node.node()
    }
}

impl From<SampleNode> for NodeRef {
    fn from(node: SampleNode) -> Self {
        node.node
    }
}

/// `sample( callback )` — see [`SampleNode`].
pub fn sample(callback: impl Fn(NodeRef) -> NodeRef + 'static) -> SampleNode {
    let callback: SampleCallback = Rc::new(callback);
    // The port types nodes eagerly, so the callback runs once here to learn
    // the type of what it returns; `setup()` runs it again at each build.
    let ty = callback(uv()).ty();
    let node = custom(SampleSetup {
        callback: callback.clone(),
        ty,
    });
    SampleNode { callback, node }
}

// ---------------------------------------------------------------------------
// CodeNode
// ---------------------------------------------------------------------------

/// `wgsl( code, includes )` — a raw `CodeNode`: `code` lands in `// codes`
/// verbatim, after whatever `includes` declare, and the node itself is
/// nothing. It is what a [`wgsl_fn`](super::wgsl_fn)'s `includes` list takes
/// to put a helper the WGSL calls, or any other module-scope declaration, in
/// front of it. Unlike `wgslFn`, nothing is parsed: the snippet may hold any
/// number of declarations, and is emitted once per `wgsl()` node however
/// often it is reached.
pub fn wgsl(code: &str, includes: Vec<NodeRef>) -> NodeRef {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    // `getCodeFromNode( this )` keys the code by node; the port's `// codes`
    // are keyed by name, so each raw snippet gets one no declaration can have.
    let name = format!("#wgsl{}", NEXT.fetch_add(1, Ordering::Relaxed));
    super::code(&Rc::new(CodeDef {
        name,
        code: code.to_string(),
        params: Vec::new(),
        ret: Type::Void,
        includes,
    }))
}

// ---------------------------------------------------------------------------
// EventNode
// ---------------------------------------------------------------------------

/// When an [`EventNode`]'s callback runs: `EventNode.OBJECT` and the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventType {
    Object,
    Material,
    Frame,
    AfterObject,
    BeforeObject,
    BeforeMaterial,
    BeforeFrame,
}

/// `EventNode` — a `void` node that emits nothing and runs a callback in one
/// of the renderer's update phases, for as long as a material that reaches
/// it is drawn. Built by the `on_*` functions below; attached to a graph with
/// [`NodeRef::bypass`], as three's `.toStack()` would inside a `Fn()`.
///
/// The callback gets the [`Renderer`], what every update hook in the port
/// gets, where three's gets the `NodeFrame` with the object and material
/// being drawn.
struct EventNode {
    event: EventType,
    callback: Box<dyn Fn(&mut Renderer)>,
}

impl EventNode {
    fn phase(&self, before: bool, after: bool) -> NodeUpdateType {
        use EventType::*;
        match (self.event, before, after) {
            (Object, false, false) | (AfterObject, false, true) | (BeforeObject, true, false) => {
                NodeUpdateType::Object
            }
            (Material, false, false) | (BeforeMaterial, true, false) => NodeUpdateType::Render,
            (Frame, false, false) | (BeforeFrame, true, false) => NodeUpdateType::Frame,
            _ => NodeUpdateType::None,
        }
    }
}

impl CustomNode for EventNode {
    fn type_name(&self) -> &'static str {
        "EventNode"
    }
    fn node_type(&self) -> Type {
        Type::Void
    }
    fn is_cacheable(&self) -> bool {
        false
    }
    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        // `Node.setup()` with no children: nothing to build.
        expression("", Type::Void)
    }
    fn update_before_type(&self) -> NodeUpdateType {
        self.phase(true, false)
    }
    fn update_type(&self) -> NodeUpdateType {
        self.phase(false, false)
    }
    fn update_after_type(&self) -> NodeUpdateType {
        self.phase(false, true)
    }
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        (self.callback)(renderer);
        true
    }
    fn update(&self, renderer: &mut Renderer) -> bool {
        (self.callback)(renderer);
        true
    }
    fn update_after(&self, renderer: &mut Renderer) -> bool {
        (self.callback)(renderer);
        true
    }
}

fn event(event: EventType, callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    custom(EventNode {
        event,
        callback: Box::new(callback),
    })
}

/// `OnObjectUpdate( callback )` — `updateType = OBJECT`: once per draw of an
/// object whose material reaches the node.
pub fn on_object_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::Object, callback)
}

/// `OnMaterialUpdate( callback )` — `updateType = RENDER`: once per
/// `render()` call that draws the material.
pub fn on_material_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::Material, callback)
}

/// `OnFrameUpdate( callback )` — `updateType = FRAME`: once per frame.
pub fn on_frame_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::Frame, callback)
}

/// `OnAfterObjectUpdate( callback )` — `updateAfterType = OBJECT`: after each
/// draw.
pub fn on_after_object_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::AfterObject, callback)
}

/// `OnBeforeObjectUpdate( callback )` — `updateBeforeType = OBJECT`: before
/// each draw.
pub fn on_before_object_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::BeforeObject, callback)
}

/// `OnBeforeMaterialUpdate( callback )` — `updateBeforeType = RENDER`.
pub fn on_before_material_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::BeforeMaterial, callback)
}

/// `OnBeforeFrameUpdate( callback )` — `updateBeforeType = FRAME`.
pub fn on_before_frame_update(callback: impl Fn(&mut Renderer) + 'static) -> NodeRef {
    event(EventType::BeforeFrame, callback)
}

// ---------------------------------------------------------------------------
// method chaining
// ---------------------------------------------------------------------------

impl NodeRef {
    /// `node.bypass( call )` — see [`bypass`].
    pub fn bypass(&self, call: impl Into<NodeRef>) -> NodeRef {
        bypass(self, call)
    }

    /// `node.uniformFlow()` — see [`uniform_flow`].
    pub fn uniform_flow(&self) -> NodeRef {
        uniform_flow(self)
    }

    /// `node.setName( name )` — see [`set_name`].
    pub fn set_name(&self, name: &'static str) -> NodeRef {
        set_name(self, name)
    }

    /// `node.label( name )` — deprecated in three for
    /// [`set_name`](Self::set_name).
    #[deprecated(note = "three.js r179 renamed `label()` to `setName()`; use `set_name`")]
    pub fn label(&self, name: &'static str) -> NodeRef {
        set_name(self, name)
    }

    /// `node.toVertexStage()` — see [`vertex_stage`].
    pub fn to_vertex_stage(&self) -> NodeRef {
        vertex_stage(self)
    }

    /// `node.debug( callback )` — see [`debug`].
    pub fn debug(&self, callback: Option<DebugCallback>) -> NodeRef {
        debug(self, callback)
    }
}
