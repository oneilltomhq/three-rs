//! Port of `three.js/src/core/BufferGeometry.js`: named
//! [`BufferAttribute`]s (typed, possibly interleaved — see
//! [`buffer_attribute`](super::buffer_attribute)) plus a `u16`/`u32` index.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use super::buffer_attribute::{AttributeDesc, BufferAttribute, InterleavedBuffer};
use crate::math::{Matrix3, Matrix4, Quaternion, Vector3};

/// `BufferGeometry.id` — three.js' module-level `let _id = 0` counter, handed
/// out in construction order, exactly as [`MaterialId`](crate::materials::MaterialId)
/// is for materials.
///
/// The renderer keys its uploaded GPU buffers on this. It has to be an
/// *identity*, and a never-reused one: the previous key was
/// `Rc::as_ptr( &geometry )`, and an address is reused the moment the geometry
/// behind it is dropped, so a new geometry allocated at a dead one's address
/// inherited its vertex buffers — a panic when the attribute sets differed and
/// the wrong shape, silently, when they did not (issue #58).
///
/// As with `MaterialId`, **`clone()` mints a fresh id**: a cloned geometry is a
/// new object in three.js (`new BufferGeometry().copy( this )`), and one that
/// may be mutated away from its source before it is ever drawn, so it must not
/// be served the source's upload.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeometryId(usize);

impl GeometryId {
    fn next() -> Self {
        thread_local! {
            static GEOMETRY_ID: Cell<usize> = const { Cell::new(0) };
        }
        GEOMETRY_ID.with(|id| {
            let next = id.get();
            id.set(next + 1);
            GeometryId(next)
        })
    }

    /// The number itself, for keying on.
    pub fn get(&self) -> usize {
        self.0
    }
}

/// A fresh id, never a copy — see the type's docs.
impl Clone for GeometryId {
    fn clone(&self) -> Self {
        Self::next()
    }
}

impl Default for GeometryId {
    fn default() -> Self {
        Self::next()
    }
}

/// `Box3`, as far as `BufferGeometry.computeBoundingBox()` needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    /// `Box3.min`.
    pub min: Vector3,
    /// `Box3.max`.
    pub max: Vector3,
}

impl BoundingBox {
    /// `Box3.makeEmpty()`.
    pub fn empty() -> Self {
        Self {
            min: Vector3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Vector3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }

    /// `Box3.setFromBufferAttribute()`.
    pub fn from_buffer_attribute(attribute: &BufferAttribute) -> Self {
        let mut box3 = Self::empty();

        for i in 0..attribute.count() {
            box3.expand_by_point(&attribute.get_vector3(i));
        }

        box3
    }

    /// `Box3.expandByPoint()`.
    pub fn expand_by_point(&mut self, point: &Vector3) -> &mut Self {
        self.min.min(point);
        self.max.max(point);
        self
    }

    /// `Box3.getCenter()`.
    pub fn center(&self) -> Vector3 {
        Vector3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }
}

/// `Sphere`, as far as `BufferGeometry.computeBoundingSphere()` needs it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingSphere {
    /// `Sphere.center`.
    pub center: Vector3,
    /// `Sphere.radius`.
    pub radius: f64,
}

/// `BufferGeometry.index`: the array type three.js picks for it, `Uint16Array`
/// or `Uint32Array`, kept distinct because the renderer uploads them at
/// different widths.
#[derive(Clone, Debug)]
pub enum Index {
    /// A `Uint16Array` index — used while every value fits in 16 bits.
    U16(Vec<u16>),
    /// A `Uint32Array` index.
    U32(Vec<u32>),
}

impl Index {
    /// `index.count` — the number of indices.
    pub fn count(&self) -> usize {
        match self {
            Index::U16(v) => v.len(),
            Index::U32(v) => v.len(),
        }
    }

    /// `index.getX( i )` — the `i`-th vertex index.
    pub fn get_x(&self, i: usize) -> usize {
        match self {
            Index::U16(v) => v[i] as usize,
            Index::U32(v) => v[i] as usize,
        }
    }

    /// `index.setX( i, x )`. The index carries no version of its own, so
    /// this is for a geometry the renderer has not uploaded yet.
    pub fn set_x(&mut self, i: usize, x: usize) -> &mut Self {
        match self {
            Index::U16(v) => v[i] = x as u16,
            Index::U32(v) => v[i] = x as u32,
        }
        self
    }
}

/// One entry of `BufferGeometry.groups`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    /// The first index (or vertex, for a non-indexed geometry) this group draws.
    pub start: usize,
    /// How many indices (or vertices) this group draws.
    pub count: usize,
    /// Which entry of the mesh's material array this group draws with.
    pub material_index: usize,
}

/// `BufferGeometry.drawRange`. `count: None` is three.js' `Infinity` ("draw
/// everything"), which has no `usize` spelling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawRange {
    /// `drawRange.start`.
    pub start: usize,
    /// `drawRange.count`; `None` is three.js' `Infinity`.
    pub count: Option<usize>,
}

/// What a cached pair of bounds was computed from: the `position` attribute's
/// id and version, every `position` morph target's id and version in order,
/// and `morph_targets_relative`. The ids are the numbers, never an
/// [`AttributeId`] clone, which would mint a fresh id and never match.
#[derive(Clone, Debug, PartialEq)]
struct BoundsKey {
    position: (usize, u32),
    morph_positions: Vec<(usize, u32)>,
    morph_targets_relative: bool,
}

impl BoundsKey {
    /// Whether this key still describes `geometry`'s inputs, without building
    /// a new key (that would allocate on every call, i.e. every frame).
    fn matches(&self, geometry: &BufferGeometry, position: &BufferAttribute) -> bool {
        if self.position != (position.id.get(), position.version())
            || self.morph_targets_relative != geometry.morph_targets_relative
        {
            return false;
        }
        let morph = geometry.get_morph_attribute("position").unwrap_or(&[]);
        morph.len() == self.morph_positions.len()
            && morph
                .iter()
                .zip(&self.morph_positions)
                .all(|(attribute, &key)| (attribute.id.get(), attribute.version()) == key)
    }

    fn of(geometry: &BufferGeometry, position: &BufferAttribute) -> Self {
        Self {
            position: (position.id.get(), position.version()),
            morph_positions: geometry
                .get_morph_attribute("position")
                .unwrap_or(&[])
                .iter()
                .map(|attribute| (attribute.id.get(), attribute.version()))
                .collect(),
            morph_targets_relative: geometry.morph_targets_relative,
        }
    }
}

/// The box and the sphere computed together from one [`BoundsKey`].
#[derive(Clone, Debug)]
struct CachedBounds {
    key: BoundsKey,
    bounding_box: BoundingBox,
    bounding_sphere: BoundingSphere,
}

/// Port of `BufferGeometry`'s state: the named attribute map, the index, morph
/// attributes, groups and the draw range.
///
/// `attributes` is a `Vec` of pairs rather than a `HashMap` because three.js'
/// `attributes` is a plain object, and `toNonIndexed()` and the renderer both
/// iterate it in insertion order.
#[derive(Debug, Default)]
pub struct BufferGeometry {
    /// `BufferGeometry.id`. Read-only in spirit; see [`GeometryId`] for why a
    /// clone gets a new one, and why the renderer keys on it.
    pub id: GeometryId,
    attributes: Vec<(String, BufferAttribute)>,
    /// `BufferGeometry.index`.
    pub index: Option<Index>,
    /// `BufferGeometry.morphAttributes` — per name, one attribute per morph
    /// target.
    morph_attributes: Vec<(String, Vec<BufferAttribute>)>,
    /// `BufferGeometry.morphTargetsRelative`.
    pub morph_targets_relative: bool,
    /// `BufferGeometry.groups`.
    pub groups: Vec<Group>,
    /// `BufferGeometry.drawRange`.
    pub draw_range: DrawRange,
    /// `BufferGeometry.boundingSphere` when it was computed from something
    /// other than the `position` attribute, which
    /// [`compute_bounding_sphere`](Self::compute_bounding_sphere) then returns
    /// instead of recomputing.
    ///
    /// three.js has this for free: `computeBoundingSphere()` writes
    /// `this.boundingSphere` and the renderer reads the *field*, so a subclass
    /// that overrides the method (`LineSegmentsGeometry`, which must measure
    /// `instanceStart` / `instanceEnd` rather than its fixed ±2 quad) is
    /// automatically what culling sees. The port's method has no field behind
    /// it, so the override lands here.
    pub bounding_sphere: Option<BoundingSphere>,
    /// `InstancedBufferGeometry.instanceCount`. `Some` makes this an
    /// `InstancedBufferGeometry`: `RenderObject.getInstanceCount()` reads it
    /// ahead of the object's own count, and its
    /// [instanced attributes](BufferAttribute::new_instanced) step per
    /// instance.
    pub instance_count: Option<usize>,
    /// `BufferGeometry.indirect` — set with
    /// [`set_indirect`](Self::set_indirect).
    indirect: Option<super::IndirectStorageBufferAttribute>,
    /// The last computed box and sphere and what they were computed from; see
    /// [`compute_bounding_sphere`](Self::compute_bounding_sphere). Behind a
    /// `RefCell` because the geometry is shared as `Rc<BufferGeometry>` and
    /// culling asks for its sphere through `&self` every frame (issue #133).
    bounds: RefCell<Option<CachedBounds>>,
}

/// `BufferGeometry.clone()` — `new this.constructor().copy( this )`, whose
/// `copy()` clones every attribute and morph attribute through one shared
/// `data` object, so views of one [`InterleavedBuffer`] still share a single
/// (cloned) buffer afterwards rather than each de-interleaving. A fresh
/// [`GeometryId`] and fresh attribute ids, as for any clone.
impl Clone for BufferGeometry {
    fn clone(&self) -> Self {
        let mut buffers: HashMap<usize, Rc<InterleavedBuffer>> = HashMap::new();
        let attributes = self
            .attributes
            .iter()
            .map(|(name, attribute)| (name.clone(), attribute.clone_with(&mut buffers)))
            .collect();
        let morph_attributes = self
            .morph_attributes
            .iter()
            .map(|(name, list)| {
                let list = list.iter().map(|a| a.clone_with(&mut buffers)).collect();
                (name.clone(), list)
            })
            .collect();
        Self {
            id: self.id.clone(),
            attributes,
            index: self.index.clone(),
            morph_attributes,
            morph_targets_relative: self.morph_targets_relative,
            groups: self.groups.clone(),
            draw_range: self.draw_range,
            bounding_sphere: self.bounding_sphere,
            instance_count: self.instance_count,
            indirect: self.indirect.clone(),
            bounds: self.bounds.clone(),
        }
    }
}

impl BufferGeometry {
    /// `new BufferGeometry()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `geometry.setIndirect( attribute )`: the draw's arguments come from
    /// this buffer on the GPU — `drawIndirect` for a non-indexed geometry,
    /// `drawIndexedIndirect` for an indexed one — so the vertex and instance
    /// counts a kernel wrote are the ones drawn, with no CPU round trip.
    pub fn set_indirect(&mut self, indirect: super::IndirectStorageBufferAttribute) -> &mut Self {
        self.indirect = Some(indirect);
        self
    }

    /// `geometry.indirect`.
    pub fn indirect(&self) -> Option<&super::IndirectStorageBufferAttribute> {
        self.indirect.as_ref()
    }

    /// `BufferGeometry.id` — the renderer's cache key for this geometry's
    /// uploaded buffers.
    pub fn id(&self) -> usize {
        self.id.get()
    }

    /// `BufferGeometry.getAttribute( name )`.
    pub fn get_attribute(&self, name: &str) -> Option<&BufferAttribute> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, attribute)| attribute)
    }

    /// `BufferGeometry.getAttribute( name )`, mutably.
    ///
    /// The `&mut` setters on [`BufferAttribute`] (`set_xyz` and the rest) do not
    /// bump its version, so handing one out drops the cached bounds.
    pub fn get_attribute_mut(&mut self, name: &str) -> Option<&mut BufferAttribute> {
        self.invalidate_bounds();
        self.attributes
            .iter_mut()
            .find(|(key, _)| key == name)
            .map(|(_, attribute)| attribute)
    }

    /// `BufferGeometry.setAttribute( name, attribute )`.
    pub fn set_attribute(&mut self, name: &str, attribute: BufferAttribute) -> &mut Self {
        self.invalidate_bounds();
        match self.attributes.iter_mut().find(|(key, _)| key == name) {
            // a JS object keeps the key's original position on reassignment
            Some(slot) => slot.1 = attribute,
            None => self.attributes.push((name.to_string(), attribute)),
        }

        self
    }

    /// `BufferGeometry.deleteAttribute( name )`.
    pub fn delete_attribute(&mut self, name: &str) -> &mut Self {
        self.invalidate_bounds();
        self.attributes.retain(|(key, _)| key != name);
        self
    }

    /// `BufferGeometry.hasAttribute( name )`.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.get_attribute(name).is_some()
    }

    /// `for ( const name in geometry.attributes )`, in insertion order.
    pub fn attributes(&self) -> impl Iterator<Item = (&str, &BufferAttribute)> {
        self.attributes
            .iter()
            .map(|(name, attribute)| (name.as_str(), attribute))
    }

    /// One [`AttributeDesc`] per attribute, in insertion order — what
    /// `SetupContext::geometry_attributes` carries to the node builder.
    /// Views of one [`InterleavedBuffer`] share a group number: the position
    /// of the first of them.
    pub fn attribute_descs(&self) -> Vec<AttributeDesc> {
        let mut groups: Vec<(usize, usize)> = Vec::new();
        self.attributes
            .iter()
            .enumerate()
            .map(|(i, (name, attribute))| {
                let group = match attribute.data_buffer() {
                    Some(data) => match groups.iter().find(|(id, _)| *id == data.id()) {
                        Some((_, group)) => *group,
                        None => {
                            groups.push((data.id(), i));
                            i
                        }
                    },
                    None => i,
                };
                attribute.desc(name, group)
            })
            .collect()
    }

    /// `geometry.attributes.position`.
    pub fn position(&self) -> Option<&BufferAttribute> {
        self.get_attribute("position")
    }

    /// `geometry.attributes.normal`.
    pub fn normal(&self) -> Option<&BufferAttribute> {
        self.get_attribute("normal")
    }

    /// `geometry.attributes.uv`.
    pub fn uv(&self) -> Option<&BufferAttribute> {
        self.get_attribute("uv")
    }

    /// `geometry.morphAttributes[ name ]`.
    pub fn get_morph_attribute(&self, name: &str) -> Option<&[BufferAttribute]> {
        self.morph_attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, attributes)| attributes.as_slice())
    }

    /// `geometry.morphAttributes[ name ] = attributes`.
    pub fn set_morph_attribute(
        &mut self,
        name: &str,
        attributes: Vec<BufferAttribute>,
    ) -> &mut Self {
        self.invalidate_bounds();
        match self
            .morph_attributes
            .iter_mut()
            .find(|(key, _)| key == name)
        {
            Some(slot) => slot.1 = attributes,
            None => self.morph_attributes.push((name.to_string(), attributes)),
        }

        self
    }

    /// `for ( const name in geometry.morphAttributes )`, in insertion order.
    pub fn morph_attributes(&self) -> impl Iterator<Item = (&str, &[BufferAttribute])> {
        self.morph_attributes
            .iter()
            .map(|(name, attributes)| (name.as_str(), attributes.as_slice()))
    }

    /// `BufferGeometry.addGroup( start, count, materialIndex )`.
    pub fn add_group(&mut self, start: usize, count: usize, material_index: usize) {
        self.groups.push(Group {
            start,
            count,
            material_index,
        });
    }

    /// `BufferGeometry.clearGroups()`.
    pub fn clear_groups(&mut self) {
        self.groups = Vec::new();
    }

    /// `BufferGeometry.setDrawRange( start, count )`.
    pub fn set_draw_range(&mut self, start: usize, count: usize) {
        self.draw_range.start = start;
        self.draw_range.count = Some(count);
    }

    /// `BufferGeometry.setFromPoints( points )`.
    ///
    /// three.js overwrites an existing `position` attribute in place when there
    /// already is one, and only allocates a fresh `Float32BufferAttribute` when
    /// there is not; the in-place branch keeps the attribute's own count, so
    /// extra points are dropped and missing ones leave whatever was there.
    pub fn set_from_points(&mut self, points: &[Vector3]) -> &mut Self {
        match self.get_attribute_mut("position") {
            Some(position) => {
                let count = position.count().min(points.len());
                for (i, point) in points.iter().take(count).enumerate() {
                    position.set_xyz(i, point.x, point.y, point.z);
                }
            }
            None => {
                let mut array = Vec::with_capacity(points.len() * 3);
                for point in points {
                    array.push(point.x as f32);
                    array.push(point.y as f32);
                    array.push(point.z as f32);
                }
                self.set_attribute("position", BufferAttribute::new(array, 3));
            }
        }

        self
    }

    /// `BufferGeometry.setIndex( array )` — picks `Uint16` when it fits, the
    /// same rule three.js uses.
    pub fn set_index(&mut self, indices: &[u32]) {
        let max = indices.iter().copied().max().unwrap_or(0);
        self.index = Some(if max > 65535 {
            Index::U32(indices.to_vec())
        } else {
            Index::U16(indices.iter().map(|&i| i as u16).collect())
        });
    }

    /// `BufferGeometry.setIndex( bufferAttribute )` — keeps the array type the
    /// source declared, which is what `BufferGeometryLoader.parse()` does.
    pub fn set_index_attribute(&mut self, index: Index) {
        self.index = Some(index);
    }

    /// Drops the cached bounds. The `&mut` methods that swap or hand out an
    /// attribute call it; an edit through `array_mut()` is caught by the
    /// version instead, once `set_needs_update()` bumps it.
    fn invalidate_bounds(&mut self) {
        *self.bounds.get_mut() = None;
    }

    /// The centre of `BufferGeometry.computeBoundingSphere()`'s sphere, which is
    /// `Box3.setFromBufferAttribute( position ).getCenter()` — the only part of
    /// the bounding sphere the render-list sort reads. Served from the same
    /// cache as [`compute_bounding_box`](Self::compute_bounding_box).
    pub(crate) fn bounding_sphere_center(&self) -> Vector3 {
        match self.compute_bounding_box() {
            Some(box3) => box3.center(),
            None => Vector3::ZERO,
        }
    }

    /// `BufferGeometry.computeBoundingBox()` — `Box3.setFromBufferAttribute(
    /// position )`. `None` when there is no position attribute (three.js leaves
    /// `boundingBox` alone in that case).
    ///
    /// Cached, together with the sphere; see
    /// [`compute_bounding_sphere`](Self::compute_bounding_sphere) for when the
    /// cache is recomputed.
    pub fn compute_bounding_box(&self) -> Option<BoundingBox> {
        self.cached_bounds().map(|(bounding_box, _)| bounding_box)
    }

    /// `BufferGeometry.computeBoundingSphere()`: the bounding box's centre, then
    /// the largest distance from it to any vertex (which beats the box's own
    /// sphere by up to sqrt(3)).
    ///
    /// The pub [`bounding_sphere`](Self::bounding_sphere) field, when set, is
    /// returned as it is, ahead of anything computed here.
    ///
    /// Otherwise the box and the sphere are computed once and cached, keyed on
    /// the `position` attribute's id and version, each `position` morph
    /// target's id and version, and `morph_targets_relative`. Culling calls
    /// this for every object every frame, so walking every vertex each time
    /// was most of a frame for a scene of many meshes (issue #133). The cache
    /// is dropped by the `&mut` methods that replace or hand out an attribute
    /// (`set_attribute`, `delete_attribute`, `set_morph_attribute`,
    /// `get_attribute_mut` and so everything built on it, `apply_matrix4` and
    /// `set_from_points` included). An edit through
    /// [`array_mut()`](BufferAttribute::array_mut) is seen only after
    /// [`set_needs_update()`](BufferAttribute::set_needs_update), the same rule
    /// the renderer's buffer upload follows. That is stricter than three.js,
    /// whose `boundingSphere` is never invalidated by itself: there a moved
    /// vertex needs `computeBoundingSphere()` called again by hand.
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        if let Some(sphere) = self.bounding_sphere {
            return Some(sphere);
        }
        self.cached_bounds()
            .map(|(_, bounding_sphere)| bounding_sphere)
    }

    /// The cached box and sphere, recomputed first if their key has moved.
    fn cached_bounds(&self) -> Option<(BoundingBox, BoundingSphere)> {
        let position = self.position()?;

        if let Some(cached) = self.bounds.borrow().as_ref() {
            if cached.key.matches(self, position) {
                return Some((cached.bounding_box, cached.bounding_sphere));
            }
        }

        let bounding_box = self.bounding_box_from_positions(position);
        let bounding_sphere = self.bounding_sphere_from_positions(position, &bounding_box);

        *self.bounds.borrow_mut() = Some(CachedBounds {
            key: BoundsKey::of(self, position),
            bounding_box,
            bounding_sphere,
        });

        Some((bounding_box, bounding_sphere))
    }

    /// `computeBoundingBox()`'s body, uncached.
    fn bounding_box_from_positions(&self, position: &BufferAttribute) -> BoundingBox {
        let mut bounding_box = BoundingBox::from_buffer_attribute(position);

        // process morph attributes if present
        if let Some(morph_positions) = self.get_morph_attribute("position") {
            for morph_attribute in morph_positions {
                let box3 = BoundingBox::from_buffer_attribute(morph_attribute);

                if self.morph_targets_relative {
                    let mut vector = Vector3::ZERO;
                    vector.add_vectors(&bounding_box.min, &box3.min);
                    bounding_box.expand_by_point(&vector);

                    vector.add_vectors(&bounding_box.max, &box3.max);
                    bounding_box.expand_by_point(&vector);
                } else {
                    bounding_box.expand_by_point(&box3.min);
                    bounding_box.expand_by_point(&box3.max);
                }
            }
        }

        bounding_box
    }

    /// `computeBoundingSphere()`'s body, uncached, around `bounding_box`'s
    /// centre.
    fn bounding_sphere_from_positions(
        &self,
        position: &BufferAttribute,
        bounding_box: &BoundingBox,
    ) -> BoundingSphere {
        // `_box` already has the morph targets expanded into it
        let center = bounding_box.center();

        let mut max_radius_sq: f64 = 0.0;

        for i in 0..position.count() {
            let v = position.get_vector3(i);
            max_radius_sq = max_radius_sq.max(center.distance_to_squared(&v));
        }

        // process morph attributes if present
        if let Some(morph_positions) = self.get_morph_attribute("position") {
            for morph_attribute in morph_positions {
                for j in 0..morph_attribute.count() {
                    let mut vector = morph_attribute.get_vector3(j);

                    if self.morph_targets_relative {
                        let offset = position.get_vector3(j);
                        vector.add(&offset);
                    }

                    max_radius_sq = max_radius_sq.max(center.distance_to_squared(&vector));
                }
            }
        }

        BoundingSphere {
            center,
            radius: max_radius_sq.sqrt(),
        }
    }

    /// `BufferGeometry.computeVertexNormals()`.
    ///
    /// The accumulation runs through the `normal` attribute itself, so every
    /// partial sum is rounded to `f32` before the next triangle adds to it.
    /// An existing `normal` of the right count is written in place through
    /// its setters, whatever its array kind and interleaved or not, and
    /// marked for re-upload; a missing one (or one of another count) is
    /// replaced by a new `Float32Array` attribute.
    pub fn compute_vertex_normals(&mut self) {
        let Some(count) = self.position().map(BufferAttribute::count) else {
            return;
        };

        let needs_new = match self.normal() {
            Some(normal) => normal.count() != count,
            None => true,
        };
        if needs_new {
            self.set_attribute("normal", BufferAttribute::new(vec![0.0; count * 3], 3));
        }

        let position = self.position().expect("three-rs: checked above");
        let normal = self.normal().expect("three-rs: set above if missing");
        let set_xyz = |i: usize, v: &Vector3| {
            normal.store(i, 0, v.x);
            normal.store(i, 1, v.y);
            normal.store(i, 2, v.z);
        };
        if !needs_new {
            for i in 0..normal.count() {
                set_xyz(i, &Vector3::ZERO);
            }
        }

        let mut cb = Vector3::ZERO;
        let mut ab = Vector3::ZERO;

        // indexed elements
        if let Some(index) = &self.index {
            let get = |i: usize| -> usize {
                match index {
                    Index::U16(v) => v[i] as usize,
                    Index::U32(v) => v[i] as usize,
                }
            };

            // three.js writes `i < index.count()`, which in JS reads past the
            // end of the typed array for a trailing partial triangle and stores
            // NaN; in Rust that panics, so incomplete triangles are skipped.
            let mut i = 0;
            while i + 2 < index.count() {
                let (v_a, v_b, v_c) = (get(i), get(i + 1), get(i + 2));

                let p_a = position.get_vector3(v_a);
                let p_b = position.get_vector3(v_b);
                let p_c = position.get_vector3(v_c);

                cb.sub_vectors(&p_c, &p_b);
                ab.sub_vectors(&p_a, &p_b);
                cb.cross(&ab);

                let mut n_a = normal.get_vector3(v_a);
                let mut n_b = normal.get_vector3(v_b);
                let mut n_c = normal.get_vector3(v_c);

                n_a.add(&cb);
                n_b.add(&cb);
                n_c.add(&cb);

                set_xyz(v_a, &n_a);
                set_xyz(v_b, &n_b);
                set_xyz(v_c, &n_c);

                i += 3;
            }
        } else {
            // As above: a position count that is not a multiple of three
            // leaves the trailing vertices' normals at zero rather than NaN.
            let mut i = 0;
            while i + 2 < position.count() {
                let p_a = position.get_vector3(i);
                let p_b = position.get_vector3(i + 1);
                let p_c = position.get_vector3(i + 2);

                cb.sub_vectors(&p_c, &p_b);
                ab.sub_vectors(&p_a, &p_b);
                cb.cross(&ab);

                set_xyz(i, &cb);
                set_xyz(i + 1, &cb);
                set_xyz(i + 2, &cb);

                i += 3;
            }
        }

        self.normalize_normals();
        if let Some(normal) = self.normal() {
            normal.set_needs_update();
        }
    }

    /// `BufferGeometry.normalizeNormals()`.
    pub fn normalize_normals(&mut self) {
        let Some(normal) = self.get_attribute_mut("normal") else {
            return;
        };

        for i in 0..normal.count() {
            let mut v = normal.get_vector3(i);
            v.normalize();
            normal.set_xyz(i, v.x, v.y, v.z);
        }
    }

    /// `BufferGeometry.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, matrix: &Matrix4) -> &mut Self {
        if let Some(position) = self.get_attribute_mut("position") {
            position.apply_matrix4(matrix);
        }

        if let Some(normal) = self.get_attribute_mut("normal") {
            let mut normal_matrix = Matrix3::identity();
            normal_matrix.get_normal_matrix(matrix);
            normal.apply_normal_matrix(&normal_matrix);
        }

        self
    }

    /// `BufferGeometry.scale()`.
    pub fn scale(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_scale(x, y, z);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.applyQuaternion()`.
    pub fn apply_quaternion(&mut self, q: &Quaternion) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_rotation_from_quaternion(q);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.rotateX()` — rotates the geometry about the world x-axis.
    pub fn rotate_x(&mut self, angle: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_rotation_x(angle);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.rotateY()`.
    pub fn rotate_y(&mut self, angle: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_rotation_y(angle);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.rotateZ()`.
    pub fn rotate_z(&mut self, angle: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_rotation_z(angle);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.translate()`.
    pub fn translate(&mut self, x: f64, y: f64, z: f64) -> &mut Self {
        let mut m1 = Matrix4::identity();
        m1.make_translation(x, y, z);
        self.apply_matrix4(&m1)
    }

    /// `BufferGeometry.lookAt()` — rotates the geometry so its +Z points away
    /// from `vector`, through a scratch `Object3D` exactly as three.js does.
    pub fn look_at(&mut self, vector: &Vector3) -> &mut Self {
        let mut object = crate::core::Object3D::default();
        object.look_at(vector);
        object.update_matrix();

        let matrix = object.matrix;
        self.apply_matrix4(&matrix)
    }

    /// `BufferGeometry.center()`.
    pub fn center(&mut self) -> &mut Self {
        let Some(bounding_box) = self.compute_bounding_box() else {
            return self;
        };

        let mut offset = bounding_box.center();
        offset.negate();

        self.translate(offset.x, offset.y, offset.z)
    }

    /// `BufferGeometry.toNonIndexed()`.
    pub fn to_non_indexed(&self) -> Self {
        let Some(index) = self.index.as_ref() else {
            // `BufferGeometry.toNonIndexed(): BufferGeometry is already
            // non-indexed.`
            return self.clone();
        };

        let indices: Vec<usize> = match index {
            Index::U16(v) => v.iter().map(|&i| i as usize).collect(),
            Index::U32(v) => v.iter().map(|&i| i as usize).collect(),
        };

        // `convertBufferAttribute()`: same kind, item size and `normalized`;
        // an interleaved view is read through its stride and comes out plain.
        let convert = |attribute: &BufferAttribute| attribute.gather(&indices);

        let mut geometry2 = Self::new();

        // attributes
        for (name, attribute) in self.attributes() {
            geometry2.set_attribute(name, convert(attribute));
        }

        // morph attributes
        let morph: Vec<(String, Vec<BufferAttribute>)> = self
            .morph_attributes()
            .map(|(name, attributes)| {
                (
                    name.to_string(),
                    attributes.iter().map(convert).collect::<Vec<_>>(),
                )
            })
            .collect();
        for (name, attributes) in morph {
            geometry2.set_morph_attribute(&name, attributes);
        }

        geometry2.morph_targets_relative = self.morph_targets_relative;

        // groups
        for group in &self.groups {
            geometry2.add_group(group.start, group.count, group.material_index);
        }

        geometry2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> BufferGeometry {
        let mut geometry = BufferGeometry::new();
        geometry.set_attribute(
            "position",
            BufferAttribute::new(vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0], 3),
        );
        geometry
    }

    #[test]
    fn bounds_are_the_same_on_a_second_call() {
        let geometry = triangle();
        let box1 = geometry.compute_bounding_box().unwrap();
        let sphere1 = geometry.compute_bounding_sphere().unwrap();
        assert_eq!(geometry.compute_bounding_box().unwrap(), box1);
        assert_eq!(geometry.compute_bounding_sphere().unwrap(), sphere1);
        assert_eq!(geometry.bounding_sphere_center(), box1.center());
        assert_eq!(sphere1.center, Vector3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn an_in_place_edit_shows_after_set_needs_update() {
        let geometry = triangle();
        let before = geometry.compute_bounding_sphere().unwrap();

        let position = geometry.position().unwrap();
        position.array_mut()[0] = -11.0;
        // not yet marked: the bounds, like the uploaded buffer, are stale
        assert_eq!(geometry.compute_bounding_sphere().unwrap(), before);

        position.set_needs_update();
        let after = geometry.compute_bounding_sphere().unwrap();
        assert_eq!(after.center, Vector3::new(-5.0, 1.0, 0.0));
        assert_eq!(geometry.bounding_sphere_center(), after.center);
        assert_eq!(
            geometry.compute_bounding_box().unwrap().min,
            Vector3::new(-11.0, 0.0, 0.0)
        );
    }

    #[test]
    fn set_attribute_recomputes() {
        let mut geometry = triangle();
        geometry.compute_bounding_sphere().unwrap();
        geometry.set_attribute(
            "position",
            BufferAttribute::new(vec![10.0, 0.0, 0.0, 12.0, 0.0, 0.0], 3),
        );
        let sphere = geometry.compute_bounding_sphere().unwrap();
        assert_eq!(sphere.center, Vector3::new(11.0, 0.0, 0.0));
        assert_eq!(sphere.radius, 1.0);
    }

    #[test]
    fn a_mutating_transform_recomputes() {
        let mut geometry = triangle();
        geometry.compute_bounding_sphere().unwrap();
        geometry.translate(0.0, 0.0, 5.0);
        assert_eq!(
            geometry.compute_bounding_sphere().unwrap().center,
            Vector3::new(0.0, 1.0, 5.0)
        );
        geometry.center();
        assert_eq!(geometry.bounding_sphere_center(), Vector3::ZERO);
    }

    #[test]
    fn a_clone_keeps_its_own_bounds() {
        let original = triangle();
        let original_sphere = original.compute_bounding_sphere().unwrap();

        // the clone carries the original's cache, keyed on the original's ids
        let clone = original.clone();
        assert_eq!(clone.compute_bounding_sphere().unwrap(), original_sphere);

        clone.position().unwrap().array_mut()[0] = -11.0;
        clone.position().unwrap().set_needs_update();
        assert_eq!(
            clone.compute_bounding_sphere().unwrap().center,
            Vector3::new(-5.0, 1.0, 0.0)
        );
        assert_eq!(original.compute_bounding_sphere().unwrap(), original_sphere);

        original.position().unwrap().array_mut()[3] = 21.0;
        original.position().unwrap().set_needs_update();
        assert_eq!(
            original.compute_bounding_sphere().unwrap().center,
            Vector3::new(10.0, 1.0, 0.0)
        );
        assert_eq!(
            clone.compute_bounding_sphere().unwrap().center,
            Vector3::new(-5.0, 1.0, 0.0)
        );
    }

    #[test]
    fn a_morph_position_edit_recomputes() {
        let mut geometry = triangle();
        geometry.set_morph_attribute(
            "position",
            vec![BufferAttribute::new(
                vec![-1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0],
                3,
            )],
        );
        let before = geometry.compute_bounding_box().unwrap();
        assert_eq!(before.max, Vector3::new(1.0, 2.0, 0.0));

        let morph = &geometry.get_morph_attribute("position").unwrap()[0];
        morph.array_mut()[3] = 9.0;
        morph.set_needs_update();
        assert_eq!(
            geometry.compute_bounding_box().unwrap().max,
            Vector3::new(9.0, 2.0, 0.0)
        );

        // relative targets are offsets from `position`: a key change too
        geometry.morph_targets_relative = true;
        assert_eq!(
            geometry.compute_bounding_box().unwrap().max,
            Vector3::new(10.0, 4.0, 0.0)
        );
    }

    #[test]
    fn the_override_field_still_wins() {
        let mut geometry = triangle();
        geometry.compute_bounding_sphere().unwrap();
        let sphere = BoundingSphere {
            center: Vector3::new(7.0, 7.0, 7.0),
            radius: 3.0,
        };
        geometry.bounding_sphere = Some(sphere);
        assert_eq!(geometry.compute_bounding_sphere(), Some(sphere));
    }
}
