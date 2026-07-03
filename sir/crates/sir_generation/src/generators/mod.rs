pub mod bit_iteration;
pub mod popcount;
pub mod packed_bitfield;
pub mod mask_construction;

use sir_transform::context::TransformationContext;
use crate::candidate::Candidate;

/// Run all generators and collect their candidates.
pub fn all_plans(context: &TransformationContext) -> Vec<Candidate> {
    let mut candidates = Vec::new();

    if let Some(c) = bit_iteration::plan(context) { candidates.push(c); }
    if let Some(c) = popcount::plan(context) { candidates.push(c); }
    if let Some(c) = packed_bitfield::plan(context) { candidates.push(c); }
    if let Some(c) = mask_construction::plan(context) { candidates.push(c); }

    candidates
}
