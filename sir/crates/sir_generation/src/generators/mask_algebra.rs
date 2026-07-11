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

    if concepts.contains(&SemanticConcept::ClearLowestSetBit) {
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

    candidates
}
