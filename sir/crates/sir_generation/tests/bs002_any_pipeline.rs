//! BS002 — "Any" Pattern Pipeline Test.
//!
//! End-to-end test: build SIR -> analyze -> semantic + structural -> inference -> generation.
//! Traces the "any" (disjunctive reduction) pattern through the pipeline to identify
//! which phase blocks recognition of `any(board)` ≡ `pack(board) != 0`.

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

/// Build the BS002 "board any" SIR function.
///
/// Represents:
/// ```text
/// fn board_any(board: [bool; 64]) -> bool {
///     let mut found = false;
///     for i in 0..64 {
///         if board[i] { found = true; }
///     }
///     found
/// }
/// ```
///
/// This is a disjunctive reduction (OR across elements) —
/// different from BS001's counting reduction.
fn build_board_any() -> sir_nodes::Function {
    let mut b = Builder::new(
        "board_any",
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

    // ── Constants ──
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let found_initial = b.constant(ConstantData::boolean(false), Type::Bool, Span::unknown());

    // ── Loop body ──
    // board[i]
    let elem = b
        .array_access(board, i_initial, Type::Bool, Span::unknown())
        .unwrap();

    // new_found = found || board[i]  (OR reduction)
    let new_found = b
        .bool_or(found_initial, elem, Span::unknown())
        .unwrap();

    // i = i + 1
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap();

    // i < 64
    let cond = b
        .lt(i_initial, limit, Span::unknown())
        .unwrap();

    // ── Loop node ──
    let loop_node = b
        .r#loop(
            &[elem, new_found, i_next, cond],
            cond,
            &[new_found, i_next],
            &[found_initial, i_initial],
            Type::Tuple {
                elements: vec![Type::Bool, Type::u64()],
            },
            Span::unknown(),
        )
        .unwrap();

    // Extract `found` from the tuple result
    let found = b
        .field_access(loop_node, "0", Type::Bool, Span::unknown())
        .unwrap();

    b.return_value(found, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn bs002_trace_pipeline() {
    let func = build_board_any();

    // ── Phase 1: Analysis ──
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let fact_count = analysis.database().total_facts();
    println!("[BS002] Analysis: {} facts", fact_count);
    assert!(fact_count > 0, "Should have analysis facts");

    // ── Phase 2: Semantics + Structure ──
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    let truth_regions = semantics.database().region_count();
    let struct_regions = semantics.structural_database().region_count();
    println!(
        "[BS002] Semantics: {} truth regions, {} structural regions",
        truth_regions, struct_regions
    );

    // Print recognized concepts
    for (rid, region) in semantics.database().regions() {
        println!("[BS002]   Region {}: concepts = {:?}", rid, region.concepts());
    }
    for (_rid, desc) in semantics.structural_database().regions() {
        println!(
            "[BS002]   Structure: {:?}, roles = {:?}",
            desc.source_structure, desc.roles
        );
    }

    // ── Phase 3: Inference ──
    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let ctx_count: usize = inference.context_database().contexts().count();
    println!("[BS002] Inference: {} contexts", ctx_count);

    for (_rid, ctxs) in inference.context_database().contexts() {
        for ctx in ctxs {
            println!(
                "[BS002]   Representation: {:?}, constraints: {:?}",
                ctx.representation, ctx.constraints
            );
        }
    }

    // ── Phase 4: Generation ──
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database());
    let db = generator.database();
    let candidate_count = db.all_candidates().count();
    println!("[BS002] Generation: {} candidates", candidate_count);

    for c in db.all_candidates() {
        println!(
            "[BS002]   {}: {:?} (def={})",
            c.id, c.strategy, c.definition_id
        );
    }

    // ── Diagnostic ──
    if truth_regions == 0 {
        println!("\n*** BLOCKER: No semantic truths recognized ***");
        println!("The semantic recognizers don't match the 'any' pattern.");
        println!("Need: a DisjunctiveReduction recognizer (OR across elements).");
    } else if ctx_count == 0 {
        println!("\n*** BLOCKER: No transformation contexts inferred ***");
        println!("Inference didn't produce contexts from the recognized truths.");
    } else if candidate_count == 0 {
        println!("\n*** BLOCKER: No candidates generated ***");
        println!("The generator didn't produce any transformation plans.");
        println!("Either: no BitSet context, or concepts don't match any strategy.");
    } else {
        println!("\n*** Candidates generated! Pipeline partially works. ***");
    }
}
