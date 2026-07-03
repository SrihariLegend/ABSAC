//! BS001 Board Scan Integration Test — Acceptance Criterion for the SRI Pipeline.
//!
//! These tests build a realistic SIR function representing a fixed-size boolean
//! membership scan (board scan), run the full pipeline (analysis → semantics
//! → inference), and assert that:
//!
//! 1. All four semantic concepts are recognized
//! 2. The BitSet representation is inferred with strong support (>50)
//! 3. The explanation accounts for the support score

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_inference::engine::InferenceEngine;
use sir_transform::representation::Representation;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

/// Build a SIR function that represents:
///
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
/// This is BS001: the canonical fixed-size boolean membership scan.
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
    let board = b.parameter_index(0).unwrap(); // %0

    // ── Constants ───────────────────────────────────────────
    // NOTE: loop counter and bound use unsigned u64 because the loop
    // analysis's trip-count estimation uses `as_u64()` which only
    // decodes unsigned integer constants.
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown()); // %1
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown()); // %2
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown()); // %3
    let count_initial = b.constant(ConstantData::i32(0), Type::i32(), Span::unknown()); // %4
    let one_i32 = b.constant(ConstantData::i32(1), Type::i32(), Span::unknown()); // %5

    // ── Loop body (references carried inputs as loop variables) ──

    // board[i] — element access (pure, produces Bool)
    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap(); // %6

    // inc = board[i] ? 1 : 0 — convert boolean to integer increment
    let inc = b
        .select(elem, one_i32, count_initial, Span::unknown())
        .unwrap(); // %7

    // count = count + inc — accumulate (sum reduction)
    let new_count = b
        .add(count_initial, inc, Span::unknown())
        .unwrap(); // %8

    // i = i + 1 — increment loop counter
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap(); // %9

    // i < 64 — loop termination condition
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap(); // %10

    // ── Loop node ──────────────────────────────────────────
    // Carried inputs: [count_initial, i_initial]
    // Outputs: [new_count, i_next]
    //
    // The loop iterates while `cond` is true.
    // Each iteration feeds the outputs back as the carried values.
    // The loop returns both values as a Tuple (i32, i64).
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
        .unwrap(); // %11

    // ── Return ──────────────────────────────────────────────
    b.return_value(loop_node, Span::unknown()).unwrap(); // %12
    b.build()
}

// ── Tests ────────────────────────────────────────────────────

#[test]
fn bs001_board_scan_recognizes_all_four_concepts() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let db = semantics.database();

    // Verify all four concepts are recognized somewhere in the function.
    let mut found_boolean = false;
    let mut found_finite = false;
    let mut found_membership = false;
    let mut found_cardinality = false;

    for (_rid, region) in db.regions() {
        if region.contains(SemanticConcept::BooleanCollection) {
            found_boolean = true;
        }
        if region.contains(SemanticConcept::FiniteCollection) {
            found_finite = true;
        }
        if region.contains(SemanticConcept::MembershipTraversal) {
            found_membership = true;
        }
        if region.contains(SemanticConcept::CardinalityReduction) {
            found_cardinality = true;
        }
    }

    assert!(found_boolean, "Expected BooleanCollection concept");
    assert!(found_finite, "Expected FiniteCollection concept");
    assert!(found_membership, "Expected MembershipTraversal concept");
    assert!(
        found_cardinality,
        "Expected CardinalityReduction concept"
    );

    // After region merging, all concepts should be in the same region.
    assert_eq!(db.region_count(), 1, "All concepts should merge into one region");
}

#[test]
fn bs001_board_scan_infers_bitset_with_strong_support() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database());

    let db = inference.database();
    let mut found = false;

    for (rid, _region) in semantics.database().regions() {
        if let Some(h) = db.best(rid) {
            assert_eq!(
                h.representation,
                Representation::BitSet,
                "Expected BitSet representation"
            );
            assert!(
                h.support.score() > 50,
                "Expected strong support (>50), got {}",
                h.support.score()
            );
            found = true;
        }
    }

    assert!(found, "Expected at least one BitSet hypothesis");

    // Combined evidence weights:
    //   BooleanCollection:   30 (STRONG)
    //   FiniteCollection:    20 (MODERATE)
    //   MembershipTraversal: 30 (STRONG)
    //   CardinalityReduction:20 (MODERATE)
    //   Total:              100 (>50)
    //
    // After region merging, all contributions accumulate in one region.
}

#[test]
fn bs001_explanation_accounts_for_support() {
    let func = build_board_scan();

    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database());

    let mut found_explanation = false;

    for (rid, _region) in semantics.database().regions() {
        if let Some(h) = inference.database().best(rid) {
            let explanation = inference.explain(rid, h.representation).unwrap();
            let explanation_str = format!("{}", explanation);

            // The explanation must reference the concepts that contributed evidence.
            assert!(
                explanation_str.contains("BooleanCollection"),
                "Explanation should mention BooleanCollection"
            );
            assert!(
                explanation_str.contains("MembershipTraversal"),
                "Explanation should mention MembershipTraversal"
            );

            // The support score should appear in the explanation.
            assert!(
                explanation_str.contains(&h.support.score().to_string()),
                "Explanation should show the support score {}",
                h.support.score()
            );

            found_explanation = true;
        }
    }

    assert!(
        found_explanation,
        "Expected at least one explanation for a hypothesis"
    );
}
