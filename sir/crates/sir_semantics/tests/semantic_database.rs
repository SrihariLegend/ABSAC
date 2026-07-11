use sir_semantics::concepts::SemanticConcept;
use sir_semantics::region::{RecognitionExplanation, Region, RegionId};
use sir_semantics::semantics::SemanticDatabase;

#[test]
fn empty_database_has_no_regions() {
    let db = SemanticDatabase::new();
    assert_eq!(db.region_count(), 0);
    assert!(db.regions().next().is_none());
}

#[test]
fn database_stores_and_retrieves_region() {
    let mut db = SemanticDatabase::new();
    let rid = RegionId::new(0);
    let mut region = Region::new(rid);
    region.add_concept(
        SemanticConcept::BooleanCollection,
        RecognitionExplanation {
            concept: SemanticConcept::BooleanCollection,
            triggering_facts: vec!["Array<bool>"],
        },
    );
    db.add_region(region);

    assert_eq!(db.region_count(), 1);
    let retrieved = db.region(rid).unwrap();
    assert!(retrieved.contains(SemanticConcept::BooleanCollection));
    assert!(!retrieved.contains(SemanticConcept::MembershipTraversal));
}

#[test]
fn database_regions_iterates_all() {
    let mut db = SemanticDatabase::new();
    for i in 0..3 {
        let rid = RegionId::new(i);
        let mut region = Region::new(rid);
        region.add_concept(
            SemanticConcept::FiniteCollection,
            RecognitionExplanation {
                concept: SemanticConcept::FiniteCollection,
                triggering_facts: vec!["trip_count"],
            },
        );
        db.add_region(region);
    }
    let regions: Vec<_> = db.regions().collect();
    assert_eq!(regions.len(), 3);
}

#[test]
fn database_explain_returns_explanation() {
    let mut db = SemanticDatabase::new();
    let rid = RegionId::new(0);
    let mut region = Region::new(rid);
    region.add_concept(
        SemanticConcept::BooleanCollection,
        RecognitionExplanation {
            concept: SemanticConcept::BooleanCollection,
            triggering_facts: vec!["Array element type is Bool"],
        },
    );
    db.add_region(region);

    let explanation = db.explain(rid, SemanticConcept::BooleanCollection);
    assert!(explanation.is_some());
    assert!(explanation
        .unwrap()
        .triggering_facts
        .contains(&"Array element type is Bool"));
}

#[test]
fn database_explain_unknown_returns_none() {
    let db = SemanticDatabase::new();
    assert!(db
        .explain(RegionId::new(99), SemanticConcept::BooleanCollection)
        .is_none());
}
