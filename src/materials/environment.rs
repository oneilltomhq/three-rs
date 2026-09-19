//! Port of `three.js/src/nodes/lighting/EnvironmentNode.js`.
//!
//! Image-based lighting: the PMREM sampled twice per fragment — once along the
//! reflection vector at the surface's roughness, which is `radiance`, and once
//! along the world normal at roughness 1, which is `iblIrradiance`. Everything
//! downstream of those two is already in `PhysicalLightingModel`; without an
//! environment they stay zero and the same arithmetic runs on zeros.

use crate::nodes::pmrem_utils::{texture_cube_uv, CubeUvSize};
use crate::nodes::tsl::{
    camera_world_matrix, float, material_env_intensity, material_env_rotation, mix, normal_view,
    normal_world, position_view_direction, reflect, roughness, transform_direction, vec3_join,
    vec4_join,
};
use crate::nodes::NodeRef;
use crate::textures::Texture;

/// What a material needs to read a generated PMREM: the atlas and its three
/// shape uniforms.
///
/// This is the half of `PMREMNode` that has to travel with a material rather
/// than with the renderer. [`PmremEnvironment::handle`] hands one out; it is a
/// cheap clone, and the `Texture` inside is the same borrowed handle the
/// environment repoints when it builds, so a handle taken before the PMREM
/// exists still works afterwards.
///
/// [`PmremEnvironment::handle`]: crate::nodes::pmrem_node::PmremEnvironment::handle
#[derive(Clone, Debug)]
pub struct PmremHandle {
    pub texture: Texture,
    pub size: CubeUvSize,
}

/// The handle is part of [`SetupContext`]'s derived hash — the render object's
/// dynamic cache key — so it needs one, and neither `Texture` nor the three
/// `NodeRef` uniforms in [`CubeUvSize`] derives `Hash`. What the *program*
/// depends on is only that there **is** an environment and which texture it
/// reads; the three cubeUV numbers are uniforms and change no code. So the
/// texture's id is the whole key, which is also what makes two materials
/// sharing one generated atlas share one program.
///
/// [`SetupContext`]: crate::materials::node_material::SetupContext
impl std::hash::Hash for PmremHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture.id().hash(state);
    }
}

impl PmremHandle {
    /// `PMREMNode.setup()` — the rotated, Y-flipped sample.
    pub fn sample(&self, uv: NodeRef, level: NodeRef) -> NodeRef {
        let uv = material_env_rotation().mul(vec4_join(vec![
            vec3_join(vec![uv.x(), uv.y().negate(), uv.z()]),
            float(1.0),
        ]));
        texture_cube_uv(&self.texture, uv, level, &self.size)
    }
}

/// `createRadianceContext( roughness, normalView )`'s `getUV`.
///
/// "Mixing the reflection with the normal is more accurate and keeps rough
/// objects from gathering light from behind their tangent plane" — three's
/// comment for the `pow4( roughness )` mix.
fn reflect_vector() -> NodeRef {
    let reflect_vec = reflect(position_view_direction().negate(), normal_view());
    let reflect_vec = mix(reflect_vec, normal_view(), pow4(roughness())).normalize();
    transform_direction(camera_world_matrix(), reflect_vec)
}

/// `pow4( x )` — `x.pow(4)` is three's `MathNode.POW` with an integer exponent,
/// which it unrolls to three multiplies.
fn pow4(x: NodeRef) -> NodeRef {
    x.clone().mul(x.clone()).mul(x.clone()).mul(x)
}

/// `EnvironmentNode.setup()`'s two `addAssign`s, in three's order: `radiance`
/// first, then `iblIrradiance`.
///
/// The two zero assignments are three's `LightingContextNode` properties being
/// declared at their first use, which is here and not in `indirectSpecular` —
/// so this function owns them, and `PhysicalLightingModel::indirect_specular`
/// only writes them when there is no environment.
pub fn setup(env: &PmremHandle, out: &mut Vec<NodeRef>) {
    let radiance_prop = crate::nodes::tsl::radiance();
    let ibl_prop = crate::nodes::tsl::ibl_irradiance();

    out.push(radiance_prop.assign(crate::nodes::tsl::vec3(0.0, 0.0, 0.0)));
    let radiance = env
        .sample(reflect_vector(), roughness())
        .mul(material_env_intensity());
    out.push(radiance_prop.assign(radiance_prop.add(radiance)));

    out.push(ibl_prop.assign(crate::nodes::tsl::vec3(0.0, 0.0, 0.0)));
    // `.mul( Math.PI )`: `textureCubeUV` returns radiance, and the lighting
    // model wants irradiance.
    let irradiance = env
        .sample(normal_world(), float(1.0))
        .mul(float(std::f64::consts::PI))
        .mul(material_env_intensity());
    out.push(ibl_prop.assign(ibl_prop.add(irradiance)));
}
