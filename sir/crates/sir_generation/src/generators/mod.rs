mod bitset;
pub mod arithmetic;

use std::collections::HashSet;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::context::TransformationContext;
use crate::candidate::Candidate;

/// Run all generators and collect their candidates.
///
/// Returns an iterator over candidates.
pub fn all_plans<'a>(
    context: &'a TransformationContext,
    concepts: &'a HashSet<SemanticConcept>,
) -> impl Iterator<Item = Candidate> + 'a {
    bitset::all_bitset_plans(context, concepts).into_iter()
        .chain(arithmetic::all_arithmetic_plans(context, concepts))
}
