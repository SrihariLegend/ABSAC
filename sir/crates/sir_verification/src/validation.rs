//! AssumptionValidator — validates proof obligation admissibility.
//!
//! Assumptions are admissibility conditions, not proofs.
//! This stage runs before backend verification.

use sir_transform::assumptions::Assumption;
use sir_transform::context::TransformationContext;

use crate::obligation::ProofObligation;

/// Validates that a proof obligation's required assumptions are
/// satisfied by the transformation context.
///
/// Assumptions are admissibility conditions, not proofs.
/// This stage runs before backend verification.
pub struct AssumptionValidator;

impl AssumptionValidator {
    /// Check that all required assumptions are satisfied by the context.
    /// Returns Ok if the obligation is admissible.
    /// Returns Err with the first violated assumption otherwise.
    pub fn validate(
        obligation: &ProofObligation,
        context: &TransformationContext,
    ) -> Result<(), Assumption> {
        for assumption in &obligation.assumptions {
            if !context.assumptions.contains(assumption) {
                return Err(assumption.clone());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::expression::SemanticExpression;
    use crate::semantic::theorem::Theorem;
    use sir_generation::candidate::CandidateId;
    use sir_transform::ids::{DefinitionId, ObligationId};
    use sir_transform::representation::Representation;
    use sir_transform::structures::SourceStructure;
    use sir_types::RegionId;
    use std::collections::HashSet;

    #[test]
    fn assumption_validator_passes_when_all_assumptions_match() {
        let mut assumptions = HashSet::new();
        assumptions.insert(Assumption::EquivalentCardinality);
        assumptions.insert(Assumption::PreservesIterationOrder);

        let ctx = TransformationContext::new(
            RegionId::new(0),
            Representation::BitSet,
            SourceStructure::BooleanArray { length: 64 },
            HashSet::new(),
            assumptions,
        );

        let obl = ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(0),
            definition: DefinitionId::new(0),
            theorem: Theorem::new(
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            ),
            assumptions: vec![
                Assumption::EquivalentCardinality,
                Assumption::PreservesIterationOrder,
            ],
            domain: None,
        };

        assert!(AssumptionValidator::validate(&obl, &ctx).is_ok());
    }

    #[test]
    fn assumption_validator_fails_on_missing_assumption() {
        let mut assumptions = HashSet::new();
        assumptions.insert(Assumption::EquivalentCardinality);
        // Missing: PreservesLayout

        let ctx = TransformationContext::new(
            RegionId::new(0),
            Representation::BitSet,
            SourceStructure::BooleanArray { length: 64 },
            HashSet::new(),
            assumptions,
        );

        let obl = ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(0),
            definition: DefinitionId::new(0),
            theorem: Theorem::new(
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            ),
            assumptions: vec![
                Assumption::EquivalentCardinality,
                Assumption::PreservesLayout, // not in context!
            ],
            domain: None,
        };

        let result = AssumptionValidator::validate(&obl, &ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Assumption::PreservesLayout);
    }

    #[test]
    fn assumption_validator_passes_with_empty_assumptions() {
        let ctx = TransformationContext::new(
            RegionId::new(0),
            Representation::BitSet,
            SourceStructure::BooleanArray { length: 64 },
            HashSet::new(),
            HashSet::new(),
        );

        let obl = ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(0),
            definition: DefinitionId::new(0),
            theorem: Theorem::new(
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
                SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            ),
            assumptions: vec![],
            domain: None,
        };

        assert!(AssumptionValidator::validate(&obl, &ctx).is_ok());
    }
}
