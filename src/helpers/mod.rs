//! Ports of `three.js/src/helpers/`: scene-graph objects that build their own
//! geometry to visualise something else (axes, a box, a plane, a light, a
//! skeleton).
//!
//! # Shape
//!
//! A helper without state of its own past construction ([`GridHelper`],
//! [`PolarGridHelper`]) is a unit struct whose `new` returns the [`ObjectRef`]
//! three would return. A helper with methods is a struct holding the node as
//! `pub node` beside three's public fields (`light`, `box3`, `plane`, `cone`,
//! …), whose methods keep three's names in snake case (`update`,
//! `set_colors`, `set_direction`, …). Optional constructor arguments are
//! plain values: pass three's default, which each `new` names, or take
//! `Default` where three's constructor needs no arguments at all
//! ([`AxesHelper`], [`ArrowHelper`], [`BoxHelper`]). An argument is an
//! [`Option`] only where three's `undefined` means something no value can:
//! [`ArrowHelper`]'s head length and width (defaults derived from
//! `length`), [`BoxHelper`]'s object (none yet), and the light helpers'
//! `color`, where `None` follows the light's own colour on every `update()`.
//!
//! Each helper keeps three's geometry (attribute names, vertex order, index),
//! its material class (`LineBasicMaterial` is
//! [`MeshBasicNodeMaterial::line`], `MeshBasicMaterial`
//! [`MeshBasicNodeMaterial::new`]), side, colour and flags (`vertexColors`,
//! `depthTest`, `depthWrite`, `transparent`, `fog`, `wireframe`), its `type`
//! string, and its child transforms; the
//! `helpers_core` gate compares all of these node for node against the
//! pinned checkout.
//!
//! # Divergences
//!
//! - **`updateMatrixWorld` overrides.** [`Box3Helper`], [`PlaneHelper`] and
//!   [`SkeletonHelper`] recompute their transform or vertices in an
//!   `updateMatrixWorld()` override. The port's traversal has no per-type
//!   hook, so each exposes `update_matrix_world(force)` instead: the
//!   override's body, then [`ObjectRef::update_matrix_world`]. Nothing calls it
//!   for you, and [`Renderer::render`]'s own `scene.update_matrix_world()`
//!   comes too late for a helper that reads another object's world matrix
//!   ([`PlaneHelper`] its parent's, [`SkeletonHelper`] the bones'). So each
//!   frame: update the graph first (`root.update_matrix_world(false)`, or
//!   [`Scene::update_matrix_world`]), then the helper's
//!   `update_matrix_world`, then render. [`Box3Helper`] reads only its box,
//!   so it needs only to come before the render, whose update recomputes its
//!   world matrix from the position and scale it set.
//! - **`toneMapped: false`** has no counterpart: the port's materials carry no
//!   tone-mapping flag.
//! - **`dispose()`** is not ported: geometry and materials are freed on drop,
//!   and nothing in the port has a `dispose` to call.
//! - **`copy()`** on `ArrowHelper` and `BoxHelper` is not ported, as the port
//!   has no `Object3D.copy` to extend.
//! - A material three shares between two children (`DirectionalLightHelper`)
//!   is a clone per child here; `update()` writes both.
//!
//! [`ObjectRef`]: crate::core::ObjectRef
//! [`MeshBasicNodeMaterial::line`]: crate::materials::MeshBasicNodeMaterial::line
//! [`MeshBasicNodeMaterial::new`]: crate::materials::MeshBasicNodeMaterial::new
//! [`ObjectRef::update_matrix_world`]: crate::core::ObjectRef::update_matrix_world
//! [`Renderer::render`]: crate::Renderer::render
//! [`Scene::update_matrix_world`]: crate::objects::Scene::update_matrix_world

mod arrow_helper;
mod axes_helper;
mod box3_helper;
mod box_helper;
mod camera_helper;
mod directional_light_helper;
mod grid_helper;
mod hemisphere_light_helper;
mod plane_helper;
mod point_light_helper;
mod polar_grid_helper;
mod skeleton_helper;
mod spot_light_helper;

pub use arrow_helper::ArrowHelper;
pub use axes_helper::AxesHelper;
pub use box3_helper::Box3Helper;
pub use box_helper::BoxHelper;
pub use camera_helper::CameraHelper;
pub use directional_light_helper::DirectionalLightHelper;
pub use grid_helper::GridHelper;
pub use hemisphere_light_helper::HemisphereLightHelper;
pub use plane_helper::PlaneHelper;
pub use point_light_helper::PointLightHelper;
pub use polar_grid_helper::PolarGridHelper;
pub use skeleton_helper::SkeletonHelper;
pub use spot_light_helper::SpotLightHelper;
