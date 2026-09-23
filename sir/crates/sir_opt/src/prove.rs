//! Algebraic checks the family rewrites rely on.
//!
//! The chains are left folds in loop order, so they need the monoid
//! identity and commutativity (to match the loop's operand order); full
//! associativity is only required if a rewrite regroups the fold.

use sir_mech::{Bv, CheckResult, Term, VarId};

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

/// The masks the SWAR chain is built from, as the repeated-byte patterns
/// they are: `0x55..`, `0x33..`, `0x0F..` and the fold's `0x01..`. The
/// recognizer matches these exact values and the law is stated with them —
/// one definition, so a deviation from the pattern cannot be matched here
/// and proved there.
pub(crate) const SWAR_M1: u64 = 0x5555_5555_5555_5555;
pub(crate) const SWAR_M2: u64 = 0x3333_3333_3333_3333;
pub(crate) const SWAR_M3: u64 = 0x0F0F_0F0F_0F0F_0F0F;
pub(crate) const SWAR_FOLD_UNIT: u64 = 0x0101_0101_0101_0101;

/// Which spelling of the byte-fold a term uses. Both are decided by the
/// same queries with the same CNF — the encoder expands the multiply — so
/// neither spelling is more expensive to prove; they are separate claims
/// only because they are separate terms.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SwarFold {
    /// `(v * 0x0101..) >> (W - 8)`.
    Multiply,
    /// `(v + (v << 8) + ... ) >> (W - 8)`, no multiply.
    ShiftAdd,
}

/// SWAR bit-count: the three shift/mask/add steps **and** the fold.
///
/// The steps alone are not the object. They yield one count per byte,
/// packed *in* that byte, and the bare-steps claim is refutable: at
/// `x = 0x80000000` the steps give 16777216 where the popcount is 1. What
/// makes the chain a bit-count is the fold that adds the packed bytes into
/// one total, so a recognizer that matched the steps without it would be
/// unsound, and this is the obligation that says so.
///
/// Widths 16 and 32 are decided by one query each. The width-64 monolithic
/// query is valid but takes ~5 minutes (measured 313.7 s, CNF 5303/18209),
/// so it is posed decomposed:
///
/// 1. every byte of the stepped term is ≤ 8 — discharged against the
///    actual step term, not argued from the packed-counts claim;
/// 2. the packed bytes sum to `popcount(x)`;
/// 3. the fold lemma: given every byte ≤ 8, folding gives the byte sum.
///
/// Each part is machine-checked before the composite is used; the
/// composite itself is never assumed. The byte-bound hypothesis is not
/// free — the fold is refuted at byte bound 64 (`v = 0x40404040`) — and 8
/// is the exact bound for a popcount's output bytes.
pub fn swar_popcount_laws(width: u32, fold: SwarFold) -> bool {
    // Only the widths a byte-wise chain can describe. 8 is refused rather
    // than supported: its fold shift is 0, so the "fold" is the identity
    // and a recognizer matching it would accept an unfurled step chain.
    if !matches!(width, 16 | 32 | 64) {
        return false;
    }
    if width <= 32 {
        let mut bv = Bv::with_capacity(64);
        let x = bv.var(VarId(0), width);
        let steps = swar_steps(&mut bv, x, width);
        let folded = swar_fold(&mut bv, steps, width, fold);
        let count = bv.popcount(x);
        return matches!(
            sir_mech::prove_equal(&bv, folded, count),
            CheckResult::Valid
        );
    }
    // (1) the byte bound, on the step term itself.
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), 64);
    let steps = swar_steps(&mut bv, x, 64);
    for byte in swar_bytes(&mut bv, steps, 64) {
        let eight = bv.constant(8, 8);
        let claim = bv.ule(byte, eight);
        if !matches!(sir_mech::prove_tautology(&bv, claim), CheckResult::Valid) {
            return false;
        }
    }
    // (2) the packed bytes sum to the popcount.
    let mut bv = Bv::with_capacity(64);
    let x = bv.var(VarId(0), 64);
    let steps = swar_steps(&mut bv, x, 64);
    let summed = swar_byte_sum(&mut bv, steps, 64);
    let count = bv.popcount(x);
    if !matches!(
        sir_mech::prove_equal(&bv, summed, count),
        CheckResult::Valid
    ) {
        return false;
    }
    // (3) the fold lemma, over a free operand, with (1) as its hypothesis.
    let mut bv = Bv::with_capacity(64);
    let v = bv.var(VarId(0), 64);
    let hypotheses: Vec<Term> = {
        let eight = bv.constant(8, 8);
        let bytes = swar_bytes(&mut bv, v, 64);
        bytes.into_iter().map(|byte| bv.ule(byte, eight)).collect()
    };
    let folded = swar_fold(&mut bv, v, 64, fold);
    let summed = swar_byte_sum(&mut bv, v, 64);
    matches!(
        sir_mech::prove_implied_equal(&bv, &hypotheses, folded, summed),
        CheckResult::Valid
    )
}

/// The three steps, exactly as the source spells them: `x - ((x>>1)&M1)`,
/// `(t&M2) + ((t>>2)&M2)`, `(t + (t>>4)) & M3`. The result holds one
/// byte-count per byte.
fn swar_steps(bv: &mut Bv, x: Term, width: u32) -> Term {
    let one = bv.constant(1, width);
    let two = bv.constant(2, width);
    let four = bv.constant(4, width);
    let shifted = bv.lshr(x, one);
    let m1 = bv.constant(SWAR_M1, width);
    let low_bits = bv.and(shifted, m1);
    let step1 = bv.sub(x, low_bits);
    let shifted = bv.lshr(step1, two);
    let m2 = bv.constant(SWAR_M2, width);
    let low = bv.and(step1, m2);
    let high = bv.and(shifted, m2);
    let step2 = bv.add(low, high);
    let shifted = bv.lshr(step2, four);
    let total = bv.add(step2, shifted);
    let m3 = bv.constant(SWAR_M3, width);
    bv.and(total, m3)
}

/// The fold: `(v * 0x0101..) >> (W - 8)` or its shift-add spelling.
fn swar_fold(bv: &mut Bv, v: Term, width: u32, fold: SwarFold) -> Term {
    let amount = bv.constant(u64::from(width - 8), width);
    match fold {
        SwarFold::Multiply => {
            let unit = bv.constant(SWAR_FOLD_UNIT, width);
            let product = bv.mul(v, unit);
            bv.lshr(product, amount)
        }
        SwarFold::ShiftAdd => {
            let mut sum: Option<Term> = None;
            for k in 0..(width / 8) {
                // The first addend is `v` itself, not `v << 0`: a shift by
                // zero is a different term and encodes larger for nothing.
                let shifted = if k == 0 {
                    v
                } else {
                    let amount = bv.constant(u64::from(8 * k), width);
                    bv.shl(v, amount)
                };
                sum = Some(match sum {
                    None => shifted,
                    Some(acc) => bv.add(acc, shifted),
                });
            }
            let sum = sum.expect("width >= 16 has at least two bytes");
            bv.lshr(sum, amount)
        }
    }
}

/// The bytes of `v`, least significant first.
fn swar_bytes(bv: &mut Bv, v: Term, width: u32) -> Vec<Term> {
    (0..(width / 8))
        .map(|k| bv.extract(v, 8 * k + 7, 8 * k))
        .collect()
}

/// The sum of `v`'s bytes, zero-extended to `width`. Seeded from the first
/// byte rather than from zero: `zero + b` is a different term from `b` and
/// encodes larger for no gain.
fn swar_byte_sum(bv: &mut Bv, v: Term, width: u32) -> Term {
    let mut sum: Option<Term> = None;
    for byte in swar_bytes(bv, v, width) {
        let widened = bv.zero_ext(byte, width);
        sum = Some(match sum {
            None => widened,
            Some(acc) => bv.add(acc, widened),
        });
    }
    sum.expect("width >= 16 has at least two bytes")
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
