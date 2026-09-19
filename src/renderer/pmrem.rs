//! Port of `three.js/src/renderers/common/extras/PMREMGenerator.js`.
//!
//! A PMREM is a *P*refiltered, *M*ipmapped *R*adiance *E*nvironment *M*ap: one
//! `rgba16float` 2-D texture holding the environment convolved with the GGX
//! lobe at a ladder of roughnesses, so a material can reach any roughness with
//! a single tap. It is not a cube mip chain — the levels are **tiles** of a
//! single atlas, three faces across and two down per level, with the levels
//! past `LOD_MIN` laid out sideways instead of shrinking further.
//! [`crate::nodes::pmrem_utils`] does the addressing; this module does the
//! rendering.
//!
//! For a 256² source cube that atlas is 768×1024 — the `PMREM.cubeUv` texture
//! in three's own dump of `webgpu_pmrem_cubemap`. One pass converts the cube
//! into the top 768×512 of it, then ten GGX steps of two passes each prefilter
//! the pyramid, which is the dump's 21 PMREM render passes.
//!
//! Only `fromCubemap` is ported. `fromScene` (and with it `_blur` /
//! `sphericalGaussianBlur`) and `fromEquirectangular` have no caller on this
//! rung.

use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry};
use crate::error::Error;
use crate::materials::{Blending, MeshBasicNodeMaterial};
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::pmrem_utils::{self, CubeUvSize};
use crate::nodes::tsl::{
    attribute, cube_texture, float, material_env_rotation, uniform_settable, vec4_join,
};
use crate::nodes::NodeRef;
use crate::textures::{CubeTexture, Texture, TextureFilter, TextureType};

use super::render_target::{RenderTarget, RenderTargetOptions};

/// `LOD_MIN` — the smallest face the pyramid shrinks to. Below it the extra
/// levels keep that size and step sideways into their own columns.
pub const LOD_MIN: usize = 4;

/// `EXTRA_LODS` — the number of extra, more heavily filtered levels at
/// `LOD_MIN` resolution.
pub const EXTRA_LODS: usize = 6;

/// `GGX_SAMPLES` — VNDF samples per prefilter pass.
pub const GGX_SAMPLES: usize = 256;

/// `_faceLib` — the WebGPU face order, which is the order the six quads of a
/// lod plane are written into the vertex arrays.
const FACE_LIB: [usize; 6] = [3, 1, 5, 0, 4, 2];

/// The GGX material's `_uniformsMap` entry: the cells `_applyGGXFilter`
/// rewrites between passes.
pub struct GgxUniforms {
    /// `ggxUniforms.envMap.value` — the texture the pass reads.
    ///
    /// Three swaps a `TextureNode`'s `.value`; a `TextureSource` in this port
    /// holds its `Texture` by value, and the texture's identity is what the
    /// binding resolves through. So the node keeps **one** handle, of the
    /// borrowed kind a render target's colour attachment uses
    /// (`own_gpu = false`), and the generator points its `gpu` at whichever of
    /// the two targets the pass should read. Bind groups are built per draw,
    /// so the swap lands on the next draw and no cache has to be invalidated.
    pub env_map: Texture,
    /// `ggxUniforms.roughness` — the *incremental* roughness of this step, or
    /// zero for the copy-back half of it.
    pub roughness: SettableValue,
    /// `ggxUniforms.mipInt` — the atlas level being read.
    pub mip_int: SettableValue,
}

/// One entry of `_createPlanes()`: the six-quad geometry covering a level's
/// tiles, and that level's face size (`_sizeLods`).
pub struct LodMesh {
    pub geometry: Rc<BufferGeometry>,
    pub size: usize,
}

/// `new PMREMGenerator( renderer )`.
///
/// The renderer is not held: [`from_cubemap`](Self::from_cubemap) takes it,
/// because a `Renderer` in this port is a `&mut` owner rather than a JS object
/// reference.
pub struct PmremGenerator {
    /// `this._lodMax` — `floor( log2( cubeSize ) )`.
    lod_max: usize,
    /// `this._cubeSize` — `2 ^ lodMax`.
    cube_size: usize,
    /// `this._lodMeshes` and `this._sizeLods`, which are indexed together.
    lod_meshes: Vec<LodMesh>,
    /// `this._pingPongRenderTarget`.
    ping_pong: Option<RenderTarget>,
    /// `this._ggxMaterial` plus its `_uniformsMap` entry.
    ggx: Option<(MeshBasicNodeMaterial, GgxUniforms)>,
    /// `this._cubemapMaterial`.
    cubemap_material: Option<MeshBasicNodeMaterial>,
}

impl Default for PmremGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PmremGenerator {
    pub fn new() -> Self {
        Self {
            lod_max: 0,
            cube_size: 0,
            lod_meshes: Vec::new(),
            ping_pong: None,
            ggx: None,
            cubemap_material: None,
        }
    }

    /// `_setSize( cubeSize )`.
    fn set_size(&mut self, cube_size: usize) {
        self.lod_max = (cube_size as f64).log2().floor() as usize;
        self.cube_size = 1usize << self.lod_max;
    }

    /// `_allocateTarget( depthBuffer )` — the cubeUV atlas.
    ///
    /// `3 * max( cubeSize, 16 * 7 )` wide by `4 * cubeSize` tall. The `16 * 7`
    /// floor is what reserves the extra-LOD columns when the source cube is
    /// small; a 256² cube is past it, so the atlas is 768×1024.
    fn allocate_target(&self, depth_buffer: bool) -> Result<RenderTarget, Error> {
        let width = 3 * self.cube_size.max(16 * 7);
        let height = 4 * self.cube_size;
        create_render_target(width as u32, height as u32, depth_buffer)
    }

    /// `_init( renderTarget )` — the ping-pong target, the lod planes and the
    /// GGX material, rebuilt whenever the atlas changes shape.
    fn init(&mut self, target: &RenderTarget) -> Result<(), Error> {
        let (width, height) = target.size();
        let fresh = match &self.ping_pong {
            None => true,
            Some(ping_pong) => ping_pong.size() != (width, height),
        };
        if !fresh {
            return Ok(());
        }

        self.ping_pong = Some(create_render_target(width, height, false)?);
        self.lod_meshes = create_planes(self.lod_max);
        self.ggx = Some(ggx_material(self.lod_max, width as f64, height as f64));
        Ok(())
    }

    /// `fromCubemap( cubemap )` — the PMREM of a cube texture. The returned
    /// target's `texture()` is what a `pmrem_texture()` node samples.
    pub fn from_cubemap(
        &mut self,
        renderer: &mut super::Renderer,
        cubemap: &CubeTexture,
        render_target: Option<RenderTarget>,
    ) -> Result<RenderTarget, Error> {
        // `_setSizeFromTexture()`: a cube texture is sized from face 0, and an
        // empty cube falls back to 16.
        let (face_width, _) = cubemap.size();
        self.set_size(if face_width == 0 {
            16
        } else {
            face_width as usize
        });

        let old_target = renderer.render_target();

        let target = match render_target {
            Some(target) => target,
            None => self.allocate_target(false)?,
        };
        self.init(&target)?;
        self.texture_to_cube_uv(renderer, cubemap, &target);
        self.apply_pmrem(renderer, &target);

        // `_cleanup( outputTarget )`.
        renderer.set_render_target(old_target);
        target.set_scissor_test(false);
        let (width, height) = target.size();
        set_viewport(&target, 0, 0, width as usize, height as usize);

        Ok(target)
    }

    /// `_textureToCubeUV( texture, cubeUVRenderTarget )` — the one blit that
    /// fills level 0, the top `3 * cubeSize` × `2 * cubeSize` of the atlas.
    fn texture_to_cube_uv(
        &mut self,
        renderer: &mut super::Renderer,
        cubemap: &CubeTexture,
        target: &RenderTarget,
    ) {
        // Three keeps one material and assigns `fragmentNode.value = texture`;
        // the port bakes the cube into the node, so the material is built for
        // the cube it is first asked about. `PMREMNode` caches one generated
        // PMREM per source texture, so a generator never sees two.
        if self.cubemap_material.is_none() {
            self.cubemap_material = Some(cubemap_material(cubemap));
        }
        let material = self
            .cubemap_material
            .clone()
            .expect("three-rs: the cubemap material was just built");

        let size = self.cube_size;
        set_viewport(target, 0, 0, 3 * size, 2 * size);
        renderer.set_render_target(Some(target.clone()));
        let geometry = self.lod_meshes[0].geometry.clone();
        // `autoClear` is still whatever the app set, so this first pass clears
        // the atlas the way three's does.
        renderer.render_pmrem_mesh(geometry, &material, true);
    }

    /// `_applyPMREM( cubeUVRenderTarget )` — one GGX step per level, with
    /// `autoClear` off so every pass loads the atlas it writes a tile of.
    fn apply_pmrem(&mut self, renderer: &mut super::Renderer, target: &RenderTarget) {
        for i in 1..self.lod_meshes.len() {
            self.apply_ggx_filter(renderer, target, i - 1, i);
        }
    }

    /// `_applyGGXFilter( cubeUVRenderTarget, lodIn, lodOut )`.
    ///
    /// Two passes: the filter reads level `lodIn` of the atlas and writes level
    /// `lodOut`'s tile of the ping-pong target, then the same material with
    /// `roughness = 0` — a straight copy — writes it back into the atlas. The
    /// copy exists because a pass cannot sample the texture it renders into.
    fn apply_ggx_filter(
        &mut self,
        renderer: &mut super::Renderer,
        target: &RenderTarget,
        lod_in: usize,
        lod_out: usize,
    ) {
        let (roughness, mip_int) = ggx_step(self.lod_max, self.lod_meshes.len(), lod_in, lod_out);
        let (x, y, width, height) = self.tile(lod_out);

        let ping_pong = self
            .ping_pong
            .clone()
            .expect("three-rs: _init() allocated the ping-pong target");
        let geometry = self.lod_meshes[lod_out].geometry.clone();
        let (material, uniforms) = self
            .ggx
            .as_ref()
            .expect("three-rs: _init() built the GGX material");
        let material = material.clone();

        // Read the atlas at `lodIn` with the incremental roughness; write the
        // ping-pong target.
        uniforms.env_map.set_gpu(gpu_texture_of(target));
        uniforms.roughness.set(vec![roughness]);
        uniforms.mip_int.set(vec![mip_int]);
        set_viewport(&ping_pong, x, y, width, height);
        renderer.set_render_target(Some(ping_pong.clone()));
        renderer.render_pmrem_mesh(geometry.clone(), &material, false);

        // Copy the tile back.
        let (_, uniforms) = self
            .ggx
            .as_ref()
            .expect("three-rs: _init() built the GGX material");
        uniforms.env_map.set_gpu(gpu_texture_of(&ping_pong));
        uniforms.roughness.set(vec![0.0]);
        uniforms
            .mip_int
            .set(vec![self.lod_max as f64 - lod_out as f64]);
        set_viewport(target, x, y, width, height);
        renderer.set_render_target(Some(target.clone()));
        renderer.render_pmrem_mesh(geometry, &material, false);
    }

    /// The atlas rectangle level `lod_out` occupies.
    fn tile(&self, lod_out: usize) -> (usize, usize, usize, usize) {
        let (x, y, size) = tile_rect(
            self.lod_max,
            self.cube_size,
            self.lod_meshes[lod_out].size,
            lod_out,
        );
        (x, y, 3 * size, 2 * size)
    }

    /// `this._sizeLods` — exposed so a test can walk the ladder.
    pub fn lod_meshes(&self) -> &[LodMesh] {
        &self.lod_meshes
    }

    pub fn lod_max(&self) -> usize {
        self.lod_max
    }

    pub fn cube_size(&self) -> usize {
        self.cube_size
    }
}

/// `_applyGGXFilter`'s arithmetic: the incremental roughness of a step and the
/// atlas level it reads.
///
/// Split out because it is pure and is the numeric gate on the blur ladder —
/// `tests/pmrem.rs` checks the whole table against values taken from three's
/// own `_applyGGXFilter`.
pub fn ggx_step(lod_max: usize, lod_count: usize, lod_in: usize, lod_out: usize) -> (f64, f64) {
    let last = (lod_count - 1) as f64;
    let target_roughness = lod_out as f64 / last;
    let source_roughness = lod_in as f64 / last;
    let incremental =
        (target_roughness * target_roughness - source_roughness * source_roughness).sqrt();
    // "Apply blur strength mapping for better quality across the roughness
    // range" — not a normalisation, a deliberate extra 1.25× of blur.
    let blur_strength = target_roughness * 1.25;
    // `_lodMax - lodIn` in JS arithmetic: `lodIn` runs past `lodMax` for the
    // extra LODs, so the two go negative (-1 and -2 for a 256² source).
    (incremental * blur_strength, lod_max as f64 - lod_in as f64)
}

/// The `( x, y )` of level `lod_out`'s tile and the level's face size.
///
/// `lodOut > lodMax - LOD_MIN` is what sends the extra levels sideways: they
/// all have face size `2 ^ LOD_MIN`, so they would otherwise stack on top of
/// each other in the same corner.
pub fn tile_rect(
    lod_max: usize,
    cube_size: usize,
    output_size: usize,
    lod_out: usize,
) -> (usize, usize, usize) {
    let column = (lod_out + LOD_MIN).saturating_sub(lod_max);
    (
        3 * output_size * column,
        4 * (cube_size - output_size),
        output_size,
    )
}

/// The `wgpu::Texture` behind a render target's colour attachment.
fn gpu_texture_of(target: &RenderTarget) -> wgpu::Texture {
    target.texture().with_gpu(|gpu| gpu.clone())
}

/// `_createRenderTarget( width, height, depthBuffer )`.
fn create_render_target(
    width: u32,
    height: u32,
    depth_buffer: bool,
) -> Result<RenderTarget, Error> {
    let target = RenderTarget::new_with_options(
        width,
        height,
        RenderTargetOptions {
            texture_type: TextureType::HalfFloat,
            samples: 0,
            depth_buffer,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )?;
    target.set_scissor_test(true);
    Ok(target)
}

/// `_setViewport( target, x, y, width, height )` — the WebGPU arm, the only one
/// this port has: origin top-left, no flip.
fn set_viewport(target: &RenderTarget, x: usize, y: usize, width: usize, height: usize) {
    target.set_viewport(x as f64, y as f64, width as f64, height as f64);
    target.set_scissor(x as f64, y as f64, width as f64, height as f64);
}

/// `_getMaterial( type )`.
fn pmrem_material(name: &'static str) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.depth_test = false;
    material.depth_write = false;
    material.blending = Blending::No;
    material.name = name;
    material
}

/// `_outputDirection` — `attribute( 'outputDirection' ).normalize()`.
///
/// The attribute is read in the fragment stage, so the builder carries the raw
/// direction across as a varying and normalises it there, which is the dump's
/// `varyings.nodeVarying4 = outputDirection;` / `normalize( nodeVarying4 )`.
pub fn output_direction() -> NodeRef {
    attribute("outputDirection", Type::Vec3).normalize()
}

/// `_getCubemapMaterial( envTexture )` — `cubeTexture( envTexture,
/// _outputDirection )` and nothing else.
pub fn cubemap_material(cubemap: &CubeTexture) -> MeshBasicNodeMaterial {
    let mut material = pmrem_material("PMREM_cubemap");
    let dir = material_env_rotation().mul(vec4_join(vec![output_direction(), float(1.0)]));
    material.fragment_node = Some(cube_texture(cubemap, dir));
    material
}

/// `_getGGXShader( lodMax, width, height )`.
///
/// The three cubeUV constants are baked `float()`s here, exactly as three bakes
/// them: the atlas' size is fixed for the life of the material, so they are
/// literals in the WGSL rather than uniforms. (`PMREMNode`'s side of the same
/// numbers *are* uniforms, because a scene material outlives the environment it
/// is pointed at — the asymmetry is three's, not the port's.)
pub fn ggx_material(
    lod_max: usize,
    width: f64,
    height: f64,
) -> (MeshBasicNodeMaterial, GgxUniforms) {
    let (roughness, roughness_cell) = uniform_settable(Type::F32, vec![0.0]);
    let (mip_int, mip_int_cell) = uniform_settable(Type::F32, vec![0.0]);
    let env_map = Texture::render_target(
        width as u32,
        height as u32,
        wgpu::TextureFormat::Rgba16Float,
    );
    let size = CubeUvSize {
        texel_width: float(1.0 / width),
        texel_height: float(1.0 / height),
        max_mip: float(lod_max as f64),
    };

    let mut material = pmrem_material("PMREM_ggx");
    material.fragment_node = Some(pmrem_utils::ggx_convolution(
        roughness,
        mip_int,
        &env_map,
        output_direction(),
        GGX_SAMPLES,
        &size,
    ));

    (
        material,
        GgxUniforms {
            env_map,
            roughness: roughness_cell,
            mip_int: mip_int_cell,
        },
    )
}

/// `_createPlanes( lodMax )` — one six-quad geometry per level.
///
/// Each quad covers one face's tile in clip space and carries the cube
/// direction of its corners in an `outputDirection` attribute. The uvs
/// **overshoot the face by one texel** (`min = -1/(size-2)`, `max = 1 +
/// 1/(size-2)`), which bakes the cubeUV border into the directions and is why
/// `bilinearCubeUV` reads back with `uv * ( faceSize - 2 ) + 1`.
pub fn create_planes(lod_max: usize) -> Vec<LodMesh> {
    let total_lods = lod_max - LOD_MIN + 1 + EXTRA_LODS;
    let mut meshes = Vec::with_capacity(total_lods);
    let mut lod = lod_max;

    for _ in 0..total_lods {
        let size_lod = 1usize << lod;

        let texel_size = 1.0 / (size_lod as f64 - 2.0);
        let lo = -texel_size;
        let hi = 1.0 + texel_size;
        let uv1 = [lo, lo, hi, lo, hi, hi, lo, lo, hi, hi, lo, hi];

        const CUBE_FACES: usize = 6;
        const VERTICES: usize = 6;
        const POSITION_SIZE: usize = 3;

        let mut position = vec![0.0f32; POSITION_SIZE * VERTICES * CUBE_FACES];
        let mut output_direction = vec![0.0f32; POSITION_SIZE * VERTICES * CUBE_FACES];

        for (face, &face_idx) in FACE_LIB.iter().enumerate() {
            let x = (face % 3) as f64 * 2.0 / 3.0 - 1.0;
            let y = if face > 2 { 0.0 } else { -1.0 };
            let coordinates = [
                x,
                y,
                0.0,
                x + 2.0 / 3.0,
                y,
                0.0,
                x + 2.0 / 3.0,
                y + 1.0,
                0.0,
                x,
                y,
                0.0,
                x + 2.0 / 3.0,
                y + 1.0,
                0.0,
                x,
                y + 1.0,
                0.0,
            ];

            let base = POSITION_SIZE * VERTICES * face_idx;
            for (i, value) in coordinates.iter().enumerate() {
                position[base + i] = *value as f32;
            }

            for vertex in 0..VERTICES {
                let u = uv1[vertex * 2] * 2.0 - 1.0;
                let v = uv1[vertex * 2 + 1] * 2.0 - 1.0;

                // RH coordinate system; PMREM face-indexing convention.
                let direction = match face_idx {
                    0 => [1.0, v, u],   // pos x
                    1 => [-u, 1.0, -v], // pos y
                    2 => [-u, v, 1.0],  // pos z
                    3 => [-1.0, v, -u], // neg x
                    4 => [-u, -1.0, v], // neg y
                    _ => [u, v, -1.0],  // neg z
                };
                let at = (face_idx * VERTICES + vertex) * POSITION_SIZE;
                for (i, value) in direction.iter().enumerate() {
                    output_direction[at + i] = *value as f32;
                }
            }
        }

        let mut geometry = BufferGeometry::new();
        geometry.set_attribute("position", BufferAttribute::new(position, POSITION_SIZE));
        geometry.set_attribute(
            "outputDirection",
            BufferAttribute::new(output_direction, POSITION_SIZE),
        );
        meshes.push(LodMesh {
            geometry: Rc::new(geometry),
            size: size_lod,
        });

        if lod > LOD_MIN {
            lod -= 1;
        }
    }

    meshes
}
