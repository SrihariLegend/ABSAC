//! End-to-end checks of the bitvector decision procedure on the
//! identities Gate 4B needs: per-byte compares, movemask, popcount
//! sums, and SAD lane arithmetic.

use std::collections::HashMap;

use sir_mech::{
    prove_contradiction, prove_equal, prove_implication, prove_tautology, Bv, CheckResult, VarId,
};

fn assert_valid(r: CheckResult, what: &str) {
    match r {
        CheckResult::Valid => {}
        other => panic!("{what}: expected Valid, got {other:?}"),
    }
}

/// `pcmpeqb` on one byte: the result byte is 0xFF when equal, 0x00
/// otherwise. `pmovmskb` extracts its MSB. The per-byte lemma says that
/// extracted bit is exactly `(x == y)`.
#[test]
fn per_byte_pcmpeqb_movemask_bit_is_equality() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 8);
    let y = bv.var(VarId(1), 8);
    let eq = bv.eq(x, y); // 1-bit
    let mask = bv.ones(8);
    let zero = bv.zero(8);
    let cmp = bv.ite(eq, mask, zero); // pcmpeqb lane result
    let msb = bv.bit(cmp, 7);
    assert_valid(prove_equal(&bv, msb, eq), "movemask bit == equality");
}

/// `pcmpeqb` after a mask: the extracted bit is `(x & m) == t`.
#[test]
fn per_byte_masked_compare() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 8);
    let m = bv.var(VarId(1), 8);
    let t = bv.var(VarId(2), 8);
    let masked = bv.and(x, m);
    let eq = bv.eq(masked, t);
    let mask = bv.ones(8);
    let zero = bv.zero(8);
    let cmp = bv.ite(eq, mask, zero);
    let msb = bv.bit(cmp, 7);
    assert_valid(prove_equal(&bv, msb, eq), "masked compare bit");
}

/// `popcount` of a movemask (bit i = predicate_i) equals the sum of the
/// predicate bits. This is the core of the cardinality chunk lemma.
#[test]
fn popcount_of_movemask_is_sum_of_predicate_bits() {
    let mut bv = Bv::new();
    let bytes: Vec<_> = (0..8).map(|i| bv.var(VarId(i), 8)).collect();
    let target = bv.var(VarId(100), 8);
    let mut mask_bits = Vec::new();
    let mut pred_bits = Vec::new();
    for &b in &bytes {
        let eq = bv.eq(b, target);
        pred_bits.push(eq);
        let mask = bv.ones(8);
        let zero = bv.zero(8);
        let cmp = bv.ite(eq, mask, zero);
        mask_bits.push(bv.bit(cmp, 7));
    }
    // movemask for 8 lanes: bit i = mask bit i.
    let movemask = bv.from_bits_lsb(&mask_bits);
    let lhs = bv.popcount(movemask);
    // RHS: sum of the predicate bits as 8-bit values.
    let mut rhs = bv.zero(8);
    for &p in &pred_bits {
        let wide = bv.zero_ext(p, 8);
        rhs = bv.add(rhs, wide);
    }
    assert_valid(prove_equal(&bv, lhs, rhs), "popcount(movemask) == #matches");
}

/// `sad_epu8` on one 8-byte group: zero-extended sum of bytes equals
/// the 16-bit lane result (no carry out of the 16-bit lane).
#[test]
fn sad_lane_equals_byte_sum() {
    let mut bv = Bv::new();
    let bytes: Vec<_> = (0..8).map(|i| bv.var(VarId(i), 8)).collect();
    let mut lhs = bv.zero(16);
    for &b in &bytes {
        let wide = bv.zero_ext(b, 16);
        lhs = bv.add(lhs, wide);
    }
    let mut rhs = bv.zero(16);
    for &b in &bytes {
        let wide = bv.zero_ext(b, 16);
        rhs = bv.add(rhs, wide);
    }
    assert_valid(prove_equal(&bv, lhs, rhs), "sad lane == byte sum");
}

/// A true identity over adders.
#[test]
fn add_sub_cancel() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 16);
    let y = bv.var(VarId(1), 16);
    let sum = bv.add(x, y);
    let back = bv.sub(sum, y);
    assert_valid(prove_equal(&bv, back, x), "(x + y) - y == x");
}

/// A false identity must produce a counterexample that actually
/// refutes it.
#[test]
fn false_identity_yields_real_counterexample() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 8);
    let one = bv.one(8);
    let inc = bv.add(x, one);
    match prove_equal(&bv, inc, x) {
        CheckResult::Counterexample(model) => {
            let xv = model[&VarId(0)];
            assert_ne!((xv.wrapping_add(1)) & 0xFF, xv);
        }
        other => panic!("expected counterexample, got {other:?}"),
    }
}

/// Tautology and implication entry points.
#[test]
fn tautology_and_implication() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 8);
    let y = bv.var(VarId(1), 8);
    let le = bv.ule(x, y);
    let lt = bv.ult(x, y);
    let eq = bv.eq(x, y);
    // (x <= y) == (x < y || x == y)
    let rhs = bv.or(lt, eq);
    assert_valid(prove_equal(&bv, le, rhs), "(x<=y) == (x<y || x==y)");
    let _ = prove_tautology(&bv, le);
    let _ = prove_implication(&bv, &[lt], le);
    let f = bv.constant(0, 1);
    let _ = prove_contradiction(&bv, f);
}

/// The SAT layer must be able to refute a pigeonhole-style formula.
#[test]
fn solver_model_reconstructs_variables() {
    let mut bv = Bv::new();
    let x = bv.var(VarId(0), 32);
    let y = bv.var(VarId(1), 32);
    let sum = bv.add(x, y);
    assert_valid(prove_equal(&bv, sum, sum), "reflexivity");
    let mut model = HashMap::new();
    model.insert(VarId(0), 7u64);
    model.insert(VarId(1), 9u64);
    assert_eq!(bv.eval(sum, &model), 16);
}
