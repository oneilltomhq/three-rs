//! Port of `three.js/src/renderers/common/CubeRenderTarget.js` —
//! `fromEquirectangularTexture()` only, which is the one arm this ladder
//! reaches.
//!
//! `CubeMapNode.updateBefore()` takes it whenever a material's or a scene's
//! environment is a `Texture` with `EquirectangularReflectionMapping`: the
//! equirectangular map becomes a `CubeTexture` of `image.height` a side, cached
//! per source texture, and everything downstream samples the cube. That is why
//! three's dump of `webgpu_postprocessing_bloom_emissive` — whose page assigns
//! the raw 1024×512 HDR straight to `scene.background` — contains a 512² cube
//! conversion pass nobody wrote.
//!
//! **What this port does differently.** Three renders the six faces straight
//! into the cube texture's array layers, one `setRenderTarget( target, face )`
//! each. [`Renderer`] has no layered colour attachment, so each face is drawn
//! into a plain 2-D render target of the same size and format and then copied
//! into its layer. The copy is exact — same format, same extent, no sampling —
//! so the cube's texels are the ones three's are, and the divergence is a
//! command-buffer one, recorded in `docs/nodes.md`.

use crate::cameras::PerspectiveCamera;
use crate::error::Error;
use crate::geometries::box_geometry;
use crate::materials::{Blending, MeshBasicNodeMaterial, Side};
use crate::math::Vector3;
use crate::nodes::tsl::{equirect_uv, float, position_world_direction, texture_level};
use crate::objects::{Mesh, Scene};
use crate::textures::{CubeTexture, Texture, TextureFilter, TextureType};

use super::render_target::{RenderTarget, RenderTargetOptions};
use super::Renderer;

use std::rc::Rc;

/// `CubeCamera.updateCoordinateSystem()`'s `WebGPUCoordinateSystem` branch: the
/// `up` vector and the point each face camera looks at, in the order px, nx,
/// py, ny, pz, nz — the order [`CubeTexture`]'s images are in.
///
/// Note px looking at **-x** and nx at **+x**: the same left-handed cube-map
/// convention the `x` negation in `CubeTextureNode.setupUV()` undoes at the
/// sampling end.
const FACES: [([f64; 3], [f64; 3]); 6] = [
    ([0.0, -1.0, 0.0], [-1.0, 0.0, 0.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0]),
    ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
];

/// `const fov = - 90; // negative fov is not an error` — the vertical flip a
/// cube face needs under WebGPU's top-left texture origin, expressed as a
/// frustum with `top < bottom`.
const FOV: f64 = -90.0;

/// `new CubeRenderTarget( texture.image.height ).fromEquirectangularTexture(
/// renderer, texture )`.
///
/// The returned cube carries the source's type and colour space, no mipmaps and
/// `LinearFilter` both ways, which is what three's descriptor dump shows.
pub fn from_equirectangular_texture(
    renderer: &mut Renderer,
    source: &Texture,
) -> Result<CubeTexture, Error> {
    let size = source.size().1;

    let texture_type = match source.format() {
        wgpu::TextureFormat::Rgba16Float => TextureType::HalfFloat,
        _ => TextureType::UnsignedByte,
    };
    let cube = CubeTexture::render_target(size, texture_type);
    cube.set_color_space(source.color_space());

    // `new Mesh( new BoxGeometry( 5, 5, 5 ), material )`, seen from inside:
    // `side = BackSide`, `blending = NoBlending`, and a colour node that is the
    // equirectangular map read at level 0 along the world-space view direction.
    let mut material = MeshBasicNodeMaterial::new();
    material.color_node = Some(texture_level(
        source,
        equirect_uv(position_world_direction()),
        float(0.0),
    ));
    material.side = Side::Back;
    material.blending = Blending::No;

    let mesh = Mesh::new(Rc::new(box_geometry(5.0, 5.0, 5.0, 1, 1, 1)), material);
    let mut scene = Scene::new();
    scene.add(&mesh);

    let face_target = RenderTarget::new_with_options(
        size,
        size,
        RenderTargetOptions {
            texture_type,
            samples: 0,
            depth_buffer: true,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
        },
    )?;

    // `const currentMRT = renderer.getMRT(); renderer.setMRT( null )` — the
    // conversion writes one attachment however many the frame that asked for it
    // has.
    let previous_mrt = renderer.mrt();
    let previous_target = renderer.render_target();
    renderer.set_mrt(None);
    renderer.set_render_target(Some(face_target.clone()));

    let mut camera = PerspectiveCamera::new(FOV, 1.0, 1.0, 10.0);
    for (layer, (up, look_at)) in FACES.iter().enumerate() {
        {
            let mut object = camera.node.borrow_mut();
            object.up = Vector3::new(up[0], up[1], up[2]);
        }
        camera.look_at(&Vector3::new(look_at[0], look_at[1], look_at[2]));
        renderer.render(&mut scene, &mut camera);
        renderer.copy_to_cube_layer(&face_target, &cube, layer as u32);
    }

    renderer.set_render_target(previous_target);
    renderer.set_mrt(previous_mrt);

    Ok(cube)
}
