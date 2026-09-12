//! Ports of `three.js/src/textures`.

mod cube_texture;
mod depth_texture;

pub use cube_texture::{ColorSpace, CubeTexture, CubeTextureInner, Image, Mapping};
pub use depth_texture::{DepthTexture, DepthTextureInner, TextureFilter, TextureType};
