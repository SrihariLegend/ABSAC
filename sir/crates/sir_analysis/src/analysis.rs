//! The Analysis trait and AnalysisResult container.

use std::fmt::Debug;
use std::time::Duration;
use sir_nodes::Function;
use crate::facts::FactDatabase;

/// Every analysis implements this trait.
///
/// Analyses are **read-only** — they inspect the SIR graph and the
/// fact database but never modify either.
pub trait Analysis: Sized {
    /// The output type produced by this analysis.
    type Output: Clone + Debug + Send + Sync + 'static;

    /// Human-readable name for this analysis.
    fn name() -> &'static str;

    /// Run the analysis on a function.
    ///
    /// `facts` is an optional reference to previously-computed facts
    /// that this analysis may depend on.
    fn analyze(func: &Function, facts: Option<&FactDatabase>) -> AnalysisResult<Self::Output>;
}

/// The result of running an analysis.
///
/// Carries the computed data, timing information, node count,
/// and any warnings generated during the analysis.
#[derive(Clone, Debug)]
pub struct AnalysisResult<T> {
    /// The computed data.
    pub data: T,
    /// Wall-clock time for this analysis.
    pub runtime: Duration,
    /// Number of SIR nodes processed.
    pub nodes_processed: usize,
    /// Warnings generated during analysis.
    pub warnings: Vec<String>,
}

impl<T> AnalysisResult<T> {
    /// Create a new analysis result.
    pub fn new(data: T, runtime: Duration, nodes_processed: usize) -> Self {
        Self {
            data,
            runtime,
            nodes_processed,
            warnings: Vec::new(),
        }
    }

    /// Add a warning to the result.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}
