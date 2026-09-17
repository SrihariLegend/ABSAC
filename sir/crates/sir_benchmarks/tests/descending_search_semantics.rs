//! Direction precision for the synthesized descending search
//! (2026-09-17): clang's `for (i = n; i-- > 0;) if (buf[i]) return i;`
//! scans indices n-1 .. 0, so it is a LAST-occurrence (reverse) search.
//! An `Add(counter, -1)` successor was mis-classified by the position
//! heuristic as a forward scan; normalizing it to `Sub(counter, 1)` makes
//! the direction reverse. The exclusive-counter form must also stay
//! unauthorized (the reverse totality witness requires the access to use
//! the carried index), so no candidate is minted.

use std::collections::HashSet;

use sir_analysis::manager::AnalysisManager;
use sir_generation::generator::CandidateGenerator;
use sir_inference::engine::InferenceEngine;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::semantics::SemanticEngine;

const DESCENDING_SEARCH: &str = r#"
define i64 @desc(ptr %0, i64 %1) {
  br label %3

3:
  %4 = phi i64 [ %1, %2 ], [ %7, %6 ]
  %5 = icmp eq i64 %4, 0
  br i1 %5, label %11, label %6

6:
  %7 = add i64 %4, -1
  %8 = getelementptr inbounds i8, ptr %0, i64 %7
  %9 = load i8, ptr %8
  %10 = icmp eq i8 %9, 0
  br i1 %10, label %3, label %11

11:
  %12 = phi i64 [ %1, %3 ], [ %7, %6 ]
  ret i64 %12
}
"#;

#[test]
fn descending_search_is_reverse_and_unauthorized() {
    let func = sir_lower::lower_function(DESCENDING_SEARCH, "desc")
        .expect("the descending search must lower");
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
        concepts.contains(&SemanticConcept::LastOccurrence),
        "a descending scan must derive LastOccurrence, got {concepts:?}"
    );
    assert!(
        !concepts.contains(&SemanticConcept::FirstOccurrence),
        "a descending scan must never derive FirstOccurrence, got {concepts:?}"
    );

    let mut inference = InferenceEngine::new();
    inference.infer(semantics.database(), semantics.structural_database());
    let authorizations = sir_semantics::authorization::derive_authorizations(
        &func,
        analysis.database(),
        semantics.database(),
    );
    let mut generator = CandidateGenerator::new();
    generator.generate(
        inference.context_database(),
        semantics.database(),
        &authorizations,
        &func,
    );
    assert_eq!(
        generator.database().all_candidates().count(),
        0,
        "the exclusive-counter reverse form must not be authorized"
    );
}
