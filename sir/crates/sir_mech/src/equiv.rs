//! The decision procedure: is `lhs == rhs` valid for *all* inputs?
//!
//! Both sides are bit-blasted into one CNF together with the
//! negated-difference constraint. `Valid` means the difference is
//! unsatisfiable (a proof over all inputs); `Counterexample` carries a
//! concrete assignment that is re-evaluated against the original terms
//! before it is reported, so a solver bug can never manufacture a
//! refutation of a true identity.

use std::collections::HashMap;

use crate::bv::{Bv, Term, VarId};
use crate::cnf::Encoder;
use crate::sat::{self, SatResult};

/// Result of a validity query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// The claim holds for every input.
    Valid,
    /// The claim fails; the model refutes it (verified by re-evaluation
    /// against the original terms).
    Counterexample(HashMap<VarId, u64>),
    /// The solver could not decide within its budget.
    Unknown(String),
}

/// Build a CNF asserting `bits_a != bits_b` (bitwise).
fn assert_difference(enc: &mut Encoder<'_>, a: &[i32], b: &[i32]) {
    let cnf = enc.cnf_mut();
    let mut diffs = Vec::with_capacity(a.len());
    for (&x, &y) in a.iter().zip(b.iter()) {
        diffs.push(cnf.xor2(x, y));
    }
    let any = cnf.or_all(&diffs);
    cnf.add_clause([any]);
}

/// Decode a SAT model into a bitvector valuation.
fn decode_model(
    bv: &Bv,
    var_bits: &HashMap<VarId, Vec<i32>>,
    model: &[bool],
) -> HashMap<VarId, u64> {
    let mut out = HashMap::new();
    for (var, width) in bv.vars() {
        let bits = var_bits
            .get(&var)
            .unwrap_or_else(|| panic!("variable {var} was never encoded"));
        let mut value = 0u64;
        for (i, &lit) in bits.iter().enumerate() {
            let truth = if lit > 0 {
                model[lit as usize]
            } else {
                !model[(-lit) as usize]
            };
            if truth {
                value |= 1u64 << i;
            }
        }
        let masked = if width == 64 {
            value
        } else {
            value & ((1u64 << width) - 1)
        };
        out.insert(var, masked);
    }
    out
}

/// Shared conclusion: the CNF asserts the negation of the claim, so
/// UNSAT is a proof. SAT models are re-checked against the original
/// terms by the caller-supplied predicate before being reported.
fn conclude(
    bv: &Bv,
    enc: Encoder<'_>,
    falsified: impl FnOnce(&Bv, &HashMap<VarId, u64>) -> bool,
) -> CheckResult {
    let var_bits = enc.var_bit_map();
    let cnf = enc.finish();
    match sat::solve_default(&cnf) {
        SatResult::Unsatisfiable => CheckResult::Valid,
        SatResult::Unknown => {
            CheckResult::Unknown("solver exhausted its conflict budget".to_string())
        }
        SatResult::Satisfiable(model) => {
            let decoded = decode_model(bv, &var_bits, &model);
            if falsified(bv, &decoded) {
                CheckResult::Counterexample(decoded)
            } else {
                CheckResult::Unknown(
                    "solver reported SAT but the model does not refute the claim".to_string(),
                )
            }
        }
    }
}

/// Prove `lhs == rhs` for all inputs.
pub fn prove_equal(bv: &Bv, lhs: Term, rhs: Term) -> CheckResult {
    assert_eq!(bv.width(lhs), bv.width(rhs), "prove_equal: width mismatch");
    let mut enc = Encoder::new(bv);
    let a = enc.encode(lhs);
    let b = enc.encode(rhs);
    assert_difference(&mut enc, &a, &b);
    conclude(bv, enc, move |bv, model| {
        bv.eval(lhs, model) != bv.eval(rhs, model)
    })
}

/// Prove that a 1-bit term is true for all inputs.
pub fn prove_tautology(bv: &Bv, t: Term) -> CheckResult {
    assert_eq!(bv.width(t), 1, "prove_tautology: expected a 1-bit term");
    let mut enc = Encoder::new(bv);
    let bits = enc.encode(t);
    enc.cnf_mut().add_clause([-bits[0]]);
    conclude(bv, enc, move |bv, model| bv.eval(t, model) == 0)
}

/// Prove `assumptions ⇒ goal`, where all terms are 1-bit.
pub fn prove_implication(bv: &Bv, assumptions: &[Term], goal: Term) -> CheckResult {
    assert_eq!(bv.width(goal), 1, "prove_implication: goal must be 1 bit");
    let mut enc = Encoder::new(bv);
    for &a in assumptions {
        assert_eq!(
            bv.width(a),
            1,
            "prove_implication: assumptions must be 1 bit"
        );
        let bits = enc.encode(a);
        enc.cnf_mut().add_clause([bits[0]]);
    }
    let goal_bits = enc.encode(goal);
    enc.cnf_mut().add_clause([-goal_bits[0]]);
    conclude(bv, enc, move |bv, model| {
        assumptions.iter().all(|&a| bv.eval(a, model) != 0) && bv.eval(goal, model) == 0
    })
}

/// Prove that a 1-bit term is false for all inputs.
pub fn prove_contradiction(bv: &Bv, t: Term) -> CheckResult {
    assert_eq!(bv.width(t), 1, "prove_contradiction: expected 1 bit");
    let mut enc = Encoder::new(bv);
    let bits = enc.encode(t);
    enc.cnf_mut().add_clause([bits[0]]);
    conclude(bv, enc, move |bv, model| bv.eval(t, model) != 0)
}

/// Prove `assumptions ⇒ lhs = rhs`, where the assumptions are 1-bit
/// terms. Used for case-conditioned identities: a propositional fact
/// may only hold under the case's guard.
pub fn prove_implied_equal(bv: &Bv, assumptions: &[Term], lhs: Term, rhs: Term) -> CheckResult {
    assert_eq!(bv.width(lhs), bv.width(rhs), "prove_implied_equal: widths");
    let mut enc = Encoder::new(bv);
    for &a in assumptions {
        assert_eq!(bv.width(a), 1, "prove_implied_equal: assumption width");
        let bits = enc.encode(a);
        enc.cnf_mut().add_clause([bits[0]]);
    }
    let l = enc.encode(lhs);
    let r = enc.encode(rhs);
    assert_difference(&mut enc, &l, &r);
    conclude(bv, enc, move |bv, model| {
        assumptions.iter().all(|&a| bv.eval(a, model) != 0)
            && bv.eval(lhs, model) != bv.eval(rhs, model)
    })
}
