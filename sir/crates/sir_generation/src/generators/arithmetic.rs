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
        strategy: ImplementationStrategy::BitwiseAnd,
        source_concepts: &[SemanticConcept::ModuloPowerOfTwo],
        rationale:
            "Modulo by a power of two can be computed using a bitwise AND with mask (divisor - 1)",
        effects: &[CandidateEffect::InstructionSubstitution],
        definition_id: DefinitionId::new(100), // unique ID
        compute_cost: || CostProfile {
            instruction_count: 2, // Sub + And
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 2,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::ShiftRight,
        source_concepts: &[SemanticConcept::DividePowerOfTwo],
        rationale: "Division by a power of two can be computed using a bitwise ShiftRight",
        effects: &[CandidateEffect::InstructionSubstitution],
        definition_id: DefinitionId::new(101),
        compute_cost: || CostProfile {
            instruction_count: 2, // TrailingZeros + Shr
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 2,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::ShiftLeft,
        source_concepts: &[SemanticConcept::MultiplyPowerOfTwo],
        rationale: "Multiplication by a power of two can be computed using a bitwise ShiftLeft",
        effects: &[CandidateEffect::InstructionSubstitution],
        definition_id: DefinitionId::new(102),
        compute_cost: || CostProfile {
            instruction_count: 2, // TrailingZeros + Shl
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 2,
        },
    },
    StrategyDef {
        strategy: ImplementationStrategy::MaskExtract,
        source_concepts: &[SemanticConcept::ShiftMask],
        rationale: "Shift left followed by shift right extracts a bitmask",
        effects: &[CandidateEffect::InstructionSubstitution],
        definition_id: DefinitionId::new(103),
        compute_cost: || CostProfile {
            instruction_count: 1, // And
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 1,
        },
    },
];

pub fn all_arithmetic_plans(
    context: &TransformationContext,
    concepts: &HashSet<SemanticConcept>,
) -> Vec<Candidate> {
    if context.representation != Representation::BitwiseArithmetic {
        return vec![];
    }

    STRATEGIES
        .iter()
        .filter(|s| s.source_concepts.iter().all(|c| concepts.contains(c)))
        .map(|s| s.build(context))
        .collect()
}
