use std::collections::HashSet;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use sir_transform::ids::DefinitionId;
use sir_transform::representation::Representation;
use sir_types::CostProfile;

use crate::candidate::{CandidateEffect, CandidateExplanation, ImplementationStrategy};

/// Generate candidates for `MaskAlgebra` representations.
pub fn generate(context: &TransformationContext, concepts: &HashSet<SemanticConcept>) -> Vec<(
    ImplementationStrategy,
    CandidateExplanation,
    Vec<CandidateEffect>,
    CostProfile,
    DefinitionId,
)> {
    let mut candidates = Vec::new();

    if context.representation != Representation::MaskAlgebra {
        return candidates;
    }

    // The ClearLowestSetBit inside a BitsetIteration loop is loop structure
    // (the iteration primitive), not a standalone idiom: a sub-expression
    // rewrite would fragment the loop before the wholesale loop-eliminating
    // strategies (Popcount/Parity) can run, and the cost model cannot compare
    // the two fairly. Only offer blsr for standalone `x & (x - 1)` regions.
    if concepts.contains(&SemanticConcept::ClearLowestSetBit)
        && !concepts.contains(&SemanticConcept::BitsetIteration)
    {
        let def_id = DefinitionId::new(300);

        let strategy = ImplementationStrategy::ClearLowestBit;
        let explanation = CandidateExplanation {
            source_concepts: vec![SemanticConcept::ClearLowestSetBit],
            rationale: "x & (x - 1) matches clear lowest set bit idiom",
        };
        let effects = vec![CandidateEffect::InstructionSubstitution];

        let mut expected_cost = CostProfile::default();
        expected_cost.instruction_count = 1;

        candidates.push((strategy, explanation, effects, expected_cost, def_id));
    }

    if concepts.contains(&SemanticConcept::LowestSetBit) {
        let def_id = DefinitionId::new(301);

        let strategy = ImplementationStrategy::IsolateLowestBit;
        let explanation = CandidateExplanation {
            source_concepts: vec![SemanticConcept::LowestSetBit],
            rationale: "x & -x matches isolate lowest set bit idiom",
        };
        let effects = vec![CandidateEffect::InstructionSubstitution];

        let mut expected_cost = CostProfile::default();
        expected_cost.instruction_count = 1;

        candidates.push((strategy, explanation, effects, expected_cost, def_id));
    }

    if concepts.contains(&SemanticConcept::LowestClearBitMask) {
        let def_id = DefinitionId::new(302);

        let strategy = ImplementationStrategy::IsolateLowestClearBit;
        let explanation = CandidateExplanation {
            source_concepts: vec![SemanticConcept::LowestClearBitMask],
            rationale: "~x & (x + 1) matches isolate lowest clear bit idiom",
        };
        let effects = vec![CandidateEffect::InstructionSubstitution];

        let mut expected_cost = CostProfile::default();
        expected_cost.instruction_count = 2; // not + blsi

        candidates.push((strategy, explanation, effects, expected_cost, def_id));
    }

    if concepts.contains(&SemanticConcept::SetLowestClearBit) {
        let def_id = DefinitionId::new(303);

        let strategy = ImplementationStrategy::SetLowestClearBit;
        let explanation = CandidateExplanation {
            source_concepts: vec![SemanticConcept::SetLowestClearBit],
            rationale: "x | (x + 1) matches set lowest clear bit idiom",
        };
        let effects = vec![CandidateEffect::InstructionSubstitution];

        let mut expected_cost = CostProfile::default();
        expected_cost.instruction_count = 3; // not + blsmsk + or

        candidates.push((strategy, explanation, effects, expected_cost, def_id));
    }

    candidates
}
