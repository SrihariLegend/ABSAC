use std::collections::HashSet;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::ids::DefinitionId;
use sir_transform::representation::Representation;
use sir_transform::roles::ShiftDirection;
use sir_types::CostProfile;

use crate::candidate::{CandidateEffect, CandidateExplanation, ImplementationStrategy};

/// Generate candidates for `BitPermutation` representations.
///
/// The representation is the semantic transformation (a permutation of bit
/// positions); the instruction (`rol`/`ror`) is only selected here once the
/// direction is known. Direction travels as a `RotationDirection` constraint
/// attached by the semantic layer, so this generator never inspects the
/// physical graph.
pub fn generate(context: &TransformationContext, concepts: &HashSet<SemanticConcept>) -> Vec<(
    ImplementationStrategy,
    CandidateExplanation,
    Vec<CandidateEffect>,
    CostProfile,
    DefinitionId,
)> {
    let mut candidates = Vec::new();

    if context.representation != Representation::BitPermutation {
        return candidates;
    }

    if !concepts.contains(&SemanticConcept::CircularPermutation) {
        return candidates;
    }

    // Only the derived CircularPermutation concept triggers a candidate: the
    // intermediate shift stages are never rewritten on their own.
    let direction = if context
        .constraints
        .contains(&Constraint::RotationDirection(ShiftDirection::Left))
    {
        Some(ShiftDirection::Left)
    } else if context
        .constraints
        .contains(&Constraint::RotationDirection(ShiftDirection::Right))
    {
        Some(ShiftDirection::Right)
    } else {
        None
    };

    let mut expected_cost = CostProfile::default();
    expected_cost.instruction_count = 1;

    if direction == Some(ShiftDirection::Left) {
        candidates.push((
            ImplementationStrategy::RotateLeft,
            CandidateExplanation {
                source_concepts: vec![SemanticConcept::CircularPermutation],
                rationale: "left-rotating shift pair matches rotate-left instruction",
            },
            vec![CandidateEffect::InstructionSubstitution],
            expected_cost.clone(),
            DefinitionId::new(310),
        ));
    } else if direction == Some(ShiftDirection::Right) {
        candidates.push((
            ImplementationStrategy::RotateRight,
            CandidateExplanation {
                source_concepts: vec![SemanticConcept::CircularPermutation],
                rationale: "right-rotating shift pair matches rotate-right instruction",
            },
            vec![CandidateEffect::InstructionSubstitution],
            expected_cost,
            DefinitionId::new(311),
        ));
    }

    candidates
}
