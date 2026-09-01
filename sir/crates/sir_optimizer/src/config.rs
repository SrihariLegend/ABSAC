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

    /// Permit candidates carrying the UNIT_TEST AuthorizationId sentinel
    /// (u64::MAX). Production candidates are minted with real ids by the
    /// AuthorizationDatabase; the sentinel exists only for downstream
    /// unit tests that construct candidates by hand. Default: false —
    /// the optimizer rejects sentinel candidates (advisor sentinel
    /// hardening: a self-consistent forged candidate must not pass the
    /// authorization gate merely because its id is the test sentinel).
    pub allow_unit_test_authorizations: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_total_rewrites: None,
            beam_width: Some(3),
            allow_unit_test_authorizations: false,
        }
    }
}
