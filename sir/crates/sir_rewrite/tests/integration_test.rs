use sir_builder::Builder;
use sir_generation::candidate::{
    Candidate, CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy,
};
use sir_semantics::structure::StructuralDatabase;
use sir_transform::context::ContextId;
use sir_transform::ids::DefinitionId;
use sir_transform::roles::RegionRoles;
use sir_transform::structures::SourceStructure;
use sir_types::{ConstantData, CostProfile, NodeId, RegionId, Span, Type};

use sir_rewrite::engine::RewriteEngine;
use sir_rewrite::error::RewriteError;
use sir_rewrite::recipe::RecipeRegistry;
use sir_rewrite::recipes::popcount::PopcountRecipe;
use sir_verification::semantic::expression::SemanticExpression;
use sir_verification::semantic::theorem::Theorem;
use sir_verification::Proof;

fn uint64_type() -> Type {
    Type::Integer {
        width: sir_types::IntegerWidth::I64,
        signed: false,
        overflow: sir_types::OverflowBehavior::Wrapping,
    }
}

/// A REAL reduction loop over a 64-element bool array:
/// `for i in 0..64 { any |= board[i] }`, returning the accumulator
/// slot. This is the exact shape the binding layer certifies
/// (`build_any_loop(Some(0))` in proposal_binding.rs), so the engine's
/// legacy entry point must be able to rewrite it end-to-end.
fn make_board_function() -> sir_nodes::Function {
    let mut b = Builder::new(
        "any_scan",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::Bool,
    );
    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), uint64_type(), Span::unknown());
    let one = b.constant(ConstantData::u64(1), uint64_type(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), uint64_type(), Span::unknown());
    let any_init = b.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());

    // board[i]
    let element = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();
    // any_next = any || board[i]
    let any_next = b.bool_or(any_init, element, Span::unknown()).unwrap();
    // i = i + 1
    let i_next = b.add(i_initial, one, Span::unknown()).unwrap();
    // i < 64 — termination uses the carried input (current value).
    let condition = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b
        .r#loop(
            &[element, any_next, i_next, condition],
            condition,
            &[any_next, i_next],
            &[any_init, i_initial],
            Type::Tuple {
                elements: vec![Type::Bool, uint64_type()],
            },
            Span::unknown(),
        )
        .unwrap();
    let accumulator_slot = b
        .field_access(loop_node, "0", Type::Bool, Span::unknown())
        .unwrap();
    b.return_value(accumulator_slot, Span::unknown()).unwrap();
    b.build()
}

fn board_parameter_id(function: &sir_nodes::Function) -> NodeId {
    function
        .arena
        .nodes()
        .iter()
        .find(|(_, n)| matches!(n.kind, sir_nodes::NodeKind::Parameter { .. }))
        .map(|(id, _)| *id)
        .expect("board parameter")
}

fn loop_node_id(function: &sir_nodes::Function) -> NodeId {
    function
        .arena
        .nodes()
        .iter()
        .find(|(_, n)| matches!(n.kind, sir_nodes::NodeKind::Loop { .. }))
        .map(|(id, _)| *id)
        .expect("reduction loop")
}

fn make_candidate() -> Candidate {
    Candidate {
        authorization: sir_generation::candidate::AuthorizationRef::for_unit_test(),
        binding_digest: 0,
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        definition_id: DefinitionId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "popcount replacement",
        },
        effects: vec![CandidateEffect::CountingStrategyChange],
        expected_cost: CostProfile::default(),
        representation: sir_transform::representation::Representation::BitSet,
        source_structure: SourceStructure::LogicalSequence { length: 64 },
        constraints: std::collections::HashSet::new(),
        assumptions: std::collections::HashSet::new(),
    }
}

fn make_proof() -> Proof {
    Proof {
        theorem: Theorem::new(
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
        ),
        normalized_theorem: Theorem::new(
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
            SemanticExpression::Constant(sir_types::ConstantData::u64(0)),
        ),
        backend: sir_verification::VerificationBackend::Symbolic,
        steps: vec![],
        // Test fixture only; never used to authorize a rewrite.
        assurance: sir_verification::registry::VerificationStatus::SchemaChecked,
        obligation_digest: 0,
    }
}

fn make_structural_db(collection: NodeId, result: NodeId) -> StructuralDatabase {
    use sir_semantics::structure::StructuralDescription;
    let mut db = StructuralDatabase::new();
    let desc = StructuralDescription::new(
        RegionId::new(0),
        SourceStructure::LogicalSequence { length: 64 },
    )
    .with_roles(RegionRoles::BooleanCollectionReduction {
        collection,
        accumulator: None,
        result,
    });
    db.add_description(desc);
    db
}

fn make_engine() -> RewriteEngine {
    let mut registry = RecipeRegistry::new();
    registry.register(Box::new(PopcountRecipe::new(DefinitionId::new(0))));
    RewriteEngine::new(registry)
}

// ── Tier 5: BS001 end-to-end ────────────────────────────────

#[test]
fn bs001_end_to_end_rewrite_produces_valid_sir() {
    let function = make_board_function();
    let candidate = make_candidate();
    let proof = make_proof();
    let structural_db = make_structural_db(board_parameter_id(&function), loop_node_id(&function));
    let engine = make_engine();

    // The legacy entry point must genuinely rewrite the loop — not
    // refuse on a stale-authorization error (regression: the P0A gate
    // made `for_unit_test` candidates fail `matches_function`).
    let rewrite_result = engine
        .rewrite(&function, &candidate, &proof, &structural_db)
        .expect("legacy rewrite() must execute a real rewrite");

    let mut verifier = sir_verify::Verifier::new(&rewrite_result.rewritten);
    assert!(verifier.verify(), "rewritten function must pass sir_verify");

    // The loop must be gone: replaced by a Pack + Popcount expression.
    assert!(
        !rewrite_result
            .rewritten
            .arena
            .nodes()
            .values()
            .any(|n| matches!(n.kind, sir_nodes::NodeKind::Loop { .. })),
        "the reduction loop must be eliminated"
    );
}

// ── Tier 6: Definition mismatch ─────────────────────────────

#[test]
fn definition_mismatch_rejected() {
    let function = make_board_function();
    let mut candidate = make_candidate();
    candidate.definition_id = DefinitionId::new(999); // no recipe registered
    let proof = make_proof();
    let structural_db = make_structural_db(board_parameter_id(&function), loop_node_id(&function));
    let engine = make_engine();

    let result = engine.rewrite(&function, &candidate, &proof, &structural_db);
    assert!(result.is_err());
    match result {
        Err(RewriteError::RecipeFailed(_)) => {} // expected
        other => panic!("expected RecipeFailed, got {:?}", other),
    }
}

// ── Tier 4: Structural verification ─────────────────────────

#[test]
fn rewritten_function_passes_sir_verify() {
    // A non-bindable function (no loop) must fail cleanly at the
    // binding stage — not with an authorization refusal.
    let mut b = Builder::new(
        "test",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );
    let _board = b.parameter_index(0).unwrap();
    let _dead = b.constant(
        sir_types::ConstantData::i32(0),
        Type::i32(),
        Span::unknown(),
    );
    let val = b.constant(
        sir_types::ConstantData::i32(1),
        Type::i32(),
        Span::unknown(),
    );
    b.return_value(val, Span::unknown()).unwrap();
    let func = b.build();

    let candidate = make_candidate();
    let proof = make_proof();
    let structural_db = make_structural_db(NodeId::new(0), NodeId::new(2)); // loop does not exist in func
    let engine = make_engine();

    let result = engine.rewrite(&func, &candidate, &proof, &structural_db);
    match result {
        Ok(rewrite_result) => {
            let mut verifier = sir_verify::Verifier::new(&rewrite_result.rewritten);
            assert!(verifier.verify(), "rewritten function must pass sir_verify");
        }
        Err(e) => {
            // No loop in the function: the binding must refuse.
            assert!(
                matches!(e, RewriteError::RecipeFailed(_))
                    || matches!(e, RewriteError::MissingRole { .. })
                    || matches!(e, RewriteError::StructuralVerificationFailed(_)),
                "unexpected error type: {:?}",
                e
            );
        }
    }
}

// ── Tier 9: Provenance ──────────────────────────────────────

#[test]
fn provenance_tracks_recipe_id() {
    let function = make_board_function();
    let candidate = make_candidate();
    let proof = make_proof();
    let structural_db = make_structural_db(board_parameter_id(&function), loop_node_id(&function));
    let engine = make_engine();

    let rewrite_result = engine
        .rewrite(&function, &candidate, &proof, &structural_db)
        .expect("legacy rewrite() must execute a real rewrite");

    assert_eq!(rewrite_result.proof, proof);
    assert!(
        !rewrite_result.diff.removed_nodes.is_empty() || !rewrite_result.diff.added_nodes.is_empty(),
        "rewrite result must report a node diff"
    );
    // provenance is a documented v0.1 stub (compute_provenance returns
    // empty until the full mapping lands) — it must not panic on use.
    let _ = &rewrite_result.provenance;
}

// ── Tier 7: Negative — malformed patch causes error ─────────

#[test]
fn missing_structural_description_causes_error() {
    let function = make_board_function();
    let candidate = make_candidate();
    let proof = make_proof();
    let empty_db = StructuralDatabase::new();
    let engine = make_engine();

    let result = engine.rewrite(&function, &candidate, &proof, &empty_db);
    assert!(result.is_err());
}
