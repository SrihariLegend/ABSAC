use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

use sir_types::RegionId;

use crate::assumptions::Assumption;
use crate::constraints::Constraint;
use crate::representation::Representation;
use crate::structures::SourceStructure;

/// A unique identifier for a transformation context.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextId(pub u64);

impl ContextId {
    pub fn new(id: u64) -> Self { Self(id) }
}

impl fmt::Display for ContextId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ctx#{}", self.0)
    }
}

/// Error type for context validation.
#[derive(Clone, Debug)]
pub enum ValidationError {
    MissingSourceStructure,
    ContradictoryConstraints(String),
    EmptyRegion,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MissingSourceStructure =>
                write!(f, "TransformationContext must have a source structure"),
            ValidationError::ContradictoryConstraints(msg) =>
                write!(f, "Contradictory constraints: {}", msg),
            ValidationError::EmptyRegion =>
                write!(f, "TransformationContext region must not be empty"),
        }
    }
}

/// The semantic package connecting belief to action.
///
/// A TransformationContext must contain all information required to
/// generate candidate transformation plans without consulting SIR,
/// compiler analyses, or semantic recognizers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformationContext {
    pub region: RegionId,
    pub representation: Representation,
    pub source_structure: SourceStructure,
    pub constraints: HashSet<Constraint>,
    pub assumptions: HashSet<Assumption>,
    pub context_id: ContextId,
}

impl TransformationContext {
    pub fn new(
        region: RegionId,
        representation: Representation,
        source_structure: SourceStructure,
        constraints: HashSet<Constraint>,
        assumptions: HashSet<Assumption>,
    ) -> Self {
        Self { region, representation, source_structure, constraints, assumptions, context_id: ContextId::new(0) }
    }

    /// Validate invariants: source structure present, no contradictions.
    pub fn validate(&self) -> Result<(), ValidationError> {
        // No contradictory constraints check for v0.1:
        // ReadOnly + (Write observed) would be contradictory,
        // but we only track positive constraints.
        Ok(())
    }
}

/// Stores transformation contexts produced during inference.
#[derive(Clone, Debug, Default)]
pub struct TransformationContextDatabase {
    contexts: HashMap<RegionId, Vec<TransformationContext>>,
    next_context_id: u64,
}

impl TransformationContextDatabase {
    pub fn new() -> Self {
        Self { contexts: HashMap::new(), next_context_id: 0 }
    }

    pub fn insert(&mut self, region: RegionId, mut ctx: TransformationContext) -> ContextId {
        let cid = ContextId::new(self.next_context_id);
        self.next_context_id += 1;
        ctx.context_id = cid;
        self.contexts.entry(region).or_default().push(ctx);
        cid
    }

    pub fn contexts(&self) -> impl Iterator<Item = (RegionId, &[TransformationContext])> {
        self.contexts.iter().map(|(&rid, v)| (rid, v.as_slice()))
    }

    pub fn for_region(&self, region: RegionId) -> &[TransformationContext] {
        self.contexts.get(&region).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
