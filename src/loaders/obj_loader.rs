//! Port of `three.js/examples/jsm/loaders/OBJLoader.js` — the Wavefront OBJ
//! mesh format, as far as `webgpu_postprocessing_outline` loads
//! `models/obj/tree.obj` with it, plus the parts of the face grammar every OBJ
//! file uses.
//!
//! Ported: `ParserState` and `parse()` for `v` (with the optional
//! `v x y z r g b` vertex colour, converted from sRGB as `Color.setRGB( …,
//! SRGBColorSpace )` does), `vn`, `vt`, `f` (triangles and n-gons, fanned
//! from the first vertex; `v`, `v/vt`, `v//vn` and `v/vt/vn`, negative
//! indices counting back from the end), `o` / `g`, `s`, `usemtl` (one geometry
//! group and one material per run, `Mesh.material` an array when there is more
//! than one), `mtllib` (recorded in [`Obj::material_libraries`]), `usemap` and
//! comments, CRLF line ends and `\` continuations. Upstream's quirks are kept:
//!
//! - every face vertex is its own vertex — the geometry is never indexed;
//! - a face whose first vertex has no normal index gets the face normal on all
//!   three corners, one whose first vertex has no uv index gets `( 0, 0 )`
//!   placeholders, and the `uv` attribute exists only once some face had uv
//!   indices;
//! - `parseFloat` / `parseInt` read the longest numeric prefix of a token and
//!   anything they cannot read, or an index that falls outside the arrays, is
//!   NaN in the attribute — `undefined` in a JS array becomes NaN in a
//!   `Float32BufferAttribute`;
//! - a mesh's materials are `MeshPhongMaterial`s with `flatShading` from
//!   `s 0` / `s off` and `vertexColors` when the vertices carried colours.
//!
//! Not ported:
//!
//! - `l` (line) and `p` (point) elements, and the "only vertices, no faces"
//!   point-cloud fallback: both build `LineSegments` / `Points`, and loading a
//!   file with either is an [`Error::UnsupportedFormat`] rather than a silent
//!   skip;
//! - `setMaterials( MTLLoader.MaterialCreator )` — there is no `MTLLoader`, so
//!   `usemtl` names only ever create default materials;
//! - `material.name = sourceMaterial.name`: the port's `Material.name` is a
//!   `&'static str`, so the `usemtl` name is not carried onto the material;
//! - the per-hash material cache: three hands two objects with the same
//!   `usemtl`, smoothing and colour flag one shared material instance, while
//!   here each mesh gets its own (equal) copy;
//! - `console.warn` for `usemap` and unexpected lines, which are skipped
//!   silently;
//! - `Loader`'s `manager`, `path`, `requestHeader` and `withCredentials`, and
//!   the callback-style `load()` — this one is synchronous.
//!
//! `tests/loaders_obj.rs` checks the parse of `tree.obj` and of the face and
//! index grammar.

use std::path::Path;
use std::rc::Rc;

use crate::core::{BufferAttribute, BufferGeometry, Node};
use crate::error::Error;
use crate::materials::MeshBasicNodeMaterial;
use crate::math::{Color, ColorSpace, Vector3};
use crate::objects::{Group, Mesh};

/// `OBJLoader.parse()`'s return value: the `Group` and what three hangs on it.
pub struct Obj {
    /// The `Group` holding one `Mesh` per object (`o` / `g`) that had faces.
    pub group: Node,
    /// `container.materialLibraries`: every `mtllib` reference, in order.
    pub material_libraries: Vec<String>,
}

/// `new OBJLoader()`.
#[derive(Debug, Clone, Default)]
pub struct ObjLoader;

/// A `startMaterial()` record.
#[derive(Debug, Clone)]
struct MaterialRecord {
    index: usize,
    name: String,
    smooth: bool,
    group_start: i64,
    group_end: i64,
    group_count: i64,
    inherited: bool,
}

impl MaterialRecord {
    /// `material.clone( index )`.
    fn clone_at(&self, index: usize) -> Self {
        Self {
            index,
            name: self.name.clone(),
            smooth: self.smooth,
            group_start: 0,
            group_end: -1,
            group_count: -1,
            inherited: false,
        }
    }
}

/// `state.object`.
#[derive(Debug)]
struct ObjectRecord {
    name: String,
    from_declaration: bool,
    vertices: Vec<f64>,
    normals: Vec<f64>,
    colors: Vec<f64>,
    uvs: Vec<f64>,
    has_uv_indices: bool,
    materials: Vec<MaterialRecord>,
    smooth: bool,
}

impl ObjectRecord {
    fn new(name: &str, from_declaration: bool) -> Self {
        Self {
            name: name.to_string(),
            from_declaration,
            vertices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            uvs: Vec::new(),
            has_uv_indices: false,
            materials: Vec::new(),
            smooth: true,
        }
    }

    /// `currentMaterial()`.
    fn current_material(&self) -> Option<&MaterialRecord> {
        self.materials.last()
    }

    /// `startMaterial( name, libraries )`.
    fn start_material(&mut self, name: &str) {
        let previous = self.finalize(false);

        // New usemtl declaration overwrites an inherited material, except if
        // faces were declared after the material, then it must be preserved
        // for proper MultiMaterial continuation.
        if let Some(previous) = previous {
            if previous.inherited || previous.group_count <= 0 {
                self.materials.remove(previous.index);
            }
        }

        let previous = self.materials.last();
        let material = MaterialRecord {
            index: self.materials.len(),
            name: name.to_string(),
            smooth: previous.map_or(self.smooth, |previous| previous.smooth),
            group_start: previous.map_or(0, |previous| previous.group_end),
            group_end: -1,
            group_count: -1,
            inherited: false,
        };
        self.materials.push(material);
    }

    /// `_finalize( end )`: closes the open material's group and returns it as
    /// it was when closed.
    fn finalize(&mut self, end: bool) -> Option<MaterialRecord> {
        let vertex_count = (self.vertices.len() / 3) as i64;
        let last = self.materials.last_mut().map(|last| {
            if last.group_end == -1 {
                last.group_end = vertex_count;
                last.group_count = last.group_end - last.group_start;
                last.inherited = false;
            }
            last.clone()
        });

        // Ignore objects tail materials if no face declarations followed them
        // before a new o/g started.
        if end && self.materials.len() > 1 {
            self.materials.retain(|material| material.group_count > 0);
        }

        // Guarantee at least one empty material, this makes the creation later
        // more straight forward.
        if end && self.materials.is_empty() {
            self.materials.push(MaterialRecord {
                index: 0,
                name: String::new(),
                smooth: self.smooth,
                group_start: 0,
                group_end: -1,
                group_count: -1,
                inherited: false,
            });
        }

        last
    }
}

/// `ParserState()`.
struct ParserState {
    objects: Vec<ObjectRecord>,
    /// `state.vertices` etc.: the file's pools, indexed by the faces.
    vertices: Vec<f64>,
    normals: Vec<f64>,
    /// `undefined` placeholders are `None`.
    colors: Vec<Option<f64>>,
    uvs: Vec<f64>,
    material_libraries: Vec<String>,
}

/// `src[ i ]` for an index that may be NaN, negative or past the end, all of
/// which read `undefined` — NaN once it reaches a `Float32Array`.
fn at(src: &[f64], i: f64) -> f64 {
    if i.fract() == 0.0 && i >= 0.0 && (i as usize) < src.len() {
        src[i as usize]
    } else {
        f64::NAN
    }
}

impl ParserState {
    fn new() -> Self {
        let mut state = Self {
            objects: Vec::new(),
            vertices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            uvs: Vec::new(),
            material_libraries: Vec::new(),
        };
        state.start_object("", false);
        state
    }

    fn object(&mut self) -> &mut ObjectRecord {
        self.objects
            .last_mut()
            .expect("ParserState starts an object")
    }

    /// `startObject( name, fromDeclaration )`.
    fn start_object(&mut self, name: &str, from_declaration: bool) {
        // If the current object (initial from reset) is not from a g/o
        // declaration in the parsed file. We need to use it for the first
        // parsed g/o to keep things in sync.
        if let Some(object) = self.objects.last_mut() {
            if !object.from_declaration {
                object.name = name.to_string();
                object.from_declaration = from_declaration;
                return;
            }
        }

        let previous_material = self
            .objects
            .last()
            .and_then(|object| object.current_material().cloned());

        if let Some(object) = self.objects.last_mut() {
            object.finalize(true);
        }

        let mut object = ObjectRecord::new(name, from_declaration);

        // Inherit previous objects material.
        if let Some(previous) = previous_material.filter(|material| !material.name.is_empty()) {
            let mut declared = previous.clone_at(0);
            declared.inherited = true;
            object.materials.push(declared);
        }

        self.objects.push(object);
    }

    /// `finalize()`.
    fn finalize(&mut self) {
        if let Some(object) = self.objects.last_mut() {
            object.finalize(true);
        }
    }

    /// `parseVertexIndex` / `parseNormalIndex` (`stride` 3) and `parseUVIndex`
    /// (`stride` 2).
    fn parse_index(value: Option<&str>, len: usize, stride: usize) -> f64 {
        let index = value.map_or(f64::NAN, parse_int);
        let index = if index >= 0.0 {
            index - 1.0
        } else {
            index + (len / stride) as f64
        };
        index * stride as f64
    }

    /// `addFace( a, b, c, ua, ub, uc, na, nb, nc )`.
    fn add_face(&mut self, v: [&[&str]; 3]) {
        fn part<'a>(vertex: &[&'a str], i: usize) -> Option<&'a str> {
            vertex.get(i).copied()
        }
        let v_len = self.vertices.len();
        let ia = Self::parse_index(part(v[0], 0), v_len, 3);
        let ib = Self::parse_index(part(v[1], 0), v_len, 3);
        let ic = Self::parse_index(part(v[2], 0), v_len, 3);

        // `addVertex( ia, ib, ic )`.
        for i in [ia, ib, ic] {
            for k in 0..3 {
                let value = at(&self.vertices, i + k as f64);
                self.objects.last_mut().unwrap().vertices.push(value);
            }
        }
        // `addColor( ia, ib, ic )`: only corners whose colour is defined.
        for i in [ia, ib, ic] {
            let defined = |j: f64| {
                (j.fract() == 0.0 && j >= 0.0)
                    .then(|| self.colors.get(j as usize).copied().flatten())
                    .flatten()
            };
            if defined(i).is_some() {
                let rgb: Vec<f64> = (0..3)
                    .map(|k| defined(i + k as f64).unwrap_or(f64::NAN))
                    .collect();
                self.object().colors.extend(rgb);
            }
        }

        // normals
        if part(v[0], 2).is_some_and(|na| !na.is_empty()) {
            let n_len = self.normals.len();
            let ia = Self::parse_index(part(v[0], 2), n_len, 3);
            let ib = Self::parse_index(part(v[1], 2), n_len, 3);
            let ic = Self::parse_index(part(v[2], 2), n_len, 3);
            // `addNormal( ia, ib, ic )`.
            for i in [ia, ib, ic] {
                for k in 0..3 {
                    let value = at(&self.normals, i + k as f64);
                    self.object().normals.push(value);
                }
            }
        } else {
            // `addFaceNormal( ia, ib, ic )`: `( c - b ) × ( a - b )`,
            // normalized, on all three corners.
            let read = |i: f64| {
                Vector3::new(
                    at(&self.vertices, i),
                    at(&self.vertices, i + 1.0),
                    at(&self.vertices, i + 2.0),
                )
            };
            let (a, b, c) = (read(ia), read(ib), read(ic));
            let mut cb = c;
            cb.sub(&b);
            let mut ab = a;
            ab.sub(&b);
            cb.cross(&ab);
            // `normalize()` is `divideScalar( this.length() || 1 )`, and a NaN
            // length is falsy too — a face with a NaN or infinite corner keeps
            // its raw cross product. `Vector3::normalize` only special-cases 0.
            let length = cb.length();
            cb.divide_scalar(if length == 0.0 || length.is_nan() {
                1.0
            } else {
                length
            });
            for _ in 0..3 {
                self.object().normals.extend([cb.x, cb.y, cb.z]);
            }
        }

        // uvs
        if part(v[0], 1).is_some_and(|ua| !ua.is_empty()) {
            let uv_len = self.uvs.len();
            let ia = Self::parse_index(part(v[0], 1), uv_len, 2);
            let ib = Self::parse_index(part(v[1], 1), uv_len, 2);
            let ic = Self::parse_index(part(v[2], 1), uv_len, 2);
            // `addUV( ia, ib, ic )`.
            for i in [ia, ib, ic] {
                for k in 0..2 {
                    let value = at(&self.uvs, i + k as f64);
                    self.object().uvs.push(value);
                }
            }
            self.object().has_uv_indices = true;
        } else {
            // add placeholder values (for inconsistent face definitions)
            self.object().uvs.extend([0.0; 6]);
        }
    }
}

/// ECMAScript `WhiteSpace` and `LineTerminator`: what `\s` and `trimStart()`
/// match.
fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// `string.split( /\s+/ )`: empty pieces at either end are kept.
fn split_whitespace_runs(s: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut chars = s.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if is_js_whitespace(c) {
            pieces.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = chars.peek() {
                if !is_js_whitespace(d) {
                    break;
                }
                end = j + d.len_utf8();
                chars.next();
            }
            start = end;
        }
    }
    pieces.push(&s[start..]);
    pieces
}

/// The length of the longest prefix of `s` made of ASCII digits.
fn digit_run(s: &str) -> usize {
    s.bytes().take_while(u8::is_ascii_digit).count()
}

/// `parseFloat( s )`: leading whitespace skipped, then the longest prefix
/// that is a `StrDecimalLiteral` (`Infinity` included); NaN when there is
/// none.
fn parse_float(s: &str) -> f64 {
    let s = s.trim_start_matches(is_js_whitespace);
    let (sign, body) = match s.as_bytes().first() {
        Some(b'-') => (-1.0, &s[1..]),
        Some(b'+') => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    if body.starts_with("Infinity") {
        return sign * f64::INFINITY;
    }
    let int = digit_run(body);
    let mut end = int;
    let mut frac = 0;
    if body[end..].starts_with('.') {
        frac = digit_run(&body[end + 1..]);
        if int > 0 || frac > 0 {
            end += 1 + frac;
        }
    }
    if int == 0 && frac == 0 {
        return f64::NAN;
    }
    let rest = &body[end..];
    if let Some(exponent) = rest.strip_prefix(['e', 'E']) {
        let digits_from = usize::from(exponent.starts_with(['+', '-']));
        let n = digit_run(&exponent[digits_from..]);
        if n > 0 {
            end += 1 + digits_from + n;
        }
    }
    sign * body[..end].parse::<f64>().unwrap_or(f64::NAN)
}

/// `parseInt( s, 10 )`: leading whitespace and a sign, then the longest run of
/// digits; NaN when there is none.
fn parse_int(s: &str) -> f64 {
    let s = s.trim_start_matches(is_js_whitespace);
    let (sign, body) = match s.as_bytes().first() {
        Some(b'-') => (-1.0, &s[1..]),
        Some(b'+') => (1.0, &s[1..]),
        _ => (1.0, s),
    };
    let n = digit_run(body);
    if n == 0 {
        return f64::NAN;
    }
    sign * body[..n].parse::<f64>().unwrap_or(f64::NAN)
}

/// `/^[og]\s*(.+)?/` on a line, returning the name `( ' ' + result[ 0
/// ].slice( 1 ).trim() ).slice( 1 )` makes of it.
fn object_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix(['o', 'g'])?;
    // `\s*` may cross line terminators, `.` may not: the match runs from the
    // letter through the whitespace after it to the next terminator, and
    // `result[ 0 ].slice( 1 )` is everything after the letter.
    let name_from = rest.len() - rest.trim_start_matches(is_js_whitespace).len();
    let matched = rest[name_from..]
        .find(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map_or(rest, |end| &rest[..name_from + end]);
    Some(matched.trim_matches(is_js_whitespace).to_string())
}

impl ObjLoader {
    /// `new OBJLoader()`.
    pub fn new() -> Self {
        Self
    }

    /// `loader.loadAsync( url )`, synchronously.
    pub fn load<P: AsRef<Path>>(&self, path: P) -> Result<Obj, Error> {
        let bytes = crate::io::read(path.as_ref())?;
        self.parse(&String::from_utf8_lossy(&bytes))
    }

    /// `loader.parse( text )`.
    pub fn parse(&self, text: &str) -> Result<Obj, Error> {
        let mut state = ParserState::new();

        // This is faster than String.split with regex that splits on both
        let text = text.replace("\r\n", "\n");
        // join lines separated by a line continuation character (\)
        let text = text.replace("\\\n", "");

        for line in text.split('\n') {
            let line = line.trim_start_matches(is_js_whitespace);
            let Some(line_first_char) = line.chars().next() else {
                continue;
            };

            // skip comments
            if line_first_char == '#' {
                continue;
            }

            if line_first_char == 'v' {
                let data = split_whitespace_runs(line);
                let float = |i: usize| data.get(i).map_or(f64::NAN, |s| parse_float(s));
                match data[0] {
                    "v" => {
                        state.vertices.extend([float(1), float(2), float(3)]);
                        if data.len() >= 7 {
                            let mut color = Color::default();
                            color.set_rgb(float(4), float(5), float(6), ColorSpace::Srgb);
                            state
                                .colors
                                .extend([Some(color.r), Some(color.g), Some(color.b)]);
                        } else {
                            // if no colors are defined, add placeholders so
                            // color and vertex indices match
                            state.colors.extend([None; 3]);
                        }
                    }
                    "vn" => state.normals.extend([float(1), float(2), float(3)]),
                    "vt" => state.uvs.extend([float(1), float(2)]),
                    _ => {}
                }
            } else if line_first_char == 'f' {
                let line_data = line[1..].trim_matches(is_js_whitespace);
                let face_vertices: Vec<Vec<&str>> = split_whitespace_runs(line_data)
                    .into_iter()
                    .filter(|vertex| !vertex.is_empty())
                    .map(|vertex| vertex.split('/').collect())
                    .collect();

                // Draw an edge between the first vertex and all subsequent
                // vertices to form an n-gon
                for j in 1..face_vertices.len().saturating_sub(1) {
                    state.add_face([&face_vertices[0], &face_vertices[j], &face_vertices[j + 1]]);
                }
            } else if line_first_char == 'l' || line_first_char == 'p' {
                return Err(Error::UnsupportedFormat {
                    what: "OBJ element",
                    value: format!(
                        "'{line_first_char}' ({}; only faces are ported)",
                        if line_first_char == 'l' {
                            "LineSegments"
                        } else {
                            "Points"
                        }
                    ),
                });
            } else if let Some(name) = object_name(line) {
                // o object_name
                // or
                // g group_name
                state.start_object(&name, true);
            } else if let Some(name) = line.strip_prefix("usemtl ") {
                // material
                state
                    .object()
                    .start_material(name.trim_matches(is_js_whitespace));
            } else if let Some(library) = line.strip_prefix("mtllib ") {
                // mtl file
                state
                    .material_libraries
                    .push(library.trim_matches(is_js_whitespace).to_string());
            } else if line.starts_with("usemap ") {
                // the line is parsed but ignored since the loader assumes
                // textures are defined MTL files
            } else if line_first_char == 's' {
                let result: Vec<&str> = line.split(' ').collect();
                let smooth = match result.get(1) {
                    Some(value) => {
                        let value = value.trim_matches(is_js_whitespace).to_lowercase();
                        value != "0" && value != "off"
                    }
                    // ZBrush can produce "s" lines #11707
                    None => true,
                };
                let object = state.object();
                object.smooth = smooth;
                if let Some(material) = object.materials.last_mut() {
                    material.smooth = smooth;
                }
            }
            // Anything else — `'\0'` included — is skipped, where three
            // `console.warn`s an unexpected line.
        }

        state.finalize();

        let container = Group::new();

        let has_primitives = !(state.objects.len() == 1 && state.objects[0].vertices.is_empty());
        if !has_primitives {
            if !state.vertices.is_empty() {
                return Err(Error::UnsupportedFormat {
                    what: "OBJ content",
                    value: "vertices with no faces (a Points cloud; only faces are ported)".into(),
                });
            }
            return Ok(Obj {
                group: container,
                material_libraries: state.material_libraries,
            });
        }

        for object in &state.objects {
            // Skip o/g line declarations that did not follow with any faces
            if object.vertices.is_empty() {
                continue;
            }

            let f32s = |values: &[f64]| values.iter().map(|&v| v as f32).collect::<Vec<f32>>();
            let mut geometry = BufferGeometry::new();
            geometry.set_attribute("position", BufferAttribute::new(f32s(&object.vertices), 3));
            if !object.normals.is_empty() {
                geometry.set_attribute("normal", BufferAttribute::new(f32s(&object.normals), 3));
            }
            let has_vertex_colors = !object.colors.is_empty();
            if has_vertex_colors {
                geometry.set_attribute("color", BufferAttribute::new(f32s(&object.colors), 3));
            }
            if object.has_uv_indices {
                geometry.set_attribute("uv", BufferAttribute::new(f32s(&object.uvs), 2));
            }

            // Create materials
            let created: Vec<MeshBasicNodeMaterial> = object
                .materials
                .iter()
                .map(|source| {
                    // `new MeshPhongMaterial()`.
                    let mut material = MeshBasicNodeMaterial::phong(Color::from_hex(0xffffff));
                    material.flat_shading = !source.smooth;
                    material.vertex_colors = has_vertex_colors;
                    material
                })
                .collect();

            // Create mesh
            let mesh = if created.len() > 1 {
                for (mi, source) in object.materials.iter().enumerate() {
                    geometry.add_group(
                        source.group_start.max(0) as usize,
                        source.group_count.max(0) as usize,
                        mi,
                    );
                }
                Mesh::with_materials(Rc::new(geometry), created)
            } else {
                Mesh::new(Rc::new(geometry), created.into_iter().next())
            };
            mesh.borrow_mut().name = object.name.clone();
            container.add(&mesh);
        }

        Ok(Obj {
            group: container,
            material_libraries: state.material_libraries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_float_reads_the_longest_numeric_prefix() {
        assert_eq!(parse_float("0.5"), 0.5);
        assert_eq!(parse_float("-1e2x"), -100.0);
        assert_eq!(parse_float("1e"), 1.0);
        assert_eq!(parse_float(".5"), 0.5);
        assert_eq!(parse_float("5."), 5.0);
        assert_eq!(parse_float("-Infinity"), f64::NEG_INFINITY);
        assert!(parse_float("abc").is_nan());
        assert!(parse_float(".").is_nan());
        assert!(parse_float("").is_nan());
    }

    #[test]
    fn parse_int_reads_a_run_of_digits() {
        assert_eq!(parse_int("12/3"), 12.0);
        assert_eq!(parse_int("-3"), -3.0);
        assert_eq!(parse_int("4.7"), 4.0);
        assert!(parse_int("").is_nan());
        assert!(parse_int("x1").is_nan());
    }

    #[test]
    fn split_keeps_empty_ends_as_js_does() {
        assert_eq!(
            split_whitespace_runs("v 1  2\t3 "),
            vec!["v", "1", "2", "3", ""]
        );
        assert_eq!(split_whitespace_runs(""), vec![""]);
    }
}
