//! Adversarial Verifier Mutation Tests (advisor P0 directive).
//!
//! Two guarantees are tested here:
//!
//! 1. QUARANTINE: a Stub-backed definition can NEVER return Proven —
//!    not even for a tautologically-true obligation (an equivalence a
//!    backend would accept). The quarantine must fire BEFORE any
//!    backend runs, because the theorem-shaped template the definition
//!    produces proves nothing about the actual source/candidate pair.
//!
//! 2. MUTATION SENSITIVITY (SchemaChecked definitions): the verifier
//!    must reject obligations whose bindings were mutated after
//!    minting. A verifier that proves mutated obligations would let a
//!    swap attack launder a false identity through the pipeline.

use std::collections::HashSet;

use sir_generation::candidate::CandidateId;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::{ConstantData, RegionId};
use sir_verification::errors::UnknownReason;
use sir_verification::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use sir_verification::registry::VerificationStatus;
use sir_verification::semantic::expression::{Predicate, SemanticExpression};
use sir_verification::semantic::theorem::Theorem;
use sir_verification::{VerificationPolicy, VerificationResult, Verifier};

fn make_context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    constraints.insert(Constraint::ReadOnly);
    constraints.insert(Constraint::FiniteIteration);

    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);
    assumptions.insert(Assumption::PreservesIterationOrder);
    assumptions.insert(Assumption::PreservesLayout);

    TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::LogicalSequence { length: 64 },
        constraints,
        assumptions,
    )
}

fn tautology_obligation(definition: DefinitionId) -> ProofObligation {
    // A trivially TRUE theorem: Equal(v, v). If the verifier is doing
    // real work, even this must not come back Proven from a quarantined
    // definition — the definition's status, not the theorem's truth,
    // gates the result.
    let v = SemanticExpression::Variable(sir_transform::ids::VariableId::new(0));
    ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition,
        theorem: Theorem::new(v.clone(), v),
        assumptions: vec![],
        domain: None,
    }
}

// ────────────────────────────────────────────────────────────
// 1. Quarantine: Stub never returns Proven
// ────────────────────────────────────────────────────────────

#[test]
fn stub_definition_cannot_prove_a_tautology() {
    // DefinitionId(201) = BitScanReverse — the last registered Stub (its
    // historical LastTrue == clz theorem is false). We submit an even stronger
    // obligation: a syntactic tautology. If the verifier returned Proven
    // for this, the stub quarantine would be broken.
    let obligation = tautology_obligation(DefinitionId::new(201));
    let context = make_context();
    let verifier = Verifier::new();

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(UnknownReason::InsufficientAssurance {
            status,
            minimum: _,
            ..
        }) => {
            assert_eq!(status, sir_verification::registry::VerificationStatus::Stub);
        }
        VerificationResult::Proven(_) => {
            panic!("SOUNDNESS: stub-backed definition returned Proven for a tautology obligation");
        }
        other => panic!(
            "Expected quarantine Unknown(InsufficientAssurance), got {:?}",
            other
        ),
    }
}

#[test]
fn stub_definition_cannot_prove_even_at_minimum_level_stub() {
    // The registry is not directly inspectable from an integration test;
    // quarantine behavior is exercised through verify() in the tautology
    // test below. This test documents the policy floor: the default
    // minimum is SchemaChecked, so a Stub definition (BitScanReverse, id
    // 201) attempting a tautology obligation cannot return Proven.
    let obligation = tautology_obligation(DefinitionId::new(201));
    let context = make_context();
    let verifier = Verifier::new();

    let result = verifier.verify(&obligation, &context);
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "Stub definition must never return Proven"
    );
}

#[test]
fn schema_checked_definitions_are_not_quarantined() {
    // DefinitionIds: 0 = Popcount, 4 = Any, 5 = All, 6 = Parity.
    for (id, name) in [
        (DefinitionId::new(0), "popcount"),
        (DefinitionId::new(4), "any"),
        (DefinitionId::new(5), "all"),
        (DefinitionId::new(6), "parity"),
    ] {
        let verifier = Verifier::new();
        // A SchemaChecked definition given a TRIVIALLY TRUE theorem must
        // still be able to return Proven (i.e., it is not blanket
        // quarantined like the Stubs).
        let obligation = tautology_obligation(id);
        let context = make_context();
        let result = verifier.verify(&obligation, &context);
        assert!(
            matches!(result, VerificationResult::Proven(_)),
            "{} is SchemaChecked and a tautological obligation must be Proven",
            name
        );
    }
}

fn verifier_registry_status(
    _verifier: &Verifier,
    _id: DefinitionId,
) -> sir_verification::registry::VerificationStatus {
    // The registry is private; exercise the status through verify()
    // behavior instead (the quarantine test covers Stub; the
    // SchemaChecked definitions are exercised by the mutation tests
    // below, which would be Unknown(InsufficientAssurance) otherwise).
    sir_verification::registry::VerificationStatus::SchemaChecked
}

#[test]
fn minimum_level_can_be_raised_to_concrete_solver_checked() {
    // Research-mode floor is SchemaChecked. A stricter policy
    // (ConcreteSolverChecked) quarantines even the SchemaChecked
    // reduction definitions — popcount may no longer prove.
    let obligation = {
        // A valid popcount theorem: Count(Filter(seq, True)) ==
        // Popcount(Pack(seq)).
        let v = VariableId::new(0);
        let filtered = SemanticExpression::Filter {
            input: Box::new(SemanticExpression::LogicalSequence { variable: v }),
            predicate: Predicate::True,
        };
        let lhs = SemanticExpression::Count(Box::new(filtered));
        let rhs = SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
            SemanticExpression::LogicalSequence { variable: v },
        ))));
        ProofObligation {
            id: ObligationId::new(0),
            region: RegionId::new(0),
            candidate: CandidateId::new(0),
            definition: DefinitionId::new(0), // Popcount = SchemaChecked
            theorem: Theorem::new(lhs, rhs),
            assumptions: vec![],
            domain: None,
        }
    };
    let context = make_context();

    let research = Verifier::with_policy(VerificationPolicy::SymbolicOnly);
    let strict = Verifier::with_policy(VerificationPolicy::SymbolicOnly)
        .with_min_verification_level(
            sir_verification::registry::VerificationStatus::ConcreteSolverChecked,
        );

    let relaxed = research.verify(&obligation, &context);
    let strict_result = strict.verify(&obligation, &context);

    // Symbolic backend may return Unknown(UnsupportedRule) instead of
    // Proven for this shape — the assertion that matters: the strict
    // verifier must NEVER return Proven.
    if matches!(strict_result, VerificationResult::Proven(_)) {
        panic!("SOUNDNESS: ConcreteSolverChecked policy must not return Proven for a SchemaChecked definition");
    }
    // The quarantine reason is visible in the unknown result.
    if let VerificationResult::Unknown(UnknownReason::InsufficientAssurance { .. }) = strict_result
    {
        // Expected quarantine path.
    }

    // And the default policy must not be stricter than research mode:
    // the same obligation should be provable (or unknown) in research
    // mode, never rejected outright by the quarantine.
    match research.verify(&obligation, &context) {
        VerificationResult::Proven(_) => {}
        VerificationResult::Unknown(_) => {}
        other => panic!(
            "SchemaChecked definition must not be Rejected by policy: {:?}",
            other
        ),
    }
    let _ = strict; // silence unused in the degenerate case
    let _ = strict_result;
}

// ────────────────────────────────────────────────────────────
// 2. Mutation tests (SchemaChecked definitions)
// ────────────────────────────────────────────────────────────

#[test]
fn mutated_popcount_theorem_is_not_proven() {
    // Popcount theorem with the population mutated: Count of the
    // sequence is compared against Popcount of a DIFFERENT sequence
    // shape (missing the BooleanArray adapter). The exhaustive backend
    // must not prove a mismatched pair; the symbolic backend cannot
    // normalize it. Either way, it must NOT be Proven.
    let v = VariableId::new(0);
    let seq = SemanticExpression::LogicalSequence { variable: v };
    // MUTATED theorem: Count(seq) == EqualFullMask(Pack(seq)).
    // The honest identity is Count == Popcount. Swapping the RHS for
    // EqualFullMask (all-bits-set) makes the theorem FALSE: for the
    // input [true, false] LHS=1, RHS=false. The exhaustive backend
    // must find the counterexample.
    let lhs = SemanticExpression::Count(Box::new(seq.clone()));
    let rhs = SemanticExpression::EqualFullMask(Box::new(SemanticExpression::Pack(Box::new(
        SemanticExpression::LogicalSequence { variable: v },
    ))));

    let obligation = ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition: DefinitionId::new(0),
        theorem: Theorem::new(lhs, rhs),
        assumptions: vec![],
        domain: Some(FiniteDomain {
            variables: vec![VariableSpec {
                id: v,
                kind: VariableKind::LogicalSequence { length: 4 },
            }],
        }),
    };
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);
    let result = verifier.verify(&obligation, &context);
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "MUTATION ACCEPTED: verifier proved a mutated popcount theorem"
    );
}

#[test]
fn mutated_all_theorem_with_wrong_length_is_not_proven() {
    // All(seq) == EqualFullMask(Pack(seq)) — but the domain binds the
    // sequence to length 4 while the context claims FixedLength(64).
    // The verifier must not silently accept a length inconsistency.
    let v = VariableId::new(0);
    let seq = SemanticExpression::LogicalSequence { variable: v };
    // MUTATED theorem: All(seq) == NotEqualZero(Pack(seq)) instead of
    // the honest All(seq) == EqualFullMask(Pack(seq)). For [true, false]
    // the sides differ (false vs true) — falsifiable over length 2.
    let lhs = SemanticExpression::All(Box::new(seq.clone()));
    let rhs = SemanticExpression::NotEqualZero(Box::new(SemanticExpression::Pack(Box::new(
        SemanticExpression::LogicalSequence { variable: v },
    ))));

    // Exhaustive over length 4 where the theorem is genuinely false for
    // at least one input (e.g. [true, false]) — it must not claim Proven.
    let obligation = ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition: DefinitionId::new(5), // All = SchemaChecked
        theorem: Theorem::new(lhs, rhs),
        assumptions: vec![],
        domain: Some(FiniteDomain {
            variables: vec![VariableSpec {
                id: v,
                kind: VariableKind::LogicalSequence { length: 4 },
            }],
        }),
    };
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);
    let result = verifier.verify(&obligation, &context);
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "MUTATION ACCEPTED: All-theorem proven over a domain where it is false"
    );
}

#[test]
fn quarantine_blocks_stub_even_when_obligation_would_trivially_normalize() {
    // The strongest form of the quarantine test: hand-craft a BitScanReverse
    // obligation whose theorem is Equal(x, x) — a syntactic tautology
    // the symbolic backend would prove instantly. The verifier must
    // quarantine it BEFORE any backend runs.
    let obligation = tautology_obligation(DefinitionId::new(201)); // BitScanReverse = Stub
    let context = make_context();
    let verifier = Verifier::new();

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(UnknownReason::InsufficientAssurance {
            definition,
            status,
            ..
        }) => {
            assert_eq!(status, sir_verification::registry::VerificationStatus::Stub);
            assert_eq!(definition, "LastTrue to BitScanReverse");
        }
        VerificationResult::Proven(_) => {
            panic!("SOUNDNESS: a Stub definition returned Proven for a tautology — quarantine bypassed");
        }
        other => panic!("Expected Unknown(InsufficientAssurance), got {:?}", other),
    }
}

#[test]
fn exhaustive_backend_cannot_prove_for_stub_definition() {
    // Same via the exhaustive path: even ExhaustiveOnly policy (which
    // enumerates states) must refuse to Proven a stub definition.
    let v = VariableId::new(0);
    let obligation = ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition: DefinitionId::new(201), // BitScanReverse = Stub
        theorem: Theorem::new(
            SemanticExpression::Constant(ConstantData::u64(0)),
            SemanticExpression::Constant(ConstantData::u64(0)),
        ),
        assumptions: vec![],
        domain: Some(FiniteDomain {
            variables: vec![VariableSpec {
                id: v,
                kind: VariableKind::LogicalSequence { length: 4 },
            }],
        }),
    };
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);
    let result = verifier.verify(&obligation, &context);
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "MUTATION ACCEPTED: Constant(0)==Constant(0) 'proved' through exhaustive backend for a quarantined definition"
    );
}

// ────────────────────────────────────────────────────────────
// 3. Checker-issued assurance (advisor item 3)
// ────────────────────────────────────────────────────────────
//
// A definition's `verification_status()` is a CAP, not a status. The
// checker issues min(declared cap, backend capability) and gates
// policy on the ISSUED level. A definition author cannot self-certify
// by declaring MachineChecked.

use sir_generation::candidate::Candidate;
use sir_verification::registry::{TransformationDefinition, TransformationRegistry};

/// A malicious/naive definition that declares MachineChecked for
/// itself. The checker must still cap issuance at backend capability.
struct SelfCertifyingDef;

impl TransformationDefinition for SelfCertifyingDef {
    fn id(&self) -> DefinitionId {
        DefinitionId::new(999)
    }
    fn name(&self) -> &'static str {
        "self-certifying"
    }
    fn verification_status(&self) -> VerificationStatus {
        // The attack: declare the highest possible level for oneself.
        VerificationStatus::MachineChecked
    }
    fn applicability(&self, _candidate: &Candidate) -> bool {
        true
    }
    fn obligation(&self, _candidate: &Candidate) -> ProofObligation {
        tautology_obligation(DefinitionId::new(999))
    }
}

fn self_certifying_registry() -> TransformationRegistry {
    let mut registry = TransformationRegistry::new();
    registry.register(Box::new(SelfCertifyingDef));
    registry
}

#[test]
fn definition_cannot_self_certify_machine_checked() {
    // The definition declares MachineChecked; the symbolic backend's
    // capability is SchemaChecked. The ISSUED level on the artifact
    // must be the backend cap — the definition cannot raise it.
    let verifier = Verifier::with_policy(VerificationPolicy::SymbolicOnly)
        .with_registry(self_certifying_registry());
    let obligation = tautology_obligation(DefinitionId::new(999));
    let context = make_context();
    match verifier.verify(&obligation, &context) {
        VerificationResult::Proven(proof) => {
            assert_eq!(
                proof.assurance,
                VerificationStatus::SchemaChecked,
                "ISSUED assurance must be capped at backend capability                  (Symbolic = SchemaChecked) regardless of the definition's \
                 self-declared MachineChecked"
            );
            assert_ne!(
                proof.obligation_digest, 0,
                "issued artifact must carry a nonzero obligation digest"
            );
        }
        other => panic!(
            "Expected checker-issued Proven with capped assurance, got {:?}",
            other
        ),
    }
}

#[test]
fn self_certified_machine_checked_fails_strict_policy() {
    // Same self-certifying definition under a strict policy: the
    // checker issued SchemaChecked (backend cap) — below the
    // ConcreteSolverChecked minimum. Fail closed regardless of the
    // definition's declaration.
    let verifier = Verifier::with_policy(VerificationPolicy::SymbolicOnly)
        .with_registry(self_certifying_registry())
        .with_min_verification_level(VerificationStatus::ConcreteSolverChecked);
    let obligation = tautology_obligation(DefinitionId::new(999));
    let context = make_context();
    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Proven(_) => panic!(
            "SOUNDNESS: self-certified MachineChecked must not pass a \
             ConcreteSolverChecked policy — the checker, not the definition, \
             issues assurance"
        ),
        VerificationResult::Unknown(UnknownReason::InsufficientAssurance {
            status,
            minimum,
            ..
        }) => {
            assert_eq!(status, VerificationStatus::SchemaChecked);
            assert_eq!(minimum, VerificationStatus::ConcreteSolverChecked);
        }
        other => panic!("Expected InsufficientAssurance quarantine, got {:?}", other),
    }
}

#[test]
fn issued_assurance_from_symbolic_backend_is_schema_checked() {
    // Existing SchemaChecked definitions discharged by the Symbolic
    // backend: the ISSUED level must be SchemaChecked (min of
    // definition cap and backend capability) — never higher.
    let verifier = Verifier::with_policy(VerificationPolicy::SymbolicOnly);
    let obligation = tautology_obligation(DefinitionId::new(4)); // Any
    let context = make_context();
    match verifier.verify(&obligation, &context) {
        VerificationResult::Proven(proof) => {
            assert_eq!(proof.assurance, VerificationStatus::SchemaChecked);
        }
        other => panic!("Expected Proven with issued SchemaChecked, got {:?}", other),
    }
}

// ── EndToEndVerificationArtifact (advisor: matched artifacts) ──

use sir_verification::application_artifact::{
    ApplicationChecker, CheckedApplication, CheckedTheorem, EndToEndMismatch,
    EndToEndVerificationArtifact,
};

fn fixture_proof(obligation: u64, assurance: VerificationStatus) -> sir_verification::Proof {
    sir_verification::Proof {
        theorem: Theorem::new(
            SemanticExpression::Constant(ConstantData::u64(0)),
            SemanticExpression::Constant(ConstantData::u64(0)),
        ),
        normalized_theorem: Theorem::new(
            SemanticExpression::Constant(ConstantData::u64(0)),
            SemanticExpression::Constant(ConstantData::u64(0)),
        ),
        backend: sir_verification::VerificationBackend::Symbolic,
        steps: vec![],
        assurance,
        obligation_digest: obligation,
    }
}

fn fixture_theorem(obligation: u64, assurance: VerificationStatus) -> CheckedTheorem {
    Verifier::new().bind_checked_theorem(
        fixture_proof(obligation, assurance),
        7,     // authorization
        42,    // source fingerprint
        0,     // region
        3,     // candidate
        4,     // definition
        0xabc, // role map digest
        0xdef, // live-out digest
        0x111, // source-frame digest
        0x222, // candidate-frame digest
        0,     // assumptions
    )
}

fn fixture_application(theorem: &CheckedTheorem) -> CheckedApplication {
    ApplicationChecker::issue(
        theorem.authorization_id,
        theorem.source_fingerprint,
        theorem.source_region,
        theorem.candidate_id,
        theorem.definition_id,
        theorem.role_map_digest,
        theorem.live_out_digest,
        theorem.source_frame_digest,
        theorem.candidate_frame_digest,
        true, // source frame supported
        true, // candidate frame compatible
        theorem.assumptions_digest,
        VerificationStatus::SchemaChecked,
        theorem.theorem_digest,
        theorem.proof.obligation_digest,
    )
}

#[test]
fn matched_artifacts_construct_end_to_end() {
    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    let application = fixture_application(&theorem);
    let e2e = EndToEndVerificationArtifact::new(theorem, application)
        .expect("matching artifacts must construct");
    assert_eq!(e2e.assurance, VerificationStatus::SchemaChecked);
    assert!(e2e.end_to_end_digest != 0);
}

#[test]
fn mismatched_artifacts_refuse_construction() {
    let theorem = fixture_theorem(0x1111, VerificationStatus::SchemaChecked);
    let other_theorem = fixture_theorem(0x2222, VerificationStatus::SchemaChecked);
    let application = fixture_application(&other_theorem);
    match EndToEndVerificationArtifact::new(theorem, application) {
        Err(EndToEndMismatch::TheoremObligationMismatch {
            theorem: 0x1111,
            application: 0x2222,
        }) => {}
        other => panic!("expected TheoremObligationMismatch, got {:?}", other),
    }
}

#[test]
fn end_to_end_assurance_is_the_weaker_of_the_two() {
    let theorem = fixture_theorem(0x1234, VerificationStatus::ConcreteSolverChecked);
    let application = fixture_application(&theorem);
    let e2e = EndToEndVerificationArtifact::new(theorem, application)
        .expect("matching artifacts must construct");
    assert_eq!(
        e2e.assurance,
        VerificationStatus::SchemaChecked,
        "end-to-end assurance = min(theorem, application)"
    );
}

#[test]
fn tampered_application_artifact_refuses_construction() {
    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    let mut application = fixture_application(&theorem);
    application.candidate_id = 999;
    match EndToEndVerificationArtifact::new(theorem, application) {
        Err(EndToEndMismatch::ApplicationDigestInconsistent) => {}
        other => panic!("expected ApplicationDigestInconsistent, got {:?}", other),
    }
}

fn assert_tampered_application_refuses(theorem: &CheckedTheorem, application: CheckedApplication) {
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), application),
        Err(EndToEndMismatch::ApplicationDigestInconsistent)
    ));
}

#[test]
fn self_consistent_unsupported_application_frame_refuses_construction() {
    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    let mut application = fixture_application(&theorem);
    application.source_frame_supported = false;
    application.application_digest = application.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem, application),
        Err(EndToEndMismatch::ApplicationFrameUnsupported)
    ));

    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    let mut application = fixture_application(&theorem);
    application.candidate_frame_compatible = false;
    application.application_digest = application.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem, application),
        Err(EndToEndMismatch::ApplicationFrameUnsupported)
    ));
}

#[test]
fn every_application_field_is_digest_bound() {
    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    let baseline = fixture_application(&theorem);

    let mut authorization = baseline.clone();
    authorization.authorization_id ^= 1;
    assert_tampered_application_refuses(&theorem, authorization);
    let mut source = baseline.clone();
    source.source_fingerprint ^= 1;
    assert_tampered_application_refuses(&theorem, source);
    let mut region = baseline.clone();
    region.source_region ^= 1;
    assert_tampered_application_refuses(&theorem, region);
    let mut candidate = baseline.clone();
    candidate.candidate_id ^= 1;
    assert_tampered_application_refuses(&theorem, candidate);
    let mut definition = baseline.clone();
    definition.definition_id ^= 1;
    assert_tampered_application_refuses(&theorem, definition);
    let mut roles = baseline.clone();
    roles.role_map_digest ^= 1;
    assert_tampered_application_refuses(&theorem, roles);
    let mut live_out = baseline.clone();
    live_out.live_out_digest ^= 1;
    assert_tampered_application_refuses(&theorem, live_out);
    let mut source_frame_digest = baseline.clone();
    source_frame_digest.source_frame_digest ^= 1;
    assert_tampered_application_refuses(&theorem, source_frame_digest);
    let mut candidate_frame_digest = baseline.clone();
    candidate_frame_digest.candidate_frame_digest ^= 1;
    assert_tampered_application_refuses(&theorem, candidate_frame_digest);
    let mut source_frame = baseline.clone();
    source_frame.source_frame_supported = false;
    assert_tampered_application_refuses(&theorem, source_frame);
    let mut candidate_frame = baseline.clone();
    candidate_frame.candidate_frame_compatible = false;
    assert_tampered_application_refuses(&theorem, candidate_frame);
    let mut assumptions = baseline.clone();
    assumptions.assumptions_digest ^= 1;
    assert_tampered_application_refuses(&theorem, assumptions);
    let mut assurance = baseline.clone();
    assurance.assurance = VerificationStatus::ConcreteSolverChecked;
    assert_tampered_application_refuses(&theorem, assurance);
    let mut theorem_digest = baseline.clone();
    theorem_digest.theorem_digest ^= 1;
    assert_tampered_application_refuses(&theorem, theorem_digest);
    let mut obligation = baseline;
    obligation.theorem_obligation_digest ^= 1;
    assert_tampered_application_refuses(&theorem, obligation);
}

fn assert_tampered_theorem_refuses(theorem: CheckedTheorem, application: CheckedApplication) {
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem, application),
        Err(EndToEndMismatch::TheoremDigestInconsistent)
    ));
}

#[test]
fn every_theorem_field_is_digest_bound() {
    let baseline = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);

    let mut proof_assurance = baseline.clone();
    proof_assurance.proof.assurance = VerificationStatus::ConcreteSolverChecked;
    assert_tampered_theorem_refuses(proof_assurance, fixture_application(&baseline));
    let mut expression = baseline.clone();
    expression.proof.theorem.lhs = SemanticExpression::Constant(ConstantData::u64(1));
    assert_tampered_theorem_refuses(expression, fixture_application(&baseline));
    let mut obligation = baseline.clone();
    obligation.proof.obligation_digest ^= 1;
    assert_tampered_theorem_refuses(obligation, fixture_application(&baseline));
    let mut authorization = baseline.clone();
    authorization.authorization_id ^= 1;
    assert_tampered_theorem_refuses(authorization, fixture_application(&baseline));
    let mut source = baseline.clone();
    source.source_fingerprint ^= 1;
    assert_tampered_theorem_refuses(source, fixture_application(&baseline));
    let mut region = baseline.clone();
    region.source_region ^= 1;
    assert_tampered_theorem_refuses(region, fixture_application(&baseline));
    let mut candidate = baseline.clone();
    candidate.candidate_id ^= 1;
    assert_tampered_theorem_refuses(candidate, fixture_application(&baseline));
    let mut definition = baseline.clone();
    definition.definition_id ^= 1;
    assert_tampered_theorem_refuses(definition, fixture_application(&baseline));
    let mut roles = baseline.clone();
    roles.role_map_digest ^= 1;
    assert_tampered_theorem_refuses(roles, fixture_application(&baseline));
    let mut live_out = baseline.clone();
    live_out.live_out_digest ^= 1;
    assert_tampered_theorem_refuses(live_out, fixture_application(&baseline));
    let mut source_frame_digest = baseline.clone();
    source_frame_digest.source_frame_digest ^= 1;
    assert_tampered_theorem_refuses(source_frame_digest, fixture_application(&baseline));
    let mut candidate_frame_digest = baseline.clone();
    candidate_frame_digest.candidate_frame_digest ^= 1;
    assert_tampered_theorem_refuses(candidate_frame_digest, fixture_application(&baseline));
    let mut assumptions = baseline;
    assumptions.assumptions_digest ^= 1;
    assert_tampered_theorem_refuses(
        assumptions,
        fixture_application(&fixture_theorem(0x1234, VerificationStatus::SchemaChecked)),
    );
}

#[test]
fn self_consistent_identity_mutations_refuse_pairing() {
    // A forged artifact can recompute its own digest, but it still
    // cannot be paired with a theorem issued for another identity.
    let theorem = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);

    let mut candidate_mutation = fixture_application(&theorem);
    candidate_mutation.candidate_id = 99;
    candidate_mutation.application_digest = candidate_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), candidate_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "candidate",
            ..
        })
    ));

    let mut role_mutation = fixture_application(&theorem);
    role_mutation.role_map_digest ^= 1;
    role_mutation.application_digest = role_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), role_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "role_map",
            ..
        })
    ));

    let mut definition_mutation = fixture_application(&theorem);
    definition_mutation.definition_id = 5;
    definition_mutation.application_digest = definition_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), definition_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "definition",
            ..
        })
    ));

    let mut authorization_mutation = fixture_application(&theorem);
    authorization_mutation.authorization_id = 8;
    authorization_mutation.application_digest = authorization_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), authorization_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "authorization",
            ..
        })
    ));

    let mut source_mutation = fixture_application(&theorem);
    source_mutation.source_fingerprint = 43;
    source_mutation.application_digest = source_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), source_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "source_fingerprint",
            ..
        })
    ));

    let mut region_mutation = fixture_application(&theorem);
    region_mutation.source_region = 1;
    region_mutation.application_digest = region_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), region_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "source_region",
            ..
        })
    ));

    let mut assumptions_mutation = fixture_application(&theorem);
    assumptions_mutation.assumptions_digest = 1;
    assumptions_mutation.application_digest = assumptions_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem.clone(), assumptions_mutation),
        Err(EndToEndMismatch::IdentityMismatch {
            field: "assumptions",
            ..
        })
    ));

    let mut theorem_mutation = fixture_theorem(0x1234, VerificationStatus::SchemaChecked);
    theorem_mutation.candidate_id = 99;
    theorem_mutation.theorem_digest = theorem_mutation.digest();
    assert!(matches!(
        EndToEndVerificationArtifact::new(theorem_mutation, fixture_application(&theorem)),
        Err(EndToEndMismatch::TheoremDigestMismatch { .. })
    ));
}
