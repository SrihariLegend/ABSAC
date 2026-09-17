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

fn candidate(region_nodes: Vec<sir_types::NodeId>, definition: u64) -> Candidate {
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
        definition_id: DefinitionId::new(definition),
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
    let obligation = def.obligation_bound(&candidate(vec![mul], 102), &func);
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
    let mut obligation = def.obligation_bound(&candidate(vec![mul], 102), &func);

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
    let obligation = def.obligation_bound(&candidate(vec![mul], 102), &func);
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
    let obligation = def.obligation_bound(&candidate(vec![], 102), &func);
    assert!(obligation.domain.is_none());
    let result = Verifier::new().verify(&obligation, &context());
    assert!(!matches!(result, VerificationResult::Proven(_)));
}

/// `x % C` / `x / C` in an unsigned W-bit function.
///
/// These helpers exist for the eventual ModuloAnd/DivideShift lift. That
/// lift is blocked on proof cost, not on binding: the `urem`/`udiv`
/// bit-blasting is implemented and tested in `sir_mech`, but a 32-bit
/// equivalence proof takes tens of seconds with the current CDCL
/// encoding, so the definitions stay Stub and the tests below pin the
/// fail-closed behaviour.
fn pow2_op_function(
    width: u32,
    constant: u64,
    signed: bool,
    divide: bool,
) -> (sir_nodes::Function, sir_types::NodeId) {
    let ty = match (width, signed) {
        (8, false) => Type::u8(),
        (16, false) => Type::u16(),
        (32, false) => Type::u32(),
        (8, true) => Type::i8(),
        (16, true) => Type::i16(),
        (32, true) => Type::i32(),
        _ => Type::i64(),
    };
    let mut b = Builder::new("pow2", &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let c = b.constant(
        if signed {
            ConstantData::i64(constant as i64)
        } else {
            ConstantData::u64(constant)
        },
        ty.clone(),
        span(),
    );
    let node = if divide {
        b.div(x, c, span()).unwrap()
    } else {
        b.rem(x, c, span()).unwrap()
    };
    b.return_value(node, span()).unwrap();
    (b.build(), node)
}

#[test]
fn quarantined_modulo_and_cannot_be_proven() {
    use sir_verification::definitions::modulo_and::ModuloAndDefinition;
    let (func, rem) = pow2_op_function(32, 16, false, false);
    let def = ModuloAndDefinition::new(DefinitionId::new(100));
    let obligation = def.obligation_bound(&candidate(vec![rem], 100), &func);
    assert!(
        obligation.domain.is_none(),
        "the Stub definition carries no bound domain"
    );
    assert!(!matches!(
        Verifier::new().verify(&obligation, &context()),
        VerificationResult::Proven(_)
    ));
}

#[test]
fn quarantined_divide_shift_cannot_be_proven() {
    use sir_verification::definitions::divide_shift::DivideShiftDefinition;
    let (func, div) = pow2_op_function(32, 8, false, true);
    let def = DivideShiftDefinition::new(DefinitionId::new(101));
    let obligation = def.obligation_bound(&candidate(vec![div], 101), &func);
    assert!(obligation.domain.is_none());
    assert!(!matches!(
        Verifier::new().verify(&obligation, &context()),
        VerificationResult::Proven(_)
    ));
}

#[test]
fn signed_modulo_and_divide_cannot_be_bound() {
    use sir_verification::definitions::divide_shift::DivideShiftDefinition;
    use sir_verification::definitions::modulo_and::ModuloAndDefinition;
    // Signed remainder/division semantics are not the unsigned bitvector
    // identities: the binding must refuse (no domain), so no proof and
    // no rewrite can follow.
    let (func, rem) = pow2_op_function(32, 16, true, false);
    let def = ModuloAndDefinition::new(DefinitionId::new(100));
    let obligation = def.obligation_bound(&candidate(vec![rem], 100), &func);
    assert!(obligation.domain.is_none());
    assert!(!matches!(
        Verifier::new().verify(&obligation, &context()),
        VerificationResult::Proven(_)
    ));

    let (func, div) = pow2_op_function(32, 8, true, true);
    let def = DivideShiftDefinition::new(DefinitionId::new(101));
    let obligation = def.obligation_bound(&candidate(vec![div], 101), &func);
    assert!(obligation.domain.is_none());
    assert!(!matches!(
        Verifier::new().verify(&obligation, &context()),
        VerificationResult::Proven(_)
    ));
}

#[test]
fn quarantined_modulo_mask_mutation_is_not_proven() {
    use sir_verification::definitions::modulo_and::ModuloAndDefinition;
    let (func, rem) = pow2_op_function(32, 16, false, false);
    let def = ModuloAndDefinition::new(DefinitionId::new(100));
    let mut obligation = def.obligation_bound(&candidate(vec![rem], 100), &func);
    let v = VariableId::new(rem.as_u64());
    // MUTATION: claim x % 16 == x & 14 (should be x & 15).
    obligation.theorem = Theorem::new(
        SemanticExpression::Modulo(
            Box::new(SemanticExpression::Variable(v)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(16))),
        ),
        SemanticExpression::BitwiseAnd(
            Box::new(SemanticExpression::Variable(v)),
            Box::new(SemanticExpression::Constant(ConstantData::u64(14))),
        ),
    );
    // The Stub quarantine fires before any backend: the mutated claim is
    // Unknown(InsufficientAssurance), never Proven.
    let result = Verifier::new().verify(&obligation, &context());
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "a quarantined mutated claim must not be Proven, got {result:?}"
    );
}

/// One mask-algebra pattern, built as SIR, with the region nodes the
/// recognizer would authorize (pattern root + operand).
fn mask_pattern_function(
    kind: &str,
) -> (sir_nodes::Function, Vec<sir_types::NodeId>) {
    let ty = Type::u64();
    let mut b = Builder::new(kind, &[("x", ty.clone())], ty.clone());
    let x = b.parameter_index(0).unwrap();
    let one = b.constant(ConstantData::u64(1), ty.clone(), span());
    let (root, operand) = match kind {
        "clear" => {
            let sub = b.sub(x, one, span()).unwrap();
            let and = b.bit_and(x, sub, span()).unwrap();
            (and, x)
        }
        "isolate" => {
            let neg = b.neg(x, span()).unwrap();
            let and = b.bit_and(x, neg, span()).unwrap();
            (and, x)
        }
        "isolate_clear" => {
            let not = b.bit_not(x, span()).unwrap();
            let add = b.add(x, one, span()).unwrap();
            let and = b.bit_and(not, add, span()).unwrap();
            (and, x)
        }
        "set_clear" => {
            let add = b.add(x, one, span()).unwrap();
            let or = b.bit_or(x, add, span()).unwrap();
            (or, x)
        }
        other => panic!("unknown pattern {other}"),
    };
    b.return_value(root, span()).unwrap();
    let func = b.build();
    (func, vec![root, operand])
}

#[test]
fn bound_mask_algebra_patterns_are_concrete_solver_checked() {
    use sir_verification::definitions::clear_lowest_set_bit::ClearLowestSetBitDefinition;
    use sir_verification::definitions::isolate_lowest_clear_bit::IsolateLowestClearBitDefinition;
    use sir_verification::definitions::isolate_lowest_set_bit::IsolateLowestSetBitDefinition;
    use sir_verification::definitions::set_lowest_clear_bit::SetLowestClearBitDefinition;

    let cases: Vec<(&str, u64, Box<dyn Fn(DefinitionId) -> Box<dyn TransformationDefinition>>)> = vec![
        (
            "clear",
            300,
            Box::new(|id| Box::new(ClearLowestSetBitDefinition::new(id))),
        ),
        (
            "isolate",
            301,
            Box::new(|id| Box::new(IsolateLowestSetBitDefinition::new(id))),
        ),
        (
            "isolate_clear",
            302,
            Box::new(|id| Box::new(IsolateLowestClearBitDefinition::new(id))),
        ),
        (
            "set_clear",
            303,
            Box::new(|id| Box::new(SetLowestClearBitDefinition::new(id))),
        ),
    ];
    for (kind, id, make) in cases {
        let (func, nodes) = mask_pattern_function(kind);
        let def = make(DefinitionId::new(id));
        let obligation = def.obligation_bound(&candidate(nodes, id), &func);
        assert!(
            obligation.domain.is_some(),
            "{kind}: the actual pattern must bind"
        );
        match Verifier::new().verify(&obligation, &context()) {
            VerificationResult::Proven(proof) => {
                assert_eq!(proof.backend, VerificationBackend::ConcreteSolver);
                assert_eq!(proof.assurance, VerificationStatus::ConcreteSolverChecked);
            }
            other => panic!("{kind}: expected a concrete-solver proof, got {other:?}"),
        }
    }
}

#[test]
fn unrelated_region_cannot_bind_a_mask_pattern() {
    use sir_verification::definitions::isolate_lowest_set_bit::IsolateLowestSetBitDefinition;
    // A region containing only the parameter is not the `x & -x`
    // pattern: the definition must leave the obligation unbound.
    let mut b = Builder::new("unrelated", &[("x", Type::u64())], Type::u64());
    let x = b.parameter_index(0).unwrap();
    b.return_value(x, span()).unwrap();
    let func = b.build();
    let def = IsolateLowestSetBitDefinition::new(DefinitionId::new(301));
    let obligation = def.obligation_bound(&candidate(vec![x], 301), &func);
    assert!(obligation.domain.is_none());
    assert!(!matches!(
        Verifier::new().verify(&obligation, &context()),
        VerificationResult::Proven(_)
    ));
}
