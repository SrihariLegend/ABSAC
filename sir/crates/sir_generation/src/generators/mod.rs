mod bitset;

use sir_transform::context::TransformationContext;
use crate::candidate::Candidate;

/// Run all generators and collect their candidates.
///
/// Returns an iterator over candidates. For v0.1 the only
/// generator is the BitSet handler.
pub fn all_plans(context: &TransformationContext) -> impl Iterator<Item = Candidate> {
    bitset::all_bitset_plans(context).into_iter()
}
