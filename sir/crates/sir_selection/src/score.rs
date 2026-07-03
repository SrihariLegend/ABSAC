use sir_generation::candidate::{CandidateId, ImplementationStrategy};
use sir_types::RegionId;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScoreBreakdown {
    pub instruction_delta: i64, // positive = fewer instructions
    pub select_delta: i64,      // positive = fewer Select ops
    pub memory_delta: i64,      // positive = fewer memory accesses
    pub depth_delta: i64,       // positive = shallower critical path
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TransformationScore {
    pub candidate: CandidateId,
    pub strategy: ImplementationStrategy,
    pub total: i64,
    pub breakdown: ScoreBreakdown,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CostModelReport {
    pub region: RegionId,
    pub scores: Vec<TransformationScore>, // sorted highest first
}

impl fmt::Display for CostModelReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Region {}", self.region.0)?;
        for score in &self.scores {
            writeln!(
                f,
                "  {:<16} {:>+3}",
                score.strategy.to_string(),
                score.total
            )?;
        }
        if let Some(winner) = self.scores.first() {
            if winner.total > 0 {
                write!(f, "  Winner: {}", winner.strategy)?;
            } else {
                write!(f, "  Winner: None (all rejected)")?;
            }
        } else {
            write!(f, "  Winner: None")?;
        }
        Ok(())
    }
}
