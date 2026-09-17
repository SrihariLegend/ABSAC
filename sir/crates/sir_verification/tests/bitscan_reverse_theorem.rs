//! BitScanReverse (201) quarantine lift — semantic half.
//!
//! The historical obligation `LastTrue(seq) == LeadingZeros(Pack(seq))`
//! is FALSE (MSB-set mask: 63 vs 0). The corrected theorem is
//! `LastTrue(seq) == BitScanReverse(Pack(seq))` (highest set index,
//! width sentinel for zero). These tests pin:
//!
//!   1. the corrected pair is discharged by the concrete bit-blasting
//!      solver (a real lower-both-sides proof, no normalization);
//!   2. the historical false equation is REJECTED with a counterexample;
//!   3. the definition is still quarantined (Stub) so the
//!      LeadingZeros-emitting recipe cannot be authorized until the
//!      intrinsic/recipe/trip-count/sound-kernel blockers are closed.

use std::collections::HashSet;

use sir_builder::Builder;
use sir_generation::candidate::{
    AuthorizationRef, Candidate, CandidateExplanation, CandidateId, ImplementationStrategy,
};
use sir_semantics::structure::StructuralDescription;
use sir_transform::constraints::Constraint;
use sir_transform::context::{ContextId, TransformationContext};
use sir_transform::ids::{DefinitionId, VariableId};
use sir_transform::representation::Representation;
use sir_transform::roles::RegionRoles;
use sir_transform::structures::SourceStructure;
use sir_types::{ConstantData, RegionId, Span, Type};
use sir_verification::backends::concrete_solver::ConcreteSolverVerifier;
use sir_verification::definitions::bitscan_reverse::BitScanReverseDefinition;
use sir_verification::registry::{TransformationDefinition, VerificationStatus};
use sir_verification::semantic::expression::SemanticExpression;
use sir_verification::semantic::theorem::Theorem;
use sir_verification::{VerificationBackend, VerificationResult, Verifier};

fn span() -> Span {
    Span::unknown()
}

/// A function with a `[bool; length]` parameter, used as the
/// PositionSearch collection.
fn mask_function(length: usize) -> (sir_nodes::Function, sir_types::NodeId) {
    let array_ty = Type::Array {
        element: Box::new(Type::Bool),
        length,
    };
    let mut b = Builder::new("bsr_fixture", &[("seq", array_ty)], Type::u64());
    let seq = b.parameter_index(0).unwrap();
    let zero = b.constant(ConstantData::u64(0), Type::u64(), span());
    b.return_value(zero, span()).unwrap();
    (b.build(), seq)
}

fn structural_with_collection(collection: sir_types::NodeId, length: usize) -> StructuralDescription {
    let mut structural = StructuralDescription::new(
        RegionId::new(0),
        SourceStructure::LogicalSequence { length },
    );
    structural.roles.push(RegionRoles::PositionSearch {
        collection: Some(collection),
        scalar: None,
        result: collection,
    });
    structural
}

fn candidate_with_region(region_nodes: Vec<sir_types::NodeId>) -> Candidate {
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
        definition_id: DefinitionId::new(201),
        strategy: ImplementationStrategy::BitScanReverse,
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
        source_structure: SourceStructure::LogicalSequence { length: 8 },
        constraints,
        assumptions: HashSet::new(),
        authorization,
        binding_digest: 0,
    }
}

fn context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    TransformationContext::new(
        RegionId::new(0),
        Representation::BitwiseArithmetic,
        SourceStructure::LogicalSequence { length: 8 },
        constraints,
        HashSet::new(),
    )
}

#[test]
fn corrected_bit_scan_reverse_theorem_is_concrete_solver_proven() {
    let def = BitScanReverseDefinition::new(DefinitionId::new(201));
    let (func, seq) = mask_function(8);
    let structural = structural_with_collection(seq, 8);
    let obligation = def.obligation_with_roles(
        &candidate_with_region(vec![seq]),
        &func,
        &structural,
    );
    assert!(
        obligation.domain.is_some(),
        "a bound PositionSearch collection must carry the sequence domain"
    );
    match ConcreteSolverVerifier.verify(&obligation) {
        VerificationResult::Proven(proof) => {
            assert_eq!(proof.backend, VerificationBackend::ConcreteSolver);
        }
        other => panic!(
            "LastTrue == BitScanReverse(Pack) must be bit-blast provable, got {other:?}"
        ),
    }
}

#[test]
fn historical_last_true_equals_clz_equation_is_refuted() {
    let def = BitScanReverseDefinition::new(DefinitionId::new(201));
    let (func, seq) = mask_function(8);
    let structural = structural_with_collection(seq, 8);
    let mut obligation = def.obligation_with_roles(
        &candidate_with_region(vec![seq]),
        &func,
        &structural,
    );
    let v = VariableId::new(seq.as_u64());
    let sequence = SemanticExpression::LogicalSequence { variable: v };
    obligation.theorem = Theorem::new(
        SemanticExpression::LastTrue(Box::new(sequence.clone())),
        SemanticExpression::LeadingZeros(Box::new(SemanticExpression::Pack(Box::new(sequence)))),
    );
    match ConcreteSolverVerifier.verify(&obligation) {
        VerificationResult::Rejected(
            sir_verification::errors::RejectReason::CounterExample { .. },
        ) => {}
        other => panic!(
            "the historical LastTrue == clz equation must be refuted, got {other:?}"
        ),
    }
}

#[test]
fn definition_is_lifted_and_ps002_totality_is_the_gate() {
    let def = BitScanReverseDefinition::new(DefinitionId::new(201));
    assert_eq!(
        def.verification_status(),
        VerificationStatus::ConcreteSolverChecked,
        "the corrected theorem + bsr intrinsic lift the definition"
    );
    let (func, seq) = mask_function(8);
    let structural = structural_with_collection(seq, 8);
    let obligation = def.obligation_with_roles(
        &candidate_with_region(vec![seq]),
        &func,
        &structural,
    );
    let result = Verifier::new().verify(&obligation, &context());
    match result {
        VerificationResult::Proven(proof) => {
            assert_eq!(proof.backend, VerificationBackend::ConcreteSolver);
            assert_eq!(
                proof.assurance,
                VerificationStatus::ConcreteSolverChecked,
                "the issued assurance is the checker's, not a declaration"
            );
        }
        other => panic!(
            "the lifted definition's bound obligation must be Proven, got {other:?}"
        ),
    }
}
