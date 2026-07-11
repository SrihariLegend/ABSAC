use std::collections::HashSet;

use sir_transform::assumptions::Assumption;
use sir_transform::constraints::Constraint;
use sir_transform::context::TransformationContext;
use sir_transform::representation::Representation;
use sir_transform::structures::SourceStructure;
use sir_types::RegionId;

#[test]
fn valid_context_passes_validation() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(64));
    let mut assumptions = HashSet::new();
    assumptions.insert(Assumption::EquivalentCardinality);

    let ctx = TransformationContext::new(
        RegionId::new(0),
        Representation::BitSet,
        SourceStructure::LogicalSequence { length: 64 },
        constraints,
        assumptions,
    );
    assert!(ctx.validate().is_ok());
}

#[test]
fn context_validation_accepts_bitmask() {
    let mut constraints = HashSet::new();
    constraints.insert(Constraint::FixedLength(32));
    let assumptions = HashSet::new();

    let ctx = TransformationContext::new(
        RegionId::new(1),
        Representation::BitSet,
        SourceStructure::BitMask { width: 32 },
        constraints,
        assumptions,
    );
    assert!(ctx.validate().is_ok());
}
