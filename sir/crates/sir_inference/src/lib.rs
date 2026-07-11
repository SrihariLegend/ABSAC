//! SIR Inference — Representation Beliefs v0.1
//!
//! Accumulates evidence from semantic truths and forms representation
//! hypotheses. Also produces `TransformationContextDatabase` — the
//! sealed contract between inference (understanding) and generation (action).
//!
//! This is Layer 3 of the knowledge hierarchy:
//!   Facts (sir_analysis) → Truths (sir_semantics) → Beliefs (sir_inference)
//!
//! Consumes both `SemanticDatabase` and `StructuralDatabase` to produce
//! hypotheses and transformation contexts.

pub mod engine;
pub mod evidence;
pub mod hypothesis;
pub mod sources;

pub mod concepts {
    pub use sir_semantics::concepts::SemanticConcept;
}
