//! `KHR_draco_mesh_compression`: the half of `DRACOLoader` that `GLTFLoader`
//! drives, on the pure-Rust [`draco_core`] decoder.
//!
//! three.js does not ship Draco source, only an Emscripten build of Google's
//! C++ libdraco (`examples/jsm/libs/draco/`). `draco_core` is a Rust port of
//! the same library, so the bitstream half is taken from it the way PNG and
//! JPEG are taken from `png` and `zune-jpeg`. What is ported here is the part
//! three.js itself owns: the worker's `decodeGeometry` / `decodeAttribute` /
//! `decodeIndex`, and the one libdraco call they lean on for their output
//! format, `GetAttributeDataArrayForAllPoints` (with the
//! `GeometryAttribute::ConvertComponentValue` it runs per component), since
//! that is what turns Draco's stored values into the typed array
//! `GLTFLoader` asked for.
//!
//! `tests/gltf_draco.rs` checks every Draco asset in the three.js examples
//! against `DRACOLoader` itself, run under node by `tools/draco_reference.mjs`.

use draco_core::{DataType, DecoderBuffer, Mesh, MeshDecoder, PointAttribute, PointIndex};

/// A typed array, as `decodeAttribute` returns one: the constructor
/// `GLTFLoader` passed in `attributeTypes`, from the accessor's
/// `componentType`.
#[derive(Clone, Debug, PartialEq)]
pub enum DracoArray {
    I8(Vec<i8>),
    U8(Vec<u8>),
    I16(Vec<i16>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    F32(Vec<f32>),
}

impl DracoArray {
    /// The JavaScript constructor's name, `Float32Array` and so on.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::I8(_) => "Int8Array",
            Self::U8(_) => "Uint8Array",
            Self::I16(_) => "Int16Array",
            Self::U16(_) => "Uint16Array",
            Self::U32(_) => "Uint32Array",
            Self::F32(_) => "Float32Array",
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::I8(v) => v.len(),
            Self::U8(v) => v.len(),
            Self::I16(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::F32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Every element widened to `f64`, exactly.
    pub fn to_f64(&self) -> Vec<f64> {
        match self {
            Self::I8(v) => v.iter().map(|&x| x as f64).collect(),
            Self::U8(v) => v.iter().map(|&x| x as f64).collect(),
            Self::I16(v) => v.iter().map(|&x| x as f64).collect(),
            Self::U16(v) => v.iter().map(|&x| x as f64).collect(),
            Self::U32(v) => v.iter().map(|&x| x as f64).collect(),
            Self::F32(v) => v.iter().map(|&x| x as f64).collect(),
        }
    }
}

/// The element type `GLTFLoader` asks `DRACOLoader` for, by glTF
/// `componentType` (`WEBGL_COMPONENT_TYPES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DracoArrayType {
    I8,
    U8,
    I16,
    U16,
    U32,
    F32,
}

impl DracoArrayType {
    /// `WEBGL_COMPONENT_TYPES[ componentType ]`. That table has no `5124`
    /// (`INT`, which glTF does not allow), so `Int32Array`, though
    /// `getDracoDataType` knows it, is never asked for.
    pub fn from_component_type(component_type: u64) -> Option<Self> {
        Some(match component_type {
            5120 => Self::I8,
            5121 => Self::U8,
            5122 => Self::I16,
            5123 => Self::U16,
            5125 => Self::U32,
            5126 => Self::F32,
            _ => return None,
        })
    }
}

/// One attribute of a decoded primitive, as `DRACOLoader._createGeometry`
/// sets it on the geometry and `GLTFDracoMeshCompressionExtension` then
/// patches `normalized` on.
#[derive(Clone, Debug, PartialEq)]
pub struct DracoAttribute {
    /// The three.js name (`position`, `skinIndex`, `uv1`, …).
    pub name: String,
    pub item_size: usize,
    pub normalized: bool,
    /// `item_size` values per point, no padding.
    pub array: DracoArray,
}

/// A decoded primitive: `decodeGeometry`'s `{ index, attributes }`.
#[derive(Clone, Debug, PartialEq)]
pub struct DracoPrimitive {
    pub attributes: Vec<DracoAttribute>,
    /// `decodeIndex` — `GetTrianglesUInt32Array`; `None` for a point cloud.
    pub index: Option<Vec<u32>>,
}

/// What `decodeAttribute` needs to know about one requested attribute.
pub(crate) struct Request {
    pub name: String,
    pub unique_id: u32,
    pub array_type: DracoArrayType,
    /// `attributeNormalizedMap[ name ]`: the accessor's `normalized === true`.
    pub normalized: bool,
}

/// `DRACOWorker.decodeGeometry` plus the extension's `normalized` patch.
pub(crate) fn decode_primitive(
    data: &[u8],
    requests: &[Request],
) -> Result<DracoPrimitive, String> {
    // `GetEncodedGeometryType`: byte 7 of the header, 0 for a point cloud.
    // `MeshDecoder` decodes either, into a mesh that has no faces for the
    // latter; `decodeIndex` only runs for a `TRIANGULAR_MESH`.
    let is_mesh = match data.get(7) {
        Some(0) => false,
        Some(1) => true,
        _ => return Err("Unexpected geometry type.".into()),
    };

    let mut mesh = Mesh::new();
    MeshDecoder::new()
        .decode(&mut DecoderBuffer::new(data), &mut mesh)
        .map_err(|error| format!("Decoding failed: {error}"))?;

    let mut attributes = Vec::with_capacity(requests.len());
    for request in requests {
        // `GetAttributeByUniqueId`, which is `null` for an unknown id; the
        // worker then throws on `attribute.num_components()`.
        let attribute = (0..mesh.num_attributes())
            .map(|id| mesh.attribute(id))
            .find(|attribute| attribute.unique_id() == request.unique_id)
            .ok_or_else(|| format!("no attribute with unique id {}", request.unique_id))?;

        let array = attribute_data_for_all_points(&mesh, attribute, request.array_type)?;

        attributes.push(DracoAttribute {
            name: request.name.clone(),
            item_size: attribute.num_components() as usize,
            normalized: request.normalized,
            array,
        });
    }

    let index = is_mesh.then(|| {
        mesh.faces()
            .iter()
            .flat_map(|face| face.map(|point| point.0))
            .collect()
    });

    Ok(DracoPrimitive { attributes, index })
}

/// `GetAttributeDataArrayForAllPoints` (`decoder_webidl_wrapper.cc`): one
/// entry per point through the attribute's point map, each component
/// converted to the requested type by `ConvertComponentValue`.
///
/// Where libdraco reports a conversion failure the worker ignores it and
/// hands back whatever the scratch allocation held; that cannot be
/// reproduced, so here it is an error.
fn attribute_data_for_all_points(
    mesh: &Mesh,
    attribute: &PointAttribute,
    array_type: DracoArrayType,
) -> Result<DracoArray, String> {
    let num_components = attribute.num_components() as usize;
    let size = data_type_size(attribute.data_type()).ok_or("attribute has no data type")?;
    let data = attribute.buffer().data();
    let stride = attribute.byte_stride() as usize;
    let normalized = attribute.normalized();
    let data_type = attribute.data_type();
    let num_points = mesh.num_points();

    let component = |point: usize, c: usize| -> Result<Component, String> {
        let entry = attribute.mapped_index(PointIndex(point as u32)).0 as usize;
        // A decoded attribute owns its buffer, so its byte offset is 0.
        let at = entry * stride + c * size;
        let bytes = data
            .get(at..at + size)
            .ok_or("attribute value out of range")?;
        Ok(Component::read(data_type, bytes, normalized))
    };

    macro_rules! collect {
        ($variant:ident, $t:ty) => {{
            let mut out: Vec<$t> = Vec::with_capacity(num_points * num_components);
            for point in 0..num_points {
                for c in 0..num_components {
                    out.push(
                        <$t as Output>::convert(component(point, c)?).ok_or_else(|| {
                            format!(
                                "point {point} component {c} does not convert to {}",
                                stringify!($t)
                            )
                        })?,
                    );
                }
            }
            DracoArray::$variant(out)
        }};
    }

    Ok(match array_type {
        DracoArrayType::I8 => collect!(I8, i8),
        DracoArrayType::U8 => collect!(U8, u8),
        DracoArrayType::I16 => collect!(I16, i16),
        DracoArrayType::U16 => collect!(U16, u16),
        DracoArrayType::U32 => collect!(U32, u32),
        DracoArrayType::F32 => collect!(F32, f32),
    })
}

/// `DataTypeLength`.
fn data_type_size(data_type: DataType) -> Option<usize> {
    Some(match data_type {
        DataType::Int8 | DataType::Uint8 | DataType::Bool => 1,
        DataType::Int16 | DataType::Uint16 => 2,
        DataType::Int32 | DataType::Uint32 | DataType::Float32 => 4,
        DataType::Int64 | DataType::Uint64 | DataType::Float64 => 8,
        _ => return None,
    })
}

/// One stored component and the attribute's `normalized()` flag, which is
/// what `ConvertComponentValue` reads (not the glTF accessor's).
#[derive(Clone, Copy)]
struct Component {
    value: Value,
    normalized: bool,
}

#[derive(Clone, Copy)]
enum Value {
    /// A signed integer and its width in bits.
    Signed(i64, u32),
    /// An unsigned integer and its width in bits; `bool` is one bit wide
    /// (`numeric_limits<bool>::max()` is 1).
    Unsigned(u64, u32),
    F32(f32),
    F64(f64),
}

impl Component {
    fn read(data_type: DataType, bytes: &[u8], normalized: bool) -> Self {
        let mut le = [0u8; 8];
        le[..bytes.len()].copy_from_slice(bytes);
        let u = u64::from_le_bytes(le);
        let value = match data_type {
            DataType::Int8 => Value::Signed(u as u8 as i8 as i64, 8),
            DataType::Int16 => Value::Signed(u as u16 as i16 as i64, 16),
            DataType::Int32 => Value::Signed(u as u32 as i32 as i64, 32),
            DataType::Int64 => Value::Signed(u as i64, 64),
            DataType::Uint8 => Value::Unsigned(u, 8),
            DataType::Uint16 => Value::Unsigned(u, 16),
            DataType::Uint32 => Value::Unsigned(u, 32),
            DataType::Bool => Value::Unsigned((u != 0) as u64, 1),
            DataType::Float32 => Value::F32(f32::from_bits(u as u32)),
            DataType::Float64 => Value::F64(f64::from_bits(u)),
            // Uint64, and Invalid (unreachable: it has no size)
            _ => Value::Unsigned(u, 64),
        };
        Self { value, normalized }
    }
}

/// `ConvertComponentValue<T, OutT>`, per output type; `None` where it
/// returns `false`.
trait Output: Sized {
    fn convert(component: Component) -> Option<Self>;
}

macro_rules! integer_output {
    ($t:ty, $signed:expr) => {
        impl Output for $t {
            fn convert(component: Component) -> Option<Self> {
                let (min, max) = (<$t>::MIN as i128, <$t>::MAX as i128);
                match component.value {
                    Value::Signed(v, bits) => {
                        // The range check is `in_value > OutT::max()` and
                        // `in_value < OutT::min()` under C++'s usual
                        // arithmetic conversions: a signed input of at most
                        // 32 bits against a `uint32_t` bound is converted to
                        // unsigned, so the check never fails and the value
                        // wraps.
                        let wraps = !$signed && <$t>::BITS == 32 && bits <= 32;
                        if !wraps && ((v as i128) < min || (v as i128) > max) {
                            return None;
                        }
                        Some(v as $t)
                    }
                    Value::Unsigned(v, _) => ((v as i128) <= max).then_some(v as $t),
                    Value::F32(f) => {
                        // Compared in `float`, where `OutT::max()` may round
                        // up; the normalized product is `float * OutT`
                        // (a float), plus the `double` literal `0.5`.
                        if f.is_nan() || f.is_infinite() || f < min as f32 || f >= max as f32 {
                            return None;
                        }
                        if component.normalized {
                            if !(0.0..=1.0).contains(&f) {
                                return None;
                            }
                            return Some(((f * max as f32) as f64 + 0.5).floor() as $t);
                        }
                        Some(f as $t)
                    }
                    Value::F64(f) => {
                        if f.is_nan() || f.is_infinite() || f < min as f64 || f >= max as f64 {
                            return None;
                        }
                        if component.normalized {
                            if !(0.0..=1.0).contains(&f) {
                                return None;
                            }
                            return Some((f * max as f64 + 0.5).floor() as $t);
                        }
                        Some(f as $t)
                    }
                }
            }
        }
    };
}

integer_output!(i8, true);
integer_output!(u8, false);
integer_output!(i16, true);
integer_output!(u16, false);
integer_output!(u32, false);

impl Output for f32 {
    fn convert(component: Component) -> Option<Self> {
        // An integer into a float divides by `T::max()` when normalized,
        // both sides as `OutT` (`float`).
        Some(match component.value {
            Value::F32(f) => f,
            Value::F64(f) => f as f32,
            Value::Signed(v, bits) if component.normalized => {
                v as f32 / ((1u64 << (bits - 1)) - 1) as f32
            }
            Value::Unsigned(v, bits) if component.normalized => {
                v as f32 / (u64::MAX >> (64 - bits)) as f32
            }
            Value::Signed(v, _) => v as f32,
            Value::Unsigned(v, _) => v as f32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(value: Value, normalized: bool) -> Component {
        Component { value, normalized }
    }

    #[test]
    fn normalized_integers_divide_by_their_own_max() {
        assert_eq!(
            f32::convert(component(Value::Unsigned(65535, 16), true)),
            Some(1.0)
        );
        assert_eq!(
            f32::convert(component(Value::Signed(-127, 8), true)),
            Some(-1.0)
        );
        assert_eq!(
            f32::convert(component(Value::Unsigned(7, 16), false)),
            Some(7.0)
        );
    }

    #[test]
    fn integer_range_checks_follow_the_cpp_conversions() {
        assert_eq!(
            u8::convert(component(Value::Unsigned(256, 16), false)),
            None
        );
        assert_eq!(u16::convert(component(Value::Signed(-1, 16), false)), None);
        // int32 into uint32_t: compared as unsigned, never out of range
        assert_eq!(
            u32::convert(component(Value::Signed(-1, 32), false)),
            Some(u32::MAX)
        );
    }

    #[test]
    fn floats_into_normalized_integers_round() {
        assert_eq!(u8::convert(component(Value::F32(0.5), true)), Some(128));
        assert_eq!(u8::convert(component(Value::F32(1.5), true)), None);
        assert_eq!(u8::convert(component(Value::F32(255.0), false)), None);
        assert_eq!(u8::convert(component(Value::F32(254.9), false)), Some(254));
    }
}
