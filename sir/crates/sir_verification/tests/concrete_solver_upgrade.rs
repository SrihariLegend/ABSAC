//! Quarantine lift for MultiplyShift (advisor P0 item 3).
//!
//! The definition now binds its theorem to the candidate's ACTUAL
//! multiply node (constant + width from the authorized region) and is
//! discharged by the concrete bit-blasting solver. These tests pin:
//!
//!   1. a correct pair is Proven with ISSUED ConcreteSolverChecked;
//!   2. a mutated claim (wrong shift) is Rejected with a counterexample;
//!   3. an unbound/unsupported pair (non-power-of-two constant, no
//!      authorized region) can never be Proven.

use std::collections::HashSet;

use sir_builder::Builder;
use sir_generation::candidate::{
    AuthorizationRef, Candidate, CandidateExplanation, CandidateId, ImplementationStrategy,
};
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::{ContextId, TransformationContext};
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::{ConstantData, RegionId, Span, Type};
use sir_verification::definitions::multiply_shift::MultiplyShiftDefinition;
use sir_verification::registry::{TransformationDefinition, VerificationStatus};
use sir_verification::semantic::expression::SemanticExpression;
use sir_verification::semantic::theorem::Theorem;
use sir_verification::{VerificationBackend, VerificationResult, Verifier};

fn span() -> Span {
    Span::unknown()
}

/// `x * C` in a W-bit function, returning the function and the multiply
/// node id.
fn multiply_function(width: u32, constant: u64) -> (sir_nodes::Function, sir_types::NodeId) {
    let ty = match width {
        8 => Type::u8(),
        16 => Type::u16(),
        32 => Type::u32(),
        _ => Type::u64(),
    };
    let mut b = Builder::new("mul", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(ConstantData::u64(constant), ty.clone(), span());
    let mul = b.mul(x, c, span()).unwrap();
    b.return_value(mul, span()).unwrap();
    (b.build(), mul)
}

fn candidate(region_nodes: Vec<sir_types::NodeId>) -> Candidate {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    constraints.insert(Constraint::ReadOnly);
    constraints.insert(Constraint::FiniteIteration);
    let mut authorization = AuthorizationRef::for_unit_test();
    authorization.region_nodes = region_nodes;
    Candidate {
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        definition_id: DefinitionId::new(102),
        strategy: ImplementationStrategy::ShiftLeft,
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "",
        },
        effects: vec![],
        expected_cost: sir_types::CostProfile {
            instruction_count: 0,
            select_count: 0,
            memory_accesses: 0,
            critical_path_depth: 0,
        },
        representation: Representation::BitwiseArithmetic,
        source_structure: SourceStructure::LogicalSequence { length: 64 },
        constraints,
        assumptions: HashSet::new(),
        authorization,
        binding_digest: 0,
    }
}

fn context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::PreservesLayout);
    TransformationContext::new(
        RegionId::new(0),
        Representation::BitwiseArithmetic,
        SourceStructure::LogicalSequence { length: 64 },
        constraints,
        assumptions,
    )
}

#[test]
fn bound_multiply_shift_is_concrete_solver_checked() {
    let (func, mul) = multiply_function(32, 8);
    let def = MultiplyShiftDefinition::new(DefinitionId::new(102));
    assert_eq!(
        def.verification_status(),
        VerificationStatus::ConcreteSolverChecked
    );
    let obligation = def.obligation_bound(&candidate(vec![mul]), &func);
    assert!(
        obligation.domain.is_some(),
        "the bound obligation must carry the operand's finite domain"
    );

    let verifier = Verifier::new();
    match verifier.verify(&obligation, &context()) {
        VerificationResult::Proven(proof) => {
            assert_eq!(proof.backend, VerificationBackend::ConcreteSolver);
            assert_eq!(proof.assurance, VerificationStatus::ConcreteSolverChecked);
        }
        other => panic!("expected a concrete-solver proof, got {other:?}"),
    }
}

#[test]
fn mutated_shift_claim_is_rejected_with_a_counterexample() {
    let (func, mul) = multiply_function(32, 8);
    let def = MultiplyShiftDefinition::new(DefinitionId::new(102));
    let mut obligation = def.obligation_bound(&candidate(vec![mul]), &func);

    // MUTATION: claim x * 8 == x << 4 (should be << 3).
    // The binding abstracts the non-constant operand with its node id.
    let v = match &func.get_node(mul).unwrap().kind {
        sir_nodes::NodeKind::Mul { lhs, rhs } => {
            let l = func.get_node(*lhs).unwrap();
            let r = func.get_node(*rhs).unwrap();
            let dynamic = if matches!(l.kind, sir_nodes::NodeKind::Constant(_)) {
                *rhs
            } else {
                *lhs
            };
            let _ = r;
            VariableId::new(dynamic.as_u64())
        }
        _ => unreachable!(),
    };
    obligation.theorem = Theorem::new(
        SemanticExpression::Multiply(
            Box::new(SemanticExpression::Variable(v)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(8))),
        ),
        SemanticExpression::ShiftLeft(
            Box::new(SemanticExpression::Variable(v)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(4))),
        ),
    );

    let verifier = Verifier::new();
    match verifier.verify(&obligation, &context()) {
        VerificationResult::Rejected(sir_verification::errors::RejectReason::CounterExample {
            ..
        }) => {}
        other => panic!("a wrong shift claim must be rejected, got {other:?}"),
    }
}

#[test]
fn non_power_of_two_constant_cannot_be_proven() {
    let (func, mul) = multiply_function(32, 12);
    let def = MultiplyShiftDefinition::new(DefinitionId::new(102));
    let obligation = def.obligation_bound(&candidate(vec![mul]), &func);
    assert!(
        obligation.domain.is_none(),
        "an unbindable pair must not carry a domain"
    );
    let result = Verifier::new().verify(&obligation, &context());
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "an unbound obligation must never be Proven, got {result:?}"
    );
}

#[test]
fn unauthorized_region_cannot_be_bound() {
    let (func, _mul) = multiply_function(32, 8);
    let def = MultiplyShiftDefinition::new(DefinitionId::new(102));
    // Empty authorized region: the multiply exists in the function but
    // was not authorized, so no theorem may be bound to it.
    let obligation = def.obligation_bound(&candidate(vec![]), &func);
    assert!(obligation.domain.is_none());
    let result = Verifier::new().verify(&obligation, &context());
    assert!(!matches!(result, VerificationResult::Proven(_)));
}
