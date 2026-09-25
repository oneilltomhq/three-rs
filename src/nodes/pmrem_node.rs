//! Port of `three.js/src/nodes/pmrem/PMREMNode.js`.
//!
//! `pmremTexture( value, uv, level )` is the read side of a PMREM: since
//! three.js 2f80402 a cube sample at an explicit level,
//! `cubeTexture( pmrem ).sample( materialEnvRotation * uv ).level(
//! roughnessToMip( level, maxLod ) ).rgb`.
//!
//! # Where `updateBefore` went
//!
//! In three a `PMREMNode` carries `updateBeforeType = NodeUpdateType.RENDER`,
//! so the renderer calls `updateBefore()` once per render — before the scene
//! pass records a draw — and the node lazily builds the PMREM there, with the
//! generated texture and `maxLod` reaching the shader through the uniforms the
//! node owns.
//!
//! The port keeps the *shape* and moves the *trigger*: the state lives on a
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

use crate::materials::environment::PmremHandle;
use crate::nodes::node::Type;
use crate::nodes::tsl::uniform_value;
use crate::nodes::NodeRef;
use crate::renderer::pmrem::{allocate_target, cube_size_for, max_lod_for, PmremGenerator, PmremSource, SCENE_SIZE};
use crate::renderer::Renderer;
use crate::textures::{CubeTexture, Texture};

/// One environment: a source texture (a cube or an equirect map), the PMREM
/// generated from it, and the `maxLod` uniform the shader reads it with.
///
/// This is `PMREMNode`'s private state (`_texture`, `_maxLod`, `_generator`,
/// `_pmrem`) with a name, because in Rust it has to outlive the nodes that
/// read it.
///
/// The PMREM cube is allocated up front: its size follows from the source's
/// alone (`_setSize`), so the texture a handle carries is the one the
/// generator later renders into, and nothing has to be repointed.
pub struct PmremEnvironment {
    /// The texture this environment is generated from, or `None` when it came
    /// from a scene — `fromScene` has no source texture to re-read, and it is
    /// generated eagerly, so [`update`](Self::update) has nothing left to do.
    source: Option<PmremSource>,
    generator: PmremGenerator,
    /// `this._texture.value` — the PMREM cube render target.
    texture: CubeTexture,
    /// `this._maxLod` — `mipmaps.length - 1`.
    max_lod: NodeRef,
    built: bool,
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
    /// reached lazily through `NodeManager.updateEnvironment`, and the texture
    /// it returns is `isPMREMTexture`, which `PMREMNode` reads as it is.
    ///
    /// `sigma` is `fromScene`'s second argument, the pre-blur radius in
    /// radians: 0 for `webgpu_furnace_test`, `0.04` for every
    /// `RoomEnvironment` page.
    pub fn from_scene(
        renderer: &mut Renderer,
        scene: &mut crate::objects::Scene,
        sigma: f64,
    ) -> Result<Self, crate::error::Error> {
        let mut environment = Self::with_size(None, cube_size_for(SCENE_SIZE));
        let texture = environment.texture.clone();
        environment
            .generator
            .from_scene(renderer, scene, sigma, Some(texture))?;
        environment.built = true;
        Ok(environment)
    }

    fn from_source(source: PmremSource) -> Self {
        let size = cube_size_for(source.requested_size());
        Self::with_size(Some(source), size)
    }

    fn with_size(source: Option<PmremSource>, size: u32) -> Self {
        let texture = allocate_target(size);
        Self {
            source,
            generator: PmremGenerator::new(),
            texture,
            max_lod: uniform_value(Type::F32, vec![max_lod_for(size) as f64]),
            built: false,
        }
    }

    /// `PMREMNode.updateBefore( frame )` — build the PMREM if it is not there
    /// yet.
    ///
    /// Idempotent: three re-runs only when `texture.pmremVersion` moves, and
    /// this port's environments are immutable once loaded, so the second call
    /// is a no-op.
    pub fn update(&mut self, renderer: &mut Renderer) -> Result<(), crate::error::Error> {
        if self.built {
            return Ok(());
        }
        let Some(source) = self.source.clone() else {
            return Ok(());
        };
        self.generator
            .from_texture(renderer, &source, Some(self.texture.clone()))?;
        self.built = true;
        Ok(())
    }

    /// What a material holds to read this environment:
    /// `EnvironmentNode( pmremTexture( material.envMap ) )`'s inner node, minus
    /// the direction it will be sampled along.
    pub fn handle(&self) -> PmremHandle {
        PmremHandle {
            texture: self.texture.clone(),
            max_lod: self.max_lod.clone(),
        }
    }

    /// Whether [`update`](Self::update) has run.
    pub fn is_ready(&self) -> bool {
        self.built
    }

    /// The PMREM cube, for a test that reads it back.
    pub fn texture(&self) -> &CubeTexture {
        &self.texture
    }

    /// `PMREMNode.setup()`.
    pub fn sample(&self, uv: NodeRef, level: NodeRef) -> NodeRef {
        self.handle().sample(uv, level)
    }
}

/// `pmremTexture( value, uvNode, levelNode )`.
pub fn pmrem_texture(env: &PmremEnvironment, uv: NodeRef, level: NodeRef) -> NodeRef {
    env.sample(uv, level)
}
