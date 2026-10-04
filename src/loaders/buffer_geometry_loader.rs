//! Port of `three.js/src/loaders/BufferGeometryLoader.js`: `data.index`,
//! `data.attributes` and `data.morphAttributes` (plain, instanced and
//! interleaved, of any typed-array kind but `Float64Array`), and
//! `data.morphTargetsRelative`. Not ported: `groups`, `boundingSphere`,
//! `name`, `userData`, `usage`, `gpuType`, `isInstancedBufferGeometry`.
//!
//! `load()` reads from the filesystem instead of the examples web server; the
//! JSON it parses is byte-identical to what the page fetches.

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use crate::core::{
    ArrayKind, BufferAttribute, BufferGeometry, Index, InterleavedBuffer, TypedArray,
};
use crate::error::Error;

/// three.js' `BufferGeometryLoader`.
pub struct BufferGeometryLoader;

impl Default for BufferGeometryLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferGeometryLoader {
    /// A loader with no state — `new BufferGeometryLoader()`.
    pub fn new() -> Self {
        Self
    }

    /// `loader.load( url, onLoad )` — synchronous here, since the harness
    /// renders a single frame once loading has settled.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<BufferGeometry, Error> {
        let path = path.as_ref();
        let text = crate::io::read_to_string(path)?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|source| Error::Json {
                path: Some(path.to_path_buf()),
                source,
            })?;
        self.parse(&json)
    }

    /// `BufferGeometryLoader.parse( json )`.
    pub fn parse(&self, json: &serde_json::Value) -> Result<BufferGeometry, Error> {
        let mut geometry = BufferGeometry::new();

        let data = &json["data"];

        if let Some(index) = data.get("index") {
            let array = number_array(&index["array"])?;
            geometry.set_index_attribute(match index["type"].as_str() {
                Some("Uint16Array") => Index::U16(array.iter().map(|&v| v as u16).collect()),
                Some("Uint32Array") => Index::U32(array.iter().map(|&v| v as u32).collect()),
                other => {
                    return Err(Error::UnsupportedFormat {
                        what: "index type",
                        value: format!("{other:?}"),
                    })
                }
            });
        }

        let mut interleaved = HashMap::new();

        if let Some(attributes) = data.get("attributes").and_then(|a| a.as_object()) {
            for (key, attribute) in attributes {
                let buffer_attribute = parse_attribute(data, attribute, &mut interleaved)?;
                geometry.set_attribute(key, buffer_attribute);
            }
        }

        if let Some(morph_attributes) = data.get("morphAttributes").and_then(|a| a.as_object()) {
            for (key, attribute_array) in morph_attributes {
                let array = attribute_array
                    .as_array()
                    .ok_or_else(|| Error::UnsupportedFormat {
                        what: "morphAttributes entry",
                        value: attribute_array.to_string(),
                    })?
                    .iter()
                    .map(|attribute| parse_attribute(data, attribute, &mut interleaved))
                    .collect::<Result<Vec<_>, _>>()?;
                geometry.set_morph_attribute(key, array);
            }
        }

        if data["morphTargetsRelative"].as_bool() == Some(true) {
            geometry.morph_targets_relative = true;
        }

        Ok(geometry)
    }
}

/// One entry of `data.attributes` or `data.morphAttributes`: an
/// `InterleavedBufferAttribute` over a shared [`InterleavedBuffer`], or an
/// (instanced) `BufferAttribute` over `getTypedArray( type, array )`.
fn parse_attribute(
    data: &serde_json::Value,
    attribute: &serde_json::Value,
    interleaved: &mut HashMap<String, Rc<InterleavedBuffer>>,
) -> Result<BufferAttribute, Error> {
    let item_size = usize_field(attribute, "itemSize")?;
    let normalized = attribute["normalized"].as_bool().unwrap_or(false);

    if attribute["isInterleavedBufferAttribute"].as_bool() == Some(true) {
        let uuid = string_field(attribute, "data")?;
        let buffer = interleaved_buffer(data, uuid, interleaved)?;
        let offset = usize_field(attribute, "offset")?;
        return Ok(BufferAttribute::interleaved(
            buffer, item_size, offset, normalized,
        ));
    }

    // `getTypedArray( type, array )`: each JSON number stored the way the
    // typed array's constructor stores it.
    let kind = array_kind(&attribute["type"])?;
    let array = TypedArray::from_f64(kind, &number_array(&attribute["array"])?);
    Ok(
        if attribute["isInstancedBufferAttribute"].as_bool() == Some(true) {
            BufferAttribute::from_typed_instanced(array, item_size, normalized)
        } else {
            BufferAttribute::from_typed(array, item_size, normalized)
        },
    )
}

/// `getInterleavedBuffer( json, uuid )`: `data.interleavedBuffers[ uuid ]`,
/// a typed view of `data.arrayBuffers[ buffer ]` (stored as `Uint32Array`
/// words) with a stride, built once per uuid so every view shares it.
///
/// three also caches the `ArrayBuffer`, so two interleaved buffers over one
/// array buffer alias each other's memory; here each gets its own copy.
fn interleaved_buffer(
    data: &serde_json::Value,
    uuid: &str,
    interleaved: &mut HashMap<String, Rc<InterleavedBuffer>>,
) -> Result<Rc<InterleavedBuffer>, Error> {
    if let Some(buffer) = interleaved.get(uuid) {
        return Ok(buffer.clone());
    }
    let json = &data["interleavedBuffers"][uuid];
    if json.is_null() {
        return Err(Error::UnsupportedFormat {
            what: "interleavedBuffers uuid",
            value: uuid.to_string(),
        });
    }
    let array_buffer = &data["arrayBuffers"][string_field(json, "buffer")?];
    // `new Uint32Array( arrayBuffer ).buffer`.
    let TypedArray::U32(words) = TypedArray::from_f64(ArrayKind::U32, &number_array(array_buffer)?)
    else {
        unreachable!("from_f64(U32) is a Uint32Array")
    };
    let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    let array = TypedArray::from_le_bytes(array_kind(&json["type"])?, &bytes);
    let buffer = Rc::new(InterleavedBuffer::new(array, usize_field(json, "stride")?));
    interleaved.insert(uuid.to_string(), buffer.clone());
    Ok(buffer)
}

/// `TYPED_ARRAYS[ type ]` from three's `utils.js`, less `Float64Array`,
/// which no attribute kind holds.
fn array_kind(value: &serde_json::Value) -> Result<ArrayKind, Error> {
    Ok(match value.as_str() {
        Some("Float32Array") => ArrayKind::F32,
        Some("Int8Array") => ArrayKind::I8,
        Some("Uint8Array") => ArrayKind::U8,
        Some("Uint8ClampedArray") => ArrayKind::U8Clamped,
        Some("Int16Array") => ArrayKind::I16,
        Some("Uint16Array") => ArrayKind::U16,
        Some("Int32Array") => ArrayKind::I32,
        Some("Uint32Array") => ArrayKind::U32,
        _ => {
            return Err(Error::UnsupportedFormat {
                what: "attribute type",
                value: value.to_string(),
            })
        }
    })
}

fn usize_field(json: &serde_json::Value, what: &'static str) -> Result<usize, Error> {
    json[what]
        .as_u64()
        .map(|v| v as usize)
        .ok_or_else(|| Error::UnsupportedFormat {
            what,
            value: json[what].to_string(),
        })
}

fn string_field<'a>(json: &'a serde_json::Value, what: &'static str) -> Result<&'a str, Error> {
    json[what].as_str().ok_or_else(|| Error::UnsupportedFormat {
        what,
        value: json[what].to_string(),
    })
}

fn number_array(value: &serde_json::Value) -> Result<Vec<f64>, Error> {
    value
        .as_array()
        .ok_or_else(|| Error::UnsupportedFormat {
            what: "attribute array",
            value: value.to_string(),
        })?
        .iter()
        .map(|v| {
            v.as_f64().ok_or_else(|| Error::UnsupportedFormat {
                what: "attribute value",
                value: v.to_string(),
            })
        })
        .collect()
}
