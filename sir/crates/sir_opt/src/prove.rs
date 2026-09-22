//! Algebraic checks the family rewrites rely on.
//!
//! The chains are left folds in loop order, so they need the monoid
//! identity and commutativity (to match the loop's operand order); full
//! associativity is only required if a rewrite regroups the fold.

use sir_mech::{Bv, CheckResult, VarId};

/// Extent-independent first-match scan law. The proof lives in the
/// kernel (`sir_mech::scan`) and is cached there for the process.
pub fn first_match_law() -> bool {
    sir_mech::scan::first_match_law()
}

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

/// Both bitwise-OR zero identities, covering either operand order.
pub fn or_laws(width: u32) -> bool {
    if width == 0 || width > 64 {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let a = bv.var(VarId(0), width);
    let zero = bv.zero(width);
    let zero_or_a = bv.or(zero, a);
    if !matches!(sir_mech::prove_equal(&bv, zero_or_a, a), CheckResult::Valid) {
        return false;
    }
    let a_or_zero = bv.or(a, zero);
    matches!(sir_mech::prove_equal(&bv, a_or_zero, a), CheckResult::Valid)
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

/// Unsigned division by a constant power of two is a logical shift.
pub fn divide_pow2_laws(width: u32, shift: u32) -> bool {
    if width == 0 || width > 64 || shift == 0 || shift >= width {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), width);
    let divisor = bv.constant(1u64 << shift, width);
    let shift_amount = bv.constant(u64::from(shift), width);
    let divided = bv.udiv(x, divisor);
    let shifted = bv.lshr(x, shift_amount);
    matches!(
        sir_mech::prove_equal(&bv, divided, shifted),
        CheckResult::Valid
    )
}

/// Unsigned remainder by a constant power of two is a low-bit mask.
pub fn modulo_pow2_laws(width: u32, shift: u32) -> bool {
    if width == 0 || width > 64 || shift == 0 || shift >= width {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), width);
    let divisor = bv.constant(1u64 << shift, width);
    let mask = bv.constant((1u64 << shift) - 1, width);
    let remainder = bv.urem(x, divisor);
    let masked = bv.and(x, mask);
    matches!(
        sir_mech::prove_equal(&bv, remainder, masked),
        CheckResult::Valid
    )
}

/// Multiplication by a constant power of two is a left shift.
pub fn multiply_pow2_laws(width: u32, shift: u32) -> bool {
    if width == 0 || width > 64 || shift == 0 || shift >= width {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), width);
    let factor = bv.constant(1u64 << shift, width);
    let shift_amount = bv.constant(u64::from(shift), width);
    let multiplied = bv.mul(x, factor);
    let shifted = bv.shl(x, shift_amount);
    matches!(
        sir_mech::prove_equal(&bv, multiplied, shifted),
        CheckResult::Valid
    )
}

/// `(x << k) >> k` clears the top `k` bits: a mask of the low `W - k`.
pub fn shift_mask_laws(width: u32, shift: u32) -> bool {
    if width == 0 || width > 64 || shift == 0 || shift >= width {
        return false;
    }
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), width);
    let amount = bv.constant(u64::from(shift), width);
    let raised = bv.shl(x, amount);
    let lowered = bv.lshr(raised, amount);
    let mask = bv.constant((1u64 << (width - shift)) - 1, width);
    let masked = bv.and(x, mask);
    matches!(
        sir_mech::prove_equal(&bv, lowered, masked),
        CheckResult::Valid
    )
}
