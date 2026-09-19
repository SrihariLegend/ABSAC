//! Algebraic checks the family rewrites rely on.
//!
//! The chains are left folds in loop order, so they need the monoid
//! identity and commutativity (to match the loop's operand order); full
//! associativity is only required if a rewrite regroups the fold.

use sir_mech::{Bv, CheckResult, Term, VarId};

/// Both wrapping-add zero identities, covering either operand order.
/// The SAT kernel discharges them at the real accumulator width.
pub fn sum_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let zero = bv.zero(width);
    let zero_plus_a = bv.add(zero, a);
    if !matches!(
        sir_mech::prove_equal(&bv, zero_plus_a, a),
        CheckResult::Valid
    ) {
        return false;
    }
    let a_plus_zero = bv.add(a, zero);
    matches!(
        sir_mech::prove_equal(&bv, a_plus_zero, a),
        CheckResult::Valid
    )
}

/// Prove that the canonical found-flag scan and the reverse-overwrite
/// scan compute the same first-match index (or the extent sentinel).
/// This is the semantic contract of the `sir_memchr` primitive.
pub fn first_match_laws(extent: u32, width: u32) -> bool {
    if extent == 0 || extent > 128 || width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(extent as usize * 16 + 64);
    let bits: Vec<Term> = (0..extent)
        .map(|index| bv.var(VarId(0x8000 + index), 1))
        .collect();

    let mut found = bv.zero(1);
    let mut forward = bv.constant(u64::from(extent), width);
    for (index, &bit) in bits.iter().enumerate() {
        let not_found = bv.not(found);
        let take = bv.and(not_found, bit);
        let position = bv.constant(index as u64, width);
        forward = bv.ite(take, position, forward);
        found = bv.or(found, bit);
    }

    let mut reverse = bv.constant(u64::from(extent), width);
    for (index, &bit) in bits.iter().enumerate().rev() {
        let position = bv.constant(index as u64, width);
        reverse = bv.ite(bit, position, reverse);
    }
    matches!(
        sir_mech::prove_equal(&bv, forward, reverse),
        CheckResult::Valid
    )
}

/// Both bitwise-OR zero identities, covering either operand order.
pub fn or_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let zero = bv.zero(width);
    let zero_or_a = bv.or(zero, a);
    if !matches!(
        sir_mech::prove_equal(&bv, zero_or_a, a),
        CheckResult::Valid
    ) {
        return false;
    }
    let a_or_zero = bv.or(a, zero);
    matches!(
        sir_mech::prove_equal(&bv, a_or_zero, a),
        CheckResult::Valid
    )
}

/// Both 0/1 indicator identities used by All.
pub fn and_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    // Indicator identity: for a 1-bit value zero-extended to `width`,
    // AND-ing with the constant 1 is the identity in either order.
    let bit = bv.var(VarId(3), 1);
    let widened = bv.zero_ext(bit, width);
    let one = bv.constant(1, width);
    let masked = bv.and(widened, one);
    if !matches!(
        sir_mech::prove_equal(&bv, masked, widened),
        CheckResult::Valid
    ) {
        return false;
    }
    let masked_rev = bv.and(one, widened);
    matches!(
        sir_mech::prove_equal(&bv, masked_rev, widened),
        CheckResult::Valid
    )
}

/// Both bitwise-XOR zero identities, covering either operand order.
pub fn xor_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let zero = bv.zero(width);
    let zero_xor_a = bv.xor(zero, a);
    if !matches!(
        sir_mech::prove_equal(&bv, zero_xor_a, a),
        CheckResult::Valid
    ) {
        return false;
    }
    let a_xor_zero = bv.xor(a, zero);
    matches!(
        sir_mech::prove_equal(&bv, a_xor_zero, a),
        CheckResult::Valid
    )
}
