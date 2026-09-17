//! TransformationDefinition trait and TransformationRegistry.

use sir_generation::candidate::Candidate;
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

    /// The MAXIMUM assurance the checker may issue for obligations
    /// produced by this definition's schema (advisor item 3: checker-
    /// issued assurance is distinct from definition metadata).
    ///
    /// This is a CAP, not a status: the definition cannot raise its
    /// issued assurance by declaring a higher level. The verifier
    /// issues `min(self declaration, backend capability)` and gates
    /// policy on the ISSUED level, never on this declaration alone.
    /// A Stub declaration still fails closed regardless of what any
    /// backend proves (the obligation quality itself is untrusted).
    fn verification_status(&self) -> VerificationStatus;

    /// Is this transformation applicable to the given candidate?
    fn applicability(&self, candidate: &Candidate) -> bool;

    /// Construct the full proof obligation for a given candidate.
    /// Owns: theorem construction, assumption enumeration, domain specification.
    fn obligation(&self, candidate: &Candidate) -> ProofObligation;

    /// Construct the obligation bound to the ACTUAL function version the
    /// candidate was authorized against (P0A quarantine lift, advisor
    /// item 3).
    ///
    /// The default keeps legacy/stub definitions unchanged. Definitions
    /// that declare `ConcreteSolverChecked` override this and must build
    /// a theorem whose constants/widths come from the candidate's
    /// authorized region nodes — never a hardcoded template. An
    /// unbound obligation must be left without a finite domain so no
    /// backend can discharge it.
    fn obligation_bound(
        &self,
        candidate: &Candidate,
        _function: &sir_nodes::Function,
    ) -> ProofObligation {
        self.obligation(candidate)
    }
}

/// Explicit assurance level of a transformation definition's proof
/// (advisor directive). Production rewrite policy fails closed on
/// insufficient level; the level travels with the proof artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VerificationStatus {
    /// Hardcoded example or unconditional result that does not bind
    /// actual source/candidate operands. NEVER permits a rewrite.
    Stub,
    /// Backed by differential/unit tests but no concrete proof.
    /// Permitted only in explicitly experimental mode.
    TestedOnly,
    /// Checks a valid theorem schema and concrete bindings/
    /// preconditions, relying on trusted handwritten verification
    /// code. Usable during research with honest labeling.
    SchemaChecked,
    /// The exact source/candidate expressions and assumptions were
    /// submitted to a solver; the candidate carries a reproducible
    /// obligation/result.
    ConcreteSolverChecked,
    /// Checked by an independent proof system with a replayable
    /// artifact. Required for the strongest Gate 4B claim.
    MachineChecked,
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

    /// Find a definition applicable to the given candidate.
    /// Checks both applicability and definition_id match.
    pub fn find_for(&self, candidate: &Candidate) -> Option<&dyn TransformationDefinition> {
        self.definitions.iter().find_map(|def| {
            if def.id() == candidate.definition_id && def.applicability(candidate) {
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
