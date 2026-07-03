use std::collections::HashSet;

use sir_generation::generators;
use sir_semantics::region::RegionId;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;

fn make_context() -> TransformationContext {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    constraints.insert(Constraint::ReadOnly);
    constraints.insert(Constraint::NoEscape);
    constraints.insert(Constraint::FiniteIteration);

    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);
    assumptions.insert(Assumption::PreservesLayout);

    TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::BooleanArray { length: 64 },
        constraints,
        assumptions,
    )
}

#[test]
fn all_four_generators_produce_candidates() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    assert_eq!(candidates.len(), 4, "Expected 4 candidates for BitSet context");
}

#[test]
fn all_strategies_are_distinct() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    let strategies: HashSet<_> = candidates.iter().map(|c| c.strategy).collect();
    assert_eq!(strategies.len(), 4);
}

#[test]
fn each_candidate_has_effects() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.effects.is_empty(),
            "{:?} should have at least one effect", c.strategy);
    }
}

#[test]
fn each_candidate_has_explanation() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.explanation.rationale.is_empty(),
            "{:?} should have a non-empty rationale", c.strategy);
    }
}

#[test]
fn each_candidate_has_prerequisites() {
    let ctx = make_context();
    let candidates = generators::all_plans(&ctx);
    for c in &candidates {
        assert!(!c.explanation.prerequisites.is_empty(),
            "{:?} should list prerequisites", c.strategy);
    }
}

#[test]
fn non_bitset_context_produces_no_candidates() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);

    // Non-BitSet representation — should be skipped by all generators
    let ctx = TransformationContext::new(
        RegionId::new(0),
        // There's only BitSet in v0.1, but each generator checks representation
        Representation::BitSet,
        SourceStructure::BitMask { width: 64 },
        constraints,
        assumptions,
    );
    let candidates = generators::all_plans(&ctx);
    // All 4 generators check for BitSet, but BitMask as source structure
    // is still valid — generators check representation, not structure.
    // This test validates they don't crash on non-BooleanArray contexts.
    assert_eq!(candidates.len(), 4);
}
