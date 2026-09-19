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

use super::NodeRef;

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
pub enum MrtValue {
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
    blend_modes: Vec<(String, crate::materials::Blending)>,
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
    /// puts `NormalBlending` back on its `emissive` output. On an opaque draw
    /// the two agree to the bit — `src-alpha` is 1 and `one-minus-src-alpha` 0
    /// — which is why this is a pipeline-descriptor fidelity item rather than a
    /// pixel one; `docs/postprocessing.md` says so.
    pub fn set_blend_mode<N: Into<String>>(
        &mut self,
        name: N,
        blending: crate::materials::Blending,
    ) {
        let name = name.into();
        match self.blend_modes.iter_mut().find(|(n, _)| *n == name) {
            Some(entry) => entry.1 = blending,
            None => self.blend_modes.push((name, blending)),
        }
    }

    /// `mrtNode.getBlendMode( name )`.
    pub fn blend_mode(&self, name: &str) -> Option<crate::materials::Blending> {
        self.blend_modes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, b)| *b)
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
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.outputs.iter().map(|(n, _)| n.as_str())
    }

    /// `mrtNode.merge( other )` — `{ ...this.outputNodes, ...other.outputNodes }`.
    /// The material's entries overwrite the pass's, which is how
    /// `webgpu_postprocessing_bloom_selective` gives each sphere its own
    /// `bloomIntensity` on top of the pass's `float( 0 )` default.
    pub fn merge(&self, other: &MrtNode) -> MrtNode {
        let mut merged = self.clone();
        for (name, value) in &other.outputs {
            merged.set_value(name.clone(), value.clone());
        }
        for (name, blending) in &other.blend_modes {
            merged.set_blend_mode(name.clone(), *blending);
        }
        merged
    }

    /// `MRTNode.setup()`: resolve each output against the bound target's
    /// attachment names and lay the members out by attachment *index*.
    ///
    /// `attachments` is `renderTarget.textures.map( t => t.name )`. An output
    /// whose name is not among them is skipped (`index === -1`), and the
    /// members array is trimmed to the last one that is — an
    /// `OutputStructNode` with a hole would generate a read of `undefined` in
    /// three.js too, so nothing is lost by not modelling it.
    pub fn members(&self, attachments: &[String]) -> Vec<NodeRef> {
        let mut members: Vec<Option<NodeRef>> = vec![None; attachments.len()];
        for (name, value) in &self.outputs {
            if let Some(index) = attachments.iter().position(|a| a == name) {
                members[index] = Some(value.resolve());
            }
        }
        while matches!(members.last(), Some(None)) {
            members.pop();
        }
        members
            .into_iter()
            .map(|m| m.expect("three-rs: an MRT member index below the last is always filled"))
            .collect()
    }
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
        let members = node.members(&attachments);

        assert_eq!(members.len(), 2);
        assert_eq!(members[0].key(), output_property().key());
    }

    #[test]
    fn members_are_laid_out_by_attachment_index_not_dictionary_order() {
        let node = mrt(vec![
            ("bloomIntensity", float(1.0)),
            ("output", output_property()),
        ]);
        let attachments = ["output".to_string(), "bloomIntensity".to_string()];
        let members = node.members(&attachments);

        assert_eq!(members[0].key(), output_property().key());
        assert_eq!(members[1].ty(), crate::nodes::Type::F32);
    }
}
