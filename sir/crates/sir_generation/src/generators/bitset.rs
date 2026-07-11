use sir_semantics::concepts::SemanticConcept;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::ids::DefinitionId;
use sir_transform::representation::Representation;
use sir_types::CostProfile;
use std::collections::HashSet;

use crate::candidate::{
    Candidate, CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy,
};

/// Data-driven strategy definition for a bitset transformation plan.
struct StrategyDef {
    strategy: ImplementationStrategy,
    source_concepts: &'static [SemanticConcept],
    rationale: &'static str,
    effects: &'static [CandidateEffect],
    definition_id: DefinitionId,
    compute_cost: fn(usize) -> CostProfile,
}

impl StrategyDef {
    fn build(&self, context: &TransformationContext, length: usize) -> Candidate {
        Candidate {
            id: CandidateId::new(0), // assigned by database
            region: context.region,
            context_id: context.context_id,
            strategy: self.strategy,
            definition_id: self.definition_id,
            explanation: CandidateExplanation {
                source_concepts: self.source_concepts.to_vec(),
                rationale: self.rationale,
            },
            effects: self.effects.to_vec(),
            expected_cost: (self.compute_cost)(length),
            representation: context.representation,
            source_structure: context.source_structure.clone(),
            constraints: context.constraints.clone(),
            assumptions: context.assumptions.clone(),
        }
    }
}

static STRATEGIES: &[StrategyDef] = &[
    StrategyDef {
        strategy: ImplementationStrategy::Popcount,
        source_concepts: &[
            SemanticConcept::CardinalityReduction,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Count set bits directly using hardware popcount instruction, \
                    eliminating the counting loop entirely.",
        effects: &[CandidateEffect::CountingStrategyChange],
        definition_id: DefinitionId::new(0),
        compute_cost: |length| CostProfile {
            instruction_count: 2,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 1.max(length.max(1).ilog2() / 6),
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::BitIteration,
        source_concepts: &[
            SemanticConcept::MembershipTraversal,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Iterate over only set bits using trailing-zero count and bit clear, \
                    visiting only populated elements rather than all 64 positions.",
        effects: &[CandidateEffect::TraversalChange],
        definition_id: DefinitionId::new(1),
        compute_cost: |_length| CostProfile {
            instruction_count: 4,
            select_count: 1,
            memory_accesses: 1,
            critical_path_depth: 2,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::PackedBitfield,
        source_concepts: &[
            SemanticConcept::LogicalSequence,
            SemanticConcept::FiniteCollection,
        ],
        rationale: "Replace the bool[64] array with a single u64 value, \
                    reducing memory footprint from 64 bytes to 8 bytes \
                    and enabling bitwise operations on the entire set.",
        effects: &[CandidateEffect::RepresentationChange],
        definition_id: DefinitionId::new(2),
        compute_cost: |length| CostProfile {
            instruction_count: 1.max((length / 8) as u32),
            select_count: 0,
            memory_accesses: 2,
            critical_path_depth: 3,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::MaskConstruction,
        source_concepts: &[
            SemanticConcept::LogicalSequence,
            SemanticConcept::MembershipTraversal,
        ],
        rationale: "Replace boolean predicate evaluation with bitmask construction, \
                    enabling parallel evaluation of multiple conditions via AND/OR/XOR.",
        effects: &[CandidateEffect::PredicateEncodingChange],
        definition_id: DefinitionId::new(3),
        compute_cost: |_length| CostProfile {
            instruction_count: 6,
            select_count: 0,
            memory_accesses: 2,
            critical_path_depth: 2,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::Any,
        source_concepts: &[
            SemanticConcept::DisjunctiveReduction,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Check if any elements are true using a single non-zero comparison against the packed bitset, \
                    eliminating the disjunctive loop entirely.",
        effects: &[CandidateEffect::ReductionStrategyChange],
        definition_id: DefinitionId::new(4),
        compute_cost: |length| CostProfile {
            instruction_count: 2,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 1.max(length.max(1).ilog2() / 6),
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::All,
        source_concepts: &[
            SemanticConcept::ConjunctiveReduction,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Check if all elements are true using a single full-mask comparison against the packed bitset, \
                    eliminating the conjunctive loop entirely.",
        effects: &[CandidateEffect::ReductionStrategyChange],
        definition_id: DefinitionId::new(5),
        compute_cost: |length| CostProfile {
            instruction_count: 2,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 1.max(length.max(1).ilog2() / 6),
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::Parity,
        source_concepts: &[
            SemanticConcept::ExclusiveReduction,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Compute parity of elements using hardware popcount and bitwise AND, \
                    eliminating the exclusive loop entirely.",
        effects: &[CandidateEffect::ReductionStrategyChange],
        definition_id: DefinitionId::new(6),
        compute_cost: |length| CostProfile {
            instruction_count: 3,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 1.max(length.max(1).ilog2() / 6) + 1,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::Popcount,
        source_concepts: &[
            SemanticConcept::CardinalityReduction,
            SemanticConcept::LogicalSequence,
        ],
        rationale: "Count elements matching a predicate by constructing a bitmask and using hardware popcount.",
        effects: &[CandidateEffect::CountingStrategyChange],
        definition_id: DefinitionId::new(0),
        compute_cost: |length| CostProfile {
            instruction_count: 3,
            select_count: 0,
            memory_accesses: 1,
            critical_path_depth: 1.max(length.max(1).ilog2() / 6),
        },
    },
];

/// Generate candidate plans for a BitSet transformation context.
///
/// Returns all applicable strategies when the context targets BitSet
/// representation. Returns an empty vec for any other representation.
pub fn all_bitset_plans(
    context: &TransformationContext,
    concepts: &HashSet<SemanticConcept>,
) -> Vec<Candidate> {
    if context.representation != Representation::BitSet {
        return vec![];
    }

    let length = context
        .constraints
        .iter()
        .find_map(|c| {
            if let Constraint::FixedLength(len) = c {
                Some(*len)
            } else {
                None
            }
        })
        .unwrap_or(64);

    STRATEGIES
        .iter()
        .filter(|s| s.source_concepts.iter().all(|c| concepts.contains(c)))
        .map(|s| s.build(context, length))
        .collect()
}
