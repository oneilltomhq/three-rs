//! Port of `three.js/src/renderers/common/RenderList.js` plus
//! `Renderer._projectObject()` — the scene walk that turns a tree of
//! [`Object3D`](crate::core::Object3D) nodes into the ordered lists
//! `Renderer.render()` draws.
//!
//! This is the whole of the renderer that is GPU-free, so it is unit-tested
//! directly (`cargo test -p three-rs --lib`).

use std::cmp::Ordering;

use crate::cameras::RenderCamera;
use crate::core::{Layers, Node, Object3DNode};
use crate::math::{CoordinateSystem, Frustum, Matrix4};

/// One entry of `RenderList.opaque` / `RenderList.transparent`.
///
/// three.js' render item also carries `geometry`, `material`, `group` and the
/// clipping context; geometry and material are reachable through
/// `node.borrow().mesh()`, and the other two belong to features this port has
/// not reached.
#[derive(Clone)]
pub struct RenderItem {
    /// `renderItem.object`.
    pub node: Node,
    /// `renderItem.id` — `object.id`, the final sort tie-break.
    pub id: u32,
    /// `renderItem.groupOrder` — the `renderOrder` of the nearest `Group`
    /// ancestor, 0 at the scene root.
    pub group_order: f64,
    /// `renderItem.renderOrder` — `object.renderOrder`.
    pub render_order: f64,
    /// `renderItem.z` — the geometry's bounding-sphere centre in clip space,
    /// unnormalised (three.js keeps `Vector4.z`, with no perspective divide).
    pub z: f64,
    /// `object.matrixWorld` at the time of the walk.
    pub matrix_world: Matrix4,
}

/// `painterSortStable( a, b )`.
fn painter_sort_stable(a: &RenderItem, b: &RenderItem) -> Ordering {
    compare(a.group_order, b.group_order)
        .then_with(|| compare(a.render_order, b.render_order))
        .then_with(|| compare(a.z, b.z))
        .then_with(|| a.id.cmp(&b.id))
}

/// `reversePainterSortStable( a, b )` — the same, with `z` reversed.
fn reverse_painter_sort_stable(a: &RenderItem, b: &RenderItem) -> Ordering {
    compare(a.group_order, b.group_order)
        .then_with(|| compare(a.render_order, b.render_order))
        .then_with(|| compare(b.z, a.z))
        .then_with(|| a.id.cmp(&b.id))
}

/// `a - b` as a JS comparator sees it. `z` can be any finite float here, and a
/// NaN would come from a degenerate projection matrix; JS treats a NaN
/// comparator result as "equal", so `Ordering::Equal` is the faithful answer.
fn compare(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// `class RenderList`, minus the bundles, the transmissive double pass and the
/// occlusion-query bookkeeping.
#[derive(Default)]
pub struct RenderList {
    /// `RenderList.opaque`.
    pub opaque: Vec<RenderItem>,
    /// `RenderList.transparent`.
    pub transparent: Vec<RenderItem>,
    /// `RenderList.lightsArray` — every visible light in the tree, in traversal
    /// order, which is what `LightsNode.setLights()` receives.
    pub lights: Vec<Node>,
}

impl RenderList {
    /// `RenderList.begin()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `RenderList.push( object, geometry, material, groupOrder, z, … )`.
    fn push(&mut self, item: RenderItem, transparent: bool) {
        if transparent {
            self.transparent.push(item);
        } else {
            self.opaque.push(item);
        }
    }

    /// `RenderList.pushLight( light )`.
    fn push_light(&mut self, light: Node) {
        self.lights.push(light);
    }

    /// `RenderList.sort( null, null )`. `slice::sort_by` is stable, as
    /// `Array.prototype.sort` is.
    pub fn sort(&mut self) {
        if self.opaque.len() > 1 {
            self.opaque.sort_by(painter_sort_stable);
        }
        if self.transparent.len() > 1 {
            self.transparent.sort_by(reverse_painter_sort_stable);
        }
    }

    /// The draw order `Renderer._renderScene()` uses: all the opaque items, then
    /// all the transparent ones.
    pub fn items(&self) -> impl Iterator<Item = &RenderItem> {
        self.opaque.iter().chain(self.transparent.iter())
    }

    /// Total drawable count.
    pub fn len(&self) -> usize {
        self.opaque.len() + self.transparent.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The camera state `_projectObject()` reads: `camera.layers` plus the
/// projection used for the sort `z` and for frustum culling.
pub struct ProjectCamera {
    /// `camera.layers`.
    pub layers: Layers,
    /// `_projScreenMatrix` — `projectionMatrix * matrixWorldInverse`.
    pub proj_screen_matrix: Matrix4,
    /// `_frustum`, set from `_projScreenMatrix`.
    pub frustum: Frustum,
}

impl ProjectCamera {
    /// `_projScreenMatrix.multiplyMatrices( camera.projectionMatrix,
    /// camera.matrixWorldInverse ); _frustum.setFromProjectionMatrix(
    /// _projScreenMatrix, camera.coordinateSystem, camera.reversedDepth )`.
    ///
    /// `camera.matrixWorldInverse` must already be up to date — in three.js
    /// `Renderer.render()` calls `camera.updateMatrixWorld()` just above this.
    /// The same thing for a camera the port models without an `Object3D`
    /// wrapper — the shadow cameras, whose projection and world-inverse
    /// matrices `LightShadow.updateMatrices()` has just refreshed.
    pub fn from_parts(
        layers: Layers,
        projection_matrix: &Matrix4,
        matrix_world_inverse: &Matrix4,
        coordinate_system: CoordinateSystem,
    ) -> Self {
        let mut proj_screen_matrix = Matrix4::identity();
        proj_screen_matrix.multiply_matrices(projection_matrix, matrix_world_inverse);

        let mut frustum = Frustum::default();
        frustum.set_from_projection_matrix(&proj_screen_matrix, coordinate_system, false);

        Self {
            layers,
            proj_screen_matrix,
            frustum,
        }
    }

    pub fn new(camera: &dyn RenderCamera) -> Self {
        let mut proj_screen_matrix = Matrix4::identity();
        proj_screen_matrix
            .multiply_matrices(&camera.projection_matrix(), &camera.matrix_world_inverse());

        let mut frustum = Frustum::default();
        // `reversedDepth` is off: `Renderer.reversedDepth` defaults to false and
        // nothing in the ladder turns it on.
        frustum.set_from_projection_matrix(&proj_screen_matrix, camera.coordinate_system(), false);

        Self {
            layers: camera.layers(),
            proj_screen_matrix,
            frustum,
        }
    }
}

/// `Renderer._projectObject( object, camera, groupOrder, renderList, … )`.
///
/// Walks the subtree under `object` and pushes every visible drawable into
/// `render_list`, with the `z` the sort needs. `sort_objects` mirrors
/// `Renderer.sortObjects`: with it off, three.js leaves `z` at whatever
/// `_vector4` last held, so here it stays 0 and the lists keep traversal order.
///
/// Note which gates stop the recursion and which do not, because three.js is
/// deliberately asymmetric: `visible === false` returns immediately, so a hidden
/// parent hides its whole subtree, while failing the `layers` test only skips
/// *this* object's own render item — its children are still projected.
pub fn project_object(
    object: &Node,
    camera: &ProjectCamera,
    group_order: f64,
    render_list: &mut RenderList,
    sort_objects: bool,
) {
    if !object.borrow().visible {
        return;
    }

    let mut group_order = group_order;

    let visible = object.borrow().layers.test(&camera.layers);

    if visible {
        let (is_group, is_light, is_drawable) = {
            let o = object.borrow();
            (o.is_group, o.is_light, o.is_mesh() || o.is_line())
        };

        if is_group {
            group_order = object.borrow().render_order;
        } else if is_light {
            render_list.push_light(object.clone());
        } else if is_drawable {
            project_drawable(object, camera, group_order, render_list, sort_objects);
        }
    }

    for child in object.children() {
        project_object(&child, camera, group_order, render_list, sort_objects);
    }
}

/// The `object.isMesh || object.isLine || object.isPoints` arm of
/// `_projectObject()` — one arm in three.js too, because everything it does
/// reads `object.geometry` and `object.material` and neither the frustum test
/// nor the sort `z` cares which primitive the object draws.
///
/// `object.isLineLoop` is *not* handled: three.js' own arm above this one calls
/// `error( 'Renderer: Objects of type THREE.LineLoop are not supported…' )`, so
/// the port has no `LineLoop` to reach here.
fn project_drawable(
    object: &Node,
    camera: &ProjectCamera,
    group_order: f64,
    render_list: &mut RenderList,
    sort_objects: bool,
) {
    let o = object.borrow();
    let geometry = match o.geometry() {
        Some(geometry) => geometry.clone(),
        None => return,
    };

    // `if ( ! object.frustumCulled || object.intersectsFrustum( _frustum ) )` —
    // `Mesh.intersectsFrustum` / `Line.intersectsFrustum` are both
    // `frustum.intersectsObject( this )`, the geometry's bounding sphere pushed
    // through `matrixWorld`.
    if o.frustum_culled {
        let inside = match o.payload.bounding_sphere_in(&o.matrix_world) {
            Some(sphere) => camera.frustum.intersects_sphere(&sphere),
            // A geometry with no position attribute has no bounding sphere;
            // three.js would throw, we treat it as nothing to draw.
            None => false,
        };

        if !inside {
            return;
        }
    }

    // `_vector4.copy( geometry.boundingSphere.center )
    //      .applyMatrix4( object.matrixWorld ).applyMatrix4( _projScreenMatrix )`
    let z = if sort_objects {
        let center = geometry.bounding_sphere_center();
        let v = apply_matrix4_vector4(&o.matrix_world, [center.x, center.y, center.z, 1.0]);
        apply_matrix4_vector4(&camera.proj_screen_matrix, v)[2]
    } else {
        0.0
    };

    // `if ( material.visible ) renderList.push( … )`. A mesh with no material of
    // its own is drawn with `scene.overrideMaterial`, which three.js substitutes
    // later (in `_renderObjects`), after the list is built — so a missing
    // material is not a reason to skip the object here.
    let (visible, transparent) = match o.material() {
        Some(material) => (material.visible, material.transparent),
        None => (true, false),
    };

    if !visible {
        return;
    }

    let item = RenderItem {
        node: object.clone(),
        id: o.id,
        group_order,
        render_order: o.render_order,
        z,
        matrix_world: o.matrix_world,
    };

    drop(o);
    render_list.push(item, transparent);
}

/// `Vector4.applyMatrix4()` — no perspective divide, unlike `Vector3`'s.
fn apply_matrix4_vector4(m: &Matrix4, v: [f64; 4]) -> [f64; 4] {
    let e = &m.elements;
    [
        e[0] * v[0] + e[4] * v[1] + e[8] * v[2] + e[12] * v[3],
        e[1] * v[0] + e[5] * v[1] + e[9] * v[2] + e[13] * v[3],
        e[2] * v[0] + e[6] * v[1] + e[10] * v[2] + e[14] * v[3],
        e[3] * v[0] + e[7] * v[1] + e[11] * v[2] + e[15] * v[3],
    ]
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::cameras::PerspectiveCamera;
    use crate::core::BufferGeometry;
    use crate::geometries::box_geometry;
    use crate::lights::PointLight;
    use crate::materials::MeshBasicNodeMaterial;
    use crate::math::{Color, Vector3};
    use crate::objects::{Group, Mesh, Scene};

    /// A camera looking down -Z from z = 10, with everything in its frustum.
    fn camera() -> PerspectiveCamera {
        let mut camera = PerspectiveCamera::new(70.0, 2.0, 0.1, 100.0);
        camera.node.borrow_mut().position.z = 10.0;
        camera.update_matrix_world();
        camera
    }

    fn unit_box() -> Rc<BufferGeometry> {
        Rc::new(box_geometry(1.0, 1.0, 1.0, 1, 1, 1))
    }

    fn mesh_at(z: f64) -> Node {
        let node = Mesh::new(unit_box());
        node.borrow_mut().mesh_mut().unwrap().material = Some(MeshBasicNodeMaterial::new());
        node.borrow_mut().position.z = z;
        node
    }

    fn world_x(node: &Node) -> f64 {
        let mut position = Vector3::ZERO;
        position.set_from_matrix_position(&node.borrow().matrix_world);
        position.x
    }

    fn project(scene: &Scene, camera: &PerspectiveCamera) -> RenderList {
        scene.update_matrix_world();
        let mut list = RenderList::new();
        project_object(
            &scene.node,
            &ProjectCamera::new(camera),
            0.0,
            &mut list,
            true,
        );
        list.sort();
        list
    }

    #[test]
    fn collects_meshes_nested_under_groups() {
        let scene = Scene::new();
        let group = Group::new();
        let inner = Group::new();

        let a = mesh_at(0.0);
        let b = mesh_at(1.0);

        scene.add(&group);
        group.add(&inner);
        inner.add(&a);
        scene.add(&b);

        let list = project(&scene, &camera());

        assert_eq!(list.len(), 2, "a mesh two groups deep is still collected");
        assert!(list.transparent.is_empty());
    }

    #[test]
    fn group_world_matrix_reaches_the_nested_mesh() {
        let scene = Scene::new();
        let group = Group::new();
        group.borrow_mut().position.set(3.0, 0.0, 0.0);

        let mesh = mesh_at(0.0);
        mesh.borrow_mut().position.x = 1.0;

        scene.add(&group);
        group.add(&mesh);

        let list = project(&scene, &camera());

        assert_eq!(list.len(), 1);
        let world = list.opaque[0].matrix_world;
        let mut position = Vector3::ZERO;
        position.set_from_matrix_position(&world);
        assert_eq!(position.x, 4.0, "the group's translation is composed in");
    }

    /// `Scene::update_matrix_world` is `Object3D.updateMatrixWorld()` on the
    /// root, so the flags three.js honours there hold for the render path too.
    /// The per-object semantics are covered by `tests/core_object3d.rs`; this is
    /// the scene → group → mesh chain the renderer actually walks.
    #[test]
    fn scene_update_matrix_world_honours_the_auto_update_flags() {
        let scene = Scene::new();
        let group = Group::new();
        let mesh = mesh_at(0.0);

        scene.add(&group);
        group.add(&mesh);

        group.borrow_mut().position.x = 1.0;
        mesh.borrow_mut().position.x = 2.0;
        scene.update_matrix_world();
        assert_eq!(world_x(&mesh), 3.0, "composed through the group");

        // `matrixAutoUpdate = false`: the local matrix is left alone, so nothing
        // downstream moves either.
        group.borrow_mut().matrix_auto_update = false;
        group.borrow_mut().position.x = 10.0;
        scene.update_matrix_world();
        assert_eq!(world_x(&mesh), 3.0, "matrixAutoUpdate = false pins it");

        // …until something raises `matrixWorldNeedsUpdate`, which is what
        // `updateMatrix()` (and the animation setters) do.
        group.borrow_mut().update_matrix();
        scene.update_matrix_world();
        assert_eq!(
            world_x(&mesh),
            12.0,
            "matrixWorldNeedsUpdate forces the walk"
        );

        // `matrixWorldAutoUpdate = false` freezes this object's world matrix and,
        // because its `force` return is unaffected, still lets children recompose
        // against the stale value.
        mesh.borrow_mut().matrix_world_auto_update = false;
        mesh.borrow_mut().position.x = 5.0;
        scene.update_matrix_world();
        assert_eq!(
            world_x(&mesh),
            12.0,
            "matrixWorldAutoUpdate = false freezes it"
        );
    }

    #[test]
    fn invisible_parent_hides_the_whole_subtree() {
        let scene = Scene::new();
        let group = Group::new();
        group.borrow_mut().visible = false;
        scene.add(&group);
        group.add(&mesh_at(0.0));

        assert_eq!(project(&scene, &camera()).len(), 0);
    }

    #[test]
    fn a_layers_miss_skips_the_object_but_not_its_children() {
        // `_projectObject`: the `layers.test` gate wraps only the push, so a
        // child on the camera's layer is still reached through a parent that is
        // not. (An invisible parent, by contrast, returns early.)
        let scene = Scene::new();

        let outer = mesh_at(0.0);
        outer.borrow_mut().layers.set(1);

        let inner = mesh_at(1.0);
        outer.add(&inner);
        scene.add(&outer);

        let list = project(&scene, &camera());

        assert_eq!(list.len(), 1, "only the child is drawn");
        assert_eq!(list.opaque[0].id, inner.borrow().id);
    }

    #[test]
    fn frustum_culling_drops_an_object_behind_the_camera() {
        let scene = Scene::new();
        let behind = mesh_at(40.0); // camera sits at z = 10 looking at -Z
        scene.add(&behind);

        assert_eq!(project(&scene, &camera()).len(), 0, "culled");

        behind.borrow_mut().frustum_culled = false;
        assert_eq!(
            project(&scene, &camera()).len(),
            1,
            "frustumCulled = false keeps it"
        );
    }

    #[test]
    fn opaque_sorts_front_to_back_and_transparent_back_to_front() {
        let scene = Scene::new();

        // Nearer the camera (z = 10) means a smaller clip-space z.
        let near = mesh_at(4.0);
        let far = mesh_at(-4.0);
        scene.add(&far);
        scene.add(&near);

        let list = project(&scene, &camera());
        assert_eq!(
            [list.opaque[0].id, list.opaque[1].id],
            [near.borrow().id, far.borrow().id],
            "painterSortStable: ascending z"
        );

        for node in [&near, &far] {
            node.borrow_mut()
                .mesh_mut()
                .unwrap()
                .material
                .as_mut()
                .unwrap()
                .transparent = true;
        }

        let list = project(&scene, &camera());
        assert!(list.opaque.is_empty());
        assert_eq!(
            [list.transparent[0].id, list.transparent[1].id],
            [far.borrow().id, near.borrow().id],
            "reversePainterSortStable: descending z"
        );
    }

    #[test]
    fn render_order_and_group_order_outrank_z() {
        let scene = Scene::new();

        let near = mesh_at(4.0);
        near.borrow_mut().render_order = 1.0;
        let far = mesh_at(-4.0);

        scene.add(&near);
        scene.add(&far);

        let list = project(&scene, &camera());
        assert_eq!(
            [list.opaque[0].id, list.opaque[1].id],
            [far.borrow().id, near.borrow().id],
            "renderOrder beats z"
        );

        // `groupOrder` comes from the nearest `Group` ancestor's `renderOrder`
        // and beats `renderOrder` in turn.
        let group = Group::new();
        group.borrow_mut().render_order = -1.0;
        scene.add(&group);
        group.add(&near);

        let list = project(&scene, &camera());
        assert_eq!(list.opaque[0].group_order, -1.0);
        assert_eq!(
            [list.opaque[0].id, list.opaque[1].id],
            [near.borrow().id, far.borrow().id],
            "groupOrder beats renderOrder"
        );
    }

    #[test]
    fn an_invisible_material_is_not_pushed() {
        let scene = Scene::new();
        let mesh = mesh_at(0.0);
        mesh.borrow_mut()
            .mesh_mut()
            .unwrap()
            .material
            .as_mut()
            .unwrap()
            .visible = false;
        scene.add(&mesh);

        assert_eq!(project(&scene, &camera()).len(), 0);
    }

    #[test]
    fn lights_are_collected_and_not_drawn_but_their_children_are() {
        let scene = Scene::new();

        let light = PointLight::new(Color::new(1.0, 1.0, 1.0), 1.0, 100.0);
        let bulb = mesh_at(0.0);

        scene.add(&light);
        light.add(&bulb);

        let list = project(&scene, &camera());

        assert_eq!(list.lights.len(), 1, "the light is pushed as a light");
        assert_eq!(list.lights[0].borrow().id, light.borrow().id);
        assert_eq!(list.len(), 1, "and its bulb mesh is still drawn");
        assert_eq!(list.opaque[0].id, bulb.borrow().id);

        // `PointLight.matrixWorld` is what the renderer reads the light's world
        // position from, and the bulb is an ordinary child, so it composes.
        light.borrow_mut().position.x = 2.0;
        let list = project(&scene, &camera());
        assert_eq!(world_x(&list.lights[0]), 2.0);
        assert_eq!(world_x(&bulb), 2.0, "the bulb rides the light's matrix");
    }

    #[test]
    fn unsorted_keeps_traversal_order() {
        let scene = Scene::new();
        let far = mesh_at(-4.0);
        let near = mesh_at(4.0);
        scene.add(&far);
        scene.add(&near);
        scene.update_matrix_world();

        let mut list = RenderList::new();
        project_object(
            &scene.node,
            &ProjectCamera::new(&camera()),
            0.0,
            &mut list,
            false,
        );
        list.sort();

        assert_eq!(
            [list.opaque[0].id, list.opaque[1].id],
            [far.borrow().id, near.borrow().id],
            "with sortObjects off, z stays 0 and the id tie-break holds add order"
        );
    }
}
