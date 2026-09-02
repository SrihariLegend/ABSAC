//! Hardening tests for the narrow Any vertical slice.
//!
//! These tests intentionally stop before All/Parity/Popcount. They check
//! that a binding mutation is rejected by canonical replay, that the
//! abstract candidate frame is fail-closed, and that source-version
//! authorization cannot survive the real rewrite.

use sir_analysis::facts::FactDatabase;
use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_nodes::{Node, NodeKind};
use sir_rewrite::detached_arena::DetachedArena;
use sir_rewrite::engine::check_candidate_frame;
use sir_rewrite::local_id::LocalNodeId;
use sir_rewrite::patch::ReplacementPatch;
use sir_semantics::authorization::{derive_authorizations, ConcreteFacts};
use sir_semantics::binding::{
    derive_proposal_binding, validate_proposal_binding, BindingError, LiveOutBinding, LiveOutKind,
    ProposalBinding,
};
use sir_semantics::semantics::SemanticEngine;
use sir_semantics::structure::StructuralDescription;
use sir_transform::roles::RegionRoles;
use sir_types::{ConstantData, Effects, NodeId, Span, Type};

struct BoundFixture {
    function: sir_nodes::Function,
    facts: FactDatabase,
    structural: StructuralDescription,
    concrete: ConcreteFacts,
    binding: ProposalBinding,
}

fn bool_array(length: usize) -> Type {
    Type::Array {
        element: Box::new(Type::Bool),
        length,
    }
}

fn i32_array(length: usize) -> Type {
    Type::Array {
        element: Box::new(Type::i32()),
        length,
    }
}

fn build_any_function(
    name: &str,
    length: usize,
    two_arrays: bool,
    start: u64,
    bound: u64,
    stride: i64,
    identity: bool,
    return_ty: Type,
    consumer: impl FnOnce(&mut Builder, NodeId) -> Result<NodeId, sir_builder::BuildError>,
) -> sir_nodes::Function {
    let mut params = vec![("board_a", bool_array(length))];
    if two_arrays {
        params.push(("board_b", bool_array(length)));
    }
    let mut builder = Builder::new(name, &params, return_ty);
    let board = builder.parameter_index(0).unwrap();
    let i_initial = builder.constant(ConstantData::u64(start), Type::u64(), Span::unknown());
    let step = builder.constant(
        ConstantData::u64(stride.unsigned_abs()),
        Type::u64(),
        Span::unknown(),
    );
    let limit = builder.constant(ConstantData::u64(bound), Type::u64(), Span::unknown());
    let any_initial =
        builder.constant(ConstantData::boolean(identity), Type::Bool, Span::unknown());
    let element = builder
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();
    let any_next = builder
        .bool_or(any_initial, element, Span::unknown())
        .unwrap();
    let index_next = if stride >= 0 {
        builder.add(i_initial, step, Span::unknown()).unwrap()
    } else {
        builder.sub(i_initial, step, Span::unknown()).unwrap()
    };
    let condition = if stride >= 0 {
        builder.lt(i_initial, limit, Span::unknown()).unwrap()
    } else {
        builder.gt(i_initial, limit, Span::unknown()).unwrap()
    };
    let loop_node = builder
        .r#loop(
            &[element, any_next, index_next, condition],
            condition,
            &[any_next, index_next],
            &[any_initial, i_initial],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();
    let result = consumer(&mut builder, loop_node).unwrap();
    builder.return_value(result, Span::unknown()).unwrap();
    builder.build()
}

fn build_any_loop(name: &str, length: usize) -> sir_nodes::Function {
    build_any_function(
        name,
        length,
        false,
        0,
        length as u64,
        1,
        false,
        Type::Bool,
        |builder, loop_node| builder.field_access(loop_node, "0", Type::Bool, Span::unknown()),
    )
}

fn bind(function: &sir_nodes::Function) -> BoundFixture {
    let mut analysis = AnalysisManager::new();
    analysis.run_all(function);
    let facts = analysis.database().clone();

    let mut semantics = SemanticEngine::new();
    semantics.derive(function, &facts);
    let structural = semantics
        .structural_database()
        .regions()
        .find(|(_, description)| {
            description.roles.iter().any(|role| {
                matches!(
                    role,
                    RegionRoles::BooleanCollectionReduction { .. }
                        | RegionRoles::PredicateCollectionReduction { .. }
                )
            })
        })
        .map(|(_, description)| description.clone())
        .expect("fixture must produce a collection-reduction role set");
    let authorizations = derive_authorizations(function, &facts, semantics.database());
    let concrete = authorizations
        .for_region(structural.region)
        .iter()
        .find(|authorization| authorization.concrete.accumulator.is_some())
        .map(|authorization| authorization.concrete.clone())
        .unwrap_or_default();
    let binding = derive_proposal_binding(function, &facts, &structural, &concrete)
        .expect("fixture must derive a complete binding");

    BoundFixture {
        function: function.clone(),
        facts,
        structural,
        concrete,
        binding,
    }
}

fn assert_binding_mutation_refused(fixture: &BoundFixture, mutated: ProposalBinding) {
    let result = validate_proposal_binding(
        &fixture.function,
        &fixture.facts,
        &fixture.structural,
        &fixture.concrete,
        &mutated,
    );
    assert!(
        matches!(result, Err(BindingError::BindingMismatch { .. })),
        "binding mutation must refuse, got {result:?}"
    );
}

#[test]
fn canonical_binding_accepts_and_each_role_mutation_refuses() {
    let fixture = bind(&build_any_loop("any_binding_mutations", 64));
    assert!(validate_proposal_binding(
        &fixture.function,
        &fixture.facts,
        &fixture.structural,
        &fixture.concrete,
        &fixture.binding,
    )
    .is_ok());

    let mut collection = fixture.binding.clone();
    collection.map.collection = NodeId::new(1);
    assert_binding_mutation_refused(&fixture, collection);

    let mut element_access = fixture.binding.clone();
    element_access.map.element_access = NodeId::new(9_999);
    assert_binding_mutation_refused(&fixture, element_access);

    let mut start = fixture.binding.clone();
    start.map.start = Some(NodeId::new(9_999));
    assert_binding_mutation_refused(&fixture, start);

    let mut bound = fixture.binding.clone();
    bound.map.bound = Some(NodeId::new(9_999));
    assert_binding_mutation_refused(&fixture, bound);

    let mut stride = fixture.binding.clone();
    stride.map.stride = -1;
    assert_binding_mutation_refused(&fixture, stride);

    let mut accumulator = fixture.binding.clone();
    accumulator.map.accumulator = NodeId::new(9_999);
    assert_binding_mutation_refused(&fixture, accumulator);

    let mut identity = fixture.binding.clone();
    identity.map.identity = Some(ConstantData::boolean(true));
    assert_binding_mutation_refused(&fixture, identity);

    let mut slot = fixture.binding.clone();
    slot.map.reduction_position = 1;
    assert_binding_mutation_refused(&fixture, slot);

    let mut result = fixture.binding.clone();
    result.map.loop_node = NodeId::new(9_999);
    assert_binding_mutation_refused(&fixture, result);

    let mut replacement_site = fixture.binding.clone();
    let observable = replacement_site
        .live_outs
        .iter_mut()
        .find(|live_out| live_out.use_site.is_some())
        .expect("fixture must have a reconstructed replacement site");
    observable.use_site = Some(NodeId::new(9_999));
    assert_binding_mutation_refused(&fixture, replacement_site);

    let mut live_kind = fixture.binding.clone();
    let reconstructed = live_kind
        .live_outs
        .iter_mut()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Reconstructed { .. }))
        .unwrap();
    reconstructed.kind = LiveOutKind::Slot(99);
    assert_binding_mutation_refused(&fixture, live_kind);

    let mut live_binding = fixture.binding.clone();
    let reconstructed = live_binding
        .live_outs
        .iter_mut()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Reconstructed { .. }))
        .unwrap();
    reconstructed.binding = LiveOutBinding::Preserved;
    assert_binding_mutation_refused(&fixture, live_binding);

    let mut live_closure = fixture.binding.clone();
    let reconstructed = live_closure
        .live_outs
        .iter_mut()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Reconstructed { .. }))
        .unwrap();
    reconstructed
        .closure
        .as_mut()
        .unwrap()
        .closure_nodes
        .push(NodeId::new(9_999));
    assert_binding_mutation_refused(&fixture, live_closure);

    let mut dead_evidence = fixture.binding.clone();
    let dead = dead_evidence
        .live_outs
        .iter_mut()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Dead { .. }))
        .unwrap();
    if let LiveOutBinding::Dead { evidence } = &mut dead.binding {
        evidence.function_fingerprint ^= 1;
    }
    assert_binding_mutation_refused(&fixture, dead_evidence);

    let mut closure_evidence = fixture.binding.clone();
    let dead = closure_evidence
        .live_outs
        .iter_mut()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Dead { .. }))
        .unwrap();
    if let LiveOutBinding::Dead { evidence } = &mut dead.binding {
        evidence.closure_nodes.push(NodeId::new(9_999));
    }
    assert_binding_mutation_refused(&fixture, closure_evidence);

    let mut source_reads = fixture.binding.clone();
    source_reads.frame.source_reads.push(NodeId::new(9_999));
    assert_binding_mutation_refused(&fixture, source_reads);
    let mut source_writes = fixture.binding.clone();
    source_writes.frame.source_writes = true;
    assert_binding_mutation_refused(&fixture, source_writes);
    let mut volatile = fixture.binding.clone();
    volatile.frame.has_volatile_or_atomic = true;
    assert_binding_mutation_refused(&fixture, volatile);
    let mut calls = fixture.binding.clone();
    calls.frame.has_calls = true;
    assert_binding_mutation_refused(&fixture, calls);
    let mut traps = fixture.binding.clone();
    traps.frame.possible_traps = true;
    assert_binding_mutation_refused(&fixture, traps);
    let mut termination = fixture.binding.clone();
    termination.frame.terminates = false;
    assert_binding_mutation_refused(&fixture, termination);
    let mut exits = fixture.binding.clone();
    exits.frame.single_normal_exit = false;
    assert_binding_mutation_refused(&fixture, exits);
    let mut outputs = fixture.binding.clone();
    outputs.frame.output_count += 1;
    assert_binding_mutation_refused(&fixture, outputs);
    let mut trip = fixture.binding.clone();
    trip.frame.trip_count.as_mut().unwrap().bound_value -= 1;
    assert_binding_mutation_refused(&fixture, trip);
}

fn predicate_binding() -> BoundFixture {
    let mut builder = Builder::new(
        "any_predicate_binding_mutations",
        &[("values", i32_array(64)), ("scalar", Type::i32())],
        Type::Bool,
    );
    let values = builder.parameter_index(0).unwrap();
    let scalar = builder.parameter_index(1).unwrap();
    let index = builder.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let step = builder.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let bound = builder.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let identity = builder.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());
    let value = builder
        .array_access(values, index, Type::i32(), Span::unknown())
        .unwrap();
    let predicate = builder.gt(value, scalar, Span::unknown()).unwrap();
    let next_any = builder
        .bool_or(identity, predicate, Span::unknown())
        .unwrap();
    let next_index = builder.add(index, step, Span::unknown()).unwrap();
    let condition = builder.lt(index, bound, Span::unknown()).unwrap();
    let loop_node = builder
        .r#loop(
            &[value, predicate, next_any, next_index, condition],
            condition,
            &[next_any, next_index],
            &[identity, index],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();
    let result = builder
        .field_access(loop_node, "0", Type::Bool, Span::unknown())
        .unwrap();
    builder.return_value(result, Span::unknown()).unwrap();
    bind(&builder.build())
}

#[test]
fn predicate_operator_and_scalar_mutations_refuse() {
    let fixture = predicate_binding();
    assert_eq!(
        fixture.binding.map.predicate_op,
        Some(sir_nodes::CmpOperator::Gt)
    );
    assert!(fixture.binding.map.predicate_scalar.is_some());

    let mut operator = fixture.binding.clone();
    operator.map.predicate_op = Some(sir_nodes::CmpOperator::Ne);
    assert_binding_mutation_refused(&fixture, operator);

    let mut scalar = fixture.binding.clone();
    scalar.map.predicate_scalar = Some(NodeId::new(0));
    assert_binding_mutation_refused(&fixture, scalar);
}

fn single_node_patch(kind: NodeKind, ty: Type, effects: Effects) -> ReplacementPatch {
    let local = LocalNodeId::new(1_000_000_000);
    let mut arena = DetachedArena::new();
    arena.insert(
        local,
        Node::new(
            NodeId::new(local.as_u64()),
            kind,
            ty,
            effects,
            Span::unknown(),
        ),
    );
    ReplacementPatch::new(arena, vec![local], vec![])
}

#[test]
fn candidate_frame_rejects_wrong_collection_undeclared_input_and_effect() {
    let function = build_any_function(
        "any_candidate_frame",
        64,
        false,
        0,
        64,
        1,
        false,
        Type::Bool,
        |builder, loop_node| builder.field_access(loop_node, "0", Type::Bool, Span::unknown()),
    );
    let fixture = bind(&function);
    let other_collection = NodeId::new(9_999);

    let wrong_collection = single_node_patch(
        NodeKind::Pack {
            array: other_collection,
        },
        Type::BitVector { width: 64 },
        Effects::empty(),
    );
    assert!(check_candidate_frame(
        &function,
        &wrong_collection,
        &fixture.binding,
        sir_generation::candidate::ImplementationStrategy::Any,
    )
    .is_err());

    let undeclared_input = single_node_patch(
        NodeKind::Ne {
            lhs: other_collection,
            rhs: other_collection,
        },
        Type::Bool,
        Effects::empty(),
    );
    assert!(check_candidate_frame(
        &function,
        &undeclared_input,
        &fixture.binding,
        sir_generation::candidate::ImplementationStrategy::Any,
    )
    .is_err());

    let effectful = single_node_patch(
        NodeKind::Store {
            ptr: fixture.binding.map.collection,
            value: fixture.binding.map.collection,
        },
        Type::Unit,
        Effects::WRITE_MEMORY,
    );
    assert!(check_candidate_frame(
        &function,
        &effectful,
        &fixture.binding,
        sir_generation::candidate::ImplementationStrategy::Any,
    )
    .is_err());

    let predicate = predicate_binding();
    let predicate_scalar = predicate.binding.map.predicate_scalar.unwrap();
    let wrong_operator = single_node_patch(
        NodeKind::ArrayCmpMask {
            array: predicate.binding.map.collection,
            scalar: predicate_scalar,
            op: sir_nodes::CmpOperator::Ne,
        },
        Type::BitVector { width: 64 },
        Effects::empty(),
    );
    assert!(check_candidate_frame(
        &predicate.function,
        &wrong_operator,
        &predicate.binding,
        sir_generation::candidate::ImplementationStrategy::Any,
    )
    .is_err());
    let wrong_scalar = single_node_patch(
        NodeKind::ArrayCmpMask {
            array: predicate.binding.map.collection,
            scalar: NodeId::new(9_999),
            op: sir_nodes::CmpOperator::Gt,
        },
        Type::BitVector { width: 64 },
        Effects::empty(),
    );
    assert!(check_candidate_frame(
        &predicate.function,
        &wrong_scalar,
        &predicate.binding,
        sir_generation::candidate::ImplementationStrategy::Any,
    )
    .is_err());
}

#[test]
fn duplicate_structural_reduction_role_refuses_binding() {
    let fixture = bind(&build_any_loop("any_duplicate_role", 64));
    let mut structural = fixture.structural.clone();
    let role = structural
        .roles
        .iter()
        .find(|role| {
            matches!(
                role,
                RegionRoles::BooleanCollectionReduction { .. }
                    | RegionRoles::PredicateCollectionReduction { .. }
            )
        })
        .cloned()
        .expect("fixture must contain a reduction role");
    structural.roles.push(role);
    let result = derive_proposal_binding(
        &fixture.function,
        &fixture.facts,
        &structural,
        &fixture.concrete,
    );
    assert!(
        matches!(result, Err(BindingError::AmbiguousRole("reduction role"))),
        "duplicate reduction roles must refuse, got {result:?}"
    );
}

#[test]
fn multiple_boolean_collections_refuse_role_derivation() {
    let function = build_any_function(
        "any_ambiguous_collections",
        64,
        true,
        0,
        64,
        1,
        false,
        Type::Bool,
        |builder, loop_node| builder.field_access(loop_node, "0", Type::Bool, Span::unknown()),
    );
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&function);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&function, analysis.database());
    assert!(!semantics
        .structural_database()
        .regions()
        .any(|(_, description)| description.roles.iter().any(|role| {
            matches!(
                role,
                RegionRoles::BooleanCollectionReduction { .. }
                    | RegionRoles::PredicateCollectionReduction { .. }
            )
        })));
}

#[test]
fn transparent_projection_with_multiple_downstream_users_rewrites() {
    let function = build_any_function(
        "any_transparent_projection",
        64,
        false,
        0,
        64,
        1,
        false,
        Type::Bool,
        |builder, loop_node| {
            let projection = builder.field_access(loop_node, "0", Type::Bool, Span::unknown())?;
            let inverted = builder.bool_not(projection, Span::unknown()).unwrap();
            builder.bool_and(projection, inverted, Span::unknown())
        },
    );
    let fixture = bind(&function);
    let reconstructed = fixture
        .binding
        .live_outs
        .iter()
        .find(|live_out| matches!(&live_out.binding, LiveOutBinding::Reconstructed { .. }))
        .unwrap();
    let closure = reconstructed.closure.as_ref().unwrap();
    assert_eq!(closure.direct_users, 1);
    assert!(closure.closure_nodes.len() >= 4);

    let optimizer = sir_optimizer::optimizer::Optimizer::new(
        sir_optimizer::config::OptimizerConfig::default(),
        sir_rewrite::registry::any_only_registry(),
    );
    let result = optimizer.optimize(&function);
    assert_eq!(result.rewrites_applied, 1);
}

#[test]
fn source_with_second_live_out_refuses_binding() {
    let function = build_any_function(
        "any_second_live_out",
        64,
        false,
        0,
        64,
        1,
        false,
        Type::u64(),
        |builder, loop_node| {
            let accumulator = builder.field_access(loop_node, "0", Type::Bool, Span::unknown())?;
            let index = builder.field_access(loop_node, "1", Type::u64(), Span::unknown())?;
            let zero = builder.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
            Ok(builder.select(accumulator, index, zero, Span::unknown())?)
        },
    );
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&function);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&function, analysis.database());
    let structural = semantics
        .structural_database()
        .regions()
        .find(|(_, description)| {
            description
                .roles
                .iter()
                .any(|role| matches!(role, RegionRoles::BooleanCollectionReduction { .. }))
        })
        .map(|(_, description)| description.clone())
        .expect("second-live-out fixture must be recognized structurally");
    let authorizations =
        derive_authorizations(&function, analysis.database(), semantics.database());
    let concrete = authorizations
        .for_region(structural.region)
        .first()
        .map(|authorization| authorization.concrete.clone())
        .unwrap_or_default();
    let result = derive_proposal_binding(&function, analysis.database(), &structural, &concrete);
    assert!(
        matches!(result, Err(BindingError::UnclassifiedUse { .. })),
        "second live-out must refuse, got {result:?}"
    );
}

#[test]
fn authorization_is_stale_after_the_real_any_rewrite() {
    let function = build_any_loop("any_authorization_version", 64);
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&function);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&function, analysis.database());
    let structural = semantics
        .structural_database()
        .regions()
        .find(|(_, description)| {
            description
                .roles
                .iter()
                .any(|role| matches!(role, RegionRoles::BooleanCollectionReduction { .. }))
        })
        .map(|(_, description)| description.clone())
        .expect("Any fixture must produce a reduction role set");
    let authorizations =
        derive_authorizations(&function, analysis.database(), semantics.database());
    let authorization = authorizations
        .for_region(structural.region)
        .iter()
        .find(|authorization| authorization.concrete.accumulator.is_some())
        .expect("Any fixture must receive a reduction authorization")
        .clone();

    let optimizer = sir_optimizer::optimizer::Optimizer::new(
        sir_optimizer::config::OptimizerConfig::default(),
        sir_rewrite::registry::any_only_registry(),
    );
    let result = optimizer.optimize(&function);
    assert_eq!(result.rewrites_applied, 1);
    assert!(
        !authorization.is_valid_for(&result.function),
        "an authorization from the source version must not survive mutation"
    );
}
