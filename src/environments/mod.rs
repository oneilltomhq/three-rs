//! Port of `three.js/examples/jsm/environments/` — scenes built to be fed to
//! [`PMREMGenerator::from_scene`], not to be rendered to the screen.
//!
//! [`PMREMGenerator::from_scene`]: crate::renderer::pmrem::PMREMGenerator::from_scene

mod room_environment;

pub use room_environment::RoomEnvironment;
