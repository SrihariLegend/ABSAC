//! CheckedApplication and EndToEndVerificationArtifact (advisor:
//! application-assurance dimension).
//!
//! Two-dimensional assurance:
//!   TheoremAssurance    — what the local semantic theorem proved
//!                         (`Proof`; checker-issued `assurance` +
//!                         `obligation_digest`).
//!   ApplicationAssurance — that the theorem was bound to the COMPLETE
//!                         concrete rewrite: same source function
//!                         version, same authorization, same role map,
//!                         every live-out classified, source and
//!                         candidate frames compatible.
//!
//! EndToEndVerificationArtifact is the ONLY artifact that may
//! authorize mutation. Its constructor requires TWO real artifacts — a
//! `Proof` and a `CheckedApplication` — and verifies the application
//! was issued for exactly this theorem obligation. If the identities
//! do not match, construction fails.
//!
//! Identity model (first vertical slice): the engine is the single
//! place holding the theorem, the binding, the authorization, and the
//! candidate. It issues the `CheckedApplication` with fields taken
//! from the SAME objects, and stamps `theorem_obligation_digest` from
//! the very proof being matched. `EndToEndVerificationArtifact::new`
//! then verifies (a) the linkage equals the proof's obligation digest,
//! and (b) the application artifact's own digest is consistent with
//! its fields (it was not mutated after issuance). The proof remains
//! authoritative for semantics; the application artifact is
//! authoritative for the concrete application identity.

use crate::artifact::fnv1a64;
use crate::Proof;
use crate::VerificationStatus;

/// An application-level artifact issued by the application checker
/// (the rewrite engine) AFTER checking the complete application
/// conditions. Not a semantic theorem: it records that the theorem's
/// subject was correctly bound to one concrete region, candidate, and
/// observable interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedApplication {
    /// The authorization that authorized this transformation.
    pub authorization_id: u64,
    /// Fingerprint of the exact source function version checked
    /// (fnv1a64 over the function arena).
    pub source_fingerprint: u64,
    /// Source region the transformation applies to.
    pub source_region: u64,
    /// Identity of the candidate applied.
    pub candidate_id: u64,
    /// Digest of the role map the theorem was bound to.
    pub role_map_digest: u64,
    /// Digest of the complete live-out classification (every slot +
    /// binding state + use-closure evidence).
    pub live_out_digest: u64,
    /// The conservative frame contract held on the source.
    pub source_frame_supported: bool,
    /// The candidate's frame was verified compatible with the source
    /// frame (no new writes/calls/traps/over-reads/nontermination).
    pub candidate_frame_compatible: bool,
    /// Digest of the recorded assumptions.
    pub assumptions_digest: u64,
    /// CHECKER-ISSUED assurance. Only the issuing application checker
    /// sets this; a recipe/candidate must never self-declare it.
    pub assurance: VerificationStatus,
    /// The obligation digest of the EXACT proof this application was
    /// matched against. This is the linkage that lets
    /// `EndToEndVerificationArtifact::new` verify the two artifacts
    /// refer to the same theorem obligation.
    pub theorem_obligation_digest: u64,
    /// Digest binding this artifact to its own application identity.
    pub application_digest: u64,
}

impl CheckedApplication {
    /// Construct and stamp the application digest. `assurance` is the
    /// ISSUED level (caller = the application checker, never the
    /// candidate/recipe).
    pub fn new(
        authorization_id: u64,
        source_fingerprint: u64,
        source_region: u64,
        candidate_id: u64,
        role_map_digest: u64,
        live_out_digest: u64,
        source_frame_supported: bool,
        candidate_frame_compatible: bool,
        assumptions_digest: u64,
        assurance: VerificationStatus,
        theorem_obligation_digest: u64,
    ) -> Self {
        let mut app = Self {
            authorization_id,
            source_fingerprint,
            source_region,
            candidate_id,
            role_map_digest,
            live_out_digest,
            source_frame_supported,
            candidate_frame_compatible,
            assumptions_digest,
            assurance,
            theorem_obligation_digest,
            application_digest: 0,
        };
        app.application_digest = app.digest();
        app
    }

    /// FNV-1a over the application identity fields.
    pub fn digest(&self) -> u64 {
        let mut parts: Vec<String> = vec![
            format!("auth={}", self.authorization_id),
            format!("src={}", self.source_fingerprint),
            format!("region={}", self.source_region),
            format!("cand={}", self.candidate_id),
            format!("roles={}", self.role_map_digest),
            format!("liveouts={}", self.live_out_digest),
            format!("srcframe={}", self.source_frame_supported),
            format!("candframe={}", self.candidate_frame_compatible),
            format!("assumptions={}", self.assumptions_digest),
            format!("theorem_obligation={}", self.theorem_obligation_digest),
        ];
        fnv1a64(parts.join("|").as_bytes())
    }
}

/// The only artifact that may authorize mutation. Constructed from TWO
/// matched artifacts: a checker-issued theorem `Proof` and a
/// checker-issued `CheckedApplication`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndToEndVerificationArtifact {
    pub theorem: Proof,
    pub application: CheckedApplication,
    /// End-to-end assurance = min(theorem assurance, application
    /// assurance).
    pub assurance: VerificationStatus,
    /// Digest binding the pair to the exact end-to-end obligation.
    pub end_to_end_digest: u64,
}

impl EndToEndVerificationArtifact {
    /// Attempt to construct a matched end-to-end artifact.
    ///
    /// Identity checks:
    ///   (1) the application artifact must have been issued for THIS
    ///       proof's obligation (`theorem_obligation_digest` linkage);
    ///   (2) the application artifact's digest must be internally
    ///       consistent (fields were not mutated after issuance);
    ///   (3) the end-to-end obligation digest is computed from BOTH
    ///       artifacts.
    ///
    /// If any identity differs, construction fails. The theorem's
    /// assurance and the application's assurance are combined with
    /// `min` — the end-to-end level is the weaker of the two.
    pub fn new(theorem: Proof, application: CheckedApplication) -> Result<Self, EndToEndMismatch> {
        if application.theorem_obligation_digest != theorem.obligation_digest {
            return Err(EndToEndMismatch::TheoremObligationMismatch {
                theorem: theorem.obligation_digest,
                application: application.theorem_obligation_digest,
            });
        }
        if application.application_digest != application.digest() {
            return Err(EndToEndMismatch::ApplicationDigestInconsistent);
        }

        let assurance = theorem.assurance.min(application.assurance);
        let mut e2e = Self {
            theorem,
            application,
            assurance,
            end_to_end_digest: 0,
        };
        e2e.end_to_end_digest = e2e.digest();
        Ok(e2e)
    }

    /// FNV-1a over both artifact identities.
    pub fn digest(&self) -> u64 {
        let mut parts: Vec<String> = vec![
            format!("obligation={}", self.theorem.obligation_digest),
            format!("application={}", self.application.application_digest),
            format!("assurance={:?}", self.assurance),
        ];
        fnv1a64(parts.join("|").as_bytes())
    }
}

/// Identity mismatch between the theorem artifact and the application
/// artifact — construction of an end-to-end artifact is impossible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EndToEndMismatch {
    /// The application was not issued for this theorem obligation.
    TheoremObligationMismatch { theorem: u64, application: u64 },
    /// The application artifact's digest is inconsistent with its
    /// fields (mutated after issuance).
    ApplicationDigestInconsistent,
}
