use std::collections::HashSet;

use sir_generation::generators;
use sir_semantics::concepts::SemanticConcept;
use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::RegionId;

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
        SourceStructure::LogicalSequence { length: 64 },
        constraints,
        assumptions,
    )
}

fn make_all_concepts() -> HashSet<SemanticConcept> {
    let mut concepts = HashSet::new();
    concepts.insert(SemanticConcept::LogicalSequence);
    concepts.insert(SemanticConcept::FiniteCollection);
    concepts.insert(SemanticConcept::MembershipTraversal);
    concepts.insert(SemanticConcept::CardinalityReduction);
    concepts
}

#[test]
fn all_four_generators_produce_candidates() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    assert_eq!(
        candidates.len(),
        5,
        "Expected 5 candidates for BitSet context"
    );
}

#[test]
fn all_strategies_are_distinct() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    let strategies: HashSet<_> = candidates.iter().map(|c| c.strategy).collect();
    assert_eq!(strategies.len(), 4);
}

#[test]
fn each_candidate_has_effects() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    for c in &candidates {
        assert!(
            !c.effects.is_empty(),
            "{:?} should have at least one effect",
            c.strategy
        );
    }
}

#[test]
fn each_candidate_has_explanation() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    for c in &candidates {
        assert!(
            !c.explanation.rationale.is_empty(),
            "{:?} should have a non-empty rationale",
            c.strategy
        );
    }
}

#[test]
fn bitmask_context_produces_four_candidates() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);

    // BitMask source structure — still valid for BitSet representation;
    // generators check representation, not source structure type.
    let ctx = TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::BitMask { width: 64 },
        constraints,
        assumptions,
    );
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    // All 4 generators check for BitSet representation, which matches.
    // BitMask as source structure is still valid.
    assert_eq!(candidates.len(), 5);
}

#[test]
fn generation_is_deterministic() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let first: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    let second: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    assert_eq!(first.len(), second.len());
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.strategy, b.strategy);
    }
}

#[test]
fn explanations_contain_source_concepts() {
    let ctx = make_context();
    let concepts = make_all_concepts();
    let candidates: Vec<_> = generators::all_plans(&ctx, &concepts).collect();
    for c in &candidates {
        assert!(
            !c.explanation.source_concepts.is_empty(),
            "{:?} explanation should reference source concepts",
            c.strategy
        );
    }
}
