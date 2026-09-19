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
    /// Whether the loop is known to terminate.
    pub is_finite: bool,
    /// Known trip count, if statically determinable.
    pub trip_count: Option<u64>,
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
