mod bitset;

use std::collections::HashSet;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use crate::candidate::Candidate;

/// Run all generators and collect their candidates.
///
/// Returns an iterator over candidates. For v0.1 the only
/// generator is the BitSet handler.
pub fn all_plans<'a>(
    context: &'a TransformationContext,
    concepts: &'a HashSet<SemanticConcept>,
) -> impl Iterator<Item = Candidate> + 'a {
    bitset::all_bitset_plans(context, concepts).into_iter()
}
