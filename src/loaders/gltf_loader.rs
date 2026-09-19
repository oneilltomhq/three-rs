//! Port of `three.js/examples/jsm/loaders/GLTFLoader.js` — the data half.
//!
//! Synchronous and `Result`-based rather than promise-based: `GLTFParser`'s
//! dependency graph is a `Promise.all` fan-out only because XHR is async, and
//! nothing here is. The traversal order is the same, so names and indices come
//! out the same.
//!
//! What is ported: GLB container, buffers/bufferViews/accessors (every component
//! type, `normalized`, `byteStride`, `sparse`), meshes/primitives into
//! [`BufferGeometry`] with three.js' attribute renaming, nodes into the
//! `Object3D` tree, skins into [`Skeleton`], animations into [`AnimationClip`].
//! Materials and images come out as data records ([`GltfMaterial`],
//! [`GltfImage`]) because the material types do not exist in this crate yet.
//!
//! Not ported (explicit TODOs): extensions (`KHR_*`, Draco, meshopt),
//! `CUBICSPLINE` interpolation (needs `GLTFCubicSplineInterpolant`), cameras,
//! primitive-key geometry deduplication and the `groups` it implies, and
//! `GLTFMeshStandardSGMaterial`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde_json::Value;

use crate::animation::{AnimationClip, InterpolationMode, KeyframeTrack, SceneResolver};
use crate::core::{BufferAttribute, BufferGeometry, Index, Node, Object3D};
use crate::error::{Error, GltfError};
use crate::loaders::TextureLoader;
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::math::{Color, Matrix4, Vector2};
use crate::objects::{Bone, Mesh, Skeleton, SkinnedMesh};
use crate::textures::{ColorSpace, MinFilter, Texture, TextureFilter, Wrapping};

/// `WEBGL_CONSTANTS` component types and `WEBGL_COMPONENT_TYPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentType {
    /// `5120` — `Int8Array`.
    Byte,
    /// `5121` — `Uint8Array`.
    UnsignedByte,
    /// `5122` — `Int16Array`.
    Short,
    /// `5123` — `Uint16Array`.
    UnsignedShort,
    /// `5125` — `Uint32Array`.
    UnsignedInt,
    /// `5126` — `Float32Array`.
    Float,
}

impl ComponentType {
    fn from_gl(value: i64) -> Result<Self, Error> {
        Ok(match value {
            5120 => Self::Byte,
            5121 => Self::UnsignedByte,
            5122 => Self::Short,
            5123 => Self::UnsignedShort,
            5125 => Self::UnsignedInt,
            5126 => Self::Float,
            other => return Err(GltfError::UnsupportedComponentType(other).into()),
        })
    }

    /// `BYTES_PER_ELEMENT`.
    pub fn byte_size(self) -> usize {
        match self {
            Self::Byte | Self::UnsignedByte => 1,
            Self::Short | Self::UnsignedShort => 2,
            Self::UnsignedInt | Self::Float => 4,
        }
    }

    /// The `normalized` divisor from `GLTFLoader`'s `getNormalizedComponentScale`.
    fn normalized_scale(self) -> f64 {
        match self {
            Self::Byte => 1.0 / 127.0,
            Self::UnsignedByte => 1.0 / 255.0,
            Self::Short => 1.0 / 32767.0,
            Self::UnsignedShort => 1.0 / 65535.0,
            // `THREE.GLTFLoader: Unsupported normalized accessor component type.`
            Self::UnsignedInt | Self::Float => 1.0,
        }
    }

    fn read(self, bytes: &[u8]) -> f64 {
        match self {
            Self::Byte => bytes[0] as i8 as f64,
            Self::UnsignedByte => bytes[0] as f64,
            Self::Short => i16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            Self::UnsignedShort => u16::from_le_bytes([bytes[0], bytes[1]]) as f64,
            Self::UnsignedInt => {
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            }
            Self::Float => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64,
        }
    }
}

/// `WEBGL_TYPE_SIZES`.
fn type_size(name: &str) -> Result<usize, Error> {
    Ok(match name {
        "SCALAR" => 1,
        "VEC2" => 2,
        "VEC3" => 3,
        "VEC4" => 4,
        "MAT2" => 4,
        "MAT3" => 9,
        "MAT4" => 16,
        other => return Err(GltfError::UnsupportedAccessorType(other.to_string()).into()),
    })
}

/// `ATTRIBUTES` — glTF semantic to three.js attribute name. An unlisted
/// semantic falls through to its lowercased name, as in `GLTFLoader`.
fn attribute_name(semantic: &str) -> String {
    match semantic {
        "POSITION" => "position".into(),
        "NORMAL" => "normal".into(),
        "TANGENT" => "tangent".into(),
        "TEXCOORD_0" => "uv".into(),
        "TEXCOORD_1" => "uv1".into(),
        "TEXCOORD_2" => "uv2".into(),
        "TEXCOORD_3" => "uv3".into(),
        "COLOR_0" => "color".into(),
        "WEIGHTS_0" => "skinWeight".into(),
        "JOINTS_0" => "skinIndex".into(),
        other => other.to_lowercase(),
    }
}

/// `PATH_PROPERTIES` — glTF animation target path to three.js property name.
fn path_property(path: &str) -> Option<&'static str> {
    match path {
        "scale" => Some("scale"),
        "translation" => Some("position"),
        "rotation" => Some("quaternion"),
        "weights" => Some("morphTargetInfluences"),
        _ => None,
    }
}

/// `PropertyBinding.sanitizeNodeName`.
pub fn sanitize_node_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .filter(|c| !matches!(c, '[' | ']' | '.' | ':' | '/'))
        .collect()
}

/// The PBR half of a glTF material, kept as data until the crate has material
/// types. Field names follow `materials[ i ]` in the glTF JSON.
#[derive(Clone, Debug, Default)]
pub struct GltfMaterial {
    pub name: String,
    /// `pbrMetallicRoughness.baseColorFactor`, defaulting to `[1,1,1,1]`.
    pub base_color_factor: [f64; 4],
    /// `pbrMetallicRoughness.baseColorTexture.index`.
    pub base_color_texture: Option<GltfTextureRef>,
    pub metallic_factor: f64,
    pub roughness_factor: f64,
    pub metallic_roughness_texture: Option<GltfTextureRef>,
    pub normal_texture: Option<GltfTextureRef>,
    pub normal_scale: f64,
    pub occlusion_texture: Option<GltfTextureRef>,
    pub emissive_texture: Option<GltfTextureRef>,
    pub emissive_factor: [f64; 3],
    /// `'OPAQUE'` / `'MASK'` / `'BLEND'`.
    pub alpha_mode: String,
    pub alpha_cutoff: f64,
    pub double_sided: bool,
    /// Extension names present on this material, so the rung worker can see what
    /// it is missing.
    pub extensions: Vec<String>,
    /// `KHR_materials_ior.ior`, defaulting to three.js' `1.5` when the
    /// extension is absent — its presence is what promotes the material to a
    /// `MeshPhysicalMaterial`.
    pub ior: Option<f64>,
    /// `KHR_materials_specular.specularFactor` (default 1).
    pub specular_factor: Option<f64>,
    /// `KHR_materials_specular.specularColorFactor` (default `[1,1,1]`).
    pub specular_color_factor: [f64; 3],
    /// `KHR_materials_specular.specularColorTexture`.
    pub specular_color_texture: Option<GltfTextureRef>,
    /// `KHR_materials_sheen.sheenColorFactor` and `.sheenRoughnessFactor`.
    /// `Some` is the extension being present at all, which is what
    /// `GLTFMaterialsSheenExtension.getMaterialType()` promotes the material to
    /// a `MeshPhysicalMaterial` for — and what sets `sheen = 1`, since glTF has
    /// no intensity of its own.
    pub sheen: Option<GltfSheen>,
}

/// `KHR_materials_sheen`'s two factors, with the extension's own defaults
/// (`[ 0, 0, 0 ]` and `0`) already applied.
#[derive(Clone, Copy, Debug)]
pub struct GltfSheen {
    pub color_factor: [f64; 3],
    pub roughness_factor: f64,
}

/// One glTF *texture reference* — a `{ index, texCoord, extensions }` object on
/// a material, as distinct from the texture it points at. Two materials can
/// point at one texture through different references and get different
/// `Texture`s out, which is exactly what `KHR_texture_transform` is for.
#[derive(Clone, Debug)]
pub struct GltfTextureRef {
    pub index: usize,
    /// `textureDef.texCoord` — which `TEXCOORD_n` set to sample along.
    pub tex_coord: Option<usize>,
    /// `textureDef.extensions.KHR_texture_transform`.
    pub transform: Option<GltfTextureTransform>,
}

/// `KHR_texture_transform`. Every field is optional in the spec and three
/// branches on each being `undefined`, so they stay `Option` here: an absent
/// field is not the same as its default once
/// `GLTFTextureTransformExtension.extendTexture()`'s no-op test is involved.
#[derive(Clone, Copy, Debug, Default)]
pub struct GltfTextureTransform {
    pub offset: Option<[f64; 2]>,
    pub rotation: Option<f64>,
    pub scale: Option<[f64; 2]>,
    pub tex_coord: Option<usize>,
}

impl GltfTextureRef {
    /// A `{ index, texCoord, extensions }` object, or `None` when the material
    /// has no such reference.
    fn parse(def: Option<&Value>) -> Option<Self> {
        let def = def?;
        let index = def.get("index").and_then(Value::as_u64)? as usize;
        let transform =
            def.pointer("/extensions/KHR_texture_transform")
                .map(|t| GltfTextureTransform {
                    offset: pair(t.get("offset")),
                    rotation: t.get("rotation").and_then(Value::as_f64),
                    scale: pair(t.get("scale")),
                    tex_coord: t
                        .get("texCoord")
                        .and_then(Value::as_u64)
                        .map(|v| v as usize),
                });
        Some(Self {
            index,
            tex_coord: def
                .get("texCoord")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
            transform,
        })
    }
}

/// A two-element JSON number array.
fn pair(value: Option<&Value>) -> Option<[f64; 2]> {
    let array = value?.as_array()?;
    Some([
        array.first().and_then(Value::as_f64)?,
        array.get(1).and_then(Value::as_f64)?,
    ])
}

/// `assignFinalMaterial`'s cache key —
/// `'ClonedMaterial:' + uuid + ':derivative-tangents:vertex-colors:flat-shading:'`
/// with the glTF material index standing in for the uuid, since the record is
/// the only thing this loader has to clone from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MaterialVariant {
    material: usize,
    use_derivative_tangents: bool,
    use_vertex_colors: bool,
    use_flat_shading: bool,
}

/// One glTF image. Decoding is the [`crate::loaders::TextureLoader`]'s job; this
/// record carries the bytes and where they came from.
#[derive(Clone, Debug)]
pub struct GltfImage {
    pub name: String,
    /// `images[ i ].mimeType`, when the glTF says.
    pub mime_type: Option<String>,
    /// `images[ i ].uri`, for an external image.
    pub uri: Option<String>,
    /// The raw bytes, from the BIN chunk or the external file.
    pub data: Vec<u8>,
}

/// One glTF texture: which image, which sampler.
#[derive(Clone, Copy, Debug, Default)]
pub struct GltfTexture {
    pub source: Option<usize>,
    pub sampler: Option<usize>,
}

/// One drawable produced by a glTF primitive. `node` is the scene-graph node it
/// hangs off — the node itself when the mesh has one primitive, a child named
/// `<name>_<i>` when three.js splits it.
pub struct GltfPrimitive {
    pub node: Node,
    pub geometry: Rc<BufferGeometry>,
    /// Index into [`Gltf::materials`].
    pub material: Option<usize>,
    /// Index into [`Gltf::skins`], set when the glTF node has a `skin`.
    pub skin: Option<usize>,
    /// `mesh.morphTargetInfluences`, from `meshDef.weights` / `primitive.targets`.
    /// Shared with the `SceneResolver` that binds the morph tracks.
    pub morph_target_influences: Rc<RefCell<Vec<f64>>>,
    /// `mesh.morphTargetDictionary`, from `meshDef.extras.targetNames`.
    pub morph_target_dictionary: Vec<(String, usize)>,
}

/// The result of a load: `GLTF` as `GLTFLoader.parse`'s callback sees it, plus
/// the side tables this crate needs because `Object3D` carries no geometry.
pub struct Gltf {
    /// `gltf.scene` — `scenes[ json.scene || 0 ]`.
    pub scene: Node,
    /// `gltf.scenes`.
    pub scenes: Vec<Node>,
    /// One [`Node`] per glTF node index, in glTF order.
    pub nodes: Vec<Node>,
    /// `gltf.animations`.
    pub animations: Vec<AnimationClip>,
    /// One [`Skeleton`] per glTF skin index.
    pub skins: Vec<Rc<RefCell<Skeleton>>>,
    /// Every primitive, in node order.
    pub primitives: Vec<GltfPrimitive>,
    /// The skinned primitives' nodes, bound to their skeletons. Each carries a
    /// [`Payload::SkinnedMesh`](crate::objects::Payload::SkinnedMesh), so
    /// adding `gltf.scene` to a scene is enough for the renderer to draw them.
    pub skinned_meshes: Vec<Node>,
    pub materials: Vec<GltfMaterial>,
    pub textures: Vec<GltfTexture>,
    pub images: Vec<GltfImage>,
    /// `gltf.asset`.
    pub asset: Value,
    /// The parsed JSON, for anything not yet ported.
    pub json: Value,
}

impl Gltf {
    /// The [`SceneResolver`] for this glTF: the scene as the animation root,
    /// with every primitive's `morphTargetInfluences` registered, so
    /// `mixer.clipAction( &gltf.animations[ 0 ], None, None )` writes into the
    /// tree.
    pub fn scene_resolver(&self) -> SceneResolver {
        let mut resolver = SceneResolver::new(self.scene.clone());

        for primitive in &self.primitives {
            resolver.add_morph_target_influences(
                &primitive.node,
                primitive.morph_target_influences.clone(),
            );
        }

        resolver
    }

    /// The primitive hanging off a given scene-graph node, if any — the bridge
    /// the renderer needs, because `Object3D` carries no geometry.
    pub fn primitive(&self, node: &Node) -> Option<&GltfPrimitive> {
        self.primitives
            .iter()
            .find(|primitive| Node::ptr_eq(&primitive.node, node))
    }
}

/// Port of `GLTFLoader` + `GLTFParser`.
pub struct GLTFLoader {
    json: Value,
    /// The GLB `BIN` chunk, when there is one.
    glb_buffer: Option<Vec<u8>>,
    /// `buffers[ i ]`, resolved.
    buffers: Vec<Vec<u8>>,
    /// The directory the .gltf/.glb sits in, for relative URIs.
    base: PathBuf,
    /// `GLTFParser.nodeNamesUsed`.
    node_names_used: HashMap<String, usize>,
}

impl GLTFLoader {
    /// `loader.load( url )`, synchronously: read the file and parse it.
    pub fn load(path: impl AsRef<Path>) -> Result<Gltf, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| Error::io(path, e))?;
        let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Self::parse(&data, base)
    }

    /// `loader.parse( data, path )`. Sniffs the GLB magic the way
    /// `GLTFLoader.parse` does.
    pub fn parse(data: &[u8], base: PathBuf) -> Result<Gltf, Error> {
        let (json, glb_buffer) = if data.len() >= 4 && &data[0..4] == b"glTF" {
            let (json, bin) = parse_glb(data)?;
            (json, bin)
        } else {
            let text = std::str::from_utf8(data).map_err(|_| GltfError::NotUtf8)?;
            let json: Value =
                serde_json::from_str(text).map_err(|source| Error::Json { path: None, source })?;
            (json, None)
        };

        if let Some(version) = json
            .pointer("/asset/version")
            .and_then(Value::as_str)
            .and_then(|v| v.split('.').next().map(str::to_string))
        {
            if version != "2" {
                return Err(GltfError::UnsupportedVersion(version).into());
            }
        }

        let mut loader = Self {
            json,
            glb_buffer,
            buffers: Vec::new(),
            base,
            node_names_used: HashMap::new(),
        };

        loader.load_buffers()?;
        loader.build()
    }

    // --- buffers, buffer views, accessors ---------------------------------

    /// `GLTFParser.loadBuffer`.
    fn load_buffers(&mut self) -> Result<(), Error> {
        let buffers = self
            .json
            .get("buffers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        for buffer in &buffers {
            match buffer.get("uri").and_then(Value::as_str) {
                None => {
                    // the GLB BIN chunk
                    let bin = self.glb_buffer.clone().ok_or(GltfError::NoBinChunk)?;
                    self.buffers.push(bin);
                }
                Some(uri) if uri.starts_with("data:") => {
                    self.buffers.push(decode_data_uri(uri)?);
                }
                Some(uri) => {
                    let path = self.base.join(uri);
                    self.buffers
                        .push(std::fs::read(&path).map_err(|e| Error::io(&path, e))?);
                }
            }
        }

        Ok(())
    }

    /// `GLTFParser.loadBufferView`.
    fn buffer_view(&self, index: usize) -> Result<&[u8], Error> {
        let view =
            self.json
                .pointer(&format!("/bufferViews/{index}"))
                .ok_or(GltfError::MissingIndex {
                    kind: "bufferView",
                    index,
                })?;

        let buffer = json_usize(view, "buffer").unwrap_or(0);
        let offset = json_usize(view, "byteOffset").unwrap_or(0);
        let length = json_usize(view, "byteLength").unwrap_or(0);

        let buffer = self.buffers.get(buffer).ok_or(GltfError::MissingIndex {
            kind: "buffer",
            index: buffer,
        })?;

        Ok(&buffer[offset..offset + length])
    }

    /// `bufferViews[ i ].byteStride`.
    fn buffer_view_stride(&self, index: usize) -> Option<usize> {
        self.json
            .pointer(&format!("/bufferViews/{index}"))
            .and_then(|view| json_usize(view, "byteStride"))
    }

    /// `GLTFParser.loadAccessor`, as f64s: `itemSize`, `normalized`,
    /// `byteStride` (interleaved views included) and `sparse` all applied.
    pub fn accessor(&self, index: usize) -> Result<(Vec<f64>, usize), Error> {
        let accessor = self
            .json
            .pointer(&format!("/accessors/{index}"))
            .ok_or(GltfError::MissingIndex {
                kind: "accessor",
                index,
            })?
            .clone();

        let item_size = type_size(accessor.get("type").and_then(Value::as_str).ok_or(
            GltfError::MissingField {
                what: "accessor type",
            },
        )?)?;
        let component_type = ComponentType::from_gl(
            accessor
                .get("componentType")
                .and_then(Value::as_i64)
                .ok_or(GltfError::MissingField {
                    what: "accessor componentType",
                })?,
        )?;
        let count = json_usize(&accessor, "count").unwrap_or(0);
        let normalized = accessor
            .get("normalized")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let mut values = vec![0.0; count * item_size];

        // `// Ignore empty accessors, which may be used to declare runtime
        // information about attributes coming from another source (e.g. Draco
        // compression extension).`
        if let Some(view_index) = json_usize(&accessor, "bufferView") {
            let bytes = self.buffer_view(view_index)?;
            let element_size = component_type.byte_size();
            let default_stride = element_size * item_size;
            let stride = self
                .buffer_view_stride(view_index)
                .unwrap_or(default_stride);
            let offset = json_usize(&accessor, "byteOffset").unwrap_or(0);

            for i in 0..count {
                for c in 0..item_size {
                    let at = offset + i * stride + c * element_size;
                    values[i * item_size + c] = component_type.read(&bytes[at..at + element_size]);
                }
            }
        }

        // `// Sparse accessors`
        if let Some(sparse) = accessor.get("sparse") {
            let sparse_count = json_usize(sparse, "count").unwrap_or(0);

            let indices_type = ComponentType::from_gl(
                sparse
                    .pointer("/indices/componentType")
                    .and_then(Value::as_i64)
                    .ok_or(GltfError::MissingField {
                        what: "sparse indices componentType",
                    })?,
            )?;
            let indices_view = sparse
                .pointer("/indices/bufferView")
                .and_then(Value::as_u64)
                .ok_or(GltfError::MissingField {
                    what: "sparse indices bufferView",
                })? as usize;
            let indices_offset = sparse
                .pointer("/indices/byteOffset")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let values_view = sparse
                .pointer("/values/bufferView")
                .and_then(Value::as_u64)
                .ok_or(GltfError::MissingField {
                    what: "sparse values bufferView",
                })? as usize;
            let values_offset = sparse
                .pointer("/values/byteOffset")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;

            let index_bytes = self.buffer_view(indices_view)?.to_vec();
            let value_bytes = self.buffer_view(values_view)?.to_vec();
            let index_size = indices_type.byte_size();
            let value_size = component_type.byte_size();

            for i in 0..sparse_count {
                let at = indices_offset + i * index_size;
                let target = indices_type.read(&index_bytes[at..at + index_size]) as usize;

                for c in 0..item_size {
                    let at = values_offset + (i * item_size + c) * value_size;
                    values[target * item_size + c] =
                        component_type.read(&value_bytes[at..at + value_size]);
                }
            }
        }

        if normalized {
            let scale = component_type.normalized_scale();
            for value in &mut values {
                *value *= scale;
            }
        }

        Ok((values, item_size))
    }

    /// An accessor as a [`BufferAttribute`].
    pub fn attribute(&self, index: usize) -> Result<BufferAttribute, Error> {
        let (values, item_size) = self.accessor(index)?;
        Ok(BufferAttribute::new(
            values.iter().map(|&v| v as f32).collect(),
            item_size,
        ))
    }

    /// An accessor as a `BufferGeometry` index, in the width the accessor
    /// declares.
    ///
    /// `loadAccessor` builds the typed array from `componentType` and nothing
    /// else, so an `UNSIGNED_INT` index stays a `Uint32Array` however small its
    /// values are — `PrimaryIonDrive.glb` has 45 022 vertices and still indexes
    /// them with 32-bit indices. Narrowing "when it fits" would be invisible on
    /// screen and wrong against three's own parse, which is what
    /// `tests/gltf_primary_ion_drive.rs` reads.
    fn index_attribute(&self, index: usize) -> Result<Index, Error> {
        let (values, _) = self.accessor(index)?;
        let component_type = self
            .json
            .pointer(&format!("/accessors/{index}/componentType"))
            .and_then(Value::as_i64)
            .map(ComponentType::from_gl)
            .transpose()?;

        Ok(match component_type {
            Some(ComponentType::UnsignedInt) => {
                Index::U32(values.iter().map(|&v| v as u32).collect())
            }
            _ => Index::U16(values.iter().map(|&v| v as u16).collect()),
        })
    }

    // --- the build ---------------------------------------------------------

    fn build(&mut self) -> Result<Gltf, Error> {
        let materials = self.load_materials();
        let textures = self.load_textures();
        let images = self.load_images()?;

        // `GLTFParser.loadNode` / `_loadNodeShallow`: every node first, so the
        // parent/child wiring and the skin joints can look them up by index.
        let node_defs = self
            .json
            .get("nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        // which nodes are skin joints — those become `Bone`s
        let mut joints = vec![false; node_defs.len()];
        if let Some(skins) = self.json.get("skins").and_then(Value::as_array) {
            for skin in skins {
                for joint in skin
                    .get("joints")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[])
                {
                    if let Some(joint) = joint.as_u64() {
                        joints[joint as usize] = true;
                    }
                }
            }
        }

        let mut nodes = Vec::with_capacity(node_defs.len());
        for (index, node_def) in node_defs.iter().enumerate() {
            // `_loadNodeShallow`: `nodeDef.name ? createUniqueName( nodeDef.name
            // ) : ''`. An unnamed node keeps `''` *and does not reserve a name*,
            // so a file with several of them — this one has three — does not get
            // `''`, `'_1'`, `'_2'`.
            let name = match node_def.get("name").and_then(Value::as_str) {
                Some(name) if !name.is_empty() => self.create_unique_name(Some(name)),
                _ => String::new(),
            };

            let node = if joints[index] {
                Bone::new()
            } else {
                Object3D::new_node()
            };

            {
                let mut object = node.borrow_mut();
                object.name = name;
                apply_node_transform(&mut object, node_def);
            }

            nodes.push(node);
        }

        // parent/child wiring
        for (index, node_def) in node_defs.iter().enumerate() {
            for child in node_def
                .get("children")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                if let Some(child) = child.as_u64() {
                    nodes[index].add(&nodes[child as usize]);
                }
            }
        }

        // skins
        let mut skins = Vec::new();
        for skin_def in self
            .json
            .get("skins")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let inverse_bind_matrices = match json_usize(&skin_def, "inverseBindMatrices") {
                Some(accessor) => Some(self.accessor(accessor)?.0),
                None => None,
            };

            let mut bones = Vec::new();
            let mut bone_inverses = Vec::new();

            for (i, joint) in skin_def
                .get("joints")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
                .iter()
                .enumerate()
            {
                let Some(joint) = joint.as_u64().map(|j| j as usize) else {
                    continue;
                };
                // `console.warn( 'THREE.GLTFLoader: Joint "%s" could not be found.' )`
                let Some(node) = nodes.get(joint) else {
                    continue;
                };

                bones.push(node.clone());

                let mut matrix = Matrix4::identity();
                if let Some(values) = &inverse_bind_matrices {
                    matrix.from_array(values, i * 16);
                }
                bone_inverses.push(matrix);
            }

            skins.push(Rc::new(RefCell::new(Skeleton::new(
                bones,
                Some(bone_inverses),
            ))));
        }

        // meshes
        let mut primitives = Vec::new();
        for (index, node_def) in node_defs.iter().enumerate() {
            let Some(mesh) = json_usize(node_def, "mesh") else {
                continue;
            };
            let skin = json_usize(node_def, "skin");
            primitives.extend(self.load_mesh(mesh, &nodes[index], skin)?);
        }

        // `SkinnedMesh.bind`: the skeletons' bind matrices come from the node's
        // world matrix, so the tree has to be up to date first.
        let scenes = self.load_scenes(&nodes);
        for scene in &scenes {
            scene.update_matrix_world(true);
        }

        let mut skinned_meshes = Vec::new();
        let mut texture_cache: HashMap<usize, Texture> = HashMap::new();
        // `GLTFParser.cache`'s `'ClonedMaterial:<uuid>:<variant>'` entries, keyed
        // on the same three flags `assignFinalMaterial` spells into that string.
        let mut material_cache: HashMap<MaterialVariant, MeshBasicNodeMaterial> = HashMap::new();

        for primitive in &primitives {
            // `GLTFParser.loadMaterial`, then `assignFinalMaterial`'s variant.
            let material = self.final_material(
                &mut texture_cache,
                &mut material_cache,
                primitive,
                &materials,
                &textures,
                &images,
            )?;

            let node = primitive.node.clone();

            let Some(skin) = primitive.skin else {
                // `createNodeMesh` → `loadMesh`'s `new Mesh( geometry, material
                // )`. three.js builds the mesh and `_loadNodeShallow` then
                // *becomes* it (`objects.length === 1` → `node = objects[ 0 ]`,
                // with `nodeDef.name` overriding the mesh's own name), so the
                // payload goes on the node that already exists here, exactly as
                // the skinned branch below does.
                let mut mesh = Mesh::of(primitive.geometry.clone(), material);
                mesh.morph_target_influences = primitive.morph_target_influences.borrow().clone();

                let mut object = node.borrow_mut();
                object.object_type = "Mesh";
                object.payload = crate::objects::Payload::Mesh(mesh);
                continue;
            };

            // The primitive's node *becomes* the `SkinnedMesh`: three.js
            // constructs one and puts it in the tree, and here the tree node
            // already exists (the skin's joints and the animation bindings
            // point at it), so the payload is installed on it.
            let mut mesh = SkinnedMesh::of(primitive.geometry.clone(), material);
            mesh.morph_target_influences = primitive.morph_target_influences.clone();
            mesh.morph_target_dictionary = primitive.morph_target_dictionary.clone();
            mesh.normalize_skin_weights();

            {
                let mut object = node.borrow_mut();
                object.object_type = "SkinnedMesh";
                object.payload = crate::objects::Payload::SkinnedMesh(Box::new(mesh));
            }

            // `mesh.bind( skeleton, _identityMatrix )`: glTF joint transforms
            // are already relative to the skin, so the bind matrix is identity
            // and it is `bindMatrixInverse` (tracked from `matrixWorld`, the
            // `AttachedBindMode` default) that does the work.
            SkinnedMesh::bind(&node, skins[skin].clone(), Some(Matrix4::identity()));

            skinned_meshes.push(node);
        }

        let animations = self.load_animations(&nodes)?;

        let scene_index = json_usize(&self.json, "scene").unwrap_or(0);
        let scene = scenes
            .get(scene_index)
            .cloned()
            .ok_or(GltfError::MissingField { what: "scenes" })?;

        Ok(Gltf {
            scene,
            scenes,
            nodes,
            animations,
            skins,
            primitives,
            skinned_meshes,
            materials,
            textures,
            images,
            asset: self.json.get("asset").cloned().unwrap_or(Value::Null),
            json: self.json.clone(),
        })
    }

    /// `GLTFParser.createUniqueName`.
    fn create_unique_name(&mut self, original: Option<&str>) -> String {
        let sanitized = sanitize_node_name(original.unwrap_or(""));

        match self.node_names_used.get_mut(&sanitized) {
            Some(count) => {
                *count += 1;
                format!("{sanitized}_{count}")
            }
            None => {
                self.node_names_used.insert(sanitized.clone(), 1);
                sanitized
            }
        }
    }

    /// `GLTFParser.loadScene`.
    fn load_scenes(&self, nodes: &[Node]) -> Vec<Node> {
        let mut scenes = Vec::new();

        for scene_def in self
            .json
            .get("scenes")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let scene = Object3D::new_node();
            if let Some(name) = scene_def.get("name").and_then(Value::as_str) {
                scene.borrow_mut().name = sanitize_node_name(name);
            }
            scene.borrow_mut().object_type = "Group";

            for node in scene_def
                .get("nodes")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                if let Some(node) = node.as_u64().and_then(|i| nodes.get(i as usize)) {
                    scene.add(node);
                }
            }

            scenes.push(scene);
        }

        scenes
    }

    /// `GLTFParser.loadMesh` + `loadGeometries` + `addPrimitiveAttributes`.
    fn load_mesh(
        &self,
        index: usize,
        node: &Node,
        skin: Option<usize>,
    ) -> Result<Vec<GltfPrimitive>, Error> {
        let mesh_def = self
            .json
            .pointer(&format!("/meshes/{index}"))
            .ok_or(GltfError::MissingIndex {
                kind: "mesh",
                index,
            })?
            .clone();

        let primitive_defs = mesh_def
            .get("primitives")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let weights: Vec<f64> = mesh_def
            .get("weights")
            .and_then(Value::as_array)
            .map(|w| w.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default();

        let target_names: Vec<String> = mesh_def
            .pointer("/extras/targetNames")
            .and_then(Value::as_array)
            .map(|names| {
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let mut out = Vec::new();
        let mesh_name = node.borrow().name.clone();

        for (i, primitive) in primitive_defs.iter().enumerate() {
            let mut geometry = BufferGeometry::new();

            for (semantic, accessor) in primitive
                .get("attributes")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default()
            {
                let Some(accessor) = accessor.as_u64() else {
                    continue;
                };
                let name = attribute_name(&semantic);
                let mut attribute = self.attribute(accessor as usize)?;
                // `JOINTS_0` is an unnormalized `Uint8`/`Uint16` accessor and
                // stays one in three.js all the way to
                // `WebGPUAttributeUtils.createAttribute()`; `skinning()` reads
                // it as a `uvec4`.
                if name == "skinIndex" {
                    let values = attribute.array().clone();
                    attribute = BufferAttribute::new_integer(values, attribute.item_size);
                }
                geometry.set_attribute(&name, attribute);
            }

            if let Some(accessor) = json_usize(primitive, "indices") {
                geometry.index = Some(self.index_attribute(accessor)?);
            }

            // `// Relative morph targets`
            let targets = primitive
                .get("targets")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            if !targets.is_empty() {
                geometry.morph_targets_relative = true;

                for semantic in ["POSITION", "NORMAL", "TANGENT"] {
                    if !targets.iter().any(|target| target.get(semantic).is_some()) {
                        continue;
                    }

                    let mut attributes = Vec::new();
                    for target in &targets {
                        match json_usize(target, semantic) {
                            Some(accessor) => attributes.push(self.attribute(accessor)?),
                            // a target missing this semantic contributes zeroes
                            None => {
                                let item_size = match semantic {
                                    "TANGENT" => 4,
                                    _ => 3,
                                };
                                let count = geometry
                                    .get_attribute(&attribute_name(semantic))
                                    .map(BufferAttribute::count)
                                    .unwrap_or(0);
                                attributes.push(BufferAttribute::new(
                                    vec![0.0; count * item_size],
                                    item_size,
                                ));
                            }
                        }
                    }

                    geometry.set_morph_attribute(&attribute_name(semantic), attributes);
                }
            }

            // the node itself for a single primitive, a named child otherwise
            let primitive_node = if primitive_defs.len() == 1 {
                node.clone()
            } else {
                let child = Object3D::new_node();
                child.borrow_mut().name = format!("{mesh_name}_{i}");
                node.add(&child);
                child
            };

            let morph_target_influences = if targets.is_empty() {
                Vec::new()
            } else if weights.is_empty() {
                vec![0.0; targets.len()]
            } else {
                weights.clone()
            };

            out.push(GltfPrimitive {
                node: primitive_node,
                geometry: Rc::new(geometry),
                material: json_usize(primitive, "material"),
                skin,
                morph_target_influences: Rc::new(RefCell::new(morph_target_influences)),
                morph_target_dictionary: target_names
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(i, name)| (name, i))
                    .collect(),
            });
        }

        Ok(out)
    }

    /// `GLTFParser.loadMaterial`, as a data record.
    fn load_materials(&self) -> Vec<GltfMaterial> {
        let mut materials = Vec::new();

        for material_def in self
            .json
            .get("materials")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let pbr = material_def.get("pbrMetallicRoughness");

            let base_color_factor = pbr
                .and_then(|pbr| pbr.get("baseColorFactor"))
                .and_then(Value::as_array)
                .map(|v| {
                    let mut out = [1.0; 4];
                    for (i, slot) in out.iter_mut().enumerate() {
                        if let Some(value) = v.get(i).and_then(Value::as_f64) {
                            *slot = value;
                        }
                    }
                    out
                })
                .unwrap_or([1.0; 4]);

            let emissive_factor = material_def
                .get("emissiveFactor")
                .and_then(Value::as_array)
                .map(|v| {
                    let mut out = [0.0; 3];
                    for (i, slot) in out.iter_mut().enumerate() {
                        if let Some(value) = v.get(i).and_then(Value::as_f64) {
                            *slot = value;
                        }
                    }
                    out
                })
                .unwrap_or([0.0; 3]);

            materials.push(GltfMaterial {
                name: material_def
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                base_color_factor,
                base_color_texture: GltfTextureRef::parse(
                    pbr.and_then(|pbr| pbr.get("baseColorTexture")),
                ),
                metallic_factor: pbr
                    .and_then(|pbr| pbr.get("metallicFactor"))
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                roughness_factor: pbr
                    .and_then(|pbr| pbr.get("roughnessFactor"))
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                metallic_roughness_texture: GltfTextureRef::parse(
                    pbr.and_then(|pbr| pbr.get("metallicRoughnessTexture")),
                ),
                normal_texture: GltfTextureRef::parse(material_def.get("normalTexture")),
                normal_scale: material_def
                    .pointer("/normalTexture/scale")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                occlusion_texture: GltfTextureRef::parse(material_def.get("occlusionTexture")),
                emissive_texture: GltfTextureRef::parse(material_def.get("emissiveTexture")),
                emissive_factor,
                alpha_mode: material_def
                    .get("alphaMode")
                    .and_then(Value::as_str)
                    .unwrap_or("OPAQUE")
                    .to_string(),
                alpha_cutoff: material_def
                    .get("alphaCutoff")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5),
                double_sided: material_def
                    .get("doubleSided")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                extensions: material_def
                    .get("extensions")
                    .and_then(Value::as_object)
                    .map(|map| map.keys().cloned().collect())
                    .unwrap_or_default(),
                // `GLTFMaterialsIor.extendParams`
                ior: material_def
                    .pointer("/extensions/KHR_materials_ior/ior")
                    .and_then(Value::as_f64),
                // `GLTFMaterialsSpecular.extendParams`
                specular_factor: material_def
                    .pointer("/extensions/KHR_materials_specular")
                    .map(|specular| {
                        specular
                            .get("specularFactor")
                            .and_then(Value::as_f64)
                            .unwrap_or(1.0)
                    }),
                specular_color_factor: material_def
                    .pointer("/extensions/KHR_materials_specular/specularColorFactor")
                    .and_then(Value::as_array)
                    .map(|v| {
                        let mut out = [1.0; 3];
                        for (i, slot) in out.iter_mut().enumerate() {
                            if let Some(value) = v.get(i).and_then(Value::as_f64) {
                                *slot = value;
                            }
                        }
                        out
                    })
                    .unwrap_or([1.0; 3]),
                specular_color_texture: GltfTextureRef::parse(
                    material_def.pointer("/extensions/KHR_materials_specular/specularColorTexture"),
                ),
                // `GLTFMaterialsSheenExtension.extendMaterialParams`, which
                // starts from `sheenColor = black`, `sheenRoughness = 0` and
                // `sheen = 1` before reading the two factors.
                sheen: material_def
                    .pointer("/extensions/KHR_materials_sheen")
                    .map(|sheen| GltfSheen {
                        color_factor: sheen
                            .get("sheenColorFactor")
                            .and_then(Value::as_array)
                            .map(|v| {
                                let mut out = [0.0; 3];
                                for (i, slot) in out.iter_mut().enumerate() {
                                    if let Some(value) = v.get(i).and_then(Value::as_f64) {
                                        *slot = value;
                                    }
                                }
                                out
                            })
                            .unwrap_or([0.0; 3]),
                        roughness_factor: sheen
                            .get("sheenRoughnessFactor")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0),
                    }),
            });
        }

        materials
    }

    /// `GLTFParser.loadTexture`, as a data record.
    fn load_textures(&self) -> Vec<GltfTexture> {
        self.json
            .get("textures")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .map(|texture| GltfTexture {
                source: json_usize(texture, "source"),
                sampler: json_usize(texture, "sampler"),
            })
            .collect()
    }

    /// `GLTFParser.loadImageSource`: the bytes, not the decoded texture.
    fn load_images(&self) -> Result<Vec<GltfImage>, Error> {
        let mut images = Vec::new();

        for image_def in self
            .json
            .get("images")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let uri = image_def
                .get("uri")
                .and_then(Value::as_str)
                .map(str::to_string);

            let data = match (&uri, json_usize(&image_def, "bufferView")) {
                (Some(uri), _) if uri.starts_with("data:") => decode_data_uri(uri)?,
                (Some(uri), _) => std::fs::read(self.base.join(uri)).unwrap_or_default(),
                (None, Some(view)) => self.buffer_view(view)?.to_vec(),
                (None, None) => Vec::new(),
            };

            images.push(GltfImage {
                name: image_def
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                mime_type: image_def
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                uri,
                data,
            });
        }

        Ok(images)
    }

    /// `GLTFParser.loadTexture` + the sampler half of `assignTexture`: the
    /// decoded [`Texture`] for a glTF texture index.
    ///
    /// three.js caches by texture index inside `getDependency( 'texture', i )`,
    /// so one `Texture` object is shared by every slot that names the index —
    /// Michelle's ORM map reaches `metalnessMap` and `roughnessMap` as the same
    /// object, and the renderer therefore builds one mip chain, not two. The
    /// colour space is set by the *slot*, last write winning, exactly as
    /// `assignTexture( ..., colorSpace )` does.
    fn load_texture(
        &self,
        cache: &mut HashMap<usize, Texture>,
        textures: &[GltfTexture],
        images: &[GltfImage],
        index: usize,
    ) -> Result<Option<Texture>, Error> {
        if let Some(texture) = cache.get(&index) {
            return Ok(Some(texture.clone()));
        }

        let Some(def) = textures.get(index) else {
            return Ok(None);
        };
        let Some(image) = def.source.and_then(|source| images.get(source)) else {
            return Ok(None);
        };

        let texture = TextureLoader::new().from_bytes(&image.data, image.mime_type.as_deref())?;

        // `texture.flipY = false` in `GLTFParser.loadTextureImage`: glTF UVs
        // have their origin at the top left, so the image is not flipped.
        texture.set_flip_y(false);

        let sampler = def
            .sampler
            .and_then(|i| self.json.pointer(&format!("/samplers/{i}")))
            .cloned()
            .unwrap_or(Value::Null);

        // `WEBGL_FILTERS` / `WEBGL_WRAPPINGS`, with glTF's defaults (REPEAT,
        // and three.js' own `LinearFilter` / `LinearMipmapLinearFilter`).
        let wrap = |name: &str| match sampler.get(name).and_then(Value::as_u64) {
            Some(33071) => Wrapping::ClampToEdge,
            // `MirroredRepeatWrapping` is not ported; nothing on the ladder
            // uses it and it would be a silent difference, so it is refused.
            _ => Wrapping::Repeat,
        };
        texture.set_wrapping(wrap("wrapS"), wrap("wrapT"));

        if let Some(9728) = sampler.get("magFilter").and_then(Value::as_u64) {
            texture.set_mag_filter(TextureFilter::Nearest);
        }
        match sampler.get("minFilter").and_then(Value::as_u64) {
            Some(9728) => texture.set_min_filter(MinFilter::Nearest),
            Some(9729) => texture.set_min_filter(MinFilter::Linear),
            _ => texture.set_min_filter(MinFilter::LinearMipmapLinear),
        }

        cache.insert(index, texture.clone());

        Ok(Some(texture))
    }

    /// `GLTFParser.assignTexture` — the texture a material *reference* names,
    /// which is not always the texture the reference's `index` names.
    ///
    /// Three clones the `Texture` twice over: once when `textureDef.texCoord`
    /// is past 0, and again in
    /// `GLTFTextureTransformExtension.extendTexture()` when the transform is
    /// not a no-op against what the texture already has (mrdoob/three.js#21819
    /// is why the no-op test exists at all — cloning unconditionally broke
    /// texture sharing). The clone is what lets two materials sample one image
    /// through two different uv transforms, which `SheenChair.glb` does: its
    /// occlusion map is `texCoord: 1` on every material, and its fabric samples
    /// the base colour at `offset ( -3, 3 ) scale ( 7, 7 )`.
    fn assign_texture(
        &self,
        cache: &mut HashMap<usize, Texture>,
        textures: &[GltfTexture],
        images: &[GltfImage],
        map_def: &GltfTextureRef,
        color_space: ColorSpace,
    ) -> Result<Option<Texture>, Error> {
        let Some(mut texture) = self.load_texture(cache, textures, images, map_def.index)? else {
            return Ok(None);
        };

        if let Some(tex_coord) = map_def.tex_coord.filter(|&n| n > 0) {
            texture = texture.clone_texture();
            texture.set_channel(tex_coord);
        }

        if let Some(transform) = &map_def.transform {
            let no_op = transform.tex_coord.is_none_or(|n| n == texture.channel())
                && transform.offset.is_none()
                && transform.rotation.is_none()
                && transform.scale.is_none();

            if !no_op {
                texture = texture.clone_texture();

                if let Some(tex_coord) = transform.tex_coord {
                    texture.set_channel(tex_coord);
                }
                if let Some([x, y]) = transform.offset {
                    texture.set_offset(x, y);
                }
                if let Some(rotation) = transform.rotation {
                    texture.set_rotation(rotation);
                }
                if let Some([x, y]) = transform.scale {
                    texture.set_repeat(x, y);
                }

                if let Some(rotation) = transform.rotation {
                    // glTF composes the uv transform as `T * R * S` and three.js
                    // as `T * S * R`, so a rotated transform cannot go through
                    // `updateMatrix()`. Three writes the matrix itself and sets
                    // `matrixAutoUpdate = false`; here the matrix is simply the
                    // last write, and nothing recomputes it afterwards
                    // (`Texture::set_matrix`).
                    let (c, sin) = (rotation.cos(), rotation.sin());
                    let repeat = transform.scale.unwrap_or([1.0, 1.0]);
                    let offset = transform.offset.unwrap_or([0.0, 0.0]);
                    let mut matrix = crate::math::Matrix3::identity();
                    matrix.set(
                        repeat[0] * c,
                        repeat[1] * sin,
                        offset[0],
                        -repeat[0] * sin,
                        repeat[1] * c,
                        offset[1],
                        0.0,
                        0.0,
                        1.0,
                    );
                    texture.set_matrix(matrix);
                }
            }
        }

        if color_space == ColorSpace::SRGB {
            texture.set_color_space(color_space);
        }

        Ok(Some(texture))
    }

    /// `GLTFParser.assignFinalMaterial`: the glTF material, then the variant
    /// clone the geometry asks for, out of a cache keyed exactly as three's
    /// `'ClonedMaterial:<uuid>:derivative-tangents:vertex-colors:flat-shading:'`
    /// is.
    ///
    /// Two primitives that share a glTF material *and* want the same variant get
    /// the same material here, which is what stops `PrimaryIonDrive.glb`'s six
    /// primitives from building six materials for its three. The port's copy is
    /// a clone rather than a shared handle, because a [`crate::objects::Mesh`]
    /// owns its material by value; pipelines are still shared, because the
    /// program cache is keyed on the generated WGSL, not on the material.
    fn final_material(
        &self,
        texture_cache: &mut HashMap<usize, Texture>,
        material_cache: &mut HashMap<MaterialVariant, MeshBasicNodeMaterial>,
        primitive: &GltfPrimitive,
        materials: &[GltfMaterial],
        textures: &[GltfTexture],
        images: &[GltfImage],
    ) -> Result<Option<MeshBasicNodeMaterial>, Error> {
        // `mesh.material` is undefined for a primitive with no material, and the
        // renderer falls back to its own default — `Mesh::new`'s `None`.
        let Some(index) = primitive.material else {
            return Ok(None);
        };
        let Some(def) = materials.get(index) else {
            return Ok(None);
        };

        let variant = MaterialVariant {
            material: index,
            // `geometry.attributes.tangent === undefined`, and the two beside it.
            use_derivative_tangents: !primitive.geometry.has_attribute("tangent"),
            use_vertex_colors: primitive.geometry.has_attribute("color"),
            use_flat_shading: !primitive.geometry.has_attribute("normal"),
        };

        if let Some(cached) = material_cache.get(&variant) {
            return Ok(Some(cached.clone()));
        }

        let mut material = self.build_material(texture_cache, def, textures, images)?;

        if variant.use_vertex_colors {
            material.vertex_colors = true;
        }
        if variant.use_flat_shading {
            material.flat_shading = true;
        }
        if variant.use_derivative_tangents {
            // mrdoob/three.js#11438: the derivative TBN frame `setupNormal()`
            // falls back to has the opposite handedness. three flips the scale
            // whenever the material *has* a `normalScale` — which every
            // standard material does — not only when it has a normal map, so
            // `circle2_constant2_0` comes out of three's own parse with
            // `normalScale ( 1, -1 )` and no map to apply it to.
            material.normal_scale.y *= -1.0;
        }

        material_cache.insert(variant, material.clone());

        Ok(Some(material))
    }

    /// `GLTFParser.loadMaterial`, for the two material types this crate has:
    /// `MeshStandardNodeMaterial`, or `MeshPhysicalNodeMaterial` when
    /// `KHR_materials_ior` / `KHR_materials_specular` are on the material.
    ///
    /// The variant clone that follows it — `vertexColors`, `flatShading` and the
    /// `normalScale.y` flip — is [`Self::final_material`]'s half.
    fn build_material(
        &self,
        cache: &mut HashMap<usize, Texture>,
        material: &GltfMaterial,
        textures: &[GltfTexture],
        images: &[GltfImage],
    ) -> Result<MeshBasicNodeMaterial, Error> {
        let [r, g, b, a] = material.base_color_factor;

        // `materialParams.color.setRGB( ..., LinearSRGBColorSpace )` — the
        // factor is already linear, so no transfer is applied.
        let mut out = if material.ior.is_some()
            || material.specular_factor.is_some()
            || material.sheen.is_some()
        {
            MeshBasicNodeMaterial::physical(
                Color::new(r, g, b),
                material.roughness_factor,
                material.metallic_factor,
            )
        } else {
            MeshBasicNodeMaterial::standard(
                Color::new(r, g, b),
                material.roughness_factor,
                material.metallic_factor,
            )
        };

        out.opacity = a;
        out.side = if material.double_sided {
            Side::Double
        } else {
            Side::Front
        };

        // `alphaMode`. `BLEND` is the pair three.js writes together — a
        // transparent material that does *not* write depth, which is what puts
        // `HoloFillDark` in the render list's transparent half and lets the
        // opaque geometry behind it through.
        //
        // `MASK` is deliberately not wired: it is `materialParams.alphaTest =
        // alphaCutoff`, and this crate has only `alphaTestNode` (see
        // `MeshBasicNodeMaterial::alpha_test_node`), whose WGSL is a literal
        // where three's is the `materialAlphaTest` uniform. Nothing on the
        // ladder is `MASK`; guessing the shader here would be a silent
        // divergence rather than an API.
        if material.alpha_mode == "BLEND" {
            out.transparent = true;
            out.depth_write = false;
        } else {
            out.transparent = false;
        }

        // `materialParams.emissive = new Color().setRGB( ..., LinearSRGBColorSpace )`
        // — the factor is already linear, as `baseColorFactor` is.
        // `emissiveIntensity` stays at three's default 1; glTF has no field for
        // it (`KHR_materials_emissive_strength`, which does, is not ported).
        let [er, eg, eb] = material.emissive_factor;
        out.emissive = Color::new(er, eg, eb);

        if let Some(map_def) = &material.base_color_texture {
            out.map = self.assign_texture(cache, textures, images, map_def, ColorSpace::SRGB)?;
        }

        // `metalnessMap` and `roughnessMap` are the same glTF texture: B is
        // metalness, G is roughness, and the material reads the channels.
        if let Some(map_def) = &material.metallic_roughness_texture {
            let map =
                self.assign_texture(cache, textures, images, map_def, ColorSpace::NoColorSpace)?;
            out.metalness_map = map.clone();
            out.roughness_map = map;
        }

        // `emissiveTexture` — an sRGB colour map, multiplied into
        // `emissive * emissiveIntensity` by `MaterialNode.EMISSIVE`.
        if let Some(map_def) = &material.emissive_texture {
            out.emissive_map =
                self.assign_texture(cache, textures, images, map_def, ColorSpace::SRGB)?;
        }

        // `occlusionTexture` → `aoMap`. `materialParams.aoMapIntensity =
        // occlusionTexture.strength` is not wired: glTF's `strength` defaults
        // to 1 and DamagedHelmet leaves it there; `aoMapIntensity` is a
        // material field either way.
        //
        // `occlusionTexture.texCoord` is 1 on every `SheenChair.glb` material,
        // so the `aoMap` here really is sampled along `uv1` —
        // `Texture::channel`, and the `uv1` attribute the mesh loader already
        // names from `TEXCOORD_1`.
        if let Some(map_def) = &material.occlusion_texture {
            out.ao_map =
                self.assign_texture(cache, textures, images, map_def, ColorSpace::NoColorSpace)?;
        }

        if let Some(map_def) = &material.normal_texture {
            out.normal_map =
                self.assign_texture(cache, textures, images, map_def, ColorSpace::NoColorSpace)?;
            out.normal_scale = Vector2::new(material.normal_scale, material.normal_scale);
        }

        if let Some(ior) = material.ior {
            out.ior = ior;
        }
        if let Some(factor) = material.specular_factor {
            out.specular_intensity = factor;
        }
        let [sr, sg, sb] = material.specular_color_factor;
        out.specular_color = Color::new(sr, sg, sb);
        if let Some(map_def) = &material.specular_color_texture {
            out.specular_color_map =
                self.assign_texture(cache, textures, images, map_def, ColorSpace::SRGB)?;
        }

        // `GLTFMaterialsSheenExtension`. glTF has no sheen *intensity*, so the
        // extension's presence is `sheen = 1` and the factors are the colour
        // and the roughness. `sheenColorTexture` / `sheenRoughnessTexture` are
        // not ported — `SheenChair.glb` carries neither, and a map with no
        // dump behind it would be a guess (`docs/nodes.md` §25).
        if let Some(sheen) = material.sheen {
            out.sheen = 1.0;
            let [r, g, b] = sheen.color_factor;
            out.sheen_color = Color::new(r, g, b);
            out.sheen_roughness = sheen.roughness_factor;
        }

        Ok(out)
    }

    /// `GLTFParser.loadAnimation`.
    fn load_animations(&self, nodes: &[Node]) -> Result<Vec<AnimationClip>, Error> {
        let mut clips = Vec::new();

        for (index, animation_def) in self
            .json
            .get("animations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .enumerate()
        {
            let name = animation_def
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("animation_{index}"));

            let samplers = animation_def
                .get("samplers")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            let mut tracks = Vec::new();

            for channel in animation_def
                .get("channels")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
            {
                let Some(sampler) = json_usize(channel, "sampler").and_then(|i| samplers.get(i))
                else {
                    continue;
                };
                let Some(target) = channel.get("target") else {
                    continue;
                };
                let Some(node_index) = json_usize(target, "node") else {
                    continue;
                };
                let Some(node) = nodes.get(node_index) else {
                    continue;
                };
                let Some(path) = target.get("path").and_then(Value::as_str) else {
                    continue;
                };
                let Some(property) = path_property(path) else {
                    continue;
                };

                let input = json_usize(sampler, "input").ok_or(GltfError::MissingField {
                    what: "sampler input",
                })?;
                let output = json_usize(sampler, "output").ok_or(GltfError::MissingField {
                    what: "sampler output",
                })?;

                let (times, _) = self.accessor(input)?;
                let (mut values, _) = self.accessor(output)?;

                let interpolation = match sampler
                    .get("interpolation")
                    .and_then(Value::as_str)
                    .unwrap_or("LINEAR")
                {
                    "STEP" => InterpolationMode::Discrete,
                    // TODO(rung 10): CUBICSPLINE needs
                    // `GLTFCubicSplineInterpolant`; until the crate has it, drop
                    // the tangents and treat the keyframes as linear, exactly as
                    // `GLTFLoader` does for a one-keyframe CUBICSPLINE track.
                    "CUBICSPLINE" => {
                        let stride = values.len() / (times.len() * 3);
                        let mut linear = Vec::with_capacity(times.len() * stride);
                        for i in 0..times.len() {
                            let at = i * stride * 3 + stride;
                            linear.extend_from_slice(&values[at..at + stride]);
                        }
                        values = linear;
                        InterpolationMode::Linear
                    }
                    _ => InterpolationMode::Linear,
                };

                let track_name = format!("{}.{}", node.borrow().name, property);

                let track = match property {
                    "quaternion" => {
                        KeyframeTrack::quaternion(&track_name, times, values, Some(interpolation))
                    }
                    "morphTargetInfluences" => {
                        KeyframeTrack::number(&track_name, times, values, Some(interpolation))
                    }
                    _ => KeyframeTrack::vector(&track_name, times, values, Some(interpolation)),
                };

                match track {
                    Ok(track) => tracks.push(track),
                    // `THREE.KeyframeTrack: no keyframes in track` — skip it
                    Err(_) => continue,
                }
            }

            clips.push(AnimationClip::from_tracks(
                &sanitize_node_name(&name),
                tracks,
            ));
        }

        Ok(clips)
    }
}

/// `nodes[ i ].matrix` or its TRS triple.
fn apply_node_transform(object: &mut Object3D, node_def: &Value) {
    if let Some(matrix) = node_def.get("matrix").and_then(Value::as_array) {
        let values: Vec<f64> = matrix.iter().filter_map(Value::as_f64).collect();
        let mut m = Matrix4::identity();
        m.from_array(&values, 0);
        object.apply_matrix4(&m);
        return;
    }

    if let Some(values) = node_def.get("translation").and_then(Value::as_array) {
        let values: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
        object.position.from_array(&values, 0);
    }

    if let Some(values) = node_def.get("rotation").and_then(Value::as_array) {
        let values: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
        object.quaternion.from_array(&values, 0);
        object.sync_rotation_from_quaternion();
    }

    if let Some(values) = node_def.get("scale").and_then(Value::as_array) {
        let values: Vec<f64> = values.iter().filter_map(Value::as_f64).collect();
        object.scale.from_array(&values, 0);
    }
}

/// `GLTFBinaryExtension` — the GLB container.
fn parse_glb(data: &[u8]) -> Result<(Value, Option<Vec<u8>>), Error> {
    if data.len() < 12 {
        return Err(GltfError::BadHeader.into());
    }

    let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if version < 2 {
        return Err(GltfError::LegacyBinary.into());
    }

    let mut json = None;
    let mut bin = None;
    let mut at = 12;

    while at + 8 <= data.len() {
        let length =
            u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        let chunk_type =
            u32::from_le_bytes([data[at + 4], data[at + 5], data[at + 6], data[at + 7]]);
        at += 8;

        let chunk = data.get(at..at + length).ok_or(GltfError::TruncatedChunk)?;

        match chunk_type {
            // `BINARY_EXTENSION_CHUNK_TYPES.JSON`
            0x4E4F_534A => {
                let text = std::str::from_utf8(chunk).map_err(|_| GltfError::NotUtf8)?;
                json = Some(
                    serde_json::from_str(text)
                        .map_err(|source| Error::Json { path: None, source })?,
                );
            }
            // `BINARY_EXTENSION_CHUNK_TYPES.BIN`
            0x004E_4942 => bin = Some(chunk.to_vec()),
            // `// Clients must ignore chunks with unknown types.`
            _ => {}
        }

        at += length;
    }

    let json = json.ok_or(GltfError::NoJsonChunk)?;
    Ok((json, bin))
}

/// A `data:` URI, base64 or percent-encoded.
fn decode_data_uri(uri: &str) -> Result<Vec<u8>, Error> {
    let (header, payload) = uri
        .split_once(',')
        .ok_or_else(|| GltfError::BadDataUri(uri.to_string()))?;

    if header.ends_with(";base64") {
        decode_base64(payload)
    } else {
        Ok(payload.as_bytes().to_vec())
    }
}

/// `atob`, for the `data:` URIs glTF embeds buffers in.
fn decode_base64(input: &str) -> Result<Vec<u8>, Error> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut accumulator: u32 = 0;
    let mut bits = 0;

    for c in input.bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }

        let value = TABLE
            .iter()
            .position(|&t| t == c)
            .ok_or(GltfError::BadBase64(c as char))? as u32;

        accumulator = (accumulator << 6) | value;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            out.push((accumulator >> bits) as u8);
        }
    }

    Ok(out)
}

/// `json[ key ]` as a `usize`, for the many optional indices in a glTF.
fn json_usize(value: &Value, key: &str) -> Option<usize> {
    value.get(key).and_then(Value::as_u64).map(|v| v as usize)
}
