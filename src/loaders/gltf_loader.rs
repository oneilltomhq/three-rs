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
use crate::math::Matrix4;
use crate::objects::{Bone, Skeleton, SkinnedMesh};

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
    pub base_color_texture: Option<usize>,
    pub metallic_factor: f64,
    pub roughness_factor: f64,
    pub metallic_roughness_texture: Option<usize>,
    pub normal_texture: Option<usize>,
    pub normal_scale: f64,
    pub occlusion_texture: Option<usize>,
    pub emissive_texture: Option<usize>,
    pub emissive_factor: [f64; 3],
    /// `'OPAQUE'` / `'MASK'` / `'BLEND'`.
    pub alpha_mode: String,
    pub alpha_cutoff: f64,
    pub double_sided: bool,
    /// Extension names present on this material, so the rung worker can see what
    /// it is missing.
    pub extensions: Vec<String>,
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
    /// The skinned primitives, bound to their skeletons.
    pub skinned_meshes: Vec<SkinnedMesh>,
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

    /// An accessor as a `BufferGeometry` index, `Uint16` when it fits.
    fn index_attribute(&self, index: usize) -> Result<Index, Error> {
        let (values, _) = self.accessor(index)?;
        let max = values.iter().cloned().fold(0.0_f64, f64::max);

        Ok(if max < 65536.0 {
            Index::U16(values.iter().map(|&v| v as u16).collect())
        } else {
            Index::U32(values.iter().map(|&v| v as u32).collect())
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
            let name = self.create_unique_name(node_def.get("name").and_then(Value::as_str));

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
        for primitive in &primitives {
            let Some(skin) = primitive.skin else { continue };

            let mut mesh = SkinnedMesh::new(primitive.geometry.clone());
            mesh.node = primitive.node.clone();
            mesh.morph_target_influences = primitive.morph_target_influences.clone();
            mesh.morph_target_dictionary = primitive.morph_target_dictionary.clone();
            mesh.normalize_skin_weights();

            // `mesh.bind( skeleton, _identityMatrix )`: glTF joint transforms
            // are already relative to the skin, so the bind matrix is identity
            // and it is `bindMatrixInverse` (tracked from `matrixWorld`, the
            // `AttachedBindMode` default) that does the work.
            mesh.bind(skins[skin].clone(), Some(Matrix4::identity()));

            skinned_meshes.push(mesh);
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
                let attribute = self.attribute(accessor as usize)?;
                geometry.set_attribute(&attribute_name(&semantic), attribute);
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
                base_color_texture: pbr
                    .and_then(|pbr| pbr.pointer("/baseColorTexture/index"))
                    .and_then(Value::as_u64)
                    .map(|i| i as usize),
                metallic_factor: pbr
                    .and_then(|pbr| pbr.get("metallicFactor"))
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                roughness_factor: pbr
                    .and_then(|pbr| pbr.get("roughnessFactor"))
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                metallic_roughness_texture: pbr
                    .and_then(|pbr| pbr.pointer("/metallicRoughnessTexture/index"))
                    .and_then(Value::as_u64)
                    .map(|i| i as usize),
                normal_texture: material_def
                    .pointer("/normalTexture/index")
                    .and_then(Value::as_u64)
                    .map(|i| i as usize),
                normal_scale: material_def
                    .pointer("/normalTexture/scale")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                occlusion_texture: material_def
                    .pointer("/occlusionTexture/index")
                    .and_then(Value::as_u64)
                    .map(|i| i as usize),
                emissive_texture: material_def
                    .pointer("/emissiveTexture/index")
                    .and_then(Value::as_u64)
                    .map(|i| i as usize),
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
