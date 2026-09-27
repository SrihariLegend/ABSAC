//! Fact types and the unified `FactDatabase`.
//!
//! Only loop facts survive: recognizers, binding and the rewrite engine
//! consume trip counts, finiteness, nesting and carried/reduction
//! variables. Every other former analysis was dead weight.

use sir_types::NodeId;
use std::collections::HashMap;

/// A reduction variable detected in a loop.
#[derive(Clone, Debug, PartialEq)]
pub struct ReductionVar {
    /// The NodeId of the carried variable.
    pub variable: NodeId,
    /// The kind of reduction (e.g., "sum", "product", "xor", "or", "and").
    pub reduction_kind: String,
    /// The loop-invariant value combined in each iteration.
    pub invariant_value: NodeId,
}

/// Loop analysis facts.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopFact {
    /// The constant the counter is compared against, if the termination
    /// compares a carried variable against one.
    ///
    /// **This is a bound, not an iteration count, and it is not a
    /// correctness gate.** The runtime-bounded families read it as a
    /// *dispatch* decision — "the bound is a compile-time constant, so let
    /// the static family take this loop" — and nothing is unsound if it is
    /// wrong in that role.
    ///
    /// It must never be consumed as a count. It equals the number of
    /// iterations only when the counter starts at zero and steps by one,
    /// and this analysis checks neither, which is how `for (i = 5; i < 64;
    /// i++)` — nineteen fewer iterations than claimed — came to be reported
    /// as 64 and accepted by five families. Callers needing a count want
    /// `loop_shape::checked_counted_loop`; callers needing an index extent
    /// want `loop_shape::counted_bound`; callers deciding which family
    /// should handle a loop want this, and only this.
    pub constant_bound: Option<u64>,
    /// Whether this loop is nested inside another loop.
    pub is_nested: bool,
    /// Carried variables (values passed from one iteration to the next).
    pub carried: Vec<NodeId>,
    /// Reduction variables detected in the loop.
    pub reductions: Vec<ReductionVar>,
}

/// The unified fact database.
#[derive(Clone, Debug, Default)]
pub struct FactDatabase {
    /// Loop facts, keyed by the `Loop` node id.
    pub loops: HashMap<NodeId, LoopFact>,
}

impl FactDatabase {
    /// Create an empty fact database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return true if no facts are stored.
    pub fn is_empty(&self) -> bool {
        self.loops.is_empty()
    }

    /// Total number of facts across analyses.
    pub fn total_facts(&self) -> usize {
        self.loops.len()
    }
}
