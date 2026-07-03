//! BS001 Full Pipeline Integration Test.
//!
//! End-to-end test: build SIR -> analyze -> semantic + structural -> inference -> generation.
//! Asserts 4 distinct candidates with all four ImplementationStrategies.

use std::collections::HashSet;

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::candidate::ImplementationStrategy;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

/// Build the canonical BS001 board scan SIR function.
///
/// Represents:
/// ```text
/// fn board_scan(board: [bool; 64]) -> i32 {
///     let mut count = 0;
///     for i in 0..64 {
///         if board[i] { count += 1; }
///     }
///     count
/// }
/// ```
///
/// The SIR graph uses a Loop node with explicit carried variables:
/// - `count` (i32): accumulator, starts at 0, conditionally incremented
/// - `i` (u64): loop counter, iterates 0..64
///
/// The loop body contains an ArrayAccess (board[i]), a Select for
/// conditional increment (board[i] ? 1 : 0), an Add for accumulation,
/// and an Lt comparison for loop termination.
fn build_board_scan() -> sir_nodes::Function {
    let mut b = Builder::new(
        "board_scan",
        &[(
            "board",
            Type::Array {
                element: Box::new(Type::Bool),
                length: 64,
            },
        )],
        Type::i32(),
    );

    // ── Parameters ──────────────────────────────────────────
    let board = b.parameter_index(0).unwrap();

    // ── Constants ───────────────────────────────────────────
    // NOTE: loop counter and bound use unsigned u64 because the loop
    // analysis's trip-count estimation uses `as_u64()` which only
    // decodes unsigned integer constants.
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let zero_i32 = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown());
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown());

    // ── Loop body (references carried inputs as loop variables) ──

    // board[i] — element access (pure, produces Bool)
    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();

    // inc = board[i] ? 1 : 0 — convert boolean to integer increment
    // NOTE: false_val must be a separate zero constant, NOT count_initial,
    // because count_initial is in carried_inputs and using it would cause
    // each iteration to return the accumulated count instead of 0.
    let inc = b
        .select(elem, one_i32, zero_i32, Span::unknown())
        .unwrap();

    // count = count + inc — accumulate (sum reduction)
    let new_count = b
        .add(count_initial, inc, Span::unknown())
        .unwrap();

    // i = i + 1 — increment loop counter
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap();

    // i < 64 — loop termination condition
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    // ── Loop node ──────────────────────────────────────────
    // Carried inputs: [count_initial, i_initial]
    // Outputs: [new_count, i_next]
    //
    // The loop iterates while `cond` is true.
    // Each iteration feeds the outputs back as the carried values.
    let loop_node = b
        .r#loop(
            &[elem, inc, new_count, i_next, cond],
            cond,
            &[new_count, i_next],
            &[count_initial, i_initial],
            Type::Tuple {
                elements: vec![Type::i32(), Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    // ── Return ──────────────────────────────────────────────
    b.return_value(loop_node, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn bs001_full_pipeline_produces_four_distinct_candidates() {
    let func = build_board_scan();

    // Analysis
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    // Semantics + Structure
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    // Inference
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());

    // Generation
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database());

    let db = generator.database();
    assert!(
        db.region_count() > 0,
        "Should have at least one region with candidates"
    );

    // Collect all candidates and verify expectations.
    let mut total_candidates = 0;
    let mut strategies = HashSet::new();
    let mut def_ids = HashSet::new();
    for candidate in db.all_candidates() {
        total_candidates += 1;
        strategies.insert(candidate.strategy);
        def_ids.insert(candidate.definition_id);
    }

    assert_eq!(
        total_candidates, 4,
        "Expected exactly 4 candidates, got {}",
        total_candidates
    );
    assert_eq!(
        strategies.len(),
        4,
        "Expected 4 distinct strategies, got {}",
        strategies.len()
    );
    assert_eq!(
        def_ids.len(),
        4,
        "Expected 4 distinct definition IDs, got {}",
        def_ids.len()
    );
    assert!(
        strategies.contains(&ImplementationStrategy::BitIteration),
        "Missing BitIteration strategy"
    );
    assert!(
        strategies.contains(&ImplementationStrategy::Popcount),
        "Missing Popcount strategy"
    );
    assert!(
        strategies.contains(&ImplementationStrategy::PackedBitfield),
        "Missing PackedBitfield strategy"
    );
    assert!(
        strategies.contains(&ImplementationStrategy::MaskConstruction),
        "Missing MaskConstruction strategy"
    );
}

#[test]
fn bs001_candidates_are_deterministic() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    // Run generation twice from the same semantic state.
    let get_ids = || {
        let mut inference = InferenceEngine::new();
        inference.infer(semantics.database(), semantics.structural_database());
        let mut generator = CandidateGenerator::new();
        generator.generate(inference.context_database(), semantics.database());
        generator
            .database()
            .all_candidates()
            .map(|c| c.strategy)
            .collect::<Vec<_>>()
    };

    let first = get_ids();
    let second = get_ids();
    assert_eq!(first, second, "Generation must be deterministic");
}
