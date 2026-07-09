//! Symbolic — symbolic SMT-based verification backend.
//!
//! Normalizes both sides of the theorem to canonical form and
//! compares structurally. Handles infinite domains because
//! it never enumerates inputs.

use crate::errors::{RejectReason, UnknownReason};
use crate::obligation::ProofObligation;
use crate::semantic::normalizer::Normalizer;
use crate::semantic::rules::count_filter_to_popcount::CountFilterToPopcount;
use crate::semantic::rules::exists_to_not_equal_zero::ExistsToNotEqualZero;
use crate::semantic::rules::all_to_equal_full_mask::AllToEqualFullMask;
use crate::semantic::rules::parity_to_bitwise_and_one::ParityToBitwiseAndOne;
use crate::{Proof, ProofStep, VerificationBackend, VerificationResult};

/// Symbolic verification via normalization.
///
/// Normalizes both sides of the theorem to canonical form and
/// compares structurally. Handles infinite domains because
/// it never enumerates inputs.
pub struct SymbolicVerifier {
    normalizer: Normalizer,
}

impl SymbolicVerifier {
    /// Create a symbolic verifier with the built-in BS001 rule.
    pub fn new() -> Self {
        let mut normalizer = Normalizer::new(100);
        normalizer.add_rule(Box::new(CountFilterToPopcount));
        normalizer.add_rule(Box::new(ExistsToNotEqualZero));
        normalizer.add_rule(Box::new(AllToEqualFullMask));
        normalizer.add_rule(Box::new(ParityToBitwiseAndOne));
        Self { normalizer }
    }

    /// Verify a proof obligation via symbolic normalization.
    pub fn verify(&self, obligation: &ProofObligation) -> VerificationResult {
        let (lhs_nf, lhs_steps) = self.normalizer.normalize(&obligation.theorem.lhs);
        let (rhs_nf, rhs_steps) = self.normalizer.normalize(&obligation.theorem.rhs);

        let mut steps: Vec<ProofStep> = lhs_steps;
        steps.extend(rhs_steps);

        if lhs_nf == rhs_nf {
            VerificationResult::Proven(Proof {
                theorem: obligation.theorem.clone(),
                normalized_theorem: crate::semantic::theorem::Theorem::new(
                    lhs_nf,
                    rhs_nf,
                ),
                backend: VerificationBackend::Symbolic,
                steps,
            })
        } else {
            VerificationResult::Unknown(UnknownReason::UnsupportedRule {
                lhs: lhs_nf,
                rhs: rhs_nf,
            })
        }
    }
}

impl Default for SymbolicVerifier {
    fn default() -> Self {
        Self::new()
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

    fn make_bs001_obligation() -> ProofObligation {
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
                    kind: VariableKind::BooleanArray { length: 64 },
                }],
            }),
        }
    }

    #[test]
    fn symbolic_verifier_proves_bs001() {
        let verifier = SymbolicVerifier::new();
        let obligation = make_bs001_obligation();
        let result = verifier.verify(&obligation);

        match result {
            VerificationResult::Proven(proof) => {
                assert_eq!(proof.backend, VerificationBackend::Symbolic);
                assert!(!proof.steps.is_empty(), "Should have normalization steps");
                // At least one normalization step from CountFilterToPopcount
                assert!(proof.steps.iter().any(|s| matches!(
                    s,
                    ProofStep::Normalization { rule: "CountFilterToPopcount", .. }
                )));
                // The normalized theorem should be structurally equal
                assert_eq!(
                    proof.normalized_theorem.lhs,
                    proof.normalized_theorem.rhs
                );
            }
            other => panic!("Expected Proven, got {:?}", other),
        }
    }

    #[test]
    fn symbolic_verifier_returns_unknown_for_inequivalent() {
        let verifier = SymbolicVerifier::new();
        // Theorem: Count(BooleanArray(v)) ≡ Popcount(Pack(BooleanArray(v)))
        // Missing the Filter — the rule won't match the LHS
        let v = VariableId::new(0);
        let lhs = SemanticExpression::Count(Box::new(
            SemanticExpression::BooleanArray { variable: v },
        ));
        let rhs = SemanticExpression::Popcount(Box::new(
            SemanticExpression::Pack(Box::new(
                SemanticExpression::BooleanArray { variable: v },
            )),
        ));

        let mut obl = make_bs001_obligation();
        obl.theorem = Theorem::new(lhs, rhs);

        let result = verifier.verify(&obl);
        match result {
            VerificationResult::Unknown(UnknownReason::UnsupportedRule { .. }) => {}
            other => panic!("Expected Unknown(UnsupportedRule), got {:?}", other),
        }
    }

    #[test]
    fn symbolic_verifier_produces_idempotent_normalized_theorem() {
        let verifier = SymbolicVerifier::new();
        let obligation = make_bs001_obligation();
        let result = verifier.verify(&obligation);

        if let VerificationResult::Proven(proof) = result {
            // Normalizing again should produce the same result
            let (lhs2, steps2) = verifier.normalizer.normalize(&proof.normalized_theorem.lhs);
            assert!(steps2.is_empty(), "Already-normalized form should not change");
            assert_eq!(lhs2, proof.normalized_theorem.lhs);
        } else {
            panic!("Expected Proven");
        }
    }
}
