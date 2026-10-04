//! Port of `three.js/src/helpers/SkeletonHelper.js`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::materials::LineBasicNodeMaterial;
use crate::math::{Color, Matrix4, Vector3};
use crate::objects::{is_bone, LineSegments};

/// `new SkeletonHelper( object )` — a `LineSegments` with one segment per
/// bone under `object` (itself included) whose parent is also a bone, from
/// the bone to its parent, drawn over everything (`depthTest: false`) with
/// vertex colours: blue at the bone, green at the parent.
///
/// The helper shares `object`'s world matrix (three's `this.matrix =
/// object.matrixWorld`; the port's
/// [`Object3D::matrix_alias`](crate::core::Object3D::matrix_alias)), and
/// its vertices are the bones' positions relative to `object`.
///
/// # The `updateMatrixWorld` override
///
/// Three rewrites the vertices in an `updateMatrixWorld()` override, which
/// the port's traversal cannot call, so it is
/// [`update_matrix_world`](Self::update_matrix_world) here. It reads the
/// bones' world matrices, so they must be current when it runs, and
/// [`Renderer::render`](crate::Renderer::render) updates the scene only
/// after anything you call: each frame, `root.update_matrix_world(false)` (or
/// [`Scene::update_matrix_world`](crate::objects::Scene::update_matrix_world)),
/// then `helper.update_matrix_world(false)`, then render. That gives three's
/// result for a helper that comes after `object` in the scene, which is where
/// three's traversal reads current bone matrices too.
///
/// `toneMapped: false` has no counterpart, as for
/// [`GridHelper`](super::GridHelper).
pub struct SkeletonHelper {
    /// The `LineSegments` itself.
    pub node: Node,
    /// `SkeletonHelper.root`, the object the helper was built for.
    pub root: Node,
    /// `SkeletonHelper.bones`: every bone under `root`, depth first.
    pub bones: Vec<Node>,
}

impl SkeletonHelper {
    /// `SkeletonHelper.isSkeletonHelper`.
    pub const IS_SKELETON_HELPER: bool = true;

    /// `new SkeletonHelper( object )`.
    pub fn new(object: &Node) -> Self {
        let bones = get_bone_list(object);

        let segments = bones.iter().filter(|bone| has_bone_parent(bone)).count();

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(vec![0.0; segments * 6], 3));
        geometry.set_attribute("color", BufferAttribute::new(vec![0.0; segments * 6], 3));

        // `new LineBasicMaterial( { vertexColors: true, depthTest: false,
        // depthWrite: false, toneMapped: false, transparent: true } )`.
        let mut material = LineBasicNodeMaterial::line(Color::new(1.0, 1.0, 1.0));
        material.vertex_colors = true;
        material.depth_test = false;
        material.depth_write = false;
        material.transparent = true;

        let node = LineSegments::new(Rc::new(geometry), material);
        {
            let mut helper = node.borrow_mut();
            helper.object_type = "SkeletonHelper";
            helper.matrix_alias = Some(object.downgrade());
            helper.matrix_auto_update = false;
        }

        let helper = Self {
            node,
            root: object.clone(),
            bones,
        };

        // colors

        helper.set_colors(Color::from_hex(0x0000ff), Color::from_hex(0x00ff00));

        helper
    }

    fn geometry(&self) -> Rc<BufferGeometry> {
        self.node
            .borrow()
            .geometry()
            .expect("three-rs: a SkeletonHelper is a LineSegments")
            .clone()
    }

    /// `SkeletonHelper.updateMatrixWorld( force )`: each segment's two ends
    /// from the bone's and its parent's world matrices, in the root's space,
    /// then `Object3D.updateMatrixWorld( force )`.
    pub fn update_matrix_world(&self, force: bool) {
        let geometry = self.geometry();
        let position = geometry
            .get_attribute("position")
            .expect("three-rs: the helper has a position attribute");

        let mut matrix_world_inv = self.root.borrow().matrix_world;
        matrix_world_inv.invert();

        let local = |matrix_world: &Matrix4| {
            let mut bone_matrix = Matrix4::default();
            bone_matrix.multiply_matrices(&matrix_world_inv, matrix_world);
            let mut vector = Vector3::default();
            vector.set_from_matrix_position(&bone_matrix);
            [vector.x as f32, vector.y as f32, vector.z as f32]
        };

        {
            let mut array = position.array_mut();
            let mut j = 0;
            for bone in &self.bones {
                let Some(parent) = bone.parent().filter(|parent| is_bone(&parent.borrow())) else {
                    continue;
                };

                // The pairs are re-derived from the bones' current parents, so
                // a bone re-parented under another bone since construction
                // asks for a segment the buffer was never sized for. Three's
                // `position.setXYZ( j, … )` past the end of the
                // `Float32Array` writes nothing; so does this. `j` only grows,
                // so every later pair is out of range too.
                let Some(pair) = array.get_mut(j * 3..j * 3 + 6) else {
                    break;
                };
                pair[..3].copy_from_slice(&local(&bone.borrow().matrix_world));
                pair[3..].copy_from_slice(&local(&parent.borrow().matrix_world));

                j += 2;
            }
        }

        position.set_needs_update();

        self.node.update_matrix_world(force);
    }

    /// `SkeletonHelper.setColors( color1, color2 )`: `color1` at each bone,
    /// `color2` at its parent.
    pub fn set_colors(&self, color1: Color, color2: Color) -> &Self {
        let geometry = self.geometry();
        let color_attribute = geometry
            .get_attribute("color")
            .expect("three-rs: the helper has a color attribute");
        {
            let mut array = color_attribute.array_mut();
            let rgb1 = [color1.r as f32, color1.g as f32, color1.b as f32];
            let rgb2 = [color2.r as f32, color2.g as f32, color2.b as f32];
            let (pairs, _) = array.as_chunks_mut::<6>();
            for pair in pairs {
                pair[..3].copy_from_slice(&rgb1);
                pair[3..].copy_from_slice(&rgb2);
            }
        }
        color_attribute.set_needs_update();

        self
    }
}

/// `bone.parent && bone.parent.isBone`.
fn has_bone_parent(bone: &Node) -> bool {
    bone.parent()
        .is_some_and(|parent| is_bone(&parent.borrow()))
}

/// The module's `getBoneList( object )`: `object` if it is a bone, then each
/// child's list in order.
fn get_bone_list(object: &Node) -> Vec<Node> {
    let mut bone_list = Vec::new();

    if is_bone(&object.borrow()) {
        bone_list.push(object.clone());
    }

    for child in object.children() {
        bone_list.extend(get_bone_list(&child));
    }

    bone_list
}
