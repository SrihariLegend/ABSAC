//! SIR Inference — Representation Beliefs v0.1
//!
//! Accumulates evidence from semantic truths and forms representation
//! hypotheses. This is where heuristics and weights live.
//!
//! This is Layer 3 of the knowledge hierarchy:
//!   Facts (sir_analysis) → Truths (sir_semantics) → Beliefs (sir_inference)

pub mod evidence;
pub mod hypothesis;
pub mod engine;
pub mod sources;
