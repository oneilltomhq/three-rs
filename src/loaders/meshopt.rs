//! `EXT_meshopt_compression`: the decoder `GLTFMeshoptCompression` hands a
//! compressed bufferView to, `MeshoptDecoder.decodeGltfBuffer`.
//!
//! three.js ships that decoder only as WebAssembly
//! (`examples/jsm/libs/meshopt_decoder.module.js`, built from meshoptimizer
//! 1.1). The two codecs are taken from the pure-Rust [`meshopt_rs`] port of
//! meshoptimizer: the vertex codec (`ATTRIBUTES`), and the index codecs
//! (`TRIANGLES`, and `INDICES` behind its `experimental` feature). On both
//! meshopt assets in the three.js examples they match the WebAssembly decoder
//! byte for byte (`tests/gltf_meshopt.rs`).
//!
//! The four filters are ported here instead, from meshoptimizer 1.1's
//! `vertexfilter.cpp`, because the crate's are not what three.js runs:
//!
//! * its `decode_filter_quat` reads the 16-bit components as unsigned, so any
//!   negative component decodes wrong (on `facecap.glb`'s rotation track 5088
//!   of 8040 bytes differ from three.js), and it is the pre-0.19 formulation
//!   besides;
//! * its octahedral and exponential filters are the scalar C++ ones. The
//!   decoder three.js loads picks its SIMD build wherever WebAssembly SIMD
//!   validates, which is every current browser and node, and the SIMD filters
//!   round to nearest-even (`x + 0x1.8p23`) where the scalar ones round half
//!   away from zero (`int(x + 0.5f)`), and sum `x² + (y² + z²)` where the
//!   scalar ones sum `(x² + y²) + z²`. On valid data the results differ only
//!   at exact ties, which the two example assets happen not to hit, but they
//!   do differ, and on out-of-range data (which the filters are defined on)
//!   the SIMD rounding trick keeps bits the scalar code does not.
//!
//! So the filters below reproduce the WebAssembly SIMD kernels
//! (`decodeFilterOctSimd8/16`, `decodeFilterQuatSimd`, `decodeFilterExpSimd`)
//! operation for operation, in scalar `f32`, which is exact: those kernels
//! are lane-wise IEEE single-precision arithmetic with no fused operations.
//! `tests/gltf_meshopt.rs` checks every octahedral input and a sweep of
//! quaternion and exponential inputs against the WebAssembly build itself.
//!
//! `COLOR` is meshoptimizer's filter for `KHR_meshopt_compression`, which
//! also brings the version 1 vertex codec the crate does not decode; neither
//! is part of `EXT_meshopt_compression`, and neither is ported.

use meshopt_rs::index::buffer::decode_index_buffer;
use meshopt_rs::index::sequence::decode_index_sequence;
use meshopt_rs::vertex::buffer::decode_vertex_buffer;

/// `extensionDef.mode`: which codec the bufferView was encoded with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `ATTRIBUTES` — `meshopt_decodeVertexBuffer`.
    Attributes,
    /// `TRIANGLES` — `meshopt_decodeIndexBuffer`.
    Triangles,
    /// `INDICES` — `meshopt_decodeIndexSequence`.
    Indices,
}

impl Mode {
    /// The extension's spelling; `None` for anything else.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ATTRIBUTES" => Some(Self::Attributes),
            "TRIANGLES" => Some(Self::Triangles),
            "INDICES" => Some(Self::Indices),
            _ => None,
        }
    }
}

/// `extensionDef.filter`: what to run over an `ATTRIBUTES` view once decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// `NONE`, and an absent `filter`.
    None,
    /// `OCTAHEDRAL` — `meshopt_decodeFilterOct`.
    Octahedral,
    /// `QUATERNION` — `meshopt_decodeFilterQuat`.
    Quaternion,
    /// `EXPONENTIAL` — `meshopt_decodeFilterExp`.
    Exponential,
}

impl Filter {
    /// The extension's spelling; `None` for anything else.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "NONE" => Some(Self::None),
            "OCTAHEDRAL" => Some(Self::Octahedral),
            "QUATERNION" => Some(Self::Quaternion),
            "EXPONENTIAL" => Some(Self::Exponential),
            _ => None,
        }
    }
}

/// `MeshoptDecoder.decodeGltfBuffer( target, count, stride, source, mode,
/// filter )`, returning `target`: `count * stride` bytes.
///
/// The combinations `EXT_meshopt_compression` allows are checked first, and
/// anything else is an error. meshoptimizer `assert`s them, and its release
/// WebAssembly build has the asserts compiled out, so there three.js decodes
/// garbage or throws `Malformed buffer data`; the port says what is wrong
/// instead. A stream the codec rejects is an error either way.
pub fn decode_gltf_buffer(
    count: usize,
    stride: usize,
    source: &[u8],
    mode: Mode,
    filter: Filter,
) -> Result<Vec<u8>, String> {
    check(count, stride, mode, filter)?;
    let malformed = |error: &dyn std::fmt::Debug| format!("malformed buffer data ({error:?})");

    let mut target = match mode {
        Mode::Attributes => decode_vertices(count, stride, source).map_err(|e| malformed(&e))?,
        Mode::Triangles | Mode::Indices => {
            let mut indices = vec![0u32; count];
            if mode == Mode::Triangles {
                decode_index_buffer(&mut indices, source).map_err(|e| malformed(&e))?;
            } else {
                decode_index_sequence(&mut indices, source).map_err(|e| malformed(&e))?;
            }
            // `meshopt_decodeIndexBuffer` with `index_size == 2` stores
            // `(unsigned short)index`.
            if stride == 2 {
                indices
                    .iter()
                    .flat_map(|&i| (i as u16).to_le_bytes())
                    .collect()
            } else {
                indices.iter().flat_map(|&i| i.to_le_bytes()).collect()
            }
        }
    };

    match (filter, stride) {
        (Filter::None, _) => {}
        (Filter::Octahedral, 4) => filter_oct_8(&mut target),
        (Filter::Octahedral, _) => filter_oct_16(&mut target),
        (Filter::Quaternion, _) => filter_quat(&mut target),
        (Filter::Exponential, _) => filter_exp(&mut target),
    }

    Ok(target)
}

/// The `EXT_meshopt_compression` rules on `mode`, `filter`, `count` and
/// `byteStride`, which meshoptimizer's decoders `assert`.
fn check(count: usize, stride: usize, mode: Mode, filter: Filter) -> Result<(), String> {
    let fine = match mode {
        Mode::Attributes => stride > 0 && stride <= 256 && stride.is_multiple_of(4),
        Mode::Triangles => (stride == 2 || stride == 4) && count.is_multiple_of(3),
        Mode::Indices => stride == 2 || stride == 4,
    } && match filter {
        Filter::None => true,
        _ if mode != Mode::Attributes => false,
        Filter::Octahedral => stride == 4 || stride == 8,
        Filter::Quaternion => stride == 8,
        Filter::Exponential => true,
    };
    if fine {
        Ok(())
    } else {
        Err(format!(
            "mode {mode:?}, filter {filter:?}, count {count} and byteStride {stride} do not go together"
        ))
    }
}

/// `meshopt_decodeVertexBuffer` for a stride known only at run time: the
/// crate takes the vertex size from its element type, so every stride the
/// codec accepts gets a `[u8; N]` of its own.
fn decode_vertices(
    count: usize,
    stride: usize,
    source: &[u8],
) -> Result<Vec<u8>, meshopt_rs::vertex::DecodeError> {
    macro_rules! strides {
        ($($n:literal)*) => {
            match stride {
                $($n => {
                    let mut vertices = vec![[0u8; $n]; count];
                    decode_vertex_buffer(&mut vertices, source)?;
                    Ok(vertices.concat())
                })*
                _ => unreachable!("`check` allows no other stride"),
            }
        };
    }
    strides!(
        4 8 12 16 20 24 28 32 36 40 44 48 52 56 60 64
        68 72 76 80 84 88 92 96 100 104 108 112 116 120 124 128
        132 136 140 144 148 152 156 160 164 168 172 176 180 184 188 192
        196 200 204 208 212 216 220 224 228 232 236 240 244 248 252 256
    )
}

/// The SIMD kernels' "fast rounded signed float->int": add `3 << 22`
/// (`0x1.8p23`), after which, for `|value| < 2^22`, the mantissa holds
/// `value` rounded to nearest-even, and keep the low bits of the float's bit
/// pattern (the callers truncate to 8 or 16). It is done literally rather
/// than as `round_ties_even`, because the quaternion filter is defined on
/// every input and a negative scale takes `value` past `2^22`, where the low
/// bits are no longer the rounded integer; three.js keeps them anyway.
fn snap(value: f32) -> i32 {
    (value + 12_582_912.0).to_bits() as i32
}

/// `x += t` with `t`'s sign flipped where `x` is negative:
/// `x + (t ^ (x & -0.f))` on the bit patterns.
fn fold(x: f32, t: f32) -> f32 {
    x + f32::from_bits(t.to_bits() ^ (x.to_bits() & 0x8000_0000))
}

/// `z` if it is negative, else `+0`: `wasm_i32x4_min( z, 0 )` on the bits.
fn negative_part(z: f32) -> f32 {
    f32::from_bits((z.to_bits() as i32).min(0) as u32)
}

/// The octahedral kernel on one vector, `max` being 127 or 32767.
fn oct(x: f32, y: f32, z: f32, max: f32) -> (i32, i32, i32) {
    let z = z - (x.abs() + y.abs());
    let t = negative_part(z);
    let x = fold(x, t);
    let y = fold(y, t);

    let ll = x * x + (y * y + z * z);
    let s = max / ll.sqrt();

    (snap(x * s), snap(y * s), snap(z * s))
}

/// `decodeFilterOctSimd8`: signed 8-bit `x, y, z`, `w` kept.
fn filter_oct_8(data: &mut [u8]) {
    for v in data.as_chunks_mut::<4>().0 {
        let (x, y, z) = oct(
            v[0] as i8 as f32,
            v[1] as i8 as f32,
            v[2] as i8 as f32,
            127.0,
        );
        v[0] = x as u8;
        v[1] = y as u8;
        v[2] = z as u8;
    }
}

/// `decodeFilterOctSimd16`: signed 16-bit `x, y`, `z` read as 15 bits
/// unsigned (`& 0x7fff`), `w` kept.
fn filter_oct_16(data: &mut [u8]) {
    for v in data.as_chunks_mut::<8>().0 {
        let (x, y, z) = oct(
            i16::from_le_bytes([v[0], v[1]]) as f32,
            i16::from_le_bytes([v[2], v[3]]) as f32,
            (u16::from_le_bytes([v[4], v[5]]) & 0x7fff) as f32,
            32767.0,
        );
        v[0..2].copy_from_slice(&(x as u16).to_le_bytes());
        v[2..4].copy_from_slice(&(y as u16).to_le_bytes());
        v[4..6].copy_from_slice(&(z as u16).to_le_bytes());
    }
}

/// `decodeFilterQuatSimd`: three signed 16-bit components, and a fourth
/// whose low two bits say which component was dropped and whose rest is the
/// scale. Components are unscaled until the last multiply.
fn filter_quat(data: &mut [u8]) {
    let scale = 32767.0f32 / 2.0f32.sqrt();

    for q in data.as_chunks_mut::<8>().0 {
        let c = |i: usize| i16::from_le_bytes([q[2 * i], q[2 * i + 1]]) as i32;
        let (x, y, z, cf) = (c(0) as f32, c(1) as f32, c(2) as f32, c(3));

        let s = (cf | 3) as f32;
        let ws = s * s;
        let ww = (ws + ws) - (x * x + (y * y + z * z));
        // `wasm_i32x4_max( ww, 0 )` on the bits: a negative `ww` is `+0`.
        let w = f32::from_bits((ww.to_bits() as i32).max(0) as u32).sqrt();

        let ss = scale / s;
        let out = [snap(w * ss), snap(x * ss), snap(y * ss), snap(z * ss)];

        // The kernel packs `w x y z` and rotates left by `qc * 16` bits,
        // which is `data[ ( qc + k ) & 3 ] = out[ k ]`.
        let qc = (cf & 3) as usize;
        for (k, value) in out.into_iter().enumerate() {
            let at = 2 * ((qc + k) & 3);
            q[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes());
        }
    }
}

/// `decodeFilterExpSimd`: each 32-bit word is a signed 24-bit mantissa and
/// an 8-bit exponent, `2^e * m`, with `2^e` built straight into the float's
/// exponent bits.
fn filter_exp(data: &mut [u8]) {
    for v in data.as_chunks_mut::<4>().0 {
        let word = i32::from_le_bytes([v[0], v[1], v[2], v[3]]);
        let e = word >> 24;
        let m = (word << 8) >> 8;
        let power = f32::from_bits((e.wrapping_add(127) << 23) as u32);
        v.copy_from_slice(&(power * m as f32).to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_is_ldexp() {
        let word = |e: i32, m: i32| -> [u8; 4] { ((e << 24) | (m & 0xff_ffff)).to_le_bytes() };
        let mut data = [word(-3, 12), word(2, -5), word(0, 0)].concat();
        filter_exp(&mut data);
        let floats: Vec<f32> = data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&b| f32::from_le_bytes(b))
            .collect();
        assert_eq!(floats, [1.5, -20.0, 0.0]);
    }

    #[test]
    fn quat_identity() {
        // The encoder's fourth word for 16 bits and qc = 0: `32767 & ~3`.
        // The reconstructed component lands in slot qc.
        let mut data = [0i16, 0, 0, 0x7ffc]
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect::<Vec<u8>>();
        filter_quat(&mut data);
        let out: Vec<i16> = data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&b| i16::from_le_bytes(b))
            .collect();
        assert_eq!(out, [32767, 0, 0, 0]);
    }

    #[test]
    fn bad_combinations_are_errors() {
        assert!(decode_gltf_buffer(3, 6, &[], Mode::Attributes, Filter::None).is_err());
        assert!(decode_gltf_buffer(4, 2, &[], Mode::Triangles, Filter::None).is_err());
        assert!(decode_gltf_buffer(3, 3, &[], Mode::Indices, Filter::None).is_err());
        assert!(decode_gltf_buffer(1, 12, &[], Mode::Attributes, Filter::Octahedral).is_err());
        assert!(
            decode_gltf_buffer(3, 2, &[0xe1; 64], Mode::Triangles, Filter::Exponential).is_err()
        );
        assert!(decode_gltf_buffer(1, 4, &[0x00; 64], Mode::Attributes, Filter::None).is_err());
    }
}
