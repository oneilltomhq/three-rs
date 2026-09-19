//! Port of `three.js/src/nodes/pmrem/PMREMNode.js`.
//!
//! `pmremTexture( value, uv, level )` is the read side of a PMREM: it samples
//! the cubeUV atlas [`PmremGenerator`] built,
//! through [`texture_cube_uv`].
//!
//! # Where `updateBefore` went
//!
//! In three a `PMREMNode` carries `updateBeforeType = NodeUpdateType.RENDER`,
//! so the renderer calls `updateBefore()` once per render — before the scene
//! pass records a draw — and the node lazily builds the PMREM there, with the
//! generated texture and the three cubeUV numbers reaching the shader through
//! uniform cells the node owns.
//!
//! The port keeps the *shape* and moves the *trigger*: the cells live on a
//! [`PmremEnvironment`], which the application updates through
//! [`PmremEnvironment::update`] before it renders. The reason is that a node
//! in this port is an immutable `Rc` graph with no back-reference to the
//! renderer, so a node cannot start a nested render of its own; and the
//! renderer takes `&mut self`, so it cannot be handed to a node mid-build.
//!
//! `update` is idempotent and cheap after the first call, so "call it before
//! you render" is the whole rule, and it keeps three's guarantee that the
//! PMREM chain completes and submits before the scene pass. Listed in
//! `docs/nodes.md` §8.

use std::cell::RefCell;
use std::rc::Rc;

use crate::materials::environment::PmremHandle;
use crate::nodes::node::{SettableValue, Type};
use crate::nodes::pmrem_utils::{texture_cube_uv, CubeUvSize};
use crate::nodes::tsl::{float, material_env_rotation, uniform_settable, vec3_join, vec4_join};
use crate::nodes::NodeRef;
use crate::renderer::pmrem::{PmremGenerator, PmremSource};
use crate::renderer::{RenderTarget, Renderer};
use crate::textures::{CubeTexture, Texture};

/// `_generateCubeUVSize( imageHeight )` — the three numbers a cubeUV read needs,
/// derived from the atlas' height alone.
///
/// `maxMip = log2( h ) - 2` because the atlas is four faces tall and each face
/// is `2 ^ maxMip`; `texelWidth` carries the same `7 * 16` floor
/// `_allocateTarget` uses, so a small environment's extra-LOD columns are
/// addressed correctly.
pub fn generate_cube_uv_size(image_height: u32) -> (f64, f64, f64) {
    let max_mip = (image_height as f64).log2() - 2.0;
    let texel_height = 1.0 / image_height as f64;
    let texel_width = 1.0 / (3.0 * (2f64).powf(max_mip).max(7.0 * 16.0));
    (texel_width, texel_height, max_mip)
}

struct Cells {
    texel_width: SettableValue,
    texel_height: SettableValue,
    max_mip: SettableValue,
}

/// One environment: a source texture (a cube or an equirect map), the PMREM
/// generated from it, and the uniform cells the shader reads it through.
///
/// This is `PMREMNode`'s private state (`_texture`, `_width`, `_height`,
/// `_maxMip`, `_generator`, `_pmrem`) with a name, because in Rust it has to
/// outlive the nodes that read it.
pub struct PmremEnvironment {
    /// The texture this environment is generated from, or `None` when it came
    /// from a scene — `fromScene` has no source texture to re-read, and it is
    /// generated eagerly, so [`update`](Self::update) has nothing left to do.
    source: Option<PmremSource>,
    generator: PmremGenerator,
    /// The generated atlas, kept so a second `update` can render into it again
    /// rather than allocate — three's `cache.get( texture )` reuse.
    target: Option<RenderTarget>,
    /// `this._texture.value` — a borrowed handle pointed at the atlas, for the
    /// same reason the GGX material's `envMap` is one.
    texture: Texture,
    size: CubeUvSize,
    cells: Rc<RefCell<Cells>>,
}

impl PmremEnvironment {
    /// `pmremTexture( cubeTexture )` — an environment that has not been built
    /// yet. Nothing is rendered until [`update`](Self::update).
    pub fn new(source: &CubeTexture) -> Self {
        Self::from_source(PmremSource::Cube(source.clone()))
    }

    /// The same, from an equirectangular 2-D map — `scene.environment =
    /// hdrTexture`, which three funnels into
    /// `PMREMGenerator.fromEquirectangular`.
    pub fn from_equirectangular(source: &Texture) -> Self {
        Self::from_source(PmremSource::Equirectangular(source.clone()))
    }

    /// `radianceMap = pmremGenerator.fromScene( envScene ).texture` — the
    /// PMREM of a rendered scene.
    ///
    /// Unlike the two texture entry points this one builds **now**, because
    /// that is where three builds it: `fromScene` is called by the page, not
    /// reached lazily through `NodeManager.updateEnvironment`. So there is no
    /// `updateBefore` to move and [`update`](Self::update) is a no-op
    /// afterwards.
    ///
    /// `sigma` is `fromScene`'s second argument, the pre-blur radius in
    /// radians: 0 for `webgpu_furnace_test`, `0.04` for every
    /// `RoomEnvironment` page.
    pub fn from_scene(
        renderer: &mut Renderer,
        scene: &mut crate::objects::Scene,
        sigma: f64,
    ) -> Result<Self, crate::error::Error> {
        let mut environment = Self::from_optional_source(None);
        let target = environment
            .generator
            .from_scene(renderer, scene, sigma, None)?;
        environment.adopt(target);
        Ok(environment)
    }

    fn from_source(source: PmremSource) -> Self {
        Self::from_optional_source(Some(source))
    }

    fn from_optional_source(source: Option<PmremSource>) -> Self {
        let (texel_width, texel_width_cell) = uniform_settable(Type::F32, vec![0.0]);
        let (texel_height, texel_height_cell) = uniform_settable(Type::F32, vec![0.0]);
        let (max_mip, max_mip_cell) = uniform_settable(Type::F32, vec![0.0]);

        Self {
            source,
            generator: PmremGenerator::new(),
            target: None,
            // Size and format are replaced when the atlas is built; the handle
            // is borrowed (`own_gpu = false`) so neither is ever uploaded from.
            texture: Texture::render_target(1, 1, wgpu::TextureFormat::Rgba16Float),
            size: CubeUvSize {
                texel_width,
                texel_height,
                max_mip,
            },
            cells: Rc::new(RefCell::new(Cells {
                texel_width: texel_width_cell,
                texel_height: texel_height_cell,
                max_mip: max_mip_cell,
            })),
        }
    }

    /// `PMREMNode.updateBefore( frame )` — build the PMREM if it is not there
    /// yet, then `updateFromTexture()` the three cubeUV uniforms.
    ///
    /// Idempotent: three re-runs only when `texture.pmremVersion` moves, and
    /// this port's environments are immutable once loaded, so the second call
    /// is a no-op.
    pub fn update(&mut self, renderer: &mut Renderer) -> Result<(), crate::error::Error> {
        if self.target.is_some() {
            return Ok(());
        }
        let Some(source) = self.source.clone() else {
            // `fromScene` built this one eagerly; there is no source texture
            // to regenerate from.
            return Ok(());
        };

        let target = self
            .generator
            .from_texture(renderer, &source, self.target.take())?;
        self.adopt(target);
        Ok(())
    }

    /// `updateFromTexture( pmrem )` — point the borrowed handle at the atlas
    /// and write the three cubeUV uniforms its height implies.
    fn adopt(&mut self, target: RenderTarget) {
        let (_, height) = target.size();
        let (texel_width, texel_height, max_mip) = generate_cube_uv_size(height);
        self.texture
            .set_gpu(target.texture().with_gpu(|gpu| gpu.clone()));
        let cells = self.cells.borrow();
        cells.texel_width.set(vec![texel_width]);
        cells.texel_height.set(vec![texel_height]);
        cells.max_mip.set(vec![max_mip]);
        drop(cells);

        self.target = Some(target);
    }

    /// What a material holds to read this environment:
    /// `EnvironmentNode( pmremTexture( material.envMap ) )`'s inner node, minus
    /// the direction it will be sampled along.
    pub fn handle(&self) -> PmremHandle {
        PmremHandle {
            texture: self.texture.clone(),
            size: self.size.clone(),
        }
    }

    /// Whether [`update`](Self::update) has run.
    pub fn is_ready(&self) -> bool {
        self.target.is_some()
    }

    /// The generated atlas, for a test that reads it back.
    pub fn target(&self) -> Option<&RenderTarget> {
        self.target.as_ref()
    }

    /// `PMREMNode.setup()` — `textureCubeUV( this._texture, uv, level, … )`
    /// with the Y flip and the environment rotation on the sample direction.
    ///
    /// The flip is three's comment verbatim: "PMREMGenerator renders into a
    /// render target with inverted Y, so its output needs the Y flip on
    /// sampling." A PMREM loaded from a file would not get it; this port only
    /// ever has generated ones, so the branch is not carried.
    pub fn sample(&self, uv: NodeRef, level: NodeRef) -> NodeRef {
        let uv = material_env_rotation().mul(vec4_join(vec![
            vec3_join(vec![uv.x(), uv.y().negate(), uv.z()]),
            float(1.0),
        ]));
        texture_cube_uv(&self.texture, uv, level, &self.size)
    }
}

/// `pmremTexture( value, uvNode, levelNode )`.
pub fn pmrem_texture(env: &PmremEnvironment, uv: NodeRef, level: NodeRef) -> NodeRef {
    env.sample(uv, level)
}
