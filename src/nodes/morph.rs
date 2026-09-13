//! Port of `three.js/src/nodes/accessors/Morph.js` — `getEntry()`, which packs
//! a geometry's morph attributes into one `DataArrayTexture`, and
//! `morphReference()`, the vertex-stage blend loop `NodeMaterial.setupPosition()`
//! runs before everything else.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::core::BufferGeometry;
use crate::nodes::node::Type;
use crate::nodes::tsl::*;
use crate::nodes::NodeRef;
use crate::textures::DataArrayTexture;

/// `@TODO: Use 'capabilities.maxTextureSize'`.
const MAX_TEXTURE_SIZE: usize = 4096;

/// The `_morphTextures` entry: `{ count, texture, stride, size }`.
///
/// `Hash` — the texture by identity, the rest by value — is the entry's share
/// of the render object's cache key; `morphReference()` reads all four.
#[derive(Clone, Debug)]
pub struct MorphEntry {
    pub texture: DataArrayTexture,
    /// `vertexDataCount` — 1 for position-only morphing.
    pub stride: usize,
    pub width: usize,
    pub height: usize,
    pub count: usize,
}

impl std::hash::Hash for MorphEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture.id().hash(state);
        self.stride.hash(state);
        self.width.hash(state);
        self.height.hash(state);
        self.count.hash(state);
    }
}

thread_local! {
    /// `const _morphTextures = new WeakMap()` — keyed on the geometry, so the
    /// same geometry rendered twice reuses one texture.
    ///
    /// The key is `BufferGeometry.id`, not the geometry's address: an address
    /// is reused as soon as the geometry is dropped, and the next geometry at
    /// it would be handed this one's morph texture (issue #58). Three's
    /// `WeakMap` also *releases* the entry when the geometry goes, which the
    /// `Weak` beside each entry reproduces — swept on the next `get_entry()`,
    /// since a thread-local has no frame to hang a sweep off.
    static MORPH_TEXTURES: RefCell<HashMap<usize, (Weak<BufferGeometry>, MorphEntry)>> =
        RefCell::new(HashMap::new());
}

/// `getEntry( geometry )`. `None` when the geometry has no morph attributes.
pub fn get_entry(geometry: &Rc<BufferGeometry>) -> Option<MorphEntry> {
    let morph_targets = geometry.get_morph_attribute("position")?;
    let morph_targets_count = morph_targets.len();
    if morph_targets_count == 0 {
        return None;
    }

    let key = geometry.id();
    if let Some(entry) = MORPH_TEXTURES.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|_, (owner, _)| owner.strong_count() > 0);
        cache
            .get(&key)
            .map(|(_, entry)| entry)
            .filter(|entry| entry.count == morph_targets_count)
            .cloned()
    }) {
        return Some(entry);
    }

    // Only `morphAttributes.position` is on the ladder, so `vertexDataCount` is
    // 1 and every texel is one morph target's position delta.
    let vertex_data_count = 1;

    let mut width = geometry.get_attribute("position").unwrap().count() * vertex_data_count;
    let mut height = 1;

    if width > MAX_TEXTURE_SIZE {
        height = width.div_ceil(MAX_TEXTURE_SIZE);
        width = MAX_TEXTURE_SIZE;
    }

    let mut buffer = vec![0.0f32; width * height * 4 * morph_targets_count];

    let vertex_data_stride = vertex_data_count * 4;

    for (i, morph_target) in morph_targets.iter().enumerate() {
        let offset = width * height * 4 * i;

        for j in 0..morph_target.count() {
            let stride = j * vertex_data_stride;
            let vector = morph_target.get_vector3(j);

            buffer[offset + stride] = vector.x as f32;
            buffer[offset + stride + 1] = vector.y as f32;
            buffer[offset + stride + 2] = vector.z as f32;
            buffer[offset + stride + 3] = 0.0;
        }
    }

    let entry = MorphEntry {
        texture: DataArrayTexture::new(
            buffer,
            width as u32,
            height as u32,
            morph_targets_count as u32,
        ),
        stride: vertex_data_count,
        width,
        height,
        count: morph_targets_count,
    };

    MORPH_TEXTURES.with(|cache| {
        cache
            .borrow_mut()
            .insert(key, (Rc::downgrade(geometry), entry.clone()))
    });

    Some(entry)
}

/// `getMorph( { bufferMap, influence, stride, width, depth, offset } )` — an
/// unlayouted `Fn`, so it inlines at its one call site.
fn get_morph(entry: &MorphEntry, influence: NodeRef, depth: NodeRef) -> NodeRef {
    let texel_index = vertex_index()
        .to(Type::I32)
        .mul(int(entry.stride as i64))
        .add(int(0));

    let y = texel_index.div(int(entry.width as i64));
    let x = texel_index.sub(y.mul(int(entry.width as i64)));

    // Three's `ivec2( x, y )` lands in a `nodeVar` of its own: `TextureNode`
    // builds its uv twice during the analyze stage, so the join's usage count
    // passes `TempNode`'s threshold. Our builder counts one usage, so the var is
    // requested here instead — see `docs/nodes.md` §8.
    let coord = to_var(None, join(Type::IVec2, vec![x, y]));

    let buffer_attrib = texture_load_array(&entry.texture, coord, depth).xyz();

    buffer_attrib.mul(influence)
}

/// `morphReference( mesh )` — the statements it pushes into the vertex flow.
pub fn morph_reference(entry: &MorphEntry) -> Vec<NodeRef> {
    let mut statements = vec![position_local().mul_assign(morph_base())];

    let i = loop_index();
    let influence = to_var(None, float(0.0));

    let body = vec![
        influence.assign(to_var(
            None,
            morph_influences(entry.count, i.clone()).x(),
        )),
        if_statement(
            influence.not_equal(float(0.0)),
            vec![position_local().add_assign(get_morph(entry, influence.clone(), i.clone()))],
        ),
    ];

    statements.push(loop_statement(entry.count, i, body));

    statements
}
