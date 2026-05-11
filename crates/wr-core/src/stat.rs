//! Fixed-point numeric type for combat math.
//!
//! [`Stat`] is `fixed::types::I32F32`, a signed 64-bit value split into
//! a 32-bit integer part and a 32-bit fractional part.  This range
//! easily covers every quantity in the game (HP, attack, defense,
//! damage, multipliers) while remaining bit-for-bit identical across
//! every target architecture the simulator ships to.
//!
//! ## Why not `f32` / `f64`?
//!
//! IEEE-754 results vary subtly across platforms: subnormals are
//! flushed differently on ARM vs. x86 by default, fused multiply-add
//! changes intermediate rounding, and even `tan`/`exp` differ between
//! libm implementations.  None of those differences would matter for a
//! conventional game — but in our setting they would let the operator
//! produce battle outputs an independent verifier cannot reproduce.
//! Fixed-point is the simplest fix.
//!
//! ## Range
//!
//! * Min: `≈ -2.147 × 10⁹`
//! * Max: `≈ +2.147 × 10⁹`
//! * Resolution: `2⁻³² ≈ 2.33 × 10⁻¹⁰`

use fixed::types::I32F32;

/// Canonical numeric type for combat math.  See module docs.
pub type Stat = I32F32;

// Compile-time guard: a future change to `Stat` (e.g. swapping I32F32
// for I64F64) would silently re-shape every hash and bincode payload
// in the system. Lock the wire width here so the build fails loudly
// instead.
const _: () = assert!(
    core::mem::size_of::<Stat>() == 8,
    "Stat must serialize to exactly 8 bytes; canonical hashes assume i64 width",
);

/// Zero value for [`Stat`].  Bit pattern: all zeros.
pub const STAT_ZERO: Stat = Stat::from_bits(0);

/// One value for [`Stat`].  Bit pattern: `1 << 32` (the integer-1
/// position in an `I32F32`).
pub const STAT_ONE: Stat = Stat::from_bits(1i64 << 32);

/// Two value for [`Stat`].  Convenient for damage formulas.
pub const STAT_TWO: Stat = Stat::from_bits(2i64 << 32);

/// Half value for [`Stat`].  Bit pattern: `1 << 31`.
pub const STAT_HALF: Stat = Stat::from_bits(1i64 << 31);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_have_expected_numeric_values() {
        assert_eq!(STAT_ZERO, Stat::from_num(0));
        assert_eq!(STAT_ONE, Stat::from_num(1));
        assert_eq!(STAT_TWO, Stat::from_num(2));
        assert_eq!(STAT_HALF + STAT_HALF, STAT_ONE);
    }

    #[test]
    fn stat_arithmetic_is_lossless_within_range() {
        let a = Stat::from_num(7);
        let b = Stat::from_num(13);
        assert_eq!(a + b, Stat::from_num(20));
        assert_eq!(a * b, Stat::from_num(91));
        assert_eq!(b - a, Stat::from_num(6));
    }

    #[test]
    fn stat_serializes_to_a_fixed_width() {
        // I32F32 serializes as its underlying i64 (8 bytes) under bincode.
        let v = Stat::from_num(42);
        let bytes = bincode::serialize(&v).expect("serialize");
        assert_eq!(bytes.len(), 8);
    }
}
