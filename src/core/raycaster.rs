//! Port of `three.js/src/core/Raycaster.js`, together with the intersection
//! record every `Object3D.raycast()` fills in and the dispatch from a scene
//! node to its subclass's `raycast()`.
//!
//! three.js dispatches `object.raycast( raycaster, intersects )` through the
//! prototype chain. The port's subclasses live in a node's
//! [`Payload`](crate::objects::Payload), so [`Node::raycast`] matches on it and
//! calls the ported method of whichever subclass is there; a plain
//! `Object3D` / `Group` / light raycasts to nothing, as `Object3D.raycast()`
//! does.

use crate::cameras::RenderCamera;
use crate::core::{Layers, Node};
use crate::math::{Matrix4, Ray, Vector2, Vector3};

/// The camera state `Raycaster.camera` is read for — by `Sprite.raycast()`
/// and by `LineSegments2`'s screen-space raycast.
///
/// three.js stores a *reference* to the camera, so a raycast sees the camera
/// as it is when `intersectObject()` runs. The port's cameras are plain
/// structs rather than scene nodes both can point at, so
/// [`Raycaster::set_from_camera`] copies what those two methods read. A
/// camera that moves between `setFromCamera()` and `intersectObject()` is the
/// one case where the two differ; every three.js example sets the ray and
/// intersects in the same breath.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycasterCamera {
    /// `camera.isPerspectiveCamera`.
    pub is_perspective_camera: bool,
    /// `camera.isOrthographicCamera`.
    pub is_orthographic_camera: bool,
    /// `camera.near`.
    pub near: f64,
    /// `camera.far`.
    pub far: f64,
    /// `camera.matrixWorld`.
    pub matrix_world: Matrix4,
    /// `camera.matrixWorldInverse`.
    pub matrix_world_inverse: Matrix4,
    /// `camera.projectionMatrix`.
    pub projection_matrix: Matrix4,
    /// `camera.projectionMatrixInverse`.
    pub projection_matrix_inverse: Matrix4,
}

impl RaycasterCamera {
    /// Snapshot `camera`.
    pub fn of(camera: &impl RenderCamera) -> Self {
        Self {
            is_perspective_camera: camera.is_perspective_camera(),
            is_orthographic_camera: camera.is_orthographic_camera(),
            near: camera.near(),
            far: camera.far(),
            matrix_world: camera.matrix_world(),
            matrix_world_inverse: camera.matrix_world_inverse(),
            projection_matrix: camera.projection_matrix(),
            projection_matrix_inverse: camera.projection_matrix_inverse(),
        }
    }
}

/// One `{ threshold }` entry of `Raycaster.params`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Threshold {
    pub threshold: f64,
}

/// `Raycaster.params`.
///
/// three.js' object also has empty `Mesh`, `LOD` and `Sprite` entries, which
/// nothing reads; they are left out rather than ported as unit structs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycasterParams {
    /// `params.Line` — the world-space distance within which a ray hits a
    /// `Line` / `LineSegments`. Default 1.
    pub line: Threshold,
    /// `params.Points` — the same for `Points`. Default 1.
    pub points: Threshold,
    /// `params.Line2` — extra width added to a fat line's `linewidth` by
    /// `LineSegments2.raycast()`. three.js has no such entry until an app adds
    /// one (`raycaster.params.Line2 = { threshold: 0 }` in
    /// `webgpu_lines_fat_raycasting`), and the addon then reads
    /// `params.Line2?.threshold || 0`; 0 here is that absent entry.
    pub line2: Threshold,
}

impl Default for RaycasterParams {
    fn default() -> Self {
        Self {
            line: Threshold { threshold: 1.0 },
            points: Threshold { threshold: 1.0 },
            line2: Threshold { threshold: 0.0 },
        }
    }
}

/// `intersection.face` — the triangle a mesh ray hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    /// Vertex indices of the triangle.
    pub a: usize,
    pub b: usize,
    pub c: usize,
    /// `Triangle.getNormal()` of the (morphed, skinned) triangle, in object
    /// space.
    pub normal: Vector3,
    /// `face.materialIndex`. Always 0 in the port: see
    /// [`Mesh`](crate::objects::Mesh)'s raycast, which has a single material.
    pub material_index: usize,
}

/// One entry of the array `Raycaster.intersectObject()` returns.
///
/// three.js builds a plain object whose keys depend on what was hit; every key
/// that some `raycast()` can leave out is an `Option` here, `None` being both
/// three's absent key and its explicit `null`.
#[derive(Clone, Debug)]
pub struct Intersection {
    /// `distance` — from the ray origin to the hit, in world units.
    pub distance: f64,
    /// `distanceToRay` — `Points` only: how far the point is from the ray.
    pub distance_to_ray: Option<f64>,
    /// `point` — the hit in world space.
    pub point: Vector3,
    /// `pointOnLine` — `LineSegments2` only: the closest point on the segment.
    pub point_on_line: Option<Vector3>,
    /// `index` — `Line` / `Points` only: the segment's first vertex, or the
    /// point.
    pub index: Option<usize>,
    /// `face` — meshes only.
    pub face: Option<Face>,
    /// `faceIndex` — the triangle number for a mesh, the segment number for a
    /// `LineSegments2`.
    pub face_index: Option<usize>,
    /// `uv` at the hit — meshes with a `uv` attribute, and sprites.
    pub uv: Option<Vector2>,
    /// `uv1` at the hit.
    pub uv1: Option<Vector2>,
    /// `normal` — the interpolated vertex normal, flipped to face the ray.
    pub normal: Option<Vector3>,
    /// `barycoord` — the hit's barycentric coordinates in the triangle.
    pub barycoord: Option<Vector3>,
    /// `instanceId` — `InstancedMesh` only.
    pub instance_id: Option<usize>,
    /// `batchId` — `BatchedMesh` only.
    pub batch_id: Option<usize>,
    /// `object` — the object that was hit.
    pub object: Node,
}

impl Intersection {
    /// The fields every hit has; the rest start absent.
    pub fn new(distance: f64, point: Vector3, object: Node) -> Self {
        Self {
            distance,
            distance_to_ray: None,
            point,
            point_on_line: None,
            index: None,
            face: None,
            face_index: None,
            uv: None,
            uv1: None,
            normal: None,
            barycoord: None,
            instance_id: None,
            batch_id: None,
            object,
        }
    }
}

/// `Raycaster` — picks the objects a ray passes through.
#[derive(Clone, Debug)]
pub struct Raycaster {
    /// `raycaster.ray` — its direction is assumed normalised.
    pub ray: Ray,
    /// `raycaster.near` — hits closer than this are dropped. Default 0.
    pub near: f64,
    /// `raycaster.far` — hits farther than this are dropped. Default ∞.
    pub far: f64,
    /// `raycaster.camera`, set by [`set_from_camera`](Self::set_from_camera):
    /// `null` until then. See [`RaycasterCamera`] for why it is a copy.
    pub camera: Option<RaycasterCamera>,
    /// `raycaster.layers` — an object is only tested when its layers share one
    /// with these, though its children are still visited.
    pub layers: Layers,
    /// `raycaster.params`.
    pub params: RaycasterParams,
}

impl Default for Raycaster {
    /// `new Raycaster()` — the default `Ray`, near 0, far ∞.
    fn default() -> Self {
        let ray = Ray::default();
        Self::new(ray.origin, ray.direction, 0.0, f64::INFINITY)
    }
}

impl Raycaster {
    /// `new Raycaster( origin, direction, near, far )`.
    pub fn new(origin: Vector3, direction: Vector3, near: f64, far: f64) -> Self {
        Self {
            ray: Ray::new(origin, direction),
            near,
            far,
            camera: None,
            layers: Layers::new(),
            params: RaycasterParams::default(),
        }
    }

    /// `raycaster.set( origin, direction )` — `direction` is assumed
    /// normalised, for accurate distances.
    pub fn set(&mut self, origin: &Vector3, direction: &Vector3) {
        self.ray.set(origin, direction);
    }

    /// `raycaster.setFromCamera( coords, camera )` — a ray from the camera
    /// through `coords`, in normalised device coordinates (`-1 … 1` on both
    /// axes, `y` up).
    pub fn set_from_camera(&mut self, coords: &Vector2, camera: &impl RenderCamera) {
        if camera.is_perspective_camera() {
            self.ray
                .origin
                .set_from_matrix_position(&camera.matrix_world());
            let origin = self.ray.origin;
            self.ray.direction = Vector3::new(coords.x, coords.y, 0.5);
            self.ray
                .direction
                .unproject(camera)
                .sub(&origin)
                .normalize();
            self.camera = Some(RaycasterCamera::of(camera));
        } else if camera.is_orthographic_camera() {
            // Set the origin in the plane of the camera.
            let z = camera.projection_matrix().elements[14];
            self.ray.origin = Vector3::new(coords.x, coords.y, z);
            self.ray.origin.unproject(camera);
            self.ray.direction = Vector3::new(0.0, 0.0, -1.0);
            self.ray
                .direction
                .transform_direction(&camera.matrix_world());
            self.camera = Some(RaycasterCamera::of(camera));
        } else {
            eprintln!("THREE.Raycaster: Unsupported camera type.");
        }
    }

    /// `raycaster.intersectObject( object, recursive )` — every hit on
    /// `object`, and on its descendants when `recursive` (three's default),
    /// nearest first.
    pub fn intersect_object(&self, object: &Node, recursive: bool) -> Vec<Intersection> {
        let mut intersects = Vec::new();
        intersect(object, self, &mut intersects, recursive);
        sort(&mut intersects);
        intersects
    }

    /// `raycaster.intersectObjects( objects, recursive )`.
    pub fn intersect_objects(&self, objects: &[Node], recursive: bool) -> Vec<Intersection> {
        let mut intersects = Vec::new();
        for object in objects {
            intersect(object, self, &mut intersects, recursive);
        }
        sort(&mut intersects);
        intersects
    }
}

/// `intersects.sort( ascSort )`. `Array.prototype.sort` is stable, and so is
/// `sort_by`; a `NaN` distance compares equal, where JavaScript's comparator
/// would return `NaN` and leave the pair alone too.
fn sort(intersects: &mut [Intersection]) {
    intersects.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// `intersect( object, raycaster, intersects, recursive )`.
///
/// The layer test gates the object's own `raycast()` only: its children are
/// visited either way. three's `propagate` flag, which a `raycast()` that
/// returns `false` clears, exists for `LOD`, which is not ported.
fn intersect(
    object: &Node,
    raycaster: &Raycaster,
    intersects: &mut Vec<Intersection>,
    recursive: bool,
) {
    let tested = object.borrow().layers.test(&raycaster.layers);
    if tested {
        object.raycast(raycaster, intersects);
    }

    if recursive {
        for child in object.children() {
            intersect(&child, raycaster, intersects, true);
        }
    }
}

impl Node {
    /// `object.raycast( raycaster, intersects )` — push this object's hits
    /// (unsorted) onto `intersects`. Dispatches on the subclass in the
    /// payload; an object that draws nothing has nothing to hit.
    pub fn raycast(&self, raycaster: &Raycaster, intersects: &mut Vec<Intersection>) {
        use crate::objects::Payload;

        // `BatchedMesh.raycast()` lazily computes per-geometry bounds, and
        // `InstancedMesh` / `SkinnedMesh` their own bounding volumes, so the
        // borrow is mutable.
        let mut object = self.borrow_mut();
        let matrix_world = object.matrix_world;
        let scale = object.scale;
        match &mut object.payload {
            Payload::None | Payload::Light(_) => {}
            Payload::Mesh(mesh) => {
                if mesh.line_segments.is_some() {
                    crate::addons::lines::raycast(mesh, &matrix_world, self, raycaster, intersects);
                } else {
                    mesh.raycast(&matrix_world, self, raycaster, intersects);
                }
            }
            Payload::SkinnedMesh(skinned) => {
                skinned.raycast(&matrix_world, self, raycaster, intersects)
            }
            Payload::InstancedMesh(instanced) => {
                instanced.raycast(&matrix_world, self, raycaster, intersects)
            }
            Payload::BatchedMesh(batched) => {
                batched.raycast(&matrix_world, self, raycaster, intersects)
            }
            Payload::Line(line) => line.raycast(&matrix_world, &scale, self, raycaster, intersects),
            Payload::Points(points) => {
                points.raycast(&matrix_world, &scale, self, raycaster, intersects)
            }
            Payload::Sprite(sprite) => sprite.raycast(&matrix_world, self, raycaster, intersects),
        }
    }
}
