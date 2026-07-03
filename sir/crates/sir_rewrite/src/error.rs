/// Errors that can occur during verified rewriting.
#[derive(Clone, Debug, PartialEq)]
pub enum RewriteError {
    /// Candidate, Proof, and Recipe definition IDs don't match.
    DefinitionMismatch {
        candidate: sir_transform::ids::DefinitionId,
        proof: sir_transform::ids::DefinitionId,
        recipe: sir_transform::ids::DefinitionId,
    },

    /// The StructuralDescription doesn't carry the expected role.
    MissingRole {
        role: String,
    },

    /// A node referenced in the patch was not found in the original function.
    NodeNotFound(sir_types::NodeId),

    /// The recipe produced a patch that fails structural verification.
    StructuralVerificationFailed(Vec<sir_verify::VerificationError>),

    /// The recipe failed to produce a patch.
    RecipeFailed(String),

    /// Indicates a compiler bug — an invariant was violated.
    InternalInvariantViolation(String),
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RewriteError::DefinitionMismatch {
                candidate,
                proof,
                recipe,
            } => {
                write!(
                    f,
                    "definition mismatch: candidate={candidate}, proof={proof}, recipe={recipe}"
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
            RewriteError::InternalInvariantViolation(msg) => {
                write!(f, "INTERNAL INVARIANT VIOLATION: {msg}")
            }
        }
    }
}

impl std::error::Error for RewriteError {}
