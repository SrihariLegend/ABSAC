// sir/crates/sir_optimizer/src/lib.rs

//! SIR Optimizer — Fixed-Point Optimization Driver.
//!
//! Orchestrates the full reasoning pipeline iteratively:
//!   Analysis → Semantics → Inference → Generation
//!   → Verification → Selection → Rewrite
//!
//! Runs until fixed point (no more rewrites possible) or iteration limit.
//! All pipeline stages are constructed fresh each iteration.
//! The optimizer never walks SIR or derives knowledge from IR.

pub mod config;
pub mod optimizer;
pub mod result;

pub use config::OptimizerConfig;
pub use optimizer::Optimizer;
pub use result::{IterationOutcome, IterationRecord, OptimizationResult, TerminationReason};
