use sir_nodes::NodeKind;
use sir_types::{NodeId, Type};

/// Errors detected during SIR graph verification.
///
/// Each variant corresponds to a specific invariant violation.
/// The verifier collects all errors before reporting, so a single
/// verification run can surface multiple issues.
#[derive(Clone, Debug, PartialEq)]
pub enum VerificationError {
    /// A referenced NodeId does not exist in the arena.
    NodeNotFound(NodeId),

    /// A type mismatch between expected and actual types for a node input.
    TypeMismatch {
        /// The node where the mismatch was detected.
        node: NodeId,
        /// The kind of the node.
        kind: NodeKind,
        /// Which input index (0-based) had the mismatch.
        input_index: usize,
        /// The expected type.
        expected: Type,
        /// The actual type found.
        actual: Type,
    },

    /// The return value's type does not match the function's declared return type.
    ReturnTypeMismatch {
        /// The function's declared return type.
        expected: Type,
        /// The type of the returned value.
        actual: Type,
    },

    /// A cycle was detected in the dependency graph (outside a loop body).
    CycleDetected(NodeId),

    /// A node references a NodeId that does not exist in the arena.
    DanglingReference {
        /// The node containing the bad reference.
        node: NodeId,
        /// The missing NodeId that was referenced.
        referenced: NodeId,
    },

    /// The function has no return node.
    MissingReturn,

    /// The function has more than one return node.
    DuplicateReturn,

    /// A Parameter node has an index outside the function's parameter list.
    ParameterIndexMismatch {
        param_index: usize,
        expected_count: usize,
    },

    /// A Loop's termination condition is not Bool.
    LoopTerminationNotBool { loop_node: NodeId, actual: Type },

    /// A Select's condition is not Bool.
    SelectConditionNotBool { node: NodeId, actual: Type },

    /// A pointer operation received a non-pointer operand.
    InvalidPointerOperation { node: NodeId, actual: Type },

    /// A Loop's carried_inputs count doesn't match outputs count.
    LoopCarriedMismatch {
        node: NodeId,
        carried_count: usize,
        output_count: usize,
    },

    /// A node has inputs that are not allowed for its kind.
    InvalidInput {
        node: NodeId,
        kind: NodeKind,
        message: String,
    },
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerificationError::NodeNotFound(id) => write!(f, "node {id} not found"),
            VerificationError::TypeMismatch {
                node,
                kind,
                input_index,
                expected,
                actual,
            } => write!(
                f,
                "type mismatch: node {node} ({kind}) input {input_index}: expected {expected}, got {actual}"
            ),
            VerificationError::ReturnTypeMismatch { expected, actual } => write!(
                f,
                "return type mismatch: expected {expected}, got {actual}"
            ),
            VerificationError::CycleDetected(id) => write!(f, "cycle detected at node {id}"),
            VerificationError::DanglingReference { node, referenced } => write!(
                f,
                "dangling reference: node {node} references non-existent node {referenced}"
            ),
            VerificationError::MissingReturn => write!(f, "missing return"),
            VerificationError::DuplicateReturn => write!(f, "multiple return nodes"),
            VerificationError::ParameterIndexMismatch {
                param_index,
                expected_count,
            } => write!(
                f,
                "parameter index {param_index} out of range (expected {expected_count} params)"
            ),
            VerificationError::LoopTerminationNotBool { loop_node, actual } => write!(
                f,
                "loop {loop_node} termination must be Bool, got {actual}"
            ),
            VerificationError::SelectConditionNotBool { node, actual } => write!(
                f,
                "select {node} condition must be Bool, got {actual}"
            ),
            VerificationError::InvalidPointerOperation { node, actual } => write!(
                f,
                "node {node} expected pointer type, got {actual}"
            ),
            VerificationError::LoopCarriedMismatch {
                node,
                carried_count,
                output_count,
            } => write!(
                f,
                "loop {node}: carried_inputs count ({carried_count}) != outputs count ({output_count})"
            ),
            VerificationError::InvalidInput {
                node,
                kind,
                message,
            } => write!(f, "invalid input for node {node} ({kind}): {message}"),
        }
    }
}

impl std::error::Error for VerificationError {}
