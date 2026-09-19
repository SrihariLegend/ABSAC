use sir_types::NodeId;
use sir_types::Type;

/// Errors that can occur during SIR construction.
///
/// The builder performs type checking and effect computation at node
/// creation time. If a type mismatch or other invariant violation is
/// detected, a `BuildError` is returned instead of a `NodeId`.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildError {
    /// A referenced node was not found in the arena.
    NodeNotFound(NodeId),

    /// A type mismatch was detected between expected and actual types.
    TypeMismatch {
        /// The node that caused the mismatch.
        node: NodeId,
        /// The expected type.
        expected: Type,
        /// The actual type found.
        actual: Type,
    },

    /// A parameter with the given name was not found.
    MissingParameter { name: String },

    /// A return node was already set for this function.
    DuplicateReturn,

    /// The number of arguments doesn't match the expected count.
    InvalidArity { expected: usize, actual: usize },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NodeNotFound(id) => write!(f, "node {id} not found in arena"),
            BuildError::TypeMismatch {
                node,
                expected,
                actual,
            } => write!(
                f,
                "type mismatch for node {node}: expected {expected}, got {actual}"
            ),
            BuildError::MissingParameter { name } => {
                write!(f, "missing parameter: {name}")
            }
            BuildError::DuplicateReturn => {
                write!(f, "return node already set for this function")
            }
            BuildError::InvalidArity { expected, actual } => {
                write!(
                    f,
                    "invalid arity: expected {expected} arguments, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for BuildError {}
