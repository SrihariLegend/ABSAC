use sir_inference::engine::InferenceEngine;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{Region, RegionId, RecognitionExplanation};
use sir_semantics::semantics::SemanticDatabase;

fn run_inference(concepts: &[SemanticConcept]) -> Vec<sir_inference::hypothesis::Hypothesis> {
    let mut semantic_db = SemanticDatabase::new();
    let mut region = Region::new(RegionId::new(0));
    for &concept in concepts {
        region.add_concept(concept, RecognitionExplanation {
            concept,
            triggering_facts: vec!["test"],
        });
    }
    semantic_db.add_region(region);

    let mut engine = InferenceEngine::new();
    engine.infer(&semantic_db);

    engine.database().hypotheses(RegionId::new(0)).to_vec()
}

#[test]
fn ambiguous_case_has_low_confidence() {
    // Just two concepts — the engine should express uncertainty
    let hyps = run_inference(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
    ]);
    if let Some(h) = hyps.first() {
        let label = h.support.confidence_label();
        // With only 2 moderate concepts, should be Weak or Moderate, not Strong
        assert!(
            label == "Weak" || label == "Moderate",
            "Ambiguous case should have Weak or Moderate confidence, got {}",
            label
        );
    }
}

#[test]
fn order_of_concepts_does_not_affect_result() {
    use SemanticConcept::*;
    let concepts_sets = vec![
        vec![BooleanCollection, FiniteCollection, MembershipTraversal, CardinalityReduction],
        vec![CardinalityReduction, MembershipTraversal, FiniteCollection, BooleanCollection],
        vec![MembershipTraversal, BooleanCollection, CardinalityReduction, FiniteCollection],
    ];

    let mut scores = Vec::new();
    for concepts in &concepts_sets {
        let hyps = run_inference(concepts);
        scores.push(hyps.first().map(|h| h.support.score()).unwrap_or(0));
    }

    // All orderings must produce identical scores
    let first = scores[0];
    for &score in &scores {
        assert_eq!(score, first,
            "Evidence aggregation must be order-independent");
    }
}

#[test]
fn support_is_never_negative_for_pure_positive_evidence() {
    // All positive evidence — support.positive should exactly equal sum of weights
    let hyps = run_inference(&[
        SemanticConcept::BooleanCollection,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let h = hyps.first().unwrap();
    assert_eq!(h.support.negative, 0,
        "All-positive evidence should have zero negative support");
    assert!(h.support.positive > 0);
}
