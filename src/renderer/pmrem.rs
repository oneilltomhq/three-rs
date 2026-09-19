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
//! `fromCubemap`, `fromEquirectangular` and `fromScene` are ported. The first
//! two differ only in `_setSizeFromTexture` and in which of the two one-tap
//! materials fills level 0, which is three's own shape. `fromScene` fills
//! level 0 differently again: it renders a *scene* six times, through the
//! renderer's ordinary render path, into 256² viewport slices of the atlas
//! with `autoClear` off. `_blur` / `sphericalGaussianBlur` — the `sigma > 0`
//! arm of `fromScene` — still has no caller.

use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::core::{BufferAttribute, BufferGeometry};
use crate::error::Error;
use crate::geometries::box_geometry_default;
use crate::materials::{Blending, MeshBasicNodeMaterial, Side};
use crate::math::{Color, Vector3};
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::pmrem_utils::{self, CubeUvSize};
use crate::nodes::tsl::{
    attribute, cube_texture, equirect_uv, float, material_env_rotation, texture_level,
    uniform_settable, vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::{Background, Mesh, Scene};
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

/// `BLUR_SAMPLES` — spiral samples per `sphericalGaussianBlur` pass. Only
/// `fromScene( scene, sigma > 0 )` reaches it.
pub const BLUR_SAMPLES: usize = 20;

/// `fromScene( scene, sigma = 0, near = 0.1, far = 100, { size = 256 } )` —
/// three's defaults, which is all any ported example asks for.
pub const SCENE_SIZE: usize = 256;
/// `fromScene`'s default `near`.
pub const SCENE_NEAR: f64 = 0.1;
/// `fromScene`'s default `far`.
pub const SCENE_FAR: f64 = 100.0;

/// `_sceneToCubeUV`'s `upSign` — px, py, pz, nx, ny, nz.
///
/// From `src/renderers/common/extras/PMREMGenerator.js`, the **WebGPU**
/// generator. r186 carries a second, older copy at `src/extras/` for the WebGL
/// renderer whose tables are `[ 1, -1, 1, 1, 1, 1 ]` / `[ 1, 1, 1, -1, -1, -1 ]`
/// and which draws the background box inside the face loop; that is not the
/// file `three.webgpu.js` is built from, and following it would put five of the
/// six faces somewhere three does not. See
/// `docs/webgpu_furnace_test-progress.md`.
const UP_SIGN: [f64; 6] = [1.0, 1.0, 1.0, 1.0, -1.0, 1.0];
/// `_sceneToCubeUV`'s `forwardSign`.
const FORWARD_SIGN: [f64; 6] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0];

/// `_faceLib` — the WebGPU face order, which is the order the six quads of a
/// lod plane are written into the vertex arrays.
const FACE_LIB: [usize; 6] = [3, 1, 5, 0, 4, 2];

/// The blur material's `_uniformsMap` entry: the cells `_blurPass` rewrites
/// between its two halves.
pub struct BlurUniforms {
    /// `blurUniforms.envMap.value` — the borrowed handle, repointed between
    /// the atlas and the ping-pong target. See [`GgxUniforms::env_map`].
    pub env_map: Texture,
    /// `blurUniforms.sigma` — the blur radius in radians, already divided by
    /// √2 by `PmremGenerator::blur`.
    pub sigma: SettableValue,
    /// `blurUniforms.mipInt` — the atlas level being read.
    pub mip_int: SettableValue,
}

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

/// What a PMREM is generated from.
///
/// Three branches on `texture.mapping` inside `_setSizeFromTexture` and
/// `_textureToCubeUV`; the port makes the two cases a type, because a
/// `CubeTexture` and a `Texture` are different types here and the mapping
/// constant carries no other information. Anything that is not a cube mapping
/// takes the equirectangular branch upstream, which is why the decoded HDR's
/// `UVMapping` needs no representation at all.
#[derive(Clone, Debug)]
pub enum PmremSource {
    /// `fromCubemap( cubemap )`.
    Cube(CubeTexture),
    /// `fromEquirectangular( equirectangular )` — a 2-D longitude/latitude map.
    Equirectangular(Texture),
}

impl PmremSource {
    /// `_setSizeFromTexture( texture )`'s argument: a cube is sized from face
    /// 0 (an empty one falls back to 16), an equirect map from **a quarter of
    /// its width**, so the 1024×512 `spot1Lux.hdr` gives a 256² face and the
    /// same 768×1024 atlas a 256² cube does.
    fn cube_size(&self) -> usize {
        match self {
            PmremSource::Cube(cube) => match cube.size().0 {
                0 => 16,
                width => width as usize,
            },
            PmremSource::Equirectangular(texture) => texture.size().0 as usize / 4,
        }
    }
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
    /// `this._blurMaterial` plus its `_uniformsMap` entry. Built by `_init`
    /// alongside the GGX one — three builds it unconditionally too — but only
    /// ever bound when `fromScene` is called with a non-zero sigma.
    blur: Option<(MeshBasicNodeMaterial, BlurUniforms)>,
    /// `this._cubemapMaterial`.
    cubemap_material: Option<MeshBasicNodeMaterial>,
    /// `this._equirectMaterial`.
    equirect_material: Option<MeshBasicNodeMaterial>,
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
            blur: None,
            cubemap_material: None,
            equirect_material: None,
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
        self.blur = Some(blur_material(self.lod_max, width as f64, height as f64));
        Ok(())
    }

    /// `fromScene( scene, sigma )` — the PMREM of a rendered scene, with
    /// three's remaining defaults (`near = 0.1`, `far = 100`, `size = 256`,
    /// the cube camera at the origin).
    ///
    /// This is the entry point that does not start from a texture at all:
    /// `_sceneToCubeUV` renders the scene six times, with a 90° cube camera,
    /// into the six 256² tiles of level 0 of the atlas.
    ///
    /// `sigma` is the blur radius **in radians**, applied to level 0 before the
    /// GGX ladder runs. `webgpu_furnace_test` passes 0, which skips it;
    /// `RoomEnvironment`'s six users all pass `0.04`, which is the whole
    /// reason `_blur` exists.
    ///
    /// `scene` is `&mut` because three's `_sceneToCubeUV` assigns
    /// `scene.background = null` for the duration of a solid-colour background
    /// and puts it back afterwards; the port does the same to the same field.
    pub fn from_scene(
        &mut self,
        renderer: &mut super::Renderer,
        scene: &mut Scene,
        sigma: f64,
        render_target: Option<RenderTarget>,
    ) -> Result<RenderTarget, Error> {
        self.set_size(SCENE_SIZE);

        let old_target = renderer.render_target();

        let target = match render_target {
            Some(target) => target,
            // `_allocateTarget( true )` — a depth buffer, because what fills
            // level 0 here is a scene and not a screen-space blit.
            None => self.allocate_target(true)?,
        };
        self.init(&target)?;
        self.scene_to_cube_uv(renderer, scene, SCENE_NEAR, SCENE_FAR, &target);

        if sigma > 0.0 {
            self.blur(renderer, &target, 0, 0, sigma);
        }

        self.apply_pmrem(renderer, &target);
        self.cleanup(renderer, old_target, &target);

        Ok(target)
    }

    /// `_sceneToCubeUV( scene, near, far, cubeUVRenderTarget, position )`.
    ///
    /// The renderer capability this needs — and the only one the rung adds —
    /// is *render a scene into a viewport slice of a render target without
    /// clearing it*. Both halves already existed:
    /// [`RenderTarget::set_viewport`] / [`RenderTarget::set_scissor`], which
    /// `_setViewport` writes and which `Renderer::render()` reads through the
    /// target's pass, and [`Renderer::auto_clear`], which decides whether that
    /// pass loads or clears. So this is `PMREMGenerator.js:448-545` and
    /// nothing else.
    ///
    /// [`RenderTarget::set_viewport`]: super::render_target::RenderTarget::set_viewport
    /// [`RenderTarget::set_scissor`]: super::render_target::RenderTarget::set_scissor
    /// [`Renderer::auto_clear`]: super::Renderer::auto_clear
    fn scene_to_cube_uv(
        &mut self,
        renderer: &mut super::Renderer,
        scene: &mut Scene,
        near: f64,
        far: f64,
        target: &RenderTarget,
    ) {
        let position = Vector3::ZERO;
        let mut cube_camera = PerspectiveCamera::new(90.0, 1.0, near, far);

        let original_auto_clear = renderer.auto_clear;
        let clear_color = renderer.clear_color();
        renderer.auto_clear = false;

        // `if ( background ) { if ( background.isColor ) { … } } else { … }`:
        // a colour background becomes the box's colour and is lifted off the
        // scene for the duration; anything else stays on it and is drawn by
        // the cube camera like any other skybox; no background at all falls
        // back to the renderer's clear colour.
        let background = scene.background.clone();
        let (use_solid_color, color) = match &background {
            Some(Background::Color(color)) => {
                scene.background = None;
                (true, *color)
            }
            Some(_) => (false, clear_color),
            None => (true, clear_color),
        };

        renderer.set_render_target(Some(target.clone()));
        // `renderer.clear()` — colour and depth of the whole atlas, ignoring
        // the `autoClear` switches that were just turned off.
        renderer.clear(true, true);

        if use_solid_color {
            // `renderer.render( backgroundBox, cubeCamera )`, once, *before*
            // the face loop and so at the target's full viewport: three's dump
            // of `webgpu_furnace_test` draws the 36-index box over the whole
            // 768×1024 atlas. A unit cube seen from its own centre through a
            // 90° frustum fills any viewport, which is why one draw is enough
            // and why the six face renders that follow have nothing to add.
            let mut box_scene = background_box(color);
            renderer.render(&mut box_scene, &mut cube_camera);
        }

        for i in 0..6 {
            let (up, look_at) = face_camera(i, position);
            {
                let mut object = cube_camera.node.borrow_mut();
                object.position.set(position.x, position.y, position.z);
                object.up = up;
            }
            cube_camera.look_at(&look_at);

            let (x, y, width, height) = face_tile(self.cube_size, i);
            set_viewport(target, x, y, width, height);

            renderer.render(scene, &mut cube_camera);
        }

        renderer.auto_clear = original_auto_clear;
        scene.background = background;
    }

    /// `_cleanup( outputTarget )`.
    fn cleanup(
        &self,
        renderer: &mut super::Renderer,
        old_target: Option<RenderTarget>,
        target: &RenderTarget,
    ) {
        renderer.set_render_target(old_target);
        target.set_scissor_test(false);
        let (width, height) = target.size();
        set_viewport(target, 0, 0, width as usize, height as usize);
    }

    /// `fromCubemap( cubemap )` — the PMREM of a cube texture. The returned
    /// target's `texture()` is what a `pmrem_texture()` node samples.
    pub fn from_cubemap(
        &mut self,
        renderer: &mut super::Renderer,
        cubemap: &CubeTexture,
        render_target: Option<RenderTarget>,
    ) -> Result<RenderTarget, Error> {
        self.from_texture(renderer, &PmremSource::Cube(cubemap.clone()), render_target)
    }

    /// `fromEquirectangular( equirectangular )` — the PMREM of a 2-D
    /// longitude/latitude map, which is what an `.hdr` decodes to.
    pub fn from_equirectangular(
        &mut self,
        renderer: &mut super::Renderer,
        equirectangular: &Texture,
        render_target: Option<RenderTarget>,
    ) -> Result<RenderTarget, Error> {
        self.from_texture(
            renderer,
            &PmremSource::Equirectangular(equirectangular.clone()),
            render_target,
        )
    }

    /// `_fromTexture( texture, renderTarget )` — both entry points, which
    /// differ only in the size they take from the source and in the material
    /// `_textureToCubeUV` picks.
    pub fn from_texture(
        &mut self,
        renderer: &mut super::Renderer,
        source: &PmremSource,
        render_target: Option<RenderTarget>,
    ) -> Result<RenderTarget, Error> {
        // `_setSizeFromTexture()`.
        self.set_size(source.cube_size());

        let old_target = renderer.render_target();

        let target = match render_target {
            Some(target) => target,
            None => self.allocate_target(false)?,
        };
        self.init(&target)?;
        self.texture_to_cube_uv(renderer, source, &target);
        self.apply_pmrem(renderer, &target);

        self.cleanup(renderer, old_target, &target);

        Ok(target)
    }

    /// `_textureToCubeUV( texture, cubeUVRenderTarget )` — the one blit that
    /// fills level 0, the top `3 * cubeSize` × `2 * cubeSize` of the atlas.
    fn texture_to_cube_uv(
        &mut self,
        renderer: &mut super::Renderer,
        source: &PmremSource,
        target: &RenderTarget,
    ) {
        // Three keeps one material per kind and assigns
        // `fragmentNode.value = texture`; the port bakes the source into the
        // node, so the material is built for the texture it is first asked
        // about. `PmremEnvironment` owns one generator per source texture, so
        // a generator never sees two.
        let material = match source {
            PmremSource::Cube(cube) => self
                .cubemap_material
                .get_or_insert_with(|| cubemap_material(cube))
                .clone(),
            PmremSource::Equirectangular(map) => self
                .equirect_material
                .get_or_insert_with(|| equirect_material(map))
                .clone(),
        };

        let size = self.cube_size;
        set_viewport(target, 0, 0, 3 * size, 2 * size);
        renderer.set_render_target(Some(target.clone()));
        let geometry = self.lod_meshes[0].geometry.clone();
        // `autoClear` is still whatever the app set, so this first pass clears
        // the atlas the way three's does.
        renderer.render_pmrem_mesh(geometry, &material, true);
    }

    /// `_blur( cubeUVRenderTarget, lodIn, lodOut, sigma )` — the two-pass
    /// spherical Gaussian.
    ///
    /// "Two passes of sigma / sqrt( 2 ) compose to a blur of sigma while
    /// squaring the effective sample count. Sigmas beyond PI are visually
    /// indistinguishable from a uniform blur, so clamp to keep the shader
    /// math finite."
    fn blur(
        &mut self,
        renderer: &mut super::Renderer,
        target: &RenderTarget,
        lod_in: usize,
        lod_out: usize,
        sigma: f64,
    ) {
        let blur_sigma = sigma.min(std::f64::consts::PI) / std::f64::consts::SQRT_2;
        let ping_pong = self
            .ping_pong
            .clone()
            .expect("three-rs: _init() allocated the ping-pong target");

        self.blur_pass(renderer, target, &ping_pong, lod_in, lod_out, blur_sigma);
        self.blur_pass(renderer, &ping_pong, target, lod_out, lod_out, blur_sigma);
    }

    /// `_blurPass( targetIn, targetOut, lodIn, lodOut, sigmaRadians )`.
    ///
    /// Note the viewport arithmetic is **not** [`Self::tile`]'s: `_blurPass`
    /// computes `y` as `4 * ( cubeSize - outputSize )` directly rather than
    /// through `_sizeLods`, and with `lodIn == lodOut == 0` both halves land
    /// on the level-0 rectangle. It is spelled out here because it is three's
    /// own duplication and because a shared helper would hide that the two
    /// expressions are only equal while `lodOut <= lodMax - LOD_MIN`.
    fn blur_pass(
        &mut self,
        renderer: &mut super::Renderer,
        target_in: &RenderTarget,
        target_out: &RenderTarget,
        lod_in: usize,
        lod_out: usize,
        sigma_radians: f64,
    ) {
        let output_size = self.lod_meshes[lod_out].size;
        let (x, y) = blur_tile(self.lod_max, self.cube_size, output_size, lod_out);

        let geometry = self.lod_meshes[lod_out].geometry.clone();
        let (material, uniforms) = self
            .blur
            .as_ref()
            .expect("three-rs: _init() built the blur material");
        let material = material.clone();

        uniforms.env_map.set_gpu(gpu_texture_of(target_in));
        uniforms.sigma.set(vec![sigma_radians]);
        uniforms
            .mip_int
            .set(vec![self.lod_max as f64 - lod_in as f64]);

        set_viewport(target_out, x, y, 3 * output_size, 2 * output_size);
        renderer.set_render_target(Some(target_out.clone()));
        renderer.render_pmrem_mesh(geometry, &material, false);
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

/// `_blurPass`'s viewport origin: `x = 3 * outputSize * ( lodOut > lodMax -
/// LOD_MIN ? lodOut - lodMax + LOD_MIN : 0 )`, `y = 4 * ( cubeSize -
/// outputSize )`.
///
/// Pure, and gated in `tests/pmrem_scene.rs` against the numbers three's own
/// `_blurPass` computes, because with `lodIn == lodOut == 0` — the only call
/// on the ladder — a wrong rectangle still blurs *something* and still
/// produces a plausible atlas.
pub fn blur_tile(
    lod_max: usize,
    cube_size: usize,
    output_size: usize,
    lod_out: usize,
) -> (usize, usize) {
    // `lodOut - lodMax + LOD_MIN` under the ternary that already proved it
    // positive; `saturating_sub` spells the guard rather than trusting it.
    let column = (lod_out + LOD_MIN).saturating_sub(lod_max);
    let x = if lod_out > lod_max.saturating_sub(LOD_MIN) {
        3 * output_size * column
    } else {
        0
    };
    (x, 4 * (cube_size - output_size))
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

/// `_sceneToCubeUV`'s per-face camera basis: the `up` vector and the point the
/// cube camera looks at, for face `i` of the WebGPU face order.
///
/// Split out with [`face_tile`] because the pair is pure and is the numeric
/// gate on the six faces — `tests/pmrem_scene.rs` checks both against three's
/// own `upSign` / `forwardSign` tables. A permuted face, a flipped `up` or a
/// column/row swap is invisible in a solid-colour furnace and wrong in every
/// environment that has structure in it.
pub fn face_camera(face: usize, position: Vector3) -> (Vector3, Vector3) {
    let column = face % 3;
    let up = match column {
        1 => Vector3::new(0.0, 0.0, UP_SIGN[face]),
        _ => Vector3::new(0.0, UP_SIGN[face], 0.0),
    };
    let look_at = match column {
        0 => Vector3::new(position.x + FORWARD_SIGN[face], position.y, position.z),
        1 => Vector3::new(position.x, position.y + FORWARD_SIGN[face], position.z),
        _ => Vector3::new(position.x, position.y, position.z + FORWARD_SIGN[face]),
    };
    (up, look_at)
}

/// `_setViewport( cubeUVRenderTarget, col * size, i > 2 ? size : 0, size, size )`
/// — face `i`'s tile of level 0 of the atlas, top-left origin.
pub fn face_tile(cube_size: usize, face: usize) -> (usize, usize, usize, usize) {
    (
        (face % 3) * cube_size,
        if face > 2 { cube_size } else { 0 },
        cube_size,
        cube_size,
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

/// `this._backgroundBox` — `new Mesh( new BoxGeometry(), new
/// MeshBasicMaterial( { name: 'PMREM.Background', side: BackSide, depthWrite:
/// false, depthTest: false } ) )`, with the colour copied from the scene's
/// background (or the renderer's clear colour).
///
/// Three caches the mesh on the generator and disposes it in `dispose()`; the
/// port builds it per `fromScene`, because a generator generates once and the
/// only thing the cache saves is a `BoxGeometry` and a material. Nothing
/// observable depends on it — the WGSL is `MeshBasicNodeMaterial`'s, which
/// `webgpu_materials_basic` already gates.
///
/// Three renders it as a bare `Mesh`; `Renderer.render()` then substitutes its
/// own empty `_scene` for the scene-level state, so a background-less `Scene`
/// with the box in it is the same thing here.
fn background_box(color: Color) -> Scene {
    let mesh = Mesh::new(Rc::new(box_geometry_default()), background_material(color));
    let scene = Scene::new();
    scene.add(&mesh);
    scene
}

/// The background box's material on its own, so `examples/dump_wgsl.rs` can
/// diff it against three's `PMREM.Background` modules.
pub fn background_material(color: Color) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = "PMREM.Background";
    material.color = color;
    material.side = Side::Back;
    material.depth_write = false;
    material.depth_test = false;
    material
}

/// `_getCubemapMaterial( envTexture )` — `cubeTexture( envTexture,
/// _outputDirection )` and nothing else.
pub fn cubemap_material(cubemap: &CubeTexture) -> MeshBasicNodeMaterial {
    let mut material = pmrem_material("PMREM_cubemap");
    let dir = material_env_rotation().mul(vec4_join(vec![output_direction(), float(1.0)]));
    material.fragment_node = Some(cube_texture(cubemap, dir));
    material
}

/// `_getEquirectMaterial( envTexture )` — `texture( envTexture, equirectUV(
/// _outputDirection ), 0 )` and nothing else.
///
/// Note what is *not* here: the environment rotation. `cubeTexture()` applies
/// `materialEnvRotation` inside `CubeTextureNode.setupUV()`, so the cubemap
/// material carries it; a plain 2-D `texture()` node does not, so this one
/// does not either. Three's dump of `webgpu_pmrem_test` agrees — its
/// `PMREM_equirect` fragment module has one uniform, the map.
pub fn equirect_material(map: &Texture) -> MeshBasicNodeMaterial {
    let mut material = pmrem_material("PMREM_equirect");
    material.fragment_node = Some(texture_level(
        map,
        equirect_uv(output_direction()),
        float(0.0),
    ));
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

/// `_getBlurShader( lodMax, width, height )` — the `PMREM_blur` material.
///
/// The same shape as [`ggx_material`], with `sigma` in place of `roughness`
/// and [`spherical_gaussian_blur`] in place of the VNDF convolution.
///
/// [`spherical_gaussian_blur`]: crate::nodes::pmrem_utils::spherical_gaussian_blur
pub fn blur_material(
    lod_max: usize,
    width: f64,
    height: f64,
) -> (MeshBasicNodeMaterial, BlurUniforms) {
    let (sigma, sigma_cell) = uniform_settable(Type::F32, vec![0.0]);
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

    let mut material = pmrem_material("PMREM_blur");
    material.fragment_node = Some(pmrem_utils::spherical_gaussian_blur(
        BLUR_SAMPLES,
        sigma,
        output_direction(),
        mip_int,
        &env_map,
        &size,
    ));

    (
        material,
        BlurUniforms {
            env_map,
            sigma: sigma_cell,
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
