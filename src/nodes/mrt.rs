//! Port of `three.js/src/nodes/core/MRTNode.js` and its base
//! `src/nodes/core/OutputStructNode.js`.
//!
//! `mrt( { output, bloomIntensity: uniform( 1 ) } )` names the values a
//! material writes to a render target's several colour attachments. Both
//! levels three.js has are here: a *pass*-level MRT
//! ([`PassNode::set_mrt`](crate::renderer::PassNode::set_mrt), which the
//! renderer holds while the pass renders) and a *material*-level one
//! ([`MeshBasicNodeMaterial::mrt_node`](crate::materials::MeshBasicNodeMaterial::mrt_node)),
//! merged by `NodeMaterial.setup()` with the material's entries winning.
//!
//! An [`MrtNode`] is an ordered dictionary, not a node: `MRTNode.setup()`
//! resolves each name against the bound render target's `textures` array and
//! fills `OutputStructNode.members` by *index*, so the order of the attachments
//! decides the `@location` each value lands on and the dictionary's own order
//! decides nothing. A name with no matching attachment is dropped — three.js's
//! "ignore if the output exists in the MRT but has never been used".

use std::hash::{Hash, Hasher};
use std::rc::Rc;

use super::{NodeRef, Type};
use crate::materials::BlendMode;

/// One entry of an [`MrtNode`].
///
/// Three's outputs are *node objects*, and a node object's `setup( builder )`
/// runs once per material build — so `mrt( { normal: packNormalToRGB(
/// normalView ) } )` means a different `normalView` for every material the
/// pass draws. This port's TSL is eager: `normal_view()` reads the material
/// side and normal map that are installed *now*, so an expression built in an
/// example's `init()` would freeze the defaults into every draw.
///
/// [`MrtValue::Deferred`] is the shim for that: a closure re-run inside each
/// material's setup, where the thread-locals hold that material's state. See
/// `docs/nodes.md` §23.
#[derive(Clone)]
pub(crate) enum MrtValue {
    /// A value that does not depend on the material, such as a uniform or a
    /// property read like `output` or `diffuseColor`.
    Node(NodeRef),
    /// A value rebuilt per material, standing in for a three.js node object
    /// whose `setup()` reads the builder's material.
    Deferred(Rc<dyn Fn() -> NodeRef>),
}

impl MrtValue {
    /// The node for the material currently being set up.
    fn resolve(&self) -> NodeRef {
        match self {
            MrtValue::Node(node) => node.clone(),
            MrtValue::Deferred(build) => build(),
        }
    }
}

impl std::fmt::Debug for MrtValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MrtValue::Node(node) => node.fmt(f),
            MrtValue::Deferred(_) => f.write_str("Deferred"),
        }
    }
}

/// `mrt( outputNodes )` — the outputs by name, in insertion order.
///
/// Cloning is a value copy, unlike a `NodeRef`: three.js's `merge()` builds a
/// fresh `MRTNode` too, so nothing observes the identity of one.
#[derive(Clone, Debug, Default)]
pub struct MrtNode {
    outputs: Vec<(String, MrtValue)>,
    /// `MRTNode.blendModes`, less its `output: MaterialBlending` seed: the
    /// port has no `MaterialBlending` constant, and an unset `output` is
    /// resolved to the material's own blending where the pipeline is built
    /// instead (see [`MrtNode::blend_mode`]).
    blend_modes: Vec<(String, BlendMode)>,
    /// `MRTNode.clearColors` — `( linear r, g, b, a )` by output name.
    clear_colors: Vec<(String, [f64; 4])>,
}

/// `mrt( { name: node, … } )`.
///
/// Takes a `Vec` rather than a map because a JS object literal's key order is
/// its insertion order and `merge()` depends on it.
pub fn mrt<N: Into<String>>(outputs: Vec<(N, NodeRef)>) -> MrtNode {
    let mut node = MrtNode::default();
    for (name, value) in outputs {
        node.set(name, value);
    }
    node
}

impl MrtNode {
    /// `mrtNode.outputNodes[ name ] = value`, replacing in place so the
    /// insertion order of an existing key is kept — `{ ...a, ...b }`.
    pub fn set<N: Into<String>>(&mut self, name: N, value: NodeRef) {
        self.set_value(name, MrtValue::Node(value));
    }

    /// `mrtNode.outputNodes[ name ] = node` where the node's `setup()` reads
    /// the material — `packNormalToRGB( normalView )` and nothing else on this
    /// ladder. The closure runs once per material build, inside the same
    /// material state `NodeMaterial.setup()` installs, so `normalView` picks up
    /// that material's normal map and `side`.
    pub fn set_deferred<N: Into<String>>(
        &mut self,
        name: N,
        build: impl Fn() -> NodeRef + 'static,
    ) {
        self.set_value(name, MrtValue::Deferred(Rc::new(build)));
    }

    fn set_value<N: Into<String>>(&mut self, name: N, value: MrtValue) {
        let name = name.into();
        match self.outputs.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = value,
            None => self.outputs.push((name, value)),
        }
    }

    /// `mrtNode.setBlendMode( name, blendMode )` — the blend state the
    /// pipeline gives *that* attachment.
    ///
    /// Three defaults attachment `output` to the material's own blending and
    /// every other attachment to `NoBlending`; `webgpu_postprocessing_bloom_emissive`
    /// puts `NormalBlending` back on its `emissive` output, and `OITPassNode`
    /// gives its two accumulation targets `CustomBlending` with factors of
    /// their own. A bare [`Blending`] preset converts into
    /// `new BlendMode( blending )`. On an opaque draw `NormalBlending` and no
    /// blending agree to the bit — `src-alpha` is 1 and `one-minus-src-alpha` 0
    /// — which is why the bloom case is a pipeline-descriptor fidelity item
    /// rather than a pixel one; `docs/postprocessing.md` says so.
    ///
    /// Returns `self`, as three's does, so the calls chain.
    ///
    /// [`Blending`]: crate::materials::Blending
    pub fn set_blend_mode<N: Into<String>>(
        &mut self,
        name: N,
        blend_mode: impl Into<BlendMode>,
    ) -> &mut Self {
        let name = name.into();
        let blend_mode = blend_mode.into();
        match self.blend_modes.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = blend_mode,
            None => self.blend_modes.push((name, blend_mode)),
        }
        self
    }

    /// `mrtNode.getBlendMode( name )`, without three's fallback: `None` where
    /// three returns its `output: MaterialBlending` seed (for `output`) or
    /// `_noBlending` (for everything else). The renderer applies that rule
    /// when it builds the pipeline's colour targets.
    pub fn blend_mode(&self, name: &str) -> Option<BlendMode> {
        self.blend_modes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| *b)
    }

    /// `mrtNode.setClearColor( name, color, alpha = 1 )` — the value the
    /// render pass clears *that* attachment to, in place of the renderer's
    /// clear colour (attachment 0) or `( 0, 0, 0, 1 )` (every other one).
    /// `OITPassNode` clears `accum` to `( 0, 0, 0, 0 )` and `revealage` to
    /// `( 1, 1, 1, 1 )`.
    ///
    /// `color` is in the working (linear) colour space, as three's
    /// `Color4.set( hex )` leaves it.
    pub fn set_clear_color<N: Into<String>>(
        &mut self,
        name: N,
        color: crate::math::Color,
        alpha: f64,
    ) -> &mut Self {
        let name = name.into();
        let value = [color.r, color.g, color.b, alpha];
        match self.clear_colors.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = value,
            None => self.clear_colors.push((name, value)),
        }
        self
    }

    /// `mrtNode.getClearColor( name )` — `( r, g, b, a )`, or `None` for
    /// three's `null`.
    pub fn clear_color(&self, name: &str) -> Option<[f64; 4]> {
        self.clear_colors
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| *c)
    }

    /// `mrtNode.has( name )`.
    pub fn has(&self, name: &str) -> bool {
        self.outputs.iter().any(|(n, _)| n == name)
    }

    /// `mrtNode.get( name )`, resolved for the material being set up.
    pub fn get(&self, name: &str) -> Option<NodeRef> {
        self.outputs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.resolve())
    }

    /// The names this MRT writes, in insertion order.
    #[cfg(test)]
    fn names(&self) -> impl Iterator<Item = &str> {
        self.outputs.iter().map(|(n, _)| n.as_str())
    }

    /// `mrtNode.merge( other )` — `{ ...this.outputNodes, ...other.outputNodes }`,
    /// and the same for the blend modes and the clear colours.
    /// The material's entries overwrite the pass's, which is how
    /// `webgpu_postprocessing_bloom_selective` gives each sphere its own
    /// `bloomIntensity` on top of the pass's `float( 0 )` default.
    ///
    /// `other.blendModes` always has an `output` entry in three — the
    /// constructor's `MaterialBlending` seed, if nothing replaced it — so
    /// `this`'s `output` never survives the spread. The port leaves the seed
    /// out ([`MrtNode::blend_mode`]), so it drops `this`'s `output` first.
    pub fn merge(&self, other: &MrtNode) -> MrtNode {
        let mut merged = self.clone();
        for (name, value) in &other.outputs {
            merged.set_value(name.clone(), value.clone());
        }
        merged.blend_modes.retain(|(name, _)| name != "output");
        for (name, blend_mode) in &other.blend_modes {
            merged.set_blend_mode(name.clone(), *blend_mode);
        }
        for (name, color) in &other.clear_colors {
            let [r, g, b, a] = *color;
            merged.set_clear_color(name.clone(), crate::math::Color::new(r, g, b), a);
        }
        merged
    }

    /// `MRTNode.setup()`: resolve each output against the bound target's
    /// attachment names and lay the members out by attachment *index*, each
    /// with the type it is converted to.
    ///
    /// `attachments` is `renderTarget.textures.map( t => t.name )`. An output
    /// whose name is not among them is skipped (`index === -1`), and the
    /// members array is trimmed to the last one that is — an
    /// `OutputStructNode` with a hole would generate a read of `undefined` in
    /// three.js too, so nothing is lost by not modelling it.
    ///
    /// `types` is `builder.getOutputType( index )` per attachment — the
    /// texture's channel count and component type, so `OITPassNode`'s `RedFormat` `revealage`
    /// is an `f32` member. Shorter than `attachments` (or empty) means `vec4`
    /// for the rest, which every `RGBAFormat` attachment is.
    pub(crate) fn members(&self, attachments: &[String], types: &[Type]) -> Vec<(NodeRef, Type)> {
        let mut members: Vec<Option<NodeRef>> = vec![None; attachments.len()];
        for (name, value) in &self.outputs {
            if let Some(index) = get_texture_index(attachments, name) {
                members[index] = Some(value.resolve());
            }
        }
        while matches!(members.last(), Some(None)) {
            members.pop();
        }
        members
            .into_iter()
            .enumerate()
            .map(|(index, m)| {
                (
                    m.expect("three-rs: an MRT member index below the last is always filled"),
                    types.get(index).copied().unwrap_or(Type::Vec4),
                )
            })
            .collect()
    }
}

/// `getTextureIndex( textures, name )` — `MRTNode.js`: the position of the
/// attachment called `name` among a render target's textures, which is the
/// `@location` an MRT output of that name is written to. Three takes the
/// textures and compares their `.name`; the port's attachments are named by
/// the target (`renderTarget.textures.map( t => t.name )`), so this takes the
/// names. `None` is three's `-1`.
pub fn get_texture_index<S: AsRef<str>>(names: &[S], name: &str) -> Option<usize> {
    names.iter().position(|n| n.as_ref() == name)
}

/// By name and node identity, the way [`FogNode`](super::tsl::FogNode) hashes:
/// this is part of a render object's dynamic program key, so a *new* graph is a
/// new program and a uniform inside the same one is not.
impl Hash for MrtNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for (name, value) in &self.outputs {
            name.hash(state);
            match value {
                MrtValue::Node(node) => node.key().hash(state),
                // The closure's identity, not its result: resolving it here
                // would build nodes outside any material's setup, which is the
                // very thing it exists to avoid. Two passes with distinct
                // `set_deferred` calls therefore key distinct programs even if
                // the closures would agree — and the *material* state the
                // closure reads is already in the key by other fields.
                MrtValue::Deferred(build) => Rc::as_ptr(build).cast::<()>().hash(state),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::tsl::{float, output_property};

    #[test]
    fn merge_keeps_the_pass_order_and_takes_the_material_value() {
        let pass = mrt(vec![
            ("output", output_property()),
            ("bloomIntensity", float(0.0)),
        ]);
        let material = mrt(vec![("bloomIntensity", float(1.0))]);
        let merged = pass.merge(&material);

        assert_eq!(
            merged.names().collect::<Vec<_>>(),
            ["output", "bloomIntensity"]
        );
        assert_eq!(
            merged.get("bloomIntensity").map(|n| n.key()),
            material.get("bloomIntensity").map(|n| n.key())
        );
    }

    #[test]
    fn an_output_with_no_attachment_is_dropped() {
        let node = mrt(vec![
            ("output", output_property()),
            ("velocity", float(2.0)),
            ("bloomIntensity", float(1.0)),
        ]);
        let attachments = ["output".to_string(), "bloomIntensity".to_string()];
        let members = node.members(&attachments, &[]);

        assert_eq!(members.len(), 2);
        assert_eq!(members[0].0.key(), output_property().key());
    }

    #[test]
    fn members_are_laid_out_by_attachment_index_not_dictionary_order() {
        let node = mrt(vec![
            ("bloomIntensity", float(1.0)),
            ("output", output_property()),
        ]);
        let attachments = ["output".to_string(), "bloomIntensity".to_string()];
        let members = node.members(&attachments, &[]);

        assert_eq!(members[0].0.key(), output_property().key());
        assert_eq!(members[1].0.ty(), crate::nodes::Type::F32);
        assert_eq!(members[1].1, crate::nodes::Type::Vec4);
    }

    /// `OITPassNode._getMRTNode()`'s chain: the blend modes and clear colours
    /// are kept per output name, and a later set replaces an earlier one.
    #[test]
    fn blend_modes_and_clear_colors_are_per_output() {
        use crate::materials::{BlendFactor, Blending};
        use crate::math::Color;

        let accum = BlendMode {
            blend_src: BlendFactor::One,
            blend_dst: BlendFactor::One,
            ..BlendMode::new(Blending::Custom)
        };
        let mut node = mrt(vec![("accum", float(1.0)), ("revealage", float(1.0))]);
        node.set_blend_mode("accum", Blending::Normal)
            .set_blend_mode("accum", accum)
            .set_blend_mode("revealage", Blending::Additive)
            .set_clear_color("accum", Color::from_hex(0x000000), 0.0)
            .set_clear_color("revealage", Color::from_hex(0xffffff), 1.0);

        assert_eq!(node.blend_mode("accum"), Some(accum));
        assert_eq!(
            node.blend_mode("revealage"),
            Some(BlendMode::new(Blending::Additive))
        );
        assert_eq!(node.blend_mode("output"), None);
        assert_eq!(node.clear_color("accum"), Some([0.0, 0.0, 0.0, 0.0]));
        assert_eq!(node.clear_color("revealage"), Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(node.clear_color("output"), None);

        // The pipeline state the `accum` mode becomes: `One` / `One`, the
        // alpha factors following the colour ones (three's `null`).
        let state = crate::materials::blending::blending(&accum).expect("custom blends");
        assert_eq!(state.color.src_factor, wgpu::BlendFactor::One);
        assert_eq!(state.color.dst_factor, wgpu::BlendFactor::One);
        assert_eq!(state.alpha, state.color);
    }

    /// `merge()` carries the other node's blend modes and clear colours over
    /// this one's, as it does its outputs — `output`'s blend mode included:
    /// the other node's `MaterialBlending` seed wins over this one's
    /// `setBlendMode( 'output', … )`, unless the other node set its own.
    #[test]
    fn merge_keeps_blend_modes_and_clear_colors() {
        use crate::materials::Blending;
        use crate::math::Color;

        let mut pass = mrt(vec![("output", output_property())]);
        pass.set_clear_color("output", Color::from_hex(0x000000), 1.0)
            .set_clear_color("emissive", Color::from_hex(0x000000), 1.0)
            .set_blend_mode("output", Blending::No)
            .set_blend_mode("bloom", Blending::Additive);
        let mut material = mrt(vec![("emissive", float(0.0))]);
        material
            .set_clear_color("output", Color::from_hex(0xffffff), 0.5)
            .set_blend_mode("emissive", Blending::Normal);

        let merged = pass.merge(&material);
        assert_eq!(merged.clear_color("output"), Some([1.0, 1.0, 1.0, 0.5]));
        assert_eq!(merged.clear_color("emissive"), Some([0.0, 0.0, 0.0, 1.0]));
        // The material's seed: `MaterialBlending`, which the port leaves unset.
        assert_eq!(merged.blend_mode("output"), None);
        assert_eq!(
            merged.blend_mode("bloom"),
            Some(BlendMode::new(Blending::Additive))
        );
        assert_eq!(
            merged.blend_mode("emissive"),
            Some(BlendMode::new(Blending::Normal))
        );

        // A material that sets `output` itself replaces the pass's.
        material.set_blend_mode("output", Blending::Additive);
        assert_eq!(
            pass.merge(&material).blend_mode("output"),
            Some(BlendMode::new(Blending::Additive))
        );
    }

    /// `getOutputType( index )`: a member takes the type the renderer gives
    /// its attachment — `OITPassNode`'s `r8unorm` `revealage` an `f32`.
    #[test]
    fn members_take_the_attachment_types() {
        let node = mrt(vec![
            ("accum", output_property()),
            ("revealage", float(1.0)),
        ]);
        let attachments = ["accum".to_string(), "revealage".to_string()];
        let members = node.members(&attachments, &[Type::Vec4, Type::F32]);
        assert_eq!(members[0].1, Type::Vec4);
        assert_eq!(members[1].1, Type::F32);
    }
}
