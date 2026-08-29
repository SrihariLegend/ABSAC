use sir_inference::evidence::Polarity;
use sir_inference::sources::bitset_evidence;
use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{RecognitionExplanation, Region, RegionId};
use sir_transform::representation::Representation;

fn make_region(with_concepts: &[SemanticConcept]) -> Region {
    let mut region = Region::new(RegionId::new(0));
    for &concept in with_concepts {
        region.add_concept(
            concept,
            RecognitionExplanation {
                concept,
                triggering_facts: vec!["test"],
            },
        );
    }
    region
}

#[test]
fn empty_region_produces_no_evidence() {
    let region = Region::new(RegionId::new(0));
    let evidence = bitset_evidence::contribute(&region, &[]);
    assert!(evidence.is_empty());
}

#[test]
fn boolean_collection_supports_bitset() {
    let region = make_region(&[SemanticConcept::LogicalSequence]);
    let evidence = bitset_evidence::contribute(&region, &[]);

    assert!(!evidence.is_empty());
    let bool_ev = evidence
        .iter()
        .find(|e| matches!(e.polarity, Polarity::Supports))
        .unwrap();
    assert_eq!(bool_ev.representation, Representation::BitSet);
    assert!(bool_ev.weight > 0);
}

#[test]
fn finite_collection_supports_bitset() {
    let region = make_region(&[SemanticConcept::FiniteCollection]);
    let evidence = bitset_evidence::contribute(&region, &[]);

    let finite_ev = evidence
        .iter()
        .find(|e| matches!(e.polarity, Polarity::Supports))
        .unwrap();
    assert_eq!(finite_ev.representation, Representation::BitSet);
}

#[test]
fn membership_traversal_supports_bitset() {
    let region = make_region(&[SemanticConcept::MembershipTraversal]);
    let evidence = bitset_evidence::contribute(&region, &[]);

    assert!(!evidence.is_empty());
    let ev = evidence.first().unwrap();
    assert_eq!(ev.representation, Representation::BitSet);
}

#[test]
fn cardinality_reduction_supports_bitset() {
    let region = make_region(&[SemanticConcept::CardinalityReduction]);
    let evidence = bitset_evidence::contribute(&region, &[]);

    assert!(!evidence.is_empty());
    let ev = evidence.first().unwrap();
    assert_eq!(ev.representation, Representation::BitSet);
}

#[test]
fn all_four_concepts_together_produce_four_evidence_entries() {
    let region = make_region(&[
        SemanticConcept::LogicalSequence,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let evidence = bitset_evidence::contribute(&region, &[]);
    // Each concept contributes one evidence entry, all Supports
    let supporting: Vec<_> = evidence
        .iter()
        .filter(|e| matches!(e.polarity, Polarity::Supports))
        .collect();
    assert_eq!(supporting.len(), 4);
}

#[test]
fn evidence_contains_explanatory_text() {
    let region = make_region(&[SemanticConcept::LogicalSequence]);
    let evidence = bitset_evidence::contribute(&region, &[]);
    let ev = evidence.first().unwrap();
    assert!(!ev.explanation.is_empty());
}

#[test]
fn evidence_is_all_supports_for_positive_concepts() {
    let region = make_region(&[
        SemanticConcept::LogicalSequence,
        SemanticConcept::FiniteCollection,
        SemanticConcept::MembershipTraversal,
        SemanticConcept::CardinalityReduction,
    ]);
    let evidence = bitset_evidence::contribute(&region, &[]);
    for ev in &evidence {
        assert!(
            matches!(ev.polarity, Polarity::Supports),
            "Expected all evidence to be Supports, got {:?} for {}",
            ev.polarity,
            ev.source
        );
    }
}
