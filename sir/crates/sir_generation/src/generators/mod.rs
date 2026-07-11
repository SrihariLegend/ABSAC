pub mod arithmetic;
pub mod bitscan;
mod bitset;
pub mod mask_algebra;

use crate::candidate::Candidate;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use std::collections::HashSet;

/// Run all generators and collect their candidates.
///
/// Returns an iterator over candidates.
pub fn all_plans<'a>(
    context: &'a TransformationContext,
    concepts: &'a HashSet<SemanticConcept>,
) -> impl Iterator<Item = Candidate> + 'a {
    let mut candidates = Vec::new();

    candidates.extend(bitset::all_bitset_plans(context, concepts));
    candidates.extend(arithmetic::all_arithmetic_plans(context, concepts));
    candidates.extend(bitscan::all_bitscan_plans(context, concepts));

    for (strategy, explanation, effects, expected_cost, definition_id) in mask_algebra::generate(context, concepts) {
        let cand = Candidate {
            id: crate::candidate::CandidateId(0), // Will be assigned by CandidateDatabase
            region: context.region,
            context_id: context.context_id,
            definition_id,
            strategy,
            explanation,
            effects,
            expected_cost,
            representation: context.representation,
            source_structure: context.source_structure.clone(),
            constraints: context.constraints.clone(),
            assumptions: context.assumptions.clone(),
        };
        candidates.push(cand);
    }

    candidates.into_iter()
}
