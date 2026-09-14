//! Port of `three.js/src/animation/PropertyBinding.js`.
//!
//! Only the *parsing* half of `PropertyBinding` is ported here:
//! [`sanitize_node_name`] and [`parse_track_name`].
//!
//! Deferred (needs a live `Object3D` / scene graph, owned elsewhere):
//! `PropertyBinding` itself (`bind` / `unbind` / `getValue` / `setValue` and the
//! `BindingType` / `Versioning` state machine), `PropertyBinding.create`,
//! `PropertyBinding.findNode` (walks `root.children` and `root.skeleton`), and
//! the `Composite` class (needs `AnimationObjectGroup`).

// Characters [].:/ are reserved for track binding syntax.
// Three.js: `const _RESERVED_CHARS_RE = '\\[\\]\\.:\\/';`

/// The reserved character class `[\[\]\.:\/]`.
fn is_reserved(c: char) -> bool {
    matches!(c, '[' | ']' | '.' | ':' | '/')
}

/// Three's `_wordChar`: `[^\[\]\.:\/]`.
fn is_word_char(c: char) -> bool {
    !is_reserved(c)
}

/// Three's `_wordCharOrDot`: `[^\[\]:\/]` (the `\.` is removed from the class).
fn is_word_char_or_dot(c: char) -> bool {
    !matches!(c, '[' | ']' | ':' | '/')
}

/// JavaScript's `.` (any character except a line terminator), used by the
/// `(?:\[(.+)\])?` accessor groups.
fn is_dot_any(c: char) -> bool {
    !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// JavaScript's `\s` character class.
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{0009}' // \t
            | '\u{000A}' // \n
            | '\u{000B}' // \v
            | '\u{000C}' // \f
            | '\u{000D}' // \r
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// Three's `_supportedObjectNames`.
const SUPPORTED_OBJECT_NAMES: [&str; 4] = ["material", "materials", "bones", "map"];

/// The result of [`parse_track_name`]; mirrors the object returned by
/// `PropertyBinding.parseTrackName()`.
///
/// Note the indices are strings, not numbers: Three leaves them as the raw
/// captured text (e.g. `.bone[Armature.DEF_cog].position` yields the object
/// index `"Armature.DEF_cog"`), and resolves them against the target object
/// later.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedTrackName {
    /// `matches[ 2 ]`, possibly rewritten by the `lastDot` fixup.
    pub node_name: Option<String>,
    /// `matches[ 3 ]`, possibly filled in by the `lastDot` fixup.
    pub object_name: Option<String>,
    /// `matches[ 4 ]`.
    pub object_index: Option<String>,
    /// `matches[ 5 ]` (required).
    pub property_name: String,
    /// `matches[ 6 ]`.
    pub property_index: Option<String>,
}

/// The two `throw new Error(...)` cases of `PropertyBinding.parseTrackName()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseTrackNameError {
    /// `_trackRe` did not match at all.
    CannotParse(String),
    /// `propertyName` came back empty.
    NoPropertyName(String),
}

impl std::fmt::Display for ParseTrackNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CannotParse(track_name) => write!(
                f,
                "THREE.PropertyBinding: Cannot parse trackName: {track_name}"
            ),
            Self::NoPropertyName(track_name) => write!(
                f,
                "THREE.PropertyBinding: can not parse propertyName from trackName: {track_name}"
            ),
        }
    }
}

impl std::error::Error for ParseTrackNameError {}

/// Replaces spaces with underscores and removes unsupported characters from
/// node names, to ensure compatibility with [`parse_track_name`].
///
/// Port of `name.replace( /\s/g, '_' ).replace( _reservedRe, '' )`.
pub fn sanitize_node_name(name: &str) -> String {
    name.chars()
        .map(|c| if is_js_space(c) { '_' } else { c })
        .filter(|&c| !is_reserved(c))
        .collect()
}

/// One `\.(WC+)(?:\[(.+)\])?` segment, as captured by the object / property
/// parts of `_trackRe`. `end` is the position just past the segment.
struct Segment {
    end: usize,
    name: (usize, usize),
    index: Option<(usize, usize)>,
}

/// Enumerates the ways `\.(WC+)(?:\[(.+)\])?` can match at `pos`, in
/// JavaScript's backtracking order (greedy `WC+` longest first; the bracketed
/// accessor attempted before being skipped; greedy `.+` longest first).
fn segment_candidates(s: &[char], pos: usize) -> Vec<Segment> {
    let mut out = Vec::new();

    if pos >= s.len() || s[pos] != '.' {
        return out;
    }

    let begin = pos + 1;
    let mut max_len = 0;
    while begin + max_len < s.len() && is_word_char(s[begin + max_len]) {
        max_len += 1;
    }

    for len in (1..=max_len).rev() {
        let after_name = begin + len;

        // `(?:\[(.+)\])?` — greedy, so try to match the accessor first.
        if after_name < s.len() && s[after_name] == '[' {
            let index_begin = after_name + 1;
            let mut max_index_len = 0;
            while index_begin + max_index_len < s.len()
                && is_dot_any(s[index_begin + max_index_len])
            {
                max_index_len += 1;
            }

            for index_len in (1..=max_index_len).rev() {
                let index_end = index_begin + index_len;
                if index_end < s.len() && s[index_end] == ']' {
                    out.push(Segment {
                        end: index_end + 1,
                        name: (begin, after_name),
                        index: Some((index_begin, index_end)),
                    });
                }
            }
        }

        // ...then without it.
        out.push(Segment {
            end: after_name,
            name: (begin, after_name),
            index: None,
        });
    }

    out
}

/// The captures of `_trackRe` (groups 2..=6; group 1, `directoryName`, is
/// unused by Three as well).
struct TrackCaptures {
    node_name: Option<(usize, usize)>,
    object_name: Option<(usize, usize)>,
    object_index: Option<(usize, usize)>,
    property_name: (usize, usize),
    property_index: Option<(usize, usize)>,
}

/// Hand-rolled equivalent of
/// `^((?:WC+[\/:])*)(WCOD+)?(?:\.(WC+)(?:\[(.+)\])?)?\.(WC+)(?:\[(.+)\])?$`,
/// matching JavaScript's leftmost-greedy backtracking order. (The crate has no
/// regex dependency, so the character classes are spelled out by hand above.)
fn exec_track_re(s: &[char]) -> Option<TrackCaptures> {
    // `((?:WC+[\/:])*)` — each repetition is deterministic: a maximal run of
    // word characters (which can never contain `/` or `:`) followed by one of
    // them. Backtracking can only drop whole repetitions, so collect the
    // reachable end positions and try them longest-first.
    let mut directory_ends = vec![0usize];
    let mut p = 0;
    loop {
        let mut q = p;
        while q < s.len() && is_word_char(s[q]) {
            q += 1;
        }
        if q > p && q < s.len() && (s[q] == '/' || s[q] == ':') {
            p = q + 1;
            directory_ends.push(p);
        } else {
            break;
        }
    }

    for &start in directory_ends.iter().rev() {
        // `(WCOD+)?` — greedy, longest first, then the group not participating
        // at all (JS leaves it `undefined`).
        let mut max_len = 0;
        while start + max_len < s.len() && is_word_char_or_dot(s[start + max_len]) {
            max_len += 1;
        }

        let node_candidates = (1..=max_len)
            .rev()
            .map(|len| Some((start, start + len)))
            .chain(std::iter::once(None));

        for node_name in node_candidates {
            let after_node = node_name.map_or(start, |(_, end)| end);

            // `(?:\.(WC+)(?:\[(.+)\])?)?` — greedy, so try the object group
            // first, then skip it.
            for object in segment_candidates(s, after_node) {
                for property in segment_candidates(s, object.end) {
                    if property.end == s.len() {
                        return Some(TrackCaptures {
                            node_name,
                            object_name: Some(object.name),
                            object_index: object.index,
                            property_name: property.name,
                            property_index: property.index,
                        });
                    }
                }
            }

            for property in segment_candidates(s, after_node) {
                if property.end == s.len() {
                    return Some(TrackCaptures {
                        node_name,
                        object_name: None,
                        object_index: None,
                        property_name: property.name,
                        property_index: property.index,
                    });
                }
            }
        }
    }

    None
}

/// Parses the given track name (an object path to an animated property) and
/// returns information about the path. Matches strings in the following forms:
///
/// - `nodeName.property`
/// - `nodeName.property[accessor]`
/// - `nodeName.material.property[accessor]`
/// - `uuid.property[accessor]`
/// - `uuid.objectName[objectIndex].propertyName[propertyIndex]`
/// - `parentName/nodeName.property`
/// - `parentName/parentName/nodeName.property[index]`
/// - `.bone[Armature.DEF_cog].position`
/// - `scene:helium_balloon_model:helium_balloon_model.position`
///
/// Port of `PropertyBinding.parseTrackName()`; the `throw`s become
/// [`ParseTrackNameError`].
pub fn parse_track_name(track_name: &str) -> Result<ParsedTrackName, ParseTrackNameError> {
    let chars: Vec<char> = track_name.chars().collect();

    let matches = exec_track_re(&chars)
        .ok_or_else(|| ParseTrackNameError::CannotParse(track_name.to_string()))?;

    let slice = |(begin, end): (usize, usize)| chars[begin..end].iter().collect::<String>();

    let mut results = ParsedTrackName {
        // directory_name: matches[ 1 ], // (tschw) currently unused
        node_name: matches.node_name.map(slice),
        object_name: matches.object_name.map(slice),
        object_index: matches.object_index.map(slice),
        property_name: slice(matches.property_name),
        property_index: matches.property_index.map(slice),
    };

    if let Some(node_name) = results.node_name.clone() {
        if let Some(last_dot) = node_name.rfind('.') {
            let object_name = &node_name[last_dot + 1..];

            // Object names must be checked against an allowlist. Otherwise, there
            // is no way to parse 'foo.bar.baz': 'baz' must be a property, but
            // 'bar' could be the objectName, or part of a nodeName (which can
            // include '.' characters).
            if SUPPORTED_OBJECT_NAMES.contains(&object_name) {
                results.object_name = Some(object_name.to_string());
                results.node_name = Some(node_name[..last_dot].to_string());
            }
        }
    }

    if results.property_name.is_empty() {
        return Err(ParseTrackNameError::NoPropertyName(track_name.to_string()));
    }

    Ok(results)
}
