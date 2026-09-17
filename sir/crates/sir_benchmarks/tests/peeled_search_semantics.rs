//! Direction and authorization precision for the de-peeled search
//! (2026-09-17): the peeled ascending scan must derive FirstOccurrence
//! (never LastOccurrence), and the runtime extent must not be promoted,
//! so no candidate is authorized.

use std::collections::HashSet;

use sir_analysis::manager::AnalysisManager;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;

const PEELED_SEARCH: &str = r#"
define i64 @peeled(ptr %0, i64 %1) {
  %3 = icmp ne i64 %1, 0
  br i1 %3, label %4, label %18

4:
  %5 = load i8, ptr %0
  %6 = icmp eq i8 %5, 0
  br i1 %6, label %11, label %18

7:
  %8 = getelementptr inbounds i8, ptr %0, i64 %13
  %9 = load i8, ptr %8
  %10 = icmp eq i8 %9, 0
  br i1 %10, label %11, label %15

11:
  %12 = phi i64 [ %13, %7 ], [ 0, %4 ]
  %13 = add nuw i64 %12, 1
  %14 = icmp eq i64 %13, %1
  br i1 %14, label %15, label %7

15:
  %16 = phi i64 [ %1, %11 ], [ %13, %7 ]
  %17 = icmp ult i64 %13, %1
  br label %18

18:
  %19 = phi i64 [ 0, %2 ], [ 0, %4 ], [ %16, %15 ]
  %20 = phi i1 [ %3, %2 ], [ %3, %4 ], [ %17, %15 ]
  %21 = add i64 %1, -1
  %22 = select i1 %20, i64 %19, i64 %21
  ret i64 %22
}
"#;

#[test]
fn peeled_search_is_forward_and_unauthorized() {
    let func = sir_lower::lower_function(PEELED_SEARCH, "peeled")
        .expect("the peeled search must lower");
    let mut analysis = AnalysisManager::new();
    analysis.run_all(&func);
    let mut semantics = SemanticEngine::new();
    semantics.derive(&func, analysis.database());
    let concepts: HashSet<SemanticConcept> = semantics
        .database()
        .regions()
        .flat_map(|(_, region)| region.concepts().iter().copied())
        .collect();
    assert!(
        concepts.contains(&SemanticConcept::FirstOccurrence),
        "an ascending scan must derive FirstOccurrence, got {concepts:?}"
    );
    assert!(
        !concepts.contains(&SemanticConcept::LastOccurrence),
        "an ascending scan must never derive LastOccurrence, got {concepts:?}"
    );

    // A candidate may be minted (the scan is recognized), but without a
    // promoted fixed extent the obligation cannot bind, so no rewrite may
    // be applied.
    let optimizer = sir_optimizer::Optimizer::new(
        sir_optimizer::OptimizerConfig::default(),
        sir_rewrite::registry::default_registry(),
    );
    let result = optimizer.optimize(&func);
    assert_eq!(
        result.rewrites_applied, 0,
        "a runtime extent must not be rewritten into a fixed-width mask"
    );
}
