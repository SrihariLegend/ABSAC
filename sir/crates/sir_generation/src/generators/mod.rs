pub mod arithmetic;
pub mod bitscan;
mod bitset;

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
    bitset::all_bitset_plans(context, concepts)
        .into_iter()
        .chain(arithmetic::all_arithmetic_plans(context, concepts))
        .chain(bitscan::all_bitscan_plans(context, concepts))
}
