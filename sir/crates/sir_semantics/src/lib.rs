//! SIR Semantics — Semantic Truths v0.1
//!
//! Transforms compiler facts (`sir_analysis::FactDatabase`) into semantic
//! truths. Entirely deterministic. No heuristics, no confidence scores.
//!
//! This is Layer 2 of the knowledge hierarchy:
//!   Facts (sir_analysis) → Truths (sir_semantics) → Beliefs (sir_inference)

pub mod concepts;
pub mod cost;
pub mod cost_deriver;
pub mod recognizers;
pub mod region;
pub mod semantics;
pub mod structure;
