//! The keys of the per-draw binding caches (issue #137): which draw a
//! persistent buffer belongs to, and how long an unused entry survives.
//!
//! three.js keeps one GPU buffer per binding group for the object's lifetime
//! (`Bindings` / `UniformBuffer`) and `writeBuffer`s the changed bytes into it
//! each frame. The port re-created every one of them on every draw, which at a
//! hundred draws a frame was hundreds of wgpu allocations and the whole of
//! `webgpu_materials_basic`'s frame-time floor. Everything here is pure, so the
//! identity rules can be tested without a device.

use std::collections::HashMap;

use super::CACHE_GRACE_FRAMES;
use crate::nodes::node::TextureSource;
use crate::textures::{TextureFilter, Wrapping};

/// One draw's identity, for the buffers that hold its per-draw data — its
/// uniform groups, its bone matrices and morph influences, its instance
/// matrix and colours.
///
/// Every field is an id from a never-reused counter (`Object3D.id`,
/// `BufferGeometry.id`, `Material.id`) or a constant, never an address, for
/// the reason `docs/scene-graph.md` gives under *Identity and eviction*.
///
/// `occurrence` is what makes the key unique within one pass. A pass resolves
/// all its draws before it submits them, and a `queue.write_buffer` lands at
/// the next submit, so two draws of one pass that shared a buffer would both
/// read whichever wrote last. The same object, geometry and material can be
/// drawn twice in one pass (a material array naming one material in two
/// groups, the renderer's own quad drawn with one material twice), so the
/// n-th such draw gets the n-th buffer. Across passes sharing is safe — each
/// pass is its own submit, and the queue applies the next pass's writes after
/// the previous pass has read its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct DrawKey {
    /// `Object3D.id`; `None` for the renderer's own draws (quad, background,
    /// output pass), which carry no application object.
    pub object: Option<u32>,
    pub geometry: usize,
    /// `MaterialKey::id` and `::variant` — the *source* material's id, which is
    /// stable across frames where the per-frame clone's is not.
    pub material: usize,
    pub variant: u64,
    pub occurrence: u32,
}

/// Hands out [`DrawKey::occurrence`] for one pass.
#[derive(Default)]
pub(super) struct Occurrences(HashMap<DrawKey, u32>);

impl Occurrences {
    /// `key` with its `occurrence` set to how many times this pass has already
    /// seen the same draw.
    pub fn next(&mut self, key: DrawKey) -> DrawKey {
        let base = DrawKey {
            occurrence: 0,
            ..key
        };
        let seen = self.0.entry(base).or_insert(0);
        let occurrence = *seen;
        *seen += 1;
        DrawKey { occurrence, ..base }
    }
}

/// Who a per-draw buffer belongs to: a draw, or a compute kernel (keyed by
/// its `ComputeProgram::cache_key`; a kernel's uniforms are the same on every
/// dispatch, and each dispatch is its own submit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SlotOwner {
    Draw(DrawKey),
    Compute(u64),
}

/// The group a vertex buffer's [`SlotKey`] is filed under — no bind group
/// has this index, so an instance matrix bound as a vertex buffer and the
/// same matrix bound as a uniform are two buffers, as their usages require.
pub(super) const VERTEX_SLOTS: u32 = u32::MAX;

/// One persistent per-draw buffer: the owner, the bind group and the binding
/// within it (or [`VERTEX_SLOTS`] and the vertex buffer slot).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SlotKey {
    pub owner: SlotOwner,
    pub group: u32,
    pub binding: u32,
}

/// Everything a `wgpu::Sampler` is made of, which is all it is: a sampler has
/// no contents, so two textures whose filters and wrapping agree can share
/// one, and the renderer memoises them in one map for its whole life
/// (`WebGPUTextureUtils.updateSampler()` builds one per texture; the port
/// builds one per distinct descriptor). The map needs no eviction — its size
/// is bounded by the handful of filter, wrap and compare combinations a
/// program uses, not by how many textures it creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SamplerKey {
    pub address: [wgpu::AddressMode; 3],
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::MipmapFilterMode,
    pub anisotropy_clamp: u16,
    pub compare: Option<wgpu::CompareFunction>,
}

impl SamplerKey {
    /// `WebGPUTextureUtils.updateSampler()`'s descriptor for `source`.
    ///
    /// # Panics
    ///
    /// For a texture that is read with `textureLoad` and has no sampler.
    pub fn of(source: &TextureSource) -> Self {
        let filter = |f: TextureFilter| match f {
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
            TextureFilter::Linear => wgpu::FilterMode::Linear,
        };
        let mipmap = |f: TextureFilter| match f {
            TextureFilter::Nearest => wgpu::MipmapFilterMode::Nearest,
            TextureFilter::Linear => wgpu::MipmapFilterMode::Linear,
        };
        let address = |w: Wrapping| match w {
            Wrapping::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            Wrapping::Repeat => wgpu::AddressMode::Repeat,
            Wrapping::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        };
        let clamp = wgpu::AddressMode::ClampToEdge;

        match source {
            TextureSource::Texture2D(texture) => {
                let inner = texture.borrow();
                Self {
                    address: [address(inner.wrap_s), address(inner.wrap_t), clamp],
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter.min()),
                    mipmap_filter: mipmap(inner.min_filter.mipmap()),
                    anisotropy_clamp: inner.anisotropy,
                    compare: None,
                }
            }
            TextureSource::Cube(cube) => {
                let inner = cube.inner().borrow();
                Self {
                    // `CubeTexture`'s wrapping is `ClampToEdgeWrapping` on all axes.
                    address: [clamp; 3],
                    // `LinearFilter` / `LinearMipmapLinearFilter` by default;
                    // `HDRCubeTextureLoader` sets `minFilter = LinearFilter`,
                    // which drops the mip filter to `nearest` — there is only
                    // one mip in that cube for it to choose between anyway.
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter.min()),
                    mipmap_filter: mipmap(inner.min_filter.mipmap()),
                    anisotropy_clamp: inner.anisotropy,
                    compare: None,
                }
            }
            // `ShadowNode.setupShadow()`: `LinearFilter` on both when the
            // shadow type is `PCFShadowMap`, and `compareFunction =
            // LessEqualCompare`, which `WebGPUTextureUtils.updateSampler()`
            // turns into a comparison sampler.
            TextureSource::ShadowMap(depth) => {
                let inner = depth.inner().borrow();
                Self {
                    address: [clamp; 3],
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter),
                    mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                    anisotropy_clamp: 1,
                    compare: Some(wgpu::CompareFunction::LessEqual),
                }
            }
            // `compareFunction = LessEqualCompare` makes this a comparison
            // sampler; `CubeDepthTexture`'s filters are `LinearFilter`.
            TextureSource::CubeDepth(_) => Self {
                address: [clamp; 3],
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                anisotropy_clamp: 1,
                compare: Some(wgpu::CompareFunction::LessEqual),
            },
            TextureSource::Depth(_) | TextureSource::DataArray(_) | TextureSource::Data(_) => {
                panic!("three-rs: this texture is read with textureLoad, not sampled")
            }
        }
    }

    pub fn descriptor(&self) -> wgpu::SamplerDescriptor<'static> {
        let [address_mode_u, address_mode_v, address_mode_w] = self.address;
        wgpu::SamplerDescriptor {
            label: Some("three-rs sampler"),
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            anisotropy_clamp: self.anisotropy_clamp,
            compare: self.compare,
            ..Default::default()
        }
    }
}

/// Whether an entry last used at `last_used` survives a sweep at `frames`:
/// the same [`CACHE_GRACE_FRAMES`] window `node_builder_states` and `buffers`
/// age out by.
pub(super) fn is_fresh(last_used: u64, frames: u64) -> bool {
    last_used >= frames.saturating_sub(CACHE_GRACE_FRAMES)
}

/// `bytes` padded with zeros to the 4-byte multiple `write_buffer` requires.
pub(super) fn padded(bytes: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    if bytes.len().is_multiple_of(4) {
        std::borrow::Cow::Borrowed(bytes)
    } else {
        let mut padded = bytes.to_vec();
        padded.resize(bytes.len().next_multiple_of(4), 0);
        std::borrow::Cow::Owned(padded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(object: u32, material: usize) -> DrawKey {
        DrawKey {
            object: Some(object),
            geometry: 7,
            material,
            variant: 0,
            occurrence: 0,
        }
    }

    #[test]
    fn a_repeated_draw_in_one_pass_gets_its_own_slot() {
        let mut pass = Occurrences::default();
        let first = pass.next(draw(1, 2));
        let other = pass.next(draw(3, 2));
        let second = pass.next(draw(1, 2));
        assert_eq!((first.occurrence, other.occurrence), (0, 0));
        assert_eq!(second.occurrence, 1);
        assert_ne!(first, second);

        // The next pass starts counting again, so a steady frame keys every
        // draw the same way it did last frame.
        let mut next = Occurrences::default();
        assert_eq!(next.next(draw(1, 2)), first);
        assert_eq!(next.next(draw(1, 2)), second);
    }

    #[test]
    fn an_entry_survives_the_grace_window_and_no_longer() {
        assert!(is_fresh(10, 10));
        assert!(is_fresh(10, 10 + CACHE_GRACE_FRAMES));
        assert!(!is_fresh(10, 11 + CACHE_GRACE_FRAMES));
        // The first frames cannot underflow into evicting everything.
        assert!(is_fresh(0, 1));
    }

    #[test]
    fn textures_that_filter_alike_share_a_sampler_key() {
        use crate::textures::{DepthTexture, MinFilter, Texture};

        let a = Texture::new(1, 1, None);
        let b = Texture::new(4, 4, None);
        let key = |texture: &Texture| SamplerKey::of(&TextureSource::Texture2D(texture.clone()));
        // Two textures, one descriptor: the size and the pixels are not in it.
        assert_eq!(key(&a), key(&b));

        // Every field that is in it separates them.
        b.set_wrapping(Wrapping::Repeat, Wrapping::ClampToEdge);
        assert_ne!(key(&a), key(&b));
        assert_eq!(key(&b).address[0], wgpu::AddressMode::Repeat);
        b.set_wrapping(Wrapping::ClampToEdge, Wrapping::ClampToEdge);
        assert_eq!(key(&a), key(&b));

        b.set_min_filter(MinFilter::Nearest);
        assert_ne!(key(&a), key(&b));
        b.set_min_filter(a.borrow().min_filter);
        b.set_anisotropy(a.borrow().anisotropy + 1);
        assert_ne!(key(&a), key(&b));

        // A shadow map's sampler compares, so it never shares with a colour
        // texture's even when the filters agree.
        let shadow = SamplerKey::of(&TextureSource::ShadowMap(DepthTexture::new()));
        assert_eq!(shadow.compare, Some(wgpu::CompareFunction::LessEqual));
        assert_ne!(shadow, key(&a));

        // And the descriptor is the key, field for field.
        let descriptor = key(&a).descriptor();
        assert_eq!(descriptor.mag_filter, key(&a).mag_filter);
        assert_eq!(descriptor.mipmap_filter, key(&a).mipmap_filter);
        assert_eq!(descriptor.compare, None);
    }

    #[test]
    fn padding_is_to_four_bytes_and_zero_filled() {
        assert_eq!(&*padded(&[1, 2, 3, 4]), &[1, 2, 3, 4]);
        assert_eq!(&*padded(&[1, 2, 3, 4, 5]), &[1, 2, 3, 4, 5, 0, 0, 0]);
        assert!(padded(&[]).is_empty());
    }
}
