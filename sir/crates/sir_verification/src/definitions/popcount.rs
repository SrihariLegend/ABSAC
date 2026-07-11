//! Popcount — transformation definition for popcount equivalence.

use sir_generation::candidate::Candidate;
use sir_transform::assumptions::Assumption;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;

use crate::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use crate::registry::TransformationDefinition;
use crate::semantic::expression::{Predicate, SemanticExpression};
use crate::semantic::theorem::Theorem;

/// The Popcount transformation: replaces a boolean-array counting loop
/// with a hardware popcount instruction.
///
/// Theorem:
///   Count(Filter(BooleanArray(v), True)) ≡ Popcount(Pack(BooleanArray(v)))
///
/// Under assumptions: EquivalentCardinality, PreservesIterationOrder, PreservesLayout.
#[derive(Clone, Debug)]
pub struct PopcountDefinition {
    id: DefinitionId,
}

impl PopcountDefinition {
    pub fn new(id: DefinitionId) -> Self {
        Self { id }
    }
}

impl TransformationDefinition for PopcountDefinition {
    fn id(&self) -> DefinitionId {
        self.id
    }

    fn name(&self) -> &'static str {
        "Popcount"
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

        // Depending on whether it's a PredicateCollection or BooleanCollection,
        // we might construct a different theorem. But since Popcount is just over the packed BitVector,
        // and we are abstracting the collection generation, we can leave the mathematical theorem the same
        // by verifying that `Popcount(Pack(BooleanArray(v)))` is equivalent to `Count(Filter(BooleanArray(v), True))`.
        // The Verification Engine models `PredicateCollection` by treating the boolean stream as a `BooleanArray`.

        // Build the theorem: LHS = Count(Filter(BooleanArray(v), True))
        let lhs = SemanticExpression::Count(Box::new(SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray {
                variable: board_var,
            }),
            predicate: Predicate::True,
        }));

        // RHS = Popcount(Pack(BooleanArray(v)))
        let rhs = SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
            SemanticExpression::BooleanArray {
                variable: board_var,
            },
        ))));

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
            strategy: ImplementationStrategy::Popcount,
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
    fn popcount_definition_is_applicable_to_bitset() {
        let def = PopcountDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        assert!(def.applicability(&cand));
    }

    #[test]
    fn popcount_definition_obligation_has_correct_theorem() {
        let def = PopcountDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        // LHS: Count(Filter(BooleanArray(v), True))
        match &obl.theorem.lhs {
            SemanticExpression::Count(inner) => match inner.as_ref() {
                SemanticExpression::Filter { input, predicate } => {
                    assert_eq!(*predicate, Predicate::True);
                    match input.as_ref() {
                        SemanticExpression::BooleanArray { variable } => {
                            assert_eq!(*variable, VariableId::new(0));
                        }
                        _ => panic!("Expected BooleanArray in Filter input"),
                    }
                }
                _ => panic!("Expected Filter inside Count"),
            },
            _ => panic!("Expected Count as LHS root"),
        }

        // RHS: Popcount(Pack(BooleanArray(v)))
        match &obl.theorem.rhs {
            SemanticExpression::Popcount(inner) => match inner.as_ref() {
                SemanticExpression::Pack(inner2) => match inner2.as_ref() {
                    SemanticExpression::BooleanArray { variable } => {
                        assert_eq!(*variable, VariableId::new(0));
                    }
                    _ => panic!("Expected BooleanArray inside Pack"),
                },
                _ => panic!("Expected Pack inside Popcount"),
            },
            _ => panic!("Expected Popcount as RHS root"),
        }
    }

    #[test]
    fn popcount_definition_obligation_has_required_assumptions() {
        let def = PopcountDefinition::new(DefinitionId::new(0));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert!(obl.assumptions.contains(&Assumption::EquivalentCardinality));
        assert!(obl
            .assumptions
            .contains(&Assumption::PreservesIterationOrder));
        assert!(obl.assumptions.contains(&Assumption::PreservesLayout));
    }

    #[test]
    fn popcount_definition_obligation_has_domain() {
        let def = PopcountDefinition::new(DefinitionId::new(0));
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
    fn popcount_definition_obligation_has_correct_definition_id() {
        let def = PopcountDefinition::new(DefinitionId::new(42));
        let cand = make_candidate();
        let obl = def.obligation(&cand);

        assert_eq!(obl.definition, DefinitionId::new(42));
    }
}
