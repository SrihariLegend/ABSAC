use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::ids::DefinitionId;
use sir_transform::representation::Representation;
use sir_types::CostProfile;
use std::collections::HashSet;

use crate::candidate::{
    Candidate, CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy,
};

struct StrategyDef {
    strategy: ImplementationStrategy,
    source_concepts: &'static [SemanticConcept],
    rationale: &'static str,
    effects: &'static [CandidateEffect],
    definition_id: DefinitionId,
    compute_cost: fn() -> CostProfile,
}

impl StrategyDef {
    fn build(&self, context: &TransformationContext) -> Candidate {
        Candidate {
            id: CandidateId::new(0),
            region: context.region,
            context_id: context.context_id,
            strategy: self.strategy,
            definition_id: self.definition_id,
            explanation: CandidateExplanation {
                source_concepts: self.source_concepts.to_vec(),
                rationale: self.rationale,
            },
            effects: self.effects.to_vec(),
            expected_cost: (self.compute_cost)(),
            representation: context.representation,
            source_structure: context.source_structure.clone(),
            constraints: context.constraints.clone(),
            assumptions: context.assumptions.clone(),
        }
    }
}

static STRATEGIES: &[StrategyDef] = &[
    StrategyDef {
        strategy: ImplementationStrategy::BitScanForward,
        source_concepts: &[SemanticConcept::FirstOccurrence],
        rationale:
            "Finding the first true element maps to finding the first set bit (BitScanForward)",
        effects: &[
            CandidateEffect::InstructionSubstitution,
            CandidateEffect::TraversalChange,
        ],
        definition_id: DefinitionId::new(200),
        compute_cost: || CostProfile {
            instruction_count: 1, // Hardware tzcnt
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 1,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::BitScanReverse,
        source_concepts: &[SemanticConcept::LastOccurrence],
        rationale:
            "Finding the last true element maps to finding the last set bit (BitScanReverse)",
        effects: &[
            CandidateEffect::InstructionSubstitution,
            CandidateEffect::TraversalChange,
        ],
        definition_id: DefinitionId::new(201),
        compute_cost: || CostProfile {
            instruction_count: 1, // Hardware lzcnt
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 1,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::TrailingZeroCount,
        source_concepts: &[SemanticConcept::TrailingZeroSearch],
        rationale:
            "Scanning for the first non-zero trailing bit maps to hardware TrailingZeroCount",
        effects: &[
            CandidateEffect::InstructionSubstitution,
            CandidateEffect::TraversalChange,
        ],
        definition_id: DefinitionId::new(202),
        compute_cost: || CostProfile {
            instruction_count: 1, // TrailingZeros
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 1,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::LeadingZeroCount,
        source_concepts: &[SemanticConcept::LeadingZeroSearch],
        rationale: "Scanning for the first non-zero leading bit maps to hardware LeadingZeroCount",
        effects: &[
            CandidateEffect::InstructionSubstitution,
            CandidateEffect::TraversalChange,
        ],
        definition_id: DefinitionId::new(203),
        compute_cost: || CostProfile {
            instruction_count: 1, // LeadingZeros
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 1,
        },
    },
];

pub fn all_bitscan_plans(
    context: &TransformationContext,
    concepts: &HashSet<SemanticConcept>,
) -> Vec<Candidate> {
    if context.representation != Representation::BitScan {
        return vec![];
    }

    STRATEGIES
        .iter()
        .filter(|s| s.source_concepts.iter().all(|c| concepts.contains(c)))
        .map(|s| s.build(context))
        .collect()
}
