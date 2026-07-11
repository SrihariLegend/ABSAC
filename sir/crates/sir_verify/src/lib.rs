//! SIR Verify — Graph invariant verification for SIR functions.
//!
//! Checks SSA form, reference validity, cycle freedom, type correctness,
//! return validity, parameter consistency, and loop well-formedness.

pub mod error;
pub mod verifier;

pub use error::*;
pub use verifier::*;
