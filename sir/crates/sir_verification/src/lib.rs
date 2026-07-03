//! SIR Verification — Equivalence Proof Engine v0.1
//!
//! Proves (or rejects) candidate transformation plans through
//! symbolic normalization and exhaustive enumeration.
//! Never reads or modifies SIR.

pub mod errors;
pub mod obligation;
pub mod registry;
pub mod validation;
pub mod report;

pub mod semantic;
pub mod definitions;
pub mod backends;

use crate::errors::RejectReason;
use crate::errors::UnknownReason;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

/// A completed proof of equivalence.
#[derive(Clone, Debug)]
pub struct Proof {
    /// The original theorem that was proven.
    pub theorem: Theorem,
    /// The theorem after canonicalization (normal forms).
    pub normalized_theorem: Theorem,
    /// Which backend discharged the proof.
    pub backend: VerificationBackend,
    /// The sequence of steps that established equivalence.
    pub steps: Vec<ProofStep>,
}

/// A single step in a proof trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProofStep {
    /// A normalization rule was applied to an expression.
    Normalization {
        rule: &'static str,
        before: SemanticExpression,
        after: SemanticExpression,
    },
    /// Exhaustive enumeration covered all inputs.
    ExhaustiveCheck {
        states_checked: u64,
    },
}

/// Which verification backend discharged a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerificationBackend {
    Symbolic,
    Exhaustive,
}

/// The result of attempting to verify a proof obligation.
#[derive(Clone, Debug)]
pub enum VerificationResult {
    /// The theorem is proven — a proof trace exists.
    Proven(Proof),
    /// The theorem is false — a counterexample or semantic mismatch.
    Rejected(RejectReason),
    /// The verifier cannot determine either way.
    Unknown(UnknownReason),
}

/// Controls which backends are tried and in what order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationPolicy {
    /// Symbolic first, fall back to exhaustive if unknown.
    Default,
    /// Symbolic only — infinite domains, no enumeration.
    SymbolicOnly,
    /// Exhaustive only — requires finite domain.
    ExhaustiveOnly,
}

/// Resource limits for verification backends.
#[derive(Clone, Debug)]
pub struct VerificationLimits {
    /// Maximum states for exhaustive enumeration (default: 1_048_576 = 2^20).
    pub max_states: u64,
}

impl Default for VerificationLimits {
    fn default() -> Self {
        Self {
            max_states: 1_048_576,
        }
    }
}

/// Summary statistics from a verification run.
#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub total: usize,
    pub proven: usize,
    pub rejected: usize,
    pub unknown: usize,
}
