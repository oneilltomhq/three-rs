//! Port of `three.js/src/nodes/display/PassNode.js`.
//!
//! `pass( scene, camera )` is a `TempNode` that owns a `RenderTarget`, renders
//! the given scene into it once per frame from `updateBefore()`, and evaluates
//! to the target's colour texture. Its `setup()` returns a `PassTextureNode`,
//! which is itself a `TempNode`, so a pass **used as a value** contributes two
//! vars to the generated shader — `nodeVarN = textureSample( … ); nodeVarN+1 =
//! nodeVarN;` — and `.a` is taken on the outer one. `to_var( texture_uv( … ) )`
//! reproduces that exactly ([`PassNode::node`]).
//!
//! `passNode.getTextureNode()` is the *inner* node on its own, so it emits one
//! var and no copy ([`PassNode::texture_node`]). `webgpu_postprocessing_masking`
//! takes the first form and `webgpu_postprocessing_difference` the second; both
//! dumps show the difference.
//!
//! `PassTextureNode` also calls `setUpdateMatrix( false )`, so a pass samples
//! the raw `uv()` varying with no texture matrix — unlike `texture( map )`,
//! which carries a `mat3x3` in the object uniform block.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::cameras::PerspectiveCamera;
use crate::core::Layers;
use crate::nodes::node::SettableValue;
use crate::nodes::tsl::{
    pass_depth_texture, perspective_depth_to_view_z, texture_uv, to_var, uniform_settable, uv,
};
use crate::nodes::{MrtNode, NodeRef, Type};
use crate::objects::Scene;
use crate::textures::{DepthTexture, TextureFilter, TextureType};

/// `PassNode`'s depth output name.
pub const DEPTH_ATTACHMENT: &str = "depth";

use super::render_target::{RenderTarget, RenderTargetOptions, OUTPUT_ATTACHMENT};
use super::Renderer;

/// `pass( scene, camera )`.
///
/// Ownership divergence from three.js, documented in `docs/postprocessing.md`:
/// three.js discovers the pass nodes of a frame by collecting the graph's
/// `updateBefore` nodes while the quad's material is built, and fires them from
/// inside the quad's own render. Rust's ownership rules make a node that holds
/// `&mut Scene` across a frame impractical, so the port keeps the node and calls
/// [`PassNode::render`] explicitly, immediately before
/// [`RenderPipeline::render`](super::RenderPipeline::render). The GPU sees the
/// same order: in three.js the nested renders use their own command encoders and
/// are submitted before the canvas pass they are nested inside.
pub struct PassNode {
    render_target: RenderTarget,
    node: NodeRef,
    /// `PassNode._previousTextureNodes` — the node for the *other* texture
    /// behind an output name, memoised like `_textureNodes` so that two asks
    /// compose one node.
    previous_texture_nodes: RefCell<HashMap<String, NodeRef>>,
    /// `PassNode._mrt` — the MRT the renderer is given for the duration of this
    /// pass's own render.
    mrt: RefCell<Option<MrtNode>>,
    /// `PassNode._textureNodes` — one node per named attachment, memoised so
    /// that two `getTextureNode( name )` calls compose the *same* node and the
    /// builder sees one texture, not two.
    texture_nodes: RefCell<HashMap<String, NodeRef>>,
    /// `PassNode._viewZNodes` — memoised like the texture nodes, so two asks
    /// compose one graph.
    view_z_nodes: RefCell<HashMap<String, NodeRef>>,
    /// `PassNode._cameraNear` / `_cameraFar`: `uniform( 0 )` a piece, written
    /// from the pass's camera in `updateBefore()`. They are object-group
    /// uniforms of whatever material samples the pass, which is why the fog
    /// composite's shader reads `object.nodeUniform1` / `object.nodeUniform2`.
    camera_near: (NodeRef, SettableValue),
    camera_far: (NodeRef, SettableValue),
    /// `PassNode.autoClearDepth`.
    auto_clear_depth: bool,
    /// Whether the depth attachment is this pass's own. A pass handed another
    /// pass's depth texture must not resize it — the owner already did, and
    /// `set_size` on a shared handle would drop the contents this pass exists
    /// to read.
    owns_depth_texture: bool,
    /// `PassNode._layers`.
    layers: RefCell<Option<Layers>>,
    /// `PassNode.opaque` / `.transparent` / `lighting.enabled`.
    opaque: bool,
    transparent: bool,
    lighting_enabled: bool,
}

/// `pass( scene, camera, options )`'s options object, as far as the ported
/// pages use it.
#[derive(Clone, Debug)]
pub struct PassOptions {
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    /// `options.depthTexture` — `depthTexture = options.depthTexture || new
    /// DepthTexture()`. A pass given another pass's depth attachment renders
    /// *into* it, which is how `webgpu_deferred`'s transparent pass depth-tests
    /// against the opaque G-buffer it never drew.
    pub depth_texture: Option<DepthTexture>,
    /// `options.autoClearDepth`, copied onto `renderer.autoClearDepth` for the
    /// duration of the pass. `false` keeps the shared depth attachment's
    /// contents (`loadOp: "load"`).
    pub auto_clear_depth: bool,
}

impl Default for PassOptions {
    /// `RenderTarget`'s own defaults: `LinearFilter` on both sides, a fresh
    /// `DepthTexture`, and `autoClearDepth` on.
    fn default() -> Self {
        Self {
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            depth_texture: None,
            auto_clear_depth: true,
        }
    }
}

impl Default for PassNode {
    fn default() -> Self {
        Self::new()
    }
}

impl PassNode {
    /// `new PassNode( PassNode.COLOR, scene, camera )`: a 1×1 `RenderTarget`
    /// with `{ type: HalfFloatType }` plus a `DepthTexture` named `depth`,
    /// resized to the drawing buffer on the first frame.
    pub fn new() -> Self {
        Self::new_with_options(PassOptions::default())
    }

    /// `pass( scene, camera, options )` — the third argument, which three
    /// spreads over `renderTarget.texture` and so over every attachment
    /// `getTexture( name )` clones from it.
    ///
    /// The filter pair is not cosmetic: `webgpu_mrt` passes `NearestFilter` on
    /// both sides, which makes all four attachments unfilterable, and the
    /// composite shader that samples them is generated with `textureLoad` and
    /// no samplers at all (`docs/nodes.md` §23).
    pub fn new_with_options(options: PassOptions) -> Self {
        let render_target = RenderTarget::new_with_options(
            1,
            1,
            RenderTargetOptions {
                // `PassNode.setup()` then overwrites this with
                // `renderer.getOutputBufferType()`, which is `HalfFloatType`.
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer: true,
                min_filter: options.min_filter,
                mag_filter: options.mag_filter,
            },
        )
        .expect("three-rs: PassNode's render target is a HalfFloat colour type");
        render_target.set_depth_texture(options.depth_texture.clone().unwrap_or_default());

        // `getTextureNode()`'s node — the `PassTextureNode` — and the
        // `PassNode` that wraps it. Both are `TempNode`s, which is why a pass
        // *used as a value* contributes two vars and `getTextureNode()` one;
        // sharing the inner node keeps them one texture to the builder.
        let texture_node = texture_uv(&render_target.texture(), uv());
        let node = to_var(None, texture_node.clone());

        let texture_nodes = HashMap::from([(OUTPUT_ATTACHMENT.to_string(), texture_node)]);

        let camera_near = uniform_settable(Type::F32, vec![0.0]);
        let camera_far = uniform_settable(Type::F32, vec![0.0]);

        Self {
            render_target,
            node,
            previous_texture_nodes: RefCell::new(HashMap::new()),
            mrt: RefCell::new(None),
            texture_nodes: RefCell::new(texture_nodes),
            view_z_nodes: RefCell::new(HashMap::new()),
            camera_near,
            camera_far,
            auto_clear_depth: options.auto_clear_depth,
            owns_depth_texture: options.depth_texture.is_none(),
            layers: RefCell::new(None),
            opaque: true,
            transparent: true,
            lighting_enabled: true,
        }
    }

    /// `passNode.setLayers( layers )` — the camera layer mask this pass renders
    /// with. `Layers::default()` (layer 0 only) is what `webgpu_deferred`'s
    /// opaque and transparent passes set, to keep the resolve quad on layer 2
    /// out of them.
    pub fn set_layers(&self, layers: Layers) {
        *self.layers.borrow_mut() = Some(layers);
    }

    /// `passNode.opaque` — `false` skips the opaque half of the render list,
    /// and with it the skybox.
    pub fn set_opaque(&mut self, opaque: bool) {
        self.opaque = opaque;
    }

    /// `passNode.transparent`.
    pub fn set_transparent(&mut self, transparent: bool) {
        self.transparent = transparent;
    }

    /// `passNode.lighting = new Lighting(); passNode.lighting.enabled = false`
    /// — the pass renders every material with an empty light list.
    pub fn set_lighting_enabled(&mut self, enabled: bool) {
        self.lighting_enabled = enabled;
    }

    /// `passNode.getTexture( 'depth' )` — the pass's own depth attachment.
    ///
    /// Three seeds `_textures[ 'depth' ]` in the constructor when the target
    /// has a depth buffer and throws from `getTexture()` for any other pass;
    /// here the target always has one, so this is infallible.
    pub fn depth_texture(&self) -> DepthTexture {
        self.render_target
            .depth_texture()
            .expect("three-rs: a PassNode's render target always carries a DepthTexture")
    }

    /// `passNode.getViewZNode( name )` — the depth attachment read back as a
    /// view-space z.
    ///
    /// `perspectiveDepthToViewZ( getTextureNode( name ), cameraNear,
    /// cameraFar )`. Only `'depth'` exists as a name here: a custom depth
    /// output would be an extra colour attachment, which no ported page asks
    /// for.
    pub fn view_z_node(&self, name: &str) -> NodeRef {
        if let Some(node) = self.view_z_nodes.borrow().get(name) {
            return node.clone();
        }
        assert_eq!(
            name, DEPTH_ATTACHMENT,
            "three-rs: PassNode::view_z_node only knows the 'depth' output"
        );
        let node = perspective_depth_to_view_z(
            pass_depth_texture(&self.depth_texture()),
            self.camera_near.0.clone(),
            self.camera_far.0.clone(),
        );
        self.view_z_nodes
            .borrow_mut()
            .insert(name.to_string(), node.clone());
        node
    }

    /// `passNode.setMRT( mrt )`.
    ///
    /// The names the MRT writes are *not* what creates the attachments —
    /// `getTextureNode( name )` is, exactly as in three.js, where
    /// `MRTNode.setup()` silently drops an output whose name is not among
    /// `renderTarget.textures`. `webgpu_postprocessing_bloom_selective` asks
    /// for `getTextureNode( 'bloomIntensity' )` and so gets the second
    /// attachment; a page that set the MRT and never sampled the extra output
    /// would render single-attachment, in three.js too.
    pub fn set_mrt(&self, mrt: MrtNode) {
        *self.mrt.borrow_mut() = Some(mrt);
    }

    /// `passNode.getMRT()`.
    pub fn mrt(&self) -> Option<MrtNode> {
        self.mrt.borrow().clone()
    }

    /// `passNode.getTextureNode( name )` — the node for one named colour
    /// attachment, creating the attachment on first ask
    /// (`PassNode.getTexture()`).
    ///
    /// Unlike [`PassNode::node`] this is **one** var, not two: a
    /// `PassTextureNode` is what the graph holds, and the `PassNode` it builds
    /// from its own `setup()` is then built only that once — `hasDependencies`
    /// is false, so `TempNode` gives it no var of its own. `pass( scene, camera )`
    /// used directly is the case with two, because there the graph holds the
    /// `PassNode` as well and its usage count reaches two.
    ///
    /// `getTextureNode()` with no argument is that direct use,
    /// [`PassNode::node`]; `getTextureNode( 'output' )` is this, on the same
    /// `renderTarget.textures[ 0 ]`.
    pub fn texture_node(&self, name: &str) -> NodeRef {
        if let Some(node) = self.texture_nodes.borrow().get(name) {
            return node.clone();
        }
        // `PassNode`'s constructor seeds `_textures.depth` with the depth
        // attachment, so `getTextureNode( 'depth' )` is a tap on *that* and
        // not a new colour attachment. `webgpu_deferred`'s resolve material
        // reads it for both its `discard` and its `depthNode`.
        if name == DEPTH_ATTACHMENT {
            let node = pass_depth_texture(&self.depth_texture());
            self.texture_nodes
                .borrow_mut()
                .insert(name.to_string(), node.clone());
            return node;
        }
        let texture = if name == crate::renderer::OUTPUT_ATTACHMENT {
            self.render_target.texture()
        } else {
            self.render_target.add_texture(name)
        };
        let node = texture_uv(&texture, uv());
        self.texture_nodes
            .borrow_mut()
            .insert(name.to_string(), node.clone());
        node
    }

    /// `passNode.getPreviousTextureNode( name )` — the node for the frame
    /// *before* this one on that output.
    ///
    /// `PassNode.updateBefore()` calls `toggleTexture()` for every name that
    /// has one **before** it renders, so on the very first frame the scene is
    /// drawn into one of the pair and the node named "previous" points at the
    /// other, which nothing has ever rendered into: a zero-initialised
    /// texture. `webgpu_postprocessing_difference` grades exactly that frame,
    /// so its `|previous − current|` is `|0 − current|` and the whole image is
    /// a saturated version of the box. Rendering the previous buffer, or
    /// swapping in the other order, is a different picture.
    pub fn previous_texture_node(&self, name: &str) -> NodeRef {
        if let Some(node) = self.previous_texture_nodes.borrow().get(name) {
            return node.clone();
        }
        // `if ( this._textureNodes[ name ] === undefined ) this.getTextureNode(
        // name );` — the attachment has to exist before its shadow does.
        let _ = self.texture_node(name);
        let texture = self.render_target.add_previous_texture(name);
        let node = texture_uv(&texture, uv());
        self.previous_texture_nodes
            .borrow_mut()
            .insert(name.to_string(), node.clone());
        node
    }

    /// `passNode.getTexture( name )`.
    pub fn texture_named(&self, name: &str) -> crate::textures::Texture {
        self.render_target.add_texture(name)
    }

    /// `passNode.getTextureNode()` — the node to compose with.
    pub fn node(&self) -> NodeRef {
        self.node.clone()
    }

    /// `pass( … ).a`.
    pub fn a(&self) -> NodeRef {
        self.node.a()
    }

    /// `renderTarget.texture`.
    pub fn texture(&self) -> crate::textures::Texture {
        self.render_target.texture()
    }

    /// `passNode.renderTarget` — the accumulator an `SSAAPassNode` clones and
    /// then draws into. `pub(crate)` because a `RenderTarget` is a handle and
    /// handing one out would let an application render into a pass's target
    /// behind its back.
    pub(crate) fn render_target(&self) -> &RenderTarget {
        &self.render_target
    }

    /// `PassNode.updateBefore( frame )`: size the target to the drawing buffer,
    /// then `renderer.setRenderTarget( this.renderTarget ); renderer.render(
    /// this.scene, this.camera )` with the previous target restored after.
    pub fn render(
        &self,
        renderer: &mut Renderer,
        scene: &mut Scene,
        camera: &mut PerspectiveCamera,
    ) {
        let (width, height) = renderer.drawing_buffer_size();
        self.render_target
            .set_size_keeping_depth(width, height, !self.owns_depth_texture);
        // `PassNode.setup()`: `renderTarget.samples = renderer.samples`.
        self.render_target.set_samples(renderer.samples());

        // `for ( const name in this._previousTextures ) this.toggleTexture(
        // name );` — before the render, not after.
        for name in self.previous_texture_nodes.borrow().keys() {
            self.render_target.toggle_texture(name);
        }

        // `this._cameraNear.value = camera.near; this._cameraFar.value =
        // camera.far;`
        self.camera_near.1.set(vec![camera.near]);
        self.camera_far.1.set(vec![camera.far]);

        let previous = renderer.render_target();
        let previous_mrt = renderer.mrt();
        let previous_auto_clear_depth = renderer.auto_clear_depth;
        let previous_opaque = renderer.opaque;
        let previous_transparent = renderer.transparent;
        let previous_lighting = renderer.lighting_enabled;
        let previous_layers = renderer.camera_layers;

        renderer.set_render_target(Some(self.render_target.clone()));
        renderer.set_mrt(self.mrt.borrow().clone());
        renderer.auto_clear_depth = self.auto_clear_depth;
        renderer.opaque = self.opaque;
        renderer.transparent = self.transparent;
        renderer.lighting_enabled = self.lighting_enabled;
        renderer.camera_layers = *self.layers.borrow();

        renderer.render(scene, camera);

        renderer.set_render_target(previous);
        renderer.set_mrt(previous_mrt);
        renderer.auto_clear_depth = previous_auto_clear_depth;
        renderer.opaque = previous_opaque;
        renderer.transparent = previous_transparent;
        renderer.lighting_enabled = previous_lighting;
        renderer.camera_layers = previous_layers;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `dump-difference/dump.json` has two quad bind groups: 63, built with
    /// the previous node on texture 4, and 64 — the one pass 0 actually binds
    /// — with the current node on texture 4 (the rendered target) and the
    /// previous node on texture 59, which no pass ever writes. So the two
    /// nodes name two different textures, and only one of them is an
    /// attachment.
    ///
    /// No GPU: this is the target's own bookkeeping, before any render.
    #[test]
    fn the_previous_texture_is_never_a_colour_attachment() {
        let pass = PassNode::new();
        let current = pass.texture_node(OUTPUT_ATTACHMENT);
        let previous = pass.previous_texture_node(OUTPUT_ATTACHMENT);

        assert_ne!(current.key(), previous.key());

        let attachments: Vec<usize> = pass
            .render_target()
            .textures()
            .iter()
            .map(|texture| texture.id())
            .collect();
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0], pass.render_target().texture().id());

        let shadow = pass.render_target().add_previous_texture(OUTPUT_ATTACHMENT);
        assert!(!attachments.contains(&shadow.id()));
    }

    /// Both accessors are memoised, so a page that asks twice composes one
    /// node and the builder sees one texture — `_textureNodes` /
    /// `_previousTextureNodes`.
    #[test]
    fn the_texture_nodes_are_memoised() {
        let pass = PassNode::new();
        assert_eq!(
            pass.texture_node(OUTPUT_ATTACHMENT).key(),
            pass.texture_node(OUTPUT_ATTACHMENT).key()
        );
        assert_eq!(
            pass.previous_texture_node(OUTPUT_ATTACHMENT).key(),
            pass.previous_texture_node(OUTPUT_ATTACHMENT).key()
        );
    }
}
