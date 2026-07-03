use crate::score::{ScoreBreakdown, TransformationScore};
use sir_generation::candidate::Candidate;
use sir_types::CostProfile;

/// Assigns desirability to proven rewrites.
///
/// This is the first subjective layer in the compiler.
/// After verification, there may be multiple provably correct rewrites.
/// The CostModel determines which is *preferred* — it does not determine
/// which is correct (that is verification's job).
pub trait CostModel {
    /// Evaluate a proven candidate and return its score.
    ///
    /// Deltas are computed as: original - expected.
    /// Positive deltas mean improvement.
    fn evaluate(
        &self,
        candidate: &Candidate,
        original: &CostProfile,
        expected: &CostProfile,
    ) -> TransformationScore;
}

/// Simple additive cost model.
///
/// Every reduced instruction, Select operation, memory access,
/// and dependency level contributes equally (+1).
///
/// No architecture-specific weighting is performed.
/// These values are illustrative for the default
/// architecture-independent model and are not intended
/// to represent real hardware performance.
pub struct DefaultCostModel;

impl CostModel for DefaultCostModel {
    fn evaluate(
        &self,
        candidate: &Candidate,
        original: &CostProfile,
        expected: &CostProfile,
    ) -> TransformationScore {
        let instruction_delta =
            original.instruction_count as i64 - expected.instruction_count as i64;
        let select_delta = original.select_count as i64 - expected.select_count as i64;
        let memory_delta = original.memory_accesses as i64 - expected.memory_accesses as i64;
        let depth_delta = original.critical_path_depth as i64 - expected.critical_path_depth as i64;

        let total = instruction_delta + select_delta + memory_delta + depth_delta;

        TransformationScore {
            candidate: candidate.id,
            strategy: candidate.strategy,
            total,
            breakdown: ScoreBreakdown {
                instruction_delta,
                select_delta,
                memory_delta,
                depth_delta,
            },
        }
    }
}
