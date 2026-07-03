//! TransformationDefinition trait and TransformationRegistry.

use sir_generation::candidate::Candidate;
use sir_transform::context::TransformationContext;
use sir_transform::ids::DefinitionId;

use crate::obligation::ProofObligation;

/// The canonical owner of a transformation's mathematics.
///
/// One implementation per transformation family. The planner, verifier,
/// and (future) rewriter all ask the same definition.
///
/// Design principle: Every concept has exactly one canonical owner.
/// Transformation mathematics is owned here — no other component
/// duplicates this knowledge.
pub trait TransformationDefinition {
    /// Unique identifier for this definition.
    fn id(&self) -> DefinitionId;

    /// Human-readable name.
    fn name(&self) -> &'static str;

    /// Is this transformation applicable to the given context?
    fn applicability(&self, context: &TransformationContext) -> bool;

    /// Construct the full proof obligation for a given context.
    /// Owns: theorem construction, assumption enumeration, domain specification.
    fn obligation(&self, context: &TransformationContext) -> ProofObligation;
}

/// Registry of known transformation definitions.
pub struct TransformationRegistry {
    definitions: Vec<Box<dyn TransformationDefinition>>,
}

impl TransformationRegistry {
    pub fn new() -> Self {
        Self {
            definitions: Vec::new(),
        }
    }

    /// Register a transformation definition.
    pub fn register(&mut self, def: Box<dyn TransformationDefinition>) {
        self.definitions.push(def);
    }

    /// Look up a definition by its ID.
    pub fn lookup(&self, id: DefinitionId) -> Option<&dyn TransformationDefinition> {
        self.definitions
            .iter()
            .find(|d| d.id() == id)
            .map(|d| d.as_ref())
    }

    /// Find a definition applicable to the given candidate and context.
    ///
    /// Note: `_candidate` is accepted but `candidate.definition_id` is not
    /// checked yet — the field will be added in a later task and the
    /// full lookup logic restored then.
    /// Returns the first definition that claims applicability.
    pub fn find_for(
        &self,
        _candidate: &Candidate,
        context: &TransformationContext,
    ) -> Option<&dyn TransformationDefinition> {
        self.definitions.iter().find_map(|def| {
            if def.applicability(context) {
                Some(def.as_ref())
            } else {
                None
            }
        })
    }

    /// Number of registered definitions.
    pub fn len(&self) -> usize {
        self.definitions.len()
    }
}

impl Default for TransformationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
