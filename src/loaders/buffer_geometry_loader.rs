//! Port of `three.js/src/loaders/BufferGeometryLoader.js` (rung 2 subset:
//! plain, non-interleaved `data.attributes` plus `data.index`).
//!
//! `load()` reads from the filesystem instead of the examples web server; the
//! JSON it parses is byte-identical to what the page fetches.

use std::path::Path;

use crate::core::{BufferAttribute, BufferGeometry, Index};

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
    pub fn load(&self, path: impl AsRef<Path>) -> BufferGeometry {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("three-rs: cannot read {}: {e}", path.display()));
        let json: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("three-rs: cannot parse {}: {e}", path.display()));
        self.parse(&json)
    }

    /// `BufferGeometryLoader.parse( json )`.
    pub fn parse(&self, json: &serde_json::Value) -> BufferGeometry {
        let mut geometry = BufferGeometry::new();

        let data = &json["data"];

        if let Some(index) = data.get("index") {
            let array = number_array(&index["array"]);
            geometry.set_index_attribute(match index["type"].as_str() {
                Some("Uint16Array") => Index::U16(array.iter().map(|&v| v as u16).collect()),
                Some("Uint32Array") => Index::U32(array.iter().map(|&v| v as u32).collect()),
                other => panic!("three-rs: unsupported index type {other:?}"),
            });
        }

        if let Some(attributes) = data.get("attributes").and_then(|a| a.as_object()) {
            for (key, attribute) in attributes {
                assert_eq!(
                    attribute["type"].as_str(),
                    Some("Float32Array"),
                    "three-rs: rung 2 only reads Float32Array attributes"
                );

                let item_size = attribute["itemSize"].as_u64().unwrap() as usize;
                // `getTypedArray( 'Float32Array', array )`: each JSON number is
                // parsed as an f64 and then narrowed by the Float32Array store.
                let array: Vec<f32> = number_array(&attribute["array"])
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
                    other => panic!("three-rs: unsupported attribute {other:?}"),
                }
            }
        }

        geometry
    }
}

fn number_array(value: &serde_json::Value) -> Vec<f64> {
    value
        .as_array()
        .expect("three-rs: attribute array")
        .iter()
        .map(|v| v.as_f64().expect("three-rs: attribute number"))
        .collect()
}
