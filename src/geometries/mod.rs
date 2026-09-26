//! Ports of `three.js/src/geometries`.

mod box_geometry;
mod capsule;
mod circle;
mod cone;
mod cylinder;
mod extrude;
mod lathe;
mod plane;
mod polyhedron;
mod quad;
mod ring;
mod rounded_box;
mod shape;
mod sphere;
mod teapot;
mod torus;
mod torus_knot;
mod tube;

pub use box_geometry::{box_geometry, box_geometry_default};
pub use capsule::capsule_geometry;
pub use circle::{circle_geometry, circle_geometry_full};
pub use cone::{cone_geometry, cone_geometry_full};
pub use cylinder::{cylinder_geometry, cylinder_geometry_full};
pub use extrude::{
    extrude_geometry, extrude_geometry_default_shape, ExtrudeGeometryOptions, UvGenerator,
    WorldUvGenerator,
};
pub use lathe::{lathe_default_points, lathe_geometry, lathe_geometry_full};
pub use plane::plane_geometry;
pub use polyhedron::{
    dodecahedron_geometry, icosahedron_geometry, octahedron_geometry, polyhedron_geometry,
    tetrahedron_geometry,
};
pub use quad::quad_geometry;
pub use ring::{ring_geometry, ring_geometry_full};
pub use rounded_box::rounded_box_geometry;
pub use shape::{shape_geometry, shape_geometry_default_shape, shape_geometry_multi};
pub use sphere::{sphere_geometry, sphere_geometry_full};
pub use teapot::{teapot_geometry, teapot_geometry_full};
pub use torus::{torus_geometry, torus_geometry_full};
pub use torus_knot::torus_knot_geometry;
pub use tube::{tube_geometry, tube_geometry_default_path};
