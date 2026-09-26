//! Port of `three.js/examples/jsm/loaders/FontLoader.js` — `FontLoader` and
//! `Font`, the typeface.json reader [`crate::addons::text_geometry`] builds
//! its shapes from.
//!
//! `load()` reads from the filesystem instead of the examples web server; the
//! JSON it parses is byte-identical to what the page fetches.

use std::path::Path;

use crate::error::Error;
use crate::extras::{Shape, ShapePath};

/// `FontLoader`.
pub struct FontLoader;

impl Default for FontLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl FontLoader {
    pub fn new() -> Self {
        Self
    }

    /// `loader.load( url, onLoad )` — synchronous here, since the harness
    /// renders a single frame once loading has settled.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Font, Error> {
        let path = path.as_ref();
        let text = crate::io::read_to_string(path)?;
        let json: serde_json::Value =
            serde_json::from_str(&text).map_err(|source| Error::Json {
                path: Some(path.to_path_buf()),
                source,
            })?;
        Ok(self.parse(json))
    }

    /// `FontLoader.parse( json )`.
    pub fn parse(&self, json: serde_json::Value) -> Font {
        Font::new(json)
    }
}

/// `Font` — the parsed typeface.json, kept as the JSON value `this.data` is.
#[derive(Clone, Debug)]
pub struct Font {
    pub data: serde_json::Value,
}

/// `generateShapes`' `direction`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextDirection {
    #[default]
    Ltr,
    Rtl,
    Tb,
}

impl Font {
    pub fn new(data: serde_json::Value) -> Self {
        Self { data }
    }

    /// `font.generateShapes( text, size = 100, direction = 'ltr' )`.
    pub fn generate_shapes(&self, text: &str, size: f64, direction: TextDirection) -> Vec<Shape> {
        let mut shapes = Vec::new();
        for path in create_paths(text, size, &self.data, direction) {
            shapes.extend(path.to_shapes());
        }
        shapes
    }
}

/// A JSON number, or `NaN` for anything else — what `undefined` turns into
/// under JS arithmetic.
fn num(value: &serde_json::Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn create_paths(
    text: &str,
    size: f64,
    data: &serde_json::Value,
    direction: TextDirection,
) -> Vec<ShapePath> {
    // `Array.from( text )` iterates code points, as `chars()` does.
    let mut chars: Vec<char> = text.chars().collect();
    let scale = size / num(&data["resolution"]);
    let line_height = (num(&data["boundingBox"]["yMax"]) - num(&data["boundingBox"]["yMin"])
        + num(&data["underlineThickness"]))
        * scale;

    let mut paths = Vec::new();

    let mut offset_x = 0.0;
    let mut offset_y = 0.0;

    if direction == TextDirection::Rtl || direction == TextDirection::Tb {
        chars.reverse();
    }

    for ch in chars {
        if ch == '\n' {
            offset_x = 0.0;
            offset_y -= line_height;
        } else {
            // `createPath` logs and returns `undefined` for a glyph the font
            // lacks when it has no '?' either, and the caller then throws on
            // `ret.offsetX`; the port skips the character instead.
            let Some((advance, path)) = create_path(ch, scale, offset_x, offset_y, data) else {
                continue;
            };

            if direction == TextDirection::Tb {
                offset_x = 0.0;
                offset_y += num(&data["ascender"]) * scale;
            } else {
                offset_x += advance;
            }

            paths.push(path);
        }
    }

    paths
}

fn create_path(
    ch: char,
    scale: f64,
    offset_x: f64,
    offset_y: f64,
    data: &serde_json::Value,
) -> Option<(f64, ShapePath)> {
    let glyphs = &data["glyphs"];
    let mut key = [0u8; 4];
    let glyph = glyphs
        .get(&*ch.encode_utf8(&mut key))
        .or_else(|| glyphs.get("?"))?;

    let mut path = ShapePath::new();

    // `if ( glyph.o )`: an empty outline string is falsy too.
    if let Some(o) = glyph.get("o").and_then(|o| o.as_str()) {
        if !o.is_empty() {
            let outline: Vec<&str> = o.split(' ').collect();
            // `outline[ i ++ ] * scale`: the string coerces with `Number()`,
            // which for these plain decimals is `str::parse`.
            let mut i = 0;
            let next = |i: &mut usize, offset: f64| {
                let v = outline
                    .get(*i)
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN);
                *i += 1;
                v * scale + offset
            };
            while i < outline.len() {
                let action = outline[i];
                i += 1;
                match action {
                    "m" => {
                        let x = next(&mut i, offset_x);
                        let y = next(&mut i, offset_y);
                        path.move_to(x, y);
                    }
                    "l" => {
                        let x = next(&mut i, offset_x);
                        let y = next(&mut i, offset_y);
                        path.line_to(x, y);
                    }
                    "q" => {
                        let cpx = next(&mut i, offset_x);
                        let cpy = next(&mut i, offset_y);
                        let cpx1 = next(&mut i, offset_x);
                        let cpy1 = next(&mut i, offset_y);
                        path.quadratic_curve_to(cpx1, cpy1, cpx, cpy);
                    }
                    "b" => {
                        let cpx = next(&mut i, offset_x);
                        let cpy = next(&mut i, offset_y);
                        let cpx1 = next(&mut i, offset_x);
                        let cpy1 = next(&mut i, offset_y);
                        let cpx2 = next(&mut i, offset_x);
                        let cpy2 = next(&mut i, offset_y);
                        path.bezier_curve_to(cpx1, cpy1, cpx2, cpy2, cpx, cpy);
                    }
                    _ => {}
                }
            }
        }
    }

    Some((num(&glyph["ha"]) * scale, path))
}
