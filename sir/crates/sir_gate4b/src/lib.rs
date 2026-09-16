//! Gate 4B — mechanically checked concrete end-to-end equivalence.
//!
//! The gate requires (docs/GATE4_RESULTS.md, docs/IMMEDIATE_PROGRAM.md):
//!
//! 1. the actual SIR region is mechanically bound to the theorem's operands;
//! 2. the actual generated vector plan is encoded (not an idealized stand-in);
//! 3. the per-chunk identities are discharged by a solver/proof checker;
//! 4. the loop invariant covers arbitrary valid `n` mechanically;
//! 5. the tail decomposition is proven for all remainders;
//! 6. the target intrinsic semantics are modeled;
//! 7. memory and overflow semantics match the source;
//! 8. a reproducible proof artifact exists.
//!
//! This crate builds that proof on top of the [`sir_mech`] kernel. The
//! generated C files under `gate4/` are read from disk and matched
//! against strict, fail-closed templates, so the proof is about the
//! committed artifacts (byte-level digests are part of the artifact),
//! not a hand-written stand-in.

pub mod artifact;
pub mod binding;
pub mod bounds;
pub mod emitted;
pub mod loops;
pub mod model;
pub mod pred;
pub mod proof;

pub use artifact::{Artifact, LemmaRecord};
pub use emitted::{EmittedKernel, KernelKind, ParseError};
pub use proof::Gate4bProof;
