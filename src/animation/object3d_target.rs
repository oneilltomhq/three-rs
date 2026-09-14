//! The getter/setter half of `three.js/src/animation/PropertyBinding.js`, for
//! the `Object3D` tree: `findNode` plus the `GetterByBindingType` /
//! `SetterByBindingTypeAndVersioning` pair for the property paths glTF
//! produces.
//!
//! [`SceneResolver`] is the [`TargetResolver`] the mixer wants:
//! `mixer.clipAction( clip, root ).play(); mixer.update( 0 )` writes into
//! `position` / `quaternion` / `scale` / `morphTargetInfluences`.
//!
//! Only those four paths are implemented, because they are all glTF emits
//! (`GLTFLoader`'s `PATH_PROPERTIES`). Everything else is an explicit TODO in
//! [`SceneResolver::resolve`]: three.js also binds `material.*` (via
//! `objectName = 'material'`), `bones[ n ]`, `.morphTargetInfluences[ name ]`
//! by morph-target name, and any `fromArray`-able property by index.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::animation::binding_target::{BindingTarget, TargetResolver};
use crate::animation::property_binding::ParsedTrackName;
use crate::core::Node;
use crate::objects::Skeleton;

/// Which `Object3D` property a [`NodeTarget`] is bound to. Three picks one of
/// sixteen closures by binding type and versioning; these are the three
/// `HasFromToArray` cases plus the `EntireArray` case, all with
/// `Versioning.MatrixWorldNeedsUpdate` (every `Object3D` has
/// `matrixWorldNeedsUpdate`, which is the flag Three's setter raises).
#[derive(Clone, Debug)]
pub enum NodeProperty {
    /// `.position`.
    Position,
    /// `.quaternion`.
    Quaternion,
    /// `.scale`.
    Scale,
    /// `.morphTargetInfluences` — shared, because the influences live on the
    /// `SkinnedMesh`/`Mesh` beside the node, not on `Object3D` itself.
    MorphTargetInfluences(Rc<RefCell<Vec<f64>>>),
}

/// One resolved `Object3D` property.
pub struct NodeTarget {
    node: Node,
    property: NodeProperty,
}

impl NodeTarget {
    /// The binding for `node.<property>`.
    pub fn new(node: Node, property: NodeProperty) -> Self {
        Self { node, property }
    }
}

impl BindingTarget for NodeTarget {
    fn get_value(&self, buffer: &mut [f64], offset: usize) {
        let object = self.node.borrow();

        match &self.property {
            NodeProperty::Position => write(buffer, offset, &object.position.to_array()),
            NodeProperty::Quaternion => write(buffer, offset, &object.quaternion.to_array()),
            NodeProperty::Scale => write(buffer, offset, &object.scale.to_array()),
            NodeProperty::MorphTargetInfluences(influences) => {
                write(buffer, offset, &influences.borrow())
            }
        }
    }

    fn set_value(&mut self, buffer: &[f64], offset: usize) {
        let mut object = self.node.borrow_mut();

        match &self.property {
            NodeProperty::Position => {
                object.position.from_array(buffer, offset);
            }
            NodeProperty::Quaternion => {
                object.quaternion.from_array(buffer, offset);
                // `Quaternion.fromArray` fires `_onChangeCallback`, which is
                // what keeps `Object3D.rotation` in step.
                object.sync_rotation_from_quaternion();
            }
            NodeProperty::Scale => {
                object.scale.from_array(buffer, offset);
            }
            NodeProperty::MorphTargetInfluences(influences) => {
                let mut influences = influences.borrow_mut();
                let n = influences.len();
                influences.copy_from_slice(&buffer[offset..offset + n]);
            }
        }

        // `Versioning.MatrixWorldNeedsUpdate`
        object.matrix_world_needs_update = true;
    }

    fn value_size(&self) -> usize {
        match &self.property {
            NodeProperty::Position | NodeProperty::Scale => 3,
            NodeProperty::Quaternion => 4,
            NodeProperty::MorphTargetInfluences(influences) => influences.borrow().len(),
        }
    }
}

fn write(buffer: &mut [f64], offset: usize, values: &[f64]) {
    buffer[offset..offset + values.len()].copy_from_slice(values);
}

/// `PropertyBinding`'s tree side: an `Object3D` root plus the side table of
/// `morphTargetInfluences`, which this crate keeps beside the node rather than
/// on it.
pub struct SceneResolver {
    root: Node,
    /// `root.skeleton`, searched by `findNode` before the subtree.
    skeleton: Option<Rc<RefCell<Skeleton>>>,
    /// `node.id` to that node's `morphTargetInfluences`.
    morph_target_influences: HashMap<u32, Rc<RefCell<Vec<f64>>>>,
}

impl SceneResolver {
    /// A resolver over `root`.
    pub fn new(root: Node) -> Self {
        Self {
            root,
            skeleton: None,
            morph_target_influences: HashMap::new(),
        }
    }

    /// `root.skeleton`, so `findNode` can reach a bone that is not under the
    /// root (Three searches the skeleton first).
    pub fn with_skeleton(mut self, skeleton: Rc<RefCell<Skeleton>>) -> Self {
        self.skeleton = Some(skeleton);
        self
    }

    /// Register a node's `morphTargetInfluences`, so a
    /// `<node>.morphTargetInfluences` track can resolve.
    pub fn add_morph_target_influences(
        &mut self,
        node: &Node,
        influences: Rc<RefCell<Vec<f64>>>,
    ) -> &mut Self {
        self.morph_target_influences
            .insert(node.borrow().id, influences);
        self
    }

    /// `PropertyBinding.findNode( root, nodeName )`.
    pub fn find_node(&self, node_name: Option<&str>) -> Option<Node> {
        let name = match node_name {
            // `nodeName === undefined || nodeName === '' || nodeName === '.' ||
            //  nodeName === - 1 || nodeName === root.name`
            None | Some("") | Some(".") | Some("-1") => return Some(self.root.clone()),
            Some(name) if name == self.root.borrow().name => return Some(self.root.clone()),
            Some(name) => name,
        };

        // `// search into skeleton bones.`
        if let Some(skeleton) = &self.skeleton {
            if let Some(bone) = skeleton.borrow().get_bone_by_name(name) {
                return Some(bone);
            }
        }

        // `// search into node subtree.`
        search_node_subtree(&self.root.children(), name)
    }
}

/// `PropertyBinding.findNode`'s inner `searchNodeSubtree`. Note it does *not*
/// test the root itself, and does not look at nested skeletons.
fn search_node_subtree(children: &[Node], name: &str) -> Option<Node> {
    for child in children {
        if child.borrow().name == name {
            return Some(child.clone());
        }

        if let Some(result) = search_node_subtree(&child.children(), name) {
            return Some(result);
        }
    }

    None
}

impl TargetResolver for SceneResolver {
    fn resolve(&mut self, parsed: &ParsedTrackName) -> Option<Box<dyn BindingTarget>> {
        let node = self.find_node(parsed.node_name.as_deref())?;

        // TODO: `objectName` — Three's `material`, `materials`, `bones` and
        // `map` cases, which re-target `targetObject` before the property
        // lookup. glTF emits none of them.
        if parsed.object_name.is_some() {
            return None;
        }

        // TODO: `propertyIndex` — `.position[x]`, and
        // `.morphTargetInfluences[ <morph target name> ]`, which Three resolves
        // through `targetObject.morphTargetDictionary`.
        if parsed.property_index.is_some() {
            return None;
        }

        let property = match parsed.property_name.as_str() {
            "position" => NodeProperty::Position,
            "quaternion" => NodeProperty::Quaternion,
            "scale" => NodeProperty::Scale,
            "morphTargetInfluences" => NodeProperty::MorphTargetInfluences(
                self.morph_target_influences.get(&node.borrow().id)?.clone(),
            ),
            // `THREE.PropertyBinding: Trying to update property for track: …
            // but it wasn't found.` — TODO: `visible`, `material.*`, and the
            // generic number/array/fromArray cases.
            _ => return None,
        };

        Some(Box::new(NodeTarget::new(node, property)))
    }
}
