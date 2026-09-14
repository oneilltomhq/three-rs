//! Port of `three.js/src/loaders/BufferGeometryLoader.js` (rung 2 subset:
//! plain, non-interleaved `data.attributes` plus `data.index`).
//!
//! `load()` reads from the filesystem instead of the examples web server; the
//! JSON it parses is byte-identical to what the page fetches.

use std::path::Path;

use crate::core::{BufferAttribute, BufferGeometry, Index};
use crate::error::Error;

pub struct BufferGeometryLoader;

impl Default for BufferGeometryLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferGeometryLoader {
    pub fn new() -> Self {
        Self
    }

    /// `loader.load( url, onLoad )` — synchronous here, since the harness
    /// renders a single frame once loading has settled.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<BufferGeometry, Error> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| Error::io(path, e))?;
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

        if let Some(attributes) = data.get("attributes").and_then(|a| a.as_object()) {
            for (key, attribute) in attributes {
                // Rung 2 only reads `Float32Array` attributes.
                if attribute["type"].as_str() != Some("Float32Array") {
                    return Err(Error::UnsupportedFormat {
                        what: "attribute type",
                        value: format!("{:?}", attribute["type"].as_str()),
                    });
                }

                let item_size =
                    attribute["itemSize"]
                        .as_u64()
                        .ok_or_else(|| Error::UnsupportedFormat {
                            what: "attribute itemSize",
                            value: attribute["itemSize"].to_string(),
                        })? as usize;
                // `getTypedArray( 'Float32Array', array )`: each JSON number is
                // parsed as an f64 and then narrowed by the Float32Array store.
                let array: Vec<f32> = number_array(&attribute["array"])?
                    .into_iter()
                    .map(|v| v as f32)
                    .collect();

                let buffer_attribute = BufferAttribute::new(array, item_size);

                match key.as_str() {
                    "position" => {
                        geometry.set_attribute("position", buffer_attribute);
                    }
                    "normal" => {
                        geometry.set_attribute("normal", buffer_attribute);
                    }
                    "uv" => {
                        geometry.set_attribute("uv", buffer_attribute);
                    }
                    other => {
                        return Err(Error::UnsupportedFormat {
                            what: "attribute",
                            value: other.to_string(),
                        })
                    }
                }
            }
        }

        Ok(geometry)
    }
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
