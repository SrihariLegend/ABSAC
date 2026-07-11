use sir_generation::candidate::{
    Candidate, CandidateEffect, CandidateExplanation, CandidateId, ImplementationStrategy,
};
use sir_generation::generator::CandidateDatabase;
use sir_generation::generator::CandidateGenerator;
use sir_transform::context::ContextId;
use sir_transform::ids::DefinitionId;
use sir_types::{CostProfile, RegionId};

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
        definition_id: DefinitionId::new(0),
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "test",
        },
        effects: vec![CandidateEffect::TraversalChange],
        expected_cost: CostProfile::default(),
        representation: sir_transform::representation::Representation::BitSet,
        source_structure: sir_transform::structures::SourceStructure::LogicalSequence {
            length: 64,
        },
        constraints: std::collections::HashSet::new(),
        assumptions: std::collections::HashSet::new(),
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
        definition_id: DefinitionId::new(0),
        explanation: CandidateExplanation {
            source_concepts: vec![],
            rationale: "test",
        },
        effects: vec![],
        expected_cost: CostProfile::default(),
        representation: sir_transform::representation::Representation::BitSet,
        source_structure: sir_transform::structures::SourceStructure::LogicalSequence {
            length: 64,
        },
        constraints: std::collections::HashSet::new(),
        assumptions: std::collections::HashSet::new(),
    };
    db.add(RegionId::new(0), c);
    assert!(db.validate().is_err());
}
