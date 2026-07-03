//! Negative Verification Tests (Tier 5).
//!
//! Deliberately broken theorems that the verifier must reject.
//! Tests both the symbolic and exhaustive backends.

use std::collections::HashSet;

use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::ids::{DefinitionId, ObligationId, VariableId};
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_verification::errors::RejectReason;
use sir_verification::obligation::{FiniteDomain, ProofObligation, VariableKind, VariableSpec};
use sir_verification::semantic::expression::{Predicate, SemanticExpression};
use sir_verification::semantic::theorem::Theorem;
use sir_verification::{VerificationPolicy, VerificationResult, Verifier};
use sir_generation::candidate::CandidateId;
use sir_types::{ConstantData, RegionId};

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
        SourceStructure::BooleanArray { length: 64 },
        constraints,
        assumptions,
    )
}

/// Build a minimal domain for a 4-element boolean array.
fn make_domain_4() -> FiniteDomain {
    FiniteDomain {
        variables: vec![VariableSpec {
            id: VariableId::new(0),
            kind: VariableKind::BooleanArray { length: 4 },
        }],
    }
}

/// Build a ProofObligation with empty assumptions (so assumption
/// validation does not interfere with backend-specific testing).
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
        assumptions: vec![], // empty — no assumption validation interference
        domain,
    }
}

// ────────────────────────────────────────────────────────────
// Symbolic rejection tests
// ────────────────────────────────────────────────────────────

#[test]
fn symbolic_rejects_broken_theorem() {
    // LHS = Count(BooleanArray(v)) — missing the Filter wrapper.
    // The CountFilterToPopcount rule won't match, so LHS stays
    // as-is while RHS is Popcount(Pack(BooleanArray(v))).
    //
    // Since the two normalized forms differ, symbolic rejects.
    let v = VariableId::new(0);
    let lhs = SemanticExpression::Count(Box::new(
        SemanticExpression::BooleanArray { variable: v },
    ));
    let rhs = SemanticExpression::Popcount(Box::new(
        SemanticExpression::Pack(Box::new(
            SemanticExpression::BooleanArray { variable: v },
        )),
    ));

    let obligation = make_obligation(lhs, rhs, None);
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::SymbolicOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Rejected(RejectReason::SemanticMismatch { .. }) => {
            // Expected — the rule only matches Count(Filter(...))
        }
        other => panic!(
            "Expected Rejected(SemanticMismatch) for broken theorem without Filter, got {:?}",
            other
        ),
    }
}

#[test]
fn symbolic_rejects_count_without_filter() {
    // Explicit test: Count(BooleanArray(v)) on LHS without Filter.
    // The symbolic normalizer has no rule for this case.
    let v = VariableId::new(0);
    let lhs = SemanticExpression::Count(Box::new(
        SemanticExpression::BooleanArray { variable: v },
    ));
    let rhs = SemanticExpression::Constant(ConstantData::u64(0));

    let obligation = make_obligation(lhs, rhs, None);
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::SymbolicOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Rejected(RejectReason::SemanticMismatch { .. }) => {}
        other => panic!(
            "Expected Rejected(SemanticMismatch) for Count without Filter, got {:?}",
            other
        ),
    }
}

// ────────────────────────────────────────────────────────────
// Exhaustive rejection tests
// ────────────────────────────────────────────────────────────

#[test]
fn exhaustive_rejects_broken_theorem() {
    // RHS = Constant(0) on bool[4].
    // The correct LHS is Count(Filter(BooleanArray(v), True)).
    // For any non-empty input, LHS > 0 while RHS = 0, so
    // exhaustive enumeration finds a counterexample.
    let v = VariableId::new(0);
    let lhs = SemanticExpression::Count(Box::new(
        SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray { variable: v }),
            predicate: Predicate::True,
        },
    ));
    let rhs = SemanticExpression::Constant(ConstantData::u64(0));

    let obligation = make_obligation(lhs, rhs, Some(make_domain_4()));
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Rejected(RejectReason::CounterExample {
            environment,
            lhs: lhs_val,
            rhs: rhs_val,
        }) => {
            // The counterexample should have a non-empty environment
            // and differing values
            assert!(!environment.is_empty(), "CounterExample should have bindings");
            assert_ne!(lhs_val, rhs_val, "CounterExample values must differ");
        }
        other => panic!(
            "Expected Rejected(CounterExample) for broken theorem with Constant(0), got {:?}",
            other
        ),
    }
}

#[test]
fn exhaustive_rejects_constant_zero_on_nonempty_input() {
    // Same as above but explicitly verify the first non-zero input
    // triggers the rejection. Use a small domain for speed.
    let v = VariableId::new(0);
    let lhs = SemanticExpression::Count(Box::new(
        SemanticExpression::Filter {
            input: Box::new(SemanticExpression::BooleanArray { variable: v }),
            predicate: Predicate::True,
        },
    ));
    let rhs = SemanticExpression::Constant(ConstantData::u64(0));

    let domain = FiniteDomain {
        variables: vec![VariableSpec {
            id: VariableId::new(0),
            kind: VariableKind::BooleanArray { length: 2 },
        }],
    };

    let obligation = make_obligation(lhs, rhs, Some(domain));
    let context = make_context();
    let verifier = Verifier::with_policy(VerificationPolicy::ExhaustiveOnly);

    let result = verifier.verify(&obligation, &context);
    match result {
        VerificationResult::Rejected(RejectReason::CounterExample { .. }) => {}
        other => panic!(
            "Expected Rejected(CounterExample) for bool[2] with Constant(0), got {:?}",
            other
        ),
    }
}
