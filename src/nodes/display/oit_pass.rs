//! Port of `three.js/examples/jsm/tsl/display/OITPassNode.js` — a scene pass
//! with Weighted Blended Order-Independent Transparency (McGuire and Bavoil,
//! <https://jcgt.org/published/0002/02/09/>).
//!
//! The pass renders the scene twice into two targets that share one depth
//! attachment:
//!
//! 1. the *default* pass, into the wrapped [`PassNode`]'s own target: every
//!    opaque object, and every transparent one that does not qualify for OIT;
//! 2. the *OIT* pass, into a second, two-attachment target: only the
//!    qualifying transparent objects, depth-tested against the opaque depth
//!    but never writing it, through an MRT whose two outputs blend additively
//!    — `accum` (`rgba16float`, `One`/`One`) gathers the weighted,
//!    premultiplied colour sum and `revealage` (`r8unorm`, `Zero` /
//!    `OneMinusSrcColor`) the product of `1 − alpha`.
//!
//! The node's value is the composite of the two:
//! `vec4( mix( accum.rgb / max( accum.a, 1e-5 ), beauty.rgb, revealage ),
//! beauty.a )`. Neither blend equation depends on the order the fragments
//! arrive in, so neither does the picture.
//!
//! Three subclasses `PassNode` and swaps the renderer's render-object function
//! for each of the two renders. The port wraps a [`PassNode`], renders through
//! its `render_with` so the default pass is
//! exactly `PassNode.updateBefore()`'s, and sets the renderer's `oit` hook
//! around each render instead of a function: the render loop reads it per
//! render item exactly where three calls the function. `docs/nodes.md` §82.
//!
//! What a material has to be to qualify (`isOITCapable()`): `transparent`,
//! `NormalBlending`, no `transmission`, and no `backdropNode`.
//!
//! # Not ported
//!
//! - The WebGL backend's `renderTarget.samples = 0` branch of `setup()`: the
//!   port has only the WebGPU backend, whose branch (the OIT target takes the
//!   pass target's sample count, because the depth is shared) is ported.
//! - `isOITCapable()`'s `material.transmissionNode` test: the port's
//!   materials have no `transmissionNode`, so there is nothing to test.
//! - `PassNode`'s `autoClear` / `autoClearColor` / `autoClearStencil`
//!   copies: the port's `PassNode` carries `autoClearDepth` only, and the
//!   renderer's own colour and stencil flags are left as they are, which is
//!   what three's defaults (all `true`) copy onto them anyway.
//! - `setMRT()` on the OIT pass node (three applies it to the default pass
//!   only): the wrapped pass has no public MRT setter on this path, and the
//!   page does not use one.
//! - `dispose()`: the targets are freed when the last handle drops.
//! - The page's Inspector / GUI. The example keeps the two parameters the GUI
//!   edits (`oit` and `opacity`) as fields.

use std::cell::RefCell;
use std::rc::Rc;

use crate::materials::{BlendFactor, BlendMode, Blending, MeshBasicNodeMaterial};
use crate::math::Color;
use crate::nodes::frame::register_texture_update;
use crate::nodes::tsl::{custom, float, mix, output_property, position_view, texture, vec4_join};
use crate::nodes::{
    mrt, CustomNode, MrtNode, NodeBuilder, NodeRef, NodeUpdate, NodeUpdateType, Type,
};
use crate::renderer::{CameraRef, PassNode, RenderTarget, Renderer, SceneRef};

/// Which half of the scene the renderer draws while an [`OitPassNode`]
/// renders — the two render-object functions three's `OITPassNode` installs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OitRenderObjects {
    /// `_defaultRenderObjectFunction`: everything that does **not** qualify
    /// for OIT.
    Default,
    /// `_oitRenderObjectFunction`: only what qualifies, with `depthWrite`
    /// forced off for the draw.
    Accumulate,
}

impl OitRenderObjects {
    /// Whether a render item with `material` is drawn under this function.
    pub(crate) fn draws(&self, material: &MeshBasicNodeMaterial) -> bool {
        match self {
            Self::Default => !is_oit_capable(material),
            Self::Accumulate => is_oit_capable(material),
        }
    }
}

/// `isOITCapable( material )`.
///
/// `transparent === true && blending === NormalBlending && ( transmission > 0
/// ) === false && ! transmissionNode && ! backdropNode`. The port's materials
/// have no `transmissionNode`, so that clause is always true here.
pub fn is_oit_capable(material: &MeshBasicNodeMaterial) -> bool {
    material.transparent
        && material.blending == Blending::Normal
        // `( transmission > 0 ) === false`, which lets a NaN through
        && material.transmission.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        && material.backdrop_node.is_none()
}

/// `oitPass( scene, camera )`.
///
/// A handle: clones share one pass. Like a [`PassNode`] it renders from
/// `updateBefore()` the first time in a frame a draw samples it — here, any
/// of the beauty, `accum` or `revealage` textures the composite binds.
#[derive(Clone)]
pub struct OitPassNode(Rc<OitState>);

/// What an [`OitPassNode`] handle shares.
struct OitState {
    /// The `PassNode` half: its target is `this.renderTarget`, the default
    /// pass's.
    pass: PassNode,
    /// `this._oitRenderTarget`.
    oit_render_target: RenderTarget,
    /// `this.weightNode`.
    weight_node: RefCell<Option<NodeRef>>,
    /// `this._oitMRTNode`, made on first use by [`OitState::mrt_node`].
    oit_mrt_node: RefCell<Option<MrtNode>>,
    /// `this.scene` / `this.camera`.
    scene: SceneRef,
    camera: CameraRef,
    /// `setup()`'s return value — the composite.
    node: NodeRef,
}

/// `oitPass( scene, camera )`.
pub fn oit_pass(scene: SceneRef, camera: CameraRef) -> OitPassNode {
    OitPassNode::new(scene, camera)
}

impl OitPassNode {
    /// `new OITPassNode( scene, camera )`.
    pub fn new(scene: SceneRef, camera: CameraRef) -> Self {
        // `super( PassNode.COLOR, scene, camera, options )`.
        let pass = PassNode::new();

        // "the accumulation target shares the depth of the default pass so
        // transparent fragments are depth-tested against the opaque scene
        // (without depth writes)"
        let oit_render_target = RenderTarget::new(1, 1);
        oit_render_target.set_count(2);
        oit_render_target.set_depth_texture(pass.depth_texture());

        let textures = oit_render_target.textures();
        // `accumTexture.name = 'accum'; accumTexture.type = HalfFloatType;`
        oit_render_target.set_texture_name(0, "accum");
        textures[0].set_format(wgpu::TextureFormat::Rgba16Float);
        // `revealageTexture.name = 'revealage'; .format = RedFormat; .type =
        // UnsignedByteType;`
        oit_render_target.set_texture_name(1, "revealage");
        textures[1].set_format(wgpu::TextureFormat::R8Unorm);

        // `setup()`'s TSL.
        let beauty_node = pass.node();
        let accum_node = texture(&textures[0]);
        let revealage_node = texture(&textures[1]).x();

        let accum_color = accum_node.rgb().div(accum_node.a().max(float(1e-5)));

        let composite = vec4_join(vec![
            mix(accum_color, beauty_node.rgb(), revealage_node),
            beauty_node.a(),
        ]);
        let node = custom(Composite(composite));

        let state = Rc::new(OitState {
            pass,
            oit_render_target,
            weight_node: RefCell::new(None),
            oit_mrt_node: RefCell::new(None),
            scene,
            camera,
            node,
        });

        // The composite binds the beauty texture and both accumulation
        // textures, and every one of them is filled by *this* node's
        // `updateBefore()` — which takes the beauty over from the wrapped
        // pass's own registration. The depth too, for a graph that reads it.
        register_texture_update(state.pass.texture().id(), &state);
        register_texture_update(state.pass.depth_texture().id(), &state);
        for texture in &textures {
            register_texture_update(texture.id(), &state);
        }

        Self(state)
    }

    /// `oitPassNode.weightNode = node` — the depth-based weight of a
    /// transparent fragment, equations (7) to (9) in the paper. `None` (the
    /// default) is equation (9). Must be set before the first render, as in
    /// three: the MRT is made once.
    pub fn set_weight_node(&self, weight_node: Option<NodeRef>) {
        *self.0.weight_node.borrow_mut() = weight_node;
    }

    /// The composite — the node for the graph downstream, usually the
    /// pipeline's `outputNode`.
    pub fn node(&self) -> NodeRef {
        self.0.node.clone()
    }

    /// The wrapped `PassNode`: the default pass, whose target holds the
    /// opaque scene and the shared depth.
    pub fn pass(&self) -> &PassNode {
        &self.0.pass
    }

    /// `_getMRTNode()` — the OIT pass's MRT, for inspecting its program
    /// (`examples/dump_wgsl.rs`, the dump gate).
    #[doc(hidden)]
    pub fn mrt_node(&self) -> MrtNode {
        self.0.mrt_node()
    }

    /// The OIT pass's MRT over its accumulation target, resolved the way the
    /// renderer resolves it for that render ([`MrtContext::for_target`]):
    /// `accum` then `revealage`, with their attachments' output types. For
    /// the dump gate.
    ///
    /// [`MrtContext::for_target`]: crate::materials::MrtContext::for_target
    #[doc(hidden)]
    pub fn mrt_context(&self) -> crate::materials::MrtContext {
        crate::materials::MrtContext::for_target(self.mrt_node(), &self.0.oit_render_target)
    }

    /// `renderTarget.textures` of the accumulation target — `accum`, then
    /// `revealage`. For tests that read the targets back.
    #[doc(hidden)]
    pub fn accumulation_textures(&self) -> Vec<crate::textures::Texture> {
        self.0.oit_render_target.textures()
    }
}

/// The `OITPassNode` itself as the graph sees it: a node whose `setup()`
/// returns the composite. It matters that it is a node of its own and not the
/// bare join: `renderOutput()` reads it as `.rgb` and `.a`, and three's
/// `PassNode.isCacheable()` is `false`, so the join is built at each read
/// (it is analysed once, through the pass node) rather than into a var.
struct Composite(NodeRef);

impl CustomNode for Composite {
    fn type_name(&self) -> &'static str {
        "OITPassNode"
    }

    fn node_type(&self) -> Type {
        Type::Vec4
    }

    /// `PassNode.isCacheable()`.
    fn is_cacheable(&self) -> bool {
        false
    }

    fn setup(&self, _builder: &NodeBuilder) -> NodeRef {
        self.0.clone()
    }
}

impl OitState {
    /// `OITPassNode._getMRTNode()`.
    fn mrt_node(&self) -> MrtNode {
        if let Some(node) = self.oit_mrt_node.borrow().as_ref() {
            return node.clone();
        }

        let alpha = output_property().a();

        let weight = match self.weight_node.borrow().clone() {
            Some(weight) => weight,
            None => {
                // "equation (9) from the paper, based on the linear eye-space
                // depth"
                let z = position_view().z().negate();
                alpha.mul(
                    float(0.03)
                        .div(z.div(float(200.0)).pow(float(4.0)).add(float(1e-5)))
                        .clamp(float(1e-2), float(3e3)),
                )
            }
        };

        // "since the revealage target is single-channel, the alpha must be
        // blended via its red channel"
        let accum_blending = BlendMode {
            blend_src: BlendFactor::One,
            blend_dst: BlendFactor::One,
            ..BlendMode::new(Blending::Custom)
        };
        let revealage_blending = BlendMode {
            blend_src: BlendFactor::Zero,
            blend_dst: BlendFactor::OneMinusSrcColor,
            ..BlendMode::new(Blending::Custom)
        };

        let mut node = mrt(vec![
            (
                "accum",
                vec4_join(vec![
                    output_property().rgb().mul(alpha.clone()),
                    alpha.clone(),
                ])
                .mul(weight),
            ),
            ("revealage", alpha),
        ]);
        node.set_blend_mode("accum", accum_blending)
            .set_blend_mode("revealage", revealage_blending)
            .set_clear_color("accum", Color::from_hex(0x000000), 0.0)
            .set_clear_color("revealage", Color::from_hex(0xffffff), 1.0);

        *self.oit_mrt_node.borrow_mut() = Some(node.clone());
        node
    }
}

impl NodeUpdate for OitState {
    /// `this.updateBeforeType = NodeUpdateType.FRAME`, from `PassNode`.
    fn update_before_type(&self) -> NodeUpdateType {
        NodeUpdateType::Frame
    }

    /// `OITPassNode.updateBefore( frame )`.
    ///
    /// The size, near/far, layers, target, MRT and auto-clear save/restore
    /// around both renders is [`PassNode`]'s own (`render_with`), which also
    /// restores everything the OIT half changes.
    fn update_before(&self, renderer: &mut Renderer) -> bool {
        let (near, far) = {
            let camera = self.camera.borrow();
            (camera.near(), camera.far())
        };
        let pass_target = self.pass.render_target().clone();
        self.pass.render_with(renderer, near, far, |renderer| {
            // `setSize()`: the OIT target follows the pass target, keeping the
            // depth texture the pass target owns.
            let (width, height) = pass_target.size();
            self.oit_render_target
                .set_size_keeping_depth(width, height, true);
            // `setup()`'s "sample counts must match since the depth buffer
            // is shared".
            self.oit_render_target.set_samples(pass_target.samples());

            // "default pass: opaque objects and transparent objects that do
            // not qualify for OIT"
            let previous = renderer.oit.replace(OitRenderObjects::Default);
            renderer.render_shared(&self.scene.borrow(), &self.camera);

            // "OIT pass: accumulate the weighted colors and the revealage of
            // all OIT-qualified objects"
            //
            // `RendererUtils.resetSceneState()` — "the background must not
            // affect the accumulation targets". The port's scene has no
            // `backgroundNode`.
            let (background, override_material) = {
                let mut scene = self.scene.borrow_mut();
                (scene.background.take(), scene.override_material.take())
            };

            renderer.set_render_target(Some(self.oit_render_target.clone()));
            renderer.set_mrt(Some(self.mrt_node()));
            renderer.oit = Some(OitRenderObjects::Accumulate);
            // "the depth buffer is shared with the default pass"
            renderer.auto_clear_depth = false;
            renderer.opaque = false;
            renderer.transparent = true;

            renderer.render_shared(&self.scene.borrow(), &self.camera);

            // `RendererUtils.restoreSceneState()`; the renderer's target, MRT
            // and flags are restored by `render_with`.
            {
                let mut scene = self.scene.borrow_mut();
                scene.background = background;
                scene.override_material = override_material;
            }
            renderer.oit = previous;
        });
        true
    }

    /// `PassNode.setup()`'s `renderTarget.samples = renderer.samples`, and
    /// `OITPassNode.setup()`'s copy of it onto the OIT target, ahead of the
    /// composite's build.
    fn sync_before_build(&self, samples: u32) {
        self.pass.render_target().set_samples(samples);
        self.oit_render_target.set_samples(samples);
    }
}
