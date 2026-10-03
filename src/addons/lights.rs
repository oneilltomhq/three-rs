//! Port of `three.js/examples/jsm/lights/LightProbeGenerator.js`.

use crate::core::Node;
use crate::error::Error;
use crate::lights::LightProbe;
use crate::math::{Color, ColorSpace, SphericalHarmonics3, Vector3};
use crate::renderer::Renderer;
use crate::textures::{CubeTexture, TextureType};

/// `class LightProbeGenerator` — builds a [`LightProbe`] out of an
/// environment cube by projecting its radiance onto the nine order-2
/// spherical-harmonic basis functions (Sloan, "Stupid Spherical Harmonics
/// Tricks", the projection three cites).
pub struct LightProbeGenerator;

/// `fromCubeTexture()`'s and `fromCubeRenderTarget()`'s shared loop: every
/// texel of every face, weighted by the solid angle it subtends, projected
/// onto the SH basis and accumulated, then normalised so the weights sum to
/// the sphere's 4π.
///
/// `texel( face, index )` gives the texel's colour, already linear, and
/// `coord( face, col, row )` its point on the unit cube — the two halves that
/// differ between the two entry points.
struct Projection {
    sh: SphericalHarmonics3,
    total_weight: f64,
}

impl Projection {
    fn new() -> Self {
        Self {
            sh: SphericalHarmonics3::default(),
            total_weight: 0.0,
        }
    }

    /// One texel: `coord` on the unit cube, `color` linear.
    fn add(&mut self, coord: Vector3, color: Color) {
        // weight assigned to this pixel
        let length_sq = coord.length_sq();
        let weight = 4.0 / (length_sq.sqrt() * length_sq);
        self.total_weight += weight;

        // direction vector to this pixel
        let mut dir = coord;
        dir.normalize();

        // evaluate SH basis functions in direction dir
        let basis = SphericalHarmonics3::static_get_basis_at(&dir);

        // accumulate
        for (c, b) in self.sh.coefficients.iter_mut().zip(basis) {
            c.x += b * color.r * weight;
            c.y += b * color.g * weight;
            c.z += b * color.b * weight;
        }
    }

    /// `norm = ( 4 * Math.PI ) / totalWeight`, applied to every coefficient.
    fn finish(mut self) -> SphericalHarmonics3 {
        let norm = (4.0 * std::f64::consts::PI) / self.total_weight;
        for c in &mut self.sh.coefficients {
            c.x *= norm;
            c.y *= norm;
            c.z *= norm;
        }
        self.sh
    }
}

/// `convertColorToLinear( color, colorSpace )`. Every colour space the port
/// has is one three handles, so there is no warning arm.
fn convert_color_to_linear(color: &mut Color, color_space: ColorSpace) {
    match color_space {
        ColorSpace::Srgb => {
            color.convert_srgb_to_linear();
        }
        ColorSpace::LinearSrgb | ColorSpace::NoColorSpace => {}
    }
}

impl LightProbeGenerator {
    /// `LightProbeGenerator.fromCubeTexture( cubeTexture )` — a new
    /// [`LightProbe`] (intensity 1) holding the irradiance of `cube_texture`.
    ///
    /// Three draws each face onto a 2-D canvas and reads it back with
    /// `getImageData()`, so it sees 8-bit RGBA whatever the image was; the
    /// port reads the decoded face bytes the loader already holds, which are
    /// the same 8-bit values for the PNG and JPEG cubes `CubeTextureLoader`
    /// makes. Each texel is divided by 255 and, for an sRGB cube, converted
    /// to linear before it is projected.
    ///
    /// # Errors
    ///
    /// [`Error::UnsupportedTextureType`] for a cube that is not
    /// `UnsignedByteType` — an `HDRCubeTextureLoader` cube, which three's
    /// canvas could not draw either — and [`Error::UnsupportedFormat`] for
    /// a cube without six faces of pixels (a render target's, which
    /// [`from_cube_render_target`](Self::from_cube_render_target) reads).
    pub fn from_cube_texture(cube_texture: &CubeTexture) -> Result<Node, Error> {
        let texture_type = cube_texture.texture_type();
        if texture_type != TextureType::UnsignedByte {
            return Err(Error::UnsupportedTextureType {
                what: "LightProbeGenerator.fromCubeTexture",
                texture_type,
            });
        }
        let color_space = cube_texture.color_space();
        let inner = cube_texture.borrow();

        let mut projection = Projection::new();
        for face_index in 0..6 {
            let image = inner.images.get(face_index).filter(|image| {
                !image.data.is_empty()
                    && image.data.len() == (image.width * image.height * 4) as usize
            });
            let Some(image) = image else {
                return Err(Error::UnsupportedFormat {
                    what: "LightProbeGenerator.fromCubeTexture face",
                    value: format!("face {face_index} has no RGBA8 pixels"),
                });
            };

            let image_width = image.width as usize; // assumed to be square
            let pixel_size = 2.0 / image_width as f64;

            for (pixel_index, texel) in image.data.as_chunks::<4>().0.iter().enumerate() {
                // pixel color
                let mut color = Color::new(
                    texel[0] as f64 / 255.0,
                    texel[1] as f64 / 255.0,
                    texel[2] as f64 / 255.0,
                );

                // convert to linear color space
                convert_color_to_linear(&mut color, color_space);

                // pixel coordinate on unit cube
                let col = -1.0 + ((pixel_index % image_width) as f64 + 0.5) * pixel_size;
                let row = 1.0 - ((pixel_index / image_width) as f64 + 0.5) * pixel_size;

                let coord = match face_index {
                    0 => Vector3::new(-1.0, row, -col),
                    1 => Vector3::new(1.0, row, col),
                    2 => Vector3::new(-col, 1.0, -row),
                    3 => Vector3::new(-col, -1.0, row),
                    4 => Vector3::new(-col, row, 1.0),
                    _ => Vector3::new(col, row, -1.0),
                };

                projection.add(coord, color);
            }
        }

        Ok(LightProbe::new(projection.finish(), 1.0))
    }

    /// `LightProbeGenerator.fromCubeRenderTarget( renderer, cubeRenderTarget
    /// )` — the same projection over a cube the GPU rendered, read back one
    /// face at a time (`readRenderTargetPixelsAsync( target, 0, 0, w, w, 0,
    /// faceIndex )`; blocking here, as every readback in the port is).
    ///
    /// The face table is three's WebGPU one (`flip = 1`): a render target's
    /// faces come back in the orientation the cube camera drew them, which is
    /// not the orientation a `CubeTexture`'s images are stored in, hence the
    /// different signs from [`from_cube_texture`](Self::from_cube_texture).
    ///
    /// `UnsignedByteType` texels are divided by 255; `HalfFloatType` ones are
    /// decoded exactly. Either is converted to linear when the target's
    /// colour space is sRGB.
    ///
    /// # Errors
    ///
    /// Whatever the readback reports: [`Error::Readback`] for a format it
    /// cannot read (a `FloatType` target, which the port's cube targets never
    /// are).
    pub fn from_cube_render_target(
        renderer: &mut Renderer,
        cube_render_target: &CubeTexture,
    ) -> Result<Node, Error> {
        let flip = 1.0; // `renderer.coordinateSystem` is WebGPU's
        let texture_type = cube_render_target.texture_type();
        let color_space = cube_render_target.color_space();

        let mut projection = Projection::new();
        for face_index in 0..6u32 {
            let (image_width, _, data): (u32, u32, Vec<f32>) = match texture_type {
                TextureType::HalfFloat => {
                    renderer.read_cube_pixels_rgba16f(cube_render_target, face_index, 0)?
                }
                _ => {
                    let (w, h, bytes) =
                        renderer.read_cube_pixels_rgba8(cube_render_target, face_index, 0)?;
                    (w, h, bytes.iter().map(|&b| b as f32 / 255.0).collect())
                }
            };
            let image_width = image_width as usize; // assumed to be square
            let pixel_size = 2.0 / image_width as f64;

            for (pixel_index, texel) in data.as_chunks::<4>().0.iter().enumerate() {
                // pixel color
                let mut color = Color::new(texel[0] as f64, texel[1] as f64, texel[2] as f64);

                // convert to linear color space
                convert_color_to_linear(&mut color, color_space);

                // pixel coordinate on unit cube
                let col = (1.0 - ((pixel_index % image_width) as f64 + 0.5) * pixel_size) * flip;
                let row = 1.0 - ((pixel_index / image_width) as f64 + 0.5) * pixel_size;

                let coord = match face_index {
                    0 => Vector3::new(-flip, row, col * flip),
                    1 => Vector3::new(flip, row, -col * flip),
                    2 => Vector3::new(col, 1.0, -row),
                    3 => Vector3::new(col, -1.0, row),
                    4 => Vector3::new(col, row, 1.0),
                    _ => Vector3::new(-col, row, -1.0),
                };

                projection.add(coord, color);
            }
        }

        Ok(LightProbe::new(projection.finish(), 1.0))
    }
}
