use sir_generation::candidate::{
    Candidate, CandidateEffects, CandidateExplanation, CandidateId,
    ImplementationStrategy,
};
use sir_generation::generator::CandidateDatabase;
use sir_generation::generator::CandidateGenerator;
use sir_semantics::region::RegionId;
use sir_transform::context::ContextId;

#[test]
fn empty_generator_has_no_candidates() {
    let gen = CandidateGenerator::new();
    let db = gen.database();
    assert_eq!(db.region_count(), 0);
}

#[test]
fn database_validate_rejects_duplicate_ids() {
    let mut db = CandidateDatabase::new();
    let rid = RegionId::new(0);
    let cid = ContextId::new(0);
    let id = CandidateId::new(0);

    let c = Candidate {
        id,
        region: rid,
        context_id: cid,
        strategy: ImplementationStrategy::BitIteration,
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "test",
        },
        effects: vec![CandidateEffects::TraversalChange],
    };

    db.add(rid, c.clone());
    db.add(rid, c); // duplicate ID
    assert!(db.validate().is_err());
}

#[test]
fn database_validate_rejects_empty_effects() {
    let mut db = CandidateDatabase::new();
    let c = Candidate {
        id: CandidateId::new(0),
        region: RegionId::new(0),
        context_id: ContextId::new(0),
        strategy: ImplementationStrategy::Popcount,
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "test",
        },
        effects: vec![],
    };
    db.add(RegionId::new(0), c);
    assert!(db.validate().is_err());
}
