//! Port of `three.js/examples/jsm/environments/` — scenes built to be fed to
//! [`PmremGenerator::from_scene`], not to be rendered to the screen.
//!
//! [`PmremGenerator::from_scene`]: crate::renderer::pmrem::PmremGenerator::from_scene

mod room_environment;

pub use room_environment::RoomEnvironment;
