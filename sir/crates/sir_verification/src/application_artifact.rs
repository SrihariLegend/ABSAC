//! Checked theorem/application artifacts for the narrow SIR-level
//! end-to-end rewrite path.
//!
//! The theorem and application dimensions are deliberately separate:
//!
//! - `CheckedTheorem` records what the semantic verifier discharged and
//!   the concrete source/candidate identity the theorem was issued for.
//! - `CheckedApplication` records that the candidate was bound to the
//!   complete source interface and compatible frame.
//! - `EndToEndVerificationArtifact` is constructed only from those two
//!   artifacts and checks every shared identity before mutation.
//!
//! This is SIR-level assurance. It does not prove the later LLVM/vector
//! lowering memory behavior; that requires a separate lowering artifact.

use crate::artifact::fnv1a64;
use crate::Proof;
use crate::VerificationStatus;

/// A theorem artifact with the concrete identity of the theorem's
/// source/candidate application.
///
/// The semantic verifier issues the embedded `Proof`; the application
/// checker supplies the identity fields when it binds that proof to a
/// concrete proposal. Keeping the identity on the theorem artifact is
/// necessary: an obligation digest alone does not identify a source
/// region, candidate, authorization, or role map.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedTheorem {
    pub proof: Proof,
    pub authorization_id: u64,
    pub source_fingerprint: u64,
    pub source_region: u64,
    pub candidate_id: u64,
    pub definition_id: u64,
    pub role_map_digest: u64,
    pub live_out_digest: u64,
    pub source_frame_digest: u64,
    pub candidate_frame_digest: u64,
    pub assumptions_digest: u64,
    /// Digest over the theorem proof and all identity fields.
    pub theorem_digest: u64,
}

impl CheckedTheorem {
    /// Bind a checker-issued proof to the concrete identity held by the
    /// application checker. This constructor does not upgrade theorem
    /// assurance; it only records identity for the pair-matching gate.
    pub(crate) fn new(
        proof: Proof,
        authorization_id: u64,
        source_fingerprint: u64,
        source_region: u64,
        candidate_id: u64,
        definition_id: u64,
        role_map_digest: u64,
        live_out_digest: u64,
        source_frame_digest: u64,
        candidate_frame_digest: u64,
        assumptions_digest: u64,
    ) -> Self {
        let mut theorem = Self {
            proof,
            authorization_id,
            source_fingerprint,
            source_region,
            candidate_id,
            definition_id,
            role_map_digest,
            live_out_digest,
            source_frame_digest,
            candidate_frame_digest,
            assumptions_digest,
            theorem_digest: 0,
        };
        theorem.theorem_digest = theorem.digest();
        theorem
    }

    /// Digest of the exact theorem identity.
    pub fn digest(&self) -> u64 {
        let parts = [
            format!("proof={}", self.proof_digest()),
            format!("obligation={}", self.proof.obligation_digest),
            format!("auth={}", self.authorization_id),
            format!("src={}", self.source_fingerprint),
            format!("region={}", self.source_region),
            format!("cand={}", self.candidate_id),
            format!("definition={}", self.definition_id),
            format!("roles={}", self.role_map_digest),
            format!("liveouts={}", self.live_out_digest),
            format!("source_frame={}", self.source_frame_digest),
            format!("candidate_frame={}", self.candidate_frame_digest),
            format!("assumptions={}", self.assumptions_digest),
            format!("assurance={:?}", self.proof.assurance),
        ];
        fnv1a64(parts.join("|").as_bytes())
    }

    /// Identity digest of the complete checker-issued proof, not just
    /// its obligation number. This detects post-issuance edits to the
    /// theorem expressions or proof trace.
    fn proof_digest(&self) -> u64 {
        fnv1a64(format!("{:?}", self.proof).as_bytes())
    }

    pub fn assurance(&self) -> VerificationStatus {
        self.proof.assurance
    }
}

/// An application-level artifact issued after checking the complete
/// concrete application: live-outs, source frame, candidate frame, and
/// authorized inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedApplication {
    pub authorization_id: u64,
    pub source_fingerprint: u64,
    pub source_region: u64,
    pub candidate_id: u64,
    pub definition_id: u64,
    pub role_map_digest: u64,
    pub live_out_digest: u64,
    pub source_frame_digest: u64,
    pub candidate_frame_digest: u64,
    pub source_frame_supported: bool,
    pub candidate_frame_compatible: bool,
    pub assumptions_digest: u64,
    /// Application-checker assurance. This does not upgrade the
    /// theorem's assurance and is combined with it only at EndToEnd.
    pub assurance: VerificationStatus,
    /// Digest of the exact checked theorem this application checked.
    pub theorem_digest: u64,
    /// Obligation digest retained for readable linkage diagnostics.
    pub theorem_obligation_digest: u64,
    pub application_digest: u64,
}

/// Application-assurance issuance boundary. The rewrite/application
/// checker calls `issue`; recipes and candidates do not construct
/// `CheckedApplication` directly.
pub struct ApplicationChecker;

impl ApplicationChecker {
    pub fn issue(
        authorization_id: u64,
        source_fingerprint: u64,
        source_region: u64,
        candidate_id: u64,
        definition_id: u64,
        role_map_digest: u64,
        live_out_digest: u64,
        source_frame_digest: u64,
        candidate_frame_digest: u64,
        source_frame_supported: bool,
        candidate_frame_compatible: bool,
        assumptions_digest: u64,
        assurance: VerificationStatus,
        theorem_digest: u64,
        theorem_obligation_digest: u64,
    ) -> CheckedApplication {
        // This application checker has no trusted machine-equivalence
        // backend. It may only issue at the SIR SchemaChecked cap.
        let assurance = assurance.min(VerificationStatus::SchemaChecked);
        CheckedApplication::new(
            authorization_id,
            source_fingerprint,
            source_region,
            candidate_id,
            definition_id,
            role_map_digest,
            live_out_digest,
            source_frame_digest,
            candidate_frame_digest,
            source_frame_supported,
            candidate_frame_compatible,
            assumptions_digest,
            assurance,
            theorem_digest,
            theorem_obligation_digest,
        )
    }
}

impl CheckedApplication {
    pub(crate) fn new(
        authorization_id: u64,
        source_fingerprint: u64,
        source_region: u64,
        candidate_id: u64,
        definition_id: u64,
        role_map_digest: u64,
        live_out_digest: u64,
        source_frame_digest: u64,
        candidate_frame_digest: u64,
        source_frame_supported: bool,
        candidate_frame_compatible: bool,
        assumptions_digest: u64,
        assurance: VerificationStatus,
        theorem_digest: u64,
        theorem_obligation_digest: u64,
    ) -> Self {
        let mut application = Self {
            authorization_id,
            source_fingerprint,
            source_region,
            candidate_id,
            definition_id,
            role_map_digest,
            live_out_digest,
            source_frame_digest,
            candidate_frame_digest,
            source_frame_supported,
            candidate_frame_compatible,
            assumptions_digest,
            assurance,
            theorem_digest,
            theorem_obligation_digest,
            application_digest: 0,
        };
        application.application_digest = application.digest();
        application
    }

    pub fn digest(&self) -> u64 {
        let parts = [
            format!("auth={}", self.authorization_id),
            format!("src={}", self.source_fingerprint),
            format!("region={}", self.source_region),
            format!("cand={}", self.candidate_id),
            format!("definition={}", self.definition_id),
            format!("roles={}", self.role_map_digest),
            format!("liveouts={}", self.live_out_digest),
            format!("source_frame={}", self.source_frame_digest),
            format!("candidate_frame={}", self.candidate_frame_digest),
            format!("srcframe={}", self.source_frame_supported),
            format!("candframe={}", self.candidate_frame_compatible),
            format!("assumptions={}", self.assumptions_digest),
            format!("assurance={:?}", self.assurance),
            format!("theorem={}", self.theorem_digest),
            format!("theorem_obligation={}", self.theorem_obligation_digest),
        ];
        fnv1a64(parts.join("|").as_bytes())
    }
}

/// The only artifact that may authorize mutation for this path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndToEndVerificationArtifact {
    pub theorem: CheckedTheorem,
    pub application: CheckedApplication,
    pub assurance: VerificationStatus,
    pub end_to_end_digest: u64,
}

impl EndToEndVerificationArtifact {
    /// Construct a matched theorem/application pair. Every shared
    /// identity is compared; a digest match alone is never sufficient.
    pub fn new(
        theorem: CheckedTheorem,
        application: CheckedApplication,
    ) -> Result<Self, EndToEndMismatch> {
        if theorem.theorem_digest != theorem.digest() {
            return Err(EndToEndMismatch::TheoremDigestInconsistent);
        }
        if application.application_digest != application.digest() {
            return Err(EndToEndMismatch::ApplicationDigestInconsistent);
        }
        if !application.source_frame_supported || !application.candidate_frame_compatible {
            return Err(EndToEndMismatch::ApplicationFrameUnsupported);
        }
        if application.theorem_obligation_digest != theorem.proof.obligation_digest {
            return Err(EndToEndMismatch::TheoremObligationMismatch {
                theorem: theorem.proof.obligation_digest,
                application: application.theorem_obligation_digest,
            });
        }
        if application.theorem_digest != theorem.theorem_digest {
            return Err(EndToEndMismatch::TheoremDigestMismatch {
                theorem: theorem.theorem_digest,
                application: application.theorem_digest,
            });
        }

        let identities = [
            (
                "authorization",
                theorem.authorization_id,
                application.authorization_id,
            ),
            (
                "source_fingerprint",
                theorem.source_fingerprint,
                application.source_fingerprint,
            ),
            (
                "source_region",
                theorem.source_region,
                application.source_region,
            ),
            ("candidate", theorem.candidate_id, application.candidate_id),
            (
                "definition",
                theorem.definition_id,
                application.definition_id,
            ),
            (
                "role_map",
                theorem.role_map_digest,
                application.role_map_digest,
            ),
            (
                "live_out",
                theorem.live_out_digest,
                application.live_out_digest,
            ),
            (
                "source_frame",
                theorem.source_frame_digest,
                application.source_frame_digest,
            ),
            (
                "candidate_frame",
                theorem.candidate_frame_digest,
                application.candidate_frame_digest,
            ),
            (
                "assumptions",
                theorem.assumptions_digest,
                application.assumptions_digest,
            ),
        ];
        if let Some((field, theorem_value, application_value)) = identities
            .into_iter()
            .find(|(_, left, right)| left != right)
        {
            return Err(EndToEndMismatch::IdentityMismatch {
                field,
                theorem: theorem_value,
                application: application_value,
            });
        }

        let assurance = theorem.assurance().min(application.assurance);
        let mut artifact = Self {
            theorem,
            application,
            assurance,
            end_to_end_digest: 0,
        };
        artifact.end_to_end_digest = artifact.digest();
        Ok(artifact)
    }

    pub fn digest(&self) -> u64 {
        let parts = [
            format!("theorem={}", self.theorem.theorem_digest),
            format!("application={}", self.application.application_digest),
            format!("assurance={:?}", self.assurance),
        ];
        fnv1a64(parts.join("|").as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndToEndMismatch {
    TheoremObligationMismatch {
        theorem: u64,
        application: u64,
    },
    TheoremDigestMismatch {
        theorem: u64,
        application: u64,
    },
    TheoremDigestInconsistent,
    ApplicationDigestInconsistent,
    ApplicationFrameUnsupported,
    IdentityMismatch {
        field: &'static str,
        theorem: u64,
        application: u64,
    },
}
