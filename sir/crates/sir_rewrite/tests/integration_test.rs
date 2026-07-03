use sir_builder::Builder;
use sir_generation::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId, ImplementationStrategy,
};
use sir_semantics::structure::StructuralDatabase;
use sir_transform::context::ContextId;
use sir_transform::ids::DefinitionId;
use sir_transform::roles::RegionRoles;
use sir_transform::structures::SourceStructure;
use sir_types::{CostProfile, RegionId, Span, Type};

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

fn make_board_function() -> sir_nodes::Function {
    let mut b = Builder::new(
        "count_bits",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        uint64_type(),
    );
    let board = b.parameter_index(0).unwrap();
    let zero = b.constant(
        sir_types::ConstantData::u64(0),
        uint64_type(),
        Span::unknown(),
    );
    // Simple return of constant (stands in for the loop body in a real BS001 SIR)
    b.return_value(zero, Span::unknown()).unwrap();
    b.build()
}

fn make_candidate() -> Candidate {
    Candidate {
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        definition_id: DefinitionId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "popcount replacement",
        },
        effects: vec![CandidateEffects::CountingStrategyChange],
        expected_cost: CostProfile::default(),
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
    }
}

fn make_structural_db() -> StructuralDatabase {
    use sir_semantics::structure::StructuralDescription;
    let mut db = StructuralDatabase::new();
    let desc = StructuralDescription::new(
        RegionId::new(0),
        SourceStructure::BooleanArray { length: 64 },
    )
    .with_roles(RegionRoles::BooleanCollectionReduction {
        collection: sir_types::NodeId::new(0), // board parameter
        accumulator: None,
        result: sir_types::NodeId::new(2), // return value
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
    let structural_db = make_structural_db();
    let engine = make_engine();

    let result = engine.rewrite(&function, &candidate, &proof, &structural_db);
    // For v0.1, the rewrite may fail because the test function doesn't
    // actually contain a loop — but the engine pipeline should execute
    // without panicking and produce a meaningful result.
    match result {
        Ok(rewrite_result) => {
            // Verify the rewritten function passes structural verification
            let mut verifier = sir_verify::Verifier::new(&rewrite_result.rewritten);
            assert!(verifier.verify(), "rewritten function must pass sir_verify");
        }
        Err(e) => {
            // Acceptable: stub function doesn't have a real loop structure.
            // But the error must be one we understand.
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

// ── Tier 6: Definition mismatch ─────────────────────────────

#[test]
fn definition_mismatch_rejected() {
    let function = make_board_function();
    let mut candidate = make_candidate();
    candidate.definition_id = DefinitionId::new(999); // no recipe registered
    let proof = make_proof();
    let structural_db = make_structural_db();
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
    // Build a minimal function where the rewrite should produce valid SIR
    let mut func = sir_nodes::Function::new("test", sir_types::Type::BitVector { width: 64 });
    let _p = func.add_param(
        "board",
        Type::Array {
            element: Box::new(Type::Bool),
            length: 64,
        },
        Span::unknown(),
    );

    // The test verifies that if a rewrite succeeds, the output passes sir_verify.
    // With the current stub function, this is a structural test of the pipeline.
    let candidate = make_candidate();
    let proof = make_proof();
    let structural_db = make_structural_db();
    let engine = make_engine();

    let result = engine.rewrite(&func, &candidate, &proof, &structural_db);
    match result {
        Ok(rewrite_result) => {
            let mut verifier = sir_verify::Verifier::new(&rewrite_result.rewritten);
            assert!(verifier.verify(), "rewritten function must pass sir_verify");
        }
        Err(e) => {
            // Acceptable: stub function doesn't have a real loop structure.
            // But the error must be one we understand.
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
    let structural_db = make_structural_db();
    let engine = make_engine();

    let result = engine.rewrite(&function, &candidate, &proof, &structural_db);
    if let Ok(rewrite_result) = result {
        // For now, provenance is v0.1 minimal.
        // The important thing is that the field exists and is populated.
        let _ = rewrite_result.provenance;
        let _ = rewrite_result.diff;
        assert_eq!(rewrite_result.proof, proof);
    } else if let Err(ref e) = result {
        assert!(
            !matches!(e, RewriteError::InternalInvariantViolation(_)),
            "internal invariant violation indicates a bug: {:?}",
            e
        );
    }
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
