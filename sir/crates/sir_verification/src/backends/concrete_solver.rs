//! ConcreteSolver — bit-blast the CONCRETE obligation into `sir_mech` and
//! ask its SAT kernel whether the two sides agree for every input.
//!
//! This backend exists for the quarantined arithmetic identities: a
//! definition builds a theorem whose constants and widths come from the
//! candidate's actual region nodes, and the solver proves (or refutes)
//! the bit-level equivalence at the real width. A wrong constant, a
//! swapped operand, or a mutated theorem produces a counterexample —
//! which the template stubs could never do.
//!
//! Fail closed:
//!   - no finite domain (unbound template) => Unknown;
//!   - an expression this lowering does not model => Unknown;
//!   - the solver's budget exhausted => Unknown.

use std::collections::HashMap;

use sir_mech::{Bv, CheckResult, Term, VarId};
use sir_transform::ids::VariableId;
use sir_types::ConstantData;

use crate::errors::{RejectReason, UnknownReason};
use crate::obligation::{ProofObligation, VariableKind};
use crate::registry::VerificationStatus;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::value::{BitVectorValue, Environment, Value};
use crate::{Proof, ProofStep, VerificationBackend, VerificationResult};

pub struct ConcreteSolverVerifier;

impl ConcreteSolverVerifier {
    /// Verify a concrete obligation. Requires a finite domain whose
    /// variables carry bitvector widths.
    pub fn verify(&self, obligation: &ProofObligation) -> VerificationResult {
        let Some(domain) = obligation.domain.as_ref() else {
            return VerificationResult::Unknown(UnknownReason::NoApplicableBackend);
        };

        let mut widths: HashMap<VariableId, u32> = HashMap::new();
        for var in &domain.variables {
            match var.kind {
                VariableKind::BitVector { width } => {
                    widths.insert(var.id, u32::try_from(width).unwrap_or(u32::MAX));
                }
                VariableKind::LogicalSequence { .. } => {
                    // Collection expressions are not part of the
                    // arithmetic lowering; the symbolic/exhaustive
                    // backends own those.
                    return VerificationResult::Unknown(UnknownReason::NoApplicableBackend);
                }
            }
        }

        let mut bv = Bv::new();
        let mut vars: HashMap<VariableId, Term> = HashMap::new();
        let lhs = match lower(&obligation.theorem.lhs, &mut bv, &widths, &mut vars, None) {
            Ok(t) => t,
            Err(_) => {
                return VerificationResult::Unknown(UnknownReason::UnsupportedExpression {
                    expr: obligation.theorem.lhs.clone(),
                })
            }
        };
        let lhs_width = bv.width(lhs);
        let rhs = match lower(&obligation.theorem.rhs, &mut bv, &widths, &mut vars, Some(lhs_width)) {
            Ok(t) => t,
            Err(_) => {
                return VerificationResult::Unknown(UnknownReason::UnsupportedExpression {
                    expr: obligation.theorem.rhs.clone(),
                })
            }
        };
        if bv.width(lhs) != bv.width(rhs) {
            return VerificationResult::Unknown(UnknownReason::UnsupportedRule {
                lhs: obligation.theorem.lhs.clone(),
                rhs: obligation.theorem.rhs.clone(),
            });
        }

        match sir_mech::prove_equal(&bv, lhs, rhs) {
            CheckResult::Valid => VerificationResult::Proven(Proof {
                theorem: obligation.theorem.clone(),
                normalized_theorem: obligation.theorem.clone(),
                backend: VerificationBackend::ConcreteSolver,
                steps: vec![ProofStep::SolverCheck {
                    engine: "sir_mech::prove_equal (bitblast + CDCL)",
                }],
                // Checker-issued fields are stamped by Verifier::verify.
                assurance: VerificationStatus::Stub,
                obligation_digest: 0,
            }),
            CheckResult::Counterexample(model) => {
                let mut env = Environment::new();
                for (&var_id, &width) in &widths {
                    let bits = model.get(&VarId(var_id.0 as u32)).copied().unwrap_or(0);
                    env.bind(var_id, Value::BitVector(BitVectorValue::new(bits as u128, width as usize)));
                }
                let lhs_val = bv.eval(lhs, &model);
                let rhs_val = bv.eval(rhs, &model);
                VerificationResult::Rejected(RejectReason::CounterExample {
                    environment: env,
                    lhs: Value::BitVector(BitVectorValue::new(lhs_val as u128, lhs_width as usize)),
                    rhs: Value::BitVector(BitVectorValue::new(rhs_val as u128, bv.width(rhs) as usize)),
                })
            }
            CheckResult::Unknown(_) => {
                VerificationResult::Unknown(UnknownReason::UnsupportedRule {
                    lhs: obligation.theorem.lhs.clone(),
                    rhs: obligation.theorem.rhs.clone(),
                })
            }
        }
    }
}

/// Lower a SemanticExpression into a sir_mech bitvector term.
///
/// `expected` propagates the width of the surrounding operation so
/// constants adapt to the variable's actual width. Variables must be
/// declared by the obligation's finite domain.
fn lower(
    expr: &SemanticExpression,
    bv: &mut Bv,
    widths: &HashMap<VariableId, u32>,
    vars: &mut HashMap<VariableId, Term>,
    expected: Option<u32>,
) -> Result<Term, ()> {
    match expr {
        SemanticExpression::Variable(id) => {
            if let Some(term) = vars.get(id) {
                return Ok(*term);
            }
            let width = *widths.get(id).ok_or(())?;
            let term = bv.var(VarId(id.0 as u32), width);
            vars.insert(*id, term);
            Ok(term)
        }
        SemanticExpression::Constant(data) => {
            let width = expected.unwrap_or(64);
            Ok(bv.constant(constant_u64(data).ok_or(())?, width))
        }
        SemanticExpression::Add(lhs, rhs) => bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| {
            bv.add(a, b)
        }),
        SemanticExpression::Subtract(lhs, rhs) => bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| {
            bv.sub(a, b)
        }),
        SemanticExpression::Multiply(lhs, rhs) => bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| {
            bv.mul(a, b)
        }),
        SemanticExpression::Divide(lhs, rhs) => bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| {
            bv.udiv(a, b)
        }),
        SemanticExpression::Modulo(lhs, rhs) => bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| {
            bv.urem(a, b)
        }),
        SemanticExpression::BitwiseAnd(lhs, rhs) => {
            bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| bv.and(a, b))
        }
        SemanticExpression::BitwiseOr(lhs, rhs) => {
            bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| bv.or(a, b))
        }
        SemanticExpression::ShiftLeft(lhs, rhs) => {
            bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| bv.shl(a, b))
        }
        SemanticExpression::ShiftRight(lhs, rhs) => {
            bin(bv, widths, vars, lhs, rhs, expected, |bv, a, b| bv.lshr(a, b))
        }
        SemanticExpression::BitwiseNot(inner) => {
            let t = lower(inner, bv, widths, vars, expected)?;
            Ok(bv.not(t))
        }
        SemanticExpression::ClearLowestSetBit(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let one = bv.one(w);
            let minus_one = bv.sub(x, one);
            Ok(bv.and(x, minus_one))
        }
        SemanticExpression::LowestSetBit(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let zero = bv.zero(w);
            let neg = bv.sub(zero, x);
            Ok(bv.and(x, neg))
        }
        SemanticExpression::LowestClearBitMask(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let one = bv.one(w);
            let next = bv.add(x, one);
            let not_x = bv.not(x);
            Ok(bv.and(not_x, next))
        }
        SemanticExpression::SetLowestClearBit(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let one = bv.one(w);
            let next = bv.add(x, one);
            Ok(bv.or(x, next))
        }
        SemanticExpression::RotateLeft(inner, amount) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let k = constant_u64_expected(amount, widths, expected)?;
            if k == 0 {
                return Ok(x);
            }
            if k >= w as u64 {
                return Err(());
            }
            let kt = bv.constant(k, w);
            let w_minus_k = bv.constant(w as u64 - k, w);
            let hi = bv.shl(x, kt);
            let lo = bv.lshr(x, w_minus_k);
            Ok(bv.or(hi, lo))
        }
        SemanticExpression::RotateRight(inner, amount) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let k = constant_u64_expected(amount, widths, expected)?;
            if k == 0 {
                return Ok(x);
            }
            if k >= w as u64 {
                return Err(());
            }
            let kt = bv.constant(k, w);
            let w_minus_k = bv.constant(w as u64 - k, w);
            let lo = bv.lshr(x, kt);
            let hi = bv.shl(x, w_minus_k);
            Ok(bv.or(lo, hi))
        }
        // Full-width byte-order reversal at the variable's declared
        // width (SMT-level construction from shifts/masks; the partial
        // cases add an explicit right shift in the obligation).
        SemanticExpression::ByteSwap(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            if w % 8 != 0 {
                return Err(());
            }
            let bytes = w / 8;
            let mut acc = bv.zero(w);
            for i in 0..bytes {
                let shift = bv.constant(u64::from(8 * i), w);
                let byte = bv.lshr(x, shift);
                let mask = bv.constant(0xFF, w);
                let byte = bv.and(byte, mask);
                let out = bv.constant(u64::from(8 * (bytes - 1 - i)), w);
                let placed = bv.shl(byte, out);
                acc = bv.or(acc, placed);
            }
            Ok(acc)
        }
        // Full-width bit reversal at the variable's declared width.
        SemanticExpression::BitReverse(inner) => {
            let x = lower(inner, bv, widths, vars, expected)?;
            let w = bv.width(x);
            let mut acc = bv.zero(w);
            for i in 0..w {
                let shift = bv.constant(u64::from(i), w);
                let bit = bv.lshr(x, shift);
                let one = bv.one(w);
                let bit = bv.and(bit, one);
                let out = bv.constant(u64::from(w - 1 - i), w);
                let placed = bv.shl(bit, out);
                acc = bv.or(acc, placed);
            }
            Ok(acc)
        }
        // Collections, popcounts and bit scans are not modeled by this
        // lowering yet.
        _ => Err(()),
    }
}

fn bin(
    bv: &mut Bv,
    widths: &HashMap<VariableId, u32>,
    vars: &mut HashMap<VariableId, Term>,
    lhs: &SemanticExpression,
    rhs: &SemanticExpression,
    expected: Option<u32>,
    op: impl FnOnce(&mut Bv, Term, Term) -> Term,
) -> Result<Term, ()> {
    let a = lower(lhs, bv, widths, vars, expected)?;
    let width = bv.width(a);
    let b = lower(rhs, bv, widths, vars, Some(width))?;
    if bv.width(a) != bv.width(b) {
        return Err(());
    }
    Ok(op(bv, a, b))
}

fn constant_u64(data: &ConstantData) -> Option<u64> {
    match data {
        ConstantData::Bool(b) => Some(u64::from(*b)),
        ConstantData::Integer { .. } => data.as_u64(),
        _ => None,
    }
}

/// A rotation amount must be a constant (variable-amount rotations use a
/// different equivalence and are not modeled here yet).
fn constant_u64_expected(
    expr: &SemanticExpression,
    _widths: &HashMap<VariableId, u32>,
    _expected: Option<u32>,
) -> Result<u64, ()> {
    match expr {
        SemanticExpression::Constant(data) => constant_u64(data).ok_or(()),
        _ => Err(()),
    }
}
