//! The JavaScript semantics `LUTCubeLoader` and `LUT3dlLoader` lean on, spelled
//! out: their regular expressions, `Number()`, and the conversions a
//! `Uint8Array` / `Float32Array` store makes.
//!
//! Both loaders are a handful of regular expressions over the file text. The
//! crate has no regex engine, and none of these needs one: every pattern is
//! anchored to a line or a literal keyword, and each is reproduced here by a
//! scan that accepts exactly the strings the pattern does, including how the
//! `m` flag splits lines (`\n`, `\r`, U+2028 and U+2029 all end one) and how
//! `+` / `*` only ever match the ASCII space they are written after.

use crate::error::Error;

/// The line segments `^` and `$` see under the `m` flag.
pub(super) fn lines(input: &str) -> impl Iterator<Item = &str> {
    input.split(['\n', '\r', '\u{2028}', '\u{2029}'])
}

/// `[\d.e+-]`, the data-point token class.
fn is_point_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, '.' | 'e' | '+' | '-')
}

/// Splits off the longest non-empty prefix of `s` whose chars satisfy `class`.
fn take_run(s: &str, class: impl Fn(char) -> bool) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !class(c)).unwrap_or(s.len());
    (end > 0).then(|| s.split_at(end))
}

/// The three captures of `/^([\d.e+-]+) +([\d.e+-]+) +([\d.e+-]+) *$/` on
/// one line, or `None` when the line does not match.
///
/// A token cannot contain a space, so the greedy groups never backtrack into
/// one: the line must be exactly three tokens separated by runs of spaces,
/// with no leading space and nothing but spaces after the third. A tab
/// anywhere, or a fourth column, and the line is not a data point.
fn data_point(line: &str) -> Option<[&str; 3]> {
    let mut rest = line;
    let mut tokens = [""; 3];
    for (k, token) in tokens.iter_mut().enumerate() {
        let (run, after) = take_run(rest, is_point_char)?;
        *token = run;
        rest = after;
        if k < 2 {
            rest = take_run(rest, |c| c == ' ')?.1;
        }
    }
    rest.chars().all(|c| c == ' ').then_some(tokens)
}

/// Every match of `/^([\d.e+-]+) +([\d.e+-]+) +([\d.e+-]+) *$/gm`, in order,
/// each capture through `Number()`.
pub(super) fn data_points(input: &str) -> impl Iterator<Item = [f64; 3]> + '_ {
    lines(input)
        .filter_map(data_point)
        .map(|tokens| tokens.map(number))
}

/// `Number( s )` for the digit / `.` / `e` / sign strings these patterns
/// capture: a malformed one (`"1e"`, `"."`, `"1.2.3"`) is NaN, as in
/// JavaScript. Rust's float grammar and JavaScript's `StringNumericLiteral`
/// agree on every string made of those characters.
pub(super) fn number(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(f64::NAN)
}

/// The first match of `<keyword> +` followed by what `rest` accepts, scanning
/// every occurrence of `keyword` the way an unanchored `exec()` does.
pub(super) fn find_after_keyword<'a, T>(
    input: &'a str,
    keyword: &str,
    rest: impl Fn(&'a str) -> Option<T>,
) -> Option<T> {
    input.match_indices(keyword).find_map(|(at, _)| {
        let (_, after) = take_run(&input[at + keyword.len()..], |c| c == ' ')?;
        rest(after)
    })
}

/// `( [\d.]+ ) +( [\d.]+ ) +( [\d.]+ )` after a keyword, through `Number()`.
pub(super) fn three_numbers(s: &str) -> Option<[f64; 3]> {
    let is_class = |c: char| c.is_ascii_digit() || c == '.';
    let (a, rest) = take_run(s, is_class)?;
    let (_, rest) = take_run(rest, |c| c == ' ')?;
    let (b, rest) = take_run(rest, is_class)?;
    let (_, rest) = take_run(rest, |c| c == ' ')?;
    let (c, _) = take_run(rest, is_class)?;
    Some([number(a), number(b), number(c)])
}

/// `(\d+)` after a keyword, through `Number()`.
pub(super) fn digits(s: &str) -> Option<f64> {
    take_run(s, |c| c.is_ascii_digit()).map(|(run, _)| number(run))
}

/// `"([^"]*)"` after a keyword.
pub(super) fn quoted(s: &str) -> Option<&str> {
    let s = s.strip_prefix('"')?;
    s.find('"').map(|end| &s[..end])
}

/// A `Uint8Array` store: `ToUint8( v )` — NaN and ±Infinity are 0, anything
/// else is truncated toward zero and wrapped modulo 256, so `-127.5` is 129
/// and `510` is 254.
pub(super) fn to_uint8(v: f64) -> u8 {
    if v.is_finite() {
        v.trunc().rem_euclid(256.0) as u8
    } else {
        0
    }
}

/// `Math.max( a, b )`: NaN if either is NaN, unlike [`f64::max`].
pub(super) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `size ** 3 * 4` as a typed-array length, or the `RangeError` JavaScript
/// throws for one it cannot allocate — here, any table whose side does not
/// fit a `u32` or whose bytes do not fit the address space.
pub(super) fn table_length(
    loader: &'static str,
    size: f64,
    bytes_per_element: usize,
) -> Result<(u32, usize), Error> {
    let invalid = || Error::Lut {
        loader,
        reason: format!("Invalid typed array length: {}", size.powi(3) * 4.0),
    };
    if !(0.0..=f64::from(u32::MAX)).contains(&size) || size.fract() != 0.0 {
        return Err(invalid());
    }
    let side = size as u32;
    let length = (side as usize)
        .checked_pow(3)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| n.checked_mul(bytes_per_element).is_some())
        .ok_or_else(invalid)?;
    Ok((side, length))
}

/// `Float32Array` contents as the little-endian bytes a
/// [`Data3DTexture`](crate::textures::Data3DTexture) holds.
pub(super) fn f32_bytes(data: &[f32]) -> Vec<u8> {
    data.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_point_accepts_what_the_regex_does() {
        assert_eq!(data_point("0 0.5 1"), Some(["0", "0.5", "1"]));
        assert_eq!(data_point("1e-3  2   -3   "), Some(["1e-3", "2", "-3"]));
        assert_eq!(data_point(" 0 0 0"), None);
        assert_eq!(data_point("0\t0\t0"), None);
        assert_eq!(data_point("0 0 0 0"), None);
        assert_eq!(data_point("0 0"), None);
        assert_eq!(data_point("LUT_3D_SIZE 32"), None);
    }

    #[test]
    fn to_uint8_wraps_like_a_typed_array() {
        assert_eq!(to_uint8(-127.5), 129);
        assert_eq!(to_uint8(510.0), 254);
        assert_eq!(to_uint8(255.0255), 255);
        assert_eq!(to_uint8(f64::NAN), 0);
        assert_eq!(to_uint8(f64::INFINITY), 0);
    }

    #[test]
    fn number_is_nan_on_malformed_tokens() {
        assert!(number("1e").is_nan());
        assert!(number(".").is_nan());
        assert!(number("1.2.3").is_nan());
        assert_eq!(number("+1"), 1.0);
        assert_eq!(number(".5"), 0.5);
        assert_eq!(number("1."), 1.0);
    }
}
