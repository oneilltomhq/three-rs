//! Ports of `three.js/src/scenes/Fog.js` and `three.js/src/scenes/FogExp2.js`,
//! and the `scene.fog` slot that holds either.
//!
//! three.js has two unrelated classes told apart by `isFog` / `isFogExp2`;
//! `NodeManager.updateFog()` branches on those flags to build the fog node.
//! The port makes the pair one enum, [`SceneFog`], so the branch is a `match`.
//!
//! Neither class carries any node state. The renderer turns the one in
//! `scene.fog` into a `fog( color, factor )` node whose parameters are
//! render-group uniforms read from the scene each frame — see
//! [`SceneFog::node`] and `docs/nodes.md` §28 — so changing a value never
//! rebuilds a program.

use crate::math::Color;
use crate::nodes::tsl::{density_fog_factor, fog, range_fog_factor, uniform, FogNode};
use crate::nodes::{Type, UniformGroup, UniformSource};

/// `Fog` — linear fog: none before `near`, all fog from `far` on.
#[derive(Clone, Debug, PartialEq)]
pub struct Fog {
    /// `fog.name`.
    pub name: String,
    /// `fog.color`, in the working colour space.
    pub color: Color,
    /// `fog.near`.
    pub near: f64,
    /// `fog.far`.
    pub far: f64,
}

impl Fog {
    /// `new Fog( color, near, far )`.
    pub fn new(color: Color, near: f64, far: f64) -> Self {
        Self {
            name: String::new(),
            color,
            near,
            far,
        }
    }

    /// `new Fog( color )` — `near = 1`, `far = 1000`.
    pub fn with_color(color: Color) -> Self {
        Self::new(color, 1.0, 1000.0)
    }

    /// `fog.isFog`.
    pub fn is_fog(&self) -> bool {
        true
    }
}

impl Default for Fog {
    /// `new Fog()` — `new Color( undefined )` is white.
    fn default() -> Self {
        Self::with_color(Color::default())
    }
}

/// `FogExp2` — exponential squared fog of one `density`.
#[derive(Clone, Debug, PartialEq)]
pub struct FogExp2 {
    /// `fog.name`.
    pub name: String,
    /// `fog.color`, in the working colour space.
    pub color: Color,
    /// `fog.density`.
    pub density: f64,
}

impl FogExp2 {
    /// `new FogExp2( color, density )`.
    pub fn new(color: Color, density: f64) -> Self {
        Self {
            name: String::new(),
            color,
            density,
        }
    }

    /// `new FogExp2( color )` — `density = 0.00025`.
    pub fn with_color(color: Color) -> Self {
        Self::new(color, 0.00025)
    }

    /// `fog.isFogExp2`.
    pub fn is_fog_exp2(&self) -> bool {
        true
    }
}

impl Default for FogExp2 {
    /// `new FogExp2()`.
    fn default() -> Self {
        Self::with_color(Color::default())
    }
}

/// What `scene.fog` may hold: a [`Fog`] or a [`FogExp2`].
#[derive(Clone, Debug, PartialEq)]
pub enum SceneFog {
    Linear(Fog),
    Exp2(FogExp2),
}

impl From<Fog> for SceneFog {
    fn from(fog: Fog) -> Self {
        SceneFog::Linear(fog)
    }
}

impl From<FogExp2> for SceneFog {
    fn from(fog: FogExp2) -> Self {
        SceneFog::Exp2(fog)
    }
}

impl SceneFog {
    /// `fog.isFog`.
    pub fn is_fog(&self) -> bool {
        matches!(self, SceneFog::Linear(_))
    }

    /// `fog.isFogExp2`.
    pub fn is_fog_exp2(&self) -> bool {
        matches!(self, SceneFog::Exp2(_))
    }

    /// `fog.color`.
    pub fn color(&self) -> Color {
        match self {
            SceneFog::Linear(fog) => fog.color,
            SceneFog::Exp2(fog) => fog.color,
        }
    }

    /// `NodeManager.updateFog( scene )`'s node for this fog:
    ///
    /// ```js
    /// if ( sceneFog.isFogExp2 ) {
    ///     const color = reference( 'color', 'color', sceneFog ).setGroup( renderGroup );
    ///     const density = reference( 'density', 'float', sceneFog ).setGroup( renderGroup );
    ///     return fog( color, densityFogFactor( density ) );
    /// } else if ( sceneFog.isFog ) {
    ///     …
    ///     return fog( color, rangeFogFactor( near, far ) );
    /// }
    /// ```
    ///
    /// three.js caches the node per fog object (`getCacheNode( 'fog',
    /// sceneFog, … )`) so that the program stays keyed on one node and a new
    /// `near` is only a uniform write. The port's uniforms name *where* their
    /// value comes from — the scene's fog, written into the render group on
    /// every `render()` — rather than holding a reference to one object, so a
    /// single node per kind serves every scene: it is built once per thread and
    /// cloned, which keeps its identity (and so the program cache key) stable
    /// across frames and across fog objects of the same kind, as three's does
    /// across frames.
    pub fn node(&self) -> FogNode {
        thread_local! {
            static LINEAR: FogNode = fog(
                fog_uniform(UniformSource::FogColor, Type::Vec3),
                range_fog_factor(
                    fog_uniform(UniformSource::FogNear, Type::F32),
                    fog_uniform(UniformSource::FogFar, Type::F32),
                ),
            );
            static EXP2: FogNode = fog(
                fog_uniform(UniformSource::FogColor, Type::Vec3),
                density_fog_factor(fog_uniform(UniformSource::FogDensity, Type::F32)),
            );
        }
        match self {
            SceneFog::Linear(_) => LINEAR.with(Clone::clone),
            SceneFog::Exp2(_) => EXP2.with(Clone::clone),
        }
    }
}

/// `reference( name, type, sceneFog ).setGroup( renderGroup )`.
fn fog_uniform(source: UniformSource, ty: Type) -> crate::nodes::NodeRef {
    uniform(source, ty, UniformGroup::Render, None)
}
