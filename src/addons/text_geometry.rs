//! Port of `three.js/examples/jsm/geometries/TextGeometry.js`.

use crate::core::BufferGeometry;
use crate::geometries::{extrude_geometry, ExtrudeGeometryOptions};
use crate::loaders::{Font, TextDirection};

/// `TextGeometry`'s parameters: `ExtrudeGeometry`'s options plus the font,
/// `size` and `direction`.
///
/// `TextGeometry` fills in its own defaults for four of the extrude options
/// *only when they are undefined* — `depth` 50, `bevelThickness` 10,
/// `bevelSize` 8, `bevelEnabled` false — so [`TextGeometryOptions::new`]
/// starts from those rather than from `ExtrudeGeometry`'s.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TextGeometryOptions {
    /// `parameters.size`, `undefined` → `generateShapes`' default of 100.
    pub size: f64,
    /// `parameters.direction` — `"ltr"` or `"rtl"`.
    pub direction: TextDirection,
    /// The underlying `ExtrudeGeometry` options, with `TextGeometry`'s own
    /// defaults for `depth`, `bevel_thickness`, `bevel_size` and
    /// `bevel_enabled` — see the struct docs.
    pub extrude: ExtrudeGeometryOptions,
}

impl Default for TextGeometryOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl TextGeometryOptions {
    /// `TextGeometry`'s own defaults — see the struct docs.
    pub fn new() -> Self {
        Self {
            size: 100.0,
            direction: TextDirection::Ltr,
            extrude: ExtrudeGeometryOptions {
                depth: 50.0,
                bevel_thickness: 10.0,
                bevel_size: Some(8.0),
                bevel_enabled: false,
                ..ExtrudeGeometryOptions::default()
            },
        }
    }
}

/// `new TextGeometry( text, { font, ...parameters } )`.
pub fn text_geometry(text: &str, font: &Font, parameters: &TextGeometryOptions) -> BufferGeometry {
    let shapes = font.generate_shapes(text, parameters.size, parameters.direction);
    extrude_geometry(&shapes, &parameters.extrude)
}
