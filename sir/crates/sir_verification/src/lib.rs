//! SIR Verification — Equivalence Proof Engine v0.1
//!
//! Proves (or rejects) candidate transformation plans through
//! symbolic normalization and exhaustive enumeration.
//! Never reads or modifies SIR.

pub mod errors;
pub mod obligation;
pub mod registry;
pub mod report;
pub mod validation;

pub mod backends;
pub mod definitions;
pub mod semantic;

use crate::errors::RejectReason;
use crate::errors::UnknownReason;
use crate::semantic::expression::SemanticExpression;
use crate::semantic::theorem::Theorem;

use sir_generation::generator::CandidateDatabase;
use sir_transform::context::{TransformationContext, TransformationContextDatabase};

use crate::backends::exhaustive::ExhaustiveVerifier;
use crate::backends::symbolic::SymbolicVerifier;
use crate::definitions::all::AllDefinition;
use crate::definitions::any::AnyDefinition;
use crate::definitions::bitscan_forward::BitScanForwardDefinition;
use crate::definitions::bitscan_reverse::BitScanReverseDefinition;
use crate::definitions::divide_shift::DivideShiftDefinition;
use crate::definitions::leading_zero_count::LeadingZeroCountDefinition;
use crate::definitions::multiply_shift::MultiplyShiftDefinition;
use crate::definitions::parity::ParityDefinition;
use crate::definitions::popcount::PopcountDefinition;
use crate::definitions::shift_mask::ShiftMaskDefinition;
use crate::definitions::trailing_zero_count::TrailingZeroCountDefinition;
use crate::obligation::{ProofObligation, ProofObligationDatabase};
use crate::registry::TransformationRegistry;
use crate::report::{ReportEntry, ReportStatus, VerificationReport};
use crate::validation::AssumptionValidator;

/// A completed proof of equivalence.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    ExhaustiveCheck { states_checked: u64 },
}

/// Which verification backend discharged a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VerificationBackend {
    Symbolic,
    Exhaustive,
}

/// The result of attempting to verify a proof obligation.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub total: usize,
    pub proven: usize,
    pub rejected: usize,
    pub unknown: usize,
}

/// The main verification engine.
///
/// Owns the transformation registry, verification policy, and
/// resource limits. Produces proof obligations from candidates
/// and discharges them through configured backends.
pub struct Verifier {
    registry: TransformationRegistry,
    policy: VerificationPolicy,
    limits: VerificationLimits,
}

impl Verifier {
    /// Create a verifier with default policy and all built-in definitions registered.
    pub fn new() -> Self {
        let mut registry = TransformationRegistry::new();
        registry.register(Box::new(PopcountDefinition::new(
            sir_transform::ids::DefinitionId::new(0),
        )));
        registry.register(Box::new(AnyDefinition::new(
            sir_transform::ids::DefinitionId::new(4), // Any (BS002)
        )));
        registry.register(Box::new(AllDefinition::new(
            sir_transform::ids::DefinitionId::new(5),
        )));
        registry.register(Box::new(ParityDefinition::new(
            sir_transform::ids::DefinitionId::new(6),
        )));
        registry.register(Box::new(definitions::modulo_and::ModuloAndDefinition::new(
            sir_transform::ids::DefinitionId::new(100),
        )));
        registry.register(Box::new(DivideShiftDefinition::new(
            sir_transform::ids::DefinitionId::new(101),
        )));
        registry.register(Box::new(MultiplyShiftDefinition::new(
            sir_transform::ids::DefinitionId::new(102),
        )));
        registry.register(Box::new(ShiftMaskDefinition::new(
            sir_transform::ids::DefinitionId::new(103),
        )));
        registry.register(Box::new(BitScanForwardDefinition::new(
            sir_transform::ids::DefinitionId::new(200),
        )));
        registry.register(Box::new(BitScanReverseDefinition::new(
            sir_transform::ids::DefinitionId::new(201),
        )));
        registry.register(Box::new(TrailingZeroCountDefinition::new(
            sir_transform::ids::DefinitionId::new(202),
        )));
        registry.register(Box::new(LeadingZeroCountDefinition::new(
            sir_transform::ids::DefinitionId::new(203),
        )));

        Self {
            registry,
            policy: VerificationPolicy::Default,
            limits: VerificationLimits::default(),
        }
    }

    /// Create a verifier with a specific policy.
    pub fn with_policy(policy: VerificationPolicy) -> Self {
        let mut verifier = Self::new();
        verifier.policy = policy;
        verifier
    }

    /// Create a verifier with custom limits.
    pub fn with_limits(limits: VerificationLimits) -> Self {
        let mut verifier = Self::new();
        verifier.limits = limits;
        verifier
    }

    /// Build proof obligations for all candidates in the database.
    ///
    /// For each candidate, looks up its TransformationDefinition,
    /// checks applicability, and constructs a ProofObligation.
    pub fn build_obligations(
        &self,
        candidates: &CandidateDatabase,
        contexts: &TransformationContextDatabase,
    ) -> ProofObligationDatabase {
        let mut db = ProofObligationDatabase::new();

        for candidate in candidates.all_candidates() {
            // Get the context for this candidate's region
            let ctx_list = contexts.for_region(candidate.region);
            if ctx_list.is_empty() {
                continue;
            }

            // Find the first context this definition is applicable to
            for ctx in ctx_list {
                if let Some(def) = self.registry.find_for(candidate) {
                    let mut obligation = def.obligation(candidate);
                    obligation.candidate = candidate.id;
                    obligation.definition = def.id();
                    db.insert(obligation);
                    break;
                }
            }
        }

        db
    }

    /// Verify a single obligation using the configured policy.
    pub fn verify(
        &self,
        obligation: &ProofObligation,
        context: &TransformationContext,
    ) -> VerificationResult {
        // Step 0: Validate assumptions
        if let Err(assumption) = AssumptionValidator::validate(obligation, context) {
            return VerificationResult::Rejected(crate::errors::RejectReason::AssumptionViolated {
                assumption,
            });
        }

        match self.policy {
            VerificationPolicy::SymbolicOnly => SymbolicVerifier::new().verify(obligation),

            VerificationPolicy::ExhaustiveOnly => {
                ExhaustiveVerifier::new(self.limits.clone()).verify(obligation)
            }

            VerificationPolicy::Default => {
                // Try symbolic first
                let symbolic = SymbolicVerifier::new();
                match symbolic.verify(obligation) {
                    VerificationResult::Proven(proof) => {
                        return VerificationResult::Proven(proof);
                    }
                    VerificationResult::Rejected(reason) => {
                        return VerificationResult::Rejected(reason);
                    }
                    VerificationResult::Unknown(_) => {
                        // Fall through to exhaustive
                    }
                }

                // Fall back to exhaustive
                ExhaustiveVerifier::new(self.limits.clone()).verify(obligation)
            }
        }
    }

    /// Generate a human-readable verification report.
    pub fn report(&self, results: &[(ProofObligation, VerificationResult)]) -> VerificationReport {
        let mut report = VerificationReport::new();

        for (obligation, result) in results {
            let (status, details) = match result {
                VerificationResult::Proven(proof) => {
                    let assumptions_str: String = obligation
                        .assumptions
                        .iter()
                        .map(|a| format!("  \u{2713} {:?}", a))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let steps_str: String = proof
                        .steps
                        .iter()
                        .enumerate()
                        .map(|(i, s)| format!("  {}. {:?}", i + 1, s))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let detail = format!(
                        "Theorem:\n  {:?}\n  \u{2261}\n  {:?}\n\n\
                         Normalized theorem:\n  {:?}\n  \u{2261}\n  {:?}\n\n\
                         Assumptions:\n{}\n\n\
                         Proof steps:\n{}",
                        obligation.theorem.lhs,
                        obligation.theorem.rhs,
                        proof.normalized_theorem.lhs,
                        proof.normalized_theorem.rhs,
                        assumptions_str,
                        steps_str,
                    );

                    (ReportStatus::Proven, Some(detail))
                }
                VerificationResult::Rejected(reason) => (
                    ReportStatus::Rejected,
                    Some(format!("Reason: {:?}", reason)),
                ),
                VerificationResult::Unknown(reason) => {
                    (ReportStatus::Unknown, Some(format!("Reason: {:?}", reason)))
                }
            };

            // Determine backend name from the result
            let backend = match result {
                VerificationResult::Proven(p) => format!("{}", p.backend),
                _ => "N/A".to_string(),
            };

            // Look up definition name
            let def_name = self
                .registry
                .lookup(obligation.definition)
                .map(|d| d.name().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            report.add(ReportEntry {
                transformation_name: def_name,
                backend,
                status,
                details,
            });
        }

        report
    }

    /// Return verification statistics.
    pub fn statistics(&self, results: &[VerificationResult]) -> Statistics {
        let mut stats = Statistics::default();
        stats.total = results.len();

        for result in results {
            match result {
                VerificationResult::Proven(_) => stats.proven += 1,
                VerificationResult::Rejected(_) => stats.rejected += 1,
                VerificationResult::Unknown(_) => stats.unknown += 1,
            }
        }

        stats
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}
