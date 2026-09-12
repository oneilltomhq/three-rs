//! Port of `three.js/src/objects/Skeleton.js`.
//!
//! `boneTexture` is the renderer's business (Three allocates a `DataTexture`
//! from `boneMatrices` in `Skeleton.computeBoneTexture`), so it is left as the
//! [`Skeleton::bone_matrices`] hook plus [`Skeleton::bone_texture_size`]; the
//! renderer fills it in at rung 10.

use crate::core::{Node, Object3DNode};
use crate::math::Matrix4;

/// `class Skeleton`.
#[derive(Debug, Default)]
pub struct Skeleton {
    /// `Skeleton.bones`.
    pub bones: Vec<Node>,
    /// `Skeleton.boneInverses`.
    pub bone_inverses: Vec<Matrix4>,
    /// `Skeleton.boneMatrices` — `new Float32Array( bones.length * 16 )`,
    /// filled by [`Skeleton::update`].
    pub bone_matrices: Vec<f32>,
    /// `Skeleton.frame`, the renderer's "did the texture change" counter.
    pub frame: i64,
}

impl Skeleton {
    /// `new Skeleton( bones, boneInverses )`. `None` for the inverses is
    /// three.js' `boneInverses === undefined` branch, which calls
    /// `calculateInverses()`.
    pub fn new(bones: Vec<Node>, bone_inverses: Option<Vec<Matrix4>>) -> Self {
        let mut skeleton = Self {
            bones,
            bone_inverses: Vec::new(),
            bone_matrices: Vec::new(),
            frame: -1,
        };

        skeleton.init(bone_inverses);
        skeleton
    }

    /// `Skeleton.init()`.
    pub fn init(&mut self, bone_inverses: Option<Vec<Matrix4>>) {
        self.bone_matrices = vec![0.0; self.bones.len() * 16];

        match bone_inverses {
            None => self.calculate_inverses(),
            Some(inverses) => {
                if inverses.len() == self.bones.len() {
                    self.bone_inverses = inverses;
                } else {
                    // `THREE.Skeleton: Number of inverse bone matrices does not
                    // match amount of bones.`
                    self.bone_inverses = vec![Matrix4::identity(); self.bones.len()];
                }
            }
        }
    }

    /// `Skeleton.calculateInverses()`.
    pub fn calculate_inverses(&mut self) {
        self.bone_inverses.clear();

        for bone in &self.bones {
            let mut inverse = bone.borrow().matrix_world;
            inverse.invert();
            self.bone_inverses.push(inverse);
        }
    }

    /// `Skeleton.pose()` — put the skeleton back into its bind pose.
    pub fn pose(&mut self) {
        // recover the bind-time world matrices
        for (bone, inverse) in self.bones.iter().zip(self.bone_inverses.iter()) {
            let mut world = *inverse;
            world.invert();
            bone.borrow_mut().matrix_world = world;
        }

        // compute the local matrices, positions, rotations and scales
        for bone in &self.bones {
            let parent_world = bone.parent().map(|parent| parent.borrow().matrix_world);

            let matrix = match parent_world {
                Some(parent_world) => {
                    let mut inverse = parent_world;
                    inverse.invert();
                    let mut matrix = Matrix4::identity();
                    let world = bone.borrow().matrix_world;
                    matrix.multiply_matrices(&inverse, &world);
                    matrix
                }
                None => bone.borrow().matrix_world,
            };

            let mut object = bone.borrow_mut();
            object.matrix.copy(&matrix);
            let matrix = object.matrix;
            let (mut position, mut quaternion, mut scale) =
                (object.position, object.quaternion, object.scale);
            matrix.decompose(&mut position, &mut quaternion, &mut scale);
            object.position = position;
            object.quaternion = quaternion;
            object.scale = scale;
            object.sync_rotation_from_quaternion();
        }
    }

    /// `Skeleton.update()` — flatten `boneMatrix = boneWorld * boneInverse`
    /// into `boneMatrices`.
    pub fn update(&mut self) {
        let mut offset = 0;

        for (bone, inverse) in self.bones.iter().zip(self.bone_inverses.iter()) {
            // `// compute the offset between the current and the original
            // transformation`
            let world = bone.borrow().matrix_world;
            let mut matrix = Matrix4::identity();
            matrix.multiply_matrices(&world, inverse);

            for (i, value) in matrix.elements.iter().enumerate() {
                self.bone_matrices[offset + i] = *value as f32;
            }

            offset += 16;
        }
    }

    /// `Skeleton.getBoneByName( name )`.
    pub fn get_bone_by_name(&self, name: &str) -> Option<Node> {
        self.bones
            .iter()
            .find(|bone| bone.borrow().name == name)
            .cloned()
    }

    /// `Skeleton.computeBoneTexture()`'s size computation: the square power-of-two
    /// RGBA texture that holds `bones.length * 4` pixels. The texture itself is
    /// the renderer's; this is the hook it needs.
    pub fn bone_texture_size(&self) -> usize {
        let mut size = (self.bones.len() as f64 * 4.0).sqrt().ceil();
        size = (size / 4.0).ceil() * 4.0;
        size.max(4.0) as usize
    }
}
