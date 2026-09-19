//! Runs the loop analysis and owns the fact database.

use sir_nodes::Function;
use std::time::Instant;

use crate::facts::FactDatabase;
use crate::loops;

/// Statistics collected across analysis runs.
#[derive(Clone, Debug, Default)]
pub struct AnalysisStats {
    /// Total number of analysis runs.
    pub total_runs: usize,
    /// Total wall-clock time spent in analysis.
    pub total_runtime_ms: u64,
    /// Total nodes processed.
    pub total_nodes_processed: usize,
}

/// Owns the fact database and runs the loop analysis.
pub struct AnalysisManager {
    db: FactDatabase,
    stats: AnalysisStats,
}

impl AnalysisManager {
    /// Create a new analysis manager with an empty database.
    pub fn new() -> Self {
        Self {
            db: FactDatabase::new(),
            stats: AnalysisStats::default(),
        }
    }

    /// Get the fact database (read-only).
    pub fn database(&self) -> &FactDatabase {
        &self.db
    }
    /// Run the loop analysis on `func` and populate the database.
    pub fn run_all(&mut self, func: &Function) {
        let start = Instant::now();
        self.db.loops = loops::run_loops(func);
        self.stats.total_runs += 1;
        self.stats.total_runtime_ms += start.elapsed().as_millis() as u64;
        self.stats.total_nodes_processed += func.node_count();
    }
}

impl Default for AnalysisManager {
    fn default() -> Self {
        Self::new()
    }
}
