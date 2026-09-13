//! Ports of `three.js/src/textures`.

use std::cell::Cell;

mod cube_depth_texture;
mod cube_texture;
mod data_array_texture;
mod depth_texture;
mod texture;

pub use cube_depth_texture::{CubeDepthTexture, CubeDepthTextureInner};
pub use cube_texture::{ColorSpace, CubeTexture, CubeTextureInner, Image, Mapping};
pub use data_array_texture::{DataArrayTexture, DataArrayTextureInner};
pub use depth_texture::{DepthTexture, DepthTextureInner, TextureFilter, TextureType};
pub use texture::{MinFilter, Texture, TextureInner, Wrapping};

/// `Texture.id` — three.js' `_textureId ++`, shared by every texture class the
/// port has, the way `Source`/`Texture` ids are one counter upstream.
///
/// A texture is a *handle* (`Rc<RefCell<Inner>>`), so a clone is the same
/// texture and keeps the same id; only a `new` mints one. The id used to be
/// `Rc::as_ptr( &inner )`, and the renderer keeps its uploaded `wgpu::Texture`s
/// under it — so a texture dropped and another allocated at its address was
/// served the dead one's pixels, the same freed-address bug as issue #58's
/// geometry cache. A never-reused counter cannot do that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(usize);

impl TextureId {
    pub fn next() -> Self {
        thread_local! {
            static TEXTURE_ID: Cell<usize> = const { Cell::new(0) };
        }
        TEXTURE_ID.with(|id| {
            let next = id.get();
            id.set(next + 1);
            TextureId(next)
        })
    }

    /// The number itself, for keying on.
    pub fn get(&self) -> usize {
        self.0
    }
}
