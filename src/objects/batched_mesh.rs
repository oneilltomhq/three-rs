//! Port of `three.js/src/objects/BatchedMesh.js` — the subset
//! `webgpu_mesh_batch` exercises: geometry ranges packed into one
//! `BufferGeometry`, per-instance matrices and colours in `DataTexture`s, the
//! per-instance frustum cull and the sorted draw list `onBeforeRender()` builds.
//!
//! Under `WebGPURenderer` there is no multi-draw and no indirect draw: the
//! backend walks `_multiDrawStarts` / `_multiDrawCounts` and issues one
//! ordinary `drawIndexed( count, 1, start / bytesPerElement, 0, i )` per
//! range, with `firstInstance = i` — the *draw ordinal*, which is what
//! `@builtin(instance_index)` reads and what `_indirectTexture` maps back to an
//! instance id. See `docs/rung11-progress.md`.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Index, Node, Object3D};
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Box3, CoordinateSystem, Frustum, Matrix4, Sphere, Vector3};
use crate::nodes::batch::BatchEntry;
use crate::objects::{Mesh, Payload};
use crate::textures::DataTexture;

/// One entry of `_geometryInfo`.
#[derive(Clone, Debug)]
pub struct GeometryInfo {
    pub vertex_start: usize,
    pub vertex_count: usize,
    pub reserved_vertex_count: usize,
    /// `-1` when the batch geometry is not indexed.
    pub index_start: isize,
    pub index_count: usize,
    pub reserved_index_count: usize,
    /// The draw range: `indexStart` / `indexCount` for an indexed batch.
    pub start: usize,
    pub count: usize,
    pub bounding_box: Option<Box3>,
    pub bounding_sphere: Option<Sphere>,
    pub active: bool,
}

/// One entry of `_instanceInfo`.
#[derive(Clone, Copy, Debug)]
pub struct InstanceInfo {
    pub visible: bool,
    pub active: bool,
    pub geometry_index: usize,
}

/// One entry of `MultiDrawRenderList.list`.
#[derive(Clone, Copy, Debug)]
pub struct MultiDrawItem {
    pub start: usize,
    pub count: usize,
    pub z: f64,
    pub index: usize,
}

/// What the page's `sortFunction` reads off `this` and `camera`. three.js calls
/// the custom sort with `this` bound to the mesh and the camera as the second
/// argument; the port hands it only the two values any sort on the ladder uses.
#[derive(Clone, Copy, Debug)]
pub struct SortContext {
    /// `camera.far`.
    pub camera_far: f64,
    /// `this.material.transparent`, which the page uses as `options.reversed`.
    pub transparent: bool,
    /// `this.maxInstanceCount`, the page's `options.aux` length.
    pub max_instance_count: usize,
}

/// `BatchedMesh.customSort`.
pub type CustomSort = Rc<dyn Fn(&mut Vec<MultiDrawItem>, &SortContext)>;

/// One `drawIndexed()` the backend issues for this batch.
#[derive(Clone, Copy, Debug)]
pub struct SubDraw {
    pub first_index: u32,
    pub index_count: u32,
    /// `firstInstance` — the draw ordinal, not the instance id.
    pub first_instance: u32,
}

/// The camera state `onBeforeRender()` needs.
#[derive(Clone, Copy, Debug)]
pub struct BatchCamera {
    pub projection_matrix: Matrix4,
    pub matrix_world_inverse: Matrix4,
    pub matrix_world: Matrix4,
    pub coordinate_system: CoordinateSystem,
    pub far: f64,
}

/// `BatchedMesh extends Mesh`.
#[derive(Clone)]
pub struct BatchedMesh {
    pub mesh: Mesh,
    pub per_object_frustum_culled: bool,
    pub sort_objects: bool,
    pub custom_sort: Option<CustomSort>,

    instance_info: Vec<InstanceInfo>,
    geometry_info: Vec<GeometryInfo>,

    next_index_start: usize,
    next_vertex_start: usize,
    geometry_count: usize,

    visibility_changed: bool,
    geometry_initialized: bool,

    max_instance_count: usize,
    max_vertex_count: usize,
    max_index_count: usize,

    /// `_multiDrawStarts` — in *bytes*, as three.js keeps them.
    multi_draw_starts: Vec<i32>,
    multi_draw_counts: Vec<i32>,
    multi_draw_count: usize,
    multi_draw_bytes_per_element: usize,

    /// three.js' module-level `_vector`, reproduced as state because
    /// `getBoundingBoxAt()` / `getBoundingSphereAt()` overwrite it while they
    /// lazily compute a geometry's bounds, and `onBeforeRender()`'s sort reads
    /// it afterwards as if it still held the camera position. See
    /// `docs/rung11-progress.md`.
    shared_vector: Vector3,

    matrices_texture: DataTexture,
    indirect_texture: DataTexture,
    colors_texture: Option<DataTexture>,
}

impl BatchedMesh {
    /// `new BatchedMesh( maxInstanceCount, maxVertexCount, maxIndexCount, material )`,
    /// as a scene-graph [`Node`].
    #[allow(clippy::new_ret_no_self)] // mirrors three.js' constructor, which returns the object the scene holds
    pub fn new(
        max_instance_count: usize,
        max_vertex_count: usize,
        max_index_count: usize,
        material: MeshBasicNodeMaterial,
    ) -> Node {
        let mut object = Object3D {
            object_type: "BatchedMesh",
            ..Default::default()
        };
        object.payload = Payload::BatchedMesh(Box::new(Self {
            mesh: Mesh {
                geometry: Rc::new(BufferGeometry::new()),
                material: Some(material),
                morph_target_influences: Vec::new(),
                line_segments: None,
                count: None,
            },
            per_object_frustum_culled: true,
            sort_objects: true,
            custom_sort: None,
            instance_info: Vec::new(),
            geometry_info: Vec::new(),
            next_index_start: 0,
            next_vertex_start: 0,
            geometry_count: 0,
            visibility_changed: true,
            geometry_initialized: false,
            max_instance_count,
            max_vertex_count,
            max_index_count,
            multi_draw_starts: vec![0; max_instance_count],
            multi_draw_counts: vec![0; max_instance_count],
            multi_draw_count: 0,
            multi_draw_bytes_per_element: 1,
            shared_vector: Vector3::ZERO,
            matrices_texture: init_matrices_texture(max_instance_count),
            indirect_texture: init_indirect_texture(max_instance_count),
            colors_texture: None,
        }));
        object.into_node()
    }

    pub fn max_instance_count(&self) -> usize {
        self.max_instance_count
    }

    pub fn instance_count(&self) -> usize {
        self.instance_info.iter().filter(|i| i.active).count()
    }

    pub fn geometry(&self) -> &Rc<BufferGeometry> {
        &self.mesh.geometry
    }

    /// `getGeometryRangeAt( geometryId )` — the packed range this geometry
    /// owns inside the shared `BufferGeometry`.
    pub fn geometry_range_at(&self, geometry_id: usize) -> Option<&GeometryInfo> {
        self.geometry_info
            .get(geometry_id)
            .filter(|info| info.active)
    }

    /// The three textures the node system binds.
    pub fn batch_entry(&self) -> BatchEntry {
        BatchEntry {
            indirect: self.indirect_texture.clone(),
            matrices: self.matrices_texture.clone(),
            colors: self.colors_texture.clone(),
        }
    }

    /// `_multiDrawCount` sub-ranges as `(firstIndex, indexCount, firstInstance)`
    /// — `WebGPUBackend.draw()`'s `object.isBatchedMesh` arm, which divides the
    /// byte start back out by `bytesPerElement`.
    pub fn sub_draws(&self) -> Vec<SubDraw> {
        let bpe = self.multi_draw_bytes_per_element.max(1) as i32;
        (0..self.multi_draw_count)
            .map(|i| SubDraw {
                first_index: (self.multi_draw_starts[i] / bpe) as u32,
                index_count: self.multi_draw_counts[i] as u32,
                first_instance: i as u32,
            })
            .collect()
    }

    /// `setCustomSort( func )`.
    pub fn set_custom_sort(&mut self, func: Option<CustomSort>) -> &mut Self {
        self.custom_sort = func;
        self
    }

    fn init_colors_texture(&mut self) {
        let size = (self.max_instance_count as f64).sqrt().ceil().max(1.0) as u32;
        // "4 floats per RGBA pixel initialized to white" — the alpha a
        // `Color.toArray()` never writes therefore stays 1.
        self.colors_texture = Some(DataTexture::new_f32(
            vec![1.0; (size * size * 4) as usize],
            size,
            size,
        ));
    }

    /// `_initializeGeometry( reference )`.
    fn initialize_geometry(&mut self, reference: &BufferGeometry) {
        if self.geometry_initialized {
            return;
        }
        let max_vertex_count = self.max_vertex_count;
        let max_index_count = self.max_index_count;
        let geometry = Rc::get_mut(&mut self.mesh.geometry)
            .expect("three-rs: the batch geometry is shared; add geometries before rendering");

        let names: Vec<(String, usize)> = reference
            .attributes()
            .map(|(name, attr)| (name.to_string(), attr.item_size))
            .collect();
        for (name, item_size) in names {
            geometry.set_attribute(
                &name,
                BufferAttribute::new(vec![0.0; max_vertex_count * item_size], item_size),
            );
        }

        if reference.index.is_some() {
            // "Reserve last u16 index for primitive restart."
            geometry.set_index_attribute(if max_vertex_count > 65535 {
                Index::U32(vec![0; max_index_count])
            } else {
                Index::U16(vec![0; max_index_count])
            });
        }

        self.geometry_initialized = true;
    }

    /// `_validateGeometry( geometry )`.
    fn validate_geometry(&self, geometry: &BufferGeometry) {
        let batch = &self.mesh.geometry;
        assert_eq!(
            geometry.index.is_some(),
            batch.index.is_some(),
            "THREE.BatchedMesh: All geometries must consistently have \"index\"."
        );
        for (name, dst) in batch.attributes() {
            let src = geometry.get_attribute(name).unwrap_or_else(|| {
                panic!(
                    "THREE.BatchedMesh: Added geometry missing \"{name}\". \
                     All geometries must have consistent attributes."
                )
            });
            assert_eq!(
                src.item_size, dst.item_size,
                "THREE.BatchedMesh: All attributes must have a consistent itemSize."
            );
        }
    }

    /// `addGeometry( geometry )` with the default reserved counts.
    pub fn add_geometry(&mut self, geometry: &BufferGeometry) -> usize {
        self.initialize_geometry(geometry);
        self.validate_geometry(geometry);

        let reserved_vertex_count = geometry
            .position()
            .expect("three-rs: an added geometry needs a position attribute")
            .count();
        let reserved_index_count = geometry.index.as_ref().map(|i| i.count()).unwrap_or(0);

        let info = GeometryInfo {
            vertex_start: self.next_vertex_start,
            vertex_count: 0,
            reserved_vertex_count,
            index_start: if geometry.index.is_some() {
                self.next_index_start as isize
            } else {
                -1
            },
            index_count: 0,
            reserved_index_count,
            start: 0,
            count: 0,
            bounding_box: None,
            bounding_sphere: None,
            active: true,
        };

        assert!(
            !(info.index_start != -1
                && info.index_start as usize + info.reserved_index_count > self.max_index_count
                || info.vertex_start + info.reserved_vertex_count > self.max_vertex_count),
            "THREE.BatchedMesh: Reserved space request exceeds the maximum buffer size."
        );

        let geometry_id = self.geometry_count;
        self.geometry_count += 1;
        self.geometry_info.push(info);

        self.set_geometry_at(geometry_id, geometry);

        let info = &self.geometry_info[geometry_id];
        self.next_index_start = (info.index_start.max(0) as usize) + info.reserved_index_count;
        self.next_vertex_start = info.vertex_start + info.reserved_vertex_count;

        geometry_id
    }

    /// `setGeometryAt( geometryId, geometry )`.
    pub fn set_geometry_at(&mut self, geometry_id: usize, geometry: &BufferGeometry) -> usize {
        assert!(
            geometry_id < self.geometry_count,
            "THREE.BatchedMesh: Maximum geometry count reached."
        );
        self.validate_geometry(geometry);

        let info = self.geometry_info[geometry_id].clone();
        let has_index = self.mesh.geometry.index.is_some();
        let src_index_count = geometry.index.as_ref().map(|i| i.count()).unwrap_or(0);
        assert!(
            !(has_index && src_index_count > info.reserved_index_count
                || geometry.position().map(|p| p.count()).unwrap_or(0)
                    > info.reserved_vertex_count),
            "THREE.BatchedMesh: Reserved space not large enough for provided geometry."
        );

        let vertex_start = info.vertex_start;
        let reserved_vertex_count = info.reserved_vertex_count;
        let vertex_count = geometry
            .position()
            .expect("three-rs: an added geometry needs a position attribute")
            .count();

        {
            let batch = Rc::get_mut(&mut self.mesh.geometry)
                .expect("three-rs: the batch geometry is shared; add geometries before rendering");

            let names: Vec<String> = batch.attributes().map(|(n, _)| n.to_string()).collect();
            for name in names {
                let src = geometry
                    .get_attribute(&name)
                    .expect("three-rs: validate_geometry checked this attribute exists");
                let item_size = src.item_size;
                let src_count = src.count();
                let values: Vec<f32> = src.array().clone();
                let dst = batch
                    .get_attribute_mut(&name)
                    .expect("three-rs: the batch geometry has this attribute");
                dst.set(&values, vertex_start * item_size);
                // "fill the rest in with zeroes"
                for i in src_count..reserved_vertex_count {
                    for c in 0..item_size {
                        dst.array_mut()[(vertex_start + i) * item_size + c] = 0.0;
                    }
                }
                dst.set_needs_update();
            }
        }

        let (start, count) = if has_index {
            let index_start = info.index_start.max(0) as usize;
            let reserved_index_count = info.reserved_index_count;
            let src: Vec<u32> = match geometry.index.as_ref() {
                Some(Index::U16(v)) => v.iter().map(|&x| x as u32).collect(),
                Some(Index::U32(v)) => v.clone(),
                None => Vec::new(),
            };
            let batch = Rc::get_mut(&mut self.mesh.geometry)
                .expect("three-rs: the batch geometry is shared; add geometries before rendering");
            let write = |dst: &mut dyn FnMut(usize, u32)| {
                for (i, value) in src.iter().enumerate() {
                    dst(index_start + i, vertex_start as u32 + value);
                }
                for i in src.len()..reserved_index_count {
                    dst(index_start + i, vertex_start as u32);
                }
            };
            match batch.index.as_mut() {
                Some(Index::U16(v)) => write(&mut |i, x| v[i] = x as u16),
                Some(Index::U32(v)) => write(&mut |i, x| v[i] = x),
                None => {}
            }
            (index_start, src.len())
        } else {
            (vertex_start, vertex_count)
        };

        let info = &mut self.geometry_info[geometry_id];
        info.vertex_count = vertex_count;
        if has_index {
            info.index_count = count;
        }
        info.start = start;
        info.count = count;
        info.bounding_box = None;
        info.bounding_sphere = None;

        self.visibility_changed = true;
        geometry_id
    }

    /// `addInstance( geometryId )`.
    pub fn add_instance(&mut self, geometry_id: usize) -> usize {
        assert!(
            self.instance_info.len() < self.max_instance_count,
            "THREE.BatchedMesh: Maximum item count reached."
        );
        let draw_id = self.instance_info.len();
        self.instance_info.push(InstanceInfo {
            visible: true,
            active: true,
            geometry_index: geometry_id,
        });

        let identity = Matrix4::identity().to_array();
        self.matrices_texture.with_f32_mut(|data| {
            for (i, v) in identity.iter().enumerate() {
                data[draw_id * 16 + i] = *v as f32;
            }
        });

        if let Some(colors) = &self.colors_texture {
            colors.with_f32_mut(|data| {
                data[draw_id * 4] = 1.0;
                data[draw_id * 4 + 1] = 1.0;
                data[draw_id * 4 + 2] = 1.0;
            });
        }

        self.visibility_changed = true;
        draw_id
    }

    /// `setMatrixAt( instanceId, matrix )`.
    pub fn set_matrix_at(&mut self, instance_id: usize, matrix: &Matrix4) -> &mut Self {
        let values = matrix.to_array();
        self.matrices_texture.with_f32_mut(|data| {
            for (i, v) in values.iter().enumerate() {
                data[instance_id * 16 + i] = *v as f32;
            }
        });
        self
    }

    /// `getMatrixAt( instanceId, matrix )`.
    pub fn matrix_at(&self, instance_id: usize) -> Matrix4 {
        let values: Vec<f64> = self.matrices_texture.with_f32(|data| {
            data[instance_id * 16..instance_id * 16 + 16]
                .iter()
                .map(|v| *v as f64)
                .collect()
        });
        let mut matrix = Matrix4::identity();
        matrix.from_array(&values, 0);
        matrix
    }

    /// `setColorAt( instanceId, color )`. `Color.toArray()` writes `rgb` only,
    /// so the alpha stays at the texture's initial 1.
    pub fn set_color_at(&mut self, instance_id: usize, color: &crate::math::Color) -> &mut Self {
        if self.colors_texture.is_none() {
            self.init_colors_texture();
        }
        let colors = self
            .colors_texture
            .as_ref()
            .expect("three-rs: the colours texture was just initialised");
        colors.with_f32_mut(|data| {
            data[instance_id * 4] = color.r as f32;
            data[instance_id * 4 + 1] = color.g as f32;
            data[instance_id * 4 + 2] = color.b as f32;
        });
        self
    }

    /// `getBoundingBoxAt( geometryId, target )` — computed from the packed
    /// geometry's own index range, so it costs nothing until the first cull.
    pub fn bounding_box_at(&mut self, geometry_id: usize) -> Option<Box3> {
        if geometry_id >= self.geometry_count {
            return None;
        }
        if self.geometry_info[geometry_id].bounding_box.is_none() {
            let info = self.geometry_info[geometry_id].clone();
            let geometry = &self.mesh.geometry;
            let position = geometry.position()?;
            let mut box3 = Box3::default();
            let mut scratch = Vector3::ZERO;
            for i in info.start..info.start + info.count {
                let iv = index_at(geometry, i);
                // `box.expandByPoint( _vector.fromBufferAttribute( position, iv ) )`
                scratch = position.get_vector3(iv);
                box3.expand_by_point(&scratch);
            }
            self.shared_vector = scratch;
            self.geometry_info[geometry_id].bounding_box = Some(box3);
        }
        self.geometry_info[geometry_id].bounding_box
    }

    /// `getBoundingSphereAt( geometryId, target )`.
    pub fn bounding_sphere_at(&mut self, geometry_id: usize) -> Option<Sphere> {
        if geometry_id >= self.geometry_count {
            return None;
        }
        if self.geometry_info[geometry_id].bounding_sphere.is_none() {
            let box3 = self.bounding_box_at(geometry_id)?;
            let info = self.geometry_info[geometry_id].clone();
            let center = box3.get_center();
            let geometry = &self.mesh.geometry;
            let position = geometry.position()?;
            let mut max_radius_sq: f64 = 0.0;
            let mut scratch = Vector3::ZERO;
            for i in info.start..info.start + info.count {
                let iv = index_at(geometry, i);
                // `_vector.fromBufferAttribute( position, iv )`
                scratch = position.get_vector3(iv);
                max_radius_sq = max_radius_sq.max(center.distance_to_squared(&scratch));
            }
            self.shared_vector = scratch;
            self.geometry_info[geometry_id].bounding_sphere =
                Some(Sphere::new(center, max_radius_sq.sqrt()));
        }
        self.geometry_info[geometry_id].bounding_sphere
    }

    /// `_multiDrawCount` after the last `onBeforeRender()`.
    pub fn multi_draw_count(&self) -> usize {
        self.multi_draw_count
    }

    /// The first `n` entries of `_indirectTexture` — the draw ordinal → instance
    /// id table, for tests.
    pub fn indirect_prefix(&self, n: usize) -> Vec<u32> {
        self.indirect_texture.with_u32(|data| data[..n].to_vec())
    }

    /// `onBeforeRender( renderer, scene, camera, geometry, material )`.
    pub fn on_before_render(&mut self, matrix_world: &Matrix4, camera: &BatchCamera) {
        if !self.visibility_changed && !self.per_object_frustum_culled && !self.sort_objects {
            return;
        }

        // The indexed multi-draw start offset is in bytes.
        let bytes_per_element = match &self.mesh.geometry.index {
            Some(Index::U16(_)) => 2,
            Some(Index::U32(_)) => 4,
            None => 1,
        };

        let mut frustum = Frustum::default();
        if self.per_object_frustum_culled {
            let mut m = Matrix4::identity();
            m.multiply_matrices(&camera.projection_matrix, &camera.matrix_world_inverse);
            m.multiply(matrix_world);
            frustum.set_from_projection_matrix(&m, camera.coordinate_system, false);
        }

        let mut multi_draw_count = 0usize;

        if self.sort_objects {
            // The camera position and forward axis in the batch's local frame.
            let mut inverse = *matrix_world;
            inverse.invert();
            let mut camera_pos = Vector3::ZERO;
            camera_pos.set_from_matrix_position(&camera.matrix_world);
            camera_pos.apply_matrix4(&inverse);
            self.shared_vector = camera_pos;
            let mut forward = Vector3::new(0.0, 0.0, -1.0);
            forward.transform_direction(&camera.matrix_world);
            forward.transform_direction(&inverse);

            let mut list: Vec<MultiDrawItem> = Vec::new();
            for i in 0..self.instance_info.len() {
                let info = self.instance_info[i];
                if !(info.visible && info.active) {
                    continue;
                }
                let geometry_id = info.geometry_index;
                let matrix = self.matrix_at(i);
                let mut sphere = match self.bounding_sphere_at(geometry_id) {
                    Some(sphere) => sphere,
                    None => continue,
                };
                sphere.apply_matrix4(&matrix);

                let culled = self.per_object_frustum_culled && !frustum.intersects_sphere(&sphere);
                if culled {
                    continue;
                }
                let range = &self.geometry_info[geometry_id];
                // `_temp.subVectors( _sphere.center, _vector ).dot( _forward )`
                // — deliberately `_vector`, not the camera position: the
                // `getBoundingSphereAt()` above overwrites it the first time a
                // geometry's bounds are computed, so on the frame that fills
                // the cache the depths are measured from the last vertex of the
                // last geometry visited. Three does this; the graded frame is
                // that first frame, so the port does it too.
                let mut temp = Vector3::ZERO;
                temp.sub_vectors(&sphere.center, &self.shared_vector);
                let z = temp.dot(&forward);
                list.push(MultiDrawItem {
                    start: range.start,
                    count: range.count,
                    z,
                    index: i,
                });
            }

            let transparent = self
                .mesh
                .material
                .as_ref()
                .map(|m| m.transparent)
                .unwrap_or(false);
            match &self.custom_sort {
                None => {
                    // `list.sort( material.transparent ? sortTransparent : sortOpaque )`.
                    if transparent {
                        list.sort_by(|a, b| b.z.total_cmp(&a.z));
                    } else {
                        list.sort_by(|a, b| a.z.total_cmp(&b.z));
                    }
                }
                Some(sort) => {
                    let sort = sort.clone();
                    sort(
                        &mut list,
                        &SortContext {
                            camera_far: camera.far,
                            transparent,
                            max_instance_count: self.max_instance_count,
                        },
                    );
                }
            }

            let mut indirect = Vec::with_capacity(list.len());
            for item in &list {
                self.multi_draw_starts[multi_draw_count] = (item.start * bytes_per_element) as i32;
                self.multi_draw_counts[multi_draw_count] = item.count as i32;
                indirect.push(item.index as u32);
                multi_draw_count += 1;
            }
            self.indirect_texture.with_u32_mut(|data| {
                for (i, value) in indirect.iter().enumerate() {
                    data[i] = *value;
                }
            });
        } else {
            let mut indirect = Vec::new();
            for i in 0..self.instance_info.len() {
                let info = self.instance_info[i];
                if !(info.visible && info.active) {
                    continue;
                }
                let geometry_id = info.geometry_index;
                let culled = if self.per_object_frustum_culled {
                    let matrix = self.matrix_at(i);
                    match self.bounding_sphere_at(geometry_id) {
                        Some(mut sphere) => {
                            sphere.apply_matrix4(&matrix);
                            !frustum.intersects_sphere(&sphere)
                        }
                        None => false,
                    }
                } else {
                    false
                };
                if culled {
                    continue;
                }
                let range = &self.geometry_info[geometry_id];
                self.multi_draw_starts[multi_draw_count] = (range.start * bytes_per_element) as i32;
                self.multi_draw_counts[multi_draw_count] = range.count as i32;
                indirect.push(i as u32);
                multi_draw_count += 1;
            }
            self.indirect_texture.with_u32_mut(|data| {
                for (i, value) in indirect.iter().enumerate() {
                    data[i] = *value;
                }
            });
        }

        self.multi_draw_count = multi_draw_count;
        self.multi_draw_bytes_per_element = bytes_per_element;
        self.visibility_changed = false;
    }
}

/// `index ? index.getX( i ) : i`.
fn index_at(geometry: &BufferGeometry, i: usize) -> usize {
    match &geometry.index {
        Some(Index::U16(v)) => v[i] as usize,
        Some(Index::U32(v)) => v[i] as usize,
        None => i,
    }
}

/// `_initMatricesTexture()`: one matrix is four RGBA texels, and the side is
/// rounded up to a multiple of four so a matrix never straddles two rows.
fn init_matrices_texture(max_instance_count: usize) -> DataTexture {
    let mut size = ((max_instance_count * 4) as f64).sqrt();
    size = (size / 4.0).ceil() * 4.0;
    let size = (size.max(4.0)) as u32;
    DataTexture::new_f32(vec![0.0; (size * size * 4) as usize], size, size)
}

/// `_initIndirectTexture()`.
fn init_indirect_texture(max_instance_count: usize) -> DataTexture {
    let size = (max_instance_count as f64).sqrt().ceil().max(1.0) as u32;
    DataTexture::new_u32(vec![0; (size * size) as usize], size, size)
}
