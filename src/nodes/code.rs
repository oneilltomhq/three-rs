//! `wgslFn()` — `CodeNode` / `FunctionNode` / `FunctionCallNode` and the WGSL
//! declaration parser behind them (`WGSLNodeFunction.js`).
//!
//! A `wgslFn` is a string of hand-written WGSL that the node system does not
//! understand: it only parses far enough to learn the function's name, its
//! parameter names and types and its return type, and then emits the source
//! *verbatim* into `// codes` and calls it. Everything after the declaration —
//! the body, its indentation, the whitespace a JS template literal leaves
//! after the closing brace — is copied through untouched, which is why the
//! generated shader keeps the example file's six-tab indentation.

use std::rc::Rc;

use super::node::Type;

/// What one declared parameter of a `wgslFn` binds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// An ordinary value: the argument is generated and padded to this type.
    Value(Type),
    /// `texture_2d<f32>` and friends: the argument must be a texture node, and
    /// what is passed is the binding itself, not a sample of it.
    Texture,
    /// `sampler`: the argument is again a texture node, and what is passed is
    /// that binding's companion sampler.
    Sampler,
}

/// A parsed `wgslFn` — `CodeNode` plus the `WGSLNodeFunction` it holds.
#[derive(Debug)]
pub struct CodeDef {
    /// The name in the declaration, which is also the name it is called by.
    pub name: String,
    /// `WGSLNodeFunction.getCode()`: the rebuilt declaration line followed by
    /// the source's own block, verbatim.
    pub code: String,
    /// The declared parameters, in declaration order.
    pub params: Vec<(String, ParamKind)>,
    /// The return type. `void` is not modelled: nothing in the rung uses it.
    pub ret: Type,
    /// `CodeNode.includes` — other `wgslFn`s this one calls. Three builds them
    /// first, so they land in `// codes` before their caller.
    pub includes: Vec<Rc<CodeDef>>,
}

/// `wgslFn( source, includes )`.
///
/// # Panics
///
/// If the source does not start with a WGSL function declaration, or declares
/// a type the port does not model. Three throws on the first and silently
/// produces `undefined` on the second; a panic at setup time is the port's
/// equivalent of a thrown `Error`, and both happen long before any pixels.
pub fn wgsl_fn(source: &str, includes: Vec<Rc<CodeDef>>) -> Rc<CodeDef> {
    let parsed = parse(source);
    let output = if parsed.output_type == "void" {
        String::new()
    } else {
        format!("-> {}", parsed.output_type)
    };
    // `getCode()`: `fn <name> ( <inputsCode.trim()> ) <outputType>` + blockCode.
    // The space before the parenthesis and the one after `)` are three's, and
    // the block keeps the source's own leading space and indentation.
    let code = format!(
        "fn {} ( {} ) {}{}",
        parsed.name,
        parsed.inputs_code.trim(),
        output,
        parsed.block_code
    );
    Rc::new(CodeDef {
        name: parsed.name,
        code,
        params: parsed.params,
        ret: param_type(&parsed.output_type)
            .unwrap_or_else(|| panic!("three-rs: wgslFn returns an unmodelled type")),
        includes,
    })
}

struct Parsed {
    name: String,
    inputs_code: String,
    block_code: String,
    output_type: String,
    params: Vec<(String, ParamKind)>,
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The Rust of `declarationRegexp`. Three's regex is
/// `^[fn]*\s*([a-z_0-9]+)?\s*\(([\s\S]*?)\)\s*[\-\>]*\s*([a-z_0-9]+(?:<[\s\S]+?>)?)`
/// applied to the source after its leading comments and whitespace have been
/// stripped; the non-greedy groups mean "up to the first `)`" and "up to the
/// first `>`", which is what the hand-written scanner below does.
fn parse(source: &str) -> Parsed {
    let source = strip_leading(source);
    let bytes: Vec<char> = source.chars().collect();
    let mut i = 0;
    let at = |i: usize| bytes.get(i).copied().unwrap_or('\0');

    // `[fn]*` is a character class, not the word `fn`.
    while matches!(at(i), 'f' | 'n') {
        i += 1;
    }
    while at(i).is_whitespace() {
        i += 1;
    }
    let name_start = i;
    while is_ident(at(i)) {
        i += 1;
    }
    let name: String = bytes[name_start..i].iter().collect();
    while at(i).is_whitespace() {
        i += 1;
    }
    assert_eq!(at(i), '(', "three-rs: wgslFn source is not a WGSL function");
    i += 1;
    let inputs_start = i;
    while at(i) != ')' {
        assert!(i < bytes.len(), "three-rs: wgslFn declaration has no `)`");
        i += 1;
    }
    let inputs_code: String = bytes[inputs_start..i].iter().collect();
    i += 1;
    while at(i).is_whitespace() {
        i += 1;
    }
    while matches!(at(i), '-' | '>') {
        i += 1;
    }
    while at(i).is_whitespace() {
        i += 1;
    }
    let out_start = i;
    while is_ident(at(i)) {
        i += 1;
    }
    if at(i) == '<' {
        while at(i) != '>' {
            assert!(i < bytes.len(), "three-rs: wgslFn return type has no `>`");
            i += 1;
        }
        i += 1;
    }
    let output_type: String = bytes[out_start..i].iter().collect();
    let block_code: String = bytes[i..].iter().collect();

    Parsed {
        params: parse_params(&inputs_code),
        name,
        inputs_code,
        block_code,
        output_type,
    }
}

/// `source.replace( /^(?:\s*\/\/[^\r\n]*|\s*\/\*[\s\S]*?\*\/|\s*)+/, '' )`.
fn strip_leading(source: &str) -> &str {
    let mut rest = source.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after
                .find(['\r', '\n'])
                .map_or("", |at| &after[at..])
                .trim_start();
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after
                .find("*/")
                .map_or("", |at| &after[at + 2..])
                .trim_start();
        } else {
            return rest;
        }
    }
}

/// `propertiesRegexp` — every `name : type` pair in the declaration's inputs.
fn parse_params(inputs_code: &str) -> Vec<(String, ParamKind)> {
    let chars: Vec<char> = inputs_code.chars().collect();
    let at = |i: usize| chars.get(i).copied().unwrap_or('\0');
    let mut params = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !is_ident(at(i)) {
            i += 1;
            continue;
        }
        let start = i;
        while is_ident(at(i)) {
            i += 1;
        }
        let name: String = chars[start..i].iter().collect();
        let mut j = i;
        while at(j).is_whitespace() {
            j += 1;
        }
        if at(j) != ':' {
            // Not a `name : type` pair; the regex would skip past it too.
            continue;
        }
        j += 1;
        while at(j).is_whitespace() {
            j += 1;
        }
        let type_start = j;
        while is_ident(at(j)) {
            j += 1;
        }
        if at(j) == '<' {
            while j < chars.len() && at(j) != '>' {
                j += 1;
            }
            j += 1;
        }
        let ty: String = chars[type_start..j].iter().collect();
        params.push((name, param_kind(&ty)));
        i = j;
    }
    params
}

fn param_kind(ty: &str) -> ParamKind {
    if ty == "sampler" {
        ParamKind::Sampler
    } else if ty.starts_with("texture") {
        ParamKind::Texture
    } else {
        ParamKind::Value(
            param_type(ty).unwrap_or_else(|| panic!("three-rs: wgslFn parameter type `{ty}`")),
        )
    }
}

/// `wgslTypeLib`, as far as the port's [`Type`] reaches.
fn param_type(ty: &str) -> Option<Type> {
    Some(match ty {
        "f32" => Type::F32,
        "i32" => Type::I32,
        "u32" => Type::U32,
        "bool" => Type::Bool,
        "vec2<f32>" | "vec2f" => Type::Vec2,
        "vec3<f32>" | "vec3f" => Type::Vec3,
        "vec4<f32>" | "vec4f" => Type::Vec4,
        "vec2<u32>" | "vec2u" => Type::UVec2,
        "vec2<i32>" | "vec2i" => Type::IVec2,
        "vec3<u32>" | "vec3u" => Type::UVec3,
        "vec3<bool>" | "vec3b" => Type::BVec3,
        "mat2x2<f32>" | "mat2x2f" => Type::Mat2,
        "mat3x3<f32>" | "mat3x3f" => Type::Mat3,
        "mat4x4<f32>" | "mat4x4f" => Type::Mat4,
        _ => return None,
    })
}
