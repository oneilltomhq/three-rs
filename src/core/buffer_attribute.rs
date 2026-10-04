//! `BufferAttribute`, `InterleavedBuffer` and `InterleavedBufferAttribute` —
//! `src/core/BufferAttribute.js`, `InterleavedBuffer.js`,
//! `InterleavedBufferAttribute.js` (three r187), plus the attribute half of
//! `StorageBufferAttribute.js`.
//!
//! # Design (issue #294)
//!
//! Until this module the port held every attribute as `Vec<f32>` with one
//! `integer` flag for `skinIndex`, uploaded as `Float32xN` or `Uint32x4`, and
//! kept the only interleaved data (Line2's `InstancedInterleavedBuffer`) as a
//! node-side special case. The decisions below replace that; each is the shape
//! the port takes and why, so the porter and the next reader do not re-derive
//! them.
//!
//! ## 1. One struct, a typed value enum inside — not a generic
//!
//! [`BufferAttribute`] stays a single type. Its storage is a [`TypedArray`]
//! enum with one variant per JavaScript array three uses on an attribute:
//! `F32(Vec<f32>)`, `F16(Vec<u16>)` (IEEE binary16 bits, converted with
//! [`crate::extras::from_half_float`] / [`crate::extras::to_half_float`],
//! three's `DataUtils`), `I8`, `U8`, `U8Clamped`, `I16`, `U16`, `I32`, `U32`.
//! `normalized` is a field beside it, as in three.
//!
//! Why not `BufferAttribute<T: Element>`: `BufferGeometry` stores attributes
//! heterogeneously and `get_attribute(name)` returns one by name, so the store
//! needs type erasure anyway, and an erased wrapper over a generic is the enum
//! with a second type surface on top. The enum also keeps `set_attribute`,
//! `get_attribute` and the 120-odd `BufferAttribute::new(Vec<f32>, n)` call
//! sites compiling unchanged: `new` is `Float32BufferAttribute`, three's
//! default. The typed constructors mirror three's class names as functions:
//! `BufferAttribute::uint8(array, item_size, normalized)` is
//! `new Uint8BufferAttribute( array, itemSize, normalized )`, and so on for
//! `int8`, `uint8_clamped`, `int16`, `uint16`, `int32`, `uint32`, `float16`;
//! `BufferAttribute::from_typed(TypedArray, item_size, normalized)` is the
//! base constructor.
//!
//! ## 2. Reading: `array()` is the `Float32Array` view, `data()` the typed one
//!
//! `array()` / `array_mut()` keep returning `Ref<Vec<f32>>` / `RefMut<Vec<f32>>`
//! and are the fast path every geometry algorithm uses. On an attribute whose
//! storage is not `F32` they **panic** naming `data()`, the same way code that
//! assumes `attribute.array` is a `Float32Array` breaks in three when handed a
//! `Uint8Array`. `data()` / `data_mut()` return the [`TypedArray`] itself.
//! `get_x..get_w` and `set_x..set_xyzw` work on every variant: a read widens to
//! `f64` and, when `normalized`, applies `MathUtils.denormalize` for the
//! variant's range; a write applies `MathUtils.normalize` then narrows, with
//! `U8Clamped` clamping instead of wrapping. `count()` is `len / item_size` in
//! elements, whatever the element width.
//!
//! The `integer` flag and `new_integer` go: `skinIndex` becomes
//! `BufferAttribute::uint32` (a glTF `JOINTS_0` is `Uint8`/`Uint16`
//! non-normalized, which three widens to `Uint32` on upload — decision 3 — so
//! the GPU sees the same bytes either way).
//!
//! ## 3. `normalized` → `wgpu::VertexFormat`: three's table, including its padding
//!
//! The format is `WebGPUAttributeUtils._getVertexFormat()` after
//! `createAttribute()` has had its way with the array, in this order:
//!
//! * `F32` → `Float32xN`; `I32`/`U32` → `Sint32xN`/`Uint32xN`, `N` 1..4. At
//!   `itemSize === 1` three's map ignores `normalized`, so a normalized
//!   `Int32Array`/`Uint32Array` of item size 1 is `sint32`/`uint32` while
//!   `getTypeFromAttribute()` declares it `f32` (decision 4) — a pairing
//!   WebGPU rejects at pipeline creation. The port reproduces both halves.
//! * `F16` → `Float16xN` with `N` padded to 2 or 4 (a 16-bit stride must be a
//!   multiple of 4 bytes), and never widened. This is a deliberate
//!   divergence: three stores a `Float16BufferAttribute` in a `Uint16Array`,
//!   so a non-normalized, non-interleaved one goes through the widening below
//!   into a `Uint32Array`, and `_getVertexFormat` then pairs the `float16`
//!   prefix (picked by attribute class) with a 4-byte element and asks for
//!   `float16x3` for item size 3 — not a WebGPU format. The port keeps the
//!   binary16 bits and pads, which is what the class asks for.
//!   `itemSize === 1` has no `Float16` entry in three's item-size-1 map and
//!   errors.
//! * Every row three has no format for — `F16` at item size 1, a normalized
//!   32-bit array above item size 1, the 8- and 16-bit item-size-1 rows
//!   below — panics in [`ArrayKind::vertex_format`] with three's message.
//!   That is a deliberate divergence too: three only logs `error(
//!   'WebGPUAttributeUtils: Vertex format not supported yet.' )` and returns
//!   `undefined`, and the draw then fails as a WebGPU validation error on the
//!   pipeline, away from the attribute that caused it. Failing at the table
//!   names the array kind, item size and flags.
//! * 8- and 16-bit **non-normalized, non-interleaved**: `createAttribute()`
//!   rebuilds the array as `Int32Array`/`Uint32Array` and assigns it back, so
//!   `_getVertexFormat` then sees a 32-bit array → `Sint32xN`/`Uint32xN`. The
//!   port widens at upload only; the CPU array keeps its type (three's
//!   `bufferAttribute.array = array` side effect is not reproduced, and
//!   `array()`/`get_x()` stay consistent with what the caller built).
//! * 8- and 16-bit **normalized**: `Snorm8xN`/`Unorm8xN`/`Snorm16xN`/`Unorm16xN`
//!   with `N` padded so the byte stride is a multiple of 4 — `N = 3` becomes 4
//!   for both widths, `N = 1` is an error for 8-bit (no item-size-1 entry) and
//!   for 16-bit. The padding is applied to the uploaded bytes too, element by
//!   element, exactly as `createAttribute()`'s `paddedArray` loop does, and to
//!   the `arrayStride` (`createShaderVertexBuffers()` rounds it up).
//! * 8- and 16-bit **interleaved** (`isInterleavedBufferAttribute`): no
//!   widening; the format comes straight from the array type — `Uint8xN`,
//!   `Sint16xN`, their normalized forms — `N` padded as above; `arrayStride` is
//!   `data.stride * bytes_per_element`, the attribute's `offset` is
//!   `offset * bytes_per_element`.
//!
//! The table lives in one function, [`TypedArray::vertex_format`] (array kind,
//! item size, normalized, interleaved) → `(wgpu::VertexFormat, padded_item_size)`,
//! and is the thing the unit test checks row by row.
//!
//! ## 4. The shader type comes from the attribute, not from the node
//!
//! `NodeBuilder.getTypeFromAttribute()`: `normalized` or `F16` → `float`/`vecN`;
//! otherwise `I*` → `int`/`ivecN`, `U*` → `uint`/`uvecN`, `F32` → `float`/`vecN`.
//! `AttributeNode.generate()` declares the slot in **that** type and casts to
//! the node's own type with `builder.format()`. Today the port's
//! `Node::Attribute { name, ty }` declares the slot in `ty`. To do what three
//! does the builder has to know the geometry's attributes, so
//! `SetupContext` grows `geometry_attributes: Vec<AttributeDesc>` — one entry
//! per `geometry.attributes` in insertion order with `name`, element kind,
//! `item_size`, `normalized`, step mode, and for an interleaved attribute a
//! geometry-local group index (the position of the first view of the same
//! buffer, so two geometries laid out alike share a program), stride and
//! offset. It is hashed into the program cache key
//! (as `instanced_attributes` is now, which it replaces: "instanced" is a bit
//! on the descriptor). `vertex_color_size` and `has_tangent_attribute` could
//! be derived from it; leave them alone in this PR, the diff is wide enough.
//!
//! `NodeProgram::vertex_buffers()` builds one `VertexBufferDesc` per geometry
//! attribute buffer: non-interleaved attributes one buffer each, interleaved
//! attributes sharing an `InterleavedBuffer` one buffer with several entries —
//! the grouping it already does for `InstanceBuffer`, keyed on that group
//! index (the renderer keys the GPU buffer itself on the interleaved buffer's
//! id, decision 5). `programs::vertex_format(Type)`
//! goes; the format rides on the slot from the descriptor.
//!
//! ## 5. An interleaved buffer is one GPU buffer; its attributes are views
//!
//! [`InterleavedBuffer`] owns `RefCell<TypedArray>`, `stride`, a `version`
//! (`needsUpdate`) and an id from the attribute counter; `new_instanced(array,
//! stride, mesh_per_attribute)` is `InstancedInterleavedBuffer`. It is shared
//! as `Rc<InterleavedBuffer>`. A [`BufferAttribute`] is either its own
//! storage or a view: `BufferAttribute::interleaved(Rc<InterleavedBuffer>,
//! item_size, offset, normalized)` is `new InterleavedBufferAttribute( data,
//! itemSize, offset, normalized )`; `is_interleaved()` and `data_buffer()`
//! expose the view. The getters/setters index `offset + index * stride`.
//! `array()` on an interleaved `F32` view returns the **whole** shared array,
//! as three's `InterleavedBufferAttribute.array` getter does.
//!
//! The renderer keys the GPU buffer on the `InterleavedBuffer`'s id, three's
//! `backend.get( attribute.data )`, so two views upload once and share a
//! vertex-buffer slot, and `InterleavedBuffer::set_needs_update` refreshes it.
//! `GeometryGpu` drops the position/normal/uv/other split and the three-entry
//! version array for one list of `(buffer key, wgpu::Buffer, version)` where
//! the key is an attribute id or an interleaved buffer id — which also makes
//! every attribute refreshable on `set_needs_update`, not just the first
//! three (a limit `refresh_geometry` documents today).
//!
//! `LineSegmentsGeometry` is the first consumer: `set_positions` builds an
//! `InterleavedBuffer::new_instanced(array, 6, 1)` and sets `instanceStart`
//! and `instanceEnd` as interleaved views at offsets 0 and 3 on the geometry,
//! as `LineSegmentsGeometry.js` does; `set_colors` likewise builds
//! `instanceColorStart` / `instanceColorEnd`. The dash distances
//! (`computeLineDistances()`'s `instanceDistanceStart` / `instanceDistanceEnd`)
//! are not ported, with the rest of `_useDash`.
//! `Line2NodeMaterial` then reads them with plain `attribute("instanceStart")`
//! and `SetupContext::line_segments`, `nodes::lines::LineSegmentsAttributes`
//! and the Line2 use of `Node::InstancedAttribute` go. `instance_count` on the
//! geometry is the interleaved buffer's `count` — `array.len() / stride` — set
//! by the geometry as three does. The WGSL must stay byte-identical to the
//! `webgpu_lines_fat` fixtures: same one `stepMode: 'instance'` buffer of
//! stride 24, two entries at offsets 0 and 12. `InstancedMesh`'s
//! `instanceMatrix` (`InstancedInterleavedBuffer( array, 16, 1 )` in three) is
//! the natural second consumer and stays on `InstanceBuffer` in this PR.
//!
//! ## 6. `StorageBufferAttribute` names a `StorageArray`'s buffer
//!
//! Node-side `StorageArray` (`instancedArray` / `attributeArray`) already owns
//! a `BufferId` the renderer keys the GPU buffer on. [`StorageBufferAttribute`]
//! is the attribute-side handle to the same id: `StorageBufferAttribute::new(
//! count_or_array, item_size, TypedArray kind )` mints a `BufferId` and holds
//! the initial contents; `storage(&attribute, ty, count)` builds a
//! `StorageArray` over **that** id (`BufferSource::StorageData` with the
//! attribute's bytes as `init`), and `attribute.to_attribute()` /
//! `geometry.set_attribute(name, attribute.into())` reads it as a vertex
//! buffer with `stepMode: 'vertex'`, or `'instance'` for
//! `StorageInstancedBufferAttribute` (`new_instanced`). A kernel writing the
//! storage view and a draw reading the attribute view meet on one buffer, as
//! `IndirectStorageBufferAttribute` already arranges for indirect args. The
//! renderer looks a `BufferAttribute` whose backing is `Storage(id)` up in its
//! storage-buffer map rather than uploading it. `createAttribute()` treats
//! the storage array as any other first — a non-normalized 8- or 16-bit
//! integer array is widened to 32 bits — and then pads `itemSize === 3` to a
//! `vec4` stride; the attribute view of the buffer uses that widened, padded
//! stride, and a `storage()` node over it reads the same 32-bit words.
//!
//! ## Not in this PR
//!
//! TSL `bufferAttribute()` / `dynamicBufferAttribute()` (a `BufferAttributeNode`
//! over an attribute not on the geometry), `usage`/`updateRanges`,
//! `onUpload`, `toJSON`, and `InstancedMesh` on an interleaved buffer. Each is a
//! follow-on with this module as its base.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use crate::extras::{from_half_float, to_half_float};
use crate::math::{Matrix3, Matrix4, Vector3};
use crate::nodes::node::BufferId;
use crate::nodes::Type;

/// `BufferAttribute.id` — three.js' `_id ++` on the attribute class. The same
/// never-reused counter shape as [`GeometryId`](super::GeometryId), for the
/// same reason.
///
/// The renderer keys a geometry's vertex buffers on this (or, for an
/// interleaved attribute, on its [`InterleavedBuffer::id`], which comes from
/// the same counter so the two can never collide).
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AttributeId(usize);

impl AttributeId {
    pub(crate) fn next() -> Self {
        thread_local! {
            static ATTRIBUTE_ID: Cell<usize> = const { Cell::new(0) };
        }
        ATTRIBUTE_ID.with(|id| {
            let next = id.get();
            id.set(next + 1);
            AttributeId(next)
        })
    }

    /// The number itself, for keying on.
    pub fn get(&self) -> usize {
        self.0
    }
}

/// A fresh id, never a copy — see [`GeometryId`](super::GeometryId).
impl Clone for AttributeId {
    fn clone(&self) -> Self {
        Self::next()
    }
}

impl Default for AttributeId {
    fn default() -> Self {
        Self::next()
    }
}

/// The element type of a [`TypedArray`] — the JavaScript array constructor
/// (`Float32Array`, `Uint8Array`, …) without the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrayKind {
    /// `Float32Array`.
    F32,
    /// `Float16BufferAttribute`'s `Uint16Array` of binary16 bits.
    F16,
    /// `Int8Array`.
    I8,
    /// `Uint8Array`.
    U8,
    /// `Uint8ClampedArray`.
    U8Clamped,
    /// `Int16Array`.
    I16,
    /// `Uint16Array`.
    U16,
    /// `Int32Array`.
    I32,
    /// `Uint32Array`.
    U32,
}

impl ArrayKind {
    /// `TypedArray.BYTES_PER_ELEMENT`.
    pub fn bytes_per_element(self) -> usize {
        match self {
            ArrayKind::I8 | ArrayKind::U8 | ArrayKind::U8Clamped => 1,
            ArrayKind::F16 | ArrayKind::I16 | ArrayKind::U16 => 2,
            ArrayKind::F32 | ArrayKind::I32 | ArrayKind::U32 => 4,
        }
    }

    /// The array `WebGPUAttributeUtils.createAttribute()` uploads: 8- and
    /// 16-bit integers that are neither normalized nor interleaved are rebuilt
    /// as `Int32Array`/`Uint32Array` ("patch for INT16 and UINT16"). `F16` is
    /// not widened — see decision 3 in the module docs — nor is `U8Clamped`,
    /// which three's patch does not list.
    pub(crate) fn upload_kind(self, normalized: bool, interleaved: bool) -> ArrayKind {
        if normalized || interleaved {
            return self;
        }
        match self {
            ArrayKind::I8 | ArrayKind::I16 => ArrayKind::I32,
            ArrayKind::U8 | ArrayKind::U16 => ArrayKind::U32,
            kind => kind,
        }
    }

    /// The table of decision 3: `WebGPUAttributeUtils._getVertexFormat()`
    /// applied to the array `createAttribute()` uploads, returning the format
    /// and the padded item size (the components per element in the uploaded
    /// bytes, `item_size` rounded up so the byte stride is a multiple of 4).
    ///
    /// # Panics
    ///
    /// On the rows three has no format for, with three's message: item size 1
    /// of an 8- or 16-bit array that is normalized or interleaved (or of
    /// `F16`), a normalized `F32`/`I32`/`U32`/`F16` with item size above 1,
    /// and any `U8Clamped` that reaches the table unwidened — which is every
    /// one, since `Uint8ClampedArray` is in neither prefix map. Three logs
    /// and returns `undefined` here instead, leaving the failure to pipeline
    /// validation; panicking is a deliberate divergence (decision 3 in the
    /// module docs).
    pub fn vertex_format(
        self,
        item_size: usize,
        normalized: bool,
        interleaved: bool,
    ) -> (wgpu::VertexFormat, usize) {
        use wgpu::VertexFormat as F;

        let unsupported = || -> ! {
            panic!(
                "THREE.WebGPUAttributeUtils: Vertex format not supported yet. \
                 ({self:?}, itemSize {item_size}, normalized {normalized}, interleaved {interleaved})"
            )
        };
        assert!(
            (1..=4).contains(&item_size),
            "three-rs: a vertex attribute has 1 to 4 components, not {item_size}"
        );

        let kind = self.upload_kind(normalized, interleaved);
        if item_size == 1 {
            let format = match kind {
                ArrayKind::I32 => F::Sint32,
                ArrayKind::U32 => F::Uint32,
                ArrayKind::F32 => F::Float32,
                _ => unsupported(),
            };
            return (format, 1);
        }

        let bpe = kind.bytes_per_element();
        let padded = (bpe * item_size).div_ceil(4) * 4 / bpe;
        let format = match (kind, normalized, padded) {
            (ArrayKind::F32, false, 2) => F::Float32x2,
            (ArrayKind::F32, false, 3) => F::Float32x3,
            (ArrayKind::F32, false, 4) => F::Float32x4,
            (ArrayKind::I32, false, 2) => F::Sint32x2,
            (ArrayKind::I32, false, 3) => F::Sint32x3,
            (ArrayKind::I32, false, 4) => F::Sint32x4,
            (ArrayKind::U32, false, 2) => F::Uint32x2,
            (ArrayKind::U32, false, 3) => F::Uint32x3,
            (ArrayKind::U32, false, 4) => F::Uint32x4,
            (ArrayKind::F16, false, 2) => F::Float16x2,
            (ArrayKind::F16, false, 4) => F::Float16x4,
            (ArrayKind::I16, false, 2) => F::Sint16x2,
            (ArrayKind::I16, false, 4) => F::Sint16x4,
            (ArrayKind::I16, true, 2) => F::Snorm16x2,
            (ArrayKind::I16, true, 4) => F::Snorm16x4,
            (ArrayKind::U16, false, 2) => F::Uint16x2,
            (ArrayKind::U16, false, 4) => F::Uint16x4,
            (ArrayKind::U16, true, 2) => F::Unorm16x2,
            (ArrayKind::U16, true, 4) => F::Unorm16x4,
            (ArrayKind::I8, false, 4) => F::Sint8x4,
            (ArrayKind::I8, true, 4) => F::Snorm8x4,
            (ArrayKind::U8, false, 4) => F::Uint8x4,
            (ArrayKind::U8, true, 4) => F::Unorm8x4,
            _ => unsupported(),
        };
        (format, padded)
    }

    /// `NodeBuilder.getTypeFromAttribute()`: the type the vertex input is
    /// declared in. `normalized` or `F16` reads as floats; otherwise signed
    /// integers as `i32`, unsigned as `u32`, `F32` (and `U8Clamped`, which
    /// three's `typeFromArray` map lacks, so its `'float'` default applies) as
    /// `f32`. The vector width is the attribute's own item size, unpadded.
    pub fn shader_type(self, item_size: usize, normalized: bool) -> Type {
        let component = if normalized {
            Type::F32
        } else {
            match self {
                ArrayKind::I8 | ArrayKind::I16 | ArrayKind::I32 => Type::I32,
                ArrayKind::U8 | ArrayKind::U16 | ArrayKind::U32 => Type::U32,
                ArrayKind::F32 | ArrayKind::F16 | ArrayKind::U8Clamped => Type::F32,
            }
        };
        if item_size == 1 {
            component
        } else {
            Type::vector_of(component, item_size)
        }
    }

    /// `MathUtils.denormalize( value, array )`. `F16` is three's
    /// `Uint16Array` and gets its range.
    fn denormalize(self, value: f64) -> f64 {
        match self {
            ArrayKind::F32 => value,
            ArrayKind::U32 => value / 4294967295.0,
            ArrayKind::U16 | ArrayKind::F16 => value / 65535.0,
            ArrayKind::U8 | ArrayKind::U8Clamped => value / 255.0,
            ArrayKind::I32 => (value / 2147483647.0).max(-1.0),
            ArrayKind::I16 => (value / 32767.0).max(-1.0),
            ArrayKind::I8 => (value / 127.0).max(-1.0),
        }
    }

    /// `MathUtils.normalize( value, array )` — `Math.round( value * max )`.
    fn normalize(self, value: f64) -> f64 {
        let scale = match self {
            ArrayKind::F32 => return value,
            ArrayKind::U32 => 4294967295.0,
            ArrayKind::U16 | ArrayKind::F16 => 65535.0,
            ArrayKind::U8 | ArrayKind::U8Clamped => 255.0,
            ArrayKind::I32 => 2147483647.0,
            ArrayKind::I16 => 32767.0,
            ArrayKind::I8 => 127.0,
        };
        js_round(value * scale)
    }
}

/// `createAttribute()`'s `paddedItemSize` for an upload whose elements are
/// `kind` (already widened): a storage attribute of item size 3 is padded to
/// 4 ("WGSL does not support packed vec3 data in storage buffers"); otherwise
/// an item of more than one element whose byte stride is not a multiple of 4
/// is padded up to one. `item_size` when no padding applies.
fn padded_item_size(kind: ArrayKind, item_size: usize, storage: bool) -> usize {
    if storage && item_size == 3 {
        return 4;
    }
    let bpe = kind.bytes_per_element();
    if item_size > 1 && !(item_size * bpe).is_multiple_of(4) {
        (item_size * bpe).div_ceil(4) * 4 / bpe
    } else {
        item_size
    }
}

/// `Math.round` — halves round towards +∞.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// The ECMAScript `ToInt8`/`ToUint16`/… conversions a typed-array store does:
/// non-finite is 0, otherwise truncate and wrap modulo `2^bits`.
fn js_wrap(value: f64, bits: i32) -> u64 {
    if !value.is_finite() {
        return 0;
    }
    value.trunc().rem_euclid(2f64.powi(bits)) as u64
}

/// The data of a [`BufferAttribute`] or [`InterleavedBuffer`]: one variant per
/// JavaScript typed array three puts behind an attribute.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedArray {
    /// `Float32Array` — what [`BufferAttribute::new`] makes.
    F32(Vec<f32>),
    /// `Float16BufferAttribute`'s array: IEEE binary16 **bits** in a
    /// `Uint16Array`, read and written through `fromHalfFloat` /
    /// `toHalfFloat` by the attribute's accessors.
    F16(Vec<u16>),
    /// `Int8Array`.
    I8(Vec<i8>),
    /// `Uint8Array`.
    U8(Vec<u8>),
    /// `Uint8ClampedArray` — stores clamp to `0..=255` and round half to even
    /// instead of wrapping.
    U8Clamped(Vec<u8>),
    /// `Int16Array`.
    I16(Vec<i16>),
    /// `Uint16Array`.
    U16(Vec<u16>),
    /// `Int32Array`.
    I32(Vec<i32>),
    /// `Uint32Array`.
    U32(Vec<u32>),
}

macro_rules! each_array {
    ($array:expr, $v:ident => $body:expr) => {
        match $array {
            TypedArray::F32($v) => $body,
            TypedArray::F16($v) => $body,
            TypedArray::I8($v) => $body,
            TypedArray::U8($v) => $body,
            TypedArray::U8Clamped($v) => $body,
            TypedArray::I16($v) => $body,
            TypedArray::U16($v) => $body,
            TypedArray::I32($v) => $body,
            TypedArray::U32($v) => $body,
        }
    };
}

impl TypedArray {
    /// `new ArrayType( len )` — `len` zeros of `kind`.
    pub fn zeros(kind: ArrayKind, len: usize) -> Self {
        match kind {
            ArrayKind::F32 => TypedArray::F32(vec![0.0; len]),
            ArrayKind::F16 => TypedArray::F16(vec![0; len]),
            ArrayKind::I8 => TypedArray::I8(vec![0; len]),
            ArrayKind::U8 => TypedArray::U8(vec![0; len]),
            ArrayKind::U8Clamped => TypedArray::U8Clamped(vec![0; len]),
            ArrayKind::I16 => TypedArray::I16(vec![0; len]),
            ArrayKind::U16 => TypedArray::U16(vec![0; len]),
            ArrayKind::I32 => TypedArray::I32(vec![0; len]),
            ArrayKind::U32 => TypedArray::U32(vec![0; len]),
        }
    }

    /// `new ArrayType( values )` — each value converted the way a JavaScript
    /// typed-array store converts a number (see [`set`](Self::set)).
    pub fn from_f64(kind: ArrayKind, values: &[f64]) -> Self {
        let mut array = Self::zeros(kind, values.len());
        for (i, v) in values.iter().enumerate() {
            array.set(i, *v);
        }
        array
    }

    /// `new ArrayType( arrayBuffer )` — a view of little-endian `bytes` as
    /// `kind`, copied out. A `Float16` array keeps the raw bits, as the
    /// `Uint16Array` behind a `Float16BufferAttribute` does.
    ///
    /// # Panics
    ///
    /// If `bytes.len()` is not a multiple of the element size, where
    /// JavaScript throws a `RangeError`.
    pub fn from_le_bytes(kind: ArrayKind, bytes: &[u8]) -> Self {
        let bpe = kind.bytes_per_element();
        assert!(
            bytes.len().is_multiple_of(bpe),
            "three-rs: byte length of {kind:?} should be a multiple of {bpe}, got {}",
            bytes.len()
        );
        fn words<const N: usize, T>(bytes: &[u8], f: impl Fn([u8; N]) -> T) -> Vec<T> {
            bytes.as_chunks::<N>().0.iter().map(|w| f(*w)).collect()
        }
        match kind {
            ArrayKind::F32 => TypedArray::F32(words(bytes, f32::from_le_bytes)),
            ArrayKind::F16 => TypedArray::F16(words(bytes, u16::from_le_bytes)),
            ArrayKind::I8 => TypedArray::I8(bytes.iter().map(|&b| b as i8).collect()),
            ArrayKind::U8 => TypedArray::U8(bytes.to_vec()),
            ArrayKind::U8Clamped => TypedArray::U8Clamped(bytes.to_vec()),
            ArrayKind::I16 => TypedArray::I16(words(bytes, i16::from_le_bytes)),
            ArrayKind::U16 => TypedArray::U16(words(bytes, u16::from_le_bytes)),
            ArrayKind::I32 => TypedArray::I32(words(bytes, i32::from_le_bytes)),
            ArrayKind::U32 => TypedArray::U32(words(bytes, u32::from_le_bytes)),
        }
    }

    /// The element type.
    pub fn kind(&self) -> ArrayKind {
        match self {
            TypedArray::F32(_) => ArrayKind::F32,
            TypedArray::F16(_) => ArrayKind::F16,
            TypedArray::I8(_) => ArrayKind::I8,
            TypedArray::U8(_) => ArrayKind::U8,
            TypedArray::U8Clamped(_) => ArrayKind::U8Clamped,
            TypedArray::I16(_) => ArrayKind::I16,
            TypedArray::U16(_) => ArrayKind::U16,
            TypedArray::I32(_) => ArrayKind::I32,
            TypedArray::U32(_) => ArrayKind::U32,
        }
    }

    /// `array.length`, in elements.
    pub fn len(&self) -> usize {
        each_array!(self, v => v.len())
    }

    /// `array.length === 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// `array[ index ]` as a JavaScript number. For `F16` that is the raw
    /// bits, as three's `Uint16Array` holds them.
    pub fn get(&self, index: usize) -> f64 {
        match self {
            TypedArray::F32(v) => v[index] as f64,
            TypedArray::F16(v) => v[index] as f64,
            TypedArray::I8(v) => v[index] as f64,
            TypedArray::U8(v) => v[index] as f64,
            TypedArray::U8Clamped(v) => v[index] as f64,
            TypedArray::I16(v) => v[index] as f64,
            TypedArray::U16(v) => v[index] as f64,
            TypedArray::I32(v) => v[index] as f64,
            TypedArray::U32(v) => v[index] as f64,
        }
    }

    /// `array[ index ] = value`, with the store's conversion: `F32` rounds to
    /// the nearest `f32`; the integer arrays truncate and wrap (non-finite is
    /// 0); `U8Clamped` clamps to `0..=255` and rounds half to even (NaN is 0).
    pub fn set(&mut self, index: usize, value: f64) {
        match self {
            TypedArray::F32(v) => v[index] = value as f32,
            TypedArray::F16(v) | TypedArray::U16(v) => v[index] = js_wrap(value, 16) as u16,
            TypedArray::I8(v) => v[index] = js_wrap(value, 8) as u8 as i8,
            TypedArray::U8(v) => v[index] = js_wrap(value, 8) as u8,
            TypedArray::U8Clamped(v) => {
                v[index] = if value.is_nan() {
                    0
                } else {
                    value.clamp(0.0, 255.0).round_ties_even() as u8
                }
            }
            TypedArray::I16(v) => v[index] = js_wrap(value, 16) as u16 as i16,
            TypedArray::I32(v) => v[index] = js_wrap(value, 32) as u32 as i32,
            TypedArray::U32(v) => v[index] = js_wrap(value, 32) as u32,
        }
    }

    /// `array[ to ] = other[ from ]` — an element copy, with the store's
    /// conversion when the two arrays differ in type.
    fn copy_element(&mut self, to: usize, other: &TypedArray, from: usize) {
        self.set(to, other.get(from));
    }

    /// [`ArrayKind::vertex_format`] for this array's kind — the table of
    /// decision 3 in the module docs.
    pub fn vertex_format(
        &self,
        item_size: usize,
        normalized: bool,
        interleaved: bool,
    ) -> (wgpu::VertexFormat, usize) {
        self.kind()
            .vertex_format(item_size, normalized, interleaved)
    }

    /// The `Float32Array` itself, or `None` for any other kind.
    pub fn as_f32(&self) -> Option<&Vec<f32>> {
        match self {
            TypedArray::F32(v) => Some(v),
            _ => None,
        }
    }

    /// The `Float32Array` itself, mutably, or `None` for any other kind.
    pub fn as_f32_mut(&mut self) -> Option<&mut Vec<f32>> {
        match self {
            TypedArray::F32(v) => Some(v),
            _ => None,
        }
    }

    /// Element `index` as little-endian bytes of `kind` — `kind` is this
    /// array's own kind or the 32-bit integer one it widens to.
    fn push_bytes(&self, index: usize, kind: ArrayKind, out: &mut Vec<u8>) {
        match (self, kind) {
            (TypedArray::F32(v), _) => out.extend_from_slice(&v[index].to_le_bytes()),
            (TypedArray::I8(v), ArrayKind::I32) => {
                out.extend_from_slice(&(v[index] as i32).to_le_bytes())
            }
            (TypedArray::I16(v), ArrayKind::I32) => {
                out.extend_from_slice(&(v[index] as i32).to_le_bytes())
            }
            (TypedArray::U8(v), ArrayKind::U32) => {
                out.extend_from_slice(&(v[index] as u32).to_le_bytes())
            }
            (TypedArray::U16(v), ArrayKind::U32) => {
                out.extend_from_slice(&(v[index] as u32).to_le_bytes())
            }
            (TypedArray::F16(v) | TypedArray::U16(v), _) => {
                out.extend_from_slice(&v[index].to_le_bytes())
            }
            (TypedArray::I8(v), _) => out.push(v[index] as u8),
            (TypedArray::U8(v) | TypedArray::U8Clamped(v), _) => out.push(v[index]),
            (TypedArray::I16(v), _) => out.extend_from_slice(&v[index].to_le_bytes()),
            (TypedArray::I32(v), _) => out.extend_from_slice(&v[index].to_le_bytes()),
            (TypedArray::U32(v), _) => out.extend_from_slice(&v[index].to_le_bytes()),
        }
    }

    /// The array as little-endian bytes, `kind` per element (see
    /// [`push_bytes`](Self::push_bytes)), `item_size` elements per item laid
    /// out `padded_item_size` apart with zeros between, and the total rounded
    /// up to a multiple of 4 — `createAttribute()`'s `paddedArray` and its
    /// `size` rounding.
    pub(crate) fn to_bytes(
        &self,
        kind: ArrayKind,
        item_size: usize,
        padded_item_size: usize,
    ) -> Vec<u8> {
        let bpe = kind.bytes_per_element();
        let count = self.len() / item_size.max(1);
        let mut out = Vec::with_capacity(count * padded_item_size * bpe + 4);
        if padded_item_size == item_size {
            for i in 0..self.len() {
                self.push_bytes(i, kind, &mut out);
            }
        } else {
            for item in 0..count {
                for c in 0..item_size {
                    self.push_bytes(item * item_size + c, kind, &mut out);
                }
                out.resize(out.len() + (padded_item_size - item_size) * bpe, 0);
            }
        }
        out.resize(out.len().div_ceil(4) * 4, 0);
        out
    }
}

impl From<Vec<f32>> for TypedArray {
    fn from(v: Vec<f32>) -> Self {
        TypedArray::F32(v)
    }
}

impl From<Vec<i8>> for TypedArray {
    fn from(v: Vec<i8>) -> Self {
        TypedArray::I8(v)
    }
}

impl From<Vec<u8>> for TypedArray {
    fn from(v: Vec<u8>) -> Self {
        TypedArray::U8(v)
    }
}

impl From<Vec<i16>> for TypedArray {
    fn from(v: Vec<i16>) -> Self {
        TypedArray::I16(v)
    }
}

impl From<Vec<u16>> for TypedArray {
    fn from(v: Vec<u16>) -> Self {
        TypedArray::U16(v)
    }
}

impl From<Vec<i32>> for TypedArray {
    fn from(v: Vec<i32>) -> Self {
        TypedArray::I32(v)
    }
}

impl From<Vec<u32>> for TypedArray {
    fn from(v: Vec<u32>) -> Self {
        TypedArray::U32(v)
    }
}

/// `Ref<TypedArray>` → `Ref<Vec<f32>>`, panicking the way decision 2 says.
fn f32_ref(array: Ref<'_, TypedArray>) -> Ref<'_, Vec<f32>> {
    Ref::map(array, |a| match a {
        TypedArray::F32(v) => v,
        other => panic_not_f32(other.kind()),
    })
}

/// The `RefMut` twin of [`f32_ref`].
fn f32_ref_mut(array: RefMut<'_, TypedArray>) -> RefMut<'_, Vec<f32>> {
    RefMut::map(array, |a| match a {
        TypedArray::F32(v) => v,
        other => panic_not_f32(other.kind()),
    })
}

fn panic_not_f32(kind: ArrayKind) -> ! {
    panic!(
        "three-rs: array() is the Float32Array view, and this attribute's array is {kind:?}; \
         read it with data()"
    )
}

/// three.js' `InterleavedBuffer` / `InstancedInterleavedBuffer`: one typed
/// array holding several attributes' items side by side, `stride` elements
/// per vertex (or per instance). Attributes are views over it — see
/// [`BufferAttribute::interleaved`]. Shared as `Rc<InterleavedBuffer>`; the
/// renderer uploads it once, as one vertex buffer, keyed on [`id`](Self::id).
#[derive(Debug)]
pub struct InterleavedBuffer {
    id: AttributeId,
    array: RefCell<TypedArray>,
    stride: usize,
    version: Cell<u32>,
    instanced: bool,
    mesh_per_attribute: usize,
}

impl InterleavedBuffer {
    /// `new InterleavedBuffer( array, stride )`.
    pub fn new(array: impl Into<TypedArray>, stride: usize) -> Self {
        Self {
            id: AttributeId::next(),
            array: RefCell::new(array.into()),
            stride,
            version: Cell::new(0),
            instanced: false,
            mesh_per_attribute: 1,
        }
    }

    /// `new InstancedInterleavedBuffer( array, stride, meshPerAttribute )` —
    /// its views step once per instance.
    pub fn new_instanced(
        array: impl Into<TypedArray>,
        stride: usize,
        mesh_per_attribute: usize,
    ) -> Self {
        Self {
            instanced: true,
            mesh_per_attribute,
            ..Self::new(array, stride)
        }
    }

    /// The identity the renderer's GPU buffer is keyed on. Drawn from the
    /// [`AttributeId`] counter, so it never equals an attribute's id.
    pub fn id(&self) -> usize {
        self.id.get()
    }

    /// `interleavedBuffer.array` as the `Float32Array` it usually is.
    ///
    /// # Panics
    ///
    /// If the array is another kind; read it with [`data`](Self::data).
    pub fn array(&self) -> Ref<'_, Vec<f32>> {
        f32_ref(self.array.borrow())
    }

    /// [`array`](Self::array), for writing. Follow it with
    /// [`set_needs_update`](Self::set_needs_update).
    pub fn array_mut(&self) -> RefMut<'_, Vec<f32>> {
        f32_ref_mut(self.array.borrow_mut())
    }

    /// `interleavedBuffer.array`, whatever its kind.
    pub fn data(&self) -> Ref<'_, TypedArray> {
        self.array.borrow()
    }

    /// [`data`](Self::data), for writing.
    pub fn data_mut(&self) -> RefMut<'_, TypedArray> {
        self.array.borrow_mut()
    }

    /// `InterleavedBuffer.stride`, in elements.
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// `InterleavedBuffer.count` — `array.length / stride`.
    pub fn count(&self) -> usize {
        self.array.borrow().len() / self.stride
    }

    /// `InterleavedBuffer.version`.
    pub fn version(&self) -> u32 {
        self.version.get()
    }

    /// `interleavedBuffer.needsUpdate = true`. Every view of it re-uploads.
    pub fn set_needs_update(&self) {
        self.version.set(self.version.get() + 1);
    }

    /// `isInstancedInterleavedBuffer`.
    pub fn is_instanced(&self) -> bool {
        self.instanced
    }

    /// `InstancedInterleavedBuffer.meshPerAttribute` (1 on a plain buffer).
    pub fn mesh_per_attribute(&self) -> usize {
        self.mesh_per_attribute
    }

    /// `InterleavedBuffer.copyAt( index1, interleavedBuffer, index2 )` —
    /// copies one vertex's `stride` elements.
    pub fn copy_at(&self, index1: usize, other: &InterleavedBuffer, index2: usize) -> &Self {
        let index1 = index1 * self.stride;
        let index2 = index2 * other.stride;
        if std::ptr::eq(self, other) {
            let mut array = self.array.borrow_mut();
            for i in 0..self.stride {
                let v = array.get(index2 + i);
                array.set(index1 + i, v);
            }
        } else {
            let source = other.array.borrow();
            let mut array = self.array.borrow_mut();
            for i in 0..self.stride {
                array.copy_element(index1 + i, &source, index2 + i);
            }
        }
        self
    }

    /// `InterleavedBuffer.set( value, offset )`.
    pub fn set(&self, value: &[f32], offset: usize) -> &Self {
        let mut array = self.array.borrow_mut();
        for (i, v) in value.iter().enumerate() {
            array.set(offset + i, *v as f64);
        }
        self
    }

    /// `InterleavedBuffer.copy( source )` — the array (a copy of it, as
    /// `new source.array.constructor( source.array )`) and the stride; on an
    /// instanced buffer, `InstancedInterleavedBuffer.copy` also takes
    /// `meshPerAttribute`.
    pub fn copy(&mut self, source: &InterleavedBuffer) -> &mut Self {
        *self.array.get_mut() = source.array.borrow().clone();
        self.stride = source.stride;
        if self.instanced {
            self.mesh_per_attribute = source.mesh_per_attribute;
        }
        self
    }
}

/// `InterleavedBuffer.clone()` / `InstancedInterleavedBuffer.clone()`: a copy
/// of the array under a fresh id, `version` 0, `meshPerAttribute` kept.
impl Clone for InterleavedBuffer {
    fn clone(&self) -> Self {
        Self {
            id: AttributeId::next(),
            array: RefCell::new(self.array.borrow().clone()),
            stride: self.stride,
            version: Cell::new(0),
            instanced: self.instanced,
            mesh_per_attribute: self.mesh_per_attribute,
        }
    }
}

/// What [`StorageBufferAttribute::new`] is given: three's `count` (a number,
/// for a zero-filled array of `count * itemSize`) or a typed array.
#[derive(Clone, Debug)]
pub enum StorageContents {
    /// `new StorageBufferAttribute( count, itemSize, typeClass )`.
    Count(usize),
    /// `new StorageBufferAttribute( array, itemSize )`.
    Array(TypedArray),
}

impl From<usize> for StorageContents {
    fn from(count: usize) -> Self {
        StorageContents::Count(count)
    }
}

impl From<TypedArray> for StorageContents {
    fn from(array: TypedArray) -> Self {
        StorageContents::Array(array)
    }
}

impl From<Vec<f32>> for StorageContents {
    fn from(array: Vec<f32>) -> Self {
        StorageContents::Array(TypedArray::F32(array))
    }
}

/// three.js' `StorageBufferAttribute` / `StorageInstancedBufferAttribute`
/// (`src/renderers/common/`): an attribute whose GPU buffer is also a storage
/// buffer, so a compute kernel can write what a draw reads as vertices.
///
/// A handle, as [`IndirectStorageBufferAttribute`](super::IndirectStorageBufferAttribute)
/// is: clones are the same attribute, and the renderer keys the one GPU buffer
/// on [`id`](Self::id), which `storage( &attribute, ty, count )` (in
/// `nodes::tsl`) carries too. Turn it into a geometry attribute with
/// `.into()`. The array is the buffer's initial contents only; it is uploaded
/// once, padded to `vec4` stride when the item size is 3.
#[derive(Clone, Debug)]
pub struct StorageBufferAttribute {
    id: BufferId,
    array: Rc<RefCell<TypedArray>>,
    /// `BufferAttribute.itemSize`, unpadded.
    pub item_size: usize,
    instanced: bool,
}

impl StorageBufferAttribute {
    /// `new StorageBufferAttribute( count | array, itemSize, typeClass )`. With
    /// a count, the array is `count * item_size` zeros of `kind`; with an
    /// array, `kind` is ignored as three ignores `typeClass`.
    pub fn new(contents: impl Into<StorageContents>, item_size: usize, kind: ArrayKind) -> Self {
        let array = match contents.into() {
            StorageContents::Count(count) => TypedArray::zeros(kind, count * item_size),
            StorageContents::Array(array) => array,
        };
        Self {
            id: BufferId::next(),
            array: Rc::new(RefCell::new(array)),
            item_size,
            instanced: false,
        }
    }

    /// `new StorageInstancedBufferAttribute( count | array, itemSize, typeClass )`
    /// — read as a vertex attribute, it steps once per instance.
    pub fn new_instanced(
        contents: impl Into<StorageContents>,
        item_size: usize,
        kind: ArrayKind,
    ) -> Self {
        Self {
            instanced: true,
            ..Self::new(contents, item_size, kind)
        }
    }

    /// The identity the renderer's GPU buffer is keyed on.
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// `BufferAttribute.count` — `array.length / itemSize`.
    pub fn count(&self) -> usize {
        self.array.borrow().len() / self.item_size
    }

    /// `attribute.array` — the initial contents.
    pub fn data(&self) -> Ref<'_, TypedArray> {
        self.array.borrow()
    }

    /// `isStorageInstancedBufferAttribute`.
    pub fn is_instanced(&self) -> bool {
        self.instanced
    }

    /// The kind of the uploaded elements: the array's own, or — for 8- and
    /// 16-bit integers that are not `normalized` — the 32-bit one
    /// `createAttribute()`'s "patch for INT16 and UINT16" widens it to, which
    /// it applies to a storage attribute as to any other.
    pub(crate) fn upload_kind(&self, normalized: bool) -> ArrayKind {
        self.array.borrow().kind().upload_kind(normalized, false)
    }

    /// Components per element in the GPU buffer, after widening: 4 for an
    /// item size of 3, otherwise `item_size` rounded up to a 4-byte stride.
    pub(crate) fn padded_item_size(&self, normalized: bool) -> usize {
        padded_item_size(self.upload_kind(normalized), self.item_size, true)
    }

    /// The GPU buffer's initial contents as 32-bit words, widened and at the
    /// padded stride — what `createAttribute()` uploads for a storage
    /// attribute. `normalized` is the attribute view's (`false` for a
    /// `storage()` node, as three's `StorageBufferAttribute` never sets it).
    pub(crate) fn init_words(&self, normalized: bool) -> Vec<u32> {
        let kind = self.upload_kind(normalized);
        let padded = self.padded_item_size(normalized);
        let array = self.array.borrow();
        let bytes = array.to_bytes(kind, self.item_size, padded);
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|w| u32::from_le_bytes(*w))
            .collect()
    }
}

impl From<StorageBufferAttribute> for BufferAttribute {
    fn from(storage: StorageBufferAttribute) -> Self {
        let instanced = storage.instanced;
        let item_size = storage.item_size;
        Self {
            id: AttributeId::next(),
            storage: Backing::Storage(storage),
            item_size,
            normalized: false,
            version: Cell::new(0),
            instanced,
        }
    }
}

/// Where a [`BufferAttribute`]'s elements live.
#[derive(Debug)]
enum Backing {
    /// Its own typed array.
    Own(RefCell<TypedArray>),
    /// A view into an [`InterleavedBuffer`], `offset` elements into each
    /// stride.
    Interleaved {
        data: Rc<InterleavedBuffer>,
        offset: usize,
    },
    /// A [`StorageBufferAttribute`]'s buffer.
    Storage(StorageBufferAttribute),
}

/// three.js' `BufferAttribute`, its typed subclasses (`Uint8BufferAttribute`,
/// …, `Float16BufferAttribute`), `InstancedBufferAttribute` and
/// `InterleavedBufferAttribute`: one named vertex attribute, `item_size`
/// values per vertex. See the module docs for the shape.
#[derive(Debug)]
pub struct BufferAttribute {
    /// `BufferAttribute.id`. Read-only in spirit; see [`AttributeId`].
    pub id: AttributeId,
    /// The data, behind a `RefCell` because a geometry is shared as
    /// `Rc<BufferGeometry>` and three.js mutates attribute data in place
    /// (issue #47).
    storage: Backing,
    /// `BufferAttribute.itemSize` — the number of values per vertex.
    pub item_size: usize,
    /// `BufferAttribute.normalized` — integer data read as `0..1` (or
    /// `-1..1`) floats, by the GPU and by the `get_*`/`set_*` accessors.
    pub normalized: bool,
    /// `BufferAttribute.version` — bumped by
    /// [`set_needs_update`](Self::set_needs_update). An interleaved view uses
    /// its buffer's instead.
    version: Cell<u32>,
    /// `isInstancedBufferAttribute` — the attribute steps once per instance,
    /// not per vertex. Only meaningful on an instanced geometry; see
    /// [`BufferGeometry::instance_count`](super::BufferGeometry::instance_count).
    /// An interleaved view uses its buffer's instead.
    instanced: bool,
}

/// `BufferAttribute`'s typed constructors: `new Uint8BufferAttribute( array,
/// itemSize, normalized )` is `BufferAttribute::uint8( array, item_size,
/// normalized )`, and so on.
macro_rules! typed_constructors {
    ($($(#[$doc:meta])* $name:ident($elem:ty) => $variant:ident;)*) => {
        $(
            $(#[$doc])*
            pub fn $name(array: Vec<$elem>, item_size: usize, normalized: bool) -> Self {
                Self::from_typed(TypedArray::$variant(array), item_size, normalized)
            }
        )*
    };
}

impl BufferAttribute {
    /// `new BufferAttribute( array, itemSize )` over a `Float32Array` —
    /// `Float32BufferAttribute`, three's default.
    pub fn new(array: Vec<f32>, item_size: usize) -> Self {
        Self::from_typed(TypedArray::F32(array), item_size, false)
    }

    /// `new BufferAttribute( array, itemSize, normalized )` — the base
    /// constructor every typed one calls.
    pub fn from_typed(array: TypedArray, item_size: usize, normalized: bool) -> Self {
        Self {
            id: AttributeId::next(),
            storage: Backing::Own(RefCell::new(array)),
            item_size,
            normalized,
            version: Cell::new(0),
            instanced: false,
        }
    }

    typed_constructors! {
        /// `new Int8BufferAttribute( array, itemSize, normalized )`.
        int8(i8) => I8;
        /// `new Uint8BufferAttribute( array, itemSize, normalized )`.
        uint8(u8) => U8;
        /// `new Uint8ClampedBufferAttribute( array, itemSize, normalized )`.
        uint8_clamped(u8) => U8Clamped;
        /// `new Int16BufferAttribute( array, itemSize, normalized )`.
        int16(i16) => I16;
        /// `new Uint16BufferAttribute( array, itemSize, normalized )`.
        uint16(u16) => U16;
        /// `new Int32BufferAttribute( array, itemSize, normalized )`.
        int32(i32) => I32;
        /// `new Uint32BufferAttribute( array, itemSize, normalized )`.
        uint32(u32) => U32;
    }

    /// `new Float16BufferAttribute( array, itemSize, normalized )` — `array`
    /// is binary16 bits, as three's `Uint16Array` is. Build it from floats
    /// with [`to_half_float`], or with [`set_x`](Self::set_x) and friends,
    /// which convert.
    pub fn float16(array: Vec<u16>, item_size: usize, normalized: bool) -> Self {
        Self::from_typed(TypedArray::F16(array), item_size, normalized)
    }

    /// `new InstancedBufferAttribute( array, itemSize )` — one element per
    /// instance. See [`is_instanced`](Self::is_instanced).
    pub fn new_instanced(array: Vec<f32>, item_size: usize) -> Self {
        Self::from_typed_instanced(TypedArray::F32(array), item_size, false)
    }

    /// `new InstancedBufferAttribute( array, itemSize, normalized )` over any
    /// typed array.
    pub fn from_typed_instanced(array: TypedArray, item_size: usize, normalized: bool) -> Self {
        Self {
            instanced: true,
            ..Self::from_typed(array, item_size, normalized)
        }
    }

    /// `new InterleavedBufferAttribute( interleavedBuffer, itemSize, offset,
    /// normalized )` — a view of `item_size` elements, `offset` elements into
    /// each of `data`'s strides.
    pub fn interleaved(
        data: Rc<InterleavedBuffer>,
        item_size: usize,
        offset: usize,
        normalized: bool,
    ) -> Self {
        Self {
            id: AttributeId::next(),
            storage: Backing::Interleaved { data, offset },
            item_size,
            normalized,
            version: Cell::new(0),
            instanced: false,
        }
    }

    /// `attribute.isInstancedBufferAttribute`, or for an interleaved view
    /// `attribute.data.isInstancedInterleavedBuffer`.
    pub fn is_instanced(&self) -> bool {
        match &self.storage {
            Backing::Interleaved { data, .. } => data.is_instanced(),
            _ => self.instanced,
        }
    }

    /// `attribute.isInterleavedBufferAttribute`.
    pub fn is_interleaved(&self) -> bool {
        matches!(self.storage, Backing::Interleaved { .. })
    }

    /// `InterleavedBufferAttribute.data` — `None` for an attribute with its
    /// own array.
    pub fn data_buffer(&self) -> Option<&Rc<InterleavedBuffer>> {
        match &self.storage {
            Backing::Interleaved { data, .. } => Some(data),
            _ => None,
        }
    }

    /// `InterleavedBufferAttribute.offset`, in elements (0 for an attribute
    /// with its own array).
    pub fn offset(&self) -> usize {
        match &self.storage {
            Backing::Interleaved { offset, .. } => *offset,
            _ => 0,
        }
    }

    /// The [`StorageBufferAttribute`] this attribute reads, if it is one.
    pub fn storage_buffer(&self) -> Option<&StorageBufferAttribute> {
        match &self.storage {
            Backing::Storage(storage) => Some(storage),
            _ => None,
        }
    }

    /// The cell holding the elements: this attribute's own, the interleaved
    /// buffer's, or the storage attribute's.
    fn cell(&self) -> &RefCell<TypedArray> {
        match &self.storage {
            Backing::Own(array) => array,
            Backing::Interleaved { data, .. } => &data.array,
            Backing::Storage(storage) => &storage.array,
        }
    }

    /// `(stride, offset)` in elements: how item `i`'s first element is found,
    /// at `offset + i * stride`.
    fn layout(&self) -> (usize, usize) {
        match &self.storage {
            Backing::Interleaved { data, offset } => (data.stride, *offset),
            _ => (self.item_size, 0),
        }
    }

    /// The element type.
    pub fn kind(&self) -> ArrayKind {
        self.cell().borrow().kind()
    }

    /// `attribute.array` as the `Float32Array` it usually is, for reading. A
    /// `Ref`, so the borrow has to be held for as long as the slice is used.
    /// On an interleaved view it is the **whole** shared array, as three's
    /// `InterleavedBufferAttribute.array` getter returns.
    ///
    /// # Panics
    ///
    /// If the array is another kind (`Uint8Array`, …); read it with
    /// [`data`](Self::data).
    pub fn array(&self) -> Ref<'_, Vec<f32>> {
        f32_ref(self.cell().borrow())
    }

    /// `attribute.array`, for writing — through a shared `&self`, so it works
    /// on an attribute of a geometry already handed to a mesh as an `Rc`.
    ///
    /// Writing alone changes nothing on screen: follow it with
    /// [`set_needs_update`](Self::set_needs_update), exactly as three.js needs
    /// `attribute.needsUpdate = true`.
    ///
    /// # Panics
    ///
    /// As [`array`](Self::array).
    pub fn array_mut(&self) -> RefMut<'_, Vec<f32>> {
        f32_ref_mut(self.cell().borrow_mut())
    }

    /// `attribute.array`, whatever its kind.
    pub fn data(&self) -> Ref<'_, TypedArray> {
        self.cell().borrow()
    }

    /// [`data`](Self::data), for writing.
    pub fn data_mut(&self) -> RefMut<'_, TypedArray> {
        self.cell().borrow_mut()
    }

    /// `BufferAttribute.version` (an interleaved view's is its buffer's).
    pub fn version(&self) -> u32 {
        match &self.storage {
            Backing::Interleaved { data, .. } => data.version(),
            _ => self.version.get(),
        }
    }

    /// `attribute.needsUpdate = true` — `version ++`. The next render that
    /// sees this geometry re-writes this attribute's GPU buffer (an
    /// interleaved view's buffer, shared by all its views), and only it.
    pub fn set_needs_update(&self) {
        match &self.storage {
            Backing::Interleaved { data, .. } => data.set_needs_update(),
            _ => self.version.set(self.version.get() + 1),
        }
    }

    /// `BufferAttribute.count` — the number of vertices, `array.length /
    /// item_size` (`data.count` for an interleaved view).
    pub fn count(&self) -> usize {
        match &self.storage {
            Backing::Interleaved { data, .. } => data.count(),
            _ => self.cell().borrow().len() / self.item_size,
        }
    }

    /// `BufferAttribute.getComponent( index, component )` and the shared body
    /// of `getX..getW`: the element widened to `f64`, `fromHalfFloat` on a
    /// `Float16BufferAttribute`, `denormalize`d when `normalized`.
    pub fn get_component(&self, index: usize, component: usize) -> f64 {
        let (stride, offset) = self.layout();
        let array = self.cell().borrow();
        let kind = array.kind();
        let mut value = array.get(offset + index * stride + component);
        if kind == ArrayKind::F16 {
            value = from_half_float(value as u16) as f64;
        }
        if self.normalized {
            value = kind.denormalize(value);
        }
        value
    }

    /// `BufferAttribute.setComponent( index, component, value )` and the
    /// shared body of `setX..setXYZW`: `normalize`d when `normalized`,
    /// `toHalfFloat` on a `Float16BufferAttribute`, then stored with the
    /// typed array's conversion.
    pub fn set_component(&mut self, index: usize, component: usize, value: f64) -> &mut Self {
        self.store(index, component, value);
        self
    }

    /// [`set_component`](Self::set_component) through `&self`, for crate
    /// code holding the attribute behind a shared geometry (the array is in
    /// a `RefCell`). Does not bump the version.
    pub(crate) fn store(&self, index: usize, component: usize, value: f64) {
        let (stride, offset) = self.layout();
        let mut array = self.cell().borrow_mut();
        let kind = array.kind();
        let mut value = value;
        if self.normalized {
            value = kind.normalize(value);
        }
        if kind == ArrayKind::F16 {
            value = to_half_float(value) as f64;
        }
        array.set(offset + index * stride + component, value);
    }

    /// `BufferAttribute.getX()`.
    pub fn get_x(&self, index: usize) -> f64 {
        self.get_component(index, 0)
    }

    /// `BufferAttribute.getY()`.
    pub fn get_y(&self, index: usize) -> f64 {
        self.get_component(index, 1)
    }

    /// `BufferAttribute.getZ()`.
    pub fn get_z(&self, index: usize) -> f64 {
        self.get_component(index, 2)
    }

    /// `BufferAttribute.getW()`.
    pub fn get_w(&self, index: usize) -> f64 {
        self.get_component(index, 3)
    }

    /// `BufferAttribute.setX()`.
    pub fn set_x(&mut self, index: usize, x: f64) -> &mut Self {
        self.set_component(index, 0, x)
    }

    /// `BufferAttribute.setY()`.
    pub fn set_y(&mut self, index: usize, y: f64) -> &mut Self {
        self.set_component(index, 1, y)
    }

    /// `BufferAttribute.setZ()`.
    pub fn set_z(&mut self, index: usize, z: f64) -> &mut Self {
        self.set_component(index, 2, z)
    }

    /// `BufferAttribute.setW()`.
    pub fn set_w(&mut self, index: usize, w: f64) -> &mut Self {
        self.set_component(index, 3, w)
    }

    /// `BufferAttribute.setXY()`.
    pub fn set_xy(&mut self, index: usize, x: f64, y: f64) -> &mut Self {
        self.store(index, 0, x);
        self.store(index, 1, y);
        self
    }

    /// `BufferAttribute.setXYZ()` — narrows on the way in, which is where
    /// three.js loses precision too.
    pub fn set_xyz(&mut self, index: usize, x: f64, y: f64, z: f64) {
        self.store(index, 0, x);
        self.store(index, 1, y);
        self.store(index, 2, z);
    }

    /// `BufferAttribute.setXYZW()`.
    pub fn set_xyzw(&mut self, index: usize, x: f64, y: f64, z: f64, w: f64) -> &mut Self {
        self.store(index, 0, x);
        self.store(index, 1, y);
        self.store(index, 2, z);
        self.store(index, 3, w);
        self
    }

    /// `BufferAttribute.copyAt()` — copies one item's raw elements from
    /// `attribute` (no normalization; the store's conversion if the kinds
    /// differ).
    pub fn copy_at(&mut self, index1: usize, attribute: &Self, index2: usize) -> &mut Self {
        let (stride1, offset1) = self.layout();
        let (stride2, offset2) = attribute.layout();
        let base1 = offset1 + index1 * stride1;
        let base2 = offset2 + index2 * stride2;
        if std::ptr::eq(self.cell(), attribute.cell()) {
            let mut array = self.cell().borrow_mut();
            for i in 0..self.item_size {
                let v = array.get(base2 + i);
                array.set(base1 + i, v);
            }
        } else {
            let source = attribute.cell().borrow();
            let mut array = self.cell().borrow_mut();
            for i in 0..self.item_size {
                array.copy_element(base1 + i, &source, base2 + i);
            }
        }
        self
    }

    /// `BufferAttribute.copyArray()` — `this.array.set( array )`: `array`
    /// over the start of the attribute's array, element by element with the
    /// store's conversion. A shorter `array` leaves the rest as it was.
    ///
    /// # Panics
    ///
    /// If `array` is longer than the attribute's array — `TypedArray.set()`'s
    /// `RangeError`.
    pub fn copy_array(&mut self, array: &[f32]) -> &mut Self {
        let mut target = self.cell().borrow_mut();
        assert!(
            array.len() <= target.len(),
            "three-rs: copy_array(): offset is out of bounds ({} elements into {})",
            array.len(),
            target.len()
        );
        if let Some(v) = target.as_f32_mut() {
            v[..array.len()].copy_from_slice(array);
        } else {
            for (i, v) in array.iter().enumerate() {
                target.set(i, *v as f64);
            }
        }
        drop(target);
        self
    }

    /// `BufferAttribute.set( value, offset )` — raw elements, with the store's
    /// conversion (so on a `Float16BufferAttribute` the values are bits, as in
    /// three).
    pub fn set(&mut self, value: &[f32], offset: usize) -> &mut Self {
        let mut target = self.cell().borrow_mut();
        if let Some(v) = target.as_f32_mut() {
            v[offset..offset + value.len()].copy_from_slice(value);
        } else {
            for (i, v) in value.iter().enumerate() {
                target.set(offset + i, *v as f64);
            }
        }
        drop(target);
        self
    }

    /// `BufferAttribute.copy( source )` — array (copied), item size and
    /// `normalized`; not the id, not the version. An interleaved view copies
    /// its buffer reference and offset (`InterleavedBufferAttribute` has no
    /// `copy`, so this is what `BufferAttribute.copy` would leave it).
    pub fn copy(&mut self, source: &BufferAttribute) -> &mut Self {
        self.storage = match &source.storage {
            Backing::Own(array) => Backing::Own(RefCell::new(array.borrow().clone())),
            Backing::Interleaved { data, offset } => Backing::Interleaved {
                data: data.clone(),
                offset: *offset,
            },
            Backing::Storage(storage) => Backing::Own(RefCell::new(storage.array.borrow().clone())),
        };
        self.item_size = source.item_size;
        self.normalized = source.normalized;
        self.instanced = source.instanced;
        self
    }

    /// The items as `f32`, `item_size` per item, de-interleaved and read
    /// through [`get_component`](Self::get_component) — so a normalized
    /// integer array denormalizes and a `Float16` one decodes. For code that
    /// hands three's values on as floats whatever the array is; an own
    /// `Float32Array` is copied as it is.
    pub(crate) fn to_f32_items(&self) -> Vec<f32> {
        if let Backing::Own(array) = &self.storage {
            if let Some(v) = array.borrow().as_f32() {
                return v.clone();
            }
        }
        (0..self.count())
            .flat_map(|i| (0..self.item_size).map(move |c| self.get_component(i, c) as f32))
            .collect()
    }

    /// `Vector3.fromBufferAttribute( attribute, index )`.
    pub fn get_vector3(&self, index: usize) -> Vector3 {
        Vector3::new(self.get_x(index), self.get_y(index), self.get_z(index))
    }

    /// `BufferAttribute.applyMatrix4()`.
    pub fn apply_matrix4(&mut self, m: &Matrix4) {
        for i in 0..self.count() {
            let mut v = self.get_vector3(i);
            v.apply_matrix4(m);
            self.set_xyz(i, v.x, v.y, v.z);
        }
    }

    /// `BufferAttribute.applyNormalMatrix()`.
    pub fn apply_normal_matrix(&mut self, m: &Matrix3) {
        for i in 0..self.count() {
            let mut v = self.get_vector3(i);
            v.apply_normal_matrix(m);
            self.set_xyz(i, v.x, v.y, v.z);
        }
    }

    /// This attribute's items in its own layout and kind, de-interleaved:
    /// `item_size` raw elements per item, for every item in `indices` (or all
    /// of them).
    fn gather_array(&self, indices: Option<&[usize]>) -> TypedArray {
        let (stride, offset) = self.layout();
        let source = self.cell().borrow();
        let n = self.item_size;
        let count = indices.map_or(self.count(), <[usize]>::len);
        let mut out = TypedArray::zeros(source.kind(), count * n);
        for k in 0..count {
            let item = indices.map_or(k, |indices| indices[k]);
            for c in 0..n {
                out.copy_element(k * n + c, &source, offset + item * stride + c);
            }
        }
        out
    }

    /// `BufferGeometry.toNonIndexed()`'s `convertBufferAttribute()`: a plain
    /// attribute of the same kind, item size and `normalized` holding the
    /// items at `indices`, in order. An interleaved view is read through its
    /// stride, as three's `getComponent` would.
    pub(crate) fn gather(&self, indices: &[usize]) -> BufferAttribute {
        BufferAttribute::from_typed(
            self.gather_array(Some(indices)),
            self.item_size,
            self.normalized,
        )
    }

    /// `BufferAttribute.clone()` with three's `clone( data )` cache: a view
    /// over an [`InterleavedBuffer`] already cloned for this geometry (keyed
    /// on its id) shares that clone, so two views stay one buffer — which is
    /// what `BufferGeometry.clone()` gets from passing one `data` object to
    /// every `attribute.clone( data )`.
    pub(crate) fn clone_with(
        &self,
        buffers: &mut HashMap<usize, Rc<InterleavedBuffer>>,
    ) -> BufferAttribute {
        match &self.storage {
            Backing::Interleaved { data, offset } => {
                let data = buffers
                    .entry(data.id())
                    .or_insert_with(|| Rc::new((**data).clone()))
                    .clone();
                BufferAttribute::interleaved(data, self.item_size, *offset, self.normalized)
            }
            _ => self.clone(),
        }
    }

    /// The renderer's key for this attribute's GPU buffer: the attribute's
    /// own id, its interleaved buffer's, or its storage buffer's.
    pub(crate) fn buffer_key(&self) -> BufferKey {
        match &self.storage {
            Backing::Own(_) => BufferKey::Attribute(self.id.get()),
            Backing::Interleaved { data, .. } => BufferKey::Interleaved(data.id()),
            Backing::Storage(storage) => BufferKey::Storage(storage.id()),
        }
    }

    /// The bytes `WebGPUAttributeUtils.createAttribute()` writes for this
    /// attribute's GPU buffer: an interleaved buffer's array as it is; an own
    /// array widened and padded per decision 3; a storage attribute's padded
    /// initial contents.
    pub(crate) fn upload_bytes(&self) -> Vec<u8> {
        match &self.storage {
            Backing::Interleaved { data, .. } => {
                let array = data.array.borrow();
                array.to_bytes(array.kind(), 1, 1)
            }
            Backing::Own(array) => {
                let array = array.borrow();
                let kind = array.kind().upload_kind(self.normalized, false);
                let padded = padded_item_size(kind, self.item_size, false);
                array.to_bytes(kind, self.item_size, padded)
            }
            Backing::Storage(storage) => storage
                .init_words(self.normalized)
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect(),
        }
    }

    /// The vertex-input description of this attribute under `name`, for the
    /// node builder — see [`AttributeDesc`]. `group` is the geometry-local
    /// index naming this attribute's vertex buffer.
    pub(crate) fn desc(&self, name: &str, group: usize) -> AttributeDesc {
        let layout = match &self.storage {
            Backing::Own(_) => AttributeLayout::Own,
            Backing::Interleaved { data, offset } => AttributeLayout::Interleaved {
                buffer: group,
                stride: data.stride,
                offset: *offset,
            },
            Backing::Storage(storage) => AttributeLayout::Storage {
                padded_item_size: storage.padded_item_size(self.normalized),
            },
        };
        AttributeDesc {
            name: name.to_string(),
            kind: self.kind(),
            item_size: self.item_size,
            normalized: self.normalized,
            instanced: self.is_instanced(),
            layout,
        }
    }
}

/// `BufferAttribute.clone()` — `new this.constructor( this.array, this.itemSize
/// ).copy( this )`: a fresh id, version 0, a copy of the array.
///
/// An interleaved view is three's `InterleavedBufferAttribute.clone()` with no
/// `data` argument, which **de-interleaves**: the clone is a plain attribute
/// of the same kind holding just this view's items (three logs a warning; so
/// does nothing here). Clone a whole geometry to keep the interleaving — see
/// `BufferGeometry`'s `Clone`. A storage attribute's clone is a new storage
/// attribute (a new GPU buffer) with a copy of the initial contents.
impl Clone for BufferAttribute {
    fn clone(&self) -> Self {
        let storage = match &self.storage {
            Backing::Own(array) => Backing::Own(RefCell::new(array.borrow().clone())),
            Backing::Interleaved { .. } => Backing::Own(RefCell::new(self.gather_array(None))),
            Backing::Storage(storage) => Backing::Storage(StorageBufferAttribute {
                id: BufferId::next(),
                array: Rc::new(RefCell::new(storage.array.borrow().clone())),
                item_size: storage.item_size,
                instanced: storage.instanced,
            }),
        };
        Self {
            id: AttributeId::next(),
            storage,
            item_size: self.item_size,
            normalized: self.normalized,
            version: Cell::new(0),
            instanced: self.instanced,
        }
    }
}

/// What the renderer keys a vertex buffer on — see
/// [`BufferAttribute::buffer_key`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum BufferKey {
    /// An attribute with its own array, by [`AttributeId`].
    Attribute(usize),
    /// An [`InterleavedBuffer`], by its id.
    Interleaved(usize),
    /// A [`StorageBufferAttribute`]'s storage buffer.
    Storage(BufferId),
}

/// How a [`AttributeDesc`]'s elements are laid out in its vertex buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttributeLayout {
    /// Its own buffer, packed, padded per item to a 4-byte stride.
    Own,
    /// A view into an [`InterleavedBuffer`]. `buffer` is a geometry-local
    /// group number — the position in the geometry's attribute list of the
    /// first view of the same buffer — so geometries with the same layout
    /// hash to the same program; `stride` and `offset` are in elements.
    Interleaved {
        /// Group number of the shared vertex buffer.
        buffer: usize,
        /// `InterleavedBuffer.stride`.
        stride: usize,
        /// `InterleavedBufferAttribute.offset`.
        offset: usize,
    },
    /// A [`StorageBufferAttribute`]'s buffer, `padded_item_size` elements per
    /// item.
    Storage {
        /// Components per element in the GPU buffer.
        padded_item_size: usize,
    },
}

/// One geometry attribute as the node builder sees it — `SetupContext::
/// geometry_attributes`, decision 4 in the module docs: enough to declare the
/// vertex input in the attribute's own type and to lay out its vertex buffer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttributeDesc {
    /// The attribute's name on the geometry.
    pub name: String,
    /// Element type.
    pub kind: ArrayKind,
    /// `itemSize`.
    pub item_size: usize,
    /// `normalized`.
    pub normalized: bool,
    /// Steps per instance rather than per vertex.
    pub instanced: bool,
    /// Buffer layout.
    pub layout: AttributeLayout,
}

impl AttributeDesc {
    /// `getTypeFromAttribute()` — see [`ArrayKind::shader_type`].
    pub fn shader_type(&self) -> Type {
        self.kind.shader_type(self.item_size, self.normalized)
    }

    /// The vertex format — [`ArrayKind::vertex_format`].
    pub fn vertex_format(&self) -> wgpu::VertexFormat {
        let interleaved = matches!(self.layout, AttributeLayout::Interleaved { .. });
        self.kind
            .vertex_format(self.item_size, self.normalized, interleaved)
            .0
    }

    /// `arrayStride` of its vertex buffer, in bytes
    /// (`createShaderVertexBuffers()`).
    pub fn array_stride(&self) -> u64 {
        let stride = match self.layout {
            AttributeLayout::Own => {
                let kind = self.kind.upload_kind(self.normalized, false);
                (self.item_size * kind.bytes_per_element()).div_ceil(4) * 4
            }
            AttributeLayout::Interleaved { stride, .. } => stride * self.kind.bytes_per_element(),
            AttributeLayout::Storage { padded_item_size } => {
                let kind = self.kind.upload_kind(self.normalized, false);
                padded_item_size * kind.bytes_per_element()
            }
        };
        stride as u64
    }

    /// The attribute's byte offset within each stride.
    pub fn offset(&self) -> u64 {
        match self.layout {
            AttributeLayout::Interleaved { offset, .. } => {
                (offset * self.kind.bytes_per_element()) as u64
            }
            _ => 0,
        }
    }
}
