//! Port of `three.js/src/extras/DataUtils.js`.
//!
//! "Fast Half Float Conversions", <http://www.fox-toolkit.org/ftp/fasthalffloatconversion.pdf>:
//! `toHalfFloat` is a table lookup on the nine top bits of the float32 (sign +
//! exponent) plus a shift of the mantissa, which gets denormals, overflow and
//! the round-toward-zero of the low mantissa bits without a branch.
//!
//! The tables are ported literally rather than replaced with a rounding
//! conversion, because [`HDRLoader`](crate::loaders::HdrLoader) is graded
//! bit-for-bit against `DataUtils.toHalfFloat`'s output and the two disagree in
//! the last bit: this one truncates the dropped mantissa bits, a round-to-
//! nearest conversion does not. Reproducing the quirk is deliberate
//! (`docs/nodes.md` §8).

/// The 512-entry base/shift pair `_generateTables()` builds, indexed by
/// `( bits >> 23 ) & 0x1ff`.
struct HalfTables {
    base: [u32; 512],
    shift: [u32; 512],
}

fn tables() -> &'static HalfTables {
    use std::sync::OnceLock;
    static TABLES: OnceLock<HalfTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut base = [0u32; 512];
        let mut shift = [0u32; 512];

        for i in 0..256usize {
            let e = i as i32 - 127;

            let (b, nb, s) = if e < -27 {
                // very small number (0, -0)
                (0x0000, 0x8000, 24)
            } else if e < -14 {
                // small number (denorm)
                let b = 0x0400u32 >> (-e - 14);
                (b, b | 0x8000, (-e - 1) as u32)
            } else if e <= 15 {
                // normal number
                let b = ((e + 15) as u32) << 10;
                (b, b | 0x8000, 13)
            } else if e < 128 {
                // large number (Infinity, -Infinity)
                (0x7c00, 0xfc00, 24)
            } else {
                // stay (NaN, Infinity, -Infinity)
                (0x7c00, 0xfc00, 13)
            };

            base[i] = b;
            base[i | 0x100] = nb;
            shift[i] = s;
            shift[i | 0x100] = s;
        }

        HalfTables { base, shift }
    })
}

/// `DataUtils.toHalfFloat( val )`.
///
/// The argument is `f64` because the JavaScript is: `toHalfFloat` clamps a
/// JavaScript number and then stores it into a `Float32Array`, so the f64 → f32
/// rounding happens *inside* the function and a caller that rounded first would
/// double-round. [`HDRLoader`](crate::loaders::HdrLoader) is one such caller —
/// its `Math.pow( 2, e - 128 ) / 255` scale is f64 arithmetic.
///
/// Values outside ±65504 are clamped, as upstream (which also warns); `NaN`
/// survives as a half `NaN`.
pub fn to_half_float(val: f64) -> u16 {
    // `clamp( val, -65504, 65504 )` is `Math.max( min, Math.min( max, x ) )`,
    // which propagates NaN; `f64::clamp` panics on a NaN bound but not on a
    // NaN value, and returns NaN, so the two agree.
    let clamped = val.clamp(-65504.0, 65504.0) as f32;
    let f = clamped.to_bits();
    let e = ((f >> 23) & 0x1ff) as usize;
    let tables = tables();
    (tables.base[e] + ((f & 0x007f_ffff) >> tables.shift[e])) as u16
}

/// `DataUtils.fromHalfFloat( val )`.
///
/// Upstream does this with a second set of lookup tables; the bit arithmetic
/// below is the same function — the conversion is exact in both directions for
/// every one of the 65536 half values, so there is no quirk to reproduce.
pub fn from_half_float(val: u16) -> f32 {
    let sign = (val as u32) >> 15;
    let exponent = ((val >> 10) & 0x1f) as u32;
    let mantissa = (val & 0x3ff) as u32;

    let bits = if exponent == 0 {
        if mantissa == 0 {
            // (signed) zero
            sign << 31
        } else {
            // denormal: shift the mantissa up until its leading one is
            // implicit, paying one exponent step per shift
            let mut m = mantissa;
            let mut e: i32 = 0;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            (sign << 31) | (((127 - 15 + 1 + e) as u32) << 23) | ((m & 0x3ff) << 13)
        }
    } else if exponent == 31 {
        // Infinity / NaN
        (sign << 31) | 0x7f80_0000 | (mantissa << 13)
    } else {
        (sign << 31) | ((exponent + 127 - 15) << 23) | (mantissa << 13)
    };

    f32::from_bits(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_obvious_values() {
        assert_eq!(to_half_float(0.0), 0x0000);
        assert_eq!(to_half_float(-0.0), 0x8000);
        assert_eq!(to_half_float(1.0), 0x3c00);
        assert_eq!(to_half_float(-1.0), 0xbc00);
        assert_eq!(to_half_float(2.0), 0x4000);
        // The largest representable half, and the clamp above it.
        assert_eq!(to_half_float(65504.0), 0x7bff);
        assert_eq!(to_half_float(1.0e30), 0x7bff);
        assert_eq!(to_half_float(-1.0e30), 0xfbff);
    }

    /// The smallest denormal is `2^-24`, and everything under half of it goes
    /// to zero — the `e < -27` arm.
    #[test]
    fn denormals_and_underflow() {
        assert_eq!(to_half_float(5.960_464_477_539_063e-8), 0x0001);
        assert_eq!(to_half_float(6.097_555_2e-5), 0x03ff);
        assert_eq!(to_half_float(6.103_515_625e-5), 0x0400);
        assert_eq!(to_half_float(1.0e-12), 0x0000);
    }

    /// Every half value survives a round trip through `from_half_float` and
    /// back, which is what makes the readback of an `rgba16float` target
    /// comparable with the bytes that were uploaded.
    #[test]
    fn every_half_round_trips() {
        for bits in 0..=u16::MAX {
            let exponent = (bits >> 10) & 0x1f;
            let value = from_half_float(bits);
            if exponent == 31 {
                // Infinity clamps to 65504 on the way back and NaN is not
                // comparable; both are checked separately below.
                continue;
            }
            assert_eq!(to_half_float(value as f64), bits, "half {bits:#06x}");
        }
    }

    #[test]
    fn the_non_finite_halves() {
        assert!(from_half_float(0x7c00).is_infinite() && from_half_float(0x7c00) > 0.0);
        assert!(from_half_float(0xfc00).is_infinite() && from_half_float(0xfc00) < 0.0);
        assert!(from_half_float(0x7e00).is_nan());
        assert!(to_half_float(f64::NAN) & 0x7c00 == 0x7c00);
    }

    /// The table conversion truncates the mantissa bits it drops rather than
    /// rounding them, so `1/3` lands one ulp below the nearest half. Three.js
    /// does this, so the port does; see the module docs.
    #[test]
    fn the_mantissa_is_truncated_not_rounded() {
        // 1/3 = 0x3eaaaaab as f32; the top ten mantissa bits are 0x155 and the
        // dropped bits round up, which a rounding conversion would carry.
        assert_eq!(to_half_float(1.0 / 3.0), 0x3555);
        assert!(from_half_float(0x3555) < 1.0 / 3.0);
    }
}
