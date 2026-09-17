//! Scan assurance beyond the 64-bit concrete solver (2026-09-17).
//!
//! A forward search over a 96-element collection cannot be bit-blasted
//! by the u64 solver, so the binding must still produce an obligation
//! (extent <= 512) and the verifier's symbolic scan identity discharges
//! it, honestly issued as SchemaChecked. The sentinel guard refuses a
//! loop whose no-hit result is not the extent.

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
use sir_verification::definitions::bitscan_forward::BitScanForwardDefinition;
use sir_verification::registry::TransformationDefinition;
use sir_verification::{VerificationBackend, VerificationResult, Verifier};

fn span() -> Span {
    Span::unknown()
}

/// Forward found-flag search over `[u8; 96]` with a parameterizable
/// sentinel. Returns the function, the collection node, and the loop.
fn forward_search(sentinel: u64) -> (sir_nodes::Function, sir_types::NodeId, sir_types::NodeId) {
    let u64ty = Type::u64();
    let arr_ty = Type::Array {
        element: Box::new(Type::u8()),
        length: 96,
    };
    let mut b = Builder::new("forward_scan", &[("seq", arr_ty)], u64ty.clone());
    let seq = b.parameter_index(0).unwrap();
    let zero = b.constant(ConstantData::u64(0), u64ty.clone(), span());
    let one = b.constant(ConstantData::u64(1), u64ty.clone(), span());
    let bound = b.constant(ConstantData::u64(96), u64ty.clone(), span());
    let sentinel = b.constant(ConstantData::u64(sentinel), u64ty.clone(), span());
    let elem_zero = b.constant(ConstantData::u8(0), Type::u8(), span());
    let found_init = b.constant(ConstantData::boolean(false), Type::Bool, span());

    let elem = b.array_access(seq, zero, Type::u8(), span()).unwrap();
    let hit = b.ne(elem, elem_zero, span()).unwrap();
    let new_found = b.bool_or(found_init, hit, span()).unwrap();
    let not_found_yet = b.bool_not(found_init, span()).unwrap();
    let is_first = b.bool_and(hit, not_found_yet, span()).unwrap();
    let new_index = b.select(is_first, zero, sentinel, span()).unwrap();
    let i_next = b.add(zero, one, span()).unwrap();
    let not_found = b.bool_not(found_init, span()).unwrap();
    let in_bounds = b.lt(zero, bound, span()).unwrap();
    let cond = b.bool_and(not_found, in_bounds, span()).unwrap();
    let loop_node = b
        .r#loop(
            &[
                elem, hit, new_found, not_found_yet, is_first, new_index, i_next, not_found,
                in_bounds, cond,
            ],
            cond,
            &[new_found, new_index, i_next],
            &[found_init, sentinel, zero],
            Type::Tuple {
                elements: vec![Type::Bool, u64ty.clone(), u64ty],
            },
            span(),
        )
        .unwrap();
    let res = b.field_access(loop_node, "1", Type::u64(), span()).unwrap();
    b.return_value(res, span()).unwrap();
    (b.build(), seq, loop_node)
}

fn candidate() -> Candidate {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(96));
    let mut authorization = AuthorizationRef::for_unit_test();
    authorization.region_nodes = vec![];
    Candidate {
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        definition_id: DefinitionId::new(200),
        strategy: ImplementationStrategy::BitScanForward,
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
        source_structure: SourceStructure::LogicalSequence { length: 96 },
        constraints,
        assumptions: HashSet::new(),
        authorization,
        binding_digest: 0,
    }
}

fn context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(96));
    TransformationContext::new(
        RegionId::new(0),
        Representation::BitwiseArithmetic,
        SourceStructure::LogicalSequence { length: 96 },
        constraints,
        HashSet::new(),
    )
}

fn structural(collection: sir_types::NodeId, result: sir_types::NodeId) -> StructuralDescription {
    let mut structural = StructuralDescription::new(
        RegionId::new(0),
        SourceStructure::LogicalSequence { length: 96 },
    );
    structural.roles.push(RegionRoles::PositionSearch {
        collection: Some(collection),
        scalar: None,
        result,
    });
    structural
}

#[test]
fn forward_scan_96_is_symbolically_proven_as_schema_checked() {
    let (func, seq, loop_node) = forward_search(96);
    let def = BitScanForwardDefinition::new(DefinitionId::new(200));
    let obligation =
        def.obligation_with_roles(&candidate(), &func, &structural(seq, loop_node));
    assert!(
        obligation.domain.is_some(),
        "a 96-element extent is within the 512-bit binding cap"
    );
    match Verifier::new().verify(&obligation, &context()) {
        VerificationResult::Proven(proof) => {
            assert_eq!(proof.backend, VerificationBackend::Symbolic);
            assert_eq!(
                proof.assurance,
                sir_verification::registry::VerificationStatus::SchemaChecked,
                "extents beyond the u64 solver are issued as SchemaChecked, \
                 not ConcreteSolverChecked"
            );
        }
        other => panic!("the 96-extent scan identity must be proven, got {other:?}"),
    }
}

#[test]
fn forward_scan_with_a_mismatched_sentinel_is_unbound() {
    let (func, seq, loop_node) = forward_search(95);
    let def = BitScanForwardDefinition::new(DefinitionId::new(200));
    let obligation =
        def.obligation_with_roles(&candidate(), &func, &structural(seq, loop_node));
    assert!(
        obligation.domain.is_none(),
        "a no-hit result that is not the extent must never be bound"
    );
    let result = Verifier::new().verify(&obligation, &context());
    assert!(
        !matches!(result, VerificationResult::Proven(_)),
        "an unbound sentinel mismatch must not be Proven, got {result:?}"
    );
}
