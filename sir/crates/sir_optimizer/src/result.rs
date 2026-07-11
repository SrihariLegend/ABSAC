// sir/crates/sir_optimizer/src/result.rs

use sir_nodes::Function;

/// The result of a complete optimization run.
#[derive(Clone, Debug)]
pub struct OptimizationResult {
    /// The optimized function.
    pub function: Function,
    /// Number of fixed-point iterations executed.
    pub iterations: usize,
    /// Total rewrites applied across all iterations.
    pub rewrites_applied: usize,
    /// Per-iteration breakdown.
    pub iterations_detail: Vec<IterationRecord>,
    /// Why optimization stopped.
    pub termination: TerminationReason,
}

/// Why the optimization loop terminated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminationReason {
    /// No more rewrites possible — converged.
    FixedPoint,
    /// max_iterations or max_total_rewrites reached.
    IterationLimitReached,
}

/// Statistics for one fixed-point iteration.
#[derive(Clone, Debug, Default)]
pub struct IterationRecord {
    pub iteration: usize,
    pub facts_discovered: usize,
    pub truths_discovered: usize,
    pub beliefs_inferred: usize,
    pub candidates_generated: usize,
    pub proofs_attempted: usize,
    pub proofs_succeeded: usize,
    pub candidates_selected: usize,
    pub rewrites_applied: usize,
    pub concepts_discovered: Vec<String>,
    pub representations_inferred: Vec<String>,
    pub outcome: IterationOutcome,
}

/// What happened in a single iteration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum IterationOutcome {
    /// At least one rewrite was applied.
    RewriteApplied,
    /// Generation produced no candidates.
    NoCandidate,
    /// Candidates were generated but none could be proven equivalent.
    NoProof,
    /// Candidates were proven but none were selected (all had score <= 0).
    NoSelection,
    /// No iteration has run yet.
    #[default]
    NotStarted,
}
