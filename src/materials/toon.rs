//! Port of `three.js/src/nodes/functions/ToonLightingModel.js` — the
//! lighting model `MeshToonNodeMaterial.setupLightingModel()` returns.
//!
//! The model is `PhongLightingModel( false )` (Lambert) with one change: the
//! direct light's smooth `saturate( dotNL )` irradiance is replaced by
//! `getGradientIrradiance()`, a lookup of `dotNL * 0.5 + 0.5` in the
//! material's `gradientMap`. Its `indirect()` is Lambert's word for word, so
//! the fragment flow around it is `setup_phong()`'s Lambert arm; only the
//! per-light term lives here. Read off `webgpu_materials_toon`'s dump
//! (`m05`, `docs/nodes.md` §38).

use crate::nodes::tsl::*;
use crate::nodes::{NodeRef, Type};
use crate::textures::Texture;

use super::phong::{brdf_lambert, setup_light, LightDesc};

/// `getGradientIrradiance( { normal, lightDirection, builder } )`.
///
/// With a `gradientMap` the ramp is `materialReference( 'gradientMap',
/// 'texture' ).context( { getUV: () => coord } )`: a texture node whose uv is
/// `vec2( dotNL * 0.5 + 0.5, 0 )` *through the map's uv matrix*, since
/// `getUV` only replaces the default `uv()` and `updateMatrix` stays on. The
/// page's maps are `DataTexture`s — `NearestFilter` both ways — so the tap is
/// a `textureLoad` with the clamp-wrap helper, which is what makes the steps
/// hard. `vec3( gradientMap.r )` splats the red channel.
///
/// Without one it is the built-in two-step ramp, antialiased over one pixel
/// by `fwidth`: `mix( vec3( 0.7 ), vec3( 1 ), smoothstep( 0.7 - fw.x, 0.7 +
/// fw.x, coord.x ) )`.
pub fn gradient_irradiance(
    normal: NodeRef,
    light_direction: NodeRef,
    gradient_map: Option<&Texture>,
) -> NodeRef {
    // `dotNL` will be from -1.0 to 1.0.
    let dot_nl = normal.dot(light_direction);
    let coord = vec2_join(vec![dot_nl.mul(0.5).add(0.5), float(0.0)]);

    match gradient_map {
        Some(map) => texture_with_uv(map, coord).x().to(Type::Vec3),
        None => {
            let fw = fwidth(coord.clone()).mul(0.5);
            mix(
                vec3(0.7, 0.7, 0.7),
                vec3(1.0, 1.0, 1.0),
                smoothstep(
                    float(0.7).sub(fw.clone().x()),
                    float(0.7).add(fw.x()),
                    coord.x(),
                ),
            )
        }
    }
}

/// One direct light through `ToonLightingModel.direct()`:
///
/// ```js
/// const irradiance = getGradientIrradiance( { normal: normalView, lightDirection, builder } ).mul( lightColor );
/// reflectedLight.directDiffuse.addAssign( irradiance.mul( BRDF_Lambert( { diffuseColor: diffuseColor.rgb } ) ) );
/// ```
///
/// No specular term at all, as in Lambert.
pub fn direct_light(
    light: &LightDesc,
    received_shadow_position: Option<&NodeRef>,
    gradient_map: Option<&Texture>,
    out: &mut Vec<NodeRef>,
) {
    let Some((light_direction, light_color)) = setup_light(light, received_shadow_position, out)
    else {
        return;
    };

    let irradiance =
        gradient_irradiance(normal_view(), light_direction, gradient_map).mul(light_color);
    out.push(
        direct_diffuse()
            .assign(direct_diffuse().add(irradiance.mul(brdf_lambert(diffuse_color().xyz())))),
    );
}
