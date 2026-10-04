//! Which fields of [`MeshBasicNodeMaterial`] each [`MaterialKind`] reads —
//! one declaration, [`MaterialKind::table`], that three things consult:
//!
//! - [`setup()`](super::setup) picks the kind's fragment flow from it
//!   ([`KindTable::flow`]) and gates the fields only some kinds read on it
//!   (the Basic environment map, the Physical extensions);
//! - [`MeshBasicNodeMaterial::unsupported_fields`] is its
//!   [`loud`](KindTable::loud) list filtered by what is set, and
//!   [`warn_unsupported`] adds its [`draw`](KindTable::draw) rules, the ones
//!   only a draw can judge, under one warning registry;
//! - a no-GPU test builds every kind's program with each field set alone
//!   (over the fields it needs) and checks that the field shows in the
//!   program — binds its uniform or its own texture, or changes the WGSL —
//!   exactly when [`reads`](KindTable::reads) lists it, then sets every field
//!   at once and checks nothing undeclared shows.
//!
//! The table is an exhaustive `match`, so a new kind does not compile until it
//! says which flow it takes and what it reads. Before it, the flow was an
//! `if`/`else if` chain whose last arm caught any kind it did not name, and
//! the warnings restated the rules separately: Sprite, Points and Line2 were
//! warned that they ignore `envMap` and then sampled it (#253).
//!
//! What is not here: fields every kind treats alike — the flow-replacing
//! nodes (`fragment_node`, `vertex_node`, `output_node`, `position_node`,
//! `mask_node`, `depth_node`, `mrt_node`, the context and lighting
//! overrides), `fog`, and the pipeline state (`side`, blending, depth,
//! stencil, `wireframe`). A field three's own class for a kind lacks is
//! neither read nor loud: three ignores it too (`docs/api.md` decision 7).

use super::MaterialKind;
use super::MeshBasicNodeMaterial;
use crate::lights::LightKind;
use crate::materials::node_material::SetupContext;

/// A field of [`MeshBasicNodeMaterial`] that some kinds read and others do
/// not. Each variant is the field of the same name in `snake_case`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Field {
    Color,
    Opacity,
    Reflectivity,
    /// Read by no kind's flow, only by `refract_view`; the test checks no
    /// kind's program shows it.
    #[cfg_attr(not(test), allow(dead_code))]
    RefractionRatio,
    EnvMap,
    PmremEnv,
    ColorNode,
    OpacityNode,
    AlphaTestNode,
    AlphaTest,
    AlphaHash,
    AlphaMap,
    EmissiveNode,
    Specular,
    Shininess,
    Emissive,
    EmissiveIntensity,
    FlatShading,
    Metalness,
    Roughness,
    Map,
    VertexColors,
    RoughnessMap,
    MetalnessMap,
    EmissiveMap,
    GradientMap,
    AoMap,
    AoMapIntensity,
    LightMap,
    /// Read only with `light_map`, which only an accessor reads.
    #[cfg_attr(not(test), allow(dead_code))]
    LightMapIntensity,
    SpecularMap,
    BumpMap,
    BumpScale,
    Clearcoat,
    ClearcoatRoughness,
    ClearcoatMap,
    ClearcoatRoughnessMap,
    Sheen,
    SheenColor,
    SheenRoughness,
    DiffuseRoughness,
    Ior,
    SpecularIntensity,
    SpecularColor,
    SpecularNode,
    NormalMap,
    NormalScale,
    SpecularColorMap,
    Anisotropy,
    AnisotropyRotation,
    AnisotropyMap,
    Iridescence,
    IridescenceIor,
    IridescenceThicknessRange,
    IridescenceThicknessMap,
    ClearcoatNormalMap,
    ClearcoatNormalScale,
    Transmission,
    TransmissionMap,
    Thickness,
    ThicknessMap,
    AttenuationDistance,
    AttenuationColor,
    NormalNode,
    ScaleNode,
    RotationNode,
    SizeNode,
    /// Read by no kind's flow: a Points `size` is the
    /// [`SpriteSize`](DrawRule::SpriteSize) rule's concern.
    #[cfg_attr(not(test), allow(dead_code))]
    Size,
    Rotation,
    Linewidth,
    SizeAttenuation,
    WorldUnits,
    BackdropNode,
    BackdropAlphaNode,
    MetalnessNode,
    RoughnessNode,
}

impl Field {
    /// Every field, in declaration order: what the test walks.
    #[cfg(test)]
    pub(crate) const ALL: [Field; 76] = {
        use Field::*;
        [
            Color,
            Opacity,
            Reflectivity,
            RefractionRatio,
            EnvMap,
            PmremEnv,
            ColorNode,
            OpacityNode,
            AlphaTestNode,
            AlphaTest,
            AlphaHash,
            AlphaMap,
            EmissiveNode,
            Specular,
            Shininess,
            Emissive,
            EmissiveIntensity,
            FlatShading,
            Metalness,
            Roughness,
            Map,
            VertexColors,
            RoughnessMap,
            MetalnessMap,
            EmissiveMap,
            GradientMap,
            AoMap,
            AoMapIntensity,
            LightMap,
            LightMapIntensity,
            SpecularMap,
            BumpMap,
            BumpScale,
            Clearcoat,
            ClearcoatRoughness,
            ClearcoatMap,
            ClearcoatRoughnessMap,
            Sheen,
            SheenColor,
            SheenRoughness,
            DiffuseRoughness,
            Ior,
            SpecularIntensity,
            SpecularColor,
            SpecularNode,
            NormalMap,
            NormalScale,
            SpecularColorMap,
            Anisotropy,
            AnisotropyRotation,
            AnisotropyMap,
            Iridescence,
            IridescenceIor,
            IridescenceThicknessRange,
            IridescenceThicknessMap,
            ClearcoatNormalMap,
            ClearcoatNormalScale,
            Transmission,
            TransmissionMap,
            Thickness,
            ThicknessMap,
            AttenuationDistance,
            AttenuationColor,
            NormalNode,
            ScaleNode,
            RotationNode,
            SizeNode,
            Size,
            Rotation,
            Linewidth,
            SizeAttenuation,
            WorldUnits,
            BackdropNode,
            BackdropAlphaNode,
            MetalnessNode,
            RoughnessNode,
        ]
    };

    /// three.js' name for the field, as the warnings print it.
    pub(crate) fn name(self) -> &'static str {
        use Field::*;
        match self {
            Color => "color",
            Opacity => "opacity",
            Reflectivity => "reflectivity",
            RefractionRatio => "refractionRatio",
            EnvMap => "envMap",
            PmremEnv => "envMap (PMREM)",
            ColorNode => "colorNode",
            OpacityNode => "opacityNode",
            AlphaTestNode => "alphaTestNode",
            AlphaTest => "alphaTest",
            AlphaHash => "alphaHash",
            AlphaMap => "alphaMap",
            EmissiveNode => "emissiveNode",
            Specular => "specular",
            Shininess => "shininess",
            Emissive => "emissive",
            EmissiveIntensity => "emissiveIntensity",
            FlatShading => "flatShading",
            Metalness => "metalness",
            Roughness => "roughness",
            Map => "map",
            VertexColors => "vertexColors",
            RoughnessMap => "roughnessMap",
            MetalnessMap => "metalnessMap",
            EmissiveMap => "emissiveMap",
            GradientMap => "gradientMap",
            AoMap => "aoMap",
            AoMapIntensity => "aoMapIntensity",
            LightMap => "lightMap",
            LightMapIntensity => "lightMapIntensity",
            SpecularMap => "specularMap",
            BumpMap => "bumpMap",
            BumpScale => "bumpScale",
            Clearcoat => "clearcoat",
            ClearcoatRoughness => "clearcoatRoughness",
            ClearcoatMap => "clearcoatMap",
            ClearcoatRoughnessMap => "clearcoatRoughnessMap",
            Sheen => "sheen",
            SheenColor => "sheenColor",
            SheenRoughness => "sheenRoughness",
            DiffuseRoughness => "diffuseRoughness",
            Ior => "ior",
            SpecularIntensity => "specularIntensity",
            SpecularColor => "specularColor",
            SpecularNode => "specularNode",
            NormalMap => "normalMap",
            NormalScale => "normalScale",
            SpecularColorMap => "specularColorMap",
            Anisotropy => "anisotropy",
            AnisotropyRotation => "anisotropyRotation",
            AnisotropyMap => "anisotropyMap",
            Iridescence => "iridescence",
            IridescenceIor => "iridescenceIor",
            IridescenceThicknessRange => "iridescenceThicknessRange",
            IridescenceThicknessMap => "iridescenceThicknessMap",
            ClearcoatNormalMap => "clearcoatNormalMap",
            ClearcoatNormalScale => "clearcoatNormalScale",
            Transmission => "transmission",
            TransmissionMap => "transmissionMap",
            Thickness => "thickness",
            ThicknessMap => "thicknessMap",
            AttenuationDistance => "attenuationDistance",
            AttenuationColor => "attenuationColor",
            NormalNode => "normalNode",
            ScaleNode => "scaleNode",
            RotationNode => "rotationNode",
            SizeNode => "sizeNode",
            Size => "size",
            Rotation => "rotation",
            Linewidth => "linewidth",
            SizeAttenuation => "sizeAttenuation",
            WorldUnits => "worldUnits",
            BackdropNode => "backdropNode",
            BackdropAlphaNode => "backdropAlphaNode",
            MetalnessNode => "metalnessNode",
            RoughnessNode => "roughnessNode",
        }
    }

    /// Whether `material` sets the field. Only an optional field — a map or
    /// a node — can be unset, so only one of those can be [`loud`](KindTable::loud)
    /// or [accessor-only](ACCESSOR_ONLY); a value field always reads as set.
    pub(crate) fn is_set(self, material: &MeshBasicNodeMaterial) -> bool {
        use Field::*;
        let m = material;
        match self {
            EnvMap => m.env_map.is_some(),
            PmremEnv => m.pmrem_env.is_some(),
            ColorNode => m.color_node.is_some(),
            OpacityNode => m.opacity_node.is_some(),
            AlphaTestNode => m.alpha_test_node.is_some(),
            AlphaMap => m.alpha_map.is_some(),
            EmissiveNode => m.emissive_node.is_some(),
            Map => m.map.is_some(),
            RoughnessMap => m.roughness_map.is_some(),
            MetalnessMap => m.metalness_map.is_some(),
            EmissiveMap => m.emissive_map.is_some(),
            GradientMap => m.gradient_map.is_some(),
            AoMap => m.ao_map.is_some(),
            LightMap => m.light_map.is_some(),
            SpecularMap => m.specular_map.is_some(),
            BumpMap => m.bump_map.is_some(),
            ClearcoatMap => m.clearcoat_map.is_some(),
            ClearcoatRoughnessMap => m.clearcoat_roughness_map.is_some(),
            SpecularNode => m.specular_node.is_some(),
            NormalMap => m.normal_map.is_some(),
            SpecularColorMap => m.specular_color_map.is_some(),
            AnisotropyMap => m.anisotropy_map.is_some(),
            IridescenceThicknessMap => m.iridescence_thickness_map.is_some(),
            ClearcoatNormalMap => m.clearcoat_normal_map.is_some(),
            TransmissionMap => m.transmission_map.is_some(),
            ThicknessMap => m.thickness_map.is_some(),
            NormalNode => m.normal_node.is_some(),
            ScaleNode => m.scale_node.is_some(),
            RotationNode => m.rotation_node.is_some(),
            SizeNode => m.size_node.is_some(),
            BackdropNode => m.backdrop_node.is_some(),
            BackdropAlphaNode => m.backdrop_alpha_node.is_some(),
            MetalnessNode => m.metalness_node.is_some(),
            RoughnessNode => m.roughness_node.is_some(),
            Color
            | Opacity
            | Reflectivity
            | RefractionRatio
            | AlphaTest
            | AlphaHash
            | Specular
            | Shininess
            | Emissive
            | EmissiveIntensity
            | FlatShading
            | Metalness
            | Roughness
            | VertexColors
            | AoMapIntensity
            | LightMapIntensity
            | BumpScale
            | Clearcoat
            | ClearcoatRoughness
            | Sheen
            | SheenColor
            | SheenRoughness
            | DiffuseRoughness
            | Ior
            | SpecularIntensity
            | SpecularColor
            | NormalScale
            | Anisotropy
            | AnisotropyRotation
            | Iridescence
            | IridescenceIor
            | IridescenceThicknessRange
            | ClearcoatNormalScale
            | Transmission
            | Thickness
            | AttenuationDistance
            | AttenuationColor
            | Size
            | Rotation
            | Linewidth
            | SizeAttenuation
            | WorldUnits => true,
        }
    }
}

/// The fields no built-in flow applies but an accessor reads: `lightMap`
/// through [`material_light_map`](crate::nodes::tsl::material_light_map) and
/// Phong's `specularMap` through
/// [`material_specular_strength`](crate::nodes::tsl::material_specular_strength),
/// in a node the application builds. No kind [`reads`](KindTable::reads)
/// them; they are not [`loud`](KindTable::loud) either, so
/// [`check_supported`](MeshBasicNodeMaterial::check_supported) passes them,
/// but [`warn_unsupported`] says once per material that the flow leaves them
/// out.
pub(crate) const ACCESSOR_ONLY: [Field; 2] = [Field::LightMap, Field::SpecularMap];

/// Which fragment flow a kind takes when it has no `fragment_node`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    /// No lighting model of the kind's own: `setupDiffuseColor()` and
    /// `setupLighting()`'s unlit arms — `BasicLightingModel` for Basic, the
    /// backdrop and `emissiveNode` tail for everyone.
    Unlit,
    /// `PhongLightingModel`; `specular` false is `new PhongLightingModel(
    /// false )`, Lambert's and Toon's.
    Phong {
        /// Whether the Blinn-Phong lobe and its material properties are in.
        specular: bool,
    },
    /// `PhysicalLightingModel`, Standard's and Physical's.
    Physical,
    /// `MeshNormalNodeMaterial.setupDiffuseColor()`: the packed view normal.
    Normal,
}

/// A rule only a draw can judge, because it depends on the scene or the
/// object as well as the material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DrawRule {
    /// The anisotropic `BRDF_GGX` (`V_GGX_SmithCorrelated_Anisotropic`,
    /// `D_GGX_Anisotropic`) is not ported, so under a point, spot or
    /// directional light the highlight would be the isotropic one. The
    /// indirect bent normal is ported, which is why `anisotropy` is in
    /// Physical's [`reads`](KindTable::reads) and an unlit page is quiet.
    DirectAnisotropy,
    /// `PointsNodeMaterial.setupVertexSprite()` is ported for a `sizeNode`
    /// without size attenuation, which is what the ladder draws; on a
    /// `Sprite`, `sizeAttenuation` and a `size` without `sizeNode` are not.
    SpriteSize,
}

impl DrawRule {
    /// The fields this rule says `material` sets but the draw ignores.
    fn unsupported(
        self,
        material: &MeshBasicNodeMaterial,
        setup: &SetupContext,
    ) -> Vec<&'static str> {
        let mut fields = Vec::new();
        match self {
            DrawRule::DirectAnisotropy => {
                let direct_light = setup.lights.iter().any(|light| {
                    matches!(
                        light.kind,
                        LightKind::Point | LightKind::Spot | LightKind::Directional
                    )
                });
                if material.anisotropy > 0.0 && direct_light && material.lights {
                    fields.push("anisotropy (under a direct light)");
                }
            }
            DrawRule::SpriteSize => {
                if setup.sprite {
                    if material.size_attenuation {
                        fields.push("sizeAttenuation (PointsNodeMaterial on a Sprite)");
                    }
                    if material.size_node.is_none() {
                        fields.push("size without sizeNode (PointsNodeMaterial on a Sprite)");
                    }
                }
            }
        }
        fields
    }
}

/// One kind's row of the declaration: see [`MaterialKind::table`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct KindTable {
    /// The fragment flow [`setup()`](super::setup) builds.
    pub flow: Flow,
    /// The fields the kind's flow reads — binds as a uniform or texture, or
    /// lets change the program — as groups. A field set on a kind whose
    /// `reads` lacks it does not reach the shader.
    pub reads: &'static [&'static [Field]],
    /// Fields three's class for the kind reads and the port does not:
    /// [`unsupported_fields`](MeshBasicNodeMaterial::unsupported_fields) is
    /// this list filtered by what the material sets. Every one is optional
    /// ([`Field::is_set`]) and none is in `reads`.
    pub loud: &'static [Field],
    /// The rules only a draw can judge; [`warn_unsupported`] runs them.
    pub draw: &'static [DrawRule],
}

impl KindTable {
    /// Whether the kind's flow reads `field`.
    pub(crate) fn reads(&self, field: Field) -> bool {
        self.reads.iter().any(|group| group.contains(&field))
    }
}

use Field::*;

/// `setupDiffuseColor()`: every kind but Normal, whose override replaces it.
const DIFFUSE: &[Field] = &[
    Color,
    Opacity,
    ColorNode,
    OpacityNode,
    AlphaTestNode,
    AlphaTest,
    AlphaHash,
    AlphaMap,
    Map,
    VertexColors,
];
/// `setupAmbientOcclusion()`, which every flow runs; only Basic's and the lit
/// flows' lighting then reads `AmbientOcclusion`.
const AO: &[Field] = &[AoMap, AoMapIntensity];
/// `setupNormal()`: the material normal the lit flows shade with.
const NORMAL: &[Field] = &[
    FlatShading,
    NormalNode,
    NormalMap,
    NormalScale,
    BumpMap,
    BumpScale,
];
/// `setupLighting()`'s backdrop blend into `totalDiffuse`.
const BACKDROP: &[Field] = &[BackdropNode, BackdropAlphaNode];
/// An unlit flow's emissive: `emissiveNode` alone.
const UNLIT_EMISSIVE: &[Field] = &[EmissiveNode];
/// A lit flow's emissive: `emissiveNode ?? materialEmissive`, with the map
/// and the intensity.
const LIT_EMISSIVE: &[Field] = &[EmissiveNode, Emissive, EmissiveIntensity, EmissiveMap];
/// `MeshPhongNodeMaterial.setupVariants()`'s Blinn-Phong properties.
const PHONG_SPECULAR: &[Field] = &[Specular, Shininess, SpecularNode];
/// `MeshStandardNodeMaterial`: metalness, roughness and the PMREM
/// environment `PhysicalLightingModel` samples.
const STANDARD: &[Field] = &[
    PmremEnv,
    Metalness,
    Roughness,
    MetalnessMap,
    RoughnessMap,
    MetalnessNode,
    RoughnessNode,
];
/// `MeshPhysicalNodeMaterial`'s `setupSpecular()` and `setupVariants()`:
/// the IOR-derived F0 and every extension layer.
const PHYSICAL: &[Field] = &[
    Ior,
    SpecularIntensity,
    SpecularColor,
    SpecularColorMap,
    DiffuseRoughness,
    Clearcoat,
    ClearcoatRoughness,
    ClearcoatMap,
    ClearcoatRoughnessMap,
    ClearcoatNormalMap,
    ClearcoatNormalScale,
    Sheen,
    SheenColor,
    SheenRoughness,
    Iridescence,
    IridescenceIor,
    IridescenceThicknessRange,
    IridescenceThicknessMap,
    Anisotropy,
    AnisotropyRotation,
    AnisotropyMap,
    Transmission,
    TransmissionMap,
    Thickness,
    ThicknessMap,
    AttenuationDistance,
    AttenuationColor,
];

impl MaterialKind {
    /// The kind's flow, the fields it reads, the fields it is loud about and
    /// its draw-time rules — the single source the flow dispatch,
    /// [`unsupported_fields`](MeshBasicNodeMaterial::unsupported_fields) and
    /// the renderer's warning all read (see the module docs).
    pub(crate) const fn table(self) -> KindTable {
        match self {
            MaterialKind::Basic => KindTable {
                flow: Flow::Unlit,
                // `MeshBasicNodeMaterial.setupEnvironment()`'s
                // `BasicEnvironmentNode`, blended by `reflectivity`. Its
                // reflect vector reads the material normal, so `NORMAL` is in
                // too, though only with an `env_map` set. Three's
                // `MeshBasicNodeMaterial.setupNormal()` returns the geometry
                // normal ("not affected by normal and bump maps"), so there
                // the normal and bump maps and `normalNode` would not count.
                reads: &[
                    DIFFUSE,
                    AO,
                    UNLIT_EMISSIVE,
                    BACKDROP,
                    &[EnvMap, Reflectivity],
                    NORMAL,
                ],
                loud: &[PmremEnv],
                draw: &[],
            },
            MaterialKind::Phong => KindTable {
                flow: Flow::Phong { specular: true },
                reads: &[DIFFUSE, AO, NORMAL, LIT_EMISSIVE, BACKDROP, PHONG_SPECULAR],
                // Phong and Lambert wrap the same environment node in their
                // own lighting model, which is not ported.
                loud: &[EnvMap, PmremEnv],
                draw: &[],
            },
            MaterialKind::Lambert => KindTable {
                flow: Flow::Phong { specular: false },
                reads: &[DIFFUSE, AO, NORMAL, LIT_EMISSIVE, BACKDROP],
                loud: &[EnvMap, PmremEnv],
                draw: &[],
            },
            MaterialKind::Toon => KindTable {
                flow: Flow::Phong { specular: false },
                reads: &[DIFFUSE, AO, NORMAL, LIT_EMISSIVE, BACKDROP, &[GradientMap]],
                loud: &[EnvMap, PmremEnv],
                draw: &[],
            },
            MaterialKind::Standard => KindTable {
                flow: Flow::Physical,
                reads: &[DIFFUSE, AO, NORMAL, LIT_EMISSIVE, BACKDROP, STANDARD],
                // A plain cube three would PMREM on the fly; the port takes
                // a PMREM through `pmrem_env` (or `scene.environment`).
                loud: &[EnvMap],
                draw: &[],
            },
            MaterialKind::Physical => KindTable {
                flow: Flow::Physical,
                reads: &[
                    DIFFUSE,
                    AO,
                    NORMAL,
                    LIT_EMISSIVE,
                    BACKDROP,
                    STANDARD,
                    PHYSICAL,
                ],
                loud: &[EnvMap],
                draw: &[DrawRule::DirectAnisotropy],
            },
            MaterialKind::Normal => KindTable {
                flow: Flow::Normal,
                // No `colorNode`, no vertex colours, no alpha test: only the
                // opacity reaches the packed normal's `w`.
                reads: &[&[Opacity, OpacityNode, AlphaMap], AO, NORMAL],
                // No lighting step for a backdrop blend to sit in.
                loud: &[EnvMap, PmremEnv, BackdropNode],
                draw: &[],
            },
            MaterialKind::Sprite => KindTable {
                flow: Flow::Unlit,
                // `SpriteNodeMaterial.setupPositionView()`'s billboard.
                reads: &[
                    DIFFUSE,
                    AO,
                    UNLIT_EMISSIVE,
                    BACKDROP,
                    &[ScaleNode, RotationNode, Rotation, SizeAttenuation],
                ],
                loud: &[EnvMap, PmremEnv],
                draw: &[],
            },
            MaterialKind::Points => KindTable {
                flow: Flow::Unlit,
                // `PointsNodeMaterial.setupVertexSprite()` on a `Sprite`; on
                // a `Points` object three draws one-pixel points and reads
                // none of the three.
                reads: &[
                    DIFFUSE,
                    AO,
                    UNLIT_EMISSIVE,
                    BACKDROP,
                    &[ScaleNode, RotationNode, SizeNode],
                ],
                loud: &[EnvMap, PmremEnv],
                draw: &[DrawRule::SpriteSize],
            },
            MaterialKind::Line2 => KindTable {
                flow: Flow::Unlit,
                // `Line2NodeMaterial`'s screen-space quad and coverage.
                reads: &[
                    DIFFUSE,
                    AO,
                    UNLIT_EMISSIVE,
                    BACKDROP,
                    &[Linewidth, WorldUnits],
                ],
                loud: &[EnvMap, PmremEnv],
                draw: &[],
            },
        }
    }
}

/// The renderer's "loud" warning, said once per material and field on
/// stderr when the material's program is built: the kind's
/// [`unsupported_fields`](MeshBasicNodeMaterial::unsupported_fields), its
/// [`draw`](KindTable::draw) rules judged against this draw, and the
/// [`ACCESSOR_ONLY`] maps it sets. One registry for all three.
pub(crate) fn warn_unsupported(id: usize, material: &MeshBasicNodeMaterial, setup: &SetupContext) {
    thread_local! {
        static WARNED: std::cell::RefCell<std::collections::HashSet<(usize, &'static str)>> =
            std::cell::RefCell::new(std::collections::HashSet::new());
    }
    let table = material.kind.table();
    let mut fields = material.unsupported_fields();
    for rule in table.draw {
        fields.extend(rule.unsupported(material, setup));
    }
    let label = format!(
        "{:?}{}",
        material.kind,
        if material.name.is_empty() {
            String::new()
        } else {
            format!(" \"{}\"", material.name)
        }
    );
    WARNED.with(|warned| {
        let mut warned = warned.borrow_mut();
        for field in fields {
            if warned.insert((id, field)) {
                eprintln!(
                    "three-rs: material {id} ({label}): {field} is set but not supported, and is ignored",
                );
            }
        }
        for field in material.accessor_only_fields() {
            if warned.insert((id, field)) {
                eprintln!(
                    "three-rs: material {id} ({label}): {field} is set, but the built-in lighting flow does not apply it; only the materialLightMap / materialSpecularStrength accessors read it",
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::Field::{self, *};
    use super::{MaterialKind, ACCESSOR_ONLY};
    use crate::materials::environment::PmremHandle;
    use crate::materials::{setup, MeshBasicNodeMaterial, SetupContext};
    use crate::math::{Color as Rgb, Vector2};
    use crate::nodes::builder::BindingDesc;
    use crate::nodes::node::UniformSource as U;
    use crate::nodes::tsl::{float, texture};
    use crate::nodes::{NodeBuilder, NodeProgram};
    use crate::textures::{CubeTexture, Image, Texture};
    use std::collections::{BTreeSet, HashMap};

    const KINDS: [MaterialKind; 10] = [
        MaterialKind::Basic,
        MaterialKind::Phong,
        MaterialKind::Lambert,
        MaterialKind::Sprite,
        MaterialKind::Standard,
        MaterialKind::Physical,
        MaterialKind::Points,
        MaterialKind::Normal,
        MaterialKind::Line2,
        MaterialKind::Toon,
    ];

    /// One distinct texture per field: a map field binds its own, a node
    /// field samples its own, so the program's bindings say which field it
    /// read.
    struct Markers {
        textures: HashMap<Field, Texture>,
        cubes: HashMap<Field, CubeTexture>,
    }

    impl Markers {
        fn new() -> Self {
            let cube = || {
                CubeTexture::new(
                    (0..6)
                        .map(|_| Image::rgba8(1, 1, vec![255, 255, 255, 255]))
                        .collect(),
                )
            };
            Markers {
                textures: Field::ALL
                    .iter()
                    .map(|&field| (field, Texture::new(4, 4, None)))
                    .collect(),
                cubes: [(EnvMap, cube()), (PmremEnv, cube())].into_iter().collect(),
            }
        }
        fn map(&self, field: Field) -> Option<Texture> {
            Some(self.textures[&field].clone())
        }
        fn node(&self, field: Field) -> Option<crate::nodes::NodeRef> {
            Some(texture(&self.textures[&field]))
        }
        fn id(&self, field: Field) -> usize {
            match self.cubes.get(&field) {
                Some(cube) => cube.id(),
                None => self.textures[&field].id(),
            }
        }
    }

    fn material(kind: MaterialKind) -> MeshBasicNodeMaterial {
        let white = Rgb::from_hex(0xffffff);
        match kind {
            MaterialKind::Basic => MeshBasicNodeMaterial::new(),
            MaterialKind::Phong => MeshBasicNodeMaterial::phong(white),
            MaterialKind::Lambert => MeshBasicNodeMaterial::lambert(white),
            MaterialKind::Toon => MeshBasicNodeMaterial::toon(white, None),
            MaterialKind::Sprite => MeshBasicNodeMaterial::sprite(),
            MaterialKind::Points => MeshBasicNodeMaterial::points(),
            MaterialKind::Standard => MeshBasicNodeMaterial::standard(white, 0.5, 0.5),
            MaterialKind::Physical => MeshBasicNodeMaterial::physical(white, 0.5, 0.5),
            MaterialKind::Normal => MeshBasicNodeMaterial::normal(),
            MaterialKind::Line2 => MeshBasicNodeMaterial::line2(white),
        }
    }

    fn context(kind: MaterialKind) -> SetupContext {
        let geometry_attributes = match kind {
            MaterialKind::Line2 => {
                let mut geometry = crate::addons::lines::LineSegmentsGeometry::new();
                geometry.set_positions(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
                geometry.set_colors(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
                geometry.geometry().attribute_descs()
            }
            _ => Vec::new(),
        };
        SetupContext {
            lights: vec![crate::materials::phong::LightDesc {
                index: 0,
                kind: crate::lights::LightKind::Directional,
                shadow_map: None,
            }],
            sprite: matches!(kind, MaterialKind::Sprite | MaterialKind::Points),
            vertex_color_size: 3,
            geometry_attributes,
            viewport_opaque_mip: Some(crate::materials::transmission::OpaqueFrame {
                texture: Texture::new(4, 4, None),
            }),
            ..SetupContext::default()
        }
    }

    /// The fields that have to be set for `field` to mean anything on
    /// `kind`: a scale with its map, a clearcoat layer's map with the layer
    /// on. Basic reads the material normal only along the env map's reflect
    /// vector, so its normal fields need the env map too.
    fn requires(kind: MaterialKind, field: Field) -> Vec<Field> {
        let mut fields = match field {
            Reflectivity => vec![EnvMap],
            AoMapIntensity => vec![AoMap],
            LightMapIntensity => vec![LightMap],
            BumpScale => vec![BumpMap],
            NormalScale => vec![NormalMap],
            ClearcoatRoughness | ClearcoatMap | ClearcoatRoughnessMap | ClearcoatNormalMap => {
                vec![Clearcoat]
            }
            ClearcoatNormalScale => vec![Clearcoat, ClearcoatNormalMap],
            SheenColor | SheenRoughness => vec![Sheen],
            AnisotropyRotation | AnisotropyMap => vec![Anisotropy],
            IridescenceIor | IridescenceThicknessRange | IridescenceThicknessMap => {
                vec![Iridescence]
            }
            TransmissionMap | Thickness | ThicknessMap | AttenuationDistance | AttenuationColor => {
                vec![Transmission]
            }
            BackdropAlphaNode => vec![BackdropNode],
            _ => Vec::new(),
        };
        if kind == MaterialKind::Basic
            && matches!(
                field,
                FlatShading | NormalNode | NormalMap | NormalScale | BumpMap | BumpScale
            )
        {
            fields.push(EnvMap);
        }
        fields
    }

    fn set(m: &mut MeshBasicNodeMaterial, field: Field, t: &Markers) {
        let c = Rgb::from_hex(0x336699);
        match field {
            Color => m.color = c,
            Opacity => m.opacity = 0.5,
            Reflectivity => m.reflectivity = 0.5,
            RefractionRatio => m.refraction_ratio = 0.5,
            EnvMap => m.env_map = Some(t.cubes[&EnvMap].clone()),
            PmremEnv => {
                m.pmrem_env = Some(PmremHandle {
                    texture: t.cubes[&PmremEnv].clone(),
                    max_lod: float(8.0),
                })
            }
            ColorNode => m.color_node = t.node(field),
            OpacityNode => m.opacity_node = t.node(field).map(|n| n.x()),
            AlphaTestNode => m.alpha_test_node = t.node(field).map(|n| n.x()),
            AlphaTest => m.alpha_test = 0.5,
            AlphaHash => m.alpha_hash = true,
            AlphaMap => m.alpha_map = t.map(field),
            EmissiveNode => m.emissive_node = t.node(field).map(|n| n.xyz()),
            Specular => m.specular = c,
            Shininess => m.shininess = 10.0,
            Emissive => m.emissive = c,
            EmissiveIntensity => m.emissive_intensity = 2.0,
            FlatShading => m.flat_shading = true,
            Metalness => m.metalness = 0.25,
            Roughness => m.roughness = 0.25,
            Map => m.map = t.map(field),
            VertexColors => m.vertex_colors = true,
            RoughnessMap => m.roughness_map = t.map(field),
            MetalnessMap => m.metalness_map = t.map(field),
            EmissiveMap => m.emissive_map = t.map(field),
            GradientMap => m.gradient_map = t.map(field),
            AoMap => m.ao_map = t.map(field),
            AoMapIntensity => m.ao_map_intensity = 0.5,
            LightMap => m.light_map = t.map(field),
            LightMapIntensity => m.light_map_intensity = 0.5,
            SpecularMap => m.specular_map = t.map(field),
            BumpMap => m.bump_map = t.map(field),
            BumpScale => m.bump_scale = 2.0,
            Clearcoat => m.clearcoat = 1.0,
            ClearcoatRoughness => m.clearcoat_roughness = 0.5,
            ClearcoatMap => m.clearcoat_map = t.map(field),
            ClearcoatRoughnessMap => m.clearcoat_roughness_map = t.map(field),
            Sheen => m.sheen = 1.0,
            SheenColor => m.sheen_color = c,
            SheenRoughness => m.sheen_roughness = 0.5,
            DiffuseRoughness => m.diffuse_roughness = 0.5,
            Ior => m.ior = 1.2,
            SpecularIntensity => m.specular_intensity = 0.5,
            SpecularColor => m.specular_color = c,
            SpecularNode => m.specular_node = t.node(field).map(|n| n.xyz()),
            NormalMap => m.normal_map = t.map(field),
            NormalScale => m.normal_scale = Vector2::new(2.0, 2.0),
            SpecularColorMap => m.specular_color_map = t.map(field),
            Anisotropy => m.anisotropy = 1.0,
            AnisotropyRotation => m.anisotropy_rotation = 1.0,
            AnisotropyMap => m.anisotropy_map = t.map(field),
            Iridescence => m.iridescence = 1.0,
            IridescenceIor => m.iridescence_ior = 1.5,
            IridescenceThicknessRange => m.iridescence_thickness_range = [200.0, 300.0],
            IridescenceThicknessMap => m.iridescence_thickness_map = t.map(field),
            ClearcoatNormalMap => m.clearcoat_normal_map = t.map(field),
            ClearcoatNormalScale => m.clearcoat_normal_scale = Vector2::new(2.0, 2.0),
            Transmission => m.transmission = 1.0,
            TransmissionMap => m.transmission_map = t.map(field),
            Thickness => m.thickness = 1.0,
            ThicknessMap => m.thickness_map = t.map(field),
            AttenuationDistance => m.attenuation_distance = 2.0,
            AttenuationColor => m.attenuation_color = c,
            NormalNode => m.normal_node = t.node(field).map(|n| n.xyz()),
            ScaleNode => m.scale_node = t.node(field).map(|n| n.xy()),
            RotationNode => m.rotation_node = t.node(field).map(|n| n.x()),
            SizeNode => m.size_node = t.node(field).map(|n| n.x()),
            Size => m.size = 3.0,
            Rotation => m.rotation = 0.5,
            Linewidth => m.linewidth = 3.0,
            SizeAttenuation => m.size_attenuation = !m.size_attenuation,
            WorldUnits => m.world_units = true,
            BackdropNode => m.backdrop_node = t.node(field).map(|n| n.xyz()),
            BackdropAlphaNode => m.backdrop_alpha_node = t.node(field).map(|n| n.x()),
            MetalnessNode => m.metalness_node = t.node(field).map(|n| n.x()),
            RoughnessNode => m.roughness_node = t.node(field).map(|n| n.x()),
        }
    }

    /// How a field shows in a program.
    enum Probe {
        /// Its own marker texture is bound (a map, or a node sampling one).
        Texture,
        /// One of these uniforms is bound.
        Uniform(&'static [U]),
        /// A flag: the program differs from the one without it.
        Changes,
    }

    fn probe(field: Field) -> Probe {
        match field {
            Color => Probe::Uniform(&[U::MaterialColor]),
            Opacity => Probe::Uniform(&[U::MaterialOpacity]),
            Reflectivity => Probe::Uniform(&[U::MaterialReflectivity]),
            RefractionRatio => Probe::Uniform(&[U::MaterialRefractionRatio]),
            AlphaTest => Probe::Uniform(&[U::MaterialAlphaTest]),
            Specular => Probe::Uniform(&[U::MaterialSpecular]),
            Shininess => Probe::Uniform(&[U::MaterialShininess]),
            Emissive => Probe::Uniform(&[U::MaterialEmissive]),
            EmissiveIntensity => Probe::Uniform(&[U::MaterialEmissiveIntensity]),
            Metalness => Probe::Uniform(&[U::MaterialMetalness]),
            Roughness => Probe::Uniform(&[U::MaterialRoughness]),
            AoMapIntensity => Probe::Uniform(&[U::MaterialAoMapIntensity]),
            LightMapIntensity => Probe::Uniform(&[U::MaterialLightMapIntensity]),
            BumpScale => Probe::Uniform(&[U::MaterialBumpScale]),
            Clearcoat => Probe::Uniform(&[U::MaterialClearcoat]),
            ClearcoatRoughness => Probe::Uniform(&[U::MaterialClearcoatRoughness]),
            Sheen => Probe::Uniform(&[U::MaterialSheen]),
            SheenColor => Probe::Uniform(&[U::MaterialSheenColor]),
            SheenRoughness => Probe::Uniform(&[U::MaterialSheenRoughness]),
            DiffuseRoughness => Probe::Uniform(&[U::MaterialDiffuseRoughness]),
            Ior => Probe::Uniform(&[U::MaterialIor]),
            SpecularIntensity => Probe::Uniform(&[U::MaterialSpecularIntensity]),
            SpecularColor => Probe::Uniform(&[U::MaterialSpecularColor]),
            NormalScale => Probe::Uniform(&[U::MaterialNormalScale]),
            Anisotropy | AnisotropyRotation => Probe::Uniform(&[U::MaterialAnisotropyVector]),
            Iridescence => Probe::Uniform(&[U::MaterialIridescence]),
            IridescenceIor => Probe::Uniform(&[U::MaterialIridescenceIor]),
            IridescenceThicknessRange => Probe::Uniform(&[
                U::MaterialIridescenceThicknessMin,
                U::MaterialIridescenceThicknessMax,
            ]),
            ClearcoatNormalScale => Probe::Uniform(&[U::MaterialClearcoatNormalScale]),
            Transmission => Probe::Uniform(&[U::MaterialTransmission]),
            Thickness => Probe::Uniform(&[U::MaterialThickness]),
            AttenuationDistance => Probe::Uniform(&[U::MaterialAttenuationDistance]),
            AttenuationColor => Probe::Uniform(&[U::MaterialAttenuationColor]),
            Size => Probe::Uniform(&[U::MaterialPointSize]),
            Rotation => Probe::Uniform(&[U::MaterialRotation]),
            Linewidth => Probe::Uniform(&[U::MaterialLineWidth]),
            AlphaHash | FlatShading | VertexColors | SizeAttenuation | WorldUnits => Probe::Changes,
            _ => Probe::Texture,
        }
    }

    fn build(material: &MeshBasicNodeMaterial, kind: MaterialKind) -> NodeProgram {
        NodeBuilder::new().build(&setup(material, &context(kind), None))
    }

    fn binds_texture(program: &NodeProgram, id: usize) -> bool {
        program
            .groups
            .iter()
            .flatten()
            .any(|binding| match binding {
                BindingDesc::Texture { source, .. } => source.id() == id,
                _ => false,
            })
    }

    fn binds_uniform(program: &NodeProgram, sources: &[U]) -> bool {
        program
            .groups
            .iter()
            .flatten()
            .any(|binding| match binding {
                BindingDesc::Uniforms { members, .. } => members
                    .iter()
                    .any(|member| sources.contains(&member.source)),
                _ => false,
            })
    }

    /// Whether `program`, built from `with`, shows `field`; `without` is the
    /// same material with `field` left at its default.
    fn shows(
        field: Field,
        program: &NodeProgram,
        without: impl FnOnce() -> NodeProgram,
        t: &Markers,
    ) -> bool {
        match probe(field) {
            Probe::Texture => binds_texture(program, t.id(field)),
            Probe::Uniform(sources) => binds_uniform(program, sources),
            Probe::Changes => {
                let other = without();
                program.vertex_wgsl != other.vertex_wgsl
                    || program.fragment_wgsl != other.fragment_wgsl
            }
        }
    }

    /// The fields `kind` shows, each set alone over what it requires.
    fn shown_one_at_a_time(kind: MaterialKind, t: &Markers) -> BTreeSet<Field> {
        Field::ALL
            .into_iter()
            .filter(|&field| {
                let mut base = material(kind);
                for dep in requires(kind, field) {
                    set(&mut base, dep, t);
                }
                let mut with = base.clone();
                set(&mut with, field, t);
                shows(field, &build(&with, kind), || build(&base, kind), t)
            })
            .collect()
    }

    /// The fields `kind` shows with every field set at once.
    fn shown_all_at_once(kind: MaterialKind, t: &Markers) -> BTreeSet<Field> {
        let all_but = |skip: Option<Field>| {
            let mut material = material(kind);
            for field in Field::ALL.into_iter().filter(|&field| Some(field) != skip) {
                set(&mut material, field, t);
            }
            build(&material, kind)
        };
        let program = all_but(None);
        Field::ALL
            .into_iter()
            .filter(|&field| shows(field, &program, || all_but(Some(field)), t))
            .collect()
    }

    fn declared(kind: MaterialKind) -> BTreeSet<Field> {
        let table = kind.table();
        Field::ALL
            .into_iter()
            .filter(|&field| table.reads(field))
            .collect()
    }

    /// Every kind's program shows exactly the fields its table says it
    /// reads. Setting every field at once cannot show all of them — three's
    /// `??` lets `normalNode` hide `normalMap`, which hides `bumpMap`, and
    /// `colorNode` hide `map` — so each field is set alone, over the fields
    /// it needs ([`requires`]), and must show if and only if it is declared.
    /// Then everything is set at once, and nothing undeclared may show.
    #[test]
    fn every_kind_reads_what_its_table_declares() {
        let t = Markers::new();
        for kind in KINDS {
            let declared = declared(kind);
            let shown = shown_one_at_a_time(kind, &t);
            assert!(
                shown == declared,
                "{kind:?}: shown but not declared {:?}, declared but not shown {:?}",
                shown.difference(&declared).collect::<Vec<_>>(),
                declared.difference(&shown).collect::<Vec<_>>(),
            );
            let all = shown_all_at_once(kind, &t);
            assert!(
                all.is_subset(&declared),
                "{kind:?} with every field set shows undeclared {:?}",
                all.difference(&declared).collect::<Vec<_>>(),
            );
        }
    }

    /// A loud field is one the kind does not read and that a material can
    /// leave unset; an accessor-only map is read by no kind.
    #[test]
    fn loud_fields_are_optional_and_unread() {
        let t = Markers::new();
        for kind in KINDS {
            let table = kind.table();
            for &field in table.loud {
                assert!(!table.reads(field), "{kind:?} reads loud {field:?}");
                let mut material = material(kind);
                assert!(!field.is_set(&material), "{kind:?}: {field:?}");
                set(&mut material, field, &t);
                assert!(field.is_set(&material), "{kind:?}: {field:?}");
            }
            for field in ACCESSOR_ONLY {
                assert!(!table.reads(field), "{kind:?} reads {field:?}");
            }
        }
        assert!(Field::ALL.windows(2).all(|pair| pair[0] < pair[1]));
    }

    /// `unsupported_fields()` is the table's loud list: what #171's audit
    /// settled, unchanged by moving it into the table.
    #[test]
    fn unsupported_fields_follow_the_table() {
        let t = Markers::new();
        for kind in KINDS {
            let mut material = material(kind);
            assert!(material.unsupported_fields().is_empty(), "{kind:?}");
            for field in [EnvMap, PmremEnv, BackdropNode, LightMap, SpecularMap] {
                set(&mut material, field, &t);
            }
            let expected: &[&str] = match kind {
                MaterialKind::Basic => &["envMap (PMREM)"],
                MaterialKind::Standard | MaterialKind::Physical => &["envMap"],
                MaterialKind::Normal => &["envMap", "envMap (PMREM)", "backdropNode"],
                _ => &["envMap", "envMap (PMREM)"],
            };
            assert_eq!(material.unsupported_fields(), expected, "{kind:?}");
            assert_eq!(
                material.accessor_only_fields(),
                ["lightMap", "specularMap"],
                "{kind:?}"
            );
        }
    }

    /// The draw-time rules: Physical's anisotropy under a direct light, and
    /// a Points material's size on a `Sprite`.
    #[test]
    fn draw_rules_need_the_draw() {
        let lit = context(MaterialKind::Physical);
        let unlit = SetupContext::default();
        let mut physical = material(MaterialKind::Physical);
        physical.anisotropy = 1.0;
        let unsupported = |material: &MeshBasicNodeMaterial, setup: &SetupContext| {
            let table = material.kind.table();
            table
                .draw
                .iter()
                .flat_map(|rule| rule.unsupported(material, setup))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            unsupported(&physical, &lit),
            ["anisotropy (under a direct light)"]
        );
        assert!(unsupported(&physical, &unlit).is_empty());
        let mut standard = material(MaterialKind::Standard);
        standard.anisotropy = 1.0;
        assert!(unsupported(&standard, &lit).is_empty());

        let on_sprite = context(MaterialKind::Points);
        let mut points = material(MaterialKind::Points);
        assert_eq!(
            unsupported(&points, &on_sprite),
            [
                "sizeAttenuation (PointsNodeMaterial on a Sprite)",
                "size without sizeNode (PointsNodeMaterial on a Sprite)"
            ]
        );
        assert!(unsupported(&points, &unlit).is_empty());
        points.size_attenuation = false;
        points.size_node = Some(float(2.0));
        assert!(unsupported(&points, &on_sprite).is_empty());
    }
}
