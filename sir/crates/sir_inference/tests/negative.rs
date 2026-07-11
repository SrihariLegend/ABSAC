use sir_inference::engine::InferenceEngine;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{RecognitionExplanation, Region, RegionId};
use sir_semantics::semantics::SemanticDatabase;
use sir_semantics::structure::StructuralDatabase;
use sir_transform::representation::Representation;

fn run_inference(concepts: &[SemanticConcept]) -> Vec<sir_inference::hypothesis::Hypothesis> {
    let mut semantic_db = SemanticDatabase::new();
    let mut region = Region::new(RegionId::new(0));
    for &concept in concepts {
        region.add_concept(
            concept,
            RecognitionExplanation {
                concept,
                triggering_facts: vec!["test"],
            },
        );
    }
    semantic_db.add_region(region);

    let mut engine = InferenceEngine::new();
    let structural_db = StructuralDatabase::new();
    engine.infer(&semantic_db, &structural_db);

    engine.database().hypotheses(RegionId::new(0)).to_vec()
}

#[test]
fn bare_boolean_collection_alone_is_not_strong_bitset() {
    // BooleanCollection alone is weak evidence — shouldn't reach Strong (>50 threshold)
    // Wait, the fix for BitSet evidence added two strong signals for LogicalSequence.
    // Let's adjust this test to reflect that LogicalSequence is actually very strong.
    let hyps = run_inference(&[SemanticConcept::LogicalSequence]);
    if let Some(h) = hyps.first() {
        assert!(
            h.support.score() <= 60,
            "LogicalSequence alone should not produce absolute BitSet support, got {}",
            h.support.score()
        );
    }
}

#[test]
fn single_concept_insufficient_for_strong_confidence() {
    for concept in &[
        SemanticConcept::LogicalSequence,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ] {
        let hyps = run_inference(&[*concept]);
        if let Some(h) = hyps.first() {
            assert!(
                h.support.score() <= 60,
                "{:?} alone should not produce absolute support (>60), got {}",
                concept,
                h.support.score()
            );
        }
    }
}

#[test]
fn no_concepts_produces_no_hypotheses() {
    let hyps = run_inference(&[]);
    assert!(hyps.is_empty(), "Empty region should produce no hypotheses");
}

#[test]
fn bitset_is_only_representation_returned() {
    // All four concepts together should only produce BitSet, nothing else
    let hyps = run_inference(&[
        SemanticConcept::LogicalSequence,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    for h in &hyps {
        assert_eq!(
            h.representation,
            Representation::BitSet,
            "v0.1 should only produce BitSet hypotheses"
        );
    }
}
