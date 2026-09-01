/// Errors that can occur during verified rewriting.
#[derive(Clone, Debug, PartialEq)]
pub enum RewriteError {
    /// Candidate and Recipe definition IDs don't match.
    /// (Proof does not carry DefinitionId in v0.1 — when it does, add a third field.)
    DefinitionMismatch {
        candidate: sir_transform::ids::DefinitionId,
        recipe: sir_transform::ids::DefinitionId,
    },

    /// The StructuralDescription doesn't carry the expected role.
    MissingRole { role: String },

    /// A node referenced in the patch was not found in the original function.
    NodeNotFound(sir_types::NodeId),

    /// The recipe produced a patch that fails structural verification.
    StructuralVerificationFailed(Vec<sir_verify::VerificationError>),

    /// The recipe failed to produce a patch.
    RecipeFailed(String),

    /// The loop's downstream consumer reads a tuple slot the recipe's
    /// theorem does not speak about (e.g. a position/index live-out
    /// while the theorem covers only the boolean accumulator).
    /// Replacing it would silently change program semantics — the
    /// rewrite must refuse (advisor PS002 audit: "correctly authorized
    /// at concept level but incorrectly bound to concrete roles").
    UnauthorizedLiveOut {
        consumer: sir_types::NodeId,
        field: usize,
        reduction_position: usize,
    },

    /// A consumer of the loop tuple exists in a form the recipe cannot
    /// classify (whole-tuple escape, copy, store, non-numeric projection,
    /// multiple consumers). Until complete use-closure binding exists,
    /// the rewrite abstains (advisor PS002 follow-up).
    UnknownConsumer { value: sir_types::NodeId, reason: String },

    /// Indicates a compiler bug — an invariant was violated.
    InternalInvariantViolation(String),
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RewriteError::DefinitionMismatch { candidate, recipe } => {
                write!(
                    f,
                    "definition mismatch: candidate={candidate}, recipe={recipe}"
                )
            }
            RewriteError::MissingRole { role } => {
                write!(f, "missing role in structural region: {role}")
            }
            RewriteError::NodeNotFound(id) => write!(f, "node {id} not found"),
            RewriteError::StructuralVerificationFailed(errors) => {
                write!(f, "structural verification failed: {} errors", errors.len())
            }
            RewriteError::RecipeFailed(msg) => write!(f, "recipe failed: {msg}"),
            RewriteError::UnauthorizedLiveOut {
                consumer,
                field,
                reduction_position,
            } => write!(
                f,
                "unauthorized live-out: consumer {consumer} reads tuple slot {field} \
                 but the theorem only covers the reduction slot {reduction_position}"
            ),
            RewriteError::UnknownConsumer { value, reason } => {
                write!(f, "unknown consumer of {value}: {reason}")
            }
            RewriteError::InternalInvariantViolation(msg) => {
                write!(f, "INTERNAL INVARIANT VIOLATION: {msg}")
            }
        }
    }
}

impl std::error::Error for RewriteError {}
