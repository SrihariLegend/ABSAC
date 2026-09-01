pub mod arithmetic;
pub mod bitscan;
mod bitset;
pub mod mask_algebra;
pub mod permutation;

use crate::candidate::{AuthorizationRef, Candidate, UntrustedProposal};
use sir_semantics::authorization::{is_data_concept, AuthorizationDatabase};
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use std::collections::HashSet;

/// Run all generators and collect authorized candidates.
///
/// ── Trust boundary (advisor directive, P0A hardening items 2+3) ──
///
/// Generators produce `UntrustedProposal` values: derived from truths,
/// beliefs and context structure. They CANNOT enter the candidate
/// database, selection, verification or rewriting — there is no public
/// path from a proposal to a `Candidate` except through
/// `UntrustedProposal::authorize`, which requires a matched
/// `TransformationAuthorization`.
///
/// Admission rule: every operation concept a proposal cites must be
/// either a descriptive data concept (`is_data_concept`) or covered by
/// an authorization for the proposal's region. The authorization's
/// fingerprint and region are then attached to the candidate and travel
/// with it through verification, selection and rewriting.
///
/// Uncertified regions yield zero candidates.
pub fn all_plans(
    context: &TransformationContext,
    concepts: &HashSet<SemanticConcept>,
    auth_db: &AuthorizationDatabase,
    function_fingerprint: u64,
) -> Vec<Candidate> {
    // ── Stage 1: untrusted proposals (never escape this crate) ──
    let mut proposals: Vec<UntrustedProposal> = Vec::new();

    // ── Stage 1: untrusted proposals (never escape this crate) ──
    let mut proposals: Vec<UntrustedProposal> = Vec::new();

    proposals.extend(bitset::all_bitset_plans(context, concepts));
    proposals.extend(arithmetic::all_arithmetic_plans(context, concepts));
    proposals.extend(bitscan::all_bitscan_plans(context, concepts));

    for (strategy, explanation, effects, expected_cost, definition_id) in mask_algebra::generate(context, concepts) {
        let cites = explanation.source_concepts.clone();
        proposals.push(UntrustedProposal::new(
            context.region,
            context.context_id,
            definition_id,
            strategy,
            explanation,
            effects,
            expected_cost,
            context.representation,
            context.source_structure.clone(),
            context.constraints.clone(),
            context.assumptions.clone(),
            cites,
        ));
    }

    for (strategy, explanation, effects, expected_cost, definition_id) in permutation::generate(context, concepts) {
        let cites = explanation.source_concepts.clone();
        proposals.push(UntrustedProposal::new(
            context.region,
            context.context_id,
            definition_id,
            strategy,
            explanation,
            effects,
            expected_cost,
            context.representation,
            context.source_structure.clone(),
            context.constraints.clone(),
            context.assumptions.clone(),
            cites,
        ));
    }

    // ── Authorization gate ──
    // A proposal is admitted only when EVERY concept it cites is
    // covered: either a descriptive data concept, or an authorized
    // concept of a certificate-bound authorization for this region.
    let region_auths = auth_db.for_region(context.region);
    let authorized = auth_db.authorized_concepts(context.region);

    let mut candidates = Vec::new();
    for proposal in proposals {
        let cites = proposal.source_concepts.clone();
        let ok = cites.iter().all(|c| is_data_concept(c) || authorized.contains(c));
        if !ok {
            continue; // proposal stays untrusted; no Candidate is minted
        }

        // Record which domains cover this proposal (provenance).
        let mut domains = Vec::new();
        for auth in region_auths {
            if cites.iter().any(|c| auth.authorized_concepts.contains(c)) {
                if !domains.contains(&auth.domain.kind()) {
                    domains.push(auth.domain.kind());
                }
            }
        }

        candidates.push(proposal.authorize(
            crate::candidate::CandidateId::new(0), // assigned by database
            AuthorizationRef {
                function_fingerprint,
                region: context.region,
                domains,
            },
        ));
    }

    candidates
}
