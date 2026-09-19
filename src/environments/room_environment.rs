//! Port of `three.js/examples/jsm/environments/RoomEnvironment.js`.

use crate::core::Object3D;
use crate::geometries::box_geometry_default;
use crate::lights::PointLight;
use crate::materials::{MeshBasicNodeMaterial, Side};
use crate::math::{Color, Matrix4};
use crate::objects::{InstancedMesh, Mesh, Payload, Scene};

use std::rc::Rc;

/// `new RoomEnvironment()` — a scene with a basic room setup, built to be the
/// input of [`PmremGenerator::from_scene`] and nothing else.
///
/// Three's own class `extends Scene`; the port returns the [`Scene`], because
/// [`Scene`] is not a trait and decision 3 of `docs/api.md` keeps three's
/// shape only where there is no real trade-off. Nothing downstream needs the
/// subclass: `from_scene` takes a `&mut Scene`.
///
/// The construction is **fifteen literal transforms, one light and eight
/// materials**, ported from the JS by hand, so it is gated numerically rather
/// than by eye: `tests/room_environment.rs` compares every object's local
/// matrix, every instance matrix, the light's four parameters and the eight
/// materials' fields against `tests/fixtures/room_environment.json`, which was
/// printed by running three's own class under node. Nothing here derives from
/// a reference image.
///
/// The six emissive panels are `MeshLambertMaterial( { color: 0x000000,
/// emissive: 0xffffff, emissiveIntensity } )` — three's `createAreaLightMaterial`,
/// whose comment reads "create an emissive-only material. see #31348". Their
/// diffuse colour is black, so the Lambert lighting term is multiplied by zero
/// and contributes nothing; the material is still a
/// [`MeshLambertNodeMaterial`] and not a cheaper one, because the *generated
/// WGSL* has to match three's — see `docs/nodes.md` §22.
///
/// [`PmremGenerator::from_scene`]: crate::renderer::pmrem::PmremGenerator::from_scene
/// [`MeshLambertNodeMaterial`]: crate::materials::MeshLambertNodeMaterial
pub struct RoomEnvironment;

/// `createAreaLightMaterial( intensity )`.
fn area_light_material(intensity: f64) -> MeshBasicNodeMaterial {
    MeshBasicNodeMaterial {
        emissive: Color::from_hex(0xffffff),
        emissive_intensity: intensity,
        ..MeshBasicNodeMaterial::lambert(Color::from_hex(0x000000))
    }
}

/// `transform.position / rotation / scale; transform.updateMatrix()` — the
/// throwaway `Object3D` the JS composes each instance matrix through. Only the
/// Y rotation is ever non-zero, but the composition goes through
/// `Object3D.updateMatrix()` so that a drift in the port's TRS order fails
/// here rather than in the atlas.
fn instance_matrix(position: [f64; 3], rotation_y: f64, scale: [f64; 3]) -> Matrix4 {
    let mut transform = Object3D::default();
    transform
        .position
        .set(position[0], position[1], position[2]);
    transform.set_rotation(0.0, rotation_y, 0.0);
    transform.scale.set(scale[0], scale[1], scale[2]);
    transform.update_matrix();
    transform.matrix
}

/// The six `boxes.setMatrixAt( i, transform.matrix )` rows, in the JS's order.
const BOXES: [([f64; 3], f64, [f64; 3]); 6] = [
    ([-10.906, 2.009, 1.846], -0.195, [2.328, 7.905, 4.651]),
    ([-5.607, -0.754, -0.758], 0.994, [1.970, 1.534, 3.955]),
    ([6.167, 0.857, 7.803], 0.561, [3.927, 6.285, 3.687]),
    ([-2.017, 0.018, 6.124], 0.333, [2.002, 4.566, 2.064]),
    ([2.291, -0.756, -2.621], -0.286, [1.546, 1.552, 1.496]),
    ([-2.193, -0.369, -5.547], 0.516, [3.875, 3.487, 2.986]),
];

/// The six emissive panels: `( intensity, position, scale )`, in the JS's
/// order — `-x right`, `-x left`, `+x`, `+z`, `-z`, `+y`.
const PANELS: [(f64, [f64; 3], [f64; 3]); 6] = [
    (50.0, [-16.116, 14.37, 8.208], [0.1, 2.428, 2.739]),
    (50.0, [-16.109, 18.021, -8.207], [0.1, 2.425, 2.751]),
    (17.0, [14.904, 12.198, -1.832], [0.15, 4.265, 6.331]),
    (43.0, [-0.462, 8.89, 14.520], [4.38, 5.441, 0.088]),
    (20.0, [3.235, 11.486, -12.541], [2.5, 2.0, 0.1]),
    (100.0, [0.0, 20.0, 0.0], [1.0, 0.1, 1.0]),
];

impl RoomEnvironment {
    /// `new RoomEnvironment()`.
    #[allow(clippy::new_ret_no_self)] // `new` mirrors three.js's constructor and returns the `Scene` it builds, not `Self`.
    pub fn new() -> Scene {
        let scene = Scene::default();
        {
            let mut root = scene.node.borrow_mut();
            root.name = "RoomEnvironment".to_string();
            root.position.y = -3.5;
        }

        // `const geometry = new BoxGeometry(); geometry.deleteAttribute( 'uv' );`
        // — one geometry, shared by the room, the instanced boxes and the six
        // panels. The `uv` delete is not cosmetic: it is what leaves the
        // vertex stage of `dump-postprocessing_ca/m06` taking `normal` and
        // `position` and nothing else.
        let mut geometry = box_geometry_default();
        geometry.delete_attribute("uv");
        let geometry = Rc::new(geometry);

        // `new PointLight( 0xffffff, 900, 28, 2 )` — decay 2 is the default.
        let main_light = PointLight::new(Color::from_hex(0xffffff), 900.0, 28.0);
        main_light.borrow_mut().position.set(0.418, 16.199, 0.300);
        scene.add(&main_light);

        // `new MeshStandardMaterial( { side: BackSide } )` — the room box.
        // `MeshStandardMaterial`'s own defaults are roughness 1, metalness 0.
        let room_material = MeshBasicNodeMaterial {
            side: Side::Back,
            ..MeshBasicNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0)
        };
        let room = Mesh::new(geometry.clone(), room_material);
        {
            let mut object = room.borrow_mut();
            object.position.set(-0.757, 13.219, 0.717);
            object.scale.set(31.713, 28.305, 28.591);
        }
        scene.add(&room);

        // `new InstancedMesh( geometry, boxMaterial, 6 )`.
        let box_material = MeshBasicNodeMaterial::standard(Color::from_hex(0xffffff), 1.0, 0.0);
        let boxes = InstancedMesh::new(geometry.clone(), box_material, 6);
        {
            let mut object = boxes.borrow_mut();
            let Payload::InstancedMesh(instanced) = &mut object.payload else {
                unreachable!("three-rs: InstancedMesh::new returns an InstancedMesh payload")
            };
            for (i, (position, rotation_y, scale)) in BOXES.iter().enumerate() {
                instanced.set_matrix_at(i, &instance_matrix(*position, *rotation_y, *scale));
            }
        }
        scene.add(&boxes);

        for (intensity, position, scale) in PANELS {
            let panel = Mesh::new(geometry.clone(), area_light_material(intensity));
            {
                let mut object = panel.borrow_mut();
                object.position.set(position[0], position[1], position[2]);
                object.scale.set(scale[0], scale[1], scale[2]);
            }
            scene.add(&panel);
        }

        scene
    }
}
