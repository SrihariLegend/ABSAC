//! All — transformation definition for all equivalence.

use sir_generation::candidate::Candidate;
use sir_transform::assumptions::Assumption;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::TransformationDefinition;
use crate::semantic::expression::{Predicate, SemanticExpression};
use crate::semantic::theorem::Theorem;

/// The All transformation: replaces a boolean-array conjunctive loop
/// with a hardware pack and full-mask-check.
///
/// Theorem:
///   All(BooleanArray(v)) ≡ EqualFullMask(Pack(BooleanArray(v)))
///
/// Under assumptions: EquivalentCardinality, PreservesIterationOrder, PreservesLayout.
#[derive(Clone, Debug)]
pub struct AllDefinition {
    id: DefinitionId,
}

impl AllDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for AllDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "All"
    }

    fn applicability(&self, candidate: &Candidate) -> bool {
        // Applicable when the candidate targets BitSet representation
        candidate.representation == Representation::BitSet
    }

    fn obligation(&self, candidate: &Candidate) -> ProofObligation {
        // Synthesize the variable from the region
        let board_var = VariableId::new(0);

        // Determine array length from constraints
        let length = candidate
            .constraints
            .iter()
            .find_map(|c| match c {
                sir_transform::constraints::Constraint::FixedLength(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(64); // default for BS001

        // Build the theorem: LHS = All(BooleanArray(v))
        let lhs = SemanticExpression::All(Box::new(
            SemanticExpression::BooleanArray {
                variable: board_var,
            },
        ));

        // RHS = EqualFullMask(Pack(BooleanArray(v)))
        let rhs = SemanticExpression::EqualFullMask(Box::new(
            SemanticExpression::Pack(Box::new(
                SemanticExpression::BooleanArray {
                    variable: board_var,
                },
            )),
        ));

        let theorem = Theorem::new(lhs, rhs);

        // Build the finite domain for exhaustive verification
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: board_var,
                kind: VariableKind::BooleanArray { length },
            }],
        };

        // Required assumptions that the verifier must prove
        let assumptions = vec![
            Assumption::EquivalentCardinality,
            Assumption::PreservesIterationOrder,
            Assumption::PreservesLayout,
        ];

        ProofObligation {
            id: ObligationId::new(0), // assigned by database on insert
            region: candidate.region,
            candidate: sir_generation::candidate::CandidateId::new(0), // assigned by caller
            definition: self.id,
            theorem,
            assumptions,
            domain: Some(domain),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use sir_transform::constraints::Constraint;
    use sir_transform::structures::SourceStructure;
    use sir_types::RegionId;
    use sir_generation::candidate::{CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy};
    use sir_transform::context::ContextId;
    use std::collections::HashSet;

    fn make_candidate() -> Candidate {
        let mut constraints = HashSet::new();
        constraints.insert(Constraint::FixedLength(64));
        constraints.insert(Constraint::ReadOnly);
        constraints.insert(Constraint::FiniteIteration);

        let mut assumptions = HashSet::new();
        assumptions.insert(Assumption::EquivalentCardinality);

        Candidate {
            id: CandidateId::new(0),
            region: RegionId::new(0),
            context_id: ContextId::new(0),
            definition_id: DefinitionId::new(0),
            strategy: ImplementationStrategy::All,
            explanation: CandidateExplanation {
                source_concepts: vec![],
                rationale: "",
            },
            effects: vec![],
            expected_cost: sir_types::CostProfile {
                instruction_count: 0,
                select_count: 0,
                memory_accesses: 0,
                critical_path_depth: 0,
            },
            representation: Representation::BitSet,
            source_structure: SourceStructure::BooleanArray { length: 64 },
            constraints,
            assumptions,
        }
    }

    #[test]
    fn all_definition_is_applicable_to_bitset() {
        let def = AllDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        assert!(def.applicability(&cand));
    }

    #[test]
    fn all_definition_obligation_has_correct_theorem() {
        let def = AllDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        // LHS: All(BooleanArray(v))
        match &obl.theorem.lhs {
            SemanticExpression::All(inner) => match inner.as_ref() {
                SemanticExpression::BooleanArray { variable } => {
                    assert_eq!(*variable, VariableId::new(0));
                }
                _ => panic!("Expected BooleanArray inside All"),
            },
            _ => panic!("Expected All as LHS root"),
        }

        // RHS: EqualFullMask(Pack(BooleanArray(v)))
        match &obl.theorem.rhs {
            SemanticExpression::EqualFullMask(inner) => match inner.as_ref() {
                SemanticExpression::Pack(inner2) => match inner2.as_ref() {
                    SemanticExpression::BooleanArray { variable } => {
                        assert_eq!(*variable, VariableId::new(0));
                    }
                    _ => panic!("Expected BooleanArray inside Pack"),
                },
                _ => panic!("Expected Pack inside EqualFullMask"),
            },
            _ => panic!("Expected EqualFullMask as RHS root"),
        }
    }

    #[test]
    fn all_definition_obligation_has_required_assumptions() {
        let def = AllDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert!(obl.assumptions.contains(&Assumption::EquivalentCardinality));
        assert!(obl.assumptions.contains(&Assumption::PreservesIterationOrder));
        assert!(obl.assumptions.contains(&Assumption::PreservesLayout));
    }

    #[test]
    fn all_definition_obligation_has_domain() {
        let def = AllDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert!(obl.domain.is_some());
        let domain = obl.domain.unwrap();
        assert_eq!(domain.variables.len(), 1);
        match &domain.variables[0].kind {
            VariableKind::BooleanArray { length } => assert_eq!(*length, 64),
        }
    }

    #[test]
    fn all_definition_obligation_has_correct_definition_id() {
        let def = AllDefinition::new(DefinitionId::new(42));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert_eq!(obl.definition, DefinitionId::new(42));
    }
}
