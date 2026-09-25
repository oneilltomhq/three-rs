//! Port of `three.js/src/renderers/common/extras/PMREMGenerator.js`.
//!
//! A PMREM is a *P*refiltered, *M*ipmapped *R*adiance *E*nvironment *M*ap. Since
//! three.js 2f80402 it is a real cube render target: `rgba16float`, one mip
//! level per prefiltered roughness, `log2( size ) - LOD_MIN + 1` levels in all
//! (six for the usual 256² cube, down to 8²), read with a single
//! `textureSampleLevel` at [`roughness_to_mip`]. Level `lod` holds the
//! environment convolved with the GGX lobe at [`lod_to_roughness`]`( lod )`.
//!
//! Every level is filled the same way, `_renderCube( target, lod, material )`:
//! a 5×5×5 box seen from inside by a cube camera, one render per face. The
//! source of the convolution is a second cube, the *source target*, which holds
//! the environment at full resolution with a generated mip chain. So a PMREM is
//!
//! 1. the environment into the source target — a one-tap cube or 4-tap
//!    equirect material, or the scene itself for `fromScene` — then its mips;
//! 2. for `fromScene` with `sigma > 0`, a spherical Gaussian blur from the
//!    source into level 0 of the PMREM and back;
//! 3. level by level, `PMREM_ggx` (filtered importance sampling of the
//!    source's mips) or, for the last [`INTEGRATION_LEVELS`], `PMREM_integration`
//!    (every texel of the source's 16² level, weighted).
//!
//! **What this port does differently.** Three renders into a cube face at a mip
//! level directly; [`Renderer`](super::Renderer) has no layered colour
//! attachment, so each face is drawn into a 2-D render target of the level's
//! size and copied into its (layer, level) — the same exact, same-format copy
//! `CubeRenderTarget.fromEquirectangularTexture` uses here, recorded in
//! `docs/nodes.md` §13.
//!
//! [`roughness_to_mip`]: crate::nodes::pmrem_utils::roughness_to_mip

use std::collections::HashMap;
use std::rc::Rc;

use crate::cameras::PerspectiveCamera;
use crate::core::BufferGeometry;
use crate::error::Error;
use crate::geometries::box_geometry;
use crate::materials::{Blending, MeshBasicNodeMaterial, Side};
use crate::math::Vector3;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::pmrem_utils;
use crate::nodes::tsl::{
    block, cube_texture, dpdx, dpdy, equirect_uv, float, position_world_direction, texture_level,
    to_const, uniform_settable, uniform_value, vec4_join,
};
use crate::nodes::NodeRef;
use crate::objects::{Background, Mesh, Scene};
use crate::textures::{CubeTexture, Texture, TextureFilter, TextureType};

use super::cube_render_target::{FACES, FOV};
use super::render_target::{RenderTarget, RenderTargetOptions};

/// `MIN_SIZE` — the smallest cube a PMREM is generated at, whatever the source.
pub const MIN_SIZE: u32 = 256;
/// `LOD_MIN` — the smallest level is `2 ^ LOD_MIN` = 8² a face.
pub const LOD_MIN: u32 = 3;
/// `BLUR_SAMPLES` — taps of the spherical Gaussian.
pub const BLUR_SAMPLES: usize = 20;
/// `GGX_SAMPLES` — importance samples per texel of a `PMREM_ggx` level.
pub const GGX_SAMPLES: usize = 256;
/// `INTEGRATION_SIZE` — the face size of the source level `PMREM_integration`
/// sums over.
pub const INTEGRATION_SIZE: u32 = 16;
/// `INTEGRATION_LEVELS` — how many of the roughest levels use
/// `PMREM_integration` rather than `PMREM_ggx`.
pub const INTEGRATION_LEVELS: u32 = 3;
/// `fromScene`'s defaults.
pub const SCENE_SIZE: u32 = 256;
pub const SCENE_NEAR: f64 = 0.1;
pub const SCENE_FAR: f64 = 100.0;
/// `new CubeCamera( 1, 10 )` — `_renderCube`'s camera.
const CUBE_NEAR: f64 = 1.0;
const CUBE_FAR: f64 = 10.0;

/// Three branches on `texture.mapping` inside `_fromTexture` and
/// `_textureToCubemap`; the port makes the two cases a type, because a
/// `CubeTexture` and a `Texture` are different types here.
#[derive(Clone, Debug)]
pub enum PmremSource {
    /// `fromCubemap( cubemap )`.
    Cube(CubeTexture),
    /// `fromEquirectangular( equirectangular )` — a 2-D longitude/latitude map.
    Equirectangular(Texture),
}

impl PmremSource {
    /// `_fromTexture()`'s argument to `_setSize`: a cube is sized from face 0
    /// (an empty one falls back to `MIN_SIZE`), an equirect map from **a
    /// quarter of its width**.
    pub fn requested_size(&self) -> u32 {
        match self {
            PmremSource::Cube(cube) => match cube.size().0 {
                0 => MIN_SIZE,
                width => width,
            },
            PmremSource::Equirectangular(texture) => texture.size().0 / 4,
        }
    }
}

/// `_setSize( cubeSize )` — `max( MIN_SIZE, floorPowerOfTwo( cubeSize ) )`.
pub fn cube_size_for(requested: u32) -> u32 {
    let floor_pow2 = if requested == 0 {
        0
    } else {
        1u32 << (31 - requested.leading_zeros())
    };
    MIN_SIZE.max(floor_pow2)
}

/// `_allocateTarget()`'s `maxLod = log2( size ) - LOD_MIN`.
pub fn max_lod_for(size: u32) -> u32 {
    size.trailing_zeros().saturating_sub(LOD_MIN)
}

/// `PMREMGenerator.lodToRoughness( lod, maxLod )` —
/// `maxLod > 0 ? 1 - sqrt( 1 - lod / maxLod ) : 0`.
pub fn lod_to_roughness(lod: u32, max_lod: u32) -> f64 {
    if max_lod > 0 {
        1.0 - (1.0 - lod as f64 / max_lod as f64).sqrt()
    } else {
        0.0
    }
}

/// `_applyPMREM()`'s `lodBias`: `log2( size ) + 0.5 * log2( 6 / ( GGX_SAMPLES
/// * roughness^4 ) ) + 0.5`, zero at roughness 0. It is the source mip whose
/// texel solid angle matches one importance sample of the lobe, before the
/// per-sample `log2( alpha2 * invQ )` term.
pub fn lod_bias(size: u32, roughness: f64) -> f64 {
    if roughness > 0.0 {
        (size as f64).log2() + 0.5 * (6.0 / (GGX_SAMPLES as f64 * roughness.powi(4))).log2() + 0.5
    } else {
        0.0
    }
}

/// Which material fills level `lod` of a PMREM with `max_lod + 1` levels:
/// `lod > maxLod - INTEGRATION_LEVELS` takes `PMREM_integration`.
pub fn uses_integration(lod: u32, max_lod: u32) -> bool {
    lod as i64 > max_lod as i64 - INTEGRATION_LEVELS as i64
}

/// `_allocateTarget()` — the PMREM cube for a (already `_setSize`d) `size`.
pub fn allocate_target(size: u32) -> CubeTexture {
    CubeTexture::pmrem_render_target(size, Some(max_lod_for(size) + 1))
}

struct GgxMaterial {
    material: MeshBasicNodeMaterial,
    roughness: SettableValue,
    lod_bias: SettableValue,
}

struct IntegrationMaterial {
    material: MeshBasicNodeMaterial,
    roughness: SettableValue,
    source_lod: SettableValue,
}

/// `new PMREMGenerator( renderer )`.
///
/// The renderer is not held: every entry point takes it, because a `Renderer`
/// in this port is a `&mut` owner rather than a JS object reference.
///
/// Three keeps one material per kind and repoints its `envMap` uniform
/// through `_uniformsMap`; the port bakes the texture into the node, so the
/// GGX and integration materials are rebuilt when the source target is, and
/// the two blur passes (source → PMREM, PMREM → source) are two materials.
pub struct PmremGenerator {
    /// `this._cubeSize`.
    cube_size: u32,
    /// `this._sourceTarget`, and whether it was allocated with a depth buffer.
    source: Option<(CubeTexture, bool)>,
    /// The 2-D targets faces are drawn into before the copy, one per (size,
    /// depth buffer).
    face_targets: HashMap<(u32, bool), RenderTarget>,
    /// `this._boxMesh.geometry` — `new BoxGeometry( 5, 5, 5 )`.
    box_geometry: Rc<BufferGeometry>,
    cubemap_material: Option<MeshBasicNodeMaterial>,
    equirect_material: Option<MeshBasicNodeMaterial>,
    ggx: Option<GgxMaterial>,
    integration: Option<IntegrationMaterial>,
}

impl Default for PmremGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PmremGenerator {
    pub fn new() -> Self {
        Self {
            cube_size: 0,
            source: None,
            face_targets: HashMap::new(),
            box_geometry: Rc::new(box_geometry(5.0, 5.0, 5.0, 1, 1, 1)),
            cubemap_material: None,
            equirect_material: None,
            ggx: None,
            integration: None,
        }
    }

    /// `this._cubeSize`, once an entry point has set it.
    pub fn cube_size(&self) -> u32 {
        self.cube_size
    }

    /// `_setSize( cubeSize )`.
    fn set_size(&mut self, requested: u32) {
        self.cube_size = cube_size_for(requested);
    }

    /// `_getSourceTarget( depthBuffer )` — a full-chain cube of the current
    /// size, reallocated when the size changes or a depth buffer is newly
    /// needed. The GGX and integration materials read it, so they go with it.
    fn source_target(&mut self, depth_buffer: bool) -> CubeTexture {
        let size = self.cube_size;
        let fresh = match &self.source {
            None => true,
            Some((source, depth)) => source.size().0 != size || (depth_buffer && !depth),
        };
        if fresh {
            self.source = Some((CubeTexture::pmrem_render_target(size, None), depth_buffer));
            self.ggx = None;
            self.integration = None;
        }
        self.source
            .as_ref()
            .expect("three-rs: the source target was just allocated")
            .0
            .clone()
    }

    fn face_target(&mut self, size: u32, depth_buffer: bool) -> Result<RenderTarget, Error> {
        if let Some(target) = self.face_targets.get(&(size, depth_buffer)) {
            return Ok(target.clone());
        }
        let target = RenderTarget::new_with_options(
            size,
            size,
            RenderTargetOptions {
                texture_type: TextureType::HalfFloat,
                samples: 0,
                depth_buffer,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
        )?;
        self.face_targets
            .insert((size, depth_buffer), target.clone());
        Ok(target)
    }

    /// `CubeCamera.update( renderer, scene )` into level `lod` of `target`:
    /// six renders with the WebGPU face table, each copied into its layer.
    #[allow(clippy::too_many_arguments)]
    fn render_faces(
        &mut self,
        renderer: &mut super::Renderer,
        scene: &mut Scene,
        target: &CubeTexture,
        lod: u32,
        near: f64,
        far: f64,
        position: Vector3,
        depth_buffer: bool,
    ) -> Result<(), Error> {
        let size = target.size().0 >> lod;
        let face_target = self.face_target(size, depth_buffer)?;

        // `CubeCamera.update()` renders with the MRT the frame had; the
        // PMREM passes are one attachment, so it is cleared for them the way
        // `fromEquirectangularTexture` clears it.
        let previous_mrt = renderer.mrt();
        let previous_target = renderer.render_target();
        renderer.set_mrt(None);
        renderer.set_render_target(Some(face_target.clone()));

        let mut camera = PerspectiveCamera::new(FOV, 1.0, near, far);
        for (layer, (up, look_at)) in FACES.iter().enumerate() {
            {
                let mut object = camera.node.borrow_mut();
                object.position.set(position.x, position.y, position.z);
                object.up = Vector3::new(up[0], up[1], up[2]);
            }
            camera.look_at(&Vector3::new(
                position.x + look_at[0],
                position.y + look_at[1],
                position.z + look_at[2],
            ));
            renderer.render(scene, &mut camera);
            renderer.copy_to_cube_layer(&face_target, target, layer as u32, lod);
        }

        renderer.set_render_target(previous_target);
        renderer.set_mrt(previous_mrt);
        Ok(())
    }

    /// `_renderCube( target, lod, material )` — the box, from inside.
    fn render_cube(
        &mut self,
        renderer: &mut super::Renderer,
        target: &CubeTexture,
        lod: u32,
        material: &MeshBasicNodeMaterial,
    ) -> Result<(), Error> {
        let mesh = Mesh::new(self.box_geometry.clone(), material.clone());
        let mut scene = Scene::new();
        scene.add(&mesh);
        self.render_faces(
            renderer,
            &mut scene,
            target,
            lod,
            CUBE_NEAR,
            CUBE_FAR,
            Vector3::ZERO,
            false,
        )
    }

    /// `fromScene( scene, sigma )` — the PMREM of a rendered scene, with
    /// three's remaining defaults (`near = 0.1`, `far = 100`, `size = 256`,
    /// the cube camera at the origin).
    ///
    /// `sigma` is the blur radius **in radians**. `webgpu_furnace_test` passes
    /// 0, which skips the blur; every `RoomEnvironment` page passes `0.04`.
    ///
    /// `scene` is `&mut` because three assigns `scene.background =
    /// renderer.getClearColor()` for the capture when the scene has none, and
    /// puts it back afterwards; the port does the same to the same field.
    pub fn from_scene(
        &mut self,
        renderer: &mut super::Renderer,
        scene: &mut Scene,
        sigma: f64,
        render_target: Option<CubeTexture>,
    ) -> Result<CubeTexture, Error> {
        self.set_size(SCENE_SIZE);

        let pmrem = render_target.unwrap_or_else(|| allocate_target(self.cube_size));
        let source = self.source_target(true);

        let auto_clear = (
            renderer.auto_clear,
            renderer.auto_clear_color,
            renderer.auto_clear_depth,
        );
        renderer.auto_clear = true;
        renderer.auto_clear_color = true;
        renderer.auto_clear_depth = true;

        let background = scene.background.clone();
        if background.is_none() {
            scene.background = Some(Background::Color(renderer.clear_color()));
        }

        let result = self.render_faces(
            renderer,
            scene,
            &source,
            0,
            SCENE_NEAR,
            SCENE_FAR,
            Vector3::ZERO,
            true,
        );

        (
            renderer.auto_clear,
            renderer.auto_clear_color,
            renderer.auto_clear_depth,
        ) = auto_clear;
        scene.background = background;
        result?;

        // `sourceTarget.texture.mipmapsAutoUpdate = false` for the capture when
        // there is a blur to come: the mips are generated from the blurred
        // level 0 instead, at the end of `_blur`.
        if sigma > 0.0 {
            self.blur(renderer, &pmrem, sigma)?;
        } else {
            renderer.generate_cube_mipmaps(&source);
        }

        self.apply_pmrem(renderer, &pmrem)?;
        Ok(pmrem)
    }

    /// `fromCubemap( cubemap )` — the PMREM of a cube texture.
    pub fn from_cubemap(
        &mut self,
        renderer: &mut super::Renderer,
        cubemap: &CubeTexture,
        render_target: Option<CubeTexture>,
    ) -> Result<CubeTexture, Error> {
        self.from_texture(renderer, &PmremSource::Cube(cubemap.clone()), render_target)
    }

    /// `fromEquirectangular( equirectangular )` — the PMREM of a 2-D
    /// longitude/latitude map, which is what an `.hdr` decodes to.
    pub fn from_equirectangular(
        &mut self,
        renderer: &mut super::Renderer,
        equirectangular: &Texture,
        render_target: Option<CubeTexture>,
    ) -> Result<CubeTexture, Error> {
        self.from_texture(
            renderer,
            &PmremSource::Equirectangular(equirectangular.clone()),
            render_target,
        )
    }

    /// `_fromTexture( texture, renderTarget )`.
    pub fn from_texture(
        &mut self,
        renderer: &mut super::Renderer,
        source: &PmremSource,
        render_target: Option<CubeTexture>,
    ) -> Result<CubeTexture, Error> {
        self.set_size(source.requested_size());
        let pmrem = render_target.unwrap_or_else(|| allocate_target(self.cube_size));
        self.texture_to_cubemap(renderer, source)?;
        self.apply_pmrem(renderer, &pmrem)?;
        Ok(pmrem)
    }

    /// `_textureToCubemap( texture )` — the source texture into level 0 of the
    /// source target, then that target's mip chain.
    fn texture_to_cubemap(
        &mut self,
        renderer: &mut super::Renderer,
        source: &PmremSource,
    ) -> Result<(), Error> {
        // Three assigns `envMap.value = texture` on one material per kind; the
        // port bakes the source into the node, so the material is built for
        // the texture it is first asked about. `PmremEnvironment` owns one
        // generator per source texture, so a generator never sees two.
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
        let target = self.source_target(false);
        self.render_cube(renderer, &target, 0, &material)?;
        renderer.generate_cube_mipmaps(&target);
        Ok(())
    }

    /// `_applyPMREM( pmremTarget )` — every level of the PMREM from the source
    /// target: `PMREM_ggx` for the sharp levels, `PMREM_integration` for the
    /// last [`INTEGRATION_LEVELS`].
    fn apply_pmrem(
        &mut self,
        renderer: &mut super::Renderer,
        pmrem: &CubeTexture,
    ) -> Result<(), Error> {
        let size = self.cube_size;
        let source = self.source_target(false);
        if self.ggx.is_none() {
            self.ggx = Some(ggx_material(&source));
            self.integration = Some(integration_material(&source));
        }
        let max_lod = pmrem.mip_level_count() - 1;

        let (ggx, integration) = (
            self.ggx.as_ref().expect("three-rs: built above"),
            self.integration.as_ref().expect("three-rs: built above"),
        );
        let (ggx_material, ggx_roughness, ggx_lod_bias) = (
            ggx.material.clone(),
            ggx.roughness.clone(),
            ggx.lod_bias.clone(),
        );
        let (integration_material, integration_roughness) =
            (integration.material.clone(), integration.roughness.clone());
        integration
            .source_lod
            .set(vec![(size as f64 / INTEGRATION_SIZE as f64).log2()]);

        for lod in 0..=max_lod {
            let roughness = lod_to_roughness(lod, max_lod);
            if uses_integration(lod, max_lod) {
                integration_roughness.set(vec![roughness]);
                self.render_cube(renderer, pmrem, lod, &integration_material)?;
            } else {
                ggx_roughness.set(vec![roughness]);
                ggx_lod_bias.set(vec![lod_bias(size, roughness)]);
                self.render_cube(renderer, pmrem, lod, &ggx_material)?;
            }
        }
        Ok(())
    }

    /// `_blur( pmremTarget, sigma )` — the spherical Gaussian, source into
    /// level 0 of the PMREM and back, after which the source's mips are
    /// generated from the blurred level.
    ///
    /// "Two passes of sigma / sqrt( 2 ) compose to a blur of sigma", and
    /// sigmas past PI are clamped to keep the shader math finite.
    fn blur(
        &mut self,
        renderer: &mut super::Renderer,
        pmrem: &CubeTexture,
        sigma: f64,
    ) -> Result<(), Error> {
        let source = self.source_target(false);
        let sigma = sigma.min(std::f64::consts::PI) / std::f64::consts::SQRT_2;

        let (forward, forward_sigma) = blur_material(&source);
        forward_sigma.set(vec![sigma]);
        self.render_cube(renderer, pmrem, 0, &forward)?;

        let (back, back_sigma) = blur_material(pmrem);
        back_sigma.set(vec![sigma]);
        self.render_cube(renderer, &source, 0, &back)?;
        renderer.generate_cube_mipmaps(&source);
        Ok(())
    }
}

/// `_getMaterial( type, uniforms, fragmentNode )` — `BackSide`, `NoBlending`,
/// no depth test or write.
fn pmrem_material(name: &'static str, fragment_node: NodeRef) -> MeshBasicNodeMaterial {
    let mut material = MeshBasicNodeMaterial::new();
    material.name = name;
    material.side = Side::Back;
    material.blending = Blending::No;
    material.depth_test = false;
    material.depth_write = false;
    material.fragment_node = Some(fragment_node);
    material
}

/// `_getCubemapMaterial()` — `envMap.sample( positionWorldDirection )`.
pub fn cubemap_material(cubemap: &CubeTexture) -> MeshBasicNodeMaterial {
    pmrem_material(
        "PMREM_cubemap",
        cube_texture(cubemap, position_world_direction()),
    )
}

/// `_getEquirectMaterial()` — four taps of the equirect map a quarter of a
/// pixel's footprint apart, averaged, so the minified map is box-filtered
/// rather than point-sampled at level 0.
pub fn equirect_material(map: &Texture) -> MeshBasicNodeMaterial {
    let direction = position_world_direction();
    let dx = to_const(None, dpdx(direction.clone()).mul(float(0.25)));
    let dy = to_const(None, dpdy(direction.clone()).mul(float(0.25)));
    // Each normalised direction is read three times by `equirectUV`; three's
    // builder caches it as a `let`, the port's would make it a var, so the
    // `let` is asked for here.
    let taps: Vec<NodeRef> = [
        direction.sub(dx.clone()).sub(dy.clone()),
        direction.add(dx.clone()).sub(dy.clone()),
        direction.sub(dx.clone()).add(dy.clone()),
        direction.add(dx.clone()).add(dy.clone()),
    ]
    .into_iter()
    .map(|d| to_const(None, d.normalize()))
    .collect();
    let tap = |d: &NodeRef| texture_level(map, equirect_uv(d.clone()), float(0.0)).xyz();
    let color = tap(&taps[0])
        .add(tap(&taps[1]))
        .add(tap(&taps[2]))
        .add(tap(&taps[3]));
    pmrem_material(
        "PMREM_equirect",
        block(
            vec![dx, dy],
            vec4_join(vec![color.mul(float(0.25)), float(1.0)]),
        ),
    )
}

/// `_getGGXMaterial()`, reading `source`.
fn ggx_material(source: &CubeTexture) -> GgxMaterial {
    let (roughness, roughness_cell) = uniform_settable(Type::F32, vec![0.0]);
    let (lod_bias, lod_bias_cell) = uniform_settable(Type::F32, vec![0.0]);
    GgxMaterial {
        material: pmrem_material(
            "PMREM_ggx",
            pmrem_utils::ggx_convolution(
                roughness,
                lod_bias,
                source,
                position_world_direction(),
                GGX_SAMPLES,
            ),
        ),
        roughness: roughness_cell,
        lod_bias: lod_bias_cell,
    }
}

/// `_getIntegrationMaterial()`, reading `source`. `sourceSize` is an `int`
/// uniform "so the loops aren't unrolled".
fn integration_material(source: &CubeTexture) -> IntegrationMaterial {
    let (roughness, roughness_cell) = uniform_settable(Type::F32, vec![0.0]);
    let (source_lod, source_lod_cell) = uniform_settable(Type::F32, vec![0.0]);
    let source_size = uniform_value(Type::I32, vec![INTEGRATION_SIZE as f64]);
    IntegrationMaterial {
        material: pmrem_material(
            "PMREM_integration",
            pmrem_utils::ggx_integration(
                roughness,
                source_lod,
                source_size,
                source,
                position_world_direction(),
            ),
        ),
        roughness: roughness_cell,
        source_lod: source_lod_cell,
    }
}

/// `_getBlurMaterial()`, reading `env_map`.
fn blur_material(env_map: &CubeTexture) -> (MeshBasicNodeMaterial, SettableValue) {
    let (sigma, sigma_cell) = uniform_settable(Type::F32, vec![0.0]);
    (
        pmrem_material(
            "PMREM_blur",
            pmrem_utils::spherical_gaussian_blur(
                BLUR_SAMPLES,
                sigma,
                position_world_direction(),
                env_map,
            ),
        ),
        sigma_cell,
    )
}

/// The three generator materials over placeholder textures, for
/// `examples/dump_wgsl.rs` to diff against three's `PMREM_*` modules.
pub fn dump_materials() -> Vec<(&'static str, MeshBasicNodeMaterial)> {
    let source = CubeTexture::pmrem_render_target(MIN_SIZE, None);
    vec![
        ("pmrem_ggx", ggx_material(&source).material),
        ("pmrem_integration", integration_material(&source).material),
        ("pmrem_blur", blur_material(&source).0),
    ]
}
