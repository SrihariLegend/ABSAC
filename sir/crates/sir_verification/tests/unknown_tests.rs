//! Unknown Verification Tests (Tier 6).
//!
//! The verifier returns Unknown when it cannot determine equivalence.
//! Tests domain overflow, missing domain, and unsupported cases.

use std::collections::HashSet;

use sir_generation::candidate::CandidateId;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::RegionId;
use sir_verification::errors::UnknownReason;
use sir_verification::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use sir_verification::semantic::expression::{Predicate, SemanticExpression};
use sir_verification::semantic::theorem::Theorem;
use sir_verification::{VerificationPolicy, VerificationResult, Verifier};

/// Build a minimal TransformationContext with all required assumptions.
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

/// Build a ProofObligation with empty assumptions.
fn make_obligation(
    lhs: SemanticExpression,
    rhs: SemanticExpression,
    domain: Option<FiniteDomain>,
) -> ProofObligation {
    ProofObligation {
        id: ObligationId::new(0),
        region: RegionId::new(0),
        candidate: CandidateId::new(0),
        definition: DefinitionId::new(0),
        theorem: Theorem::new(lhs, rhs),
        assumptions: vec![],
        domain,
    }
}

/// Build a valid BS001 theorem expression pair.
fn bs001_lhs(v: VariableId) -> SemanticExpression {
    SemanticExpression::Count(Box::new(SemanticExpression::Filter {
        input: Box::new(SemanticExpression::LogicalSequence { variable: v }),
        predicate: Predicate::True,
    }))
}

fn bs001_rhs(v: VariableId) -> SemanticExpression {
    SemanticExpression::Popcount(Box::new(SemanticExpression::Pack(Box::new(
        SemanticExpression::LogicalSequence { variable: v },
    ))))
}

// ────────────────────────────────────────────────────────────
// Domain overflow tests
// ────────────────────────────────────────────────────────────

#[test]
fn exhaustive_returns_unknown_for_overflowed_domain() {
    // bool[64] domain: total_states() returns None because 2^64
    // overflows u64. The exhaustive backend should return DomainOverflow.
    let v = VariableId::new(0);
    let domain = FiniteDomain {
        variables: vec![VariableSpec {
            id: v,
            kind: VariableKind::LogicalSequence { length: 64 },
        }],
    };

    let obligation = make_obligation(bs001_lhs(v), bs001_rhs(v), Some(domain));
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(UnknownReason::DomainOverflow) => {
            // Expected — 2^64 overflows u64
        }
        other => panic!(
            "Expected Unknown(DomainOverflow) for bool[64], got {:?}",
            other
        ),
    }
}

#[test]
fn exhaustive_returns_unknown_for_overflowed_bool65() {
    // bool[65]: VariableSpec::state_count() returns None for len >= 64.
    // The domain's total_states() propagates this as None → DomainOverflow.
    let v = VariableId::new(0);
    let domain = FiniteDomain {
        variables: vec![VariableSpec {
            id: v,
            kind: VariableKind::LogicalSequence { length: 65 },
        }],
    };

    let obligation = make_obligation(bs001_lhs(v), bs001_rhs(v), Some(domain));
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(UnknownReason::DomainOverflow) => {}
        other => panic!(
            "Expected Unknown(DomainOverflow) for bool[65], got {:?}",
            other
        ),
    }
}

// ────────────────────────────────────────────────────────────
// No domain tests
// ────────────────────────────────────────────────────────────

#[test]
fn exhaustive_returns_unknown_for_no_domain() {
    // Obligation with domain: None. The exhaustive backend requires
    // a domain for enumeration and returns Unknown(NoApplicableBackend).
    let v = VariableId::new(0);
    let obligation = make_obligation(bs001_lhs(v), bs001_rhs(v), None);
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(_) => {
            // Expected — no domain means exhaustive can't enumerate
        }
        other => panic!(
            "Expected Unknown for obligation with domain=None, got {:?}",
            other
        ),
    }
}

// ────────────────────────────────────────────────────────────
// Domain too large tests
// ────────────────────────────────────────────────────────────

#[test]
fn exhaustive_returns_unknown_for_too_large_domain() {
    // bool[21] domain: 2^21 = 2,097,152 which exceeds the default
    // max_states of 1,048,576 (2^20). Should return DomainTooLarge.
    let v = VariableId::new(0);
    let domain = FiniteDomain {
        variables: vec![VariableSpec {
            id: v,
            kind: VariableKind::LogicalSequence { length: 21 },
        }],
    };

    let obligation = make_obligation(bs001_lhs(v), bs001_rhs(v), Some(domain));
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Unknown(UnknownReason::DomainTooLarge {
            states: Some(_),
            max: _,
        }) => {
            // Expected — 2^21 > 1,048,576 (default max_states)
        }
        other => panic!(
            "Expected Unknown(DomainTooLarge) for bool[21] with default max_states, got {:?}",
            other
        ),
    }
}
