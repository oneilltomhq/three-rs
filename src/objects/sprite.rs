//! Port of `three.js/src/objects/Sprite.js`.
//!
//! A sprite is a quad that always faces the camera. The facing is done in the
//! vertex shader by [`SpriteNodeMaterial`](crate::materials::SpriteNodeMaterial)'s
//! `setupPositionView()`; the object itself only carries the material, the
//! `center` anchor, and a geometry every sprite shares.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Intersection, Node, Object3D, Raycaster};
use crate::materials::{MeshBasicNodeMaterial, SpriteNodeMaterial};
use crate::math::{Matrix4, Sphere, Triangle, Vector2, Vector3};
use crate::objects::Payload;

thread_local! {
    /// `Sprite.js`' module-level `_geometry`, built by the first `new Sprite()`
    /// and shared by every sprite after it. Thread-local because the scene is
    /// single-threaded (`docs/api.md` §1) and `Rc` cannot be a `static`.
    static GEOMETRY: Rc<BufferGeometry> = Rc::new(sprite_geometry());
}

/// The unit quad, `-0.5 … 0.5` in `x` and `y`, two triangles.
///
/// three.js packs `position` and `uv` into one `InterleavedBuffer` of stride
/// 5; the port has no interleaved attributes on `BufferGeometry` yet (see
/// `docs/webgpu_lines_fat-progress.md`), so they are two plain attributes. The
/// WGSL is the same either way — the stride is vertex-buffer layout, not
/// shader — and the values are three's, in three's order.
fn sprite_geometry() -> BufferGeometry {
    #[rustfmt::skip]
    let interleaved: [f32; 20] = [
        -0.5, -0.5, 0.0, 0.0, 0.0,
         0.5, -0.5, 0.0, 1.0, 0.0,
         0.5,  0.5, 0.0, 1.0, 1.0,
        -0.5,  0.5, 0.0, 0.0, 1.0,
    ];
    let position: Vec<f32> = interleaved
        .chunks(5)
        .flat_map(|v| [v[0], v[1], v[2]])
        .collect();
    let uv: Vec<f32> = interleaved.chunks(5).flat_map(|v| [v[3], v[4]]).collect();

    let mut geometry = BufferGeometry::new();
    geometry.set_index(&[0, 1, 2, 0, 2, 3]);
    geometry.set_attribute("position", BufferAttribute::new(position, 3));
    geometry.set_attribute("uv", BufferAttribute::new(uv, 2));
    geometry
}

/// The state `Sprite` adds to `Object3D`. It lives in a node's [`Payload`], as
/// `Mesh`'s does — see `docs/scene-graph.md`.
#[derive(Clone)]
pub struct Sprite {
    /// `Sprite.geometry` — the shared quad, the same `Rc` for every sprite.
    pub geometry: Rc<BufferGeometry>,
    /// `Sprite.material`.
    pub material: SpriteNodeMaterial,
    /// `Sprite.center` — the sprite's anchor point in its own quad, `( 0.5,
    /// 0.5 )` being the middle and `( 0, 0 )` the bottom left. Read by the
    /// vertex shader through a `reference( 'center', 'vec2', object )` uniform,
    /// by `raycast()`, and by `Frustum.intersectsSprite()`.
    pub center: Vector2,
}

impl Sprite {
    /// `new Sprite( material )`, as a scene-graph [`Node`].
    ///
    /// `None` is three's default argument, `new SpriteMaterial()`, which under
    /// `WebGPURenderer` is a `SpriteNodeMaterial`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns a scene-graph `Node`, not `Self`; public API, not changing.
    pub fn new(material: impl Into<Option<SpriteNodeMaterial>>) -> Node {
        let mut object = Object3D {
            object_type: "Sprite",
            ..Default::default()
        };
        object.payload = Payload::Sprite(Self::of(material.into()));
        object.into_node()
    }

    /// The `Sprite` state alone, for a caller that already has the node.
    pub fn of(material: Option<SpriteNodeMaterial>) -> Self {
        Self {
            geometry: GEOMETRY.with(Rc::clone),
            material: material.unwrap_or_else(MeshBasicNodeMaterial::sprite),
            center: Vector2::new(0.5, 0.5),
        }
    }

    /// The sphere `Frustum.intersectsSprite( sprite )` tests: radius
    /// `√½` — the quad's half diagonal — grown by how far `center` is from the
    /// middle, at the object's origin, pushed through `matrixWorld`.
    pub fn bounding_sphere_in(&self, matrix_world: &Matrix4) -> Sphere {
        let default_center = Vector2::new(0.5, 0.5);
        let offset = default_center.distance_to(&self.center);

        let mut sphere = Sphere::new(
            Vector3::new(0.0, 0.0, 0.0),
            std::f64::consts::FRAC_1_SQRT_2 + offset,
        );
        sphere.apply_matrix4(matrix_world);
        sphere
    }

    /// `Sprite.raycast( raycaster, intersects )` — the quad as the vertex
    /// shader places it, facing `raycaster.camera`, tested as two triangles
    /// with no backface culling.
    ///
    /// three.js logs an error when `raycaster.camera` is `null` and then
    /// throws on reading it; the port logs the same message and hits nothing.
    pub fn raycast(
        &self,
        matrix_world: &Matrix4,
        object: &Node,
        raycaster: &Raycaster,
        intersects: &mut Vec<Intersection>,
    ) {
        let Some(camera) = &raycaster.camera else {
            eprintln!(
                "THREE.Sprite: \"Raycaster.camera\" needs to be set in order to raycast against sprites."
            );
            return;
        };

        let mut world_scale = Vector3::ZERO;
        world_scale.set_from_matrix_scale(matrix_world);
        let view_world_matrix = camera.matrix_world;
        let mut model_view_matrix = Matrix4::identity();
        model_view_matrix.multiply_matrices(&camera.matrix_world_inverse, matrix_world);
        let mut mv_position = Vector3::ZERO;
        mv_position.set_from_matrix_position(&model_view_matrix);

        if camera.is_perspective_camera && !self.material.size_attenuation {
            world_scale.multiply_scalar(-mv_position.z);
        }

        let rotation = self.material.rotation;
        let sin_cos = (rotation != 0.0).then(|| (rotation.sin(), rotation.cos()));

        let center = self.center;
        let vertex = |x: f64, y: f64| {
            transform_vertex(
                Vector2::new(x, y),
                &mv_position,
                &center,
                &world_scale,
                sin_cos,
                &view_world_matrix,
            )
        };
        let v_a = vertex(-0.5, -0.5);
        let mut v_b = vertex(0.5, -0.5);
        let v_c = vertex(0.5, 0.5);
        let uv_a = Vector3::new(0.0, 0.0, 0.0);
        let mut uv_b = Vector3::new(1.0, 0.0, 0.0);
        let uv_c = Vector3::new(1.0, 1.0, 0.0);

        // Check the first triangle, then the second.
        let intersect_point = match raycaster.ray.intersect_triangle(&v_a, &v_b, &v_c, false) {
            Some(point) => point,
            None => {
                v_b = vertex(-0.5, 0.5);
                uv_b = Vector3::new(0.0, 1.0, 0.0);
                match raycaster.ray.intersect_triangle(&v_a, &v_c, &v_b, false) {
                    Some(point) => point,
                    None => return,
                }
            }
        };

        let distance = raycaster.ray.origin.distance_to(&intersect_point);
        if distance < raycaster.near || distance > raycaster.far {
            return;
        }

        let mut intersection = Intersection::new(distance, intersect_point, object.clone());
        intersection.uv = Triangle::static_get_interpolation(
            &intersect_point,
            &v_a,
            &v_b,
            &v_c,
            &uv_a,
            &uv_b,
            &uv_c,
        )
        .map(|uv| Vector2::new(uv.x, uv.y));
        intersects.push(intersection);
    }
}

/// `Sprite.js`' `transformVertex( vertexPosition, mvPosition, center, scale,
/// sin, cos )`: a quad corner to world space, the way
/// `SpriteNodeMaterial.setupPositionView()` places it in view space.
fn transform_vertex(
    vertex_position: Vector2,
    mv_position: &Vector3,
    center: &Vector2,
    scale: &Vector3,
    sin_cos: Option<(f64, f64)>,
    view_world_matrix: &Matrix4,
) -> Vector3 {
    // Compute the position in camera space.
    let aligned_x = (vertex_position.x - center.x + 0.5) * scale.x;
    let aligned_y = (vertex_position.y - center.y + 0.5) * scale.y;

    let (rotated_x, rotated_y) = match sin_cos {
        Some((sin, cos)) => (
            cos * aligned_x - sin * aligned_y,
            sin * aligned_x + cos * aligned_y,
        ),
        None => (aligned_x, aligned_y),
    };

    let mut position = *mv_position;
    position.x += rotated_x;
    position.y += rotated_y;

    // Transform to world space.
    position.apply_matrix4(view_world_matrix);
    position
}
