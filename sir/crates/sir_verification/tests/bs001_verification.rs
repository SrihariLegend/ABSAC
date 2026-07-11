//! BS001 Verification Integration Tests (Tier 4).
//!
//! End-to-end pipeline tests: build SIR -> analyze -> semantics -> inference ->
//! generation -> verification. Proves the canonical BS001 theorem:
//!
//!   Count(Filter(BooleanArray(v), True)) ≡ Popcount(Pack(BooleanArray(v)))

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_nodes::Function;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};
use sir_verification::obligation::ProofObligation;
use sir_verification::{ProofStep, VerificationBackend, VerificationResult, Verifier};

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
fn build_board_scan() -> Function {
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
    let inc = b.select(elem, one_i32, zero_i32, Span::unknown()).unwrap();

    // count = count + inc — accumulate (sum reduction)
    let new_count = b.add(count_initial, inc, Span::unknown()).unwrap();

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

/// Run the full BS001 pipeline up to building obligations and verifying them.
/// Returns the verifier, the list of (obligation, result) pairs, and
/// the inference engine's context database for further inspection.
fn run_full_pipeline() -> (Verifier, Vec<(ProofObligation, VerificationResult)>) {
    let func = build_board_scan();

    // Analysis
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);

    // Semantics
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    // Inference
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());

    // Generation
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database());

    // Build the Verifier and create obligations
    let verifier = Verifier::new();
    let obligations_db =
        verifier.build_obligations(generator.database(), inference.context_database());

    assert!(
        obligations_db.len() > 0,
        "Should have at least one proof obligation"
    );

    // Verify each obligation
    let mut results = Vec::new();
    for obligation in obligations_db.all() {
        let context = inference
            .context_database()
            .for_region(obligation.region)
            .first()
            .expect("Context should exist for this region");
        let result = verifier.verify(obligation, context);
        results.push((obligation.clone(), result));
    }

    (verifier, results)
}

#[test]
fn bs001_verification_pipeline_proves_popcount_equivalence() {
    let (_, results) = run_full_pipeline();

    assert!(!results.is_empty(), "Should have verification results");

    // Find the Popcount result
    let popcount_result = results.iter().find(|(obl, _)| {
        use sir_verification::semantic::expression::SemanticExpression;
        matches!(obl.theorem.rhs, SemanticExpression::Popcount(_))
    });

    let (_, result) = popcount_result.expect("Should have a Popcount obligation in the results");

    match result {
        VerificationResult::Proven(proof) => {
            assert_eq!(
                proof.backend,
                VerificationBackend::Symbolic,
                "BS001 should be proven by the symbolic backend"
            );
            assert!(
                !proof.steps.is_empty(),
                "Proof should have normalization steps"
            );
            assert!(
                proof.steps.iter().any(|s| matches!(
                    s,
                    ProofStep::Normalization {
                        rule: "CountFilterToPopcount",
                        ..
                    }
                )),
                "Normalization should include CountFilterToPopcount rule"
            );
            assert_eq!(
                proof.normalized_theorem.lhs, proof.normalized_theorem.rhs,
                "Normalized theorem sides should be structurally equal"
            );
        }
        other => panic!(
            "Expected VerificationResult::Proven for BS001, got {:?}",
            other
        ),
    }
}

#[test]
fn bs001_verification_report_is_generated() {
    let (verifier, results) = run_full_pipeline();

    // Collect just the verification results
    let v_results: Vec<VerificationResult> = results.iter().map(|(_, r)| r.clone()).collect();

    // Check statistics
    let stats = verifier.statistics(&v_results);
    assert!(
        stats.proven > 0,
        "Expected at least 1 proven obligation, got proven={}, total={}",
        stats.proven,
        stats.total
    );

    // Create report input: pairs of (obligation, result)
    let report_input: Vec<_> = results
        .iter()
        .map(|(obl, res)| (obl.clone(), res.clone()))
        .collect();

    let report = verifier.report(&report_input);
    let report_str = report.to_string();

    // Report should mention Popcount transformation
    assert!(
        report_str.contains("Popcount"),
        "Report should contain 'Popcount', got:\n{}",
        report_str
    );

    // Report should indicate PROVEN status
    assert!(
        report_str.contains("PROVEN"),
        "Report should contain 'PROVEN', got:\n{}",
        report_str
    );

    // Each entry should have a non-empty details section
    for entry in &report.entries {
        assert!(
            entry.details.is_some(),
            "Entry '{}' should have details",
            entry.transformation_name
        );
        let details = entry.details.as_ref().unwrap();
        assert!(
            !details.is_empty(),
            "Entry '{}' should have non-empty details",
            entry.transformation_name
        );
    }
}
