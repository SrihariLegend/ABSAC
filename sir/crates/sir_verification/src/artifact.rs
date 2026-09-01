//! Checker-issued verification artifacts (advisor items 1 and 3).
//!
//! Authority model (PS002 canonical lesson — a true theorem applied to
//! the wrong observable boundary is still an incorrect compiler
//! transformation):
//!
//! ```text
//! Definition          → creates theorem obligation; CANNOT issue assurance
//! Theorem checker     → checks obligation → issues CheckedTheorem
//! Application checker → checks bindings/live-outs/frame → (pending)
//! Verification engine → matches both artifacts → EndToEnd (pending)
//! ```
//!
//! Assurance is ISSUED by the checker as
//! `min(definition-declared cap, backend capability)`:
//!
//! - The definition's `verification_status()` is a CAP only. A
//!   definition author cannot raise issued assurance by declaring a
//!   higher level.
//! - A backend's `max_assurance()` is what that backend's method can
//!   establish, independent of any declaration.
//! - The verifier issues and gates on the ISSUED level. Only the
//!   issued artifact authorizes mutation.
//!
//! Backend capability classification (strict, advisor item on
//! assurance levels):
//!
//! - Symbolic (handwritten algebraic normalization rules): at most
//!   SchemaChecked.
//! - Exhaustive (enumerates the obligation's declared finite domain;
//!   refuses oversized/partial domains): ConcreteSolverChecked — the
//!   same strength class as an SMT UNSAT on the exact finite
//!   obligation. Replay = re-enumerate with the recorded limits.
//! - MachineChecked: reserved for a proof artifact replayed by a
//!   trusted checker/prover. Nothing issues it yet.
//!
//! Bounded enumeration that does NOT exhaust the complete claimed
//! domain is TestedOnly at best — and since the exhaustive backend
//! refuses domains above its limits, a Proven from it always covered
//! the declared domain in full.

use crate::registry::VerificationStatus;
use crate::VerificationBackend;

impl crate::VerificationBackend {
    /// The maximum assurance this backend can ESTABLISH (checker
    /// capability — independent of any definition's declaration).
    pub fn max_assurance(&self) -> VerificationStatus {
        match self {
            VerificationBackend::Symbolic => VerificationStatus::SchemaChecked,
            VerificationBackend::Exhaustive => VerificationStatus::ConcreteSolverChecked,
        }
    }
}

/// FNV-1a 64-bit digest over a canonical byte string.
/// Digest-as-diagnostic authority (deterministic within a build).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
