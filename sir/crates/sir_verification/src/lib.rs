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
