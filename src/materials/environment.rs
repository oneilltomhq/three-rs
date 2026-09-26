//! Port of `three.js/src/nodes/lighting/EnvironmentNode.js`.
//!
//! Image-based lighting: the PMREM sampled twice per fragment — once along the
//! reflection vector at the surface's roughness, which is `radiance`, and once
//! along the world normal at roughness 1, which is `iblIrradiance`. Everything
//! downstream of those two is already in `PhysicalLightingModel`; without an
//! environment they stay zero and the same arithmetic runs on zeros.

use crate::nodes::node::Type;
use crate::nodes::pmrem_utils::roughness_to_mip;
use crate::nodes::tsl::{
    bent_normal_view, camera_world_matrix, clearcoat_normal_view, clearcoat_roughness,
    cube_texture_level, float, material_env_intensity, material_env_rotation, mix, normal_view,
    normal_world, position_view_direction, reflect, roughness, transform_direction, vec4_join,
};
use crate::nodes::NodeRef;
use crate::textures::CubeTexture;

/// What a material needs to read a generated PMREM: the cube and its `maxLod`
/// uniform.
///
/// This is the half of `PMREMNode` that has to travel with a material rather
/// than with the renderer. [`PmremEnvironment::handle`] hands one out; it is a
/// cheap clone of the cube the environment renders into, so a handle taken
/// before the PMREM exists still works afterwards.
///
/// [`PmremEnvironment::handle`]: crate::nodes::pmrem_node::PmremEnvironment::handle
#[derive(Clone, Debug)]
pub struct PmremHandle {
    pub texture: CubeTexture,
    pub max_lod: NodeRef,
}

/// The handle is part of [`SetupContext`]'s derived hash — the render object's
/// dynamic cache key — so it needs one, and neither `CubeTexture` nor the
/// `maxLod` uniform derives `Hash`. What the *program* depends on is only that
/// there **is** an environment and which texture it reads; `maxLod` is a
/// uniform and changes no code. So the texture's id is the whole key, which is
/// also what makes two materials sharing one PMREM share one program.
///
/// [`SetupContext`]: crate::materials::node_material::SetupContext
impl std::hash::Hash for PmremHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture.id().hash(state);
    }
}

impl PmremHandle {
    /// `PMREMNode.setup()` — `this._texture.sample( materialEnvRotation.mul(
    /// uvNode ) ).level( roughnessToMip( levelNode, this._maxLod ) ).rgb`.
    ///
    /// `materialEnvRotation` is a `mat4`, so a `vec3` direction is widened with
    /// `w = 1` first; the background's `backgroundRotation * normal` is a
    /// `vec4` already and goes in as it is, which is three's dump of both. The
    /// cube read's `x` negation is `CubeTextureNode.setupUV()`'s. There is no
    /// Y flip any more: the PMREM is rendered with the same face cameras every
    /// other cube render target is.
    pub fn sample(&self, uv: NodeRef, level: NodeRef) -> NodeRef {
        let uv = if uv.ty() == Type::Vec4 {
            uv
        } else {
            vec4_join(vec![uv, float(1.0)])
        };
        let direction = material_env_rotation().mul(uv);
        cube_texture_level(
            &self.texture,
            direction,
            roughness_to_mip(level, self.max_lod.clone()),
        )
        .xyz()
    }
}

/// `createRadianceContext( roughness, normalView )`'s `getUV`.
///
/// "Mixing the reflection with the normal is more accurate and keeps rough
/// objects from gathering light from behind their tangent plane" — three's
/// comment for the `pow4( roughness )` mix.
/// `createRadianceContext( roughness, normalView )`'s reflect vector over an
/// arbitrary normal — the anisotropic bent normal, or the clearcoat one.
fn reflect_vector_of(normal: NodeRef) -> NodeRef {
    reflect_vector_of_roughness(normal, roughness())
}

fn reflect_vector_of_roughness(normal: NodeRef, roughness_value: NodeRef) -> NodeRef {
    let reflect_vec = reflect(position_view_direction().negate(), normal.clone());
    let reflect_vec = mix(reflect_vec, normal, pow4(roughness_value)).normalize();
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
pub fn setup(env: &PmremHandle, anisotropy: bool, clearcoat: bool, out: &mut Vec<NodeRef>) {
    let radiance_prop = crate::nodes::tsl::radiance();
    let ibl_prop = crate::nodes::tsl::ibl_irradiance();

    out.push(radiance_prop.assign(crate::nodes::tsl::vec3(0.0, 0.0, 0.0)));
    // `const radianceNormalView = useAnisotropy ? bentNormalView : normalView`.
    let radiance_normal = if anisotropy {
        bent_normal_view()
    } else {
        normal_view()
    };
    let radiance = env
        .sample(reflect_vector_of(radiance_normal), roughness())
        .mul(material_env_intensity());
    out.push(radiance_prop.assign(radiance_prop.add(radiance)));

    out.push(ibl_prop.assign(crate::nodes::tsl::vec3(0.0, 0.0, 0.0)));
    // `.mul( Math.PI )`: the PMREM holds radiance, and the lighting
    // model wants irradiance.
    let irradiance = env
        .sample(normal_world(), float(1.0))
        .mul(float(std::f64::consts::PI))
        .mul(material_env_intensity());
    out.push(ibl_prop.assign(ibl_prop.add(irradiance)));

    // `EnvironmentNode`'s third `addAssign`, present only when the lighting
    // model has a clearcoat lobe: the same radiance context over the coat's
    // normal and roughness.
    if clearcoat {
        let clearcoat_prop = crate::nodes::tsl::clearcoat_radiance();
        let radiance = env
            .sample(
                reflect_vector_of_roughness(clearcoat_normal_view(), clearcoat_roughness()),
                clearcoat_roughness(),
            )
            .mul(material_env_intensity());
        out.push(clearcoat_prop.assign(clearcoat_prop.add(radiance)));
    }
}
