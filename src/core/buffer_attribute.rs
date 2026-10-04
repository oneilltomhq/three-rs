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
//! `F32(Vec<f32>)`, `F16(Vec<u16>)` (IEEE binary16 bits; `half::f16` for the
//! conversions, already in the lock file through naga), `I8`, `U8`,
//! `U8Clamped`, `I16`, `U16`, `I32`, `U32`. `normalized` is a field beside it,
//! as in three.
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
//! * `F32` → `Float32xN`; `I32`/`U32` → `Sint32xN`/`Uint32xN`, `N` 1..4.
//! * `F16` → `Float16xN` with `N` padded to 2 or 4 (a 16-bit stride must be a
//!   multiple of 4 bytes). `itemSize === 1` has no `Float16` entry in three's
//!   item-size-1 map and errors; the port panics with the same message.
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
//! `item_size`, `normalized`, step mode, and for an interleaved attribute the
//! buffer's id, stride and offset. It is hashed into the program cache key
//! (as `instanced_attributes` is now, which it replaces: "instanced" is a bit
//! on the descriptor). `vertex_color_size` and `has_tangent_attribute` could
//! be derived from it; leave them alone in this PR, the diff is wide enough.
//!
//! `NodeProgram::vertex_buffers()` builds one `VertexBufferDesc` per geometry
//! attribute buffer: non-interleaved attributes one buffer each, interleaved
//! attributes sharing an `InterleavedBuffer` one buffer with several entries —
//! the grouping it already does for `InstanceBuffer`, keyed on the interleaved
//! buffer's id instead of an `Rc` pointer. `programs::vertex_format(Type)`
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
//! as `LineSegmentsGeometry.js` does; colours and distances likewise.
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
//! storage-buffer map rather than uploading it. `itemSize === 3` storage is
//! padded to `vec4` stride as `createAttribute()` pads it; the attribute view
//! of a padded buffer uses that stride.
//!
//! ## Not in this PR
//!
//! TSL `bufferAttribute()` / `dynamicBufferAttribute()` (a `BufferAttributeNode`
//! over an attribute not on the geometry), `usage`/`updateRanges`,
//! `onUpload`, `toJSON`, and `InstancedMesh` on an interleaved buffer. Each is a
//! follow-on with this module as its base.
