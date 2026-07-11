//! Parity — transformation definition for parity equivalence.

use sir_generation::candidate::Candidate;
use sir_transform::assumptions::Assumption;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::TransformationDefinition;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// The Parity transformation: replaces a boolean-array exclusive loop
/// with a hardware pack, popcount, and bitwise-and-one.
///
/// Theorem:
///   Parity(BooleanArray(v)) ≡ BitwiseAndOne(Popcount(Pack(BooleanArray(v))))
///
/// Under assumptions: EquivalentCardinality, PreservesIterationOrder, PreservesLayout.
#[derive(Clone, Debug)]
pub struct ParityDefinition {
    id: DefinitionId,
}

impl ParityDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for ParityDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Parity"
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

        // Build the theorem: LHS = Parity(BooleanArray(v))
        let lhs = SemanticExpression::Parity(Box::new(SemanticExpression::LogicalSequence {
            variable: board_var,
        }));

        // RHS = BitwiseAndOne(Popcount(Pack(BooleanArray(v))))
        let rhs =
            SemanticExpression::BitwiseAndOne(Box::new(SemanticExpression::Popcount(Box::new(
                SemanticExpression::Pack(Box::new(SemanticExpression::LogicalSequence {
                    variable: board_var,
                })),
            ))));

        let theorem = Theorem::new(lhs, rhs);

        // Build the finite domain for exhaustive verification
        let domain = FiniteDomain {
            variables: vec![VariableSpec {
                id: board_var,
                kind: VariableKind::LogicalSequence { length },
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
    use sir_generation::candidate::{
        CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy,
    };
    use sir_transform::constraints::Constraint;
    use sir_transform::context::ContextId;
    use sir_transform::structures::SourceStructure;
    use sir_types::RegionId;
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
            strategy: ImplementationStrategy::Parity,
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
            source_structure: SourceStructure::LogicalSequence { length: 64 },
            constraints,
            assumptions,
        }
    }

    #[test]
    fn parity_definition_is_applicable_to_bitset() {
        let def = ParityDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        assert!(def.applicability(&cand));
    }

    #[test]
    fn parity_definition_obligation_has_correct_theorem() {
        let def = ParityDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        // LHS: Parity(BooleanArray(v))
        match &obl.theorem.lhs {
            SemanticExpression::Parity(inner) => match inner.as_ref() {
                SemanticExpression::LogicalSequence { variable } => {
                    assert_eq!(*variable, VariableId::new(0));
                }
                _ => panic!("Expected BooleanArray inside Parity"),
            },
            _ => panic!("Expected Parity as LHS root"),
        }

        // RHS: BitwiseAndOne(Popcount(Pack(BooleanArray(v))))
        match &obl.theorem.rhs {
            SemanticExpression::BitwiseAndOne(inner) => match inner.as_ref() {
                SemanticExpression::Popcount(inner2) => match inner2.as_ref() {
                    SemanticExpression::Pack(inner3) => match inner3.as_ref() {
                        SemanticExpression::LogicalSequence { variable } => {
                            assert_eq!(*variable, VariableId::new(0));
                        }
                        _ => panic!("Expected BooleanArray inside Pack"),
                    },
                    _ => panic!("Expected Pack inside Popcount"),
                },
                _ => panic!("Expected Popcount inside BitwiseAndOne"),
            },
            _ => panic!("Expected BitwiseAndOne as RHS root"),
        }
    }

    #[test]
    fn parity_definition_obligation_has_required_assumptions() {
        let def = ParityDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert!(obl.assumptions.contains(&Assumption::EquivalentCardinality));
        assert!(obl
            .assumptions
            .contains(&Assumption::PreservesIterationOrder));
        assert!(obl.assumptions.contains(&Assumption::PreservesLayout));
    }

    #[test]
    fn parity_definition_obligation_has_domain() {
        let def = ParityDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert!(obl.domain.is_some());
        let domain = obl.domain.unwrap();
        assert_eq!(domain.variables.len(), 1);
        match &domain.variables[0].kind {
            VariableKind::LogicalSequence { length } => assert_eq!(*length, 64),
            VariableKind::BitVector { .. } => panic!("Expected LogicalSequence"),
        }
    }

    #[test]
    fn parity_definition_obligation_has_correct_definition_id() {
        let def = ParityDefinition::new(DefinitionId::new(42));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert_eq!(obl.definition, DefinitionId::new(42));
    }
}
