pub mod arithmetic;
pub mod bitscan;
mod bitset;
pub mod mask_algebra;
pub mod permutation;

use crate::candidate::Candidate;
use sir_semantics::authorization::{is_data_concept, AuthorizationDatabase};
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use std::collections::HashSet;

/// Run all generators and collect their candidates.
///
/// ── Authorization gate (advisor directive, Gate 6A-v2 finding X06) ──
/// Candidates are admitted ONLY when every operation concept they cite
/// is authorized for the region by a complete certificate. Data
/// concepts (`is_data_concept`) are descriptive and always permitted.
/// Uncertified regions must yield zero candidates.
///
/// Returns the authorized candidates.
pub fn all_plans(
    context: &TransformationContext,
    concepts: &HashSet<SemanticConcept>,
    auth_db: &AuthorizationDatabase,
) -> Vec<Candidate> {
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

    for (strategy, explanation, effects, expected_cost, definition_id) in permutation::generate(context, concepts) {
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

    // ── Authorization gate (X06 structural fix) ──
    // No candidate may exist without an authorization derived from a
    // complete certificate. This choke point makes uncertified candidate
    // construction structurally difficult: the generator refuses whole
    // regions without certificates, and per-candidate coverage checks
    // keep operation families from consuming unrelated evidence.
    let authorized = auth_db.authorized_concepts(context.region);
    candidates.retain(|cand| {
        cand.explanation.source_concepts.iter().all(|concept| {
            is_data_concept(concept) || authorized.contains(concept)
        })
    });

    candidates
}
