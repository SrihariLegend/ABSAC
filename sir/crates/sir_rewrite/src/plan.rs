use sir_verification::Proof;

use crate::patch::ReplacementPatch;
use crate::region::RewriteRegion;

/// An immutable value aggregating everything `RewriteBuilder` needs.
///
/// `RewriteBuilder` knows nothing about candidates, proofs, or recipes —
/// it only executes plans. The engine constructs the plan; the builder
/// consumes it.
#[derive(Clone, Debug)]
pub struct RewritePlan {
    pub region: RewriteRegion,
    pub patch: ReplacementPatch,
    pub proof: Proof,
}
