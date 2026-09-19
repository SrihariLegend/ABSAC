//! Algebraic checks the sum rewrite relies on.

use sir_mech::{Bv, CheckResult, Term, VarId};

/// Prove the wrapping-add laws used by the unrolled sum: associativity
/// and both identities. The SAT kernel discharges them at the real
/// accumulator width.
pub fn sum_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let b = bv.var(VarId(1), width);
    let c = bv.var(VarId(2), width);

    let a_plus_b = bv.add(a, b);
    let left = bv.add(a_plus_b, c);
    let b_plus_c = bv.add(b, c);
    let right = bv.add(a, b_plus_c);
    if !matches!(sir_mech::prove_equal(&bv, left, right), CheckResult::Valid) {
        return false;
    }
    let zero = bv.zero(width);
    let a_plus_zero = bv.add(a, zero);
    if !matches!(sir_mech::prove_equal(&bv, a_plus_zero, a), CheckResult::Valid) {
        return false;
    }
    let zero_plus_a = bv.add(zero, a);
    matches!(
        sir_mech::prove_equal(&bv, zero_plus_a, a),
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

/// Prove the bitwise-OR monoid laws used by the unrolled Any reduction.
pub fn or_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let b = bv.var(VarId(1), width);
    let c = bv.var(VarId(2), width);
    let ab = bv.or(a, b);
    let left = bv.or(ab, c);
    let bc = bv.or(b, c);
    let right = bv.or(a, bc);
    if !matches!(sir_mech::prove_equal(&bv, left, right), CheckResult::Valid) {
        return false;
    }
    let zero = bv.zero(width);
    let a_or_zero = bv.or(a, zero);
    if !matches!(sir_mech::prove_equal(&bv, a_or_zero, a), CheckResult::Valid) {
        return false;
    }
    let ba = bv.or(b, a);
    matches!(sir_mech::prove_equal(&bv, ab, ba), CheckResult::Valid)
}

/// Prove the bitwise-AND monoid laws plus the 0/1 indicator identity
/// used by the All reduction (`indicator & 1 == indicator`).
pub fn and_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let b = bv.var(VarId(1), width);
    let c = bv.var(VarId(2), width);
    let ab = bv.and(a, b);
    let left = bv.and(ab, c);
    let bc = bv.and(b, c);
    let right = bv.and(a, bc);
    if !matches!(sir_mech::prove_equal(&bv, left, right), CheckResult::Valid) {
        return false;
    }
    let all_ones = if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    };
    let ones = bv.constant(all_ones, width);
    let a_and_ones = bv.and(a, ones);
    if !matches!(
        sir_mech::prove_equal(&bv, a_and_ones, a),
        CheckResult::Valid
    ) {
        return false;
    }
    // Indicator identity: for a 1-bit value zero-extended to `width`,
    // AND-ing with the constant 1 is the identity.
    let bit = bv.var(VarId(3), 1);
    let widened = bv.zero_ext(bit, width);
    let one = bv.constant(1, width);
    let masked = bv.and(widened, one);
    matches!(
        sir_mech::prove_equal(&bv, masked, widened),
        CheckResult::Valid
    )
}

/// Prove the bitwise-XOR monoid laws used by the unrolled parity
/// reduction: associativity, identity zero, and self-inverse.
pub fn xor_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let b = bv.var(VarId(1), width);
    let c = bv.var(VarId(2), width);
    let ab = bv.xor(a, b);
    let left = bv.xor(ab, c);
    let bc = bv.xor(b, c);
    let right = bv.xor(a, bc);
    if !matches!(sir_mech::prove_equal(&bv, left, right), CheckResult::Valid) {
        return false;
    }
    let zero = bv.zero(width);
    let a_xor_zero = bv.xor(a, zero);
    if !matches!(
        sir_mech::prove_equal(&bv, a_xor_zero, a),
        CheckResult::Valid
    ) {
        return false;
    }
    let self_xor = bv.xor(a, a);
    matches!(
        sir_mech::prove_equal(&bv, self_xor, zero),
        CheckResult::Valid
    )
}
