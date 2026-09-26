//! The keys of the per-draw binding caches (issue #137): which draw a
//! persistent buffer belongs to, and how long an unused entry survives.
//!
//! three.js keeps one GPU buffer per binding group for the object's lifetime
//! (`Bindings` / `UniformBuffer`) and `writeBuffer`s the changed bytes into it
//! each frame. The port re-created every one of them on every draw, which at a
//! hundred draws a frame was hundreds of wgpu allocations and the whole of
//! `webgpu_materials_basic`'s frame-time floor. Everything here is pure, so the
//! identity rules can be tested without a device.

use std::collections::{HashMap, HashSet};

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
            // sampler; the filters are the shadow type's (`LinearFilter` for
            // PCF, `NearestFilter` otherwise).
            TextureSource::CubeDepth(cube) => {
                let inner = cube.inner().borrow();
                Self {
                    address: [clamp; 3],
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter),
                    mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                    anisotropy_clamp: 1,
                    compare: Some(wgpu::CompareFunction::LessEqual),
                }
            }
            // `Data3DTexture`: wrapS/T/R and the two filters, no anisotropy
            // (`WebGPUTextureUtils.updateSampler()` only clamps it for a
            // filtering 2-D sampler; the volume pages leave it at 1).
            TextureSource::Texture3D(texture) => {
                let inner = texture.borrow();
                Self {
                    address: [
                        address(inner.wrap_s),
                        address(inner.wrap_t),
                        address(inner.wrap_r),
                    ],
                    mag_filter: filter(inner.mag_filter),
                    min_filter: filter(inner.min_filter.min()),
                    mipmap_filter: mipmap(inner.min_filter.mipmap()),
                    anisotropy_clamp: 1,
                    compare: None,
                }
            }
            TextureSource::Depth(_)
            | TextureSource::DataArray(_)
            | TextureSource::Data(_)
            | TextureSource::Storage(..)
            | TextureSource::Storage3D(..) => {
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

/// A GPU resource numbered for the bind group cache.
///
/// The serial is the resource's identity: handed out from one never-reused
/// counter ([`Serials`]) when the resource is created, and carried with it
/// through every cache that holds it. wgpu 30 has no public `global_id()`,
/// and its `Eq`/`Hash` on resources defer to whatever handle the backend
/// uses — a registry slot whose index is recycled on native, a JS object on
/// the web — so a key built on them would mean something different per
/// backend and, on native, be reusable once the resource is freed. A counter
/// of the renderer's own is the same on every backend and never repeats.
#[derive(Clone, Debug)]
pub(super) struct Serial<T> {
    pub serial: u64,
    pub gpu: T,
}

/// The counter behind [`Serial`]. Starts at 1 so no resource is ever 0.
#[derive(Default)]
pub(super) struct Serials(u64);

impl Serials {
    pub fn next(&mut self) -> u64 {
        self.0 += 1;
        self.0
    }
}

/// One binding's resource, as a bind group entry needs it.
pub(super) enum Resource {
    Buffer(Serial<wgpu::Buffer>),
    View(Serial<wgpu::TextureView>),
    Sampler(Serial<wgpu::Sampler>),
}

impl Resource {
    pub fn serial(&self) -> u64 {
        match self {
            Resource::Buffer(buffer) => buffer.serial,
            Resource::View(view) => view.serial,
            Resource::Sampler(sampler) => sampler.serial,
        }
    }

    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Resource::Buffer(buffer) => buffer.gpu.as_entire_binding(),
            Resource::View(view) => wgpu::BindingResource::TextureView(&view.gpu),
            Resource::Sampler(sampler) => wgpu::BindingResource::Sampler(&sampler.gpu),
        }
    }
}

/// Whose pipeline layout a bind group is made against: a render program's
/// (`Program`, by the program cache key) or a compute kernel's. Neither cache
/// is ever evicted or overwritten, so a key names one layout for the
/// renderer's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum LayoutKey {
    Render(u64),
    Compute(u64),
}

/// A bind group's identity: the layout it was made against, the group index,
/// and the [`Serial`] of each resource in binding order.
///
/// A resource's *contents* are not in it — a uniform buffer rewritten this
/// frame is the same serial, so its group is reused. Replacing a resource is
/// a new serial, and a new group.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct BindGroupKey {
    layout: LayoutKey,
    group: u32,
    resources: Vec<u64>,
}

impl BindGroupKey {
    pub fn of(layout: LayoutKey, group: u32, resources: &[Resource]) -> Self {
        Self::from_serials(layout, group, resources.iter().map(Resource::serial))
    }

    fn from_serials(layout: LayoutKey, group: u32, serials: impl Iterator<Item = u64>) -> Self {
        Self {
            layout,
            group,
            resources: serials.collect(),
        }
    }

    /// Whether any of this group's resources is one of `serials`.
    pub fn binds_any(&self, serials: &HashSet<u64>) -> bool {
        self.resources.iter().any(|serial| serials.contains(serial))
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
    fn a_bind_group_key_knows_the_serials_it_binds() {
        let key = BindGroupKey::from_serials(LayoutKey::Render(1), 0, [3, 5, 8].into_iter());
        assert!(key.binds_any(&HashSet::from([5])));
        assert!(key.binds_any(&HashSet::from([1, 8])));
        assert!(!key.binds_any(&HashSet::from([1, 2, 4])));
        assert!(!key.binds_any(&HashSet::new()));
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
    fn a_bind_group_is_keyed_on_what_it_binds_not_on_its_contents() {
        let mut serials = Serials::default();
        let (uniforms, view, sampler) = (serials.next(), serials.next(), serials.next());
        assert!(uniforms != view && view != sampler && uniforms != 0);

        let key = |layout, group, members: &[u64]| {
            BindGroupKey::from_serials(layout, group, members.iter().copied())
        };
        let first = key(LayoutKey::Render(9), 1, &[uniforms, view, sampler]);

        // Next frame the uniform buffer's bytes were rewritten, but it is the
        // same buffer: the same key.
        assert_eq!(
            key(LayoutKey::Render(9), 1, &[uniforms, view, sampler]),
            first
        );

        // A replaced member is a new serial, and a new group.
        let resized = serials.next();
        assert_ne!(
            key(LayoutKey::Render(9), 1, &[uniforms, resized, sampler]),
            first
        );

        // The same resources against another layout, another group index or
        // in another order are other groups too.
        assert_ne!(
            key(LayoutKey::Compute(9), 1, &[uniforms, view, sampler]),
            first
        );
        assert_ne!(
            key(LayoutKey::Render(9), 0, &[uniforms, view, sampler]),
            first
        );
        assert_ne!(
            key(LayoutKey::Render(9), 1, &[view, uniforms, sampler]),
            first
        );
    }

    #[test]
    fn padding_is_to_four_bytes_and_zero_filled() {
        assert_eq!(&*padded(&[1, 2, 3, 4]), &[1, 2, 3, 4]);
        assert_eq!(&*padded(&[1, 2, 3, 4, 5]), &[1, 2, 3, 4, 5, 0, 0, 0]);
        assert!(padded(&[]).is_empty());
    }
}
