//! BS003 — "All" (Conjunctive Reduction) Pattern Pipeline Test.
//!
//! Traces: `all &&= board[i]` → expected `pack(board) == all_ones`
//! Operator: `&&` (bool_and)

use sir_analysis::manager::AnalysisManager;
use sir_builder::Builder;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::semantics::SemanticEngine;
use sir_types::{ConstantData, Span, Type};

fn build_board_all() -> sir_nodes::Function {
    let mut b = Builder::new(
        "board_all",
        &[("board", Type::Array { element: Box::new(Type::Bool), length: 64 })],
        Type::Bool,
    );

    let board = b.parameter_index(0).unwrap();
    let i_initial = b.constant(ConstantData::u64(0), Type::u64(), Span::unknown());
    let i_step = b.constant(ConstantData::u64(1), Type::u64(), Span::unknown());
    let limit = b.constant(ConstantData::u64(64), Type::u64(), Span::unknown());
    let all_initial = b.constant(ConstantData::boolean(true), Type::Bool, Span::unknown());

    // Body: all = all && board[i]
    let elem = b.array_access(board, i_initial, Type::Bool, Span::unknown()).unwrap();
    let new_all = b.bool_and(all_initial, elem, Span::unknown()).unwrap();   // ← && operator
    let i_next = b.add(i_initial, i_step, Span::unknown()).unwrap();
    let cond = b.lt(i_initial, limit, Span::unknown()).unwrap();

    let loop_node = b.r#loop(
        &[elem, new_all, i_next, cond], cond,
        &[new_all, i_next], &[all_initial, i_initial],
        Type::Tuple { elements: vec![Type::Bool, Type::u64()] },
        Span::unknown(),
    ).unwrap();

    let result = b.field_access(loop_node, "0", Type::Bool, Span::unknown()).unwrap();
    b.return_value(result, Span::unknown()).unwrap();
    b.build()
}

#[test]
fn bs003_trace_pipeline() {
    let func = build_board_all();
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());

    println!("[BS003 all/&&]");
    for (_rid, region) in semantics.database().regions() {
        println!("  concepts = {:?}", region.concepts());
    }
    for (_rid, desc) in semantics.structural_database().regions() {
        println!("  structure = {:?}, roles = {:?}", desc.source_structure, desc.roles);
    }

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let mut generator = CandidateGenerator::new();
    generator.generate(inference.context_database(), semantics.database());

    for c in generator.database().all_candidates() {
        println!("  {}: {:?}", c.id, c.strategy);
    }

    // Diagnostic: check if CardinalityReduction is still firing
    let has_cardinality = semantics.database().regions()
        .any(|(_rid, r)| r.contains(sir_semantics::concepts::SemanticConcept::CardinalityReduction));

    if has_cardinality {
        println!("\n  *** CardinalityReduction fires for && operator — concept is too coarse ***");
    }
}
