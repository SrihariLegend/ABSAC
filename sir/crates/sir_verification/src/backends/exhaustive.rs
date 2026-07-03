//! Exhaustive — brute-force exhaustive enumeration backend.
//!
//! Enumerates all possible inputs in the finite domain and
//! evaluates both sides of the theorem. Short-circuits on
//! the first mismatch.
//!
//! Serves double duty:
//! - Fallback for finite domains the symbolic verifier cannot handle
//! - Reference oracle — validates the symbolic engine against concrete execution

use crate::errors::{RejectReason, UnknownReason};
use crate::obligation::ProofObligation;
use crate::semantic::interpreter::Interpreter;
use crate::{Proof, ProofStep, VerificationBackend, VerificationLimits, VerificationResult};

/// Exhaustive verification via concrete enumeration.
///
/// Enumerates all possible inputs in the finite domain and
/// evaluates both sides of the theorem. Short-circuits on
/// the first mismatch.
///
/// Serves double duty:
/// - Fallback for finite domains the symbolic verifier cannot handle
/// - Reference oracle — validates the symbolic engine against concrete execution
#[derive(Clone, Debug)]
pub struct ExhaustiveVerifier {
    limits: VerificationLimits,
}

impl ExhaustiveVerifier {
    /// Create an exhaustive verifier with the given limits.
    pub fn new(limits: VerificationLimits) -> Self {
        Self { limits }
    }

    /// Verify a proof obligation via exhaustive enumeration.
    pub fn verify(&self, obligation: &ProofObligation) -> VerificationResult {
        let domain = match &obligation.domain {
            Some(d) => d,
            None => {
                return VerificationResult::Unknown(
                    UnknownReason::NoApplicableBackend,
                );
            }
        };

        let total = match domain.total_states() {
            Some(t) => t,
            None => {
                return VerificationResult::Unknown(
                    UnknownReason::DomainOverflow,
                );
            }
        };

        if total > self.limits.max_states {
            return VerificationResult::Unknown(
                UnknownReason::DomainTooLarge {
                    states: Some(total),
                    max: self.limits.max_states,
                },
            );
        }

        let interpreter = Interpreter;

        for env in domain.enumerate() {
            let lhs_val = match interpreter.evaluate(&obligation.theorem.lhs, &env) {
                Ok(v) => v,
                Err(_) => {
                    return VerificationResult::Unknown(
                        UnknownReason::NoApplicableBackend,
                    );
                }
            };

            let rhs_val = match interpreter.evaluate(&obligation.theorem.rhs, &env) {
                Ok(v) => v,
                Err(_) => {
                    return VerificationResult::Unknown(
                        UnknownReason::NoApplicableBackend,
                    );
                }
            };

            // Short-circuit on first mismatch
            if lhs_val != rhs_val {
                return VerificationResult::Rejected(
                    RejectReason::CounterExample {
                        environment: env,
                        lhs: lhs_val,
                        rhs: rhs_val,
                    },
                );
            }
        }

        VerificationResult::Proven(Proof {
            theorem: obligation.theorem.clone(),
            normalized_theorem: obligation.theorem.clone(), // exhaustive doesn't normalize
            backend: VerificationBackend::Exhaustive,
            steps: vec![ProofStep::ExhaustiveCheck {
                states_checked: total,
            }],
        })
    }
}

impl Default for ExhaustiveVerifier {
    fn default() -> Self {
        Self::new(VerificationLimits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::obligation::{FiniteDomain, VariableKind, VariableSpec};
    use crate::semantic::expression::{Predicate, SemanticExpression};
    use crate::semantic::theorem::Theorem;
    use sir_generation::candidate::CandidateId;
    use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
    use sir_types::RegionId;

    fn make_bs001_obligation_with_length(length: usize) -> ProofObligation {
        let v = VariableId::new(0);
        let lhs = SemanticExpression::Count(Box::new(
            SemanticExpression::Filter {
                input: Box::new(SemanticExpression::BooleanArray { variable: v }),
                predicate: Predicate::True,
            },
        ));
        let rhs = SemanticExpression::Popcount(Box::new(
            SemanticExpression::Pack(Box::new(
                SemanticExpression::BooleanArray { variable: v },
            )),
        ));

        ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(0),
            definition: DefinitionId::new(0),
            theorem: Theorem::new(lhs, rhs),
            assumptions: vec![],
            domain: Some(FiniteDomain {
                variables: vec![VariableSpec {
                    id: v,
                    kind: VariableKind::BooleanArray { length },
                }],
            }),
        }
    }

    #[test]
    fn exhaustive_verifier_proves_bool4() {
        let verifier = ExhaustiveVerifier::new(VerificationLimits { max_states: 1024 });
        let obligation = make_bs001_obligation_with_length(4);
        let result = verifier.verify(&obligation);

        match result {
            VerificationResult::Proven(proof) => {
                assert_eq!(proof.backend, VerificationBackend::Exhaustive);
                match &proof.steps[0] {
                    ProofStep::ExhaustiveCheck { states_checked } => {
                        assert_eq!(*states_checked, 16); // 2^4
                    }
                    _ => panic!("Expected ExhaustiveCheck step"),
                }
            }
            other => panic!("Expected Proven, got {:?}", other),
        }
    }

    #[test]
    fn exhaustive_verifier_rejects_incorrect_theorem() {
        let verifier = ExhaustiveVerifier::new(VerificationLimits { max_states: 1024 });
        let mut obligation = make_bs001_obligation_with_length(4);
        // Deliberately broken: rhs is constant 0
        obligation.theorem.rhs = SemanticExpression::Constant(sir_types::ConstantData::u64(0));

        let result = verifier.verify(&obligation);
        match result {
            VerificationResult::Rejected(RejectReason::CounterExample { .. }) => {}
            other => panic!("Expected Rejected(CounterExample), got {:?}", other),
        }
    }

    #[test]
    fn exhaustive_verifier_short_circuits_on_first_mismatch() {
        let verifier = ExhaustiveVerifier::new(VerificationLimits { max_states: 1024 });
        let mut obligation = make_bs001_obligation_with_length(4);
        // Broken: LHS is Count(BooleanArray), RHS is constant 0
        // First input (all false) produces Count=0 → matches
        // Second input should mismatch
        obligation.theorem.rhs = SemanticExpression::Constant(sir_types::ConstantData::u64(0));

        let result = verifier.verify(&obligation);
        // Should find a counterexample (not all inputs produce Count=0)
        match result {
            VerificationResult::Rejected(RejectReason::CounterExample { .. }) => {}
            other => panic!(
                "Expected Rejected(CounterExample) — short-circuit should find mismatch, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn exhaustive_verifier_unknown_on_large_domain() {
        let verifier = ExhaustiveVerifier::new(VerificationLimits { max_states: 100 });
        let obligation = make_bs001_obligation_with_length(10); // 2^10 = 1024 > 100
        let result = verifier.verify(&obligation);

        match result {
            VerificationResult::Unknown(UnknownReason::DomainTooLarge { .. }) => {}
            other => panic!("Expected Unknown(DomainTooLarge), got {:?}", other),
        }
    }

    #[test]
    fn exhaustive_verifier_unknown_on_no_domain() {
        let verifier = ExhaustiveVerifier::new(VerificationLimits::default());
        let mut obligation = make_bs001_obligation_with_length(4);
        obligation.domain = None;

        let result = verifier.verify(&obligation);
        match result {
            VerificationResult::Unknown(_) => {}
            other => panic!("Expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn cross_validation_symbolic_and_exhaustive_agree_on_bool4() {
        // Both backends must agree on finite domains
        let obligation = make_bs001_obligation_with_length(4);

        let symbolic = crate::backends::symbolic::SymbolicVerifier::new();
        let exhaustive = ExhaustiveVerifier::new(VerificationLimits { max_states: 1024 });

        let sym_result = symbolic.verify(&obligation);
        let exh_result = exhaustive.verify(&obligation);

        match (&sym_result, &exh_result) {
            (VerificationResult::Proven(_), VerificationResult::Proven(_)) => {
                // Both agree — excellent
            }
            _ => panic!(
                "Cross-validation failed: symbolic={:?}, exhaustive={:?}",
                sym_result, exh_result
            ),
        }
    }
}
