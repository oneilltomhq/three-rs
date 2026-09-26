//! Port of `three.js/src/objects/Mesh.js` (rung 1 subset).

use std::rc::Rc;

use crate::core::{BufferGeometry, Face, Intersection, Node, Object3D, Raycaster};
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::math::{Box3, Matrix4, Ray, Sphere, Triangle, Vector2, Vector3};
use crate::objects::Payload;

/// The state `Mesh` adds to `Object3D`. It lives in a node's
/// [`Payload`](crate::objects::Payload) rather than wrapping an `Object3D`,
/// because the renderer finds meshes by walking the tree — see
/// `docs/scene-graph.md`.
#[derive(Clone)]
pub struct Mesh {
    pub geometry: Rc<BufferGeometry>,
    pub material: Option<MeshBasicNodeMaterial>,
    /// `Mesh.morphTargetInfluences` — one weight per `morphAttributes.position`
    /// entry, filled in by `updateMorphTargets()` from the constructor.
    pub morph_target_influences: Vec<f64>,
    /// `LineSegmentsGeometry`'s instanced attributes, when this mesh is a
    /// [`LineSegments2`](crate::addons::lines::LineSegments2).
    ///
    /// three.js keeps them on the geometry, as an `InstancedInterleavedBuffer`
    /// with two `InterleavedBufferAttribute` views; the port's node system
    /// carries an instanced attribute's data on the node, so they ride here
    /// instead and reach `setup()` through
    /// [`SetupContext::line_segments`](crate::materials::SetupContext). See
    /// [`crate::nodes::lines`] for the trade, and
    /// `docs/webgpu_lines_fat-progress.md` for the follow-up that moves them on
    /// to `BufferGeometry`.
    pub line_segments: Option<crate::nodes::lines::LineSegmentsAttributes>,
    /// `Mesh.count` — how many instances one draw of this mesh makes, with no
    /// instance matrix involved. `RenderObject.getInstanceCount()` reads it
    /// when the geometry is not an `InstancedBufferGeometry`; a node reads
    /// which instance it is through `instanceIndex`, which is how
    /// `webgpu_particles` draws 2000 sprites from one `range()`-driven mesh.
    pub count: usize,
}

impl Mesh {
    /// `new Mesh( geometry, material )`, as a scene-graph [`Node`].
    ///
    /// The material is `impl Into<Option<MeshBasicNodeMaterial>>`, so a call
    /// passes the material by value as `Line::new` and `InstancedMesh::new` do,
    /// or `None` where three.js leaves `material` undefined and the renderer
    /// falls back to a default white `MeshBasicMaterial` — which is what
    /// `webgpu_postprocessing_masking` and `webgpu_depth_texture` rely on,
    /// together with `scene.overrideMaterial`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(
        geometry: Rc<BufferGeometry>,
        material: impl Into<Option<MeshBasicNodeMaterial>>,
    ) -> Node {
        let mut object = Object3D {
            object_type: "Mesh",
            ..Default::default()
        };
        // `this.updateMorphTargets()`: `morphTargetInfluences` gets one 0 per
        // morph attribute of the first key in `geometry.morphAttributes`.
        let morph_target_influences = match geometry.morph_attributes().next() {
            Some((_, attributes)) => vec![0.0; attributes.len()],
            None => Vec::new(),
        };

        object.payload = Payload::Mesh(Self {
            geometry,
            material: material.into(),
            morph_target_influences,
            line_segments: None,
            count: 1,
        });
        object.into_node()
    }

    /// The `Mesh` state alone, for a caller that already has the [`Node`] to
    /// install it on — `GLTFLoader`, whose tree node exists before the
    /// primitive that turns it into a mesh, the way
    /// [`SkinnedMesh::of`](crate::objects::SkinnedMesh::of) serves the skinned
    /// half.
    pub fn of(geometry: Rc<BufferGeometry>, material: Option<MeshBasicNodeMaterial>) -> Self {
        let morph_target_influences = match geometry.morph_attributes().next() {
            Some((_, attributes)) => vec![0.0; attributes.len()],
            None => Vec::new(),
        };

        Self {
            geometry,
            material,
            morph_target_influences,
            line_segments: None,
            count: 1,
        }
    }

    /// `Mesh.intersectsFrustum( frustum )` needs the world matrix, which lives on
    /// the node; this is the geometry half, i.e. the bounding sphere three.js
    /// lazily computes in `_projectObject`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Option<crate::math::Sphere> {
        let bounding_sphere = self.geometry.compute_bounding_sphere()?;
        let mut sphere = crate::math::Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        sphere.apply_matrix4(matrix_world);
        Some(sphere)
    }

    /// `Mesh.getVertexPosition( index, target )` — the vertex's position with
    /// the morph targets applied, as the vertex shader would place it before
    /// skinning.
    pub fn get_vertex_position(&self, index: usize) -> Vector3 {
        vertex_position(&self.geometry, &self.morph_target_influences, index)
    }

    /// `Mesh.raycast( raycaster, intersects )`, for the node `object` whose
    /// `matrixWorld` is `matrix_world`.
    ///
    /// three.js returns early when `material === undefined`; a port mesh with
    /// no material draws with the renderer's default `MeshBasicMaterial`
    /// (`FrontSide`), which is what three's constructor would have given it,
    /// so it raycasts as front-sided instead.
    pub fn raycast(
        &self,
        matrix_world: &Matrix4,
        object: &Node,
        raycaster: &Raycaster,
        intersects: &mut Vec<Intersection>,
    ) {
        let Some(bounding_sphere) = self.geometry.compute_bounding_sphere() else {
            return;
        };
        let sphere = Sphere::new(bounding_sphere.center, bounding_sphere.radius);
        let target = MeshRaycast {
            geometry: &self.geometry,
            side: self.material.as_ref().map_or(Side::Front, |m| m.side),
            matrix_world: *matrix_world,
            draw_start: self.geometry.draw_range.start,
            draw_count: self.geometry.draw_range.count,
            vertex_position: &|index| self.get_vertex_position(index),
        };
        // `geometry.boundingBox` is `null` unless the app computed it, and
        // `computeBoundingSphere()` does not; the port's geometry cache always
        // holds both, so reading it would add a test three.js skips. Leave it
        // out, as in the common three.js case.
        target.raycast(&sphere, None, object, raycaster, intersects);
    }
}

/// `Mesh.getVertexPosition()` over any geometry and morph weights — shared by
/// `Mesh`, and by `SkinnedMesh` before it applies the bones.
///
/// Zero weights are skipped, as three.js does, so an unused target costs
/// nothing and cannot inject a `NaN`.
pub(crate) fn vertex_position(
    geometry: &BufferGeometry,
    morph_target_influences: &[f64],
    index: usize,
) -> Vector3 {
    let Some(position) = geometry.position() else {
        return Vector3::ZERO;
    };
    let mut target = position.get_vector3(index);

    if let Some(morph_position) = geometry.get_morph_attribute("position") {
        if !morph_target_influences.is_empty() {
            let mut morph = Vector3::ZERO;
            for (morph_attribute, &influence) in morph_position.iter().zip(morph_target_influences)
            {
                if influence == 0.0 {
                    continue;
                }
                let mut temp = morph_attribute.get_vector3(index);
                if geometry.morph_targets_relative {
                    morph.add_scaled_vector(&temp, influence);
                } else {
                    temp.sub(&target);
                    morph.add_scaled_vector(&temp, influence);
                }
            }
            target.add(&morph);
        }
    }
    target
}

/// What `Mesh.raycast()` and `_computeIntersections()` read off `this`, pulled
/// out so the subclasses that raycast *as* a mesh can supply their own:
/// `InstancedMesh` and `BatchedMesh` a per-instance world matrix (three.js
/// points a scratch `_mesh` at it), `BatchedMesh` a per-geometry draw range,
/// `SkinnedMesh` a bone-aware vertex position.
pub(crate) struct MeshRaycast<'a> {
    pub geometry: &'a BufferGeometry,
    /// `material.side`. The port's `Mesh` has one material, so three's
    /// `Array.isArray( material )` branch — one pass per geometry group, each
    /// with its group's material and `materialIndex` — collapses to the
    /// single-material loop, and `face.materialIndex` is always 0.
    pub side: Side,
    pub matrix_world: Matrix4,
    /// `geometry.drawRange.start`.
    pub draw_start: usize,
    /// `geometry.drawRange.count`, `None` for `Infinity`.
    pub draw_count: Option<usize>,
    /// `this.getVertexPosition( index, target )`.
    pub vertex_position: &'a dyn Fn(usize) -> Vector3,
}

impl MeshRaycast<'_> {
    /// `Mesh.raycast()` with `geometry.boundingSphere` = `sphere` and
    /// `geometry.boundingBox` = `bounding_box`, both in object space.
    pub fn raycast(
        &self,
        sphere: &Sphere,
        bounding_box: Option<&Box3>,
        object: &Node,
        raycaster: &Raycaster,
        intersects: &mut Vec<Intersection>,
    ) {
        // Test with the bounding sphere in world space.
        let mut sphere = *sphere;
        sphere.apply_matrix4(&self.matrix_world);

        // Check the distance from the ray origin to the bounding sphere.
        let mut ray = raycaster.ray;
        ray.recast(raycaster.near);
        if !sphere.contains_point(&ray.origin) {
            let Some(sphere_hit_at) = ray.intersect_sphere(&sphere) else {
                return;
            };
            if ray.origin.distance_to_squared(&sphere_hit_at)
                > (raycaster.far - raycaster.near).powi(2)
            {
                return;
            }
        }

        // Convert the ray to the local space of the mesh.
        let mut inverse_matrix = self.matrix_world;
        inverse_matrix.invert();
        let mut ray = raycaster.ray;
        ray.apply_matrix4(&inverse_matrix);

        // Test with the bounding box in local space.
        if let Some(bounding_box) = bounding_box {
            if !ray.intersects_box(bounding_box) {
                return;
            }
        }

        self.compute_intersections(raycaster, &ray, object, intersects);
    }

    /// `Mesh._computeIntersections( raycaster, intersects, rayLocalSpace )`.
    pub fn compute_intersections(
        &self,
        raycaster: &Raycaster,
        ray_local_space: &Ray,
        object: &Node,
        intersects: &mut Vec<Intersection>,
    ) {
        let geometry = self.geometry;
        let draw_end = |count: usize| match self.draw_count {
            Some(draw_count) => count.min(self.draw_start + draw_count),
            None => count,
        };

        let mut test = |i: usize, a: usize, b: usize, c: usize| {
            if let Some(mut intersection) =
                self.check_geometry_intersection(raycaster, ray_local_space, a, b, c, object)
            {
                // The triangle number, in indexed or non-indexed semantics.
                intersection.face_index = Some(i / 3);
                intersects.push(intersection);
            }
        };

        if let Some(index) = &geometry.index {
            let start = self.draw_start;
            let end = draw_end(index.count());
            // A trailing partial triangle reads past the index in three.js
            // and hits nothing (`undefined` vertices); here it is skipped.
            let mut i = start;
            while i < end && i + 2 < index.count() {
                test(i, index.get_x(i), index.get_x(i + 1), index.get_x(i + 2));
                i += 3;
            }
        } else if let Some(position) = geometry.position() {
            let start = self.draw_start;
            let end = draw_end(position.count());
            let mut i = start;
            while i < end && i + 2 < position.count() {
                test(i, i, i + 1, i + 2);
                i += 3;
            }
        }
    }

    /// `checkGeometryIntersection( object, material, raycaster, ray, uv, uv1,
    /// normal, a, b, c )`.
    fn check_geometry_intersection(
        &self,
        raycaster: &Raycaster,
        ray: &Ray,
        a: usize,
        b: usize,
        c: usize,
        object: &Node,
    ) -> Option<Intersection> {
        let v_a = (self.vertex_position)(a);
        let v_b = (self.vertex_position)(b);
        let v_c = (self.vertex_position)(c);

        // `checkIntersection( object, material, raycaster, ray, pA, pB, pC, point )`.
        let point = match self.side {
            Side::Back => ray.intersect_triangle(&v_c, &v_b, &v_a, true),
            side => ray.intersect_triangle(&v_a, &v_b, &v_c, side == Side::Front),
        }?;
        let mut point_world = point;
        point_world.apply_matrix4(&self.matrix_world);
        let distance = raycaster.ray.origin.distance_to(&point_world);
        if distance < raycaster.near || distance > raycaster.far {
            return None;
        }
        let mut intersection = Intersection::new(distance, point_world, object.clone());

        // `Triangle.getBarycoord()` leaves the target at ( 0, 0, 0 ) for a
        // degenerate triangle, which the ray cannot have hit anyway.
        let barycoord =
            Triangle::static_get_barycoord(&point, &v_a, &v_b, &v_c).unwrap_or(Vector3::ZERO);
        let geometry = self.geometry;
        let interpolate = |name: &str| {
            geometry.get_attribute(name).map(|attribute| {
                Triangle::static_get_interpolated_attribute(attribute, a, b, c, &barycoord)
            })
        };
        intersection.uv = interpolate("uv").map(|v| Vector2::new(v.x, v.y));
        intersection.uv1 = interpolate("uv1").map(|v| Vector2::new(v.x, v.y));
        intersection.normal = interpolate("normal").map(|v| {
            let mut normal = Vector3::new(v.x, v.y, v.z);
            if normal.dot(&ray.direction) > 0.0 {
                normal.multiply_scalar(-1.0);
            }
            normal
        });
        intersection.face = Some(Face {
            a,
            b,
            c,
            normal: Triangle::static_get_normal(&v_a, &v_b, &v_c),
            material_index: 0,
        });
        intersection.barycoord = Some(barycoord);
        Some(intersection)
    }
}
