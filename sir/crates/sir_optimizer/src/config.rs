// sir/crates/sir_optimizer/src/config.rs

/// Configuration for the fixed-point optimization driver.
#[derive(Clone, Debug)]
pub struct OptimizerConfig {
    /// Maximum fixed-point iterations before terminating.
    pub max_iterations: usize,

    /// Stop after this many total rewrites across all iterations.
    /// Safety valve against rewrite oscillation bugs.
    pub max_total_rewrites: Option<usize>,

    /// Maximum number of alternative rewrite paths to explore in parallel.
    pub beam_width: Option<usize>,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_total_rewrites: None,
            beam_width: Some(3),
        }
    }
}
