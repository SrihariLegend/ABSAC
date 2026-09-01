//! ProposalBinding integration tests (advisor sequence item 4).
//!
//! The binding layer must bind every role concretely and classify
//! every observable use of the loop result — fail-closed. The enabled
//! case is the single accumulator-slot extract (the surviving
//! whole-tuple-quarantine condition). Whole-tuple returns and
//! non-reduction slot consumers must REFUSE (PS002 canonical lesson:
//! a true theorem applied to the wrong observable boundary is still
//! an incorrect compiler transformation).

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_semantics::authorization::ConcreteFacts;
use sir_semantics::binding::{derive_proposal_binding, BindingError, LiveOutBinding, LiveOutKind};
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::structure::StructuralDescription;
use sir_types::{ConstantData, Span, Type};

/// Build a pure Any-reduction loop over a boolean array with a
/// parameterizable consumer of the loop tuple.
fn build_any_loop(field: Option<usize>) -> sir_nodes::Function {
    let ret_ty = Type::Tuple {
        elements: vec![Type::Bool, Type::u64()],
    };
    let mut b = Builder::new(
        "any_scan",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        ret_ty.clone(),
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let one = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let any_init = b.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());

    // board[i]
    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();
    // any_next = any || board[i]
    let any_next = b.bool_or(any_init, elem, Span::unknown()).unwrap();
    // i = i + 1
    let i_next = b.add(i_initial, one, Span::unknown()).unwrap();
    // i < 64 — termination uses the carried input (current value).
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[elem, any_next, i_next, cond],
            cond,
            &[any_next, i_next],
            &[any_init, i_initial],
            ret_ty,
            Span::unknown(),
        )
        .unwrap();

    match field {
        Some(slot) => {
            let extract = b
                .field_access(loop_node, slot.to_string(), Type::Bool, Span::unknown())
                .unwrap();
            b.return_value(extract, Span::unknown()).unwrap();
        }
        None => {
            b.return_value(loop_node, Span::unknown()).unwrap();
        }
    }

    b.build()
}

/// Run the real pipeline (analysis → semantics → authorizations) and
/// derive the proposal binding for the reduction region.
fn derive_for(
    function: &sir_nodes::Function,
) -> Result<sir_semantics::binding::ProposalBinding, BindingError> {
    let mut analysis = AnalysisManager::new();
    analysis.run_all(function);

    let mut semantics = SemanticEngine::new();
    semantics.derive(function, analysis.database());

    // The structural description carrying the reduction role set.
    let structural: StructuralDescription = semantics
        .structural_database()
        .regions()
        .find(|(_, desc)| {
            desc.roles.iter().any(|role| {
                matches!(
                    role,
                    sir_transform::roles::RegionRoles::BooleanCollectionReduction { .. }
                )
            })
        })
        .map(|(_, desc)| desc.clone())
        .expect("test function must produce a reduction role set");

    // Concrete facts from the production authorization database.
    let authorizations = sir_semantics::authorization::derive_authorizations(
        function,
        analysis.database(),
        semantics.database(),
    );
    let concrete: ConcreteFacts = authorizations
        .for_region(structural.region)
        .first()
        .map(|auth| auth.concrete.clone())
        .unwrap_or_default();

    derive_proposal_binding(function, analysis.database(), &structural, &concrete)
}

#[test]
fn any_accumulator_slot_extract_binds_completely() {
    let func = build_any_loop(Some(0));
    let binding = derive_for(&func).expect("enabled case must bind completely");

    // Frame: conservative contract satisfied.
    assert!(binding.frame.supported_conservative());
    assert!(binding.is_complete());

    // Roles bound concretely.
    let map = &binding.map;
    assert_eq!(map.recurrence, "bitwise_or");
    assert_eq!(map.reduction_position, 0);
    assert_eq!(map.stride, 1);
    assert_eq!(map.identity, Some(ConstantData::boolean(false)));
    assert!(map.induction.is_some());
    assert!(map.bound.is_some());
    assert!(map.live_ins.contains(&map.collection));

    // Observable interface: the accumulator slot is reconstructed by
    // the candidate; the induction slot is unobserved (single-use
    // closure) and therefore Dead.
    let mut found_reconstructed = false;
    let mut found_dead = false;
    for slot in &binding.live_outs {
        match (&slot.kind, &slot.binding) {
            (LiveOutKind::Slot(0), LiveOutBinding::Reconstructed { slot: 0 }) => {
                found_reconstructed = true;
            }
            (LiveOutKind::Slot(1), LiveOutBinding::Dead) => {
                found_dead = true;
            }
            other => panic!("unexpected live-out slot: {:?}", other),
        }
    }
    assert!(found_reconstructed);
    assert!(found_dead);

    // Digest is deterministic and stable across re-derivation.
    let func2 = build_any_loop(Some(0));
    let binding2 = derive_for(&func2).unwrap();
    assert_eq!(binding.digest(), binding2.digest());
}

#[test]
fn any_whole_tuple_return_refuses_binding() {
    let func = build_any_loop(None);
    let result = derive_for(&func);
    match binding_err(&result) {
        BindingError::UnclassifiedUse { .. } => {} // PS002 shape refused
        other => panic!("expected UnclassifiedUse, got {:?}", other),
    }
}

#[test]
fn any_index_slot_consumer_refuses_binding() {
    let func = build_any_loop(Some(1));
    let result = derive_for(&func);
    match binding_err(&result) {
        BindingError::UnclassifiedUse { .. } => {
            // The index slot is not covered by the theorem — the
            // binder must not claim it preserved or dead.
        }
        other => panic!("expected UnclassifiedUse, got {:?}", other),
    }
}

fn binding_err(
    result: &Result<sir_semantics::binding::ProposalBinding, BindingError>,
) -> BindingError {
    result.clone().err().expect("expected a binding error")
}
