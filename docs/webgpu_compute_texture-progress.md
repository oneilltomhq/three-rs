# `webgpu_compute_texture` — a kernel that writes a texture

**Result: 0 of 100000 pixels different, limit 0.1%.** Exact on the first
frame that reached the canvas.

A compute kernel runs once, in `init()`, over 512² invocations and
`textureStore`s the shadertoy plasma into a `StorageTexture`; a unit plane
under an `OrthographicCamera( -aspect, aspect, 1, -1, 0, 2 )` samples it with
`texture( storageTexture )`. The page's `render()` is one `renderer.render()`;
nothing moves.

## What was added (issue #166)

| Area | What | Why |
|---|---|---|
| `src/textures/texture.rs` | `Texture::storage( w, h )`, `is_storage()` | `new StorageTexture( w, h )`: no image, `LinearFilter`, `generateMipmaps` kept |
| `src/nodes/node.rs` | `TextureSource::Storage( texture, access )`, `StorageAccess`, `Node::TextureStore` | a storage binding is a different binding from the sampled one of the same texture |
| `src/nodes/tsl.rs` | `storage_texture()`, `texture_store()`, `.load()`, `.set_access()` | `storageTexture()`, `textureStore()`, `textureLoad` on a storage texture |
| `src/nodes/wgsl.rs` | `TextureKind::Storage`, `storage_format()` | `texture_storage_2d<rgba8unorm, write>`; read-only outside compute, as `getNodeAccess()` makes it |
| `src/nodes/builder.rs` | the `textureStore( t, vec2<u32>( … ), vec4 )` statement | `generateTextureStore()` |
| `src/renderer/programs.rs` | `BindingType::StorageTexture` layout entries | |
| `src/renderer/mod.rs` | storage textures created with `STORAGE_BINDING`, never uploaded, bound through a one-mip view; compute bind groups take textures and samplers | `WebGPUTextureUtils.createTexture()`, `WebGPUBindingUtils.createBindGroup()` |
| `src/renderer/mod.rs` | `needs_mipmap` on the 2D texture entry | `Bindings._update()`: a store marks the chain stale, the next sampled binding rebuilds it |

`docs/nodes.md` §28 has the detail.

## What the pixels found

The mip chain is the whole frame. The plane covers about 250² of the graded
400x250 pixels (500² at the render size) and samples a 512² texture, so
`textureSample` reads mips 0 and 1 with `minFilter = LinearFilter` (nearest
mip). Three regenerates the chain from level 0 before the first draw that
samples a storage texture a kernel wrote. Without that step the port reads a
zero-filled level 1: **15876 of 100000 pixels** different, the plane mostly
black. With it, 0.

## What the WGSL found

`tests/nodes_texture_wgsl.rs` diffs the kernel against three's dump as a
whole module, and the material's uniforms and colour flow. The kernel is
identical apart from §8's banner and subgroup lines. The intermediates are
`let nodeConstN` because three at 5f610f5 makes a multiply-read temp a
`let` on its own. The port's builder still makes such a temp a var, so the
example asks for `to_const` where the dump has one (§8, "Usage-promoted
temps").

## Steady frame

1.2 ms, 2 draws (the plane and the output quad), 3 triangles. The kernel is
not re-dispatched and the mips are not rebuilt after the first frame. The
ladder's steady-frame gate asserts that frames two and three create nothing.
