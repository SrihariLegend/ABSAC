use sir_inference::engine::{HypothesisDatabase, InferenceEngine};
use sir_inference::hypothesis::{Hypothesis, Representation, Support};

#[test]
fn empty_database_has_no_hypotheses() {
    let db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    assert!(db.hypotheses(rid).is_empty());
    assert!(db.best(rid).is_none());
}

#[test]
fn database_stores_and_retrieves_hypothesis() {
    let mut db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    let h = Hypothesis {
        representation: Representation::BitSet,
        support: Support {
            positive: 85,
            negative: 10,
        },
        evidence: vec![0, 1],
    };
    db.add_hypothesis(rid, h.clone());

    assert_eq!(db.hypotheses(rid).len(), 1);
    let best = db.best(rid).unwrap();
    assert_eq!(best.representation, Representation::BitSet);
    assert_eq!(best.support.score(), 75);
}

#[test]
fn database_best_returns_highest_scoring() {
    let mut db = HypothesisDatabase::new();
    let rid = sir_semantics::region::RegionId::new(0);
    db.add_hypothesis(
        rid,
        Hypothesis {
            representation: Representation::BitSet,
            support: Support {
                positive: 30,
                negative: 10,
            },
            evidence: vec![],
        },
    );
    db.add_hypothesis(
        rid,
        Hypothesis {
            representation: Representation::BitSet,
            support: Support {
                positive: 90,
                negative: 5,
            },
            evidence: vec![],
        },
    );
    let best = db.best(rid).unwrap();
    assert_eq!(best.support.score(), 85);
}

#[test]
fn database_regions_supporting_filters() {
    let mut db = HypothesisDatabase::new();
    let r1 = sir_semantics::region::RegionId::new(0);
    let r2 = sir_semantics::region::RegionId::new(1);
    db.add_hypothesis(
        r1,
        Hypothesis {
            representation: Representation::BitSet,
            support: Support {
                positive: 80,
                negative: 5,
            },
            evidence: vec![],
        },
    );
    db.add_hypothesis(
        r2,
        Hypothesis {
            representation: Representation::BitSet,
            support: Support {
                positive: 10,
                negative: 50,
            },
            evidence: vec![],
        },
    );

    let supporting = db.regions_supporting(Representation::BitSet);
    assert!(supporting.contains(&r1));
    assert!(supporting.contains(&r2)); // both have BitSet hypotheses
}

#[test]
fn engine_new_creates_empty_state() {
    let engine = InferenceEngine::new();
    assert!(engine
        .database()
        .hypotheses(sir_semantics::region::RegionId::new(0))
        .is_empty());
}
